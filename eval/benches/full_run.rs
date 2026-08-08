//! End-to-end cost of one run from a single party's view. Unlike the phase
//! benchmarks, every message is encoded on the way out and decoded on the way
//! in, and the printed byte counts feed the link model in `build_results.py`.

use janus::one_round::{DkgInitLocalState, dkg_initiate, dkg_output_key_generation_from_wire};
use janus::one_round_proofs::{
    FischlinPolyProof, FischlinProofParams, PolyProofScheme, SchnorrPolyProof,
};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, Round1LocalState, dkg_output, dkg_round1_initiate, dkg_round2_finalize,
};
use janus::two_round_proofs::{
    DecomProofScheme, DecomStatement, DecomWitness, FischlinDecomProofParams, FischlinDecomScheme,
    SchnorrDecomProof, SchnorrDecomProofParams,
};
use janus::{DkgOutput, DkgParams};

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use rand::rng;
use rayon::prelude::*;
use sha2::{Digest, Sha256, Sha512};

const FISCHLIN_SMALL: FischlinProofParams = FischlinProofParams {
    rho: 16,
    b: 8,
    t_bits: 13,
};

#[derive(Clone, Copy)]
struct Params {
    t: usize,
    n: usize,
}

impl Params {
    fn dkg(self) -> DkgParams {
        DkgParams {
            t: self.t,
            n: self.n,
        }
    }
    fn label(self) -> String {
        format!("t{}_n{}", self.t, self.n)
    }
}

// Criterion filters measurements, not setup, so capping the size is what makes
// a smoke run cheap. Also the way out if the largest set does not fit in memory.
fn parameter_sets() -> Vec<Params> {
    let max_n = std::env::var("JANUS_BENCH_MAX_N")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    all_parameter_sets()
        .into_iter()
        .filter(|p| p.n <= max_n)
        .collect()
}

fn all_parameter_sets() -> Vec<Params> {
    vec![
        Params { t: 4, n: 16 },
        Params { t: 8, n: 32 },
        Params { t: 16, n: 64 },
        Params { t: 32, n: 64 },
        Params { t: 64, n: 128 },
        Params { t: 128, n: 256 },
        Params { t: 256, n: 512 },
    ]
}

// A party holds the whole round at once, so the high-water mark is the number
// that matters. Linux exposes it directly.
fn peak_rss_bytes() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                let kb: u64 = rest
                    .trim()
                    .trim_end_matches(" kB")
                    .trim()
                    .parse()
                    .unwrap_or(0);
                return kb * 1024;
            }
        }
    }
    0
}

// Unavailable rather than zero off Linux, so a smoke run elsewhere does not
// look like a party that allocated nothing.
fn peak_rss() -> String {
    match peak_rss_bytes() {
        0 => "n/a".to_string(),
        b => format_bytes(b),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * KB;
    const GB: f64 = MB * KB;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

fn setup_parties(n: usize) -> (Vec<PartyState>, Parties) {
    let mut rng = rng();
    let states: Vec<PartyState> = (1..=n).map(|i| make_party_state(&mut rng, i)).collect();
    let parties = collect_public_parties(&states);
    (states, parties)
}

// One round: every party initiates and puts its message on the wire.
fn janus1_round<S>(
    p: Params,
    proof_params: &S::Params,
    states: &[PartyState],
    parties: &Parties,
) -> (Vec<Vec<u8>>, Vec<DkgInitLocalState>)
where
    S: PolyProofScheme,
    S::Params: Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let dkg = p.dkg();
    let out: Vec<(Vec<u8>, DkgInitLocalState)> = (1..=p.n)
        .into_par_iter()
        .map(|i| {
            let res = dkg_initiate::<_, S>(
                &mut rand::rng(),
                &dkg,
                proof_params,
                &states[i - 1],
                Scalar::from(i as u64),
                parties,
            );
            (res.broadcast.to_wire(), res.local)
        })
        .collect();
    out.into_iter().unzip()
}

fn bench_janus1<S>(c: &mut Criterion, name: &str, proof_params: S::Params)
where
    S: PolyProofScheme,
    S::Params: Sync + Clone,
    S::Proof:
        Clone + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
{
    let mut group = c.benchmark_group(format!("full_run_janus1_{name}"));
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg = p.dkg();
        let (states, parties) = setup_parties(p.n);
        let (wire, locals) = janus1_round::<S>(p, &proof_params, &states, &parties);

        let sent: u64 = wire[0].len() as u64 * (p.n as u64 - 1);
        let received: u64 = wire.iter().map(|w| w.len() as u64).sum::<u64>() - wire[0].len() as u64;
        eprintln!(
            "[janus1 {name} {}] sent={} received={} peak_rss={}",
            p.label(),
            format_bytes(sent),
            format_bytes(received),
            peak_rss(),
        );

        // What one party pays: build and encode its own message, then decode,
        // authenticate and verify everyone else's.
        group.bench_with_input(BenchmarkId::new("critical_path", p.label()), &p, |b, _| {
            b.iter(|| {
                let res = dkg_initiate::<_, S>(
                    &mut rand::rng(),
                    &dkg,
                    black_box(&proof_params),
                    &states[0],
                    Scalar::from(1u64),
                    &parties,
                );
                black_box(res.broadcast.to_wire());
                let out = dkg_output_key_generation_from_wire::<S>(
                    &dkg,
                    black_box(&proof_params),
                    &states[0],
                    &locals[0],
                    black_box(&wire),
                    &parties,
                )
                .expect("valid run");
                black_box(out);
            });
        });

        // Agreement costs a digest over everything received, on the same CPU budget as
        // verification. SHA-256, since x86 accelerates it and SHA-512 it does not.
        group.bench_with_input(BenchmarkId::new("echo_digest", p.label()), &p, |b, _| {
            b.iter(|| {
                let mut h = Sha256::new();
                for w in black_box(&wire) {
                    h.update(w);
                }
                black_box(h.finalize());
            });
        });

        // The same digest under the protocol's own hash, to price the missing
        // hardware support for SHA-512 on x86.
        group.bench_with_input(
            BenchmarkId::new("echo_digest_sha512", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    let mut h = Sha512::new();
                    for w in black_box(&wire) {
                        h.update(w);
                    }
                    black_box(h.finalize());
                });
            },
        );
    }
    group.finish();
}

#[allow(clippy::type_complexity)]
fn janus2_rounds<S>(
    p: Params,
    decom_params: &S::Params,
    states: &[PartyState],
    parties: &Parties,
) -> (
    Vec<Round1Broadcast<S::Proof>>,
    Vec<Vec<u8>>,
    Vec<Round1LocalState>,
    Vec<Vec<u8>>,
    Vec<janus::two_round::Round2LocalState>,
)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof: Clone + std::fmt::Debug + serde::Serialize + Send + Sync,
{
    let dkg = p.dkg();
    let (r1, r1_local): (Vec<_>, Vec<_>) = (1..=p.n)
        .into_par_iter()
        .map(|i| {
            dkg_round1_initiate::<_, S>(
                &mut rand::rng(),
                &dkg,
                decom_params,
                &states[i - 1],
                Scalar::from(i as u64),
                parties,
            )
        })
        .unzip();
    let r1_wire: Vec<Vec<u8>> = r1.par_iter().map(|m| m.to_wire()).collect();

    let (r2, r2_local): (Vec<_>, Vec<_>) = (1..=p.n)
        .into_par_iter()
        .map(|i| {
            dkg_round2_finalize::<S>(
                &dkg,
                decom_params,
                &states[i - 1],
                &r1_local[i - 1],
                &r1,
                parties,
            )
            .expect("valid finalize")
        })
        .unzip();
    let r2_wire: Vec<Vec<u8>> = r2.par_iter().map(|m| m.to_wire()).collect();

    (r1, r1_wire, r1_local, r2_wire, r2_local)
}

fn bench_janus2<S>(c: &mut Criterion, name: &str, decom_params: S::Params)
where
    S: DecomProofScheme<Statement = DecomStatement, Witness = DecomWitness>,
    S::Params: Clone + std::fmt::Debug + Sync,
    S::Proof:
        Clone + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
{
    let mut group = c.benchmark_group(format!("full_run_janus2_{name}"));
    group.sample_size(10);

    for p in parameter_sets() {
        let dkg = p.dkg();
        let (states, parties) = setup_parties(p.n);
        let (r1, r1_wire, _r1_local, r2_wire, r2_local) =
            janus2_rounds::<S>(p, &decom_params, &states, &parties);

        let sent = (r1_wire[0].len() + r2_wire[0].len()) as u64 * (p.n as u64 - 1);
        let received = (r1_wire.iter().map(|w| w.len() as u64).sum::<u64>()
            - r1_wire[0].len() as u64)
            + (r2_wire.iter().map(|w| w.len() as u64).sum::<u64>() - r2_wire[0].len() as u64);
        eprintln!(
            "[janus2 {name} {}] sent={} received={} peak_rss={}",
            p.label(),
            format_bytes(sent),
            format_bytes(received),
            peak_rss(),
        );

        group.bench_with_input(BenchmarkId::new("critical_path", p.label()), &p, |b, _| {
            b.iter(|| {
                // Round 1: build and encode.
                let (msg, local) = dkg_round1_initiate::<_, S>(
                    &mut rand::rng(),
                    &dkg,
                    black_box(&decom_params),
                    &states[0],
                    Scalar::from(1u64),
                    &parties,
                );
                black_box(msg.to_wire());

                // Round 2: decode the round-1 messages, then finalize.
                let decoded: Vec<Round1Broadcast<S::Proof>> = r1_wire
                    .iter()
                    .map(|w| {
                        Round1Broadcast::<S::Proof>::from_wire(w, &parties).expect("valid round1")
                    })
                    .collect();
                let (r2msg, _) = dkg_round2_finalize::<S>(
                    &dkg,
                    &decom_params,
                    &states[0],
                    &local,
                    &decoded,
                    &parties,
                )
                .expect("valid finalize");
                black_box(r2msg.to_wire());

                // Output: decode the round-2 messages, then combine.
                let r2_decoded: Vec<janus::two_round::Round2Broadcast> = r2_wire
                    .iter()
                    .map(|w| {
                        janus::two_round::Round2Broadcast::from_wire(w, &parties)
                            .expect("valid round2")
                    })
                    .collect();
                let out: DkgOutput = dkg_output::<S>(
                    &dkg,
                    &decom_params,
                    &states[0],
                    &r2_local[0],
                    &r1,
                    &r2_decoded,
                    &parties,
                )
                .expect("valid output");
                black_box(out);
            });
        });
    }
    group.finish();
}

fn full_run(c: &mut Criterion) {
    bench_janus1::<SchnorrPolyProof>(c, "schnorr", ());
    bench_janus1::<FischlinPolyProof>(c, "fischlin_small", FISCHLIN_SMALL);
    bench_janus2::<SchnorrDecomProof>(c, "schnorr", SchnorrDecomProofParams);
    bench_janus2::<FischlinDecomScheme>(
        c,
        "fischlin_small",
        FischlinDecomProofParams {
            rho: FISCHLIN_SMALL.rho,
            b: FISCHLIN_SMALL.b,
            t_bits: FISCHLIN_SMALL.t_bits,
        },
    );
}

criterion_group!(benches, full_run);
criterion_main!(benches);
