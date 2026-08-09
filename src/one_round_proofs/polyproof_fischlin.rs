use super::*;
use crate::poly::eval_poly_at;
use curve25519_dalek::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};
use rand::rngs::SysRng;
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;
use rand_core::UnwrapErr;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedFischlinRound {
    pub t_commitments: Vec<RistrettoPoint>, // n+1 first-round commitments: T_0, T_1, ..., T_n
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
    t_commitments: Vec<RistrettoPoint>,
    r_coeffs: Vec<Scalar>,
    r_blindings: Vec<Scalar>,
}

// T_0 for f0 = g^{a_0}, then T_j = g^{f_r(x_j)} h^{r_j} per commitment.
fn round_first_commitments(
    statement: &PolyWellFormedStatement,
    r_coeffs: &[Scalar],
    r_blindings: &[Scalar],
) -> Vec<RistrettoPoint> {
    let n = statement.x_points.len();
    let mut t = Vec::with_capacity(n + 1);
    t.push(g_mul_scalar(r_coeffs[0]));
    for j in 0..n {
        let e = eval_poly_at(r_coeffs, statement.x_points[j]);
        t.push(g_mul_scalar(e) + h_mul_scalar(r_blindings[j]));
    }
    t
}

#[inline]
fn statement_common_h_prefix(statement: &PolyWellFormedStatement) -> Sha512 {
    let mut h = Sha512::new();
    h.update(b"poly-well-formedness-fischlin-common-h-v1");
    h.update((statement.degree as u64).to_le_bytes());

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

// Binds the statement and every round's first-round commitments. Takes them
// already compressed, so each point is compressed once per verify.
/// Hash binding the statement and every round's first-round commitments.
/// Public so an alternative encoding can reproduce the transcript exactly.
pub fn common_h(
    statement: &PolyWellFormedStatement,
    per_round: &[&[CompressedRistretto]],
) -> [u8; 64] {
    let mut h = statement_common_h_prefix(statement);
    h.update((per_round.len() as u64).to_le_bytes());
    for tc in per_round {
        h.update((tc.len() as u64).to_le_bytes());
        for t in *tc {
            h.update(t.as_bytes());
        }
    }
    let digest = h.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

#[inline]
/// Per-round prefix of the score hash. Public for the same reason as `common_h`.
pub fn score_hasher_prefix(
    common_h: &[u8; 64],
    round_index: usize,
    t_commitments: &[CompressedRistretto],
) -> Sha512 {
    let mut h = Sha512::new();
    h.update(b"poly-well-formedness-fischlin-score-v1");
    h.update(common_h);
    h.update((round_index as u64).to_le_bytes());
    h.update((t_commitments.len() as u64).to_le_bytes());
    for t in t_commitments {
        h.update(t.as_bytes());
    }
    h
}

pub fn fischlin_score_u32(
    common_h: &[u8; 64],
    round_index: usize,
    t_commitments: &[CompressedRistretto],
    e: u16,
    z_coeffs: &[Scalar],
    z_blindings: &[Scalar],
) -> u32 {
    let base = score_hasher_prefix(common_h, round_index, t_commitments);
    fischlin_score_u32_from_prefix(&base, e, z_coeffs, z_blindings)
}

#[inline]
/// Score of one round given its prefix. Public for the same reason as `common_h`.
pub fn fischlin_score_u32_from_prefix(
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
pub fn sample_scalar<R: rand_core::CryptoRng>(rng: &mut R) -> Scalar {
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
    assert_eq!(statement.x_points.len(), statement.commitments.len());
    assert_eq!(statement.x_points.len(), witness.blindings.len());
    assert_eq!(
        witness.coeffs.len(),
        statement.degree + 1,
        "coeffs length must be degree+1"
    );

    let n = statement.x_points.len();
    let degree = statement.degree;
    let challenge_space: u16 = 1u16 << t_bits;

    // Seed once from OS entropy, then use a fast CSPRNG stream.
    let mut rng = ChaCha20Rng::from_rng(&mut UnwrapErr(SysRng));

    loop {
        // Draw the nonces from the one seeded stream, then do the group work per
        // round in parallel. The rounds are independent once common_h is fixed.
        let nonces: Vec<(Vec<Scalar>, Vec<Scalar>)> = (0..rho)
            .map(|_| {
                let r_coeffs: Vec<Scalar> = (0..=degree).map(|_| sample_scalar(&mut rng)).collect();
                let r_blindings: Vec<Scalar> = (0..n).map(|_| sample_scalar(&mut rng)).collect();
                (r_coeffs, r_blindings)
            })
            .collect();

        let states: Vec<RoundState> = nonces
            .into_par_iter()
            .map(|(r_coeffs, r_blindings)| {
                let t_commitments = round_first_commitments(statement, &r_coeffs, &r_blindings);
                RoundState {
                    t_commitments,
                    r_coeffs,
                    r_blindings,
                }
            })
            .collect();

        let per_round_compressed: Vec<Vec<CompressedRistretto>> = states
            .par_iter()
            .map(|s| s.t_commitments.iter().map(|p| p.compress()).collect())
            .collect();
        let per_round: Vec<&[CompressedRistretto]> =
            per_round_compressed.iter().map(|v| v.as_slice()).collect();
        let ch = common_h(statement, &per_round);

        let found: Vec<Option<PolyWellFormedFischlinRound>> = states
            .par_iter()
            .enumerate()
            .map(|(i, state)| {
                let mut z_coeffs = state.r_coeffs.clone();
                let mut z_blindings = state.r_blindings.clone();

                let score_prefix = score_hasher_prefix(&ch, i, &per_round_compressed[i]);

                for e in 0..challenge_space {
                    let score =
                        fischlin_score_u32_from_prefix(&score_prefix, e, &z_coeffs, &z_blindings);

                    if score_has_b_leading_zero_bits(score, b) {
                        return Some(PolyWellFormedFischlinRound {
                            t_commitments: state.t_commitments.clone(),
                            e,
                            z_coeffs,
                            z_blindings,
                        });
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

                z_coeffs.zeroize();
                z_blindings.zeroize();
                None
            })
            .collect();

        let all_found = found.iter().all(|r| r.is_some());
        let rounds: Vec<PolyWellFormedFischlinRound> = found.into_iter().flatten().collect();

        if all_found {
            return PolyWellFormedFischlinProof { rounds };
        }
    }
}

// Rank-1 fold weights, squeezed from one transcript over the whole proof so a
// prover cannot grind a single round.
fn derive_shared_weights(
    statement: &PolyWellFormedStatement,
    proof: &PolyWellFormedFischlinProof,
    per_round_compressed: &[Vec<CompressedRistretto>],
    rho: usize,
    n: usize,
) -> (Vec<Scalar>, Vec<Scalar>) {
    let mut tr = Transcript::new(b"poly-well-formedness-fischlin-rank1");
    append_statement_to_transcript(&mut tr, statement);
    for (round, tc) in proof.rounds.iter().zip(per_round_compressed) {
        for t in tc {
            tr.append_message(b"t", t.as_bytes());
        }
        tr.append_u64(b"e", round.e as u64);
        for z in &round.z_coeffs {
            tr.append_scalar(b"za", z);
        }
        for z in &round.z_blindings {
            tr.append_scalar(b"zw", z);
        }
    }
    let deltas = (0..=n)
        .map(|i| {
            tr.append_message(b"delta_index", &(i as u64).to_le_bytes());
            tr.challenge_scalar(b"delta")
        })
        .collect();
    let gammas = (0..rho)
        .map(|i| {
            tr.append_message(b"gamma_index", &(i as u64).to_le_bytes());
            tr.challenge_scalar(b"gamma")
        })
        .collect();
    (deltas, gammas)
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
    if statement.commitments.len() != n || proof.rounds.len() != rho {
        return false;
    }
    for round in &proof.rounds {
        if round.z_blindings.len() != n
            || round.z_coeffs.len() != statement.degree + 1
            || round.t_commitments.len() != n + 1
            || (round.e as u32) >= (1u32 << t_bits)
        {
            return false;
        }
    }

    // Compress each first-round commitment once and reuse across the three hashes.
    let per_round_compressed: Vec<Vec<CompressedRistretto>> = proof
        .rounds
        .iter()
        .map(|r| r.t_commitments.iter().map(|p| p.compress()).collect())
        .collect();
    let per_round: Vec<&[CompressedRistretto]> =
        per_round_compressed.iter().map(|v| v.as_slice()).collect();
    let ch = common_h(statement, &per_round);

    for (i, round) in proof.rounds.iter().enumerate() {
        let score = fischlin_score_u32(
            &ch,
            i,
            &per_round_compressed[i],
            round.e,
            &round.z_coeffs,
            &round.z_blindings,
        );
        if !score_has_b_leading_zero_bits(score, b) {
            return false;
        }
    }

    // Fold every round's Sigma equation into one multiscalar check.
    let (deltas, gammas) = derive_shared_weights(statement, proof, &per_round_compressed, rho, n);
    let rounds: Vec<RoundView> = proof
        .rounds
        .iter()
        .map(|r| RoundView {
            t_commitments: &r.t_commitments,
            challenge: Scalar::from(r.e as u64),
            z_coeffs: &r.z_coeffs,
            z_blindings: &r.z_blindings,
        })
        .collect();

    batched_point_check_rank1(statement, &rounds, &deltas, &gammas)
}

pub fn batch_verify_fischlin_with_params(
    statements: &[PolyWellFormedStatement],
    proofs: &[PolyWellFormedFischlinProof],
    rho: usize,
    b: usize,
    t_bits: usize,
) -> bool {
    if statements.len() != proofs.len() {
        return false;
    }
    statements
        .iter()
        .zip(proofs.iter())
        .all(|(s, p)| verify_fischlin_with_params(s, p, rho, b, t_bits))
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
