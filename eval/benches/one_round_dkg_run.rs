use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, dkg_initiate, dkg_output_key_generation,
};

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use rand::rng;

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

// JANUS_BENCH_MAX_N caps the committee size, since Criterion filters measurements
// but not the setup that precedes them.
fn parameter_sets() -> Vec<BaseParams> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    all_parameter_sets()
        .into_iter()
        .filter(|p| p.n <= max_n)
        .collect()
}

fn all_parameter_sets() -> Vec<BaseParams> {
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

fn setup_broadcasts_for_output<S>(
    dkg_params: &DkgParams,
    proof_params: &S::Params,
) -> (
    Vec<PartyState>,
    Parties,
    Vec<DkgInitBroadcast<S::Proof>>,
    Vec<DkgInitLocalState>,
)
where
    S: PolyProofScheme,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let mut rng = rng();
    let (party_states, parties) = setup_parties(dkg_params.n);

    let dealer_secrets: Vec<Scalar> = (0..dkg_params.n)
        .map(|_| Scalar::random(&mut rng))
        .collect();

    let mut broadcasts = Vec::with_capacity(dkg_params.n);
    let mut locals = Vec::with_capacity(dkg_params.n);

    for i in 1..=dkg_params.n {
        let res = dkg_initiate::<_, S>(
            &mut rng,
            dkg_params,
            proof_params,
            &party_states[i - 1],
            dealer_secrets[i - 1],
            &parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }

    (party_states, parties, broadcasts, locals)
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

fn proof_size_bytes<P: serde::Serialize>(msg: &DkgInitBroadcast<P>) -> u64 {
    postcard::to_allocvec(&msg.proof)
        .expect("proof serialization failed")
        .len() as u64
}

fn broadcast_size_bytes<P: serde::Serialize>(msg: &DkgInitBroadcast<P>) -> u64 {
    postcard::to_allocvec(msg)
        .expect("broadcast serialization failed")
        .len() as u64
}

fn received_bytes_for_party<P: serde::Serialize>(
    broadcasts: &[DkgInitBroadcast<P>],
    receiver_idx: usize, // 1-based
) -> u64 {
    broadcasts
        .iter()
        .filter(|msg| msg.dealer_idx != receiver_idx)
        .map(broadcast_size_bytes)
        .sum()
}

fn bench_one_party_initiate_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_initiate_schnorr");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let proof_params = ();
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_initiate::<_, SchnorrPolyProof>(
            &mut setup_rng,
            &dkg_params,
            &proof_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = proof_size_bytes(&sample.broadcast);
        let broadcast_bytes = broadcast_size_bytes(&sample.broadcast);

        eprintln!(
            "[schnorr initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_initiate::<_, SchnorrPolyProof>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&proof_params),
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

fn bench_one_party_output_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_output_schnorr");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let proof_params = ();
        let (party_states, parties, broadcasts, locals) =
            setup_broadcasts_for_output::<SchnorrPolyProof>(&dkg_params, &proof_params);

        let receiver_idx = 1usize;
        let received_bytes = received_bytes_for_party(&broadcasts, receiver_idx);

        eprintln!(
            "[schnorr output {}] received={}",
            p.label(),
            format_bytes_verbose(received_bytes),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output_key_generation::<SchnorrPolyProof>(
                    black_box(&dkg_params),
                    black_box(&proof_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&locals[receiver_idx - 1]),
                    black_box(&broadcasts),
                    black_box(&parties),
                )
                .expect("valid schnorr output");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_one_party_initiate_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_initiate_fischlin_small");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let proof_params = FischlinProofParams {
            rho: FISCHLIN_SMALL_PROOF.rho,
            b: FISCHLIN_SMALL_PROOF.b,
            t_bits: FISCHLIN_SMALL_PROOF.t_bits,
        };
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_initiate::<_, FischlinPolyProof>(
            &mut setup_rng,
            &dkg_params,
            &proof_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = proof_size_bytes(&sample.broadcast);
        let broadcast_bytes = broadcast_size_bytes(&sample.broadcast);

        eprintln!(
            "[fischlin-small initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_initiate::<_, FischlinPolyProof>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&proof_params),
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

fn bench_one_party_output_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_output_fischlin_small");
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg_params = p.to_dkg_params();
        let proof_params = FischlinProofParams {
            rho: FISCHLIN_SMALL_PROOF.rho,
            b: FISCHLIN_SMALL_PROOF.b,
            t_bits: FISCHLIN_SMALL_PROOF.t_bits,
        };
        let (party_states, parties, broadcasts, locals) =
            setup_broadcasts_for_output::<FischlinPolyProof>(&dkg_params, &proof_params);

        let receiver_idx = 1usize;
        let received_bytes = received_bytes_for_party(&broadcasts, receiver_idx);

        eprintln!(
            "[fischlin-small output {}] received={}",
            p.label(),
            format_bytes_verbose(received_bytes),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output_key_generation::<FischlinPolyProof>(
                    black_box(&dkg_params),
                    black_box(&proof_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&locals[receiver_idx - 1]),
                    black_box(&broadcasts),
                    black_box(&parties),
                )
                .expect("valid fischlin small output");

                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_one_party_initiate_fischlin_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_initiate_fischlin_large");
    group.sample_size(20);

    for p in parameter_sets_bounded() {
        let dkg_params = p.to_dkg_params();
        let proof_params = FischlinProofParams {
            rho: FISCHLIN_LARGE_PROOF.rho,
            b: FISCHLIN_LARGE_PROOF.b,
            t_bits: FISCHLIN_LARGE_PROOF.t_bits,
        };
        let (party_states, parties) = setup_parties(dkg_params.n);
        let dealer_idx = 1usize;
        let share = Scalar::from(42u64);

        let mut setup_rng = rng();
        let sample = dkg_initiate::<_, FischlinPolyProof>(
            &mut setup_rng,
            &dkg_params,
            &proof_params,
            &party_states[dealer_idx - 1],
            share,
            &parties,
        );

        let proof_bytes = proof_size_bytes(&sample.broadcast);
        let broadcast_bytes = broadcast_size_bytes(&sample.broadcast);

        eprintln!(
            "[fischlin-large initiate {}] proof={}, broadcast={}",
            p.label(),
            format_bytes_verbose(proof_bytes),
            format_bytes_verbose(broadcast_bytes),
        );

        group.bench_with_input(BenchmarkId::new("initiate", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut rng = rng();
                let res = dkg_initiate::<_, FischlinPolyProof>(
                    &mut rng,
                    black_box(&dkg_params),
                    black_box(&proof_params),
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

fn bench_one_party_output_fischlin_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("one_round_output_fischlin_large");
    group.sample_size(20);

    for p in parameter_sets_bounded() {
        let dkg_params = p.to_dkg_params();
        let proof_params = FischlinProofParams {
            rho: FISCHLIN_LARGE_PROOF.rho,
            b: FISCHLIN_LARGE_PROOF.b,
            t_bits: FISCHLIN_LARGE_PROOF.t_bits,
        };
        let (party_states, parties, broadcasts, locals) =
            setup_broadcasts_for_output::<FischlinPolyProof>(&dkg_params, &proof_params);

        let receiver_idx = 1usize;
        let received_bytes = received_bytes_for_party(&broadcasts, receiver_idx);

        eprintln!(
            "[fischlin-large output {}] received={}",
            p.label(),
            format_bytes_verbose(received_bytes),
        );

        group.bench_with_input(BenchmarkId::new("output", p.label()), &p, |b, _| {
            b.iter(|| {
                let out = dkg_output_key_generation::<FischlinPolyProof>(
                    black_box(&dkg_params),
                    black_box(&proof_params),
                    black_box(&party_states[receiver_idx - 1]),
                    black_box(&locals[receiver_idx - 1]),
                    black_box(&broadcasts),
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
    bench_one_party_initiate_schnorr,
    bench_one_party_output_schnorr,
    bench_one_party_initiate_fischlin_small,
    bench_one_party_output_fischlin_small,
    bench_one_party_initiate_fischlin_large,
    bench_one_party_output_fischlin_large,
);

criterion_main!(benches);
