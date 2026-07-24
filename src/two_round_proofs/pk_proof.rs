use super::*;
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};
use zeroize::{ZeroizeOnDrop, Zeroizing};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkStatement {
    pub pk: RistrettoPoint,
    pub commitment: PedersenCommitment,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct PkWitness {
    pub a: Scalar,
    pub b: Scalar,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkProof {
    pub t_pk: RistrettoPoint,
    pub t_com: RistrettoPoint,
    pub z_a: Scalar,
    pub z_b: Scalar,
}

impl PkProof {
    fn transcript(statement: &PkStatement) -> Transcript {
        let mut transcript = Transcript::new(b"pk_proof");
        transcript.append_point(b"pk", &statement.pk);
        transcript.append_point(b"commitment", statement.commitment.point());
        transcript
    }

    pub fn prove<R: RngCore + CryptoRng>(
        rng: &mut R,
        statement: &PkStatement,
        witness: &PkWitness,
    ) -> Self {
        let mut transcript = Self::transcript(statement);
        let g_point = g();
        let h_point = h();

        let alpha_a = Zeroizing::new(Scalar::random(rng));
        let alpha_b = Zeroizing::new(Scalar::random(rng));

        let t_pk = g_point * *alpha_a;
        let t_com = g_point * *alpha_a + h_point * *alpha_b;

        transcript.append_point(b"T_pk", &t_pk);
        transcript.append_point(b"T_com", &t_com);

        let e = transcript.challenge_scalar(b"e");

        let z_a = *alpha_a + e * witness.a;
        let z_b = *alpha_b + e * witness.b;

        Self {
            t_pk,
            t_com,
            z_a,
            z_b,
        }
    }

    pub fn verify(&self, statement: &PkStatement) -> bool {
        let mut transcript = Self::transcript(statement);
        let g_point = g();
        let h_point = h();
        let commitment_point = *statement.commitment.point();

        transcript.append_point(b"T_pk", &self.t_pk);
        transcript.append_point(b"T_com", &self.t_com);

        let e = transcript.challenge_scalar(b"e");

        // eq1: z_a * G = t_pk + e * pk
        let check_pk = RistrettoPoint::vartime_multiscalar_mul(
            [self.z_a, -Scalar::ONE, -e],
            [g_point, self.t_pk, statement.pk],
        );

        // eq2: z_a * G + z_b * H = t_com + e * commitment  (same z_a enforces shared a)
        let check_com = RistrettoPoint::vartime_multiscalar_mul(
            [self.z_a, self.z_b, -Scalar::ONE, -e],
            [g_point, h_point, self.t_com, commitment_point],
        );

        check_pk == RistrettoPoint::identity() && check_com == RistrettoPoint::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn pk_proof_roundtrip() {
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();
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
