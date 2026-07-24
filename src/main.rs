#![forbid(unsafe_code)]

use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, dkg_initiate, dkg_output_key_generation,
};
use janus::one_round_proofs::polyproof_bulletproof::make_bulletproof_params;
use janus::one_round_proofs::{
    BulletproofPolyProof, FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};

use janus::two_round::{
    Round1Broadcast, Round1LocalState, Round2Broadcast, Round2LocalState,
    dkg_output as dkg_two_round_output, dkg_round1_initiate, dkg_round2_finalize,
};

use curve25519_dalek::scalar::Scalar;
use janus::two_round_proofs::{
    DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams, FischlinDecomScheme,
    SchnorrDecomProof, SchnorrDecomProofParams,
};
use janus::{DkgOutput, DkgParams};
use rayon::prelude::*;
use std::time::Instant;

fn main() {
    let params = DkgParams { t: 16, n: 32 };
    main_one_round(&params);
    println!();
    println!("==================================================");
    println!();
    main_two_round(&params);
}

fn main_one_round(dkg_params: &DkgParams) {
    println!(
        "Running DKG with t = {}, n = {}",
        dkg_params.t, dkg_params.n
    );
    println!();

    run_case::<SchnorrPolyProof>("Schnorr", dkg_params, &());
    run_case::<BulletproofPolyProof>(
        "Bulletproof",
        dkg_params,
        &make_bulletproof_params(dkg_params.t + 1, dkg_params.n),
    );
    run_case::<FischlinPolyProof>(
        "Fischlin small proof / high prover work",
        dkg_params,
        &FischlinProofParams {
            rho: 16,
            b: 8,
            t_bits: 13,
        },
    );

    run_case::<FischlinPolyProof>(
        "Fischlin large proof / low prover work",
        dkg_params,
        &FischlinProofParams {
            rho: 43,
            b: 3,
            t_bits: 8,
        },
    );
}

fn main_two_round(dkg_params: &DkgParams) {
    println!(
        "Running TWO-ROUND DKG with t = {}, n = {}",
        dkg_params.t, dkg_params.n
    );
    println!();

    // Schnorr
    run_two_round_case::<SchnorrDecomProof>("Schnorr", dkg_params, &SchnorrDecomProofParams);

    // Fischlin small proof / high prover work
    run_two_round_case::<FischlinDecomScheme>(
        "Fischlin small proof / high prover work",
        dkg_params,
        &FischlinDecomProofParams {
            rho: 16,
            b: 8,
            t_bits: 13,
        },
    );

    // Fischlin large proof / low prover work
    run_two_round_case::<FischlinDecomScheme>(
        "Fischlin large proof / low prover work",
        dkg_params,
        &FischlinDecomProofParams {
            rho: 43,
            b: 3,
            t_bits: 8,
        },
    );
}
fn run_case<S>(label: &str, dkg_params: &DkgParams, proof_params: &S::Params)
where
    S::Params: Send + Sync,
    S: PolyProofScheme + Send + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    println!("=== {} ===", label);

    let start = Instant::now();

    match run_dkg_once_for_test::<S>(dkg_params, proof_params) {
        Ok((outputs, broadcasts)) => print_success(&outputs, &broadcasts, start.elapsed()),
        Err(e) => {
            let duration = start.elapsed();
            eprintln!("DKG run failed ");
            eprintln!("Error: {}", e);
            eprintln!("Time until failure: {:.3?}", duration);
        }
    }

    println!();
}

fn run_two_round_case<S>(label: &str, dkg_params: &DkgParams, decom_params: &S::Params)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness> + Send + Sync,
    S::Params: Clone + std::fmt::Debug + Send + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    println!("=== {} ===", label);

    let start = std::time::Instant::now();

    match run_two_round_dkg_once_for_test::<S>(dkg_params, decom_params) {
        Ok((outputs, round1, round2)) => {
            print_two_round_success(&outputs, &round1, &round2, start.elapsed())
        }
        Err(e) => {
            let duration = start.elapsed();
            eprintln!("Two-round DKG run failed");
            eprintln!("Error: {}", e);
            eprintln!("Time until failure: {:.3?}", duration);
        }
    }

    println!();
}

fn format_bytes(bytes: usize) -> String {
    let b = bytes as f64;

    if b < 1024.0 {
        format!("{:.0} B", b)
    } else if b < 1024.0 * 1024.0 {
        format!("{:.2} KB", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB", b / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", b / (1024.0 * 1024.0 * 1024.0))
    }
}

fn print_success<P: serde::Serialize>(
    outputs: &[DkgOutput],
    broadcasts: &[DkgInitBroadcast<P>],
    duration: std::time::Duration,
) {
    println!("DKG run successful");
    println!("Public key:");
    println!("{:?}", outputs[0].public_key.compress().as_bytes());
    println!("Time taken:");
    println!("{:.3?}", duration);

    let sizes: Vec<usize> = broadcasts
        .iter()
        .map(|b| bincode::serialize(b).expect("serialize broadcast").len())
        .collect();

    let n = broadcasts.len();

    let sent_per_party = sizes[0]; // alle gleich
    let total_sent: usize = sent_per_party * n;

    let received_per_party = sent_per_party * (n - 1);
    let total_received = received_per_party * n;

    println!("Communication:");
    println!("  sent per party:     {}", format_bytes(sent_per_party));
    println!("  received per party: {}", format_bytes(received_per_party));
    println!("  total sent:         {}", format_bytes(total_sent));
    println!("  total received:     {}", format_bytes(total_received));
}

fn print_two_round_success<P: serde::Serialize>(
    outputs: &[DkgOutput],
    round1_broadcasts: &[Round1Broadcast<P>],
    round2_broadcasts: &[Round2Broadcast],
    duration: std::time::Duration,
) {
    println!("Two-round DKG run successful");
    println!("Public key:");
    println!("{:?}", outputs[0].public_key.compress().as_bytes());
    println!("Time taken:");
    println!("{:.3?}", duration);

    let round1_sizes: Vec<usize> = round1_broadcasts
        .iter()
        .map(|b| {
            bincode::serialize(b)
                .expect("serialize round1 broadcast")
                .len()
        })
        .collect();

    let round2_sizes: Vec<usize> = round2_broadcasts
        .iter()
        .map(|b| {
            bincode::serialize(b)
                .expect("serialize round2 broadcast")
                .len()
        })
        .collect();

    let n = outputs.len();

    let sent_per_party_round1 = round1_sizes[0];
    let sent_per_party_round2 = round2_sizes[0];
    let sent_per_party = sent_per_party_round1 + sent_per_party_round2;

    let total_sent = sent_per_party * n;
    let received_per_party = sent_per_party * (n - 1);
    let total_received = received_per_party * n;

    println!("Communication:");
    println!(
        "  round 1 sent per party: {}",
        format_bytes(sent_per_party_round1)
    );
    println!(
        "  round 2 sent per party: {}",
        format_bytes(sent_per_party_round2)
    );
    println!("  sent per party:         {}", format_bytes(sent_per_party));
    println!(
        "  received per party:     {}",
        format_bytes(received_per_party)
    );
    println!("  total sent:             {}", format_bytes(total_sent));
    println!("  total received:         {}", format_bytes(total_received));
}

pub fn run_two_round_dkg_once_for_test<S>(
    dkg_params: &DkgParams,
    decom_params: &S::Params,
) -> Result<
    (
        Vec<DkgOutput>,
        Vec<Round1Broadcast<S::Proof>>,
        Vec<Round2Broadcast>,
    ),
    String,
>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness> + Send + Sync,
    S::Params: Clone + std::fmt::Debug + Send + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let n = dkg_params.n;
    let t = dkg_params.t;

    if n == 0 {
        return Err("n must be > 0".to_string());
    }
    if t >= n {
        return Err("require t < n".to_string());
    }

    let mut rng = rand::thread_rng();

    let mut party_states: Vec<PartyState> = Vec::with_capacity(n);
    for dealer_idx in 1..=n {
        party_states.push(make_party_state(&mut rng, dealer_idx));
    }

    let parties: Parties = collect_public_parties(&party_states);
    let dealer_secrets: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();

    let round1_results: Vec<(Round1Broadcast<S::Proof>, Round1LocalState)> = (1..=n)
        .into_par_iter()
        .map(|i| {
            let mut local_rng = rand::thread_rng();
            dkg_round1_initiate::<_, S>(
                &mut local_rng,
                dkg_params,
                decom_params,
                &party_states[i - 1],
                dealer_secrets[i - 1],
                &parties,
            )
        })
        .collect();

    let round1_broadcasts: Vec<Round1Broadcast<S::Proof>> =
        round1_results.iter().cloned().map(|r| r.0).collect();

    let round1_locals: Vec<Round1LocalState> = round1_results.into_iter().map(|r| r.1).collect();

    let round2_results: Result<Vec<(Round2Broadcast, Round2LocalState)>, String> = (1..=n)
        .into_par_iter()
        .map(|i| {
            dkg_round2_finalize::<S>(
                dkg_params,
                decom_params,
                &party_states[i - 1],
                &round1_locals[i - 1],
                &round1_broadcasts,
                &parties,
            )
            .map_err(|e| format!("round2 failed for party {}: {:?}", i, e))
        })
        .collect();

    let round2_results = round2_results?;
    let round2_broadcasts: Vec<Round2Broadcast> =
        round2_results.iter().cloned().map(|r| r.0).collect();
    let round2_locals: Vec<Round2LocalState> = round2_results.into_iter().map(|r| r.1).collect();

    let outputs: Result<Vec<DkgOutput>, String> = (1..=n)
        .into_par_iter()
        .map(|i| {
            dkg_two_round_output::<S>(
                dkg_params,
                decom_params,
                &party_states[i - 1],
                &round2_locals[i - 1],
                &round1_broadcasts,
                &round2_broadcasts,
                &parties,
            )
            .map_err(|e| format!("output failed for party {}: {:?}", i, e))
        })
        .collect();

    let outputs = outputs?;
    check_dkg_outputs(&outputs)?;

    Ok((outputs, round1_broadcasts, round2_broadcasts))
}

pub fn run_dkg_once_for_test<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
) -> Result<(Vec<DkgOutput>, Vec<DkgInitBroadcast<S::Proof>>), String>
where
    S: PolyProofScheme + Send + Sync,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let n = dkg_params.n;
    let t = dkg_params.t;

    if n == 0 {
        return Err("n must be > 0".to_string());
    }
    if t >= n {
        return Err("require t < n".to_string());
    }

    let mut rng = rand::thread_rng();

    let mut party_states: Vec<PartyState> = Vec::with_capacity(n);
    for dealer_idx in 1..=n {
        party_states.push(make_party_state(&mut rng, dealer_idx));
    }

    let parties: Parties = collect_public_parties(&party_states);
    let dealer_secrets: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();

    let init_results: Vec<_> = (1..=n)
        .into_par_iter()
        .map(|i| {
            let mut local_rng = rand::thread_rng();
            dkg_initiate::<_, S>(
                &mut local_rng,
                dkg_params,
                proof_params,
                &party_states[i - 1],
                dealer_secrets[i - 1],
                &parties,
            )
        })
        .collect();

    let broadcasts: Vec<DkgInitBroadcast<S::Proof>> =
        init_results.iter().cloned().map(|r| r.broadcast).collect();

    let locals: Vec<DkgInitLocalState> = init_results.into_iter().map(|r| r.local).collect();

    let outputs: Result<Vec<DkgOutput>, String> = (1..=n)
        .into_par_iter()
        .map(|i| {
            dkg_output_key_generation::<S>(
                dkg_params,
                proof_params,
                &party_states[i - 1],
                &locals[i - 1],
                &broadcasts,
                &parties,
            )
            .map_err(|e| format!("output failed for party {}: {:?}", i, e))
        })
        .collect();

    let outputs = outputs?;
    check_dkg_outputs(&outputs)?;
    Ok((outputs, broadcasts))
}

pub fn check_dkg_outputs(outputs: &[DkgOutput]) -> Result<(), String> {
    if outputs.is_empty() {
        return Err("outputs must not be empty".to_string());
    }

    let n = outputs.len();

    let mut seen = vec![false; n];
    for out in outputs {
        let i = out.idx;
        if i == 0 || i > n {
            return Err(format!("invalid idx {}", i));
        }
        if seen[i - 1] {
            return Err(format!("duplicate idx {}", i));
        }
        seen[i - 1] = true;
    }

    let reference_pk = outputs[0].public_key;
    for out in outputs.iter().skip(1) {
        if out.public_key != reference_pk {
            return Err(format!(
                "public key mismatch: party {} differs from party 1",
                out.idx
            ));
        }
    }

    let reference_vks = &outputs[0].partial_verification_keys;
    if reference_vks.len() != n {
        return Err(format!(
            "reference verification key vector has wrong length: expected {}, got {}",
            n,
            reference_vks.len()
        ));
    }

    for out in outputs.iter().skip(1) {
        if out.partial_verification_keys.len() != n {
            return Err(format!(
                "verification key vector of party {} has wrong length: expected {}, got {}",
                out.idx,
                n,
                out.partial_verification_keys.len()
            ));
        }

        if out.partial_verification_keys != *reference_vks {
            return Err(format!(
                "verification key vector mismatch: party {} differs from party 1",
                out.idx
            ));
        }
    }

    for out in outputs {
        let i = out.idx;
        let vk_i = &reference_vks[i - 1];

        if !vk_i.matches_opening(out.secret_share, out.blinding_share) {
            return Err(format!(
                "opening mismatch for party {}: (secret_share, blinding_share) does not open vk_{}",
                i, i
            ));
        }
    }

    Ok(())
}
