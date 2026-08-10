//! Cost of the identifiable abort path: building a complaint, verifying one, and
//! the worst case where every party complains. One-round verification is
//! constant, two-round grows with `t`. Complaint size is printed per setting.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::abort::{AbortReport, AbortVerdict};
use janus::encryption::open_share_with_shared;
use janus::encryption::proofs::prove_decryption;
use janus::one_round::{
    DkgInitBroadcast, dkg_initiate, verify_abort_report as verify_one_round_report,
};
use janus::one_round_proofs::{PolyProofScheme, SchnorrPolyProof};
use janus::party::{Parties, PartyState, collect_public_parties, make_party_state};
use janus::two_round::{
    Round1Broadcast, dkg_round1_initiate, verify_abort_report as verify_two_round_report,
};
use janus::two_round_proofs::{DecomProofScheme, SchnorrDecomProof, SchnorrDecomProofParams};
use rand::rng;

const DEALER: usize = 1;
const REPORTER: usize = 2;

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
        BaseParams { t: 8, n: 16 },
        BaseParams { t: 16, n: 32 },
        BaseParams { t: 32, n: 64 },
        BaseParams { t: 64, n: 128 },
        BaseParams { t: 128, n: 256 },
        BaseParams { t: 256, n: 512 },
    ]
}

fn setup_parties(n: usize) -> Vec<PartyState> {
    let mut rng = rng();
    (1..=n).map(|i| make_party_state(&mut rng, i)).collect()
}

// one round

struct OneRoundCase {
    parties: Parties,
    reporter: PartyState,
    accused: DkgInitBroadcast<<SchnorrPolyProof as PolyProofScheme>::Proof>,
    s_ji: Scalar,
    r_ji: Scalar,
    report: AbortReport,
}

fn make_one_round_case(p: BaseParams) -> OneRoundCase {
    let mut rng = rng();
    let dkg = p.to_dkg_params();
    let states = setup_parties(dkg.n);
    let parties = collect_public_parties(&states);

    let res = dkg_initiate::<_, SchnorrPolyProof>(
        &mut rng,
        &dkg,
        &(),
        &states[DEALER - 1],
        Scalar::from(7u64),
        &parties,
    );
    let mut accused = res.broadcast;
    accused
        .encrypted_shares
        .shares
        .get_mut(&REPORTER)
        .unwrap()
        .v1 += Scalar::ONE;
    accused.sign(&states[DEALER - 1].sig_sk);

    let reporter = states[REPORTER - 1].clone();
    let shared = accused.encrypted_shares.u * reporter.enc_sk;
    let corrupted = accused.encrypted_shares.shares.get(&REPORTER).unwrap();
    let (s_ji, r_ji) = open_share_with_shared(&shared, corrupted);

    let (shared_p, proof) = prove_decryption(
        &reporter.enc_sk,
        &reporter.enc_pk,
        &accused.encrypted_shares.u,
    );
    let report = AbortReport::new(
        REPORTER,
        DEALER,
        s_ji,
        r_ji,
        shared_p,
        proof,
        &reporter.sig_sk,
    );

    assert_eq!(
        verify_one_round_report(&parties, &accused, &report),
        AbortVerdict::DealerGuilty {
            dealer_idx: DEALER,
            reporter_idx: REPORTER,
        }
    );

    OneRoundCase {
        parties,
        reporter,
        accused,
        s_ji,
        r_ji,
        report,
    }
}

fn bench_one_round_report_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_one_round_report_create");
    group.sample_size(20);

    for p in parameter_sets() {
        let case = make_one_round_case(p);
        eprintln!(
            "[one-round abort {}] report={} bytes",
            p.label(),
            case.report.serialized_len(),
        );

        group.bench_with_input(BenchmarkId::new("report_create", p.label()), &p, |b, _| {
            b.iter(|| {
                let (shared, proof) = prove_decryption(
                    black_box(&case.reporter.enc_sk),
                    black_box(&case.reporter.enc_pk),
                    black_box(&case.accused.encrypted_shares.u),
                );
                let report = AbortReport::new(
                    REPORTER,
                    DEALER,
                    case.s_ji,
                    case.r_ji,
                    shared,
                    proof,
                    &case.reporter.sig_sk,
                );
                black_box(report);
            });
        });
    }

    group.finish();
}

fn bench_one_round_report_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_one_round_report_verify");
    group.sample_size(20);

    for p in parameter_sets() {
        let case = make_one_round_case(p);

        group.bench_with_input(BenchmarkId::new("report_verify", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(verify_one_round_report(
                    black_box(&case.parties),
                    black_box(&case.accused),
                    black_box(&case.report),
                ));
            });
        });
    }

    group.finish();
}

// two round

struct TwoRoundCase {
    parties: Parties,
    reporter: PartyState,
    accused: Round1Broadcast<<SchnorrDecomProof as DecomProofScheme>::Proof>,
    s_ji: Scalar,
    r_ji: Scalar,
    report: AbortReport,
}

fn make_two_round_case(p: BaseParams) -> TwoRoundCase {
    let mut rng = rng();
    let dkg = p.to_dkg_params();
    let states = setup_parties(dkg.n);
    let parties = collect_public_parties(&states);

    let (mut accused, _local) = dkg_round1_initiate::<_, SchnorrDecomProof>(
        &mut rng,
        &dkg,
        &SchnorrDecomProofParams,
        &states[DEALER - 1],
        Scalar::from(7u64),
        &parties,
    );
    accused
        .encrypted_shares
        .shares
        .get_mut(&REPORTER)
        .unwrap()
        .v1 += Scalar::ONE;
    accused.sign(&states[DEALER - 1].sig_sk);

    let reporter = states[REPORTER - 1].clone();
    let shared = accused.encrypted_shares.u * reporter.enc_sk;
    let corrupted = accused.encrypted_shares.shares.get(&REPORTER).unwrap();
    let (s_ji, r_ji) = open_share_with_shared(&shared, corrupted);

    let (shared_p, proof) = prove_decryption(
        &reporter.enc_sk,
        &reporter.enc_pk,
        &accused.encrypted_shares.u,
    );
    let report = AbortReport::new(
        REPORTER,
        DEALER,
        s_ji,
        r_ji,
        shared_p,
        proof,
        &reporter.sig_sk,
    );

    assert_eq!(
        verify_two_round_report(&parties, &accused, &report),
        AbortVerdict::DealerGuilty {
            dealer_idx: DEALER,
            reporter_idx: REPORTER,
        }
    );

    TwoRoundCase {
        parties,
        reporter,
        accused,
        s_ji,
        r_ji,
        report,
    }
}

fn bench_two_round_report_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_two_round_report_create");
    group.sample_size(20);

    for p in parameter_sets() {
        let case = make_two_round_case(p);
        eprintln!(
            "[two-round abort {}] report={} bytes",
            p.label(),
            case.report.serialized_len(),
        );

        group.bench_with_input(BenchmarkId::new("report_create", p.label()), &p, |b, _| {
            b.iter(|| {
                let (shared, proof) = prove_decryption(
                    black_box(&case.reporter.enc_sk),
                    black_box(&case.reporter.enc_pk),
                    black_box(&case.accused.encrypted_shares.u),
                );
                let report = AbortReport::new(
                    REPORTER,
                    DEALER,
                    case.s_ji,
                    case.r_ji,
                    shared,
                    proof,
                    &case.reporter.sig_sk,
                );
                black_box(report);
            });
        });
    }

    group.finish();
}

fn bench_two_round_report_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_two_round_report_verify");
    group.sample_size(20);

    for p in parameter_sets() {
        let case = make_two_round_case(p);

        group.bench_with_input(BenchmarkId::new("report_verify", p.label()), &p, |b, _| {
            b.iter(|| {
                black_box(verify_two_round_report(
                    black_box(&case.parties),
                    black_box(&case.accused),
                    black_box(&case.report),
                ));
            });
        });
    }

    group.finish();
}

// worst case: one malicious dealer, every other party complains

fn build_one_round_worstcase(
    p: BaseParams,
) -> (
    Parties,
    DkgInitBroadcast<<SchnorrPolyProof as PolyProofScheme>::Proof>,
    Vec<AbortReport>,
) {
    let mut rng = rng();
    let dkg = p.to_dkg_params();
    let states = setup_parties(dkg.n);
    let parties = collect_public_parties(&states);

    let res = dkg_initiate::<_, SchnorrPolyProof>(
        &mut rng,
        &dkg,
        &(),
        &states[DEALER - 1],
        Scalar::from(7u64),
        &parties,
    );
    let mut accused = res.broadcast;
    // The dealer corrupts every share it sends, so every receiver can complain.
    for j in 1..=dkg.n {
        if let Some(share) = accused.encrypted_shares.shares.get_mut(&j) {
            share.v1 += Scalar::ONE;
        }
    }
    accused.sign(&states[DEALER - 1].sig_sk);

    let mut reports = Vec::with_capacity(dkg.n - 1);
    for j in 1..=dkg.n {
        if j == DEALER {
            continue;
        }
        let reporter = &states[j - 1];
        let shared = accused.encrypted_shares.u * reporter.enc_sk;
        let share = accused.encrypted_shares.shares.get(&j).unwrap();
        let (s_ji, r_ji) = open_share_with_shared(&shared, share);
        let (shared_p, proof) = prove_decryption(
            &reporter.enc_sk,
            &reporter.enc_pk,
            &accused.encrypted_shares.u,
        );
        reports.push(AbortReport::new(
            j,
            DEALER,
            s_ji,
            r_ji,
            shared_p,
            proof,
            &reporter.sig_sk,
        ));
    }
    (parties, accused, reports)
}

fn build_two_round_worstcase(
    p: BaseParams,
) -> (
    Parties,
    Round1Broadcast<<SchnorrDecomProof as DecomProofScheme>::Proof>,
    Vec<AbortReport>,
) {
    let mut rng = rng();
    let dkg = p.to_dkg_params();
    let states = setup_parties(dkg.n);
    let parties = collect_public_parties(&states);

    let (mut accused, _local) = dkg_round1_initiate::<_, SchnorrDecomProof>(
        &mut rng,
        &dkg,
        &SchnorrDecomProofParams,
        &states[DEALER - 1],
        Scalar::from(7u64),
        &parties,
    );
    for j in 1..=dkg.n {
        if let Some(share) = accused.encrypted_shares.shares.get_mut(&j) {
            share.v1 += Scalar::ONE;
        }
    }
    accused.sign(&states[DEALER - 1].sig_sk);

    let mut reports = Vec::with_capacity(dkg.n - 1);
    for j in 1..=dkg.n {
        if j == DEALER {
            continue;
        }
        let reporter = &states[j - 1];
        let shared = accused.encrypted_shares.u * reporter.enc_sk;
        let share = accused.encrypted_shares.shares.get(&j).unwrap();
        let (s_ji, r_ji) = open_share_with_shared(&shared, share);
        let (shared_p, proof) = prove_decryption(
            &reporter.enc_sk,
            &reporter.enc_pk,
            &accused.encrypted_shares.u,
        );
        reports.push(AbortReport::new(
            j,
            DEALER,
            s_ji,
            r_ji,
            shared_p,
            proof,
            &reporter.sig_sk,
        ));
    }
    (parties, accused, reports)
}

fn bench_one_round_verify_worstcase(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_one_round_verify_worstcase");
    group.sample_size(10);

    for p in parameter_sets() {
        let (parties, accused, reports) = build_one_round_worstcase(p);
        group.bench_with_input(
            BenchmarkId::new("verify_n_minus_1", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    for report in &reports {
                        black_box(verify_one_round_report(&parties, &accused, report));
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_two_round_verify_worstcase(c: &mut Criterion) {
    let mut group = c.benchmark_group("abort_two_round_verify_worstcase");
    group.sample_size(10);

    for p in parameter_sets() {
        let (parties, accused, reports) = build_two_round_worstcase(p);
        group.bench_with_input(
            BenchmarkId::new("verify_n_minus_1", p.label()),
            &p,
            |b, _| {
                b.iter(|| {
                    for report in &reports {
                        black_box(verify_two_round_report(&parties, &accused, report));
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_one_round_report_create,
    bench_one_round_report_verify,
    bench_two_round_report_create,
    bench_two_round_report_verify,
    bench_one_round_verify_worstcase,
    bench_two_round_verify_worstcase,
);

criterion_main!(benches);
