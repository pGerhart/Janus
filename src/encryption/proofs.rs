use crate::group::g;
use crate::transcript::*;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use merlin::Transcript;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecryptionProof {
    pub a1: RistrettoPoint,
    pub a2: RistrettoPoint,
    pub z: Scalar,
}

fn decryption_challenge(
    pk: &RistrettoPoint,
    u: &RistrettoPoint,
    shared: &RistrettoPoint,
    a1: &RistrettoPoint,
    a2: &RistrettoPoint,
) -> Scalar {
    let mut transcript = Transcript::new(b"hashed-elgamal-decryption-proof");
    transcript.append_point(b"pk", pk);
    transcript.append_point(b"u", u);
    transcript.append_point(b"shared", shared);
    transcript.append_point(b"a1", a1);
    transcript.append_point(b"a2", a2);
    transcript.challenge_scalar(b"c")
}

pub fn prove_decryption(
    sk: &Scalar,
    pk: &RistrettoPoint,
    u: &RistrettoPoint,
) -> (RistrettoPoint, DecryptionProof) {
    let mut rng = OsRng;

    let shared = u * *sk;
    let r = Scalar::random(&mut rng);

    let a1 = g() * r;
    let a2 = *u * r;

    let c = decryption_challenge(pk, u, &shared, &a1, &a2);
    let z = r + c * *sk;

    (shared, DecryptionProof { a1, a2, z })
}

pub fn verify_decryption(
    pk: &RistrettoPoint,
    u: &RistrettoPoint,
    shared: &RistrettoPoint,
    proof: &DecryptionProof,
) -> bool {
    let c = decryption_challenge(pk, u, shared, &proof.a1, &proof.a2);

    let lhs1 = g() * proof.z;
    let rhs1 = proof.a1 + *pk * c;

    let lhs2 = *u * proof.z;
    let rhs2 = proof.a2 + *shared * c;

    lhs1 == rhs1 && lhs2 == rhs2
}

#[test]
fn test_decryption_proof() {
    let mut rng = OsRng;

    let sk = Scalar::random(&mut rng);
    let pk = g() * sk;

    let alpha = Scalar::random(&mut rng);
    let u = g() * alpha;

    let (shared, proof) = prove_decryption(&sk, &pk, &u);

    assert!(verify_decryption(&pk, &u, &shared, &proof));
}
