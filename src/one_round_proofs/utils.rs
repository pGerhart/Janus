use crate::group::{g, h};
use crate::poly::eval_poly_at;
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};

use super::*;

// One Sigma round to fold. Weights are supplied separately, shared across rounds.
pub struct RoundView<'a> {
    pub t_commitments: &'a [RistrettoPoint],
    pub challenge: Scalar,
    pub z_coeffs: &'a [Scalar],
    pub z_blindings: &'a [Scalar],
}

// Folds equation j of round i with weight gammas[i] * deltas[j] into one
// multiscalar check. Both weight vectors are derived after the whole proof.
pub fn batched_point_check_rank1(
    statement: &PolyWellFormedStatement,
    rounds: &[RoundView],
    deltas: &[Scalar],
    gammas: &[Scalar],
) -> bool {
    let n = statement.x_points.len();
    let degree = statement.degree;
    if deltas.len() != n + 1 || gammas.len() != rounds.len() {
        return false;
    }
    for r in rounds {
        if r.t_commitments.len() != n + 1
            || r.z_blindings.len() != n
            || r.z_coeffs.len() != degree + 1
        {
            return false;
        }
    }

    // Fold the rounds coefficient-wise, so the multipoint evaluation below runs once.
    let mut zhat = vec![Scalar::ZERO; degree + 1];
    let mut bhat = vec![Scalar::ZERO; n];
    let mut e_fold = Scalar::ZERO;
    for (r, gamma) in rounds.iter().zip(gammas) {
        for (z, zc) in zhat.iter_mut().zip(r.z_coeffs) {
            *z += *gamma * *zc;
        }
        for (bacc, zb) in bhat.iter_mut().zip(r.z_blindings) {
            *bacc += *gamma * *zb;
        }
        e_fold += *gamma * r.challenge;
    }

    let mut g_scalar = deltas[0] * zhat[0];
    let mut h_scalar = Scalar::ZERO;
    for j in 0..n {
        let zx = eval_poly_at(&zhat, statement.x_points[j]);
        g_scalar += deltas[j + 1] * zx;
        h_scalar += deltas[j + 1] * bhat[j];
    }

    let mut scalars: Vec<Scalar> = Vec::with_capacity(rounds.len() * (n + 1) + n + 3);
    let mut points: Vec<RistrettoPoint> = Vec::with_capacity(rounds.len() * (n + 1) + n + 3);

    for (r, gamma) in rounds.iter().zip(gammas) {
        for (t, delta) in r.t_commitments.iter().zip(deltas) {
            scalars.push(*gamma * *delta);
            points.push(*t);
        }
    }

    scalars.push(deltas[0] * e_fold);
    points.push(statement.f0_commitment);
    for j in 0..n {
        scalars.push(deltas[j + 1] * e_fold);
        points.push(*statement.commitments[j].point());
    }
    scalars.push(-g_scalar);
    points.push(g());
    scalars.push(-h_scalar);
    points.push(h());

    RistrettoPoint::vartime_multiscalar_mul(scalars, points) == RistrettoPoint::identity()
}

pub fn append_statement_to_transcript(
    transcript: &mut Transcript,
    statement: &PolyWellFormedStatement,
) {
    transcript.append_message(b"dom-sep", b"poly-well-formedness");
    transcript.append_u64(b"degree", statement.degree as u64);
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
