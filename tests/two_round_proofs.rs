// Decomposition, equality, and public key proofs of the two-round DKG.

use curve25519_dalek::scalar::Scalar;
use janus::group::{g, h};
use janus::pedersen::PedersenCommitment;
use janus::transcript::TranscriptExt;
use janus::two_round_proofs::comeq_proof::*;
use janus::two_round_proofs::decom_proof::*;
use janus::two_round_proofs::decom_proof_fischlin::*;
use janus::two_round_proofs::pk_proof::*;
use merlin::Transcript;
use rand::rng;

mod comeq_proof {
    use super::*;
    #[test]
    fn comeq_proof_roundtrip() {
        let mut rng = rng();

        let witness = ComEqWitness {
            s: Scalar::random(&mut rng),
            s_prime: Scalar::random(&mut rng),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = ComEqStatement {
            c: PedersenCommitment::new(witness.s, witness.s_prime),
            vk: PedersenCommitment::new(witness.s, witness.omega),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let proof = ComEqProof::prove(&mut rng, &statement, &witness);

        assert!(proof.verify(&statement));
    }

    #[test]
    fn comeq_proof_fails_for_wrong_c() {
        let mut rng = rng();

        let witness = ComEqWitness {
            s: Scalar::random(&mut rng),
            s_prime: Scalar::random(&mut rng),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = ComEqStatement {
            c: PedersenCommitment::new(witness.s, witness.s_prime),
            vk: PedersenCommitment::new(witness.s, witness.omega),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let wrong_statement = ComEqStatement {
            c: PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)),
            vk: statement.vk.clone(),
            d: statement.d.clone(),
        };

        let proof = ComEqProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn comeq_proof_fails_for_wrong_vk() {
        let mut rng = rng();

        let witness = ComEqWitness {
            s: Scalar::random(&mut rng),
            s_prime: Scalar::random(&mut rng),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = ComEqStatement {
            c: PedersenCommitment::new(witness.s, witness.s_prime),
            vk: PedersenCommitment::new(witness.s, witness.omega),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let wrong_statement = ComEqStatement {
            c: statement.c.clone(),
            vk: PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)),
            d: statement.d.clone(),
        };

        let proof = ComEqProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn comeq_proof_fails_for_wrong_d() {
        let mut rng = rng();

        let witness = ComEqWitness {
            s: Scalar::random(&mut rng),
            s_prime: Scalar::random(&mut rng),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = ComEqStatement {
            c: PedersenCommitment::new(witness.s, witness.s_prime),
            vk: PedersenCommitment::new(witness.s, witness.omega),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let wrong_statement = ComEqStatement {
            c: statement.c.clone(),
            vk: statement.vk.clone(),
            d: PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)),
        };

        let proof = ComEqProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn comeq_proof_fails_if_modified() {
        let mut rng = rng();

        let witness = ComEqWitness {
            s: Scalar::random(&mut rng),
            s_prime: Scalar::random(&mut rng),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = ComEqStatement {
            c: PedersenCommitment::new(witness.s, witness.s_prime),
            vk: PedersenCommitment::new(witness.s, witness.omega),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let mut proof = ComEqProof::prove(&mut rng, &statement, &witness);
        proof.z_omega += Scalar::ONE;

        assert!(!proof.verify(&statement));
    }

    #[test]
    fn comeq_proof_soundness_unlinked_s() {
        // Constructs a cheating proof for c=g^s1*h^{s'}, vk=g^s2*h^omega with s1≠s2.
        // The old summed verifier accepted such proofs; the fixed per-equation verifier must reject.
        let mut rng = rng();
        let g_point = g();
        let h_point = h();

        let s1 = Scalar::random(&mut rng);
        let s2 = Scalar::random(&mut rng);
        let s_prime = Scalar::random(&mut rng);
        let omega = Scalar::random(&mut rng);
        let r = Scalar::random(&mut rng);

        let statement = ComEqStatement {
            c: PedersenCommitment::new(s1, s_prime),
            vk: PedersenCommitment::new(s2, omega),
            d: PedersenCommitment::new(omega, r),
        };

        let alpha_s1 = Scalar::random(&mut rng);
        let alpha_s2 = Scalar::random(&mut rng);
        let alpha_s_prime = Scalar::random(&mut rng);
        let alpha_omega = Scalar::random(&mut rng);
        let alpha_r = Scalar::random(&mut rng);

        let t_c = g_point * alpha_s1 + h_point * alpha_s_prime;
        let t_vk = g_point * alpha_s2 + h_point * alpha_omega;
        let t_d = g_point * alpha_omega + h_point * alpha_r;

        let mut transcript = Transcript::new(b"comeq_proof");
        transcript.append_point(b"C", statement.c.point());
        transcript.append_point(b"vk", statement.vk.point());
        transcript.append_point(b"D", statement.d.point());
        transcript.append_point(b"T_c", &t_c);
        transcript.append_point(b"T_vk", &t_vk);
        transcript.append_point(b"T_d", &t_d);
        let e = transcript.challenge_scalar(b"e");

        // z_s satisfies 2*z_s + z_omega = (alpha_s1 + alpha_s2 + alpha_omega + e*(s1+s2+omega))
        // by setting z_omega = alpha_omega + e*omega and z_s = (alpha_s1 + alpha_s2 + e*(s1+s2)) / 2
        let two_inv = (Scalar::ONE + Scalar::ONE).invert();
        let z_s = (alpha_s1 + alpha_s2 + e * (s1 + s2)) * two_inv;
        let z_omega = alpha_omega + e * omega;
        let z_s_prime = alpha_s_prime + e * s_prime;
        let z_r = alpha_r + e * r;

        let cheating_proof = ComEqProof {
            t_c,
            t_vk,
            t_d,
            z_s,
            z_s_prime,
            z_omega,
            z_r,
        };
        assert!(!cheating_proof.verify(&statement));
    }
}

mod decom_proof {
    use super::*;
    #[test]
    fn decom_proof_roundtrip() {
        let mut rng = rng();

        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let proof = DecomProof::prove(&mut rng, &statement, &witness);

        assert!(proof.verify(&statement));
    }

    #[test]
    fn decom_proof_fails_for_wrong_statement() {
        let mut rng = rng();

        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let mut wrong_pedvss = statement.pedvss.clone();
        wrong_pedvss[2] =
            PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng));
        let wrong_statement = DecomStatement {
            pedvss: wrong_pedvss,
            d: statement.d.clone(),
        };

        let proof = DecomProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn decom_proof_fails_if_modified() {
        let mut rng = rng();

        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let mut proof = DecomProof::prove(&mut rng, &statement, &witness);
        proof.z_a[1] += Scalar::ONE;

        assert!(!proof.verify(&statement));
    }

    #[test]
    fn decom_proof_fails_for_wrong_length() {
        let mut rng = rng();

        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };

        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };

        let mut proof = DecomProof::prove(&mut rng, &statement, &witness);
        proof.z_b.pop();

        assert!(!proof.verify(&statement));
    }
}

mod decom_proof_fischlin {
    use super::*;
    fn params() -> FischlinDecomProofParams {
        FischlinDecomProofParams {
            rho: 4,
            b: 4,
            t_bits: 9,
        }
    }

    fn sample_witness_and_statement() -> (DecomWitness, DecomStatement) {
        let mut rng = rng();
        let witness = DecomWitness {
            a: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            b: (0..4).map(|_| Scalar::random(&mut rng)).collect(),
            omega: Scalar::random(&mut rng),
            r: Scalar::random(&mut rng),
        };
        let statement = DecomStatement {
            pedvss: witness
                .a
                .iter()
                .zip(witness.b.iter())
                .map(|(a, b)| PedersenCommitment::new(*a, *b))
                .collect(),
            d: PedersenCommitment::new(witness.omega, witness.r),
        };
        (witness, statement)
    }

    #[test]
    fn decom_fischlin_roundtrip() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);

        assert!(verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_for_wrong_statement() {
        let (witness, statement) = sample_witness_and_statement();

        let mut rng = rng();
        let mut wrong_pedvss = statement.pedvss.clone();
        wrong_pedvss[2] =
            PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng));
        let wrong_statement = DecomStatement {
            pedvss: wrong_pedvss,
            d: statement.d.clone(),
        };

        let p = params();
        let proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);

        assert!(!verify_fischlin_with_params(
            &wrong_statement,
            &proof,
            p.rho,
            p.b,
            p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_t1_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].t += g();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_e_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].e ^= 1;

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_z_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].z_a[1] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_round_removed() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds.pop();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }

    #[test]
    fn decom_fischlin_fails_if_vector_length_modified() {
        let (witness, statement) = sample_witness_and_statement();

        let p = params();
        let mut proof = prove_fischlin_with_params(&statement, &witness, p.rho, p.b, p.t_bits);
        proof.rounds[0].z_b.pop();

        assert!(!verify_fischlin_with_params(
            &statement, &proof, p.rho, p.b, p.t_bits
        ));
    }
}

mod pk_proof {
    use super::*;
    #[test]
    fn pk_proof_roundtrip() {
        let mut rng = rng();

        let witness = PkWitness {
            a: Scalar::random(&mut rng),
            b: Scalar::random(&mut rng),
        };

        let statement = PkStatement {
            pk: g() * witness.a,
            commitment: PedersenCommitment::new(witness.a, witness.b),
        };

        let proof = PkProof::prove(&mut rng, &statement, &witness);

        assert!(proof.verify(&statement));
    }

    #[test]
    fn pk_proof_fails_for_wrong_pk() {
        let mut rng = rng();

        let witness = PkWitness {
            a: Scalar::random(&mut rng),
            b: Scalar::random(&mut rng),
        };

        let statement = PkStatement {
            pk: g() * witness.a,
            commitment: PedersenCommitment::new(witness.a, witness.b),
        };

        let wrong_statement = PkStatement {
            pk: g() * Scalar::random(&mut rng),
            commitment: statement.commitment.clone(),
        };

        let proof = PkProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn pk_proof_fails_for_wrong_commitment() {
        let mut rng = rng();

        let witness = PkWitness {
            a: Scalar::random(&mut rng),
            b: Scalar::random(&mut rng),
        };

        let statement = PkStatement {
            pk: g() * witness.a,
            commitment: PedersenCommitment::new(witness.a, witness.b),
        };

        let wrong_statement = PkStatement {
            pk: statement.pk,
            commitment: PedersenCommitment::new(Scalar::random(&mut rng), Scalar::random(&mut rng)),
        };

        let proof = PkProof::prove(&mut rng, &statement, &witness);

        assert!(!proof.verify(&wrong_statement));
    }

    #[test]
    fn pk_proof_fails_if_modified() {
        let mut rng = rng();

        let witness = PkWitness {
            a: Scalar::random(&mut rng),
            b: Scalar::random(&mut rng),
        };

        let statement = PkStatement {
            pk: g() * witness.a,
            commitment: PedersenCommitment::new(witness.a, witness.b),
        };

        let mut proof = PkProof::prove(&mut rng, &statement, &witness);
        proof.z_a += Scalar::ONE;

        assert!(!proof.verify(&statement));
    }

    #[test]
    fn pk_proof_soundness_unlinked_discrete_logs() {
        // Constructs a cheating proof for (pk=g^a1, commitment=g^a2*h^b) with a1≠a2.
        // The old summed verifier accepted such proofs; the fixed per-equation verifier must reject.
        let mut rng = rng();
        let g_point = g();
        let h_point = h();

        let a1 = Scalar::random(&mut rng);
        let a2 = Scalar::random(&mut rng);
        let b = Scalar::random(&mut rng);

        let statement = PkStatement {
            pk: g_point * a1,
            commitment: PedersenCommitment::new(a2, b),
        };

        let alpha1 = Scalar::random(&mut rng);
        let alpha2 = Scalar::random(&mut rng);
        let alpha_b = Scalar::random(&mut rng);

        let t_pk = g_point * alpha1;
        let t_com = g_point * alpha2 + h_point * alpha_b;

        let mut transcript = Transcript::new(b"pk_proof");
        transcript.append_point(b"pk", &statement.pk);
        transcript.append_point(b"commitment", statement.commitment.point());
        transcript.append_point(b"T_pk", &t_pk);
        transcript.append_point(b"T_com", &t_com);
        let e = transcript.challenge_scalar(b"e");

        // z_a satisfies 2*z_a*G = (alpha1 + alpha2 + e*(a1+a2))*G (old combined check)
        let two_inv = (Scalar::ONE + Scalar::ONE).invert();
        let z_a = (alpha1 + alpha2 + e * (a1 + a2)) * two_inv;
        let z_b = alpha_b + e * b;

        let cheating_proof = PkProof {
            t_pk,
            t_com,
            z_a,
            z_b,
        };
        assert!(!cheating_proof.verify(&statement));
    }
}
