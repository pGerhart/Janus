//! Identifiable-abort complaints, shared by both protocols. A receiver whose
//! share does not open the dealer's commitment publishes a signed report; the
//! decryption proof lets every other party re-derive the same verdict.

use crate::encryption::EncryptedShare;
use crate::encryption::open_share_with_shared;
use crate::encryption::proofs::{DecryptionProof, verify_decryption};
use crate::party::Parties;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A signed complaint that a dealer's share does not open its commitment.
/// `shared` is the reporter's DH value, and `proof` binds it to the reporter's
/// encryption key so anyone can recompute the opening.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbortReport {
    pub reporter_idx: usize,
    pub accused_idx: usize,
    pub s_ji: Scalar,
    pub r_ji: Scalar,
    pub shared: RistrettoPoint,
    pub proof: DecryptionProof,
    pub signature: Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortVerdict {
    DealerGuilty {
        dealer_idx: usize,
        reporter_idx: usize,
    },
    ReporterGuilty {
        reporter_idx: usize,
    },
    InvalidReport,
}

impl AbortReport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        reporter_idx: usize,
        accused_idx: usize,
        s_ji: Scalar,
        r_ji: Scalar,
        shared: RistrettoPoint,
        proof: DecryptionProof,
        signing_key: &SigningKey,
    ) -> Self {
        let mut report = Self {
            reporter_idx,
            accused_idx,
            s_ji,
            r_ji,
            shared,
            proof,
            signature: Signature::from_bytes(&[0u8; 64]),
        };
        report.sign(signing_key);
        report
    }

    fn signing_bytes(&self) -> Vec<u8> {
        let mut tmp = self.clone();
        tmp.signature = Signature::from_bytes(&[0u8; 64]);
        bincode::serialize(&tmp).expect("serialization for signing failed")
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        self.signature = sk.sign(&self.signing_bytes());
    }

    pub fn verify_signature(&self, pk: &VerifyingKey) -> bool {
        pk.verify_strict(&self.signing_bytes(), &self.signature)
            .is_ok()
    }

    pub fn serialized_len(&self) -> usize {
        bincode::serialize(self)
            .expect("abort report serialization failed")
            .len()
    }
}

/// The check every party runs on a published complaint. `opening_ok` tests the
/// recomputed opening against the accused's commitment for the reporter. The
/// reporter cannot forge `shared`, so its self-reported opening is not trusted.
pub fn verify_report_core(
    parties: &Parties,
    u: &RistrettoPoint,
    reporter_share: Option<&EncryptedShare>,
    report: &AbortReport,
    opening_ok: impl Fn(Scalar, Scalar) -> bool,
) -> AbortVerdict {
    if !report.verify_signature(parties.sig_pk(report.reporter_idx)) {
        return AbortVerdict::InvalidReport;
    }
    if !verify_decryption(
        parties.enc_pk(report.reporter_idx),
        u,
        &report.shared,
        &report.proof,
    ) {
        return AbortVerdict::ReporterGuilty {
            reporter_idx: report.reporter_idx,
        };
    }
    let share = match reporter_share {
        Some(share) => share,
        None => {
            return AbortVerdict::DealerGuilty {
                dealer_idx: report.accused_idx,
                reporter_idx: report.reporter_idx,
            };
        }
    };
    let (s_ji, r_ji) = open_share_with_shared(&report.shared, share);
    if opening_ok(s_ji, r_ji) {
        AbortVerdict::ReporterGuilty {
            reporter_idx: report.reporter_idx,
        }
    } else {
        AbortVerdict::DealerGuilty {
            dealer_idx: report.accused_idx,
            reporter_idx: report.reporter_idx,
        }
    }
}
