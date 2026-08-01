use curve25519_dalek::scalar::Scalar;
use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, dkg_initiate, dkg_output_key_generation,
};
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, Round1LocalState, Round2Broadcast, Round2LocalState,
    dkg_output as dkg_two_round_output, dkg_round1_initiate, dkg_round2_finalize,
};
use janus::two_round_proofs::{
    DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams, FischlinDecomScheme,
    SchnorrDecomProof, SchnorrDecomProofParams,
};
use janus::{DkgOutput, DkgParams};
use rand::rng;

// Small parameters: fast for tests, still exercise the full protocol
const PARAMS_SMALL: DkgParams = DkgParams { t: 2, n: 5 };
const PARAMS_MID: DkgParams = DkgParams { t: 4, n: 9 };

// helpers

fn setup(n: usize) -> (Vec<PartyState>, Parties, Vec<Scalar>) {
    let mut rng = rng();
    let party_states: Vec<PartyState> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&party_states);
    let secrets: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();
    (party_states, parties, secrets)
}

fn check_outputs(outputs: &[DkgOutput]) {
    assert!(!outputs.is_empty(), "no outputs");

    // all indices present exactly once
    let n = outputs.len();
    let mut seen = vec![false; n];
    for o in outputs {
        assert!(o.idx >= 1 && o.idx <= n, "idx out of range: {}", o.idx);
        assert!(!seen[o.idx - 1], "duplicate idx {}", o.idx);
        seen[o.idx - 1] = true;
    }

    // all parties agree on the public key
    let pk = outputs[0].public_key;
    for o in outputs.iter().skip(1) {
        assert_eq!(o.public_key, pk, "public key mismatch at party {}", o.idx);
    }

    // all parties agree on the partial verification keys
    let vks = &outputs[0].partial_verification_keys;
    assert_eq!(vks.len(), n);
    for o in outputs.iter().skip(1) {
        assert_eq!(
            o.partial_verification_keys, *vks,
            "vk vector mismatch at party {}",
            o.idx
        );
    }

    // every party's own share opens their vk
    for o in outputs {
        assert!(
            vks[o.idx - 1].matches_opening(o.secret_share, o.blinding_share),
            "share does not open vk for party {}",
            o.idx
        );
    }
}

fn run_one_round<S>(params: &DkgParams, proof_params: &S::Params) -> Vec<DkgOutput>
where
    S: PolyProofScheme + Send + Sync,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let n = params.n;
    let (party_states, parties, secrets) = setup(n);
    let mut rng = rng();

    let init_results: Vec<_> = (1..=n)
        .map(|i| {
            dkg_initiate::<_, S>(
                &mut rng,
                params,
                proof_params,
                &party_states[i - 1],
                secrets[i - 1],
                &parties,
            )
        })
        .collect();

    let broadcasts: Vec<DkgInitBroadcast<S::Proof>> =
        init_results.iter().map(|r| r.broadcast.clone()).collect();
    let locals: Vec<DkgInitLocalState> = init_results.into_iter().map(|r| r.local).collect();

    let outputs: Vec<DkgOutput> = (1..=n)
        .map(|i| {
            dkg_output_key_generation::<S>(
                params,
                proof_params,
                &party_states[i - 1],
                &locals[i - 1],
                &broadcasts,
                &parties,
            )
            .unwrap_or_else(|e| panic!("one-round output failed for party {i}: {e:?}"))
        })
        .collect();

    check_outputs(&outputs);
    outputs
}

fn run_two_round<S>(params: &DkgParams, decom_params: &S::Params) -> Vec<DkgOutput>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness> + Send + Sync,
    S::Params: Clone + std::fmt::Debug + Send + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let n = params.n;
    let (party_states, parties, secrets) = setup(n);
    let mut rng = rng();

    let round1_results: Vec<(Round1Broadcast<S::Proof>, Round1LocalState)> = (1..=n)
        .map(|i| {
            dkg_round1_initiate::<_, S>(
                &mut rng,
                params,
                decom_params,
                &party_states[i - 1],
                secrets[i - 1],
                &parties,
            )
        })
        .collect();

    let round1_broadcasts: Vec<Round1Broadcast<S::Proof>> =
        round1_results.iter().map(|r| r.0.clone()).collect();
    let round1_locals: Vec<Round1LocalState> = round1_results.into_iter().map(|r| r.1).collect();

    let round2_results: Vec<(Round2Broadcast, Round2LocalState)> = (1..=n)
        .map(|i| {
            dkg_round2_finalize::<S>(
                params,
                decom_params,
                &party_states[i - 1],
                &round1_locals[i - 1],
                &round1_broadcasts,
                &parties,
            )
            .unwrap_or_else(|e| panic!("round2 failed for party {i}: {e:?}"))
        })
        .collect();

    let round2_broadcasts: Vec<Round2Broadcast> =
        round2_results.iter().map(|r| r.0.clone()).collect();
    let round2_locals: Vec<Round2LocalState> = round2_results.into_iter().map(|r| r.1).collect();

    let outputs: Vec<DkgOutput> = (1..=n)
        .map(|i| {
            dkg_two_round_output::<S>(
                params,
                decom_params,
                &party_states[i - 1],
                &round2_locals[i - 1],
                &round1_broadcasts,
                &round2_broadcasts,
                &parties,
            )
            .unwrap_or_else(|e| panic!("two-round output failed for party {i}: {e:?}"))
        })
        .collect();

    check_outputs(&outputs);
    outputs
}

// one-round tests

#[test]
fn one_round_schnorr_small() {
    run_one_round::<SchnorrPolyProof>(&PARAMS_SMALL, &());
}

#[test]
fn one_round_schnorr_mid() {
    run_one_round::<SchnorrPolyProof>(&PARAMS_MID, &());
}

#[test]
fn one_round_fischlin_small() {
    let proof_params = FischlinProofParams {
        rho: 4,
        b: 4,
        t_bits: 9,
    };
    run_one_round::<FischlinPolyProof>(&PARAMS_SMALL, &proof_params);
}

// two-round tests

#[test]
fn two_round_schnorr_small() {
    run_two_round::<SchnorrDecomProof>(&PARAMS_SMALL, &SchnorrDecomProofParams);
}

#[test]
fn two_round_schnorr_mid() {
    run_two_round::<SchnorrDecomProof>(&PARAMS_MID, &SchnorrDecomProofParams);
}

#[test]
fn two_round_fischlin_small() {
    let decom_params = FischlinDecomProofParams {
        rho: 4,
        b: 4,
        t_bits: 9,
    };
    run_two_round::<FischlinDecomScheme>(&PARAMS_SMALL, &decom_params);
}
