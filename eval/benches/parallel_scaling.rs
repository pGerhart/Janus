//! Every measured phase of both protocols, once on one core and once on all of
//! them, over the full range of committee sizes.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, dkg_initiate, dkg_output_key_generation,
    dkg_output_key_generation_parallel,
};
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, Round1LocalState, Round2Broadcast, Round2LocalState, dkg_output,
    dkg_output_parallel, dkg_round1_initiate, dkg_round2_finalize, dkg_round2_finalize_parallel,
};
use janus::two_round_proofs::{
    DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams, FischlinDecomScheme,
    SchnorrDecomProof, SchnorrDecomProofParams,
};
use rand::rng;
use rayon::prelude::*;

const FISCHLIN_SMALL: (usize, usize, usize) = (16, 8, 13); // rho, b, t_bits
const FISCHLIN_LARGE: (usize, usize, usize) = (43, 3, 8);

#[derive(Clone, Copy)]
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

// JANUS_BENCH_MAX_N caps the committee size, since a Criterion filter still runs
// every setup. Useful to smoke-test the suite before handing it a whole machine.
fn parameter_sets() -> Vec<BaseParams> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    [
        BaseParams { t: 8, n: 16 },
        BaseParams { t: 16, n: 32 },
        BaseParams { t: 32, n: 64 },
        BaseParams { t: 64, n: 128 },
        BaseParams { t: 128, n: 256 },
        BaseParams { t: 256, n: 512 },
    ]
    .into_iter()
    .filter(|p| p.n <= max_n)
    .collect()
}

// The large Fischlin profile is not the recommended choice at large committees.
fn parameter_sets_bounded() -> Vec<BaseParams> {
    parameter_sets().into_iter().filter(|p| p.n < 256).collect()
}

fn setup_parties(n: usize) -> (Vec<PartyState>, Parties) {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&states);
    (states, parties)
}

fn setup_one_round<S>(
    dkg: &DkgParams,
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
    let (states, parties) = setup_parties(dkg.n);
    // Setup only, never measured, so let the dealers run side by side.
    let (broadcasts, locals): (Vec<_>, Vec<_>) = (1..=dkg.n)
        .into_par_iter()
        .map(|i| {
            let res = dkg_initiate::<_, S>(
                &mut rng(),
                dkg,
                proof_params,
                &states[i - 1],
                Scalar::from((i + 1) as u64),
                &parties,
            );
            (res.broadcast, res.local)
        })
        .unzip();
    (states, parties, broadcasts, locals)
}

type TwoRoundSetup<S> = (
    Vec<PartyState>,
    Parties,
    Vec<Round1Broadcast<<S as DecomProofScheme>::Proof>>,
    Vec<Round1LocalState>,
    Vec<Round2Broadcast>,
    Vec<Round2LocalState>,
);

fn setup_two_round<S>(dkg: &DkgParams, decom_params: &S::Params) -> TwoRoundSetup<S>
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let (states, parties) = setup_parties(dkg.n);
    let (r1, r1_locals): (Vec<_>, Vec<_>) = (1..=dkg.n)
        .into_par_iter()
        .map(|i| {
            dkg_round1_initiate::<_, S>(
                &mut rng(),
                dkg,
                decom_params,
                &states[i - 1],
                Scalar::from((i + 1) as u64),
                &parties,
            )
        })
        .unzip();

    // Finalizing on behalf of every party costs minutes at the largest sets, so
    // this runs in parallel too. It is setup, not a measurement.
    let (r2, r2_locals): (Vec<_>, Vec<_>) = (1..=dkg.n)
        .into_par_iter()
        .map(|i| {
            dkg_round2_finalize::<S>(
                dkg,
                decom_params,
                &states[i - 1],
                &r1_locals[i - 1],
                &r1,
                &parties,
            )
            .expect("valid round2 finalize")
        })
        .unzip();

    (states, parties, r1, r1_locals, r2, r2_locals)
}

fn bench_one_round<S>(c: &mut Criterion, name: &str, proof_params: S::Params, sets: Vec<BaseParams>)
where
    S: PolyProofScheme,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let mut group = c.benchmark_group(format!("par_one_round_{name}"));
    group.sample_size(10);

    for p in sets {
        let dkg = p.to_dkg_params();
        let (states, parties, broadcasts, locals) = setup_one_round::<S>(&dkg, &proof_params);
        let recv = 1usize;

        group.bench_with_input(BenchmarkId::new("output_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output_key_generation::<S>(
                        &dkg,
                        &proof_params,
                        &states[recv - 1],
                        &locals[recv - 1],
                        &broadcasts,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("output_par", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output_key_generation_parallel::<S>(
                        &dkg,
                        &proof_params,
                        &states[recv - 1],
                        &locals[recv - 1],
                        &broadcasts,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
    }
    group.finish();
}

fn bench_two_round<S>(c: &mut Criterion, name: &str, decom_params: S::Params, sets: Vec<BaseParams>)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let mut group = c.benchmark_group(format!("par_two_round_{name}"));
    group.sample_size(10);

    for p in sets {
        let dkg = p.to_dkg_params();
        let (states, parties, r1, r1_locals, r2, r2_locals) =
            setup_two_round::<S>(&dkg, &decom_params);
        let recv = 1usize;

        group.bench_with_input(BenchmarkId::new("finalize_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_round2_finalize::<S>(
                        &dkg,
                        &decom_params,
                        &states[recv - 1],
                        &r1_locals[recv - 1],
                        &r1,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("finalize_par", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_round2_finalize_parallel::<S>(
                        &dkg,
                        &decom_params,
                        &states[recv - 1],
                        &r1_locals[recv - 1],
                        &r1,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("output_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output::<S>(
                        &dkg,
                        &decom_params,
                        &states[recv - 1],
                        &r2_locals[recv - 1],
                        &r1,
                        &r2,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("output_par", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output_parallel::<S>(
                        &dkg,
                        &decom_params,
                        &states[recv - 1],
                        &r2_locals[recv - 1],
                        &r1,
                        &r2,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
    }
    group.finish();
}

fn fischlin_poly(profile: (usize, usize, usize)) -> FischlinProofParams {
    FischlinProofParams {
        rho: profile.0,
        b: profile.1,
        t_bits: profile.2,
    }
}

fn fischlin_decom(profile: (usize, usize, usize)) -> FischlinDecomProofParams {
    FischlinDecomProofParams {
        rho: profile.0,
        b: profile.1,
        t_bits: profile.2,
    }
}

fn benches(c: &mut Criterion) {
    bench_one_round::<SchnorrPolyProof>(c, "schnorr", (), parameter_sets());
    bench_one_round::<FischlinPolyProof>(
        c,
        "fischlin_small",
        fischlin_poly(FISCHLIN_SMALL),
        parameter_sets(),
    );
    bench_one_round::<FischlinPolyProof>(
        c,
        "fischlin_large",
        fischlin_poly(FISCHLIN_LARGE),
        parameter_sets_bounded(),
    );

    bench_two_round::<SchnorrDecomProof>(c, "schnorr", SchnorrDecomProofParams, parameter_sets());
    bench_two_round::<FischlinDecomScheme>(
        c,
        "fischlin_small",
        fischlin_decom(FISCHLIN_SMALL),
        parameter_sets(),
    );
    bench_two_round::<FischlinDecomScheme>(
        c,
        "fischlin_large",
        fischlin_decom(FISCHLIN_LARGE),
        parameter_sets_bounded(),
    );
}

criterion_group!(parallel_scaling, benches);
criterion_main!(parallel_scaling);
