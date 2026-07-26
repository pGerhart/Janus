use super::{DkgOutput, DkgParams};
use crate::abort::{AbortReport, AbortVerdict, verify_report_core};
use crate::encryption::proofs::{DecryptionProof, prove_decryption};
use crate::encryption::{BatchEncryptedShares, decrypt_my_shares, encrypt_batch};
use crate::group::g_mul_scalar;
use crate::one_round_proofs::{PolyProofScheme, PolyWellFormedStatement, PolyWellFormedWitness};
use crate::party::{Parties, PartyState};
use crate::pedersen::PedersenCommitment;
use crate::poly::{eval_poly_on_1_to_n, sample_random_polynomial_with_constant};
use zeroize::{ZeroizeOnDrop, Zeroizing};

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::{CryptoRng, RngCore};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DkgInitBroadcast<P> {
    pub dealer_idx: usize,
    pub pedvss: Vec<PedersenCommitment>,
    pub f0_commitment: RistrettoPoint,
    pub proof: P,
    pub encrypted_shares: BatchEncryptedShares,
    pub signature: Signature,
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct DkgInitLocalState {
    pub my_share: Scalar,
    pub my_blinding: Scalar,
}

#[derive(Clone, Debug)]
pub struct DkgInitResult<P> {
    pub broadcast: DkgInitBroadcast<P>,
    pub local: DkgInitLocalState,
}

#[derive(Clone, Debug)]
pub enum DkgOutputError {
    InvalidParameters,
    InvalidBatchProof,
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
impl<P: Clone + Serialize> DkgInitBroadcast<P> {
    pub fn new(
        dealer_idx: usize,
        pedvss: Vec<PedersenCommitment>,
        f0_commitment: RistrettoPoint,
        proof: P,
        encrypted_shares: BatchEncryptedShares,
        signing_key: &SigningKey,
    ) -> Self {
        let mut msg = Self {
            dealer_idx,
            pedvss,
            f0_commitment,
            proof,
            encrypted_shares,
            signature: Signature::from_bytes(&[0u8; 64]),
        };
        msg.sign(signing_key);
        msg
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut tmp = self.clone();
        tmp.signature = Signature::from_bytes(&[0u8; 64]);
        crate::wire::signing_bytes(&tmp)
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        self.signature = sk.sign(&self.signing_bytes());
    }

    pub fn verify(&self, pk: &VerifyingKey) -> bool {
        pk.verify_strict(&self.signing_bytes(), &self.signature)
            .is_ok()
    }
}
#[inline]
fn idx1_to_vec(i: usize) -> usize {
    debug_assert!(i >= 1);
    i - 1
}

#[inline]
fn domain_points_1_to_n(n: usize) -> Vec<Scalar> {
    (1..=n).map(|i| Scalar::from(i as u64)).collect()
}

fn validate_broadcasts<P: Clone + Serialize>(
    n: usize,
    my_idx: usize,
    broadcasts: &[DkgInitBroadcast<P>],
    parties: &Parties,
) -> Result<(), DkgOutputError> {
    if n == 0 || my_idx == 0 || my_idx > n {
        return Err(DkgOutputError::InvalidParameters);
    }
    let mut seen = vec![false; n];
    for msg in broadcasts {
        if msg.dealer_idx == 0 || msg.dealer_idx > n || msg.pedvss.len() != n {
            return Err(DkgOutputError::InvalidParameters);
        }
        if !msg.verify(parties.sig_pk(msg.dealer_idx)) {
            return Err(DkgOutputError::InvalidSignature {
                dealer_idx: msg.dealer_idx,
            });
        }
        let slot = idx1_to_vec(msg.dealer_idx);
        if seen[slot] {
            return Err(DkgOutputError::InvalidParameters);
        }
        seen[slot] = true;
    }
    if seen.iter().any(|b| !*b) {
        return Err(DkgOutputError::InvalidParameters);
    }
    Ok(())
}

fn verify_poly_proofs<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
    broadcasts: &[DkgInitBroadcast<S::Proof>],
) -> Result<(), DkgOutputError>
where
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    // Reuse one statement so the domain and commitment buffer are not
    // reallocated per dealer.
    let mut stmt = PolyWellFormedStatement {
        x_points: domain_points_1_to_n(dkg_params.n),
        commitments: Vec::with_capacity(dkg_params.n),
        f0_commitment: RistrettoPoint::default(),
    };
    for msg in broadcasts {
        stmt.commitments.clear();
        stmt.commitments.extend_from_slice(&msg.pedvss);
        stmt.f0_commitment = msg.f0_commitment;
        if !S::verify(proof_params, &stmt, &msg.proof) {
            return Err(DkgOutputError::InvalidBatchProof);
        }
    }
    Ok(())
}

fn accumulate_decrypted_shares<P: Clone + Serialize>(
    state: &PartyState,
    other_msgs: &[&DkgInitBroadcast<P>],
    decrypted: &[Option<(Scalar, Scalar)>],
    init: (Scalar, Scalar),
) -> Result<(Scalar, Scalar), DkgOutputError> {
    let (mut s_i, mut s_i_blind) = init;
    for (msg, dec) in other_msgs.iter().zip(decrypted.iter()) {
        let (s_ji, r_ji) = dec.ok_or(DkgOutputError::MissingCiphertext {
            dealer_idx: msg.dealer_idx,
            receiver_idx: state.dealer_idx,
        })?;
        if !msg.pedvss[idx1_to_vec(state.dealer_idx)].matches_opening(s_ji, r_ji) {
            let pi_i = prove_decryption(&state.enc_sk, &state.enc_pk, &msg.encrypted_shares.u);
            return Err(DkgOutputError::InvalidDecryptionOpening {
                dealer_idx: msg.dealer_idx,
                receiver_idx: state.dealer_idx,
                s_ji,
                r_ji,
                pi_i: Box::new(pi_i),
            });
        }
        s_i += s_ji;
        s_i_blind += r_ji;
    }
    Ok((s_i, s_i_blind))
}

fn decrypt_and_accumulate<P: Clone + Serialize>(
    state: &PartyState,
    broadcasts: &[DkgInitBroadcast<P>],
    init: (Scalar, Scalar),
) -> Result<(Scalar, Scalar), DkgOutputError> {
    let other_msgs: Vec<_> = broadcasts
        .iter()
        .filter(|m| m.dealer_idx != state.dealer_idx)
        .collect();
    let batches: Vec<&BatchEncryptedShares> =
        other_msgs.iter().map(|m| &m.encrypted_shares).collect();
    let decrypted =
        decrypt_my_shares(&state.enc_sk, &batches, state.dealer_idx).map_err(|failed| {
            DkgOutputError::InvalidEncryptionProof {
                dealer_idx: other_msgs[failed[0]].dealer_idx,
            }
        })?;
    accumulate_decrypted_shares(state, &other_msgs, &decrypted, init)
}

fn aggregate_public_outputs<P: Clone + Serialize>(
    n: usize,
    broadcasts: &[DkgInitBroadcast<P>],
) -> (RistrettoPoint, Vec<PedersenCommitment>) {
    let mut public_key = RistrettoPoint::default();
    for msg in broadcasts {
        public_key += msg.f0_commitment;
    }
    let partial_verification_keys = (1..=n)
        .map(|k| {
            let mut agg = RistrettoPoint::default();
            for msg in broadcasts {
                agg += msg.pedvss[idx1_to_vec(k)].point();
            }
            PedersenCommitment::from_point(agg)
        })
        .collect();
    (public_key, partial_verification_keys)
}
pub fn dkg_initiate<R, S>(
    rng: &mut R,
    dkg_params: &DkgParams,
    proof_params: &S::Params,
    state: &PartyState,
    share: Scalar,
    parties: &Parties,
) -> DkgInitResult<S::Proof>
where
    R: RngCore + CryptoRng,
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    assert!(dkg_params.n > 0, "n must be > 0");
    assert!(
        state.dealer_idx >= 1 && state.dealer_idx <= dkg_params.n,
        "dealer_idx out of range"
    );
    assert_eq!(parties.len(), dkg_params.n, "parties length must equal n");
    assert!(dkg_params.t < dkg_params.n, "typically need t < n");

    let coeffs: Zeroizing<Vec<Scalar>> = Zeroizing::new(sample_random_polynomial_with_constant(
        rng,
        dkg_params.t,
        share,
    ));
    let blindings: Zeroizing<Vec<Scalar>> =
        Zeroizing::new((0..dkg_params.n).map(|_| Scalar::random(rng)).collect());
    let evaluations: Zeroizing<Vec<Scalar>> =
        Zeroizing::new(eval_poly_on_1_to_n(&coeffs, dkg_params.n));

    let pedvss: Vec<PedersenCommitment> = (1..=dkg_params.n)
        .map(|j| PedersenCommitment::new(evaluations[idx1_to_vec(j)], blindings[idx1_to_vec(j)]))
        .collect();

    let f0_commitment = g_mul_scalar(coeffs[0]);

    let proof = S::prove(
        proof_params,
        &PolyWellFormedStatement {
            x_points: domain_points_1_to_n(dkg_params.n),
            commitments: pedvss.clone(),
            f0_commitment,
        },
        &PolyWellFormedWitness {
            coeffs: coeffs.to_vec(),
            blindings: blindings.to_vec(),
        },
    );

    let receivers: Vec<(usize, RistrettoPoint)> = (1..=dkg_params.n)
        .filter(|&j| j != state.dealer_idx)
        .map(|j| (j, *parties.enc_pk(j)))
        .collect();
    let m1s: Vec<Scalar> = receivers
        .iter()
        .map(|(j, _)| evaluations[idx1_to_vec(*j)])
        .collect();
    let m2s: Vec<Scalar> = receivers
        .iter()
        .map(|(j, _)| blindings[idx1_to_vec(*j)])
        .collect();
    let encrypted_shares = encrypt_batch(&receivers, &m1s, &m2s);

    let local = DkgInitLocalState {
        my_share: evaluations[idx1_to_vec(state.dealer_idx)],
        my_blinding: blindings[idx1_to_vec(state.dealer_idx)],
    };
    let broadcast = DkgInitBroadcast::new(
        state.dealer_idx,
        pedvss,
        f0_commitment,
        proof,
        encrypted_shares,
        &state.sig_sk,
    );

    DkgInitResult { broadcast, local }
}

pub fn dkg_output_key_generation<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
    state: &PartyState,
    local: &DkgInitLocalState,
    broadcasts: &[DkgInitBroadcast<S::Proof>],
    parties: &Parties,
) -> Result<DkgOutput, DkgOutputError>
where
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    validate_broadcasts(dkg_params.n, state.dealer_idx, broadcasts, parties)?;
    verify_poly_proofs::<S>(dkg_params, proof_params, broadcasts)?;

    let (s_i, s_i_blind) =
        decrypt_and_accumulate(state, broadcasts, (local.my_share, local.my_blinding))?;

    let (public_key, partial_verification_keys) =
        aggregate_public_outputs(dkg_params.n, broadcasts);

    Ok(DkgOutput {
        idx: state.dealer_idx,
        secret_share: s_i,
        blinding_share: s_i_blind,
        public_key,
        partial_verification_keys,
    })
}

fn validate_broadcasts_parallel<P: Clone + Serialize + Sync>(
    n: usize,
    my_idx: usize,
    broadcasts: &[DkgInitBroadcast<P>],
    parties: &Parties,
) -> Result<(), DkgOutputError> {
    if n == 0 || my_idx == 0 || my_idx > n {
        return Err(DkgOutputError::InvalidParameters);
    }
    let mut seen = vec![false; n];
    for msg in broadcasts {
        if msg.dealer_idx == 0 || msg.dealer_idx > n || msg.pedvss.len() != n {
            return Err(DkgOutputError::InvalidParameters);
        }
        let slot = idx1_to_vec(msg.dealer_idx);
        if seen[slot] {
            return Err(DkgOutputError::InvalidParameters);
        }
        seen[slot] = true;
    }
    if seen.iter().any(|b| !*b) {
        return Err(DkgOutputError::InvalidParameters);
    }
    match broadcasts
        .par_iter()
        .find_any(|msg| !msg.verify(parties.sig_pk(msg.dealer_idx)))
    {
        Some(bad) => Err(DkgOutputError::InvalidSignature {
            dealer_idx: bad.dealer_idx,
        }),
        None => Ok(()),
    }
}

fn verify_poly_proofs_parallel<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
    broadcasts: &[DkgInitBroadcast<S::Proof>],
) -> Result<(), DkgOutputError>
where
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + Serialize + Sync,
    S::Params: Sync,
{
    let domain = domain_points_1_to_n(dkg_params.n);
    let ok = broadcasts.par_iter().all(|msg| {
        let stmt = PolyWellFormedStatement {
            x_points: domain.clone(),
            commitments: msg.pedvss.clone(),
            f0_commitment: msg.f0_commitment,
        };
        S::verify(proof_params, &stmt, &msg.proof)
    });
    if ok {
        Ok(())
    } else {
        Err(DkgOutputError::InvalidBatchProof)
    }
}

fn aggregate_public_outputs_parallel<P: Clone + Serialize + Sync>(
    n: usize,
    broadcasts: &[DkgInitBroadcast<P>],
) -> (RistrettoPoint, Vec<PedersenCommitment>) {
    let mut public_key = RistrettoPoint::default();
    for msg in broadcasts {
        public_key += msg.f0_commitment;
    }
    let partial_verification_keys = (1..=n)
        .into_par_iter()
        .map(|k| {
            let mut agg = RistrettoPoint::default();
            for msg in broadcasts {
                agg += msg.pedvss[idx1_to_vec(k)].point();
            }
            PedersenCommitment::from_point(agg)
        })
        .collect();
    (public_key, partial_verification_keys)
}

/// Multi-threaded [`dkg_output_key_generation`], for large committees where the
/// output phase dominates. Same result as the sequential version.
pub fn dkg_output_key_generation_parallel<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
    state: &PartyState,
    local: &DkgInitLocalState,
    broadcasts: &[DkgInitBroadcast<S::Proof>],
    parties: &Parties,
) -> Result<DkgOutput, DkgOutputError>
where
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + Serialize + Sync,
    S::Params: Sync,
{
    validate_broadcasts_parallel(dkg_params.n, state.dealer_idx, broadcasts, parties)?;
    verify_poly_proofs_parallel::<S>(dkg_params, proof_params, broadcasts)?;

    let (s_i, s_i_blind) =
        decrypt_and_accumulate(state, broadcasts, (local.my_share, local.my_blinding))?;

    let (public_key, partial_verification_keys) =
        aggregate_public_outputs_parallel(dkg_params.n, broadcasts);

    Ok(DkgOutput {
        idx: state.dealer_idx,
        secret_share: s_i,
        blinding_share: s_i_blind,
        public_key,
        partial_verification_keys,
    })
}

/// Batch-verifies all broadcast signatures at once. `true` iff every signature
/// is valid; it does not say which one failed, so identifying a culprit needs
/// the per-message check.
pub fn batch_verify_signatures<P: Clone + Serialize>(
    broadcasts: &[DkgInitBroadcast<P>],
    parties: &Parties,
) -> bool {
    let messages: Vec<Vec<u8>> = broadcasts.iter().map(|m| m.signing_bytes()).collect();
    let message_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();
    let signatures: Vec<Signature> = broadcasts.iter().map(|m| m.signature).collect();
    let keys: Vec<VerifyingKey> = broadcasts
        .iter()
        .map(|m| *parties.sig_pk(m.dealer_idx))
        .collect();
    ed25519_dalek::verify_batch(&message_refs, &signatures, &keys).is_ok()
}

/// Builds the signed complaint from a bad-opening error. `None` for any other
/// error, which is not a share-opening dispute.
pub fn build_abort_report(state: &PartyState, err: &DkgOutputError) -> Option<AbortReport> {
    if let DkgOutputError::InvalidDecryptionOpening {
        dealer_idx,
        s_ji,
        r_ji,
        pi_i,
        ..
    } = err
    {
        let (shared, proof) = pi_i.as_ref();
        Some(AbortReport::new(
            state.dealer_idx,
            *dealer_idx,
            *s_ji,
            *r_ji,
            *shared,
            proof.clone(),
            &state.sig_sk,
        ))
    } else {
        None
    }
}

/// Checks a complaint against the accused dealer's broadcast.
pub fn verify_abort_report<P: Clone + Serialize>(
    parties: &Parties,
    accused: &DkgInitBroadcast<P>,
    report: &AbortReport,
) -> AbortVerdict {
    if accused.dealer_idx != report.accused_idx
        || report.reporter_idx == 0
        || report.reporter_idx > accused.pedvss.len()
        || report.reporter_idx == accused.dealer_idx
    {
        return AbortVerdict::InvalidReport;
    }
    let reporter_share = accused.encrypted_shares.shares.get(&report.reporter_idx);
    verify_report_core(
        parties,
        &accused.encrypted_shares.u,
        reporter_share,
        report,
        |s, r| accused.pedvss[idx1_to_vec(report.reporter_idx)].matches_opening(s, r),
    )
}
