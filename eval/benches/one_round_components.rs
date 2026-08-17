// Component-level breakdown of the one-round DKG, per proof system and committee
// size. The two committee sizes bracket the crossover with the two-round variant,
// so the breakdown shows which operations move it.
// Mirrors two_round_components.rs so the two can be compared side-by-side.

use janus::{
    DkgParams,
    encryption::{decrypt_my_shares, encrypt_batch, keygen},
    one_round::{DkgInitBroadcast, dkg_initiate},
    one_round_proofs::{
        FischlinPolyProof, FischlinProofParams, PolyProofScheme, PolyWellFormedStatement,
        PolyWellFormedWitness, SchnorrPolyProof,
    },
    party::{collect_public_parties, make_party_state},
    pedersen::PedersenCommitment,
    poly::{eval_poly_on_1_to_n, sample_random_polynomial_with_constant},
};

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use rand::rng;
use serde::Serialize;
use serde::de::DeserializeOwned;

const NS: [usize; 2] = [16, 512];

fn committee_sizes() -> Vec<usize> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    NS.into_iter().filter(|n| *n <= max_n).collect()
}

const FISCHLIN: FischlinProofParams = FischlinProofParams {
    rho: 16,
    b: 8,
    t_bits: 13,
};

// n-out-of-n, as everywhere else in the suite.
fn degree(n: usize) -> usize {
    n - 1
}

// The batch benchmarks at n = 512 take tens of seconds per iteration, so the
// default sample count would run for a quarter of an hour each.
fn samples(n: usize) -> usize {
    if n >= 256 { 10 } else { 100 }
}

fn domain(n: usize) -> Vec<Scalar> {
    (1..=n).map(|i| Scalar::from(i as u64)).collect()
}

fn poly_instance(n: usize) -> (PolyWellFormedStatement, PolyWellFormedWitness) {
    let mut rng = rng();
    let c0 = Scalar::random(&mut rng);
    let coeffs = sample_random_polynomial_with_constant(&mut rng, degree(n), c0);
    let blindings: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();
    let evals = eval_poly_on_1_to_n(&coeffs, n);
    let commitments = evals
        .iter()
        .zip(blindings.iter())
        .map(|(&e, &b)| PedersenCommitment::new(e, b))
        .collect();
    use janus::group::g_mul_scalar;
    let f0_commitment = g_mul_scalar(coeffs[0]);
    let stmt = PolyWellFormedStatement {
        x_points: domain(n),
        commitments,
        f0_commitment,
        degree: coeffs.len() - 1,
    };
    let wit = PolyWellFormedWitness { coeffs, blindings };
    (stmt, wit)
}

// Everything that does not depend on the proof system: measured once per n.
fn bench_transform_independent(c: &mut Criterion, n: usize) {
    let mut rng = rng();
    let t = degree(n);

    {
        let mut group = c.benchmark_group("initiate");
        group.sample_size(samples(n));

        let enc_pks: Vec<RistrettoPoint> = (0..n - 1).map(|_| keygen().1).collect();
        let c0 = Scalar::random(&mut rng);
        let coeffs = sample_random_polynomial_with_constant(&mut rng, t, c0);
        let blindings: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();
        let evals = eval_poly_on_1_to_n(&coeffs, n);
        let receivers: Vec<(usize, RistrettoPoint)> = enc_pks
            .iter()
            .enumerate()
            .map(|(j, &pk)| (j + 2, pk))
            .collect();
        let m1s: Vec<Scalar> = evals[1..].to_vec();
        let m2s: Vec<Scalar> = blindings[1..].to_vec();

        group.bench_function(
            BenchmarkId::new("encrypt_shares_batch", format!("n{n}")),
            |b| {
                b.iter(|| {
                    let batch =
                        encrypt_batch(black_box(&receivers), black_box(&m1s), black_box(&m2s));
                    black_box(batch);
                });
            },
        );
        group.finish();
    }

    let mut group = c.benchmark_group("output");
    group.sample_size(samples(n));

    let (sk, pk) = keygen();
    let my_idx = 1usize;
    let batches: Vec<_> = (0..n - 1)
        .map(|_| {
            encrypt_batch(
                &[(my_idx, pk)],
                &[Scalar::random(&mut rng)],
                &[Scalar::random(&mut rng)],
            )
        })
        .collect();
    let batch_refs: Vec<_> = batches.iter().collect();
    group.bench_function(BenchmarkId::new("decrypt_batch", format!("n{n}")), |b| {
        b.iter(|| {
            let result =
                decrypt_my_shares(black_box(&sk), black_box(&batch_refs), black_box(my_idx));
            let _ = black_box(result);
        });
    });

    let commitments: Vec<PedersenCommitment> = (0..n - 1)
        .map(|_| PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();
    let openings: Vec<(Scalar, Scalar)> = (0..n - 1)
        .map(|_| (Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();
    group.bench_function(
        BenchmarkId::new("pedvss_opening_check_batch", format!("n{n}")),
        |b| {
            b.iter(|| {
                for (c, (s, r)) in commitments.iter().zip(openings.iter()) {
                    let ok = c.matches_opening(black_box(*s), black_box(*r));
                    black_box(ok);
                }
            });
        },
    );

    let all_pedvss: Vec<Vec<RistrettoPoint>> = (0..n)
        .map(|_| {
            (0..n)
                .map(|_| {
                    *PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng))
                        .point()
                })
                .collect()
        })
        .collect();
    group.bench_function(BenchmarkId::new("vk_aggregation", format!("n{n}")), |b| {
        b.iter(|| {
            let vks: Vec<RistrettoPoint> = (0..n)
                .map(|k| {
                    all_pedvss
                        .iter()
                        .fold(RistrettoPoint::default(), |acc, pedvss| {
                            acc + black_box(pedvss[k])
                        })
                })
                .collect();
            black_box(vks);
        });
    });
    group.finish();
}

// Everything the proof system moves: proving, verifying, and the wire path, whose
// cost follows the proof bytes.
fn bench_scheme<S>(c: &mut Criterion, scheme: &str, proof_params: S::Params, n: usize)
where
    S: PolyProofScheme,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + Serialize + DeserializeOwned + Send + Sync,
{
    let mut rng = rng();
    let (stmt, wit) = poly_instance(n);

    {
        let mut group = c.benchmark_group("initiate");
        group.sample_size(samples(n));
        group.bench_function(
            BenchmarkId::new(format!("poly_prove_{scheme}"), format!("n{n}")),
            |b| {
                b.iter(|| {
                    let proof =
                        S::prove(black_box(&proof_params), black_box(&stmt), black_box(&wit));
                    black_box(proof);
                });
            },
        );
        group.finish();
    }

    let mut group = c.benchmark_group("output");
    group.sample_size(samples(n));

    let proof = S::prove(&proof_params, &stmt, &wit);
    group.bench_function(
        BenchmarkId::new(format!("poly_verify_single_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                let ok = S::verify(
                    black_box(&proof_params),
                    black_box(&stmt),
                    black_box(&proof),
                );
                black_box(ok);
            });
        },
    );

    let proofs: Vec<(S::Proof, PolyWellFormedStatement)> = (0..n)
        .map(|_| {
            let (s, w) = poly_instance(n);
            let p = S::prove(&proof_params, &s, &w);
            (p, s)
        })
        .collect();
    group.bench_function(
        BenchmarkId::new(format!("poly_verify_batch_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                for (p, s) in &proofs {
                    let ok = S::verify(black_box(&proof_params), black_box(s), black_box(p));
                    black_box(ok);
                }
            });
        },
    );
    drop(proofs);

    // The channel-facing path. Both steps follow the proof bytes, so they are
    // measured per proof system and split so each is attributable on its own.
    let dkg_params = DkgParams { t: degree(n), n };
    let party_states: Vec<_> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&party_states);
    let msgs: Vec<DkgInitBroadcast<S::Proof>> = (1..=n)
        .map(|i| {
            let share = Scalar::random(&mut rng);
            dkg_initiate::<_, S>(
                &mut rng,
                &dkg_params,
                &proof_params,
                &party_states[i - 1],
                share,
                &parties,
            )
            .broadcast
        })
        .collect();

    group.bench_function(
        BenchmarkId::new(format!("signature_verify_batch_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                for msg in &msgs {
                    let ok = msg.verify(black_box(parties.sig_pk(msg.dealer_idx)));
                    black_box(ok);
                }
            });
        },
    );

    let wire: Vec<Vec<u8>> = msgs.iter().map(|m| m.to_wire()).collect();
    drop(msgs);

    group.bench_function(
        BenchmarkId::new(format!("wire_verify_batch_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                for (i, w) in wire.iter().enumerate() {
                    let (payload, sig) = janus::wire::split(w).expect("well-formed");
                    let ok = janus::wire::verify_payload(
                        janus::one_round::DKG_INIT_DOMAIN,
                        payload,
                        &sig,
                        black_box(parties.sig_pk(i + 1)),
                    );
                    black_box(ok.is_ok());
                }
            });
        },
    );

    group.bench_function(
        BenchmarkId::new(format!("wire_decode_batch_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                let decoded =
                    janus::one_round::decode_broadcasts::<S::Proof>(black_box(&wire), &parties)
                        .expect("decodes");
                black_box(decoded.len());
            });
        },
    );
    group.finish();
}

fn components(c: &mut Criterion) {
    for n in committee_sizes() {
        bench_transform_independent(c, n);
        bench_scheme::<SchnorrPolyProof>(c, "schnorr", (), n);
        bench_scheme::<FischlinPolyProof>(c, "fischlin_small", FISCHLIN, n);
    }
}

criterion_group!(benches, components);
criterion_main!(benches);
