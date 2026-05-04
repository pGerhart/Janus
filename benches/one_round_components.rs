// Component-level breakdown of the one-round DKG for t=32, n=64, Fischlin Small.
// Mirrors two_round_components.rs so the two can be compared side-by-side.

use adaptive_dkg::{
    DkgParams,
    encryption::{decrypt_two_scalars, encrypt_two_scalars, keygen},
    one_round::{DkgInitBroadcast, dkg_initiate},
    one_round_proofs::{
        FischlinPolyProof, FischlinProofParams, PolyProofScheme, PolyWellFormedStatement,
        PolyWellFormedWitness,
    },
    party::{collect_public_parties, make_party_state},
    pedersen::PedersenCommitment,
    poly::{eval_poly_on_1_to_n, sample_random_polynomial_with_constant},
};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use rand::thread_rng;

const T: usize = 32;
const N: usize = 64;

const FISCHLIN: FischlinProofParams = FischlinProofParams {
    rho: 16,
    b: 8,
    t_bits: 13,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn domain() -> Vec<Scalar> {
    (1..=N).map(|i| Scalar::from(i as u64)).collect()
}

fn random_poly_statement_and_witness() -> (PolyWellFormedStatement, PolyWellFormedWitness) {
    let mut rng = thread_rng();
    let c0 = Scalar::random(&mut rng);
    let coeffs = sample_random_polynomial_with_constant(&mut rng, T, c0);
    let blindings: Vec<Scalar> = (0..N).map(|_| Scalar::random(&mut rng)).collect();
    let evals = eval_poly_on_1_to_n(&coeffs, N);
    let commitments = evals
        .iter()
        .zip(blindings.iter())
        .map(|(&e, &b)| PedersenCommitment::new(e, b))
        .collect();
    use adaptive_dkg::group::g_mul_scalar;
    let f0_commitment = g_mul_scalar(coeffs[0]);
    let stmt = PolyWellFormedStatement {
        x_points: domain(),
        commitments,
        f0_commitment,
    };
    let wit = PolyWellFormedWitness { coeffs, blindings };
    (stmt, wit)
}

fn make_fischlin_proof(
    stmt: &PolyWellFormedStatement,
    wit: &PolyWellFormedWitness,
) -> <FischlinPolyProof as PolyProofScheme>::Proof {
    FischlinPolyProof::prove(&FISCHLIN, stmt, wit)
}

// N pre-baked proofs for batch-verify benchmarks
fn n_fischlin_proofs() -> Vec<(<FischlinPolyProof as PolyProofScheme>::Proof, PolyWellFormedStatement)> {
    (0..N)
        .map(|_| {
            let (stmt, wit) = random_poly_statement_and_witness();
            let proof = make_fischlin_proof(&stmt, &wit);
            (proof, stmt)
        })
        .collect()
}

// ── initiate components ──────────────────────────────────────────────────────

fn bench_initiate_poly_prove(c: &mut Criterion) {
    let (stmt, wit) = random_poly_statement_and_witness();
    c.bench_function("initiate/poly_prove_fischlin", |b| {
        b.iter(|| {
            let proof = FischlinPolyProof::prove(
                black_box(&FISCHLIN),
                black_box(&stmt),
                black_box(&wit),
            );
            black_box(proof);
        });
    });
}

fn bench_initiate_encrypt_shares(c: &mut Criterion) {
    let mut rng = thread_rng();
    let enc_pks: Vec<RistrettoPoint> = (0..N - 1).map(|_| keygen().1).collect();
    let c0 = Scalar::random(&mut rng);
    let coeffs = sample_random_polynomial_with_constant(&mut rng, T, c0);
    let blindings: Vec<Scalar> = (0..N).map(|_| Scalar::random(&mut rng)).collect();
    let evals = eval_poly_on_1_to_n(&coeffs, N);

    c.bench_function("initiate/encrypt_shares_batch", |b| {
        b.iter(|| {
            for (j, pk) in enc_pks.iter().enumerate() {
                let ct = encrypt_two_scalars(
                    black_box(pk),
                    black_box(evals[j + 1]),      // skip index 0 (self)
                    black_box(blindings[j + 1]),
                );
                black_box(ct);
            }
        });
    });
}

// ── output components ─────────────────────────────────────────────────────────

fn bench_output_poly_verify_single(c: &mut Criterion) {
    let (stmt, wit) = random_poly_statement_and_witness();
    let proof = make_fischlin_proof(&stmt, &wit);
    c.bench_function("output/poly_verify_fischlin_single", |b| {
        b.iter(|| {
            let ok = FischlinPolyProof::verify(
                black_box(&FISCHLIN),
                black_box(&stmt),
                black_box(&proof),
            );
            black_box(ok);
        });
    });
}

fn bench_output_poly_verify_batch(c: &mut Criterion) {
    let proofs = n_fischlin_proofs();
    c.bench_function("output/poly_verify_fischlin_batch_n64", |b| {
        b.iter(|| {
            for (proof, stmt) in &proofs {
                let ok = FischlinPolyProof::verify(
                    black_box(&FISCHLIN),
                    black_box(stmt),
                    black_box(proof),
                );
                black_box(ok);
            }
        });
    });
}

fn bench_output_decrypt_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let (sk, pk) = keygen();
    let cts: Vec<_> = (0..N - 1)
        .map(|_| encrypt_two_scalars(&pk, Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();
    c.bench_function("output/decrypt_batch_n63", |b| {
        b.iter(|| {
            for ct in &cts {
                let pair = decrypt_two_scalars(black_box(&sk), black_box(ct));
                black_box(pair);
            }
        });
    });
}

fn bench_output_pedvss_opening_check_batch(c: &mut Criterion) {
    // In one-round, pedvss[i] IS the commitment to the evaluation at i.
    // The check is a single matches_opening per sender — O(1), no polynomial eval.
    let mut rng = thread_rng();
    let commitments: Vec<PedersenCommitment> = (0..N - 1)
        .map(|_| PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();
    let openings: Vec<(Scalar, Scalar)> = (0..N - 1)
        .map(|_| (Scalar::random(&mut rng), Scalar::random(&mut rng)))
        .collect();

    c.bench_function("output/pedvss_opening_check_batch_n63", |b| {
        b.iter(|| {
            for (c, (s, r)) in commitments.iter().zip(openings.iter()) {
                let ok = c.matches_opening(black_box(*s), black_box(*r));
                black_box(ok);
            }
        });
    });
}

fn bench_output_vk_aggregation(c: &mut Criterion) {
    // For each of n parties, sum the n partial pedvss commitments from all senders.
    // O(n²) point additions.
    let mut rng = thread_rng();
    // n senders, each with n pedvss commitments
    let all_pedvss: Vec<Vec<RistrettoPoint>> = (0..N)
        .map(|_| (0..N).map(|_| *PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)).point()).collect())
        .collect();

    c.bench_function("output/vk_aggregation_n64", |b| {
        b.iter(|| {
            let vks: Vec<RistrettoPoint> = (0..N)
                .map(|k| {
                    all_pedvss
                        .iter()
                        .fold(RistrettoPoint::default(), |acc, pedvss| acc + black_box(pedvss[k]))
                })
                .collect();
            black_box(vks);
        });
    });
}

fn bench_output_signature_verify_batch(c: &mut Criterion) {
    let mut rng = thread_rng();
    let dkg_params = DkgParams { t: T, n: N };

    let mut party_states = Vec::with_capacity(N);
    for dealer_idx in 1..=N {
        party_states.push(make_party_state(&mut rng, dealer_idx));
    }
    let parties = collect_public_parties(&party_states);

    let msgs: Vec<DkgInitBroadcast<<FischlinPolyProof as PolyProofScheme>::Proof>> = (1..=N)
        .map(|i| {
            let share = Scalar::random(&mut rng);
            dkg_initiate::<_, FischlinPolyProof>(
                &mut rng,
                &dkg_params,
                &FISCHLIN,
                &party_states[i - 1],
                share,
                &parties,
            )
            .broadcast
        })
        .collect();

    c.bench_function("output/signature_verify_batch_n64", |b| {
        b.iter(|| {
            for msg in &msgs {
                let ok = msg.verify(black_box(parties.sig_pk(msg.dealer_idx)));
                black_box(ok);
            }
        });
    });
}

// ── criterion setup ───────────────────────────────────────────────────────────

criterion_group!(
    benches,
    // initiate
    bench_initiate_poly_prove,
    bench_initiate_encrypt_shares,
    // output
    bench_output_poly_verify_single,
    bench_output_poly_verify_batch,
    bench_output_decrypt_batch,
    bench_output_pedvss_opening_check_batch,
    bench_output_vk_aggregation,
    bench_output_signature_verify_batch,
);
criterion_main!(benches);
