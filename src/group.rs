use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::{constants::RISTRETTO_BASEPOINT_POINT, scalar::Scalar};
use sha2::Sha512;
use std::sync::OnceLock;

pub fn g() -> RistrettoPoint {
    RISTRETTO_BASEPOINT_POINT
}

pub fn h() -> RistrettoPoint {
    static H: OnceLock<RistrettoPoint> = OnceLock::new();
    *H.get_or_init(|| RistrettoPoint::hash_from_bytes::<Sha512>(b"poly-commit-proof:h"))
}

pub fn g_mul_scalar(s: Scalar) -> RistrettoPoint {
    g() * s
}
