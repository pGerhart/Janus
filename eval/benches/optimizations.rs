//! Optimizations at large committees, against the sequential baseline: batch
//! proof verification, multi-threaded output and finalize, and batch signature
//! verification.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::one_round::{
    DkgInitBroadcast, DkgInitLocalState, batch_verify_signatures, dkg_initiate,
    dkg_output_key_generation, dkg_output_key_generation_parallel,
};
use janus::one_round_proofs::polyproof_fischlin::batch_verify_fischlin_with_params;
use janus::one_round_proofs::polyproof_schnorr::batch_verify;
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, PolyWellFormedStatement,
    SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{dkg_round1_initiate, dkg_round2_finalize, dkg_round2_finalize_parallel};
use janus::two_round_proofs::{SchnorrDecomProof, SchnorrDecomProofParams};
use rand::rng;
use rayon::prelude::*;

const FISCHLIN_SMALL: (usize, usize, usize) = (16, 8, 13); // rho, b, t_bits

#[derive(Clone, Copy)]
struct BaseParams {
    t: usize,
    n: usize,
}
impl BaseParams {
    fn label(self) -> String {
        format!("t{}_n{}", self.t, self.n)
    }
}

fn large_sets() -> Vec<BaseParams> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    [128, 256, 512]
        .into_iter()
        .map(|n| BaseParams { t: n - 1, n })
        .filter(|p| p.n <= max_n)
        .collect()
}

fn domain(n: usize) -> Vec<Scalar> {
    (1..=n).map(|i| Scalar::from(i as u64)).collect()
}

fn setup<S>(
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
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=dkg.n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&states);
    // Setup only, never measured, so run the dealers in parallel.
    let (broadcasts, locals): (Vec<_>, Vec<_>) = (1..=dkg.n)
        .into_par_iter()
        .map(|i| {
            let res = dkg_initiate::<_, S>(
                &mut rand::rng(),
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

fn statements_for<P>(
    dkg: &DkgParams,
    broadcasts: &[DkgInitBroadcast<P>],
) -> Vec<PolyWellFormedStatement>
where
    P: Clone,
{
    let d = domain(dkg.n);
    broadcasts
        .iter()
        .map(|m| PolyWellFormedStatement {
            x_points: d.clone(),
            commitments: m.pedvss.clone(),
            f0_commitment: m.f0_commitment,
            degree: dkg.t,
        })
        .collect()
}

fn bench_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("opt_one_round_schnorr");
    group.sample_size(10);

    for p in large_sets() {
        let dkg = DkgParams { t: p.t, n: p.n };
        let (states, parties, broadcasts, locals) = setup::<SchnorrPolyProof>(&dkg, &());
        let statements = statements_for(&dkg, &broadcasts);
        let proofs: Vec<_> = broadcasts.iter().map(|m| m.proof.clone()).collect();
        let recv = 1usize;

        group.bench_with_input(BenchmarkId::new("output_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output_key_generation::<SchnorrPolyProof>(
                        &dkg,
                        &(),
                        &states[recv - 1],
                        &locals[recv - 1],
                        &broadcasts,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(
            BenchmarkId::new("output_parallel", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    black_box(
                        dkg_output_key_generation_parallel::<SchnorrPolyProof>(
                            &dkg,
                            &(),
                            &states[recv - 1],
                            &locals[recv - 1],
                            &broadcasts,
                            &parties,
                        )
                        .unwrap(),
                    );
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("proofverify_loop", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    let ok = statements
                        .iter()
                        .zip(proofs.iter())
                        .all(|(s, pr)| SchnorrPolyProof::verify(&(), s, pr));
                    black_box(ok);
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("proofverify_batch", p.label()),
            &p,
            |b, _| {
                b.iter(|| black_box(batch_verify(&statements, &proofs)));
            },
        );
        group.bench_with_input(BenchmarkId::new("sigverify_loop", p.label()), &p, |b, _| {
            b.iter(|| {
                let ok = broadcasts
                    .iter()
                    .all(|m| m.verify(parties.sig_pk(m.dealer_idx)));
                black_box(ok);
            });
        });
        group.bench_with_input(
            BenchmarkId::new("sigverify_batch", p.label()),
            &p,
            |b, _| {
                b.iter(|| black_box(batch_verify_signatures(&broadcasts, &parties)));
            },
        );
    }
    group.finish();
}

fn bench_fischlin_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("opt_one_round_fischlin_small");
    group.sample_size(10);

    let (rho, bb, t_bits) = FISCHLIN_SMALL;
    let fparams = FischlinProofParams { rho, b: bb, t_bits };

    for p in large_sets() {
        let dkg = DkgParams { t: p.t, n: p.n };
        let (states, parties, broadcasts, locals) = setup::<FischlinPolyProof>(&dkg, &fparams);
        let statements = statements_for(&dkg, &broadcasts);
        let proofs: Vec<_> = broadcasts.iter().map(|m| m.proof.clone()).collect();
        let recv = 1usize;

        group.bench_with_input(BenchmarkId::new("output_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_output_key_generation::<FischlinPolyProof>(
                        &dkg,
                        &fparams,
                        &states[recv - 1],
                        &locals[recv - 1],
                        &broadcasts,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(
            BenchmarkId::new("output_parallel", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    black_box(
                        dkg_output_key_generation_parallel::<FischlinPolyProof>(
                            &dkg,
                            &fparams,
                            &states[recv - 1],
                            &locals[recv - 1],
                            &broadcasts,
                            &parties,
                        )
                        .unwrap(),
                    );
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("proofverify_loop", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    let ok = statements
                        .iter()
                        .zip(proofs.iter())
                        .all(|(s, pr)| FischlinPolyProof::verify(&fparams, s, pr));
                    black_box(ok);
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("proofverify_batch", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    black_box(batch_verify_fischlin_with_params(
                        &statements,
                        &proofs,
                        rho,
                        bb,
                        t_bits,
                    ))
                });
            },
        );
    }
    group.finish();
}

fn bench_two_round_finalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("opt_two_round_finalize_schnorr");
    group.sample_size(10);

    for p in large_sets() {
        let dkg = DkgParams { t: p.t, n: p.n };
        let mut rng = rng();
        let states: Vec<PartyState> = (1..=dkg.n).map(|i| make_party_state(&mut rng, i)).collect();
        let parties = collect_public_parties(&states);
        let mut r1 = Vec::with_capacity(dkg.n);
        let mut r1_locals = Vec::with_capacity(dkg.n);
        for i in 1..=dkg.n {
            let (b, l) = dkg_round1_initiate::<_, SchnorrDecomProof>(
                &mut rng,
                &dkg,
                &SchnorrDecomProofParams,
                &states[i - 1],
                Scalar::from((i + 1) as u64),
                &parties,
            );
            r1.push(b);
            r1_locals.push(l);
        }
        let recv = 1usize;

        group.bench_with_input(BenchmarkId::new("finalize_seq", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(
                    dkg_round2_finalize::<SchnorrDecomProof>(
                        &dkg,
                        &SchnorrDecomProofParams,
                        &states[recv - 1],
                        &r1_locals[recv - 1],
                        &r1,
                        &parties,
                    )
                    .unwrap(),
                );
            });
        });
        group.bench_with_input(
            BenchmarkId::new("finalize_parallel", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    black_box(
                        dkg_round2_finalize_parallel::<SchnorrDecomProof>(
                            &dkg,
                            &SchnorrDecomProofParams,
                            &states[recv - 1],
                            &r1_locals[recv - 1],
                            &r1,
                            &parties,
                        )
                        .unwrap(),
                    );
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_schnorr,
    bench_fischlin_small,
    bench_two_round_finalize
);
criterion_main!(benches);
