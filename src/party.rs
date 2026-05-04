use crate::group::g_mul_scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::{CryptoRng, RngCore};
use zeroize::ZeroizeOnDrop;

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct PartyState {
    pub dealer_idx: usize,
    pub enc_sk: Scalar,
    #[zeroize(skip)]
    pub enc_pk: RistrettoPoint,
    pub sig_sk: SigningKey,
    #[zeroize(skip)]
    pub sig_pk: VerifyingKey,
}

#[derive(Clone, Debug)]
pub struct PublicParty {
    pub dealer_idx: usize, // 1-based
    pub enc_pk: RistrettoPoint,
    pub sig_pk: VerifyingKey,
}

#[derive(Clone, Debug)]
pub struct Parties {
    pub parties: Vec<PublicParty>, // index = dealer_idx - 1
}

impl Parties {
    pub fn get(&self, dealer_idx: usize) -> &PublicParty {
        &self.parties[dealer_idx - 1]
    }

    pub fn enc_pk(&self, dealer_idx: usize) -> &RistrettoPoint {
        &self.get(dealer_idx).enc_pk
    }

    pub fn sig_pk(&self, dealer_idx: usize) -> &VerifyingKey {
        &self.get(dealer_idx).sig_pk
    }

    pub fn len(&self) -> usize {
        self.parties.len()
    }
}

pub fn make_party_state<R: RngCore + CryptoRng>(rng: &mut R, dealer_idx: usize) -> PartyState {
    let enc_sk = Scalar::random(rng);
    let enc_pk = g_mul_scalar(enc_sk);

    let sig_sk = SigningKey::generate(rng);
    let sig_pk = sig_sk.verifying_key();

    PartyState {
        dealer_idx,
        enc_sk,
        enc_pk,
        sig_sk,
        sig_pk,
    }
}

pub fn collect_public_parties(parties: &[PartyState]) -> Parties {
    let mut out = Vec::with_capacity(parties.len());

    for p in parties {
        out.push(PublicParty {
            dealer_idx: p.dealer_idx,
            enc_pk: p.enc_pk,
            sig_pk: p.sig_pk,
        });
    }

    Parties { parties: out }
}
