use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use serde::{Deserialize, Serialize};

use crate::group::{g_mul_scalar, h_mul_scalar};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PedersenCommitment {
    point: RistrettoPoint,
}

impl PedersenCommitment {
    pub fn new(message: Scalar, blinding: Scalar) -> Self {
        let point = g_mul_scalar(message) + h_mul_scalar(blinding);
        Self { point }
    }

    pub fn from_point(point: RistrettoPoint) -> Self {
        Self { point }
    }

    pub fn point(&self) -> &RistrettoPoint {
        &self.point
    }

    pub fn compress(&self) -> [u8; 32] {
        self.point.compress().to_bytes()
    }

    pub fn verify_opening(&self, message: Scalar, blinding: Scalar) -> bool {
        self.matches_opening(message, blinding)
    }

    pub fn matches_opening(&self, value: Scalar, blinding: Scalar) -> bool {
        *self == PedersenCommitment::new(value, blinding)
    }
}
