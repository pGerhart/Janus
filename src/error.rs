use crate::encryption::proofs::DecryptionProof;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    NotOnCurve,
    NonCanonicalScalar,
    MalformedMessage,
    BadSignature,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::NotOnCurve => write!(f, "not a canonical Ristretto point"),
            WireError::NonCanonicalScalar => write!(f, "scalar is not canonically reduced"),
            WireError::MalformedMessage => write!(f, "wire message is malformed"),
            WireError::BadSignature => write!(f, "wire message signature does not verify"),
        }
    }
}

impl std::error::Error for WireError {}

#[derive(Clone, Debug)]
pub enum DkgOutputError {
    InvalidParameters,
    InvalidBatchProof {
        dealer_idx: usize,
    },
    InvalidWire,
    MissingCiphertext {
        dealer_idx: usize,
        receiver_idx: usize,
    },
    InvalidEncryptionProof {
        dealer_idx: usize,
    },
    InvalidDecryptionOpening {
        dealer_idx: usize,
        receiver_idx: usize,
        s_ji: Scalar,
        r_ji: Scalar,
        pi_i: Box<(RistrettoPoint, DecryptionProof)>,
    },
    InvalidSignature {
        dealer_idx: usize,
    },
}

impl fmt::Display for DkgOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DkgOutputError::InvalidParameters => write!(f, "invalid DKG parameters"),
            DkgOutputError::InvalidWire => {
                write!(f, "a received message is malformed or not authentic")
            }
            DkgOutputError::InvalidBatchProof { dealer_idx } => {
                write!(f, "well-formedness proof of dealer {dealer_idx} is invalid")
            }
            DkgOutputError::MissingCiphertext {
                dealer_idx,
                receiver_idx,
            } => write!(
                f,
                "dealer {dealer_idx} sent no ciphertext for party {receiver_idx}"
            ),
            DkgOutputError::InvalidEncryptionProof { dealer_idx } => {
                write!(f, "encryption proof of dealer {dealer_idx} is invalid")
            }
            DkgOutputError::InvalidDecryptionOpening {
                dealer_idx,
                receiver_idx,
                ..
            } => write!(
                f,
                "share of dealer {dealer_idx} for party {receiver_idx} does not open its commitment"
            ),
            DkgOutputError::InvalidSignature { dealer_idx } => {
                write!(f, "signature of dealer {dealer_idx} is invalid")
            }
        }
    }
}

impl std::error::Error for DkgOutputError {}

#[derive(Clone, Debug)]
pub enum TwoRoundDkgError {
    InvalidParameters,
    InvalidSignature {
        dealer_idx: usize,
    },
    MissingCiphertext {
        dealer_idx: usize,
        receiver_idx: usize,
    },
    InvalidDecryptionOpening {
        dealer_idx: usize,
        receiver_idx: usize,
        s_ji: Scalar,
        sprime_ji: Scalar,
        pi_i: Box<(RistrettoPoint, DecryptionProof)>,
    },
    InvalidEncryptionProof {
        dealer_idx: usize,
    },
    InvalidDecomProof {
        dealer_idx: usize,
    },
    InvalidPkProof {
        dealer_idx: usize,
    },
    InvalidComEqProof {
        dealer_idx: usize,
    },
    InvalidPedVssLength {
        dealer_idx: usize,
    },
    MissingRound2Message {
        dealer_idx: usize,
    },
}

impl fmt::Display for TwoRoundDkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwoRoundDkgError::InvalidParameters => write!(f, "invalid DKG parameters"),
            TwoRoundDkgError::InvalidSignature { dealer_idx } => {
                write!(f, "signature of dealer {dealer_idx} is invalid")
            }
            TwoRoundDkgError::MissingCiphertext {
                dealer_idx,
                receiver_idx,
            } => write!(
                f,
                "dealer {dealer_idx} sent no ciphertext for party {receiver_idx}"
            ),
            TwoRoundDkgError::InvalidDecryptionOpening {
                dealer_idx,
                receiver_idx,
                ..
            } => write!(
                f,
                "share of dealer {dealer_idx} for party {receiver_idx} does not open its commitment"
            ),
            TwoRoundDkgError::InvalidEncryptionProof { dealer_idx } => {
                write!(f, "encryption proof of dealer {dealer_idx} is invalid")
            }
            TwoRoundDkgError::InvalidDecomProof { dealer_idx } => {
                write!(f, "decomposition proof of dealer {dealer_idx} is invalid")
            }
            TwoRoundDkgError::InvalidPkProof { dealer_idx } => {
                write!(f, "public key proof of dealer {dealer_idx} is invalid")
            }
            TwoRoundDkgError::InvalidComEqProof { dealer_idx } => {
                write!(
                    f,
                    "commitment equality proof of dealer {dealer_idx} is invalid"
                )
            }
            TwoRoundDkgError::InvalidPedVssLength { dealer_idx } => {
                write!(
                    f,
                    "dealer {dealer_idx} sent a VSS vector of the wrong length"
                )
            }
            TwoRoundDkgError::MissingRound2Message { dealer_idx } => {
                write!(f, "dealer {dealer_idx} sent no round 2 message")
            }
        }
    }
}

impl std::error::Error for TwoRoundDkgError {}
