use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::Signature;
use janus::DkgParams;
use janus::abort::{AbortReport, AbortVerdict};
use janus::encryption::proofs::prove_decryption;
use janus::one_round::{
    DkgInitBroadcast, DkgOutputError, build_abort_report, dkg_initiate, dkg_output_key_generation,
    verify_abort_report,
};
use janus::one_round_proofs::{PolyProofScheme, SchnorrPolyProof};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, build_abort_report as build_two_round_report, dkg_round1_initiate,
    dkg_round2_finalize, verify_abort_report as verify_two_round_report,
};
use janus::two_round_proofs::{DecomProofScheme, SchnorrDecomProof, SchnorrDecomProofParams};
use rand::rng;

const PARAMS: DkgParams = DkgParams { t: 2, n: 5 };

fn setup(n: usize) -> (Vec<PartyState>, Parties) {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&states);
    (states, parties)
}

// one round

fn one_round_broadcasts(
    states: &[PartyState],
    parties: &Parties,
) -> (
    Vec<DkgInitBroadcast<<SchnorrPolyProof as PolyProofScheme>::Proof>>,
    Vec<janus::one_round::DkgInitLocalState>,
) {
    let mut rng = rng();
    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let res = dkg_initiate::<_, SchnorrPolyProof>(
            &mut rng,
            &PARAMS,
            &(),
            &states[i - 1],
            Scalar::from(i as u64),
            parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }
    (broadcasts, locals)
}

#[test]
fn one_round_detects_and_convicts_malicious_dealer() {
    let (states, parties) = setup(PARAMS.n);
    let (mut broadcasts, locals) = one_round_broadcasts(&states, &parties);

    let dealer = 3usize;
    let reporter = 1usize;

    // The dealer sends the reporter a share that no longer opens its commitment,
    // then re-signs so the tampered broadcast is authentic.
    let dealer_sk = states[dealer - 1].sig_sk.clone();
    broadcasts[dealer - 1]
        .encrypted_shares
        .shares
        .get_mut(&reporter)
        .unwrap()
        .v1 += Scalar::ONE;
    broadcasts[dealer - 1].sign(&dealer_sk);

    let err = dkg_output_key_generation::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[reporter - 1],
        &locals[reporter - 1],
        &broadcasts,
        &parties,
    )
    .expect_err("reporter must detect the bad share");

    match err {
        DkgOutputError::InvalidDecryptionOpening {
            dealer_idx,
            receiver_idx,
            ..
        } => {
            assert_eq!(dealer_idx, dealer);
            assert_eq!(receiver_idx, reporter);
        }
        other => panic!("expected an opening failure, got {other:?}"),
    }

    let report = build_abort_report(&states[reporter - 1], &err).expect("a complaint is defined");
    let verdict = verify_abort_report(&parties, &broadcasts[dealer - 1], &report);
    assert_eq!(
        verdict,
        AbortVerdict::DealerGuilty {
            dealer_idx: dealer,
            reporter_idx: reporter,
        }
    );
}

#[test]
fn one_round_rejects_false_complaint() {
    let (states, parties) = setup(PARAMS.n);
    let (broadcasts, _locals) = one_round_broadcasts(&states, &parties);

    let dealer = 3usize;
    let reporter = 1usize;
    let accused = &broadcasts[dealer - 1];
    let rep = &states[reporter - 1];

    // The reporter proves its Diffie-Hellman value honestly but complains about a
    // share that in fact opens correctly.
    let (shared, proof) = prove_decryption(&rep.enc_sk, &rep.enc_pk, &accused.encrypted_shares.u);
    let report = AbortReport::new(
        reporter,
        dealer,
        Scalar::ZERO,
        Scalar::ZERO,
        shared,
        proof,
        &rep.sig_sk,
    );

    let verdict = verify_abort_report(&parties, accused, &report);
    assert_eq!(
        verdict,
        AbortVerdict::ReporterGuilty {
            reporter_idx: reporter,
        }
    );
}

#[test]
fn one_round_rejects_report_with_bad_proof() {
    let (states, parties) = setup(PARAMS.n);
    let (broadcasts, _locals) = one_round_broadcasts(&states, &parties);

    let dealer = 3usize;
    let reporter = 1usize;
    let accused = &broadcasts[dealer - 1];
    let rep = &states[reporter - 1];

    // A wrong decryption proof, signed so authentication still passes, points the
    // blame back at the reporter.
    let (shared, mut proof) =
        prove_decryption(&rep.enc_sk, &rep.enc_pk, &accused.encrypted_shares.u);
    proof.z += Scalar::ONE;
    let report = AbortReport::new(
        reporter,
        dealer,
        Scalar::ZERO,
        Scalar::ZERO,
        shared,
        proof,
        &rep.sig_sk,
    );

    let verdict = verify_abort_report(&parties, accused, &report);
    assert_eq!(
        verdict,
        AbortVerdict::ReporterGuilty {
            reporter_idx: reporter,
        }
    );
}

#[test]
fn one_round_discards_unauthenticated_report() {
    let (states, parties) = setup(PARAMS.n);
    let (broadcasts, _locals) = one_round_broadcasts(&states, &parties);

    let dealer = 3usize;
    let reporter = 1usize;
    let accused = &broadcasts[dealer - 1];
    let rep = &states[reporter - 1];

    let (shared, proof) = prove_decryption(&rep.enc_sk, &rep.enc_pk, &accused.encrypted_shares.u);
    let mut report = AbortReport::new(
        reporter,
        dealer,
        Scalar::ZERO,
        Scalar::ZERO,
        shared,
        proof,
        &rep.sig_sk,
    );
    report.signature = Signature::from_bytes(&[0u8; 64]);

    let verdict = verify_abort_report(&parties, accused, &report);
    assert_eq!(verdict, AbortVerdict::InvalidReport);
}

// two round

fn two_round_broadcasts(
    states: &[PartyState],
    parties: &Parties,
) -> (
    Vec<Round1Broadcast<<SchnorrDecomProof as DecomProofScheme>::Proof>>,
    Vec<janus::two_round::Round1LocalState>,
) {
    let mut rng = rng();
    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let (b, l) = dkg_round1_initiate::<_, SchnorrDecomProof>(
            &mut rng,
            &PARAMS,
            &SchnorrDecomProofParams,
            &states[i - 1],
            Scalar::from(i as u64),
            parties,
        );
        broadcasts.push(b);
        locals.push(l);
    }
    (broadcasts, locals)
}

#[test]
fn two_round_detects_and_convicts_malicious_dealer() {
    let (states, parties) = setup(PARAMS.n);
    let (mut broadcasts, locals) = two_round_broadcasts(&states, &parties);

    let dealer = 3usize;
    let reporter = 1usize;

    let dealer_sk = states[dealer - 1].sig_sk.clone();
    broadcasts[dealer - 1]
        .encrypted_shares
        .shares
        .get_mut(&reporter)
        .unwrap()
        .v1 += Scalar::ONE;
    broadcasts[dealer - 1].sign(&dealer_sk);

    let err = dkg_round2_finalize::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[reporter - 1],
        &locals[reporter - 1],
        &broadcasts,
        &parties,
    )
    .expect_err("reporter must detect the bad share");

    let report =
        build_two_round_report(&states[reporter - 1], &err).expect("a complaint is defined");
    let verdict = verify_two_round_report(&parties, &broadcasts[dealer - 1], &report);
    assert_eq!(
        verdict,
        AbortVerdict::DealerGuilty {
            dealer_idx: dealer,
            reporter_idx: reporter,
        }
    );
}
