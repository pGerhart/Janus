use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::two_round::{
    Round1Broadcast, Round1LocalState, Round2Broadcast, Round2LocalState, dkg_output,
    dkg_round1_initiate, dkg_round2_finalize,
};
use janus::two_round_proofs::{
    DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams, FischlinDecomScheme,
    SchnorrDecomProof, SchnorrDecomProofParams,
};
use rand::rng;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug)]
struct FischlinProfile {
    rho: usize,
    b: usize,
    t_bits: usize,
}

const FISCHLIN_SMALL_PROOF: FischlinProfile = FischlinProfile {
    rho: 16,
    b: 8,
    t_bits: 13,
};

const FISCHLIN_LARGE_PROOF: FischlinProfile = FischlinProfile {
    rho: 43,
    b: 3,
    t_bits: 8,
};

#[derive(Clone, Copy, Debug)]
struct BaseParams {
    t: usize,
    n: usize,
}

impl BaseParams {
    fn to_dkg_params(self) -> DkgParams {
        DkgParams {
            t: self.t,
            n: self.n,
        }
    }

    fn label(self) -> String {
        format!("t{}_n{}", self.t, self.n)
    }
}

fn parameter_sets() -> Vec<BaseParams> {
    vec![
        BaseParams { t: 4, n: 16 },
        BaseParams { t: 8, n: 32 },
        BaseParams { t: 16, n: 64 },
        BaseParams { t: 32, n: 64 },
        BaseParams { t: 64, n: 128 },
        BaseParams { t: 128, n: 256 },
        BaseParams { t: 256, n: 512 },
    ]
}

// The large Fischlin profile is not recommended for large committees, so it is
// only benchmarked up to n < 256. Schnorr and the small Fischlin profile cover
// the full range.
fn parameter_sets_bounded() -> Vec<BaseParams> {
    parameter_sets().into_iter().filter(|p| p.n < 256).collect()
}

fn setup_parties(n: usize) -> (Vec<PartyState>, Parties) {
    let mut rng = rng();

    let mut party_states = Vec::with_capacity(n);
    for dealer_idx in 1..=n {
        party_states.push(make_party_state(&mut rng, dealer_idx));
    }

    let parties = collect_public_parties(&party_states);
    (party_states, parties)
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;

    if b < KB {
        format!("{} B", bytes)
    } else if b < MB {
        format!("{:.2} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

fn format_bytes_verbose(bytes: u64) -> String {
    format!("{} ({})", bytes, format_bytes(bytes))
}

fn round1_broadcast_size_bytes<P: serde::Serialize>(msg: &Round1Broadcast<P>) -> u64 {
    postcard::to_allocvec(msg)
        .expect("round1 broadcast serialization failed")
        .len() as u64
}

fn round2_broadcast_size_bytes(msg: &Round2Broadcast) -> u64 {
    postcard::to_allocvec(msg)
        .expect("round2 broadcast serialization failed")
        .len() as u64
}

fn round1_proof_size_bytes<P: serde::Serialize>(msg: &Round1Broadcast<P>) -> u64 {
    postcard::to_allocvec(&msg.decom_proof)
        .expect("round1 proof serialization failed")
        .len() as u64
}

fn received_round1_bytes_for_party<P: serde::Serialize>(
    broadcasts: &[Round1Broadcast<P>],
    receiver_idx: usize,
) -> u64 {
    broadcasts
        .iter()
        .filter(|msg| msg.dealer_idx != receiver_idx)
        .map(round1_broadcast_size_bytes)
        .sum()
}

fn received_round2_bytes_for_party(broadcasts: &[Round2Broadcast], receiver_idx: usize) -> u64 {
    broadcasts
        .iter()
        .filter(|msg| msg.dealer_idx != receiver_idx)
        .map(round2_broadcast_size_bytes)
        .sum()
}

fn setup_round1_outputs<S>(
    dkg_params: &DkgParams,
    decom_params: &S::Params,
) -> (
    Vec<PartyState>,
    Parties,
    Vec<Round1Broadcast<S::Proof>>,
    Vec<Round1LocalState>,
)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let mut rng = rng();
    let (party_states, parties) = setup_parties(dkg_params.n);

    let dealer_secrets: Vec<Scalar> = (0..dkg_params.n)
        .map(|_| Scalar::random(&mut rng))
        .collect();

    // Setup only, never measured, so run the parties in parallel. ThreadRng is
    // per-thread already, which is why each closure can take its own.
    let (round1_broadcasts, round1_locals): (Vec<_>, Vec<_>) = (1..=dkg_params.n)
        .into_par_iter()
        .map(|i| {
            dkg_round1_initiate::<_, S>(
                &mut rand::rng(),
                dkg_params,
                decom_params,
                &party_states[i - 1],
                dealer_secrets[i - 1],
                &parties,
            )
        })
        .unzip();

    (party_states, parties, round1_broadcasts, round1_locals)
}

fn setup_round2_outputs<S>(
    dkg_params: &DkgParams,
    decom_params: &S::Params,
) -> (
    Vec<PartyState>,
    Parties,
    Vec<Round1Broadcast<S::Proof>>,
    Vec<Round1LocalState>,
    Vec<Round2Broadcast>,
    Vec<Round2LocalState>,
)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let (party_states, parties, round1_broadcasts, round1_locals) =
        setup_round1_outputs::<S>(dkg_params, decom_params);

    // Setup only, never measured. Finalizing for every party costs minutes at the
    // large parameter sets, so run the parties in parallel to keep it out of the way.
    let (round2_broadcasts, round2_locals): (Vec<_>, Vec<_>) = (1..=dkg_params.n)
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
            .expect("valid round2 finalize")
        })
        .unzip();

    (
        party_states,
        parties,
        round1_broadcasts,
        round1_locals,
        round2_broadcasts,
        round2_locals,
    )
}

fn bench_two_round_initiate_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_initiate_schnorr");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = SchnorrDecomProofParams;
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_round1_initiate::<_, SchnorrDecomProof>(
            &mut setup_rng,
            &dkg_params,
            &decom_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = round1_proof_size_bytes(&sample.0);
        let broadcast_bytes = round1_broadcast_size_bytes(&sample.0);

        eprintln!(
            "[two-round schnorr initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_round1_initiate::<_, SchnorrDecomProof>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[dealer_idx - 1]),
                    black_box(share),
                    black_box(&parties),
                );
                black_box(res);
            });
        });
    }

    group.finish();
}

fn bench_two_round_finalize_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_finalize_schnorr");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = SchnorrDecomProofParams;
        let (party_states, parties, round1_broadcasts, round1_locals) =
            setup_round1_outputs::<SchnorrDecomProof>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);

        eprintln!(
            "[two-round schnorr finalize {}] round1-received={}",
            p.label(),
            format_bytes_verbose(received_round1),
        );

        group.bench_with_input(BenchmarkId::new("finalize", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_round2_finalize::<SchnorrDecomProof>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round1_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&parties),
                )
                .expect("valid schnorr finalize");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_two_round_output_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_output_schnorr");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = SchnorrDecomProofParams;

        let (
            party_states,
            parties,
            round1_broadcasts,
            _round1_locals,
            round2_broadcasts,
            round2_locals,
        ) = setup_round2_outputs::<SchnorrDecomProof>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);
        let received_round2 = received_round2_bytes_for_party(&round2_broadcasts, receiver_idx);

        eprintln!(
            "[two-round schnorr output {}] round1-received={}, round2-received={}, total={}",
            p.label(),
            format_bytes_verbose(received_round1),
            format_bytes_verbose(received_round2),
            format_bytes_verbose(received_round1 + received_round2),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output::<SchnorrDecomProof>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round2_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&round2_broadcasts),
                    black_box(&parties),
                )
                .expect("valid schnorr output");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_two_round_initiate_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_initiate_fischlin_small");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_SMALL_PROOF.rho,
            b: FISCHLIN_SMALL_PROOF.b,
            t_bits: FISCHLIN_SMALL_PROOF.t_bits,
        };
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_round1_initiate::<_, FischlinDecomScheme>(
            &mut setup_rng,
            &dkg_params,
            &decom_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = round1_proof_size_bytes(&sample.0);
        let broadcast_bytes = round1_broadcast_size_bytes(&sample.0);

        eprintln!(
            "[two-round fischlin-small initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_round1_initiate::<_, FischlinDecomScheme>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[dealer_idx - 1]),
                    black_box(share),
                    black_box(&parties),
                );
                black_box(res);
            });
        });
    }

    group.finish();
}

fn bench_two_round_finalize_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_finalize_fischlin_small");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_SMALL_PROOF.rho,
            b: FISCHLIN_SMALL_PROOF.b,
            t_bits: FISCHLIN_SMALL_PROOF.t_bits,
        };
        let (party_states, parties, round1_broadcasts, round1_locals) =
            setup_round1_outputs::<FischlinDecomScheme>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);

        eprintln!(
            "[two-round fischlin-small finalize {}] round1-received={}",
            p.label(),
            format_bytes_verbose(received_round1),
        );

        group.bench_with_input(BenchmarkId::new("finalize", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_round2_finalize::<FischlinDecomScheme>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round1_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&parties),
                )
                .expect("valid fischlin small finalize");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_two_round_output_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_output_fischlin_small");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_SMALL_PROOF.rho,
            b: FISCHLIN_SMALL_PROOF.b,
            t_bits: FISCHLIN_SMALL_PROOF.t_bits,
        };

        let (
            party_states,
            parties,
            round1_broadcasts,
            _round1_locals,
            round2_broadcasts,
            round2_locals,
        ) = setup_round2_outputs::<FischlinDecomScheme>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);
        let received_round2 = received_round2_bytes_for_party(&round2_broadcasts, receiver_idx);

        eprintln!(
            "[two-round fischlin-small output {}] round1-received={}, round2-received={}, total={}",
            p.label(),
            format_bytes_verbose(received_round1),
            format_bytes_verbose(received_round2),
            format_bytes_verbose(received_round1 + received_round2),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output::<FischlinDecomScheme>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round2_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&round2_broadcasts),
                    black_box(&parties),
                )
                .expect("valid fischlin small output");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_two_round_initiate_fischlin_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_initiate_fischlin_large");
    group.sample_size(20);

    for p in parameter_sets_bounded() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_LARGE_PROOF.rho,
            b: FISCHLIN_LARGE_PROOF.b,
            t_bits: FISCHLIN_LARGE_PROOF.t_bits,
        };
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_round1_initiate::<_, FischlinDecomScheme>(
            &mut setup_rng,
            &dkg_params,
            &decom_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = round1_proof_size_bytes(&sample.0);
        let broadcast_bytes = round1_broadcast_size_bytes(&sample.0);

        eprintln!(
            "[two-round fischlin-large initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_round1_initiate::<_, FischlinDecomScheme>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[dealer_idx - 1]),
                    black_box(share),
                    black_box(&parties),
                );
                black_box(res);
            });
        });
    }

    group.finish();
}

fn bench_two_round_finalize_fischlin_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_finalize_fischlin_large");
    group.sample_size(20);

    for p in parameter_sets_bounded() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_LARGE_PROOF.rho,
            b: FISCHLIN_LARGE_PROOF.b,
            t_bits: FISCHLIN_LARGE_PROOF.t_bits,
        };
        let (party_states, parties, round1_broadcasts, round1_locals) =
            setup_round1_outputs::<FischlinDecomScheme>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);

        eprintln!(
            "[two-round fischlin-large finalize {}] round1-received={}",
            p.label(),
            format_bytes_verbose(received_round1),
        );

        group.bench_with_input(BenchmarkId::new("finalize", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_round2_finalize::<FischlinDecomScheme>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round1_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&parties),
                )
                .expect("valid fischlin large finalize");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_two_round_output_fischlin_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_round_output_fischlin_large");
    group.sample_size(20);

    for p in parameter_sets_bounded() {
        let dkg_params = p.to_dkg_params();
        let decom_params = FischlinDecomProofParams {
            rho: FISCHLIN_LARGE_PROOF.rho,
            b: FISCHLIN_LARGE_PROOF.b,
            t_bits: FISCHLIN_LARGE_PROOF.t_bits,
        };

        let (
            party_states,
            parties,
            round1_broadcasts,
            _round1_locals,
            round2_broadcasts,
            round2_locals,
        ) = setup_round2_outputs::<FischlinDecomScheme>(&dkg_params, &decom_params);

        let receiver_idx = 1usize;
        let received_round1 = received_round1_bytes_for_party(&round1_broadcasts, receiver_idx);
        let received_round2 = received_round2_bytes_for_party(&round2_broadcasts, receiver_idx);

        eprintln!(
            "[two-round fischlin-large output {}] round1-received={}, round2-received={}, total={}",
            p.label(),
            format_bytes_verbose(received_round1),
            format_bytes_verbose(received_round2),
            format_bytes_verbose(received_round1 + received_round2),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output::<FischlinDecomScheme>(
                    black_box(&dkg_params),
                    black_box(&decom_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&round2_locals[receiver_idx - 1]),
                    black_box(&round1_broadcasts),
                    black_box(&round2_broadcasts),
                    black_box(&parties),
                )
                .expect("valid fischlin large output");

                black_box(out);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_two_round_initiate_schnorr,
    bench_two_round_finalize_schnorr,
    bench_two_round_output_schnorr,
    bench_two_round_initiate_fischlin_small,
    bench_two_round_finalize_fischlin_small,
    bench_two_round_output_fischlin_small,
    bench_two_round_initiate_fischlin_large,
    bench_two_round_finalize_fischlin_large,
    bench_two_round_output_fischlin_large,
);

criterion_main!(benches);
