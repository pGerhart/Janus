//! Canonical byte encodings. Fixed-length point and scalar codecs, and the
//! serialization used for message signatures.

use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    NotOnCurve,
    NonCanonicalScalar,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::NotOnCurve => write!(f, "not a canonical Ristretto point"),
            WireError::NonCanonicalScalar => write!(f, "scalar is not canonically reduced"),
        }
    }
}

impl std::error::Error for WireError {}

pub fn point_to_bytes(p: &RistrettoPoint) -> [u8; 32] {
    p.compress().to_bytes()
}

pub fn point_from_bytes(bytes: &[u8; 32]) -> Result<RistrettoPoint, WireError> {
    CompressedRistretto::from_slice(bytes)
        .map_err(|_| WireError::NotOnCurve)?
        .decompress()
        .ok_or(WireError::NotOnCurve)
}

pub fn scalar_to_bytes(s: &Scalar) -> [u8; 32] {
    s.to_bytes()
}

pub fn scalar_from_bytes(bytes: &[u8; 32]) -> Result<Scalar, WireError> {
    Option::<Scalar>::from(Scalar::from_canonical_bytes(*bytes))
        .ok_or(WireError::NonCanonicalScalar)
}

/// The bytes signed over a message. The caller zeroes the signature field first,
/// so the signature is never part of its own input.
pub fn signing_bytes<T: Serialize>(msg: &T) -> Vec<u8> {
    bincode::serialize(msg).expect("serialization for signing failed")
}

#[test]
fn point_roundtrip() {
    let p = RistrettoPoint::default();
    assert_eq!(point_from_bytes(&point_to_bytes(&p)).unwrap(), p);
}

#[test]
fn scalar_roundtrip() {
    let s = Scalar::from(42u64);
    assert_eq!(scalar_from_bytes(&scalar_to_bytes(&s)).unwrap(), s);
}
