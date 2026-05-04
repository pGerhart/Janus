use super::{DkgOutput, DkgParams};
use crate::encryption::proofs::{DecryptionProof, prove_decryption};
use crate::encryption::{BatchEncryptedShares, decrypt_my_shares, encrypt_batch};
use crate::group::g;
use crate::party::{Parties, PartyState};
use crate::pedersen::PedersenCommitment;
use crate::poly::{eval_poly_at, sample_random_polynomial_with_constant};
use crate::two_round_proofs::{
    DecomProofScheme,
    comeq_proof::{ComEqProof, ComEqStatement, ComEqWitness},
    decom_proof::{DecomStatement, DecomWitness},
    pk_proof::{PkProof, PkStatement, PkWitness},
};

use bincode;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroizing, ZeroizeOnDrop};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round1Broadcast<P> {
    pub dealer_idx: usize,
    pub pedvss: Vec<PedersenCommitment>,  // C_{i,0}, ..., C_{i,t}
    pub d_commitment: PedersenCommitment, // D_i = Com(omega_i, r_i)
    pub decom_proof: P,
    pub encrypted_shares: BatchEncryptedShares,
    pub signature: Signature,
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct Round1LocalState {
    pub my_s_ii: Scalar,
    pub my_sprime_ii: Scalar,
    pub omega: Scalar,
    pub r: Scalar,
    #[zeroize(skip)]
    pub pk: RistrettoPoint,
    #[zeroize(skip)]
    pub pk_proof: PkProof,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round2Broadcast {
    pub dealer_idx: usize,
    pub pk: RistrettoPoint,
    pub pk_proof: PkProof,
    pub vk_i: PedersenCommitment, // vk_i = Com(s_i, omega_i)
    pub comeq_proof: ComEqProof,
    pub signature: Signature,
}

#[derive(Clone, Debug)]
pub struct VerifiedRound1Info {
    pub c_0: PedersenCommitment,
    pub d_commitment: PedersenCommitment,
}

#[derive(Clone, Debug)]
pub struct VerifiedRound1Cache {
    pub by_idx: Vec<Option<VerifiedRound1Info>>,
    pub c_stars: Vec<PedersenCommitment>,
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct Round2LocalState {
    pub s_i: Scalar,
    pub omega: Scalar,
    #[zeroize(skip)]
    pub verified_round1: VerifiedRound1Cache,
}

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
        pi_i: (RistrettoPoint, DecryptionProof),
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

#[inline]
fn x_of(i: usize) -> Scalar {
    Scalar::from(i as u64)
}

fn evaluate_commitment_polynomial_points_at(
    coeff_points: &[RistrettoPoint],
    x: Scalar,
) -> RistrettoPoint {
    let mut x_pow = Scalar::ONE;
    let mut acc = RistrettoPoint::default();

    for p in coeff_points {
        acc += p * x_pow;
        x_pow *= x;
    }

    acc
}

fn evaluate_pedvss_at(commitments: &[PedersenCommitment], x: Scalar) -> PedersenCommitment {
    let coeff_points: Vec<RistrettoPoint> = commitments.iter().map(|c| *c.point()).collect();
    PedersenCommitment::from_point(evaluate_commitment_polynomial_points_at(&coeff_points, x))
}

fn evaluate_many_from_coeff_points(
    coeff_points: &[RistrettoPoint],
    n: usize,
) -> Vec<PedersenCommitment> {
    let mut out = Vec::with_capacity(n + 1);
    out.push(PedersenCommitment::from_point(RistrettoPoint::default()));

    for i in 1..=n {
        let point = evaluate_commitment_polynomial_points_at(coeff_points, x_of(i));
        out.push(PedersenCommitment::from_point(point));
    }

    out
}

fn aggregate_pedvss_coefficients_points<P>(
    valid_round1: &[&Round1Broadcast<P>],
    t: usize,
) -> Vec<RistrettoPoint> {
    let mut acc = vec![RistrettoPoint::default(); t + 1];

    for msg in valid_round1 {
        for (k, c) in msg.pedvss.iter().enumerate() {
            acc[k] += c.point();
        }
    }

    acc
}

fn build_verified_round1_cache<P>(
    params: &DkgParams,
    valid_round1: &[&Round1Broadcast<P>],
) -> VerifiedRound1Cache {
    let mut by_idx = vec![None; params.n + 1];
    for msg in valid_round1 {
        by_idx[msg.dealer_idx] = Some(VerifiedRound1Info {
            c_0: msg.pedvss[0].clone(),
            d_commitment: msg.d_commitment.clone(),
        });
    }

    let aggregated_coeffs = aggregate_pedvss_coefficients_points(valid_round1, params.t);
    let c_stars = evaluate_many_from_coeff_points(&aggregated_coeffs, params.n);

    VerifiedRound1Cache { by_idx, c_stars }
}

fn local_d(local: &Round1LocalState) -> PedersenCommitment {
    PedersenCommitment::new(local.omega, local.r)
}

impl<P: Clone + Serialize> Round1Broadcast<P> {
    pub fn new(
        dealer_idx: usize,
        pedvss: Vec<PedersenCommitment>,
        d_commitment: PedersenCommitment,
        decom_proof: P,
        encrypted_shares: BatchEncryptedShares,
        signing_key: &SigningKey,
    ) -> Self {
        let mut msg = Self {
            dealer_idx,
            pedvss,
            d_commitment,
            decom_proof,
            encrypted_shares,
            signature: Signature::from_bytes(&[0u8; 64]),
        };
        msg.sign(signing_key);
        msg
    }

    fn signing_bytes(&self) -> Vec<u8> {
        let mut tmp = self.clone();
        tmp.signature = Signature::from_bytes(&[0u8; 64]);
        bincode::serialize(&tmp).expect("serialization for signing failed")
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        self.signature = sk.sign(&self.signing_bytes());
    }

    pub fn verify(&self, pk: &VerifyingKey) -> bool {
        pk.verify_strict(&self.signing_bytes(), &self.signature)
            .is_ok()
    }
}

impl Round2Broadcast {
    pub fn new(
        dealer_idx: usize,
        pk: RistrettoPoint,
        pk_proof: PkProof,
        vk_i: PedersenCommitment,
        comeq_proof: ComEqProof,
        signing_key: &SigningKey,
    ) -> Self {
        let mut msg = Self {
            dealer_idx,
            pk,
            pk_proof,
            vk_i,
            comeq_proof,
            signature: Signature::from_bytes(&[0u8; 64]),
        };
        msg.sign(signing_key);
        msg
    }

    fn signing_bytes(&self) -> Vec<u8> {
        let mut tmp = self.clone();
        tmp.signature = Signature::from_bytes(&[0u8; 64]);
        bincode::serialize(&tmp).expect("serialization for signing failed")
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        self.signature = sk.sign(&self.signing_bytes());
    }

    pub fn verify(&self, pk: &VerifyingKey) -> bool {
        pk.verify_strict(&self.signing_bytes(), &self.signature)
            .is_ok()
    }
}

pub fn dkg_round1_initiate<R, S>(
    rng: &mut R,
    params: &DkgParams,
    decom_params: &S::Params,
    state: &PartyState,
    share: Scalar,
    parties: &Parties,
) -> (Round1Broadcast<S::Proof>, Round1LocalState)
where
    R: RngCore + CryptoRng,
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    assert!(params.n > 0, "n must be > 0");
    assert!(params.t < params.n, "need t < n");
    assert!(state.dealer_idx >= 1 && state.dealer_idx <= params.n);
    assert_eq!(parties.len(), params.n);

    let f_coeffs: Zeroizing<Vec<Scalar>> =
        Zeroizing::new(sample_random_polynomial_with_constant(rng, params.t, share));
    let blinding_constant = Scalar::random(rng);
    let fprime_coeffs: Zeroizing<Vec<Scalar>> =
        Zeroizing::new(sample_random_polynomial_with_constant(rng, params.t, blinding_constant));

    let pedvss: Vec<PedersenCommitment> = (0..=params.t)
        .map(|k| PedersenCommitment::new(f_coeffs[k], fprime_coeffs[k]))
        .collect();

    let a0 = f_coeffs[0];
    let b0 = fprime_coeffs[0];

    let omega = Scalar::random(rng);
    let r = Scalar::random(rng);
    let d_commitment = PedersenCommitment::new(omega, r);

    let decom_statement = DecomStatement {
        pedvss: pedvss.clone(),
        d: d_commitment.clone(),
    };
    let decom_witness = DecomWitness {
        a: f_coeffs.to_vec(),
        b: fprime_coeffs.to_vec(),
        omega,
        r,
    };
    let decom_proof = S::prove(decom_params, &decom_statement, &decom_witness);

    let pk = g() * a0;
    let pk_statement = PkStatement {
        pk,
        commitment: pedvss[0].clone(),
    };
    let pk_witness = PkWitness { a: a0, b: b0 };
    let pk_proof = PkProof::prove(rng, &pk_statement, &pk_witness);

    let receivers: Vec<(usize, RistrettoPoint)> = (1..=params.n)
        .filter(|&j| j != state.dealer_idx)
        .map(|j| (j, *parties.enc_pk(j)))
        .collect();
    let m1s: Vec<Scalar> = receivers.iter().map(|(j, _)| eval_poly_at(&f_coeffs, x_of(*j))).collect();
    let m2s: Vec<Scalar> = receivers.iter().map(|(j, _)| eval_poly_at(&fprime_coeffs, x_of(*j))).collect();
    let encrypted_shares = encrypt_batch(&receivers, &m1s, &m2s);

    let my_x = x_of(state.dealer_idx);
    let my_s_ii = eval_poly_at(&f_coeffs, my_x);
    let my_sprime_ii = eval_poly_at(&fprime_coeffs, my_x);

    let local = Round1LocalState {
        my_s_ii,
        my_sprime_ii,
        omega,
        r,
        pk,
        pk_proof,
    };

    let broadcast = Round1Broadcast::new(
        state.dealer_idx,
        pedvss,
        d_commitment,
        decom_proof,
        encrypted_shares,
        &state.sig_sk,
    );

    (broadcast, local)
}

fn validate_round1_msgs<'a, S>(
    params: &DkgParams,
    my_idx: usize,
    round1_msgs: &'a [Round1Broadcast<S::Proof>],
    parties: &Parties,
    decom_params: &S::Params,
) -> Result<Vec<&'a Round1Broadcast<S::Proof>>, TwoRoundDkgError>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    if params.n == 0 || my_idx == 0 || my_idx > params.n {
        return Err(TwoRoundDkgError::InvalidParameters);
    }
    let mut valid = Vec::with_capacity(round1_msgs.len());
    for msg in round1_msgs {
        if msg.dealer_idx == 0 || msg.dealer_idx > params.n {
            return Err(TwoRoundDkgError::InvalidParameters);
        }
        if msg.pedvss.len() != params.t + 1 {
            return Err(TwoRoundDkgError::InvalidPedVssLength { dealer_idx: msg.dealer_idx });
        }
        if !msg.verify(parties.sig_pk(msg.dealer_idx)) {
            return Err(TwoRoundDkgError::InvalidSignature { dealer_idx: msg.dealer_idx });
        }
        let decom_statement = DecomStatement {
            pedvss: msg.pedvss.clone(),
            d: msg.d_commitment.clone(),
        };
        if !S::verify(decom_params, &decom_statement, &msg.decom_proof) {
            return Err(TwoRoundDkgError::InvalidDecomProof { dealer_idx: msg.dealer_idx });
        }
        valid.push(msg);
    }
    Ok(valid)
}

fn accumulate_decrypted_shares_two_round<P: Clone + Serialize>(
    state: &PartyState,
    other_msgs: &[&Round1Broadcast<P>],
    decrypted: &[Option<(Scalar, Scalar)>],
    init: (Scalar, Scalar),
) -> Result<(Scalar, Scalar), TwoRoundDkgError> {
    let (mut sum_s, mut sum_sprime) = init;
    for (msg, dec) in other_msgs.iter().zip(decrypted.iter()) {
        let (s_ji, sprime_ji) = dec.ok_or(TwoRoundDkgError::MissingCiphertext {
            dealer_idx: msg.dealer_idx,
            receiver_idx: state.dealer_idx,
        })?;
        let expected = evaluate_pedvss_at(&msg.pedvss, x_of(state.dealer_idx));
        if !expected.matches_opening(s_ji, sprime_ji) {
            let pi_i = prove_decryption(&state.enc_sk, &state.enc_pk, &msg.encrypted_shares.u);
            return Err(TwoRoundDkgError::InvalidDecryptionOpening {
                dealer_idx: msg.dealer_idx,
                receiver_idx: state.dealer_idx,
                s_ji,
                sprime_ji,
                pi_i,
            });
        }
        sum_s += s_ji;
        sum_sprime += sprime_ji;
    }
    Ok((sum_s, sum_sprime))
}

pub fn dkg_round2_finalize<S>(
    params: &DkgParams,
    decom_params: &S::Params,
    state: &PartyState,
    local: &Round1LocalState,
    round1_msgs: &[Round1Broadcast<S::Proof>],
    parties: &Parties,
) -> Result<(Round2Broadcast, Round2LocalState), TwoRoundDkgError>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    let valid_round1 =
        validate_round1_msgs::<S>(params, state.dealer_idx, round1_msgs, parties, decom_params)?;

    let other_msgs: Vec<_> = valid_round1
        .iter()
        .filter(|&&msg| msg.dealer_idx != state.dealer_idx)
        .copied()
        .collect();

    let batches: Vec<&BatchEncryptedShares> =
        other_msgs.iter().map(|msg| &msg.encrypted_shares).collect();

    let decrypted =
        decrypt_my_shares(&state.enc_sk, &batches, state.dealer_idx).map_err(|failed| {
            TwoRoundDkgError::InvalidEncryptionProof {
                dealer_idx: other_msgs[failed[0]].dealer_idx,
            }
        })?;

    let (sum_s_i, sum_sprime_i) = accumulate_decrypted_shares_two_round(
        state,
        &other_msgs,
        &decrypted,
        (local.my_s_ii, local.my_sprime_ii),
    )?;
    let verified_round1 = build_verified_round1_cache(params, &valid_round1);
    let c_star_i = verified_round1.c_stars[state.dealer_idx].clone();

    let vk_i = PedersenCommitment::new(sum_s_i, local.omega);

    let comeq_statement = ComEqStatement {
        c: c_star_i,
        vk: vk_i.clone(),
        d: local_d(local),
    };
    let comeq_witness = ComEqWitness {
        s: sum_s_i,
        s_prime: sum_sprime_i,
        omega: local.omega,
        r: local.r,
    };

    let mut rng = rand::thread_rng();
    let comeq_proof = ComEqProof::prove(&mut rng, &comeq_statement, &comeq_witness);

    let round2 = Round2Broadcast::new(
        state.dealer_idx,
        local.pk,
        local.pk_proof.clone(),
        vk_i,
        comeq_proof,
        &state.sig_sk,
    );
    let local2 = Round2LocalState {
        s_i: sum_s_i,
        omega: local.omega,
        verified_round1,
    };

    Ok((round2, local2))
}

pub fn dkg_output<S>(
    params: &DkgParams,
    _decom_params: &S::Params,
    state: &PartyState,
    local2: &Round2LocalState,
    _round1_msgs: &[Round1Broadcast<S::Proof>],
    round2_msgs: &[Round2Broadcast],
    parties: &Parties,
) -> Result<DkgOutput, TwoRoundDkgError>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Proof: Clone + std::fmt::Debug + Serialize,
{
    if params.n == 0 || state.dealer_idx == 0 || state.dealer_idx > params.n {
        return Err(TwoRoundDkgError::InvalidParameters);
    }

    if local2.verified_round1.by_idx.len() != params.n + 1
        || local2.verified_round1.c_stars.len() != params.n + 1
    {
        return Err(TwoRoundDkgError::InvalidParameters);
    }

    let mut round2_by_idx: Vec<Option<&Round2Broadcast>> = vec![None; params.n + 1];
    for msg in round2_msgs {
        if !msg.verify(parties.sig_pk(msg.dealer_idx)) {
            return Err(TwoRoundDkgError::InvalidSignature {
                dealer_idx: msg.dealer_idx,
            });
        }
        round2_by_idx[msg.dealer_idx] = Some(msg);
    }

    for dealer_idx in 1..=params.n {
        let r1 = local2.verified_round1.by_idx[dealer_idx]
            .as_ref()
            .ok_or(TwoRoundDkgError::MissingRound2Message { dealer_idx })?;
        let r2 = round2_by_idx[dealer_idx]
            .ok_or(TwoRoundDkgError::MissingRound2Message { dealer_idx })?;

        let pk_statement = PkStatement {
            pk: r2.pk,
            commitment: r1.c_0.clone(),
        };
        if !r2.pk_proof.verify(&pk_statement) {
            return Err(TwoRoundDkgError::InvalidPkProof { dealer_idx });
        }

        let comeq_statement = ComEqStatement {
            c: local2.verified_round1.c_stars[dealer_idx].clone(),
            vk: r2.vk_i.clone(),
            d: r1.d_commitment.clone(),
        };
        if !r2.comeq_proof.verify(&comeq_statement) {
            return Err(TwoRoundDkgError::InvalidComEqProof { dealer_idx });
        }
    }

    let mut public_key = RistrettoPoint::default();
    let mut verification_keys = Vec::with_capacity(params.n);

    for dealer_idx in 1..=params.n {
        let r2 = round2_by_idx[dealer_idx]
            .ok_or(TwoRoundDkgError::MissingRound2Message { dealer_idx })?;
        public_key += r2.pk;
        verification_keys.push(r2.vk_i.clone());
    }

    Ok(DkgOutput {
        idx: state.dealer_idx,
        secret_share: local2.s_i,
        blinding_share: local2.omega,
        public_key,
        partial_verification_keys: verification_keys,
    })
}
