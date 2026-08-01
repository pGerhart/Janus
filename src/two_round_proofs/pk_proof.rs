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

    pub fn prove<R: CryptoRng>(rng: &mut R, statement: &PkStatement, witness: &PkWitness) -> Self {
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
