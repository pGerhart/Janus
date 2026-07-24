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
pub struct DecomFischlinRound {
    pub t: RistrettoPoint,
    pub e: u16,
    pub z_a: Vec<Scalar>,
    pub z_b: Vec<Scalar>,
    pub z_omega: Scalar,
    pub z_r: Scalar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecomFischlinProof {
    pub rounds: Vec<DecomFischlinRound>,
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct RoundState {
    #[zeroize(skip)]
    t: RistrettoPoint,
    r_a: Vec<Scalar>,
    r_b: Vec<Scalar>,
    r_omega: Scalar,
    r_r: Scalar,
}

#[derive(Clone, Debug)]
pub struct FischlinDecomProofParams {
    pub rho: usize,
    pub b: usize,
    pub t_bits: usize,
}

#[inline]
fn statement_common_h_prefix(statement: &DecomStatement) -> Sha512 {
    let mut hsh = Sha512::new();
    hsh.update(b"decom-fischlin-common-h-batched-pedvss-v1");
    hsh.update((statement.pedvss.len() as u64).to_le_bytes());
    for (k, c_k) in statement.pedvss.iter().enumerate() {
        hsh.update((k as u64).to_le_bytes());
        hsh.update(c_k.point().compress().as_bytes());
    }
    hsh.update(statement.d.point().compress().as_bytes());
    hsh
}

pub fn hash_statement_and_first_messages(
    statement: &DecomStatement,
    first_messages: &[RistrettoPoint],
) -> [u8; 64] {
    let mut hsh = statement_common_h_prefix(statement);
    hsh.update((first_messages.len() as u64).to_le_bytes());

    for t in first_messages {
        hsh.update(t.compress().as_bytes());
    }

    let digest = hsh.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

pub fn hash_statement_and_round_ts(
    statement: &DecomStatement,
    rounds: &[DecomFischlinRound],
) -> [u8; 64] {
    let mut hsh = statement_common_h_prefix(statement);
    hsh.update((rounds.len() as u64).to_le_bytes());

    for round in rounds {
        hsh.update(round.t.compress().as_bytes());
    }

    let digest = hsh.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

#[inline]
fn score_hasher_prefix(common_h: &[u8; 64], round_index: usize, t: &RistrettoPoint) -> Sha512 {
    let mut hsh = Sha512::new();
    hsh.update(b"decom-fischlin-score-batched-pedvss-v1");
    hsh.update(common_h);
    hsh.update((round_index as u64).to_le_bytes());
    hsh.update(t.compress().as_bytes());
    hsh
}

fn derive_lambdas(statement: &DecomStatement) -> Vec<Scalar> {
    let m = statement.pedvss.len();
    let mut transcript = Transcript::new(b"decom_fischlin_lambdas");
    transcript.append_u64(b"pedvss_len", m as u64);
    for (k, c_k) in statement.pedvss.iter().enumerate() {
        transcript.append_u64(b"k", k as u64);
        transcript.append_point(b"C_k", c_k.point());
    }
    transcript.append_point(b"D", statement.d.point());
    (0..=m)
        .map(|k| {
            transcript.append_u64(b"lambda_idx", k as u64);
            transcript.challenge_scalar(b"lambda")
        })
        .collect()
}

fn precompute_pedvss_star(statement: &DecomStatement, lambdas: &[Scalar]) -> RistrettoPoint {
    let m = statement.pedvss.len();
    let scalars = lambdas[..m].iter().chain(std::iter::once(&lambdas[m]));
    let points = statement
        .pedvss
        .iter()
        .map(|c| *c.point())
        .chain(std::iter::once(*statement.d.point()));
    RistrettoPoint::vartime_multiscalar_mul(scalars, points)
}

#[inline]
fn fischlin_score_u32_from_prefix(
    base: &Sha512,
    e: u16,
    z_a: &[Scalar],
    z_b: &[Scalar],
    z_omega: &Scalar,
    z_r: &Scalar,
) -> u32 {
    let mut hsh = base.clone();
    hsh.update((e as u64).to_le_bytes());
    hsh.update((z_a.len() as u64).to_le_bytes());
    for (k, z_a_k) in z_a.iter().enumerate() {
        hsh.update((k as u64).to_le_bytes());
        hsh.update(z_a_k.as_bytes());
    }
    hsh.update((z_b.len() as u64).to_le_bytes());
    for (k, z_b_k) in z_b.iter().enumerate() {
        hsh.update((k as u64).to_le_bytes());
        hsh.update(z_b_k.as_bytes());
    }
    hsh.update(z_omega.as_bytes());
    hsh.update(z_r.as_bytes());

    let digest = hsh.finalize();
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

pub fn fischlin_score_u32(
    common_h: &[u8; 64],
    round_index: usize,
    t: &RistrettoPoint,
    e: u16,
    z_a: &[Scalar],
    z_b: &[Scalar],
    z_omega: &Scalar,
    z_r: &Scalar,
) -> u32 {
    let base = score_hasher_prefix(common_h, round_index, t);
    fischlin_score_u32_from_prefix(&base, e, z_a, z_b, z_omega, z_r)
}

pub fn score_has_b_leading_zero_bits(score: u32, b: usize) -> bool {
    debug_assert!(b > 0 && b <= 31);
    (score >> (32 - b)) == 0
}

#[inline]
fn validate_statement_witness_shape(statement: &DecomStatement, witness: &DecomWitness) -> bool {
    !statement.pedvss.is_empty()
        && statement.pedvss.len() == witness.a.len()
        && statement.pedvss.len() == witness.b.len()
}

#[inline]
fn validate_statement_proof_shape(statement: &DecomStatement, proof: &DecomFischlinProof) -> bool {
    if statement.pedvss.is_empty() || proof.rounds.is_empty() {
        return false;
    }

    proof.rounds.iter().all(|round| {
        round.z_a.len() == statement.pedvss.len() && round.z_b.len() == statement.pedvss.len()
    })
}

pub fn prove_fischlin_with_params(
    statement: &DecomStatement,
    witness: &DecomWitness,
    rho: usize,
    b: usize,
    t_bits: usize,
) -> DecomFischlinProof {
    assert!(rho > 0, "rho must be positive");
    assert!(b > 0 && b <= 31, "b must be in 1..=31");
    assert!(
        t_bits > 0 && t_bits <= 15,
        "t_bits must be in 1..=15 for u16 challenges"
    );
    assert!(b <= t_bits, "b should be <= t_bits");
    assert!(
        validate_statement_witness_shape(statement, witness),
        "invalid decom Fischlin witness shape"
    );

    let challenge_space: u16 = 1u16 << t_bits;
    let mut rng = StdRng::from_rng(&mut OsRng).expect("failed to seed StdRng from OsRng");
    let g_point = g();
    let h_point = h();
    let m = statement.pedvss.len();

    // Derive lambdas once from the statement — same across all rounds.
    let lambdas = derive_lambdas(statement);

    loop {
        let mut states = Vec::with_capacity(rho);
        let mut first_messages = Vec::with_capacity(rho);

        for _round_index in 0..rho {
            let mut r_a = Vec::with_capacity(m);
            let mut r_b = Vec::with_capacity(m);
            let mut g_scalar = Scalar::ZERO;
            let mut h_scalar = Scalar::ZERO;

            for k in 0..m {
                let r_a_k = Scalar::random(&mut rng);
                let r_b_k = Scalar::random(&mut rng);
                g_scalar += lambdas[k] * r_a_k;
                h_scalar += lambdas[k] * r_b_k;
                r_a.push(r_a_k);
                r_b.push(r_b_k);
            }

            let r_omega = Scalar::random(&mut rng);
            let r_r = Scalar::random(&mut rng);
            g_scalar += lambdas[m] * r_omega;
            h_scalar += lambdas[m] * r_r;

            let t =
                RistrettoPoint::vartime_multiscalar_mul([g_scalar, h_scalar], [g_point, h_point]);

            first_messages.push(t);
            states.push(RoundState {
                t,
                r_a,
                r_b,
                r_omega,
                r_r,
            });
        }

        let common_h = hash_statement_and_first_messages(statement, &first_messages);

        let mut rounds = Vec::with_capacity(rho);
        let mut all_found = true;

        for (i, state) in states.into_iter().enumerate() {
            let mut z_a = state.r_a.clone();
            let mut z_b = state.r_b.clone();
            let mut z_omega = state.r_omega;
            let mut z_r = state.r_r;

            let score_prefix = score_hasher_prefix(&common_h, i, &state.t);
            let mut found_e = None;

            for e in 0..challenge_space {
                let score =
                    fischlin_score_u32_from_prefix(&score_prefix, e, &z_a, &z_b, &z_omega, &z_r);

                if score_has_b_leading_zero_bits(score, b) {
                    found_e = Some(e);
                    break;
                }

                if e + 1 != challenge_space {
                    for k in 0..statement.pedvss.len() {
                        z_a[k] += witness.a[k];
                        z_b[k] += witness.b[k];
                    }
                    z_omega += witness.omega;
                    z_r += witness.r;
                }
            }

            match found_e {
                Some(e) => rounds.push(DecomFischlinRound {
                    t: state.t,
                    e,
                    z_a,
                    z_b,
                    z_omega,
                    z_r,
                }),
                None => {
                    z_a.zeroize();
                    z_b.zeroize();
                    z_omega.zeroize();
                    z_r.zeroize();
                    all_found = false;
                    break;
                }
            }
        }

        if all_found {
            return DecomFischlinProof { rounds };
        }
    }
}

pub fn verify_fischlin_with_params(
    statement: &DecomStatement,
    proof: &DecomFischlinProof,
    rho: usize,
    b: usize,
    t_bits: usize,
) -> bool {
    if rho == 0 || b == 0 || b > 31 || t_bits == 0 || t_bits > 15 || b > t_bits {
        return false;
    }
    if proof.rounds.len() != rho {
        return false;
    }
    if !validate_statement_proof_shape(statement, proof) {
        return false;
    }

    let challenge_bound = 1u32 << t_bits;
    let common_h = hash_statement_and_round_ts(statement, &proof.rounds);
    let g_point = g();
    let h_point = h();

    let pedvss_len = statement.pedvss.len();

    // Derive lambdas once from the statement, then precompute the combined public point.
    // Each round reduces to a 4-term MSM: G, H, T, pedvss_star.
    let lambdas = derive_lambdas(statement);
    let pedvss_star = precompute_pedvss_star(statement, &lambdas);

    for (i, round) in proof.rounds.iter().enumerate() {
        if (round.e as u32) >= challenge_bound {
            return false;
        }

        let score = fischlin_score_u32(
            &common_h,
            i,
            &round.t,
            round.e,
            &round.z_a,
            &round.z_b,
            &round.z_omega,
            &round.z_r,
        );

        if !score_has_b_leading_zero_bits(score, b) {
            return false;
        }

        let combined_z_a = round
            .z_a
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (z, l)| acc + z * l)
            + lambdas[pedvss_len] * round.z_omega;

        let combined_z_b = round
            .z_b
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (z, l)| acc + z * l)
            + lambdas[pedvss_len] * round.z_r;

        let e_scalar = Scalar::from(round.e as u64);

        let check = RistrettoPoint::vartime_multiscalar_mul(
            [combined_z_a, combined_z_b, -Scalar::ONE, -e_scalar],
            [g_point, h_point, round.t, pedvss_star],
        );

        if check != RistrettoPoint::identity() {
            return false;
        }
    }

    true
}

#[derive(Clone, Debug)]
pub struct FischlinDecomScheme;

impl DecomProofScheme for FischlinDecomScheme {
    type Statement = DecomStatement;
    type Witness = DecomWitness;
    type Proof = DecomFischlinProof;
    type Params = FischlinDecomProofParams;

    fn prove(
        params: &Self::Params,
        statement: &Self::Statement,
        witness: &Self::Witness,
    ) -> Self::Proof {
        prove_fischlin_with_params(statement, witness, params.rho, params.b, params.t_bits)
    }

    fn verify(params: &Self::Params, statement: &Self::Statement, proof: &Self::Proof) -> bool {
        verify_fischlin_with_params(statement, proof, params.rho, params.b, params.t_bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    fn params() -> FischlinDecomProofParams {
        FischlinDecomProofParams {
            rho: 4,
            b: 4,
            t_bits: 9,
        }
    }

    fn sample_witness_and_statement() -> (DecomWitness, DecomStatement) {
        let mut rng = thread_rng();
        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };
        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };
        (witness, statement)
    }

    #[test]
    fn decom_fischlin_roundtrip() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);

        assert!(verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_for_wrong_statement() {
        let (witness, statement) = sample_witness_and_statement();

        let mut rng = thread_rng();
        let mut wrong_pedvss = statement.pedvss.clone();
        wrong_pedvss[2] =
            PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng));
        let wrong_statement = DecomStatement {
            pedvss: wrong_pedvss,
            d: statement.d.clone(),
        };

        let p = params();
        let proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);

        assert!(!verify_fischlin_with_params(
            &wrong_statement,
            &proof,
            p.rho,
            p.b,
            p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_t1_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].t += g();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_e_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].e ^= 1;

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_z_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].z_a[1] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_round_removed() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds.pop();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_vector_length_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].z_b.pop();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }
}
