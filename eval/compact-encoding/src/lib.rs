//! Fischlin well-formedness proof with the first-round commitments left off the
//! wire, rebuilt by the verifier from the challenge and the responses. Same
//! relation, same prover, same transcript, so it exists to price the encoding.

use curve25519_dalek::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
    traits::Identity,
};
use janus::group::{g_mul_scalar, h_mul_scalar};
use janus::one_round_proofs::polyproof_fischlin::{
    FischlinProofParams, common_h, fischlin_score_u32_from_prefix, prove_fischlin_with_params,
    score_has_b_leading_zero_bits, score_hasher_prefix,
};
use janus::one_round_proofs::{PolyProofScheme, PolyWellFormedStatement, PolyWellFormedWitness};
use janus::poly::eval_poly_at;
use serde::{Deserialize, Serialize};

/// A round without its first-round commitments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactRound {
    pub e: u16,
    pub z_coeffs: Vec<Scalar>,
    pub z_blindings: Vec<Scalar>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactFischlinProof {
    pub rounds: Vec<CompactRound>,
}

/// A full-width scalar would cost about five times as much here, and the
/// challenge is public, so branching on its bits is fine.
fn mul_small(p: &RistrettoPoint, e: u16) -> RistrettoPoint {
    if e == 0 {
        return RistrettoPoint::identity();
    }
    let mut acc = RistrettoPoint::identity();
    for i in (0..(16 - e.leading_zeros() as usize)).rev() {
        acc = acc + acc;
        if (e >> i) & 1 == 1 {
            acc += p;
        }
    }
    acc
}

/// The commitments the prover sent in the verbose encoding, rebuilt from what is
/// on the wire: `T_0 = g^{z_0} f0^{-e}` and `T_j = g^{z(x_j)} h^{z_b,j} com_j^{-e}`.
fn recompute_round(
    statement: &PolyWellFormedStatement,
    round: &CompactRound,
) -> Vec<RistrettoPoint> {
    let n = statement.x_points.len();
    let mut t = Vec::with_capacity(n + 1);
    t.push(g_mul_scalar(round.z_coeffs[0]) - mul_small(&statement.f0_commitment, round.e));
    for j in 0..n {
        let zx = eval_poly_at(&round.z_coeffs, statement.x_points[j]);
        t.push(
            g_mul_scalar(zx) + h_mul_scalar(round.z_blindings[j])
                - mul_small(statement.commitments[j].point(), round.e),
        );
    }
    t
}

pub fn prove_compact(
    statement: &PolyWellFormedStatement,
    witness: &PolyWellFormedWitness,
    rho: usize,
    b: usize,
    t_bits: usize,
) -> CompactFischlinProof {
    let full = prove_fischlin_with_params(statement, witness, rho, b, t_bits);
    CompactFischlinProof {
        rounds: full
            .rounds
            .into_iter()
            .map(|r| CompactRound {
                e: r.e,
                z_coeffs: r.z_coeffs,
                z_blindings: r.z_blindings,
            })
            .collect(),
    }
}

/// Rebuilding is the check: the commitments are what the equations define, so a
/// wrong response yields other points and a score that fails.
pub fn verify_compact(
    statement: &PolyWellFormedStatement,
    proof: &CompactFischlinProof,
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
            || (round.e as u32) >= (1u32 << t_bits)
        {
            return false;
        }
    }

    let rebuilt: Vec<Vec<CompressedRistretto>> = proof
        .rounds
        .iter()
        .map(|r| {
            recompute_round(statement, r)
                .iter()
                .map(|p| p.compress())
                .collect()
        })
        .collect();

    let per_round: Vec<&[CompressedRistretto]> = rebuilt.iter().map(|v| v.as_slice()).collect();
    let ch = common_h(statement, &per_round);

    for (i, round) in proof.rounds.iter().enumerate() {
        let prefix = score_hasher_prefix(&ch, i, &rebuilt[i]);
        let score =
            fischlin_score_u32_from_prefix(&prefix, round.e, &round.z_coeffs, &round.z_blindings);
        if !score_has_b_leading_zero_bits(score, b) {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug)]
pub struct CompactFischlinPolyProof;

impl PolyProofScheme for CompactFischlinPolyProof {
    type Proof = CompactFischlinProof;
    type Params = FischlinProofParams;

    fn prove(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof {
        prove_compact(statement, witness, params.rho, params.b, params.t_bits)
    }

    fn verify(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool {
        verify_compact(statement, proof, params.rho, params.b, params.t_bits)
    }
}
