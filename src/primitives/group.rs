use curve25519_dalek::ristretto::{RistrettoBasepointTable, RistrettoPoint};
use curve25519_dalek::{
    constants::{RISTRETTO_BASEPOINT_POINT, RISTRETTO_BASEPOINT_TABLE},
    scalar::Scalar,
};
use sha2::Sha512;
use std::sync::OnceLock;

pub fn g() -> RistrettoPoint {
    RISTRETTO_BASEPOINT_POINT
}

pub fn h() -> RistrettoPoint {
    h_table().basepoint()
}

// Precomputed table so every h^s is a fixed-base multiplication. Tables are
// constant-time, so they stay safe on secret scalars.
fn h_table() -> &'static RistrettoBasepointTable {
    static H: OnceLock<RistrettoBasepointTable> = OnceLock::new();
    H.get_or_init(|| {
        RistrettoBasepointTable::create(&RistrettoPoint::hash_from_bytes::<Sha512>(
            b"poly-commit-proof:h",
        ))
    })
}

#[inline]
pub fn g_mul_scalar(s: Scalar) -> RistrettoPoint {
    RISTRETTO_BASEPOINT_TABLE * &s
}

#[inline]
pub fn h_mul_scalar(s: Scalar) -> RistrettoPoint {
    h_table() * &s
}
