// Every error variant a party can return on malformed or adversarial input.
// The variants that name a dealer are what the abort path acts on, so each test
// also checks that the reported index is the one that actually misbehaved.

use curve25519_dalek::scalar::Scalar;
use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, dkg_initiate, dkg_output_key_generation,
    dkg_output_key_generation_from_wire,
};
use janus::one_round_proofs::{PolyProofScheme, SchnorrPolyProof};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, Round1LocalState, dkg_round1_initiate, dkg_round2_finalize,
};
use janus::two_round_proofs::{SchnorrDecomProof, SchnorrDecomProofParams};
use janus::{DkgOutputError, DkgParams, TwoRoundDkgError};
use rand::rng;

const PARAMS: DkgParams = DkgParams { t: 2, n: 5 };

type SchnorrProof = <SchnorrPolyProof as PolyProofScheme>::Proof;

fn one_round_setup() -> (
    Vec<PartyState>,
    Parties,
    Vec<DkgInitBroadcast<SchnorrProof>>,
    Vec<DkgInitLocalState>,
) {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();
    let parties = collect_public_parties(&states);

    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let share = Scalar::random(&mut rng);
        let res = dkg_initiate::<_, SchnorrPolyProof>(
            &mut rng,
            &PARAMS,
            &(),
            &states[i - 1],
            share,
            &parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }
    (states, parties, broadcasts, locals)
}

fn run_output(
    states: &[PartyState],
    parties: &Parties,
    broadcasts: &[DkgInitBroadcast<SchnorrProof>],
    locals: &[DkgInitLocalState],
    receiver: usize,
) -> Result<janus::DkgOutput, DkgOutputError> {
    dkg_output_key_generation::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[receiver - 1],
        &locals[receiver - 1],
        broadcasts,
        parties,
    )
}

// InvalidParameters covers the structural checks: the receiver index, a dealer
// index outside the committee, a duplicate dealer, and a short commitment vector.

#[test]
fn receiver_index_out_of_range_is_rejected() {
    let (states, parties, broadcasts, locals) = one_round_setup();
    let bad = DkgParams { t: PARAMS.t, n: 0 };
    let err = dkg_output_key_generation::<SchnorrPolyProof>(
        &bad,
        &(),
        &states[0],
        &locals[0],
        &broadcasts,
        &parties,
    )
    .unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidParameters));
}

#[test]
fn duplicate_dealer_is_rejected() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();
    broadcasts[1] = broadcasts[0].clone();
    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidParameters));
}

#[test]
fn missing_dealer_is_rejected() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();
    broadcasts.pop();
    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidParameters));
}

#[test]
fn short_commitment_vector_is_rejected() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();
    broadcasts[2].pedvss.pop();
    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidParameters));
}

// A dealer whose message was altered after signing must be named, not just refused.

#[test]
fn tampered_broadcast_names_its_dealer() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();
    broadcasts[2].f0_commitment += janus::group::g();

    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    match err {
        DkgOutputError::InvalidSignature { dealer_idx } => assert_eq!(dealer_idx, 3),
        other => panic!("expected InvalidSignature, got {other:?}"),
    }
}

// A dealer that swaps in another dealer's proof keeps a valid signature, since it
// re-signs its own message, but the well-formedness proof no longer verifies.

#[test]
fn invalid_well_formedness_proof_names_its_dealer() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();

    let stolen = broadcasts[0].proof.clone();
    broadcasts[3] = DkgInitBroadcast::new(
        4,
        broadcasts[3].pedvss.clone(),
        broadcasts[3].f0_commitment,
        stolen,
        broadcasts[3].encrypted_shares.clone(),
        &states[3].sig_sk,
    );
    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    match err {
        DkgOutputError::InvalidBatchProof { dealer_idx } => assert_eq!(dealer_idx, 4),
        other => panic!("expected InvalidBatchProof, got {other:?}"),
    }
}

// A dealer that drops the receiver's ciphertext, then re-signs, is caught at the
// decryption step rather than at the signature or proof step.

#[test]
fn missing_ciphertext_names_both_parties() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();
    let receiver = 1usize;

    let mut shares = broadcasts[2].encrypted_shares.clone();
    shares.shares.remove(&receiver);
    broadcasts[2] = DkgInitBroadcast::new(
        3,
        broadcasts[2].pedvss.clone(),
        broadcasts[2].f0_commitment,
        broadcasts[2].proof.clone(),
        shares,
        &states[2].sig_sk,
    );

    let err = run_output(&states, &parties, &broadcasts, &locals, receiver).unwrap_err();
    match err {
        DkgOutputError::MissingCiphertext {
            dealer_idx,
            receiver_idx,
        } => {
            assert_eq!(dealer_idx, 3);
            assert_eq!(receiver_idx, receiver);
        }
        other => panic!("expected MissingCiphertext, got {other:?}"),
    }
}

// The proof of knowledge on the ephemeral encryption key is what binds the batch
// to its sender, so a broken one must be attributed before anything is decrypted.

#[test]
fn invalid_encryption_proof_names_its_dealer() {
    let (states, parties, mut broadcasts, locals) = one_round_setup();

    let mut shares = broadcasts[1].encrypted_shares.clone();
    shares.u += janus::group::g();
    broadcasts[1] = DkgInitBroadcast::new(
        2,
        broadcasts[1].pedvss.clone(),
        broadcasts[1].f0_commitment,
        broadcasts[1].proof.clone(),
        shares,
        &states[1].sig_sk,
    );

    let err = run_output(&states, &parties, &broadcasts, &locals, 1).unwrap_err();
    match err {
        DkgOutputError::InvalidEncryptionProof { dealer_idx } => assert_eq!(dealer_idx, 2),
        other => panic!("expected InvalidEncryptionProof, got {other:?}"),
    }
}

// The channel path reports a single variant for anything that fails to decode or
// to authenticate, since at that point no dealer can be named yet.

#[test]
fn malformed_wire_message_is_rejected() {
    let (states, parties, broadcasts, locals) = one_round_setup();
    let mut wire: Vec<Vec<u8>> = broadcasts.iter().map(|b| b.to_wire()).collect();
    let mid = wire[2].len() / 2;
    wire[2][mid] ^= 1;

    let err = dkg_output_key_generation_from_wire::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[0],
        &locals[0],
        &wire,
        &parties,
    )
    .unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidWire));
}

#[test]
fn truncated_wire_message_is_rejected() {
    let (states, parties, broadcasts, locals) = one_round_setup();
    let mut wire: Vec<Vec<u8>> = broadcasts.iter().map(|b| b.to_wire()).collect();
    wire[1].truncate(8);

    let err = dkg_output_key_generation_from_wire::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[0],
        &locals[0],
        &wire,
        &parties,
    )
    .unwrap_err();
    assert!(matches!(err, DkgOutputError::InvalidWire));
}

// Two-round protocol.

fn two_round_setup() -> (
    Vec<PartyState>,
    Parties,
    Vec<Round1Broadcast<<SchnorrDecomProof as janus::two_round_proofs::DecomProofScheme>::Proof>>,
    Vec<Round1LocalState>,
) {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();
    let parties = collect_public_parties(&states);

    let mut msgs = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let share = Scalar::random(&mut rng);
        let (msg, local) = dkg_round1_initiate::<_, SchnorrDecomProof>(
            &mut rng,
            &PARAMS,
            &SchnorrDecomProofParams,
            &states[i - 1],
            share,
            &parties,
        );
        msgs.push(msg);
        locals.push(local);
    }
    (states, parties, msgs, locals)
}

fn run_finalize(
    states: &[PartyState],
    parties: &Parties,
    msgs: &[Round1Broadcast<
        <SchnorrDecomProof as janus::two_round_proofs::DecomProofScheme>::Proof,
    >],
    locals: &[Round1LocalState],
    receiver: usize,
) -> Result<
    (
        janus::two_round::Round2Broadcast,
        janus::two_round::Round2LocalState,
    ),
    TwoRoundDkgError,
> {
    dkg_round2_finalize::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[receiver - 1],
        &locals[receiver - 1],
        msgs,
        parties,
    )
}

#[test]
fn two_round_tampered_message_names_its_dealer() {
    let (states, parties, mut msgs, locals) = two_round_setup();
    msgs[2].d_commitment =
        janus::pedersen::PedersenCommitment::new(Scalar::from(7u64), Scalar::from(9u64));

    let err = run_finalize(&states, &parties, &msgs, &locals, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidSignature { dealer_idx } => assert_eq!(dealer_idx, 3),
        other => panic!("expected InvalidSignature, got {other:?}"),
    }
}

#[test]
fn two_round_wrong_commitment_length_names_its_dealer() {
    let (states, parties, mut msgs, locals) = two_round_setup();
    let mut pedvss = msgs[1].pedvss.clone();
    pedvss.pop();
    msgs[1] = Round1Broadcast::new(
        2,
        pedvss,
        msgs[1].d_commitment.clone(),
        msgs[1].decom_proof.clone(),
        msgs[1].encrypted_shares.clone(),
        &states[1].sig_sk,
    );

    let err = run_finalize(&states, &parties, &msgs, &locals, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidPedVssLength { dealer_idx } => assert_eq!(dealer_idx, 2),
        other => panic!("expected InvalidPedVssLength, got {other:?}"),
    }
}

#[test]
fn two_round_invalid_decom_proof_names_its_dealer() {
    let (states, parties, mut msgs, locals) = two_round_setup();
    let stolen = msgs[0].decom_proof.clone();
    msgs[3] = Round1Broadcast::new(
        4,
        msgs[3].pedvss.clone(),
        msgs[3].d_commitment.clone(),
        stolen,
        msgs[3].encrypted_shares.clone(),
        &states[3].sig_sk,
    );

    let err = run_finalize(&states, &parties, &msgs, &locals, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidDecomProof { dealer_idx } => assert_eq!(dealer_idx, 4),
        other => panic!("expected InvalidDecomProof, got {other:?}"),
    }
}

#[test]
fn two_round_missing_ciphertext_names_both_parties() {
    let (states, parties, mut msgs, locals) = two_round_setup();
    let receiver = 1usize;

    let mut shares = msgs[2].encrypted_shares.clone();
    shares.shares.remove(&receiver);
    msgs[2] = Round1Broadcast::new(
        3,
        msgs[2].pedvss.clone(),
        msgs[2].d_commitment.clone(),
        msgs[2].decom_proof.clone(),
        shares,
        &states[2].sig_sk,
    );

    let err = run_finalize(&states, &parties, &msgs, &locals, receiver).unwrap_err();
    match err {
        TwoRoundDkgError::MissingCiphertext {
            dealer_idx,
            receiver_idx,
        } => {
            assert_eq!(dealer_idx, 3);
            assert_eq!(receiver_idx, receiver);
        }
        other => panic!("expected MissingCiphertext, got {other:?}"),
    }
}

#[test]
fn two_round_invalid_encryption_proof_names_its_dealer() {
    let (states, parties, mut msgs, locals) = two_round_setup();

    let mut shares = msgs[1].encrypted_shares.clone();
    shares.u += janus::group::g();
    msgs[1] = Round1Broadcast::new(
        2,
        msgs[1].pedvss.clone(),
        msgs[1].d_commitment.clone(),
        msgs[1].decom_proof.clone(),
        shares,
        &states[1].sig_sk,
    );

    let err = run_finalize(&states, &parties, &msgs, &locals, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidEncryptionProof { dealer_idx } => assert_eq!(dealer_idx, 2),
        other => panic!("expected InvalidEncryptionProof, got {other:?}"),
    }
}

#[test]
fn two_round_bad_parameters_are_rejected() {
    let (states, parties, msgs, locals) = two_round_setup();
    let bad = DkgParams { t: PARAMS.t, n: 0 };
    let err = dkg_round2_finalize::<SchnorrDecomProof>(
        &bad,
        &SchnorrDecomProofParams,
        &states[0],
        &locals[0],
        &msgs,
        &parties,
    )
    .unwrap_err();
    assert!(matches!(err, TwoRoundDkgError::InvalidParameters));
}

// Round 2 output phase. These three variants are only reachable once a full
// round 1 and round 2 have run, so they need the complete setup.

type R1Msg =
    Round1Broadcast<<SchnorrDecomProof as janus::two_round_proofs::DecomProofScheme>::Proof>;

#[allow(clippy::type_complexity)]
fn two_round_through_finalize() -> (
    Vec<PartyState>,
    Parties,
    Vec<R1Msg>,
    Vec<janus::two_round::Round2Broadcast>,
    Vec<janus::two_round::Round2LocalState>,
) {
    let (states, parties, msgs, locals) = two_round_setup();
    let mut round2 = Vec::new();
    let mut locals2 = Vec::new();
    for i in 1..=PARAMS.n {
        let (msg, local) = run_finalize(&states, &parties, &msgs, &locals, i).expect("finalize");
        round2.push(msg);
        locals2.push(local);
    }
    (states, parties, msgs, round2, locals2)
}

fn run_two_round_output(
    states: &[PartyState],
    parties: &Parties,
    msgs: &[R1Msg],
    round2: &[janus::two_round::Round2Broadcast],
    locals2: &[janus::two_round::Round2LocalState],
    receiver: usize,
) -> Result<janus::DkgOutput, TwoRoundDkgError> {
    janus::two_round::dkg_output::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[receiver - 1],
        &locals2[receiver - 1],
        msgs,
        round2,
        parties,
    )
}

#[test]
fn two_round_missing_round2_message_names_its_dealer() {
    let (states, parties, msgs, mut round2, locals2) = two_round_through_finalize();
    round2.retain(|m| m.dealer_idx != 3);

    let err = run_two_round_output(&states, &parties, &msgs, &round2, &locals2, 1).unwrap_err();
    match err {
        TwoRoundDkgError::MissingRound2Message { dealer_idx } => assert_eq!(dealer_idx, 3),
        other => panic!("expected MissingRound2Message, got {other:?}"),
    }
}

#[test]
fn two_round_invalid_pk_proof_names_its_dealer() {
    let (states, parties, msgs, mut round2, locals2) = two_round_through_finalize();
    let stolen = round2[0].pk_proof.clone();
    round2[2] = janus::two_round::Round2Broadcast::new(
        3,
        round2[2].pk,
        stolen,
        round2[2].vk_i.clone(),
        round2[2].comeq_proof.clone(),
        &states[2].sig_sk,
    );

    let err = run_two_round_output(&states, &parties, &msgs, &round2, &locals2, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidPkProof { dealer_idx } => assert_eq!(dealer_idx, 3),
        other => panic!("expected InvalidPkProof, got {other:?}"),
    }
}

#[test]
fn two_round_invalid_comeq_proof_names_its_dealer() {
    let (states, parties, msgs, mut round2, locals2) = two_round_through_finalize();
    let stolen = round2[0].comeq_proof.clone();
    round2[3] = janus::two_round::Round2Broadcast::new(
        4,
        round2[3].pk,
        round2[3].pk_proof.clone(),
        round2[3].vk_i.clone(),
        stolen,
        &states[3].sig_sk,
    );

    let err = run_two_round_output(&states, &parties, &msgs, &round2, &locals2, 1).unwrap_err();
    match err {
        TwoRoundDkgError::InvalidComEqProof { dealer_idx } => assert_eq!(dealer_idx, 4),
        other => panic!("expected InvalidComEqProof, got {other:?}"),
    }
}
