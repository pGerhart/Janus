// Component-level breakdown of the two-round DKG, per proof system and committee
// size. The two committee sizes bracket the crossover with the one-round variant,
// so the breakdown shows which operations move it.
//
// Each benchmark isolates one step so we can see where the time goes within
// initiate, finalize, and output.

use janus::{
    DkgParams,
    encryption::{decrypt_my_shares, encrypt_batch, keygen},
    group::g,
    party::{collect_public_parties, make_party_state},
    pedersen::PedersenCommitment,
    poly::{eval_poly_at, sample_random_polynomial_with_constant},
    two_round::{Round1Broadcast, dkg_round1_initiate},
    two_round_proofs::{
        DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams,
        FischlinDecomScheme, SchnorrDecomProof, SchnorrDecomProofParams,
        comeq_proof::{ComEqProof, ComEqStatement, ComEqWitness},
        pk_proof::{PkProof, PkStatement, PkWitness},
    },
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

const FISCHLIN: FischlinDecomProofParams = FischlinDecomProofParams {
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

fn decom_instance(n: usize) -> (DecomStatement, DecomWitness) {
    let mut rng = rng();
    let t = degree(n);
    let a: Vec<Scalar> = (0..=t).map(|_| Scalar::random(&mut rng)).collect();
    let b: Vec<Scalar> = (0..=t).map(|_| Scalar::random(&mut rng)).collect();
    let omega = Scalar::random(&mut rng);
    let r = Scalar::random(&mut rng);
    let pedvss = a
        .iter()
        .zip(b.iter())
        .map(|(&ai, &bi)| PedersenCommitment::new(ai, bi))
        .collect();
    let stmt = DecomStatement {
        pedvss,
        d: PedersenCommitment::new(omega, r),
    };
    let wit = DecomWitness { a, b, omega, r };
    (stmt, wit)
}

fn random_pk_statement_and_witness() -> (PkStatement, PkWitness) {
    let mut rng = rng();
    let a = Scalar::random(&mut rng);
    let b = Scalar::random(&mut rng);
    let stmt = PkStatement {
        pk: g() * a,
        commitment: PedersenCommitment::new(a, b),
    };
    let wit = PkWitness { a, b };
    (stmt, wit)
}

fn random_comeq_statement_and_witness() -> (ComEqStatement, ComEqWitness) {
    let mut rng = rng();
    let s = Scalar::random(&mut rng);
    let s_prime = Scalar::random(&mut rng);
    let omega = Scalar::random(&mut rng);
    let r = Scalar::random(&mut rng);
    let stmt = ComEqStatement {
        c: PedersenCommitment::new(s, s_prime),
        vk: PedersenCommitment::new(s, omega),
        d: PedersenCommitment::new(omega, r),
    };
    let wit = ComEqWitness {
        s,
        s_prime,
        omega,
        r,
    };
    (stmt, wit)
}

fn build_cstar(all_pedvss: &[Vec<PedersenCommitment>], n: usize) -> Vec<RistrettoPoint> {
    let mut agg = vec![RistrettoPoint::default(); degree(n) + 1];
    for pedvss in all_pedvss {
        for (k, c) in pedvss.iter().enumerate() {
            agg[k] += c.point();
        }
    }
    (1..=n)
        .map(|i| {
            let x = Scalar::from(i as u64);
            let mut x_pow = Scalar::ONE;
            let mut acc = RistrettoPoint::default();
            for &p in &agg {
                acc += p * x_pow;
                x_pow *= x;
            }
            acc
        })
        .collect()
}

// Everything that does not depend on the proof system: measured once per n.
fn bench_transform_independent(c: &mut Criterion, n: usize) {
    let mut rng = rng();
    let t = degree(n);

    {
        let mut group = c.benchmark_group("initiate");
        group.sample_size(samples(n));

        let (stmt, wit) = random_pk_statement_and_witness();
        group.bench_function(BenchmarkId::new("pk_prove", format!("n{n}")), |b| {
            b.iter(|| {
                let proof = PkProof::prove(&mut rng, black_box(&stmt), black_box(&wit));
                black_box(proof);
            });
        });

        let enc_pks: Vec<RistrettoPoint> = (0..n - 1).map(|_| keygen().1).collect();
        let c0 = Scalar::random(&mut rng);
        let f: Vec<Scalar> = sample_random_polynomial_with_constant(&mut rng, t, c0);
        let c0p = Scalar::random(&mut rng);
        let fp: Vec<Scalar> = sample_random_polynomial_with_constant(&mut rng, t, c0p);
        let receivers: Vec<(usize, RistrettoPoint)> = enc_pks
            .iter()
            .enumerate()
            .map(|(j, &pk)| (j + 2, pk))
            .collect();
        let m1s: Vec<Scalar> = (0..n - 1)
            .map(|j| eval_poly_at(&f, Scalar::from((j + 2) as u64)))
            .collect();
        let m2s: Vec<Scalar> = (0..n - 1)
            .map(|j| eval_poly_at(&fp, Scalar::from((j + 2) as u64)))
            .collect();
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

    {
        let mut group = c.benchmark_group("finalize");
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

        // n-1 pedvss sets (one per remote party), each with t+1 commitments.
        let pedvss_sets: Vec<Vec<PedersenCommitment>> = (0..n - 1)
            .map(|_| {
                (0..=t)
                    .map(|_| {
                        PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng))
                    })
                    .collect()
            })
            .collect();
        let x_i = Scalar::from(1u64);
        let share_pairs: Vec<(Scalar, Scalar)> = (0..n - 1)
            .map(|_| (Scalar::random(&mut rng), Scalar::random(&mut rng)))
            .collect();
        group.bench_function(
            BenchmarkId::new("pedvss_eval_check_batch", format!("n{n}")),
            |b| {
                b.iter(|| {
                    for (pedvss, (s, sp)) in pedvss_sets.iter().zip(share_pairs.iter()) {
                        let coeff_pts: Vec<RistrettoPoint> =
                            pedvss.iter().map(|c| *c.point()).collect();
                        let mut x_pow = Scalar::ONE;
                        let mut acc = RistrettoPoint::default();
                        for &p in &coeff_pts {
                            acc += p * x_pow;
                            x_pow *= x_i;
                        }
                        let expected = PedersenCommitment::from_point(acc);
                        let ok = expected.matches_opening(black_box(*s), black_box(*sp));
                        black_box(ok);
                    }
                });
            },
        );
        drop(pedvss_sets);

        let all_pedvss: Vec<Vec<PedersenCommitment>> = (0..n)
            .map(|_| {
                (0..=t)
                    .map(|_| {
                        PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng))
                    })
                    .collect()
            })
            .collect();
        group.bench_function(
            BenchmarkId::new("cstar_aggregate_and_eval", format!("n{n}")),
            |b| {
                b.iter(|| {
                    let stars = build_cstar(black_box(&all_pedvss), n);
                    black_box(stars);
                });
            },
        );
        drop(all_pedvss);

        let (stmt, wit) = random_comeq_statement_and_witness();
        group.bench_function(BenchmarkId::new("comeq_prove", format!("n{n}")), |b| {
            b.iter(|| {
                let proof = ComEqProof::prove(&mut rng, black_box(&stmt), black_box(&wit));
                black_box(proof);
            });
        });
        group.finish();
    }

    let mut group = c.benchmark_group("output");
    group.sample_size(samples(n));

    let pk_pairs: Vec<(PkStatement, PkProof)> = (0..n)
        .map(|_| {
            let (stmt, wit) = random_pk_statement_and_witness();
            let proof = PkProof::prove(&mut rng, &stmt, &wit);
            (stmt, proof)
        })
        .collect();
    group.bench_function(BenchmarkId::new("pk_verify_batch", format!("n{n}")), |b| {
        b.iter(|| {
            for (stmt, proof) in &pk_pairs {
                let ok = proof.verify(black_box(stmt));
                black_box(ok);
            }
        });
    });
    drop(pk_pairs);

    let comeq_pairs: Vec<(ComEqStatement, ComEqProof)> = (0..n)
        .map(|_| {
            let (stmt, wit) = random_comeq_statement_and_witness();
            let proof = ComEqProof::prove(&mut rng, &stmt, &wit);
            (stmt, proof)
        })
        .collect();
    group.bench_function(
        BenchmarkId::new("comeq_verify_batch", format!("n{n}")),
        |b| {
            b.iter(|| {
                for (stmt, proof) in &comeq_pairs {
                    let ok = proof.verify(black_box(stmt));
                    black_box(ok);
                }
            });
        },
    );
    group.finish();
}

// Everything the proof system moves: proving, verifying, and the wire path, whose
// cost follows the proof bytes.
fn bench_scheme<S>(c: &mut Criterion, scheme: &str, proof_params: S::Params, n: usize)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + Serialize + DeserializeOwned + Send + Sync,
{
    let mut rng = rng();
    let (stmt, wit) = decom_instance(n);

    {
        let mut group = c.benchmark_group("initiate");
        group.sample_size(samples(n));
        group.bench_function(
            BenchmarkId::new(format!("decom_prove_{scheme}"), format!("n{n}")),
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

    let mut group = c.benchmark_group("finalize");
    group.sample_size(samples(n));

    let proof = S::prove(&proof_params, &stmt, &wit);
    group.bench_function(
        BenchmarkId::new(format!("decom_verify_single_{scheme}"), format!("n{n}")),
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

    let proofs: Vec<(S::Proof, DecomStatement)> = (0..n)
        .map(|_| {
            let (s, w) = decom_instance(n);
            let p = S::prove(&proof_params, &s, &w);
            (p, s)
        })
        .collect();
    group.bench_function(
        BenchmarkId::new(format!("decom_verify_batch_{scheme}"), format!("n{n}")),
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
    // measured per proof system.
    let dkg_params = DkgParams { t: degree(n), n };
    let party_states: Vec<_> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&party_states);
    let msgs: Vec<Round1Broadcast<S::Proof>> = (1..=n)
        .map(|i| {
            let share = Scalar::random(&mut rng);
            dkg_round1_initiate::<_, S>(
                &mut rng,
                &dkg_params,
                &proof_params,
                &party_states[i - 1],
                share,
                &parties,
            )
            .0
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
        BenchmarkId::new(format!("wire_decode_batch_{scheme}"), format!("n{n}")),
        |b| {
            b.iter(|| {
                let decoded: Vec<_> = black_box(&wire)
                    .iter()
                    .map(|w| Round1Broadcast::<S::Proof>::from_wire(w, &parties).expect("decodes"))
                    .collect();
                black_box(decoded.len());
            });
        },
    );
    group.finish();
}

fn components(c: &mut Criterion) {
    for n in committee_sizes() {
        bench_transform_independent(c, n);
        bench_scheme::<SchnorrDecomProof>(c, "schnorr", SchnorrDecomProofParams, n);
        bench_scheme::<FischlinDecomScheme>(c, "fischlin_small", FISCHLIN, n);
    }
}

criterion_group!(benches, components);
criterion_main!(benches);
