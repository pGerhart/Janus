#![forbid(unsafe_code)]

// Protocol code.
pub mod abort;
pub mod error;
pub mod one_round;
pub mod party;
pub mod two_round;
pub mod wire;

pub use error::{DkgOutputError, TwoRoundDkgError, WireError};

// Cryptographic modules.
pub mod encryption;
pub mod one_round_proofs;
pub mod two_round_proofs;

// Small shared primitives, re-exported at the crate root so existing paths hold.
pub mod primitives;
pub use primitives::{group, pedersen, poly, transcript};

use crate::pedersen::PedersenCommitment;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use zeroize::ZeroizeOnDrop;

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct DkgOutput {
    pub idx: usize,
    pub secret_share: Scalar,
    pub blinding_share: Scalar,
    #[zeroize(skip)]
    pub public_key: RistrettoPoint,
    #[zeroize(skip)]
    pub partial_verification_keys: Vec<PedersenCommitment>,
}

#[derive(Clone, Debug)]
pub struct DkgParams {
    pub t: usize,
    pub n: usize,
}
