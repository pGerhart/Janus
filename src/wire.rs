//! Canonical byte encodings. Fixed-length point and scalar codecs, and the
//! serialization used for message signatures.

pub use crate::error::WireError;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Serialize;
use serde::de::DeserializeOwned;

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

/// The bytes signed over a message. `domain` separates the message types, so a
/// signature on one type is never valid on another. Pass the signed fields only.
pub fn signing_bytes<T: Serialize>(domain: &[u8], fields: &T) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + 8);
    out.extend_from_slice(&(domain.len() as u64).to_le_bytes());
    out.extend_from_slice(domain);
    out.extend_from_slice(
        &postcard::to_allocvec(fields).expect("serialization for signing failed"),
    );
    out
}

/// Signed bytes built from a payload that is already encoded.
fn signing_bytes_of_payload(domain: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + 8 + payload.len());
    out.extend_from_slice(&(domain.len() as u64).to_le_bytes());
    out.extend_from_slice(domain);
    out.extend_from_slice(payload);
    out
}

/// Wire layout of a signed message: `payload_len || payload || signature`.
/// Keeping the payload contiguous lets a receiver authenticate the bytes it was
/// handed, so attribution never depends on re-deriving the encoding.
pub fn seal<T: Serialize>(fields: &T, signature: &Signature) -> Vec<u8> {
    let payload = postcard::to_allocvec(fields).expect("serialization for wire failed");
    let mut out = Vec::with_capacity(8 + payload.len() + 64);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&signature.to_bytes());
    out
}

/// Splits a wire message into its payload and signature without decoding either.
pub fn split(wire: &[u8]) -> Result<(&[u8], Signature), WireError> {
    if wire.len() < 8 {
        return Err(WireError::MalformedMessage);
    }
    let len = u64::from_le_bytes(
        wire[..8]
            .try_into()
            .map_err(|_| WireError::MalformedMessage)?,
    ) as usize;
    let end = 8usize.checked_add(len).ok_or(WireError::MalformedMessage)?;
    if wire.len() != end + 64 {
        return Err(WireError::MalformedMessage);
    }
    let sig_bytes: [u8; 64] = wire[end..]
        .try_into()
        .map_err(|_| WireError::MalformedMessage)?;
    Ok((&wire[8..end], Signature::from_bytes(&sig_bytes)))
}

/// Decodes a wire message without authenticating it, and hands back the payload
/// slice so the caller can look up the claimed sender's key and then verify.
pub fn open_unverified<T: DeserializeOwned>(
    wire: &[u8],
) -> Result<(T, &[u8], Signature), WireError> {
    let (payload, signature) = split(wire)?;
    let fields = postcard::from_bytes(payload).map_err(|_| WireError::MalformedMessage)?;
    Ok((fields, payload, signature))
}

/// Checks a signature over the payload bytes as received. No re-encoding happens
/// here, so attribution does not depend on reproducing the sender's encoding.
pub fn verify_payload(
    domain: &[u8],
    payload: &[u8],
    signature: &Signature,
    pk: &VerifyingKey,
) -> Result<(), WireError> {
    pk.verify_strict(&signing_bytes_of_payload(domain, payload), signature)
        .map_err(|_| WireError::BadSignature)
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
