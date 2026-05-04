use super::*;
use curve25519_dalek::{
    ristretto::RistrettoPoint,
    scalar::Scalar,
    traits::{Identity, VartimeMultiscalarMul},
};
use rand::{SeedableRng, rngs::OsRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedFischlinRound {
    pub t: RistrettoPoint,
    pub e: u16,
    pub z_coeffs: Vec<Scalar>,
    pub z_blindings: Vec<Scalar>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedFischlinProof {
    pub rounds: Vec<PolyWellFormedFischlinRound>,
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct RoundState {
    #[zeroize(skip)]
    t: RistrettoPoint,
    r_coeffs: Vec<Scalar>,
    r_blindings: Vec<Scalar>,
}

#[inline]
fn statement_common_h_prefix(statement: &PolyWellFormedStatement) -> Sha512 {
    let mut h = Sha512::new();
    h.update(b"poly-well-formedness-fischlin-common-h-v1");

    h.update((statement.x_points.len() as u64).to_le_bytes());
    for x in &statement.x_points {
        h.update(x.as_bytes());
    }

    h.update(statement.f0_commitment.compress().as_bytes());

    h.update((statement.commitments.len() as u64).to_le_bytes());
    for c in &statement.commitments {
        h.update(c.point().compress().as_bytes());
    }

    h
}

pub fn hash_statement_and_first_messages(
    statement: &PolyWellFormedStatement,
    first_messages: &[RistrettoPoint],
) -> [u8; 64] {
    let mut h = statement_common_h_prefix(statement);
    h.update((first_messages.len() as u64).to_le_bytes());
    for t in first_messages {
        h.update(t.compress().as_bytes());
    }

    let digest = h.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

pub fn hash_statement_and_round_ts(
    statement: &PolyWellFormedStatement,
    rounds: &[PolyWellFormedFischlinRound],
) -> [u8; 64] {
    let mut h = statement_common_h_prefix(statement);
    h.update((rounds.len() as u64).to_le_bytes());
    for round in rounds {
        h.update(round.t.compress().as_bytes());
    }

    let digest = h.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

#[inline]
fn score_hasher_prefix(common_h: &[u8; 64], round_index: usize, t: &RistrettoPoint) -> Sha512 {
    let mut h = Sha512::new();
    h.update(b"poly-well-formedness-fischlin-score-v1");
    h.update(common_h);
    h.update((round_index as u64).to_le_bytes());
    h.update(t.compress().as_bytes());
    h
}

pub fn fischlin_score_u32(
    common_h: &[u8; 64],
    round_index: usize,
    t: &RistrettoPoint,
    e: u16,
    z_coeffs: &[Scalar],
    z_blindings: &[Scalar],
) -> u32 {
    let base = score_hasher_prefix(common_h, round_index, t);
    fischlin_score_u32_from_prefix(&base, e, z_coeffs, z_blindings)
}

#[inline]
fn fischlin_score_u32_from_prefix(
    base: &Sha512,
    e: u16,
    z_coeffs: &[Scalar],
    z_blindings: &[Scalar],
) -> u32 {
    let mut h = base.clone();
    h.update((e as u64).to_le_bytes());

    h.update((z_coeffs.len() as u64).to_le_bytes());
    for z in z_coeffs {
        h.update(z.as_bytes());
    }

    h.update((z_blindings.len() as u64).to_le_bytes());
    for z in z_blindings {
        h.update(z.as_bytes());
    }

    let digest = h.finalize();
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

pub fn score_has_b_leading_zero_bits(score: u32, b: usize) -> bool {
    debug_assert!(b > 0 && b <= 31);
    (score >> (32 - b)) == 0
}

#[inline]
pub fn sample_scalar<R: rand::RngCore + rand::CryptoRng>(rng: &mut R) -> Scalar {
    Scalar::random(rng)
}

pub fn prove_fischlin_with_params(
    statement: &PolyWellFormedStatement,
    witness: &PolyWellFormedWitness,
    rho: usize,
    b: usize,
    t_bits: usize,
) -> PolyWellFormedFischlinProof {
    assert!(rho > 0, "rho must be positive");
    assert!(b > 0 && b <= 31, "b must be in 1..=31");
    assert!(
        t_bits > 0 && t_bits <= 15,
        "t_bits must be in 1..=15 for u16 challenges"
    );
    assert!(b <= t_bits, "b should be <= t_bits");
    assert!(!witness.coeffs.is_empty(), "coeffs must not be empty");
    assert_eq!(statement.x_points.len(), statement.commitments.len());
    assert_eq!(statement.x_points.len(), witness.blindings.len());

    let n = statement.x_points.len();
    let degree = witness.coeffs.len() - 1;

    let mut transcript = Transcript::new(b"poly-well-formedness-fischlin");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, degree);

    let g_point = g();
    let h_point = h();

    let challenge_space: u16 = 1u16 << t_bits;

    // Seed once from OS entropy, then use a fast CSPRNG stream.
    let mut rng = StdRng::from_rng(&mut OsRng).expect("failed to seed StdRng from OsRng");

    loop {
        let mut states = Vec::with_capacity(rho);
        let mut first_messages = Vec::with_capacity(rho);

        for _ in 0..rho {
            let r_coeffs: Vec<Scalar> = (0..=degree).map(|_| sample_scalar(&mut rng)).collect();
            let r_blindings: Vec<Scalar> = (0..n).map(|_| sample_scalar(&mut rng)).collect();

            let coeff_part = r_coeffs
                .iter()
                .zip(mus.iter())
                .fold(Scalar::ZERO, |acc, (r, mu)| acc + (*r * *mu));

            let blind_part = r_blindings
                .iter()
                .zip(lambdas.iter().skip(1))
                .fold(Scalar::ZERO, |acc, (r, lambda)| acc + (*r * *lambda));

            let t = RistrettoPoint::vartime_multiscalar_mul(
                [coeff_part, blind_part],
                [g_point, h_point],
            );

            first_messages.push(t);
            states.push(RoundState {
                t,
                r_coeffs,
                r_blindings,
            });
        }

        let common_h = hash_statement_and_first_messages(statement, &first_messages);

        let mut rounds = Vec::with_capacity(rho);
        let mut all_found = true;

        for (i, state) in states.into_iter().enumerate() {
            let mut z_coeffs = state.r_coeffs.clone();
            let mut z_blindings = state.r_blindings.clone();

            let score_prefix = score_hasher_prefix(&common_h, i, &state.t);
            let mut found_e = None;

            for e in 0..challenge_space {
                let score =
                    fischlin_score_u32_from_prefix(&score_prefix, e, &z_coeffs, &z_blindings);

                if score_has_b_leading_zero_bits(score, b) {
                    found_e = Some(e);
                    break;
                }

                if e + 1 != challenge_space {
                    for (z, a) in z_coeffs.iter_mut().zip(witness.coeffs.iter()) {
                        *z += *a;
                    }
                    for (z, rho_i) in z_blindings.iter_mut().zip(witness.blindings.iter()) {
                        *z += *rho_i;
                    }
                }
            }

            match found_e {
                Some(e) => rounds.push(PolyWellFormedFischlinRound {
                    t: state.t,
                    e,
                    z_coeffs,
                    z_blindings,
                }),
                None => {
                    z_coeffs.zeroize();
                    z_blindings.zeroize();
                    all_found = false;
                    break;
                }
            }
        }

        if all_found {
            return PolyWellFormedFischlinProof { rounds };
        }
    }
}

pub fn verify_fischlin_with_params(
    statement: &PolyWellFormedStatement,
    proof: &PolyWellFormedFischlinProof,
    rho: usize,
    b: usize,
    t_bits: usize,
) -> bool {
    let n = statement.x_points.len();

    if rho == 0 || b == 0 || b > 31 || t_bits == 0 || t_bits > 15 || b > t_bits {
        return false;
    }
    if statement.commitments.len() != n {
        return false;
    }
    if proof.rounds.len() != rho {
        return false;
    }
    if proof.rounds.is_empty() || proof.rounds[0].z_coeffs.is_empty() {
        return false;
    }

    let degree = proof.rounds[0].z_coeffs.len() - 1;

    let mut transcript = Transcript::new(b"poly-well-formedness-fischlin");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, degree);
    let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

    let common_h = hash_statement_and_round_ts(statement, &proof.rounds);

    let g_point = g();
    let h_point = h();

    for (i, round) in proof.rounds.iter().enumerate() {
        if round.z_blindings.len() != n {
            return false;
        }
        if round.z_coeffs.len() != degree + 1 {
            return false;
        }
        if (round.e as u32) >= (1u32 << t_bits) {
            return false;
        }

        // Cheap check first
        let score = fischlin_score_u32(
            &common_h,
            i,
            &round.t,
            round.e,
            &round.z_coeffs,
            &round.z_blindings,
        );

        if !score_has_b_leading_zero_bits(score, b) {
            return false;
        }

        let coeff_part = round
            .z_coeffs
            .iter()
            .zip(mus.iter())
            .fold(Scalar::ZERO, |acc, (z, mu)| acc + (*z * *mu));

        let blind_part = round
            .z_blindings
            .iter()
            .zip(lambdas.iter().skip(1))
            .fold(Scalar::ZERO, |acc, (z, lambda)| acc + (*z * *lambda));

        let e_scalar = Scalar::from(round.e as u64);

        let check = RistrettoPoint::vartime_multiscalar_mul(
            [coeff_part, blind_part, -Scalar::ONE, -e_scalar],
            [g_point, h_point, round.t, p_star],
        );

        if check != RistrettoPoint::identity() {
            return false;
        }
    }

    true
}

pub fn batch_verify_fischlin_with_params(
    statements: &[PolyWellFormedStatement],
    proofs: &[PolyWellFormedFischlinProof],
    rho: usize,
    b: usize,
    t_bits: usize,
) -> bool {
    if statements.len() != proofs.len() || statements.is_empty() {
        return false;
    }

    if rho == 0 || b == 0 || b > 31 || t_bits == 0 || t_bits > 15 || b > t_bits {
        return false;
    }

    let g_point = g();
    let h_point = h();

    let mut g_scalar_acc = Scalar::ZERO;
    let mut h_scalar_acc = Scalar::ZERO;

    let mut msm_scalars: Vec<Scalar> = Vec::new();
    let mut msm_points: Vec<RistrettoPoint> = Vec::new();

    for (proof_idx, (statement, proof)) in statements.iter().zip(proofs.iter()).enumerate() {
        let n = statement.x_points.len();

        if statement.commitments.len() != n {
            return false;
        }

        if proof.rounds.len() != rho {
            return false;
        }

        if proof.rounds.is_empty() || proof.rounds[0].z_coeffs.is_empty() {
            return false;
        }

        let degree = proof.rounds[0].z_coeffs.len() - 1;

        let mut transcript = Transcript::new(b"poly-well-formedness-fischlin");
        append_statement_to_transcript(&mut transcript, statement);

        let lambdas = derive_lambdas(&mut transcript, n);
        let mus = compute_mus(&statement.x_points, &lambdas, degree);
        let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

        let common_h = hash_statement_and_round_ts(statement, &proof.rounds);

        let mut p_star_scalar_acc = Scalar::ZERO;

        for (round_idx, round) in proof.rounds.iter().enumerate() {
            if round.z_blindings.len() != n {
                return false;
            }

            if round.z_coeffs.len() != degree + 1 {
                return false;
            }

            if (round.e as u32) >= (1u32 << t_bits) {
                return false;
            }

            // Cheap check first
            let score = fischlin_score_u32(
                &common_h,
                round_idx,
                &round.t,
                round.e,
                &round.z_coeffs,
                &round.z_blindings,
            );

            if !score_has_b_leading_zero_bits(score, b) {
                return false;
            }

            let coeff_part = round
                .z_coeffs
                .iter()
                .zip(mus.iter())
                .fold(Scalar::ZERO, |acc, (z, mu)| acc + (*z * *mu));

            let blind_part = round
                .z_blindings
                .iter()
                .zip(lambdas.iter().skip(1))
                .fold(Scalar::ZERO, |acc, (z, lambda)| acc + (*z * *lambda));

            let e_scalar = Scalar::from(round.e as u64);

            let mut batch_tr = Transcript::new(b"poly-well-formedness-fischlin-batch-weight");
            batch_tr.append_message(b"proof_index", &(proof_idx as u64).to_le_bytes());
            batch_tr.append_message(b"round_index", &(round_idx as u64).to_le_bytes());

            append_statement_to_transcript(&mut batch_tr, statement);
            batch_tr.append_point(b"t", &round.t);
            batch_tr.append_u64(b"e", round.e as u64);

            for z in &round.z_coeffs {
                batch_tr.append_scalar(b"z_coeff", z);
            }
            for z in &round.z_blindings {
                batch_tr.append_scalar(b"z_blinding", z);
            }

            let alpha = batch_tr.challenge_scalar(b"alpha");

            g_scalar_acc += alpha * coeff_part;
            h_scalar_acc += alpha * blind_part;

            msm_scalars.push(-alpha);
            msm_points.push(round.t);

            p_star_scalar_acc += alpha * e_scalar;
        }

        if p_star_scalar_acc != Scalar::ZERO {
            msm_scalars.push(-p_star_scalar_acc);
            msm_points.push(p_star);
        }
    }

    if g_scalar_acc != Scalar::ZERO {
        msm_scalars.push(g_scalar_acc);
        msm_points.push(g_point);
    }

    if h_scalar_acc != Scalar::ZERO {
        msm_scalars.push(h_scalar_acc);
        msm_points.push(h_point);
    }

    let acc = if msm_scalars.is_empty() {
        RistrettoPoint::identity()
    } else {
        RistrettoPoint::vartime_multiscalar_mul(msm_scalars, msm_points)
    };

    acc == RistrettoPoint::identity()
}

#[derive(Clone, Debug)]
pub struct FischlinProofParams {
    pub rho: usize,
    pub b: usize,
    pub t_bits: usize,
}

#[derive(Clone, Debug)]
pub struct FischlinPolyProof;

impl PolyProofScheme for FischlinPolyProof {
    type Proof = PolyWellFormedFischlinProof;
    type Params = FischlinProofParams;

    fn prove(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof {
        prove_fischlin_with_params(statement, witness, params.rho, params.b, params.t_bits)
    }

    fn verify(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool {
        verify_fischlin_with_params(statement, proof, params.rho, params.b, params.t_bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{g, h};
    use crate::pedersen::PedersenCommitment;
    use crate::poly::eval_poly_at;
    use curve25519_dalek::scalar::Scalar;

    fn build_instance(
        coeffs: Vec<Scalar>,
        xs: Vec<Scalar>,
        blindings: Vec<Scalar>,
    ) -> (PolyWellFormedStatement, PolyWellFormedWitness) {
        let commitments: Vec<PedersenCommitment> = xs
            .iter()
            .zip(blindings.iter())
            .map(|(x, rho)| {
                let fx = eval_poly_at(&coeffs, *x);
                PedersenCommitment::new(fx, *rho)
            })
            .collect();

        let statement = PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g() * coeffs[0],
        };

        let witness = PolyWellFormedWitness { coeffs, blindings };

        (statement, witness)
    }

    #[test]
    fn test_fischlin_proof_valid_custom_params() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let blindings = vec![Scalar::from(7u64), Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_batch_verify_valid_single() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        assert!(batch_verify_fischlin_with_params(
            &[statement],
            &[proof],
            4,
            4,
            9
        ));
    }

    #[test]
    fn test_batch_verify_valid_many() {
        let mut statements = Vec::new();
        let mut proofs = Vec::new();

        for k in 0..4u64 {
            let coeffs = vec![
                Scalar::from(2 + k),
                Scalar::from(3 + k),
                Scalar::from(5 + k),
            ];
            let xs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
            let blindings = vec![
                Scalar::from(7 + k),
                Scalar::from(11 + k),
                Scalar::from(13 + k),
            ];

            let (statement, witness) = build_instance(coeffs, xs, blindings);
            let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

            statements.push(statement);
            proofs.push(proof);
        }

        assert!(batch_verify_fischlin_with_params(
            &statements,
            &proofs,
            4,
            4,
            9
        ));
    }

    #[test]
    fn test_fischlin_verify_fails_if_commitment_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        let modified = *statement.commitments[1].point() + g() * Scalar::ONE;
        statement.commitments[1] = PedersenCommitment::from_point(modified);

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_f0_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        statement.f0_commitment += g() * Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_x_points_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        statement.x_points[1] = Scalar::from(9u64);

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_t_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].t += h() * Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_e_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].e ^= 1;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_z_coeff_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_coeffs[1] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_z_blinding_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_blindings[0] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_removed() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_wrong_blinding_length() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_blindings.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_wrong_coeff_length() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_coeffs.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_invalid_params() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64)];
        let xs = vec![Scalar::from(1u64)];
        let blindings = vec![Scalar::from(5u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 2, 4, 9);

        assert!(!verify_fischlin_with_params(&statement, &proof, 0, 4, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 0, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 0));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 10, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 16));
    }

    #[test]
    fn test_fischlin_verify_fails_if_invalid_round_count() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64)];
        let xs = vec![Scalar::from(1u64)];
        let blindings = vec![Scalar::from(5u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 3, 4, 9);

        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_for_wrong_witness() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(7u64), Scalar::from(11u64)];

        let (statement, mut witness) = build_instance(coeffs, xs, blindings);
        witness.coeffs[1] += Scalar::ONE;

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_constant_polynomial() {
        let coeffs = vec![Scalar::from(42u64)];
        let xs = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(5u64),
            Scalar::from(9u64),
        ];
        let blindings = vec![
            Scalar::from(10u64),
            Scalar::from(11u64),
            Scalar::from(12u64),
            Scalar::from(13u64),
        ];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_many_evaluation_points() {
        let coeffs = vec![
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(5u64),
            Scalar::from(7u64),
        ];
        let xs = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
            Scalar::from(5u64),
        ];
        let blindings = vec![
            Scalar::from(11u64),
            Scalar::from(12u64),
            Scalar::from(13u64),
            Scalar::from(14u64),
            Scalar::from(15u64),
        ];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }
}
