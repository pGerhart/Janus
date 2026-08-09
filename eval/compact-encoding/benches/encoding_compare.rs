//! The trade the compact encoding makes: fewer bytes on the wire against
//! rebuilding the first-round commitments. Both encodings are measured over the
//! same path, encode then decode then verify, so the saved parsing counts.

use compact_encoding::{CompactFischlinProof, prove_compact, verify_compact};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::group::g_mul_scalar;
use janus::one_round_proofs::polyproof_fischlin::{
    PolyWellFormedFischlinProof, prove_fischlin_with_params, verify_fischlin_with_params,
};
use janus::one_round_proofs::{PolyWellFormedStatement, PolyWellFormedWitness};
use janus::pedersen::PedersenCommitment;
use janus::poly::eval_poly_at;
use rand::rng;

const RHO: usize = 16;
const B: usize = 8;
const T_BITS: usize = 13;

fn instance(t: usize, n: usize) -> (PolyWellFormedStatement, PolyWellFormedWitness) {
    let mut rng = rng();
    let coeffs: Vec<Scalar> = (0..=t).map(|_| Scalar::random(&mut rng)).collect();
    let blindings: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();
    let xs: Vec<Scalar> = (1..=n).map(|i| Scalar::from(i as u64)).collect();
    let commitments: Vec<PedersenCommitment> = xs
        .iter()
        .zip(blindings.iter())
        .map(|(x, r)| PedersenCommitment::new(eval_poly_at(&coeffs, *x), *r))
        .collect();
    (
        PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g_mul_scalar(coeffs[0]),
            degree: t,
        },
        PolyWellFormedWitness { coeffs, blindings },
    )
}

// Same range as the protocol suites, since the point of the table is how the
// ratio moves with the committee. JANUS_BENCH_MAX_N caps it for a quick run.
fn parameter_sets() -> Vec<(usize, usize)> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    [
        (4usize, 16usize),
        (8, 32),
        (16, 64),
        (32, 64),
        (64, 128),
        (128, 256),
        (256, 512),
    ]
    .into_iter()
    .filter(|(_t, n)| *n <= max_n)
    .collect()
}

fn compare(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding");
    group.sample_size(10);

    for (t, n) in parameter_sets() {
        let label = format!("t{t}_n{n}");
        let (stmt, wit) = instance(t, n);

        let verbose = prove_fischlin_with_params(&stmt, &wit, RHO, B, T_BITS);
        let compact = prove_compact(&stmt, &wit, RHO, B, T_BITS);
        let vb = postcard::to_allocvec(&verbose).expect("encode");
        let cb = postcard::to_allocvec(&compact).expect("encode");
        eprintln!(
            "[{label}] verbose={} B compact={} B saved={:.1}%",
            vb.len(),
            cb.len(),
            100.0 * (vb.len() - cb.len()) as f64 / vb.len() as f64
        );

        group.bench_with_input(BenchmarkId::new("verbose", &label), &label, |b, _| {
            b.iter(|| {
                let p: PolyWellFormedFischlinProof =
                    postcard::from_bytes(black_box(&vb)).expect("decode");
                black_box(verify_fischlin_with_params(&stmt, &p, RHO, B, T_BITS));
            });
        });

        group.bench_with_input(BenchmarkId::new("compact", &label), &label, |b, _| {
            b.iter(|| {
                let p: CompactFischlinProof = postcard::from_bytes(black_box(&cb)).expect("decode");
                black_box(verify_compact(&stmt, &p, RHO, B, T_BITS));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, compare);
criterion_main!(benches);
