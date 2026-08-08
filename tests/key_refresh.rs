// Refresh is the same protocol with zero as the shared value, which is the only
// thing separating the two modes, so these pin what that changes.

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use janus::one_round::{dkg_initiate, dkg_output_key_generation};
use janus::one_round_proofs::SchnorrPolyProof;
use janus::party::{PartyState, collect_public_parties, make_party_state};
use janus::pedersen::PedersenCommitment;
use janus::{DkgOutput, DkgParams};
use rand::rng;

const PARAMS: DkgParams = DkgParams { t: 3, n: 7 };

// Runs the one-round protocol with the given contribution per party and returns
// what every party ends up with.
fn run(states: &[PartyState], shares: &[Scalar]) -> Vec<DkgOutput> {
    let mut rng = rng();
    let parties = collect_public_parties(states);

    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let res = dkg_initiate::<_, SchnorrPolyProof>(
            &mut rng,
            &PARAMS,
            &(),
            &states[i - 1],
            shares[i - 1],
            &parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }

    (1..=PARAMS.n)
        .map(|i| {
            dkg_output_key_generation::<SchnorrPolyProof>(
                &PARAMS,
                &(),
                &states[i - 1],
                &locals[i - 1],
                &broadcasts,
                &parties,
            )
            .expect("output")
        })
        .collect()
}

#[test]
fn refresh_run_shares_zero() {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();

    let refreshed = run(&states, &vec![Scalar::ZERO; PARAMS.n]);

    // Contributing zero everywhere means the shared secret is zero, so the joint
    // key of a refresh run carries no key material of its own.
    assert_eq!(
        refreshed[0].public_key,
        RistrettoPoint::identity(),
        "a refresh must share zero, not a new secret"
    );
    for out in &refreshed {
        assert_eq!(out.public_key, refreshed[0].public_key);
    }
}

#[test]
fn refresh_rerandomizes_shares_without_moving_the_key() {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();

    let secrets: Vec<Scalar> = (0..PARAMS.n).map(|_| Scalar::random(&mut rng)).collect();
    let original = run(&states, &secrets);
    let refresh = run(&states, &vec![Scalar::ZERO; PARAMS.n]);

    for i in 0..PARAMS.n {
        // The refresh moved every share.
        assert_ne!(
            refresh[i].secret_share,
            Scalar::ZERO,
            "party {} kept a zero share, nothing was re-randomized",
            i + 1
        );

        // A party updates by adding what the refresh handed it.
        let new_share = original[i].secret_share + refresh[i].secret_share;
        let new_blinding = original[i].blinding_share + refresh[i].blinding_share;
        assert_ne!(
            new_share,
            original[i].secret_share,
            "party {} ended up with its old share",
            i + 1
        );

        // The verification key moves the same way, and the updated share opens it.
        let old_vk = *original[i].partial_verification_keys[i].point();
        let delta_vk = *refresh[i].partial_verification_keys[i].point();
        let new_vk = PedersenCommitment::from_point(old_vk + delta_vk);
        assert!(
            new_vk.verify_opening(new_share, new_blinding),
            "updated share of party {} does not open its updated key",
            i + 1
        );
    }

    // The joint public key is what a refresh must leave alone.
    let combined = original[0].public_key + refresh[0].public_key;
    assert_eq!(
        combined, original[0].public_key,
        "a refresh must not move the public key"
    );
}
