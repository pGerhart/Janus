use crate::group::{g, g_mul_scalar};
use curve25519_dalek::{
    ristretto::RistrettoPoint,
    scalar::Scalar,
    traits::{Identity, VartimeMultiscalarMul},
};
use rand::rngs::SysRng;
use rand_core::UnwrapErr;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchnorrDLogProof {
    pub r_pt: RistrettoPoint,
    pub z: Scalar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashedElgamalCiphertext2 {
    pub u: RistrettoPoint,
    pub v1: Scalar,
    pub v2: Scalar,
    pub pok: SchnorrDLogProof,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedShare {
    pub v1: Scalar,
    pub v2: Scalar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEncryptedShares {
    pub u: RistrettoPoint,
    pub pok: SchnorrDLogProof,
    pub shares: BTreeMap<usize, EncryptedShare>,
}

fn scalar_from_hash(compressed_bytes: &[u8], idx: u64) -> Scalar {
    let mut h = Sha512::new();
    h.update(b"hashed-elgamal-2scalar-v1");
    h.update(compressed_bytes);
    h.update(idx.to_le_bytes());
    let digest = h.finalize();
    let mut wide = [0u8; 64];
    wide.copy_from_slice(&digest);
    Scalar::from_bytes_mod_order_wide(&wide)
}

fn hash_to_two_scalars(shared: &RistrettoPoint) -> (Scalar, Scalar) {
    let compressed = shared.compress();
    let bytes = compressed.as_bytes();
    (scalar_from_hash(bytes, 1), scalar_from_hash(bytes, 2))
}

fn schnorr_challenge(u: &RistrettoPoint, r_pt: &RistrettoPoint) -> Scalar {
    let mut h = Sha512::new();
    h.update(b"schnorr-dlog-pok-u-v1");
    h.update(u.compress().as_bytes());
    h.update(r_pt.compress().as_bytes());
    let digest = h.finalize();
    let mut wide = [0u8; 64];
    wide.copy_from_slice(&digest);
    Scalar::from_bytes_mod_order_wide(&wide)
}

fn prove_dlog(u: &RistrettoPoint, alpha: Scalar) -> SchnorrDLogProof {
    let mut rng = UnwrapErr(SysRng);
    let r = Scalar::random(&mut rng);
    let r_pt = g_mul_scalar(r);
    let c = schnorr_challenge(u, &r_pt);
    SchnorrDLogProof {
        r_pt,
        z: r + c * alpha,
    }
}

fn batch_weight(seed: &[u8; 64], i: usize) -> Scalar {
    let mut h = Sha512::new();
    h.update(b"batch-schnorr-weight-v1");
    h.update(seed);
    h.update((i as u64).to_le_bytes());
    let digest = h.finalize();
    let mut wide = [0u8; 64];
    wide.copy_from_slice(&digest);
    Scalar::from_bytes_mod_order_wide(&wide)
}

fn batch_verify_u_pok_pairs(pairs: &[(RistrettoPoint, &SchnorrDLogProof)]) -> bool {
    if pairs.is_empty() {
        return true;
    }

    let mut seed_h = Sha512::new();
    seed_h.update(b"batch-schnorr-seed-v1");
    seed_h.update((pairs.len() as u64).to_le_bytes());
    for (u, pok) in pairs {
        seed_h.update(u.compress().as_bytes());
        seed_h.update(pok.r_pt.compress().as_bytes());
        seed_h.update(pok.z.as_bytes());
    }
    let seed: [u8; 64] = seed_h.finalize().into();

    let mut g_scalar = Scalar::ZERO;
    let mut scalars = Vec::with_capacity(2 * pairs.len() + 1);
    let mut points = Vec::with_capacity(2 * pairs.len() + 1);

    for (i, (u, pok)) in pairs.iter().enumerate() {
        let w = batch_weight(&seed, i);
        let c = schnorr_challenge(u, &pok.r_pt);
        g_scalar += w * pok.z;
        scalars.push(-w);
        points.push(pok.r_pt);
        scalars.push(-(w * c));
        points.push(*u);
    }
    scalars.push(g_scalar);
    points.push(g());

    RistrettoPoint::vartime_multiscalar_mul(scalars, points) == RistrettoPoint::identity()
}

pub fn verify_dlog(u: &RistrettoPoint, pok: &SchnorrDLogProof) -> bool {
    batch_verify_u_pok_pairs(&[(*u, pok)])
}

pub fn keygen() -> (Scalar, RistrettoPoint) {
    let mut rng = UnwrapErr(SysRng);
    let sk = Scalar::random(&mut rng);
    let pk = g_mul_scalar(sk);
    (sk, pk)
}

pub fn encrypt_two_scalars(
    pk: &RistrettoPoint,
    m1: Scalar,
    m2: Scalar,
) -> HashedElgamalCiphertext2 {
    let mut rng = UnwrapErr(SysRng);
    let alpha = Scalar::random(&mut rng);
    let u = g_mul_scalar(alpha);
    let shared = pk * alpha;
    let (k1, k2) = hash_to_two_scalars(&shared);
    HashedElgamalCiphertext2 {
        u,
        v1: m1 + k1,
        v2: m2 + k2,
        pok: prove_dlog(&u, alpha),
    }
}

pub fn decrypt_two_scalars(sk: &Scalar, ct: &HashedElgamalCiphertext2) -> (Scalar, Scalar) {
    let shared = ct.u * *sk;
    let (k1, k2) = hash_to_two_scalars(&shared);
    (ct.v1 - k1, ct.v2 - k2)
}

/// Opens an [`EncryptedShare`] from the DH value `shared = u^{sk}`, without the
/// receiver's key. Used when checking an abort report, which carries `shared`.
pub fn open_share_with_shared(shared: &RistrettoPoint, share: &EncryptedShare) -> (Scalar, Scalar) {
    let (k1, k2) = hash_to_two_scalars(shared);
    (share.v1 - k1, share.v2 - k2)
}

pub fn encrypt_batch(
    receivers: &[(usize, RistrettoPoint)],
    m1s: &[Scalar],
    m2s: &[Scalar],
) -> BatchEncryptedShares {
    assert_eq!(receivers.len(), m1s.len());
    assert_eq!(receivers.len(), m2s.len());

    let (alpha, u) = keygen();
    let pok = prove_dlog(&u, alpha);

    let shares = receivers
        .iter()
        .zip(m1s.iter().zip(m2s.iter()))
        .map(|((idx, pk), (m1, m2))| {
            let shared = pk * alpha;
            let (k1, k2) = hash_to_two_scalars(&shared);
            (
                *idx,
                EncryptedShare {
                    v1: m1 + k1,
                    v2: m2 + k2,
                },
            )
        })
        .collect();

    BatchEncryptedShares { u, pok, shares }
}

pub fn decrypt_my_shares(
    sk: &Scalar,
    batches: &[&BatchEncryptedShares],
    my_idx: usize,
) -> Result<Vec<Option<(Scalar, Scalar)>>, Vec<usize>> {
    if batches.is_empty() {
        return Ok(vec![]);
    }

    let pairs: Vec<(RistrettoPoint, &SchnorrDLogProof)> =
        batches.iter().map(|b| (b.u, &b.pok)).collect();

    if !batch_verify_u_pok_pairs(&pairs) {
        let failed = batches
            .iter()
            .enumerate()
            .filter(|(_, b)| !batch_verify_u_pok_pairs(&[(b.u, &b.pok)]))
            .map(|(i, _)| i)
            .collect();
        return Err(failed);
    }

    Ok(batches
        .iter()
        .map(|b| {
            b.shares.get(&my_idx).map(|share| {
                let shared = b.u * *sk;
                let (k1, k2) = hash_to_two_scalars(&shared);
                (share.v1 - k1, share.v2 - k2)
            })
        })
        .collect())
}

#[test]
fn test_hashed_elgamal_two_scalars() {
    let (sk, pk) = keygen();
    let m1 = Scalar::from(123u64);
    let m2 = Scalar::from(456u64);
    let ct = encrypt_two_scalars(&pk, m1, m2);
    assert!(verify_dlog(&ct.u, &ct.pok));
    let (d1, d2) = decrypt_two_scalars(&sk, &ct);
    assert_eq!(m1, d1);
    assert_eq!(m2, d2);
}

#[test]
fn test_encrypt_batch_roundtrip() {
    let (sk1, pk1) = keygen();
    let (sk2, pk2) = keygen();

    let receivers = vec![(1usize, pk1), (2usize, pk2)];
    let m1s = vec![Scalar::from(10u64), Scalar::from(20u64)];
    let m2s = vec![Scalar::from(11u64), Scalar::from(21u64)];

    let batch = encrypt_batch(&receivers, &m1s, &m2s);
    assert!(verify_dlog(&batch.u, &batch.pok));

    let dec1 = decrypt_my_shares(&sk1, &[&batch], 1).unwrap();
    let dec2 = decrypt_my_shares(&sk2, &[&batch], 2).unwrap();

    assert_eq!(dec1[0], Some((m1s[0], m2s[0])));
    assert_eq!(dec2[0], Some((m1s[1], m2s[1])));
}

#[test]
fn test_decrypt_my_shares_multi_batch() {
    let (sk, pk) = keygen();
    let (_, pk2) = keygen();

    let b1 = encrypt_batch(
        &[(3usize, pk)],
        &[Scalar::from(1u64)],
        &[Scalar::from(2u64)],
    );
    let b2 = encrypt_batch(
        &[(3usize, pk), (7usize, pk2)],
        &[Scalar::from(3u64), Scalar::from(99u64)],
        &[Scalar::from(4u64), Scalar::from(99u64)],
    );

    let result = decrypt_my_shares(&sk, &[&b1, &b2], 3).unwrap();
    assert_eq!(result[0], Some((Scalar::from(1u64), Scalar::from(2u64))));
    assert_eq!(result[1], Some((Scalar::from(3u64), Scalar::from(4u64))));
}

#[test]
fn test_decrypt_my_shares_missing_idx() {
    let (sk, pk) = keygen();
    let batch = encrypt_batch(
        &[(5usize, pk)],
        &[Scalar::from(7u64)],
        &[Scalar::from(8u64)],
    );
    let result = decrypt_my_shares(&sk, &[&batch], 99).unwrap();
    assert_eq!(result[0], None);
}

#[test]
fn test_decrypt_my_shares_bad_pok() {
    let (sk, pk) = keygen();
    let mut b0 = encrypt_batch(
        &[(1usize, pk)],
        &[Scalar::from(1u64)],
        &[Scalar::from(2u64)],
    );
    let b1 = encrypt_batch(
        &[(1usize, pk)],
        &[Scalar::from(3u64)],
        &[Scalar::from(4u64)],
    );
    let b2 = encrypt_batch(
        &[(1usize, pk)],
        &[Scalar::from(5u64)],
        &[Scalar::from(6u64)],
    );
    b0.pok.z += Scalar::ONE; // corrupt

    let err = decrypt_my_shares(&sk, &[&b0, &b1, &b2], 1).unwrap_err();
    assert_eq!(err, vec![0]);
}
