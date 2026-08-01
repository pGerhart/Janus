use crate::group::{g_mul_scalar, h_mul_scalar};
use crate::transcript::*;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use merlin::Transcript;
use utils::*;

use rand::rngs::SysRng;
use rand_core::UnwrapErr;
use serde::{Deserialize, Serialize};

pub mod polyproof_fischlin;
pub mod polyproof_schnorr;
pub(crate) mod utils;

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
