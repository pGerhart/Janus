// The channel-facing path: messages are authenticated over the bytes as received
// and decoded once. Attribution in the abort case rests on those bytes, so these
// tests pin both the agreement with the in-memory path and the rejection cases.

use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::one_round::{
    DkgInitBroadcast, dkg_initiate, dkg_output_key_generation, dkg_output_key_generation_from_wire,
};
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use rand::rng;

const PARAMS: DkgParams = DkgParams { t: 6, n: 16 };

fn setup<S>(
    proof_params: &S::Params,
) -> (
    Vec<PartyState>,
    Parties,
    Vec<DkgInitBroadcast<S::Proof>>,
    Vec<janus::one_round::DkgInitLocalState>,
)
where
    S: PolyProofScheme,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize,
{
    let mut rng = rng();
    let states: Vec<_> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();
    let parties = collect_public_parties(&states);

    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let res = dkg_initiate::<_, S>(
            &mut rng,
            &PARAMS,
            proof_params,
            &states[i - 1],
            Scalar::from(i as u64),
            &parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }
    (states, parties, broadcasts, locals)
}

#[test]
fn wire_output_matches_in_memory_output_schnorr() {
    let (states, parties, broadcasts, locals) = setup::<SchnorrPolyProof>(&());
    let wire: Vec<Vec<u8>> = broadcasts.iter().map(|b| b.to_wire()).collect();
    let receiver = 1usize;

    let direct = dkg_output_key_generation::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[receiver - 1],
        &locals[receiver - 1],
        &broadcasts,
        &parties,
    )
    .expect("in-memory output");

    let from_wire = dkg_output_key_generation_from_wire::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[receiver - 1],
        &locals[receiver - 1],
        &wire,
        &parties,
    )
    .expect("wire output");

    assert_eq!(direct.public_key, from_wire.public_key);
    assert_eq!(direct.secret_share, from_wire.secret_share);
    assert_eq!(direct.blinding_share, from_wire.blinding_share);
    assert_eq!(
        direct.partial_verification_keys,
        from_wire.partial_verification_keys
    );
}

#[test]
fn wire_output_matches_in_memory_output_fischlin() {
    let params = FischlinProofParams {
        rho: 4,
        b: 4,
        t_bits: 9,
    };
    let (states, parties, broadcasts, locals) = setup::<FischlinPolyProof>(&params);
    let wire: Vec<Vec<u8>> = broadcasts.iter().map(|b| b.to_wire()).collect();
    let receiver = 2usize;

    let direct = dkg_output_key_generation::<FischlinPolyProof>(
        &PARAMS,
        &params,
        &states[receiver - 1],
        &locals[receiver - 1],
        &broadcasts,
        &parties,
    )
    .expect("in-memory output");

    let from_wire = dkg_output_key_generation_from_wire::<FischlinPolyProof>(
        &PARAMS,
        &params,
        &states[receiver - 1],
        &locals[receiver - 1],
        &wire,
        &parties,
    )
    .expect("wire output");

    assert_eq!(direct.public_key, from_wire.public_key);
    assert_eq!(direct.secret_share, from_wire.secret_share);
}

#[test]
fn wire_roundtrip_preserves_the_message() {
    let (_states, parties, broadcasts, _locals) = setup::<SchnorrPolyProof>(&());
    for msg in &broadcasts {
        let parsed = DkgInitBroadcast::<<SchnorrPolyProof as PolyProofScheme>::Proof>::from_wire(
            &msg.to_wire(),
            &parties,
        )
        .expect("roundtrip");
        assert_eq!(parsed.dealer_idx, msg.dealer_idx);
        assert_eq!(parsed.pedvss, msg.pedvss);
        assert_eq!(parsed.f0_commitment, msg.f0_commitment);
        assert_eq!(parsed.signature, msg.signature);
    }
}

// A dealer must not be able to disown a message it signed, and nobody else must
// be able to put a dealer's name on one. Both are what the abort path rests on.
#[test]
fn wire_rejects_a_forged_sender() {
    let (_states, parties, broadcasts, _locals) = setup::<SchnorrPolyProof>(&());
    let mut wire = broadcasts[0].to_wire();

    // Re-label the message as coming from dealer 2 while keeping dealer 1's
    // signature. bincode puts the dealer index first in the payload.
    assert_eq!(wire[8], 1, "payload starts with the dealer index");
    wire[8] = 2;

    let parsed = DkgInitBroadcast::<<SchnorrPolyProof as PolyProofScheme>::Proof>::from_wire(
        &wire, &parties,
    );
    assert!(parsed.is_err(), "a re-labelled message must not verify");
}

#[test]
fn wire_rejects_a_tampered_payload() {
    let (_states, parties, broadcasts, _locals) = setup::<SchnorrPolyProof>(&());
    let mut wire = broadcasts[0].to_wire();
    let mid = wire.len() / 2;
    wire[mid] ^= 1;

    let parsed = DkgInitBroadcast::<<SchnorrPolyProof as PolyProofScheme>::Proof>::from_wire(
        &wire, &parties,
    );
    assert!(parsed.is_err(), "a tampered message must not verify");
}

#[test]
fn wire_rejects_a_truncated_message() {
    let (_states, parties, broadcasts, _locals) = setup::<SchnorrPolyProof>(&());
    let wire = broadcasts[0].to_wire();

    for cut in [0usize, 4, 8, wire.len() - 1] {
        let parsed = DkgInitBroadcast::<<SchnorrPolyProof as PolyProofScheme>::Proof>::from_wire(
            &wire[..cut],
            &parties,
        );
        assert!(parsed.is_err(), "truncation at {cut} must not verify");
    }
}
