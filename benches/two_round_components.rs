// Component-level breakdown of the two-round DKG for t=32, n=64, Fischlin Small.
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
        FischlinDecomScheme,
        comeq_proof::{ComEqProof, ComEqStatement, ComEqWitness},
        pk_proof::{PkProof, PkStatement, PkWitness},
    },
};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use rand::thread_rng;

const T: usize = 32;
const N: usize = 64;

const FISCHLIN: FischlinDecomProofParams = FischlinDecomProofParams {
    rho: 16,
    b: 8,
    t_bits: 13,
};

fn random_decom_statement_and_witness() -> (DecomStatement, DecomWitness) {
    let mut rng = thread_rng();
    let a: Vec<Scalar> = (0..=T).map(|_| Scalar::random(&mut rng)).collect();
    let b: Vec<Scalar> = (0..=T).map(|_| Scalar::random(&mut rng)).collect();
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

fn make_fischlin_proof(
    stmt: &DecomStatement,
    wit: &DecomWitness,
) -> <FischlinDecomScheme as DecomProofScheme>::Proof {
    FischlinDecomScheme::prove(&FISCHLIN, stmt, wit)
}

fn n_fischlin_proofs() -> Vec<(
    <FischlinDecomScheme as DecomProofScheme>::Proof,
    DecomStatement,
)> {
    (0..N)
        .map(|_| {
            let (stmt, wit) = random_decom_statement_and_witness();
            let proof = make_fischlin_proof(&stmt, &wit);
            (proof, stmt)
        })
        .collect()
}

fn random_pk_statement_and_witness() -> (PkStatement, PkWitness) {
    let mut rng = thread_rng();
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
    let mut rng = thread_rng();
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

fn build_cstar(all_pedvss: &[Vec<PedersenCommitment>]) -> Vec<RistrettoPoint> {
    let mut agg = vec![RistrettoPoint::default(); T + 1];
    for pedvss in all_pedvss {
        for (k, c) in pedvss.iter().enumerate() {
            agg[k] += c.point();
        }
    }
    (1..=N)
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

fn bench_initiate_decom_prove(c: &mut Criterion) {
    let (stmt, wit) = random_decom_statement_and_witness();
    c.bench_function("initiate/decom_prove_fischlin", |b| {
        b.iter(|| {
            let proof =
                FischlinDecomScheme::prove(black_box(&FISCHLIN), black_box(&stmt), black_box(&wit));
            black_box(proof);
        });
    });
}

fn bench_initiate_pk_prove(c: &mut Criterion) {
    let mut rng = thread_rng();
    let (stmt, wit) = random_pk_statement_and_witness();
    c.bench_function("initiate/pk_prove", |b| {
        b.iter(|| {
            let proof = PkProof::prove(&mut rng, black_box(&stmt), black_box(&wit));
            black_box(proof);
        });
    });
}

fn bench_initiate_encrypt_shares(c: &mut Criterion) {
    let mut rng = thread_rng();
    let enc_pks: Vec<RistrettoPoint> = (0..N - 1).map(|_| keygen().1).collect();
    let c0 = Scalar::random(&mut rng);
    let f: Vec<Scalar> = sample_random_polynomial_with_constant(&mut rng, T, c0);
    let c0p = Scalar::random(&mut rng);
    let fp: Vec<Scalar> = sample_random_polynomial_with_constant(&mut rng, T, c0p);

    let receivers: Vec<(usize, RistrettoPoint)> = enc_pks
        .iter()
        .enumerate()
        .map(|(j, &pk)| (j + 2, pk))
        .collect();
    let m1s: Vec<Scalar> = (0..N - 1)
        .map(|j| eval_poly_at(&f, Scalar::from((j + 2) as u64)))
        .collect();
    let m2s: Vec<Scalar> = (0..N - 1)
        .map(|j| eval_poly_at(&fp, Scalar::from((j + 2) as u64)))
        .collect();

    c.bench_function("initiate/encrypt_shares_batch", |b| {
        b.iter(|| {
            let batch = encrypt_batch(black_box(&receivers), black_box(&m1s), black_box(&m2s));
            black_box(batch);
        });
    });
}

fn bench_finalize_decom_verify_single(c: &mut Criterion) {
    let (stmt, wit) = random_decom_statement_and_witness();
    let proof = make_fischlin_proof(&stmt, &wit);
    c.bench_function("finalize/decom_verify_fischlin_single", |b| {
        b.iter(|| {
            let ok = FischlinDecomScheme::verify(
                black_box(&FISCHLIN),
                black_box(&stmt),
                black_box(&proof),
            );
            black_box(ok);
        });
    });
}

fn bench_finalize_decom_verify_batch(c: &mut Criterion) {
    let proofs = n_fischlin_proofs();
    c.bench_function("finalize/decom_verify_fischlin_batch_n64", |b| {
        b.iter(|| {
            for (proof, stmt) in &proofs {
                let ok = FischlinDecomScheme::verify(
                    black_box(&FISCHLIN),
                    black_box(stmt),
                    black_box(proof),
                );
                black_box(ok);
            }
        });
    });
}

fn bench_finalize_decrypt_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let (sk, pk) = keygen();
    let my_idx = 1usize;
    let batches: Vec<_> = (0..N - 1)
        .map(|_| {
            encrypt_batch(
                &[(my_idx, pk)],
                &[Scalar::random(&mut rng)],
                &[Scalar::random(&mut rng)],
            )
        })
        .collect();
    let batch_refs: Vec<_> = batches.iter().collect();

    c.bench_function("finalize/decrypt_batch_n63", |b| {
        b.iter(|| {
            let result =
                decrypt_my_shares(black_box(&sk), black_box(&batch_refs), black_box(my_idx));
            let _ = black_box(result);
        });
    });
}

fn bench_finalize_pedvss_eval_check_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    // N-1 pedvss sets (one per remote party), each with t+1 commitments
    let pedvss_sets: Vec<Vec<PedersenCommitment>> = (0..N - 1)
        .map(|_| {
            (0..=T)
                .map(|_| {
                    PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng))
                })
                .collect()
        })
        .collect();
    let my_idx = 1usize;
    let x_i = Scalar::from(my_idx as u64);
    let share_pairs: Vec<(Scalar, Scalar)> = (0..N - 1)
        .map(|_| (Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();

    c.bench_function("finalize/pedvss_eval_check_batch_n63", |b| {
        b.iter(|| {
            for (pedvss, (s, sp)) in pedvss_sets.iter().zip(share_pairs.iter()) {
                // Evaluate the commitment polynomial at x_i
                let coeff_pts: Vec<RistrettoPoint> = pedvss.iter().map(|c| *c.point()).collect();
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
    });
}

fn bench_finalize_cstar_build(c: &mut Criterion) {
    let mut rng = thread_rng();
    let all_pedvss: Vec<Vec<PedersenCommitment>> = (0..N)
        .map(|_| {
            (0..=T)
                .map(|_| {
                    PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng))
                })
                .collect()
        })
        .collect();
    c.bench_function("finalize/cstar_aggregate_and_eval_n64", |b| {
        b.iter(|| {
            let stars = build_cstar(black_box(&all_pedvss));
            black_box(stars);
        });
    });
}

fn bench_finalize_comeq_prove(c: &mut Criterion) {
    let mut rng = thread_rng();
    let (stmt, wit) = random_comeq_statement_and_witness();
    c.bench_function("finalize/comeq_prove", |b| {
        b.iter(|| {
            let proof = ComEqProof::prove(&mut rng, black_box(&stmt), black_box(&wit));
            black_box(proof);
        });
    });
}

fn bench_finalize_signature_verify_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let dkg_params = DkgParams { t: T, n: N };

    let mut party_states = Vec::with_capacity(N);
    for dealer_idx in 1..=N {
        party_states.push(make_party_state(&mut rng, dealer_idx));
    }
    let parties = collect_public_parties(&party_states);

    let msgs: Vec<Round1Broadcast<<FischlinDecomScheme as DecomProofScheme>::Proof>> = (1..=N)
        .map(|i| {
            let share = Scalar::random(&mut rng);
            dkg_round1_initiate::<_, FischlinDecomScheme>(
                &mut rng,
                &dkg_params,
                &FISCHLIN,
                &party_states[i - 1],
                share,
                &parties,
            )
            .0
        })
        .collect();

    c.bench_function("finalize/signature_verify_batch_n64", |b| {
        b.iter(|| {
            for msg in &msgs {
                let ok = msg.verify(black_box(parties.sig_pk(msg.dealer_idx)));
                black_box(ok);
            }
        });
    });
}

fn bench_output_pk_verify_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let pk_pairs: Vec<(PkStatement, PkProof)> = (0..N)
        .map(|_| {
            let (stmt, wit) = random_pk_statement_and_witness();
            let proof = PkProof::prove(&mut rng, &stmt, &wit);
            (stmt, proof)
        })
        .collect();
    c.bench_function("output/pk_verify_batch_n64", |b| {
        b.iter(|| {
            for (stmt, proof) in &pk_pairs {
                let ok = proof.verify(black_box(stmt));
                black_box(ok);
            }
        });
    });
}

fn bench_output_comeq_verify_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let comeq_pairs: Vec<(ComEqStatement, ComEqProof)> = (0..N)
        .map(|_| {
            let (stmt, wit) = random_comeq_statement_and_witness();
            let proof = ComEqProof::prove(&mut rng, &stmt, &wit);
            (stmt, proof)
        })
        .collect();
    c.bench_function("output/comeq_verify_batch_n64", |b| {
        b.iter(|| {
            for (stmt, proof) in &comeq_pairs {
                let ok = proof.verify(black_box(stmt));
                black_box(ok);
            }
        });
    });
}

criterion_group!(
    benches,
    // initiate
    bench_initiate_decom_prove,
    bench_initiate_pk_prove,
    bench_initiate_encrypt_shares,
    // finalize
    bench_finalize_decom_verify_single,
    bench_finalize_decom_verify_batch,
    bench_finalize_decrypt_batch,
    bench_finalize_pedvss_eval_check_batch,
    bench_finalize_cstar_build,
    bench_finalize_comeq_prove,
    bench_finalize_signature_verify_batch,
    // output
    bench_output_pk_verify_batch,
    bench_output_comeq_verify_batch,
);
criterion_main!(benches);
