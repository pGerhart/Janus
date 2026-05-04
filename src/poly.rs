use curve25519_dalek::scalar::Scalar;
use rand_core::{CryptoRng, RngCore};

pub fn eval_poly_at(coeffs: &[Scalar], x: Scalar) -> Scalar {
    coeffs.iter().rev().fold(Scalar::ZERO, |acc, a| acc * x + a)
}

pub fn eval_poly_on_1_to_n(coeffs: &[Scalar], n: usize) -> Vec<Scalar> {
    let mut out = Vec::with_capacity(n);
    for j in 1..=n {
        out.push(eval_poly_at(coeffs, Scalar::from(j as u64)));
    }
    out
}

pub fn sample_random_polynomial_with_constant<R: RngCore + CryptoRng>(
    rng: &mut R,
    degree: usize,
    constant: Scalar,
) -> Vec<Scalar> {
    let mut coeffs = Vec::with_capacity(degree + 1);
    coeffs.push(constant);
    for _ in 0..degree {
        coeffs.push(Scalar::random(rng));
    }
    coeffs
}
