use crate::pedersen::PedersenCommitment;
use curve25519_dalek::traits::VartimeMultiscalarMul;

use super::*;

pub fn append_statement_to_transcript(
    transcript: &mut Transcript,
    statement: &PolyWellFormedStatement,
) {
    transcript.append_message(b"dom-sep", b"poly-well-formedness-batched");
    transcript.append_point(b"f0_commitment", &statement.f0_commitment);

    transcript.append_u64(b"n", statement.x_points.len() as u64);

    for (i, x) in statement.x_points.iter().enumerate() {
        transcript.append_message(b"x_index", &(i as u64).to_le_bytes());
        transcript.append_scalar(b"x", x);
    }

    for (i, c) in statement.commitments.iter().enumerate() {
        transcript.append_message(b"c_index", &(i as u64).to_le_bytes());
        transcript.append_point(b"commitment", c.point());
    }
}

pub fn derive_lambdas(transcript: &mut Transcript, n: usize) -> Vec<Scalar> {
    let mut lambdas = Vec::with_capacity(n + 1);
    for i in 0..=n {
        transcript.append_message(b"lambda_index", &(i as u64).to_le_bytes());
        lambdas.push(transcript.challenge_scalar(b"lambda"));
    }
    lambdas
}

pub fn compute_mus(x_points: &[Scalar], lambdas: &[Scalar], degree: usize) -> Vec<Scalar> {
    let n = x_points.len();
    assert_eq!(lambdas.len(), n + 1);

    let mut mus = vec![Scalar::ZERO; degree + 1];

    mus[0] = lambdas[0];
    for i in 0..n {
        mus[0] += lambdas[i + 1];
    }

    if degree >= 1 {
        // x_pows[i] holds x_points[i]^j, updated incrementally to avoid O(n·d²) recomputation
        let mut x_pows = x_points.to_vec();
        for j in 1..=degree {
            let mut acc = Scalar::ZERO;
            for i in 0..n {
                acc += lambdas[i + 1] * x_pows[i];
            }
            mus[j] = acc;
            if j < degree {
                for i in 0..n {
                    x_pows[i] *= x_points[i];
                }
            }
        }
    }

    mus
}

pub fn combine_publics(
    f0_commitment: &RistrettoPoint,
    commitments: &[PedersenCommitment],
    lambdas: &[Scalar],
) -> RistrettoPoint {
    RistrettoPoint::vartime_multiscalar_mul(
        lambdas.iter().copied(),
        std::iter::once(*f0_commitment).chain(commitments.iter().map(|c| *c.point())),
    )
}
