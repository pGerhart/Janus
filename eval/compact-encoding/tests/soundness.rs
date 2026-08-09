// The alternative encoding must accept and reject exactly what the protocol's own
// does, so these mirror the well-formedness tests, including the forgery.

use compact_encoding::{prove_compact, verify_compact};
use curve25519_dalek::scalar::Scalar;
use janus::group::g_mul_scalar;
use janus::one_round_proofs::{PolyWellFormedStatement, PolyWellFormedWitness};
use janus::pedersen::PedersenCommitment;
use janus::poly::eval_poly_at;

fn instance(
    coeffs: Vec<Scalar>,
    xs: Vec<Scalar>,
    blindings: Vec<Scalar>,
) -> (PolyWellFormedStatement, PolyWellFormedWitness) {
    let commitments: Vec<PedersenCommitment> = xs
        .iter()
        .zip(blindings.iter())
        .map(|(x, r)| PedersenCommitment::new(eval_poly_at(&coeffs, *x), *r))
        .collect();
    let degree = coeffs.len() - 1;
    (
        PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g_mul_scalar(coeffs[0]),
            degree,
        },
        PolyWellFormedWitness { coeffs, blindings },
    )
}

fn small() -> (PolyWellFormedStatement, PolyWellFormedWitness) {
    instance(
        vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)],
        vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)],
        vec![Scalar::from(7u64), Scalar::from(11u64), Scalar::from(13u64)],
    )
}

#[test]
fn accepts_an_honest_proof() {
    let (st, wit) = small();
    let p = prove_compact(&st, &wit, 4, 4, 9);
    assert!(verify_compact(&st, &p, 4, 4, 9));
}

// Commitments that lie on no degree-2 polynomial, the case the aggregate-equation
// proof used to accept.
#[test]
fn rejects_commitments_off_the_polynomial() {
    let xs: Vec<Scalar> = (1..=5).map(|i| Scalar::from(i as u64)).collect();
    let vals: Vec<Scalar> = [1u64, 2, 9, 1, 7]
        .iter()
        .map(|v| Scalar::from(*v))
        .collect();
    let blindings: Vec<Scalar> = (0..5).map(|i| Scalar::from(100 + i as u64)).collect();
    let commitments: Vec<PedersenCommitment> = vals
        .iter()
        .zip(blindings.iter())
        .map(|(v, b)| PedersenCommitment::new(*v, *b))
        .collect();
    let a0 = Scalar::from(3u64);
    let st = PolyWellFormedStatement {
        x_points: xs,
        commitments,
        f0_commitment: g_mul_scalar(a0),
        degree: 2,
    };
    let wit = PolyWellFormedWitness {
        coeffs: vec![a0, Scalar::from(5u64), Scalar::from(7u64)],
        blindings,
    };
    let p = prove_compact(&st, &wit, 4, 4, 9);
    assert!(!verify_compact(&st, &p, 4, 4, 9));
}

#[test]
fn rejects_tampered_responses() {
    let (st, wit) = small();

    let mut p = prove_compact(&st, &wit, 5, 4, 9);
    p.rounds[2].z_coeffs[1] += Scalar::ONE;
    assert!(!verify_compact(&st, &p, 5, 4, 9));

    let mut q = prove_compact(&st, &wit, 5, 4, 9);
    q.rounds[3].z_blindings[2] += Scalar::ONE;
    assert!(!verify_compact(&st, &q, 5, 4, 9));

    let mut r = prove_compact(&st, &wit, 5, 4, 9);
    r.rounds[1].e ^= 1;
    assert!(!verify_compact(&st, &r, 5, 4, 9));
}

#[test]
fn rejects_a_modified_statement() {
    let (mut st, wit) = small();
    let p = prove_compact(&st, &wit, 4, 4, 9);
    st.f0_commitment += g_mul_scalar(Scalar::ONE);
    assert!(!verify_compact(&st, &p, 4, 4, 9));
}
