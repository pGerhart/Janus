use crate::group::{g, h};
use crate::transcript::*;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use serde::{Deserialize, Serialize};

use crate::pedersen::PedersenCommitment;
use merlin::Transcript;
use rand_core::{CryptoRng, RngCore};

pub mod comeq_proof;
pub mod decom_proof;
pub mod decom_proof_fischlin;
pub mod pk_proof;

pub trait DecomProofScheme {
    type Statement: Clone + std::fmt::Debug;
    type Witness: Clone + std::fmt::Debug;
    type Proof: Clone + std::fmt::Debug;
    type Params: Clone + std::fmt::Debug;

    fn prove(
        params: &Self::Params,
        statement: &Self::Statement,
        witness: &Self::Witness,
    ) -> Self::Proof;

    fn verify(params: &Self::Params, statement: &Self::Statement, proof: &Self::Proof) -> bool;
}

pub use decom_proof::{
    DecomProof, DecomStatement, DecomWitness, SchnorrDecomProof, SchnorrDecomProofParams,
};
pub use decom_proof_fischlin::{DecomFischlinProof, FischlinDecomProofParams, FischlinDecomScheme};
