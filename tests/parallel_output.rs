use curve25519_dalek::scalar::Scalar;
use janus::DkgParams;
use janus::one_round::{
    dkg_initiate, dkg_output_key_generation, dkg_output_key_generation_parallel,
};
use janus::one_round_proofs::{FischlinPolyProof, FischlinProofParams, SchnorrPolyProof};
use janus::party::{collect_public_parties, make_party_state};
use janus::two_round::{
    dkg_output, dkg_output_parallel, dkg_round1_initiate, dkg_round2_finalize,
    dkg_round2_finalize_parallel,
};
use janus::two_round_proofs::{SchnorrDecomProof, SchnorrDecomProofParams};
use rand::thread_rng;

const PARAMS: DkgParams = DkgParams { t: 8, n: 20 };

#[test]
fn parallel_output_matches_sequential() {
    let mut rng = thread_rng();
    let states: Vec<_> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();
    let parties = collect_public_parties(&states);

    // one-round Schnorr, then Fischlin small, both must agree seq vs parallel.
    let mut broadcasts = Vec::new();
    let mut locals = Vec::new();
    for i in 1..=PARAMS.n {
        let res = dkg_initiate::<_, SchnorrPolyProof>(
            &mut rng,
            &PARAMS,
            &(),
            &states[i - 1],
            Scalar::from(i as u64),
            &parties,
        );
        broadcasts.push(res.broadcast);
        locals.push(res.local);
    }

    let receiver = 1usize;
    let seq = dkg_output_key_generation::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[receiver - 1],
        &locals[receiver - 1],
        &broadcasts,
        &parties,
    )
    .expect("sequential output");
    let par = dkg_output_key_generation_parallel::<SchnorrPolyProof>(
        &PARAMS,
        &(),
        &states[receiver - 1],
        &locals[receiver - 1],
        &broadcasts,
        &parties,
    )
    .expect("parallel output");

    assert_eq!(seq.secret_share, par.secret_share);
    assert_eq!(seq.blinding_share, par.blinding_share);
    assert_eq!(seq.public_key, par.public_key);
    assert_eq!(seq.partial_verification_keys, par.partial_verification_keys);

    // Fischlin small path (parallel proof verification uses S::Params: Sync).
    let params = FischlinProofParams {
        rho: 16,
        b: 8,
        t_bits: 13,
    };
    let mut fbroadcasts = Vec::new();
    let mut flocals = Vec::new();
    for i in 1..=PARAMS.n {
        let res = dkg_initiate::<_, FischlinPolyProof>(
            &mut rng,
            &PARAMS,
            &params,
            &states[i - 1],
            Scalar::from(i as u64),
            &parties,
        );
        fbroadcasts.push(res.broadcast);
        flocals.push(res.local);
    }
    let fseq = dkg_output_key_generation::<FischlinPolyProof>(
        &PARAMS,
        &params,
        &states[receiver - 1],
        &flocals[receiver - 1],
        &fbroadcasts,
        &parties,
    )
    .expect("sequential fischlin output");
    let fpar = dkg_output_key_generation_parallel::<FischlinPolyProof>(
        &PARAMS,
        &params,
        &states[receiver - 1],
        &flocals[receiver - 1],
        &fbroadcasts,
        &parties,
    )
    .expect("parallel fischlin output");
    assert_eq!(fseq.secret_share, fpar.secret_share);
    assert_eq!(fseq.public_key, fpar.public_key);
    assert_eq!(
        fseq.partial_verification_keys,
        fpar.partial_verification_keys
    );
}

#[test]
fn parallel_two_round_matches_sequential() {
    let mut rng = thread_rng();
    let states: Vec<_> = (1..=PARAMS.n)
        .map(|i| make_party_state(&mut rng, i))
        .collect();
    let parties = collect_public_parties(&states);

    let mut r1 = Vec::new();
    let mut r1_locals = Vec::new();
    for i in 1..=PARAMS.n {
        let (b, l) = dkg_round1_initiate::<_, SchnorrDecomProof>(
            &mut rng,
            &PARAMS,
            &SchnorrDecomProofParams,
            &states[i - 1],
            Scalar::from(i as u64),
            &parties,
        );
        r1.push(b);
        r1_locals.push(l);
    }

    let recv = 1usize;
    let (_r2_seq, l2_seq) = dkg_round2_finalize::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[recv - 1],
        &r1_locals[recv - 1],
        &r1,
        &parties,
    )
    .expect("sequential finalize");
    let (_r2_par, l2_par) = dkg_round2_finalize_parallel::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[recv - 1],
        &r1_locals[recv - 1],
        &r1,
        &parties,
    )
    .expect("parallel finalize");
    assert_eq!(l2_seq.s_i, l2_par.s_i);
    assert_eq!(l2_seq.omega, l2_par.omega);

    let mut r2 = Vec::new();
    let mut r2_locals = Vec::new();
    for i in 1..=PARAMS.n {
        let (b, l) = dkg_round2_finalize::<SchnorrDecomProof>(
            &PARAMS,
            &SchnorrDecomProofParams,
            &states[i - 1],
            &r1_locals[i - 1],
            &r1,
            &parties,
        )
        .expect("finalize all");
        r2.push(b);
        r2_locals.push(l);
    }
    let out_seq = dkg_output::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[recv - 1],
        &r2_locals[recv - 1],
        &r1,
        &r2,
        &parties,
    )
    .expect("sequential output");
    let out_par = dkg_output_parallel::<SchnorrDecomProof>(
        &PARAMS,
        &SchnorrDecomProofParams,
        &states[recv - 1],
        &r2_locals[recv - 1],
        &r1,
        &r2,
        &parties,
    )
    .expect("parallel output");
    assert_eq!(out_seq.secret_share, out_par.secret_share);
    assert_eq!(out_seq.public_key, out_par.public_key);
    assert_eq!(
        out_seq.partial_verification_keys,
        out_par.partial_verification_keys
    );
}
