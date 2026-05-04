use crate::group::{g, h};
use crate::transcript::*;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use helpers::*;
use merlin::Transcript;

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

pub mod helpers;
pub mod polyproof_bulletproof;
pub mod polyproof_fischlin;
pub mod polyproof_schnorr;

pub use polyproof_bulletproof::{
    BulletproofPolyProof, PolyWellFormedBulletproofParams as BulletproofProofParams,
};
pub use polyproof_fischlin::{FischlinPolyProof, FischlinProofParams};
pub use polyproof_schnorr::SchnorrPolyProof;
pub use polyproof_schnorr::{PolyWellFormedStatement, PolyWellFormedWitness};

pub trait PolyProofScheme {
    type Proof: Clone + std::fmt::Debug;
    type Params: Clone + std::fmt::Debug;

    fn prove(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof;

    fn verify(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool;
}
