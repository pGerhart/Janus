use super::*;
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};
use zeroize::{Zeroizing, ZeroizeOnDrop};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComEqStatement {
    pub c: PedersenCommitment,
    pub vk: PedersenCommitment,
    pub d: PedersenCommitment,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct ComEqWitness {
    pub s: Scalar,
    pub s_prime: Scalar,
    pub omega: Scalar,
    pub r: Scalar,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComEqProof {
    pub t_c: RistrettoPoint,
    pub t_vk: RistrettoPoint,
    pub t_d: RistrettoPoint,
    pub z_s: Scalar,
    pub z_s_prime: Scalar,
    pub z_omega: Scalar,
    pub z_r: Scalar,
}

impl ComEqProof {
    fn transcript(statement: &ComEqStatement) -> Transcript {
        let mut transcript = Transcript::new(b"comeq_proof");
        transcript.append_point(b"C", statement.c.point());
        transcript.append_point(b"vk", statement.vk.point());
        transcript.append_point(b"D", statement.d.point());
        transcript
    }

    pub fn prove<R: RngCore + CryptoRng>(
        rng: &mut R,
        statement: &ComEqStatement,
        witness: &ComEqWitness,
    ) -> Self {
        let mut transcript = Self::transcript(statement);
        let g_point = g();
        let h_point = h();

        let alpha_s = Zeroizing::new(Scalar::random(rng));
        let alpha_s_prime = Zeroizing::new(Scalar::random(rng));
        let alpha_omega = Zeroizing::new(Scalar::random(rng));
        let alpha_r = Zeroizing::new(Scalar::random(rng));

        let t_c = g_point * *alpha_s + h_point * *alpha_s_prime;
        let t_vk = g_point * *alpha_s + h_point * *alpha_omega;
        let t_d = g_point * *alpha_omega + h_point * *alpha_r;

        transcript.append_point(b"T_c", &t_c);
        transcript.append_point(b"T_vk", &t_vk);
        transcript.append_point(b"T_d", &t_d);

        let e = transcript.challenge_scalar(b"e");

        let z_s = *alpha_s + e * witness.s;
        let z_s_prime = *alpha_s_prime + e * witness.s_prime;
        let z_omega = *alpha_omega + e * witness.omega;
        let z_r = *alpha_r + e * witness.r;

        Self {
            t_c,
            t_vk,
            t_d,
            z_s,
            z_s_prime,
            z_omega,
            z_r,
        }
    }

    pub fn verify(&self, statement: &ComEqStatement) -> bool {
        let mut transcript = Self::transcript(statement);
        let g_point = g();
        let h_point = h();
        let c_point = *statement.c.point();
        let vk_point = *statement.vk.point();
        let d_point = *statement.d.point();

        transcript.append_point(b"T_c", &self.t_c);
        transcript.append_point(b"T_vk", &self.t_vk);
        transcript.append_point(b"T_d", &self.t_d);

        let e = transcript.challenge_scalar(b"e");

        // eq1: z_s * G + z_s_prime * H = t_c + e * c
        let check_c = RistrettoPoint::vartime_multiscalar_mul(
            [self.z_s, self.z_s_prime, -Scalar::ONE, -e],
            [g_point, h_point, self.t_c, c_point],
        );

        // eq2: z_s * G + z_omega * H = t_vk + e * vk  (same z_s enforces s is shared with c)
        let check_vk = RistrettoPoint::vartime_multiscalar_mul(
            [self.z_s, self.z_omega, -Scalar::ONE, -e],
            [g_point, h_point, self.t_vk, vk_point],
        );

        // eq3: z_omega * G + z_r * H = t_d + e * d  (same z_omega enforces omega is shared with vk)
        let check_d = RistrettoPoint::vartime_multiscalar_mul(
            [self.z_omega, self.z_r, -Scalar::ONE, -e],
            [g_point, h_point, self.t_d, d_point],
        );

        check_c == RistrettoPoint::identity()
            && check_vk == RistrettoPoint::identity()
            && check_d == RistrettoPoint::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn comeq_proof_roundtrip() {
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();
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

        let cheating_proof = ComEqProof { t_c, t_vk, t_d, z_s, z_s_prime, z_omega, z_r };
        assert!(!cheating_proof.verify(&statement));
    }
}
