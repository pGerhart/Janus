use super::*;
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};
use zeroize::{ZeroizeOnDrop, Zeroizing};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecomStatement {
    pub pedvss: Vec<PedersenCommitment>,
    pub d: PedersenCommitment,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct DecomWitness {
    pub a: Vec<Scalar>,
    pub b: Vec<Scalar>,
    pub omega: Scalar,
    pub r: Scalar,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecomProof {
    pub t: RistrettoPoint,
    pub z_a: Vec<Scalar>,
    pub z_b: Vec<Scalar>,
    pub z_omega: Scalar,
    pub z_r: Scalar,
}

impl DecomProof {
    fn transcript(statement: &DecomStatement) -> Transcript {
        let mut transcript = Transcript::new(b"decom_proof_combined");
        transcript.append_u64(b"pedvss_len", statement.pedvss.len() as u64);
        for (k, c_k) in statement.pedvss.iter().enumerate() {
            transcript.append_u64(b"pedvss_idx", k as u64);
            transcript.append_point(b"C_k", c_k.point());
        }
        transcript.append_point(b"D", statement.d.point());
        transcript
    }

    fn derive_lambdas(transcript: &mut Transcript, count: usize) -> Vec<Scalar> {
        (0..count)
            .map(|k| {
                transcript.append_u64(b"lambda_idx", k as u64);
                transcript.challenge_scalar(b"lambda")
            })
            .collect()
    }

    fn validate_shapes(
        statement: &DecomStatement,
        witness: Option<&DecomWitness>,
        proof: Option<&Self>,
    ) -> bool {
        if statement.pedvss.is_empty() {
            return false;
        }

        if let Some(witness) = witness
            && (witness.a.len() != statement.pedvss.len()
                || witness.b.len() != statement.pedvss.len())
        {
            return false;
        }

        if let Some(proof) = proof
            && (proof.z_a.len() != statement.pedvss.len()
                || proof.z_b.len() != statement.pedvss.len())
        {
            return false;
        }

        true
    }

    pub fn prove<R: RngCore + CryptoRng>(
        rng: &mut R,
        statement: &DecomStatement,
        witness: &DecomWitness,
    ) -> Self {
        assert!(
            Self::validate_shapes(statement, Some(witness), None),
            "invalid decom proof witness shape"
        );

        let g_point = g();
        let h_point = h();
        let n = statement.pedvss.len();

        let mut transcript = Self::transcript(statement);

        // Derive n+1 lambdas from the statement before committing any randomness.
        // lambdas[0..n-1] weight the pedvss equations, lambdas[n] weights the d equation.
        let lambdas = Self::derive_lambdas(&mut transcript, n + 1);

        // Sample individual nonces for each witness component.
        let alpha_a: Zeroizing<Vec<Scalar>> =
            Zeroizing::new((0..n).map(|_| Scalar::random(rng)).collect());
        let alpha_b: Zeroizing<Vec<Scalar>> =
            Zeroizing::new((0..n).map(|_| Scalar::random(rng)).collect());
        let alpha_omega = Zeroizing::new(Scalar::random(rng));
        let alpha_r = Zeroizing::new(Scalar::random(rng));

        // Compute combined nonce scalars (pure scalar arithmetic, no group ops).
        let combined_a = alpha_a
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (a, l)| acc + a * l)
            + lambdas[n] * *alpha_omega;
        let combined_b = alpha_b
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (b, l)| acc + b * l)
            + lambdas[n] * *alpha_r;

        // Single commitment: 2 group multiplications regardless of t.
        let t = g_point * combined_a + h_point * combined_b;

        transcript.append_point(b"T", &t);
        let e = transcript.challenge_scalar(b"e");

        // Individual responses — one per witness scalar, extraction works component-wise.
        let z_a = alpha_a
            .iter()
            .zip(witness.a.iter())
            .map(|(alpha, a)| alpha + e * a)
            .collect();
        let z_b = alpha_b
            .iter()
            .zip(witness.b.iter())
            .map(|(alpha, b)| alpha + e * b)
            .collect();
        let z_omega = *alpha_omega + e * witness.omega;
        let z_r = *alpha_r + e * witness.r;

        Self {
            t,
            z_a,
            z_b,
            z_omega,
            z_r,
        }
    }

    pub fn verify(&self, statement: &DecomStatement) -> bool {
        if !Self::validate_shapes(statement, None, Some(self)) {
            return false;
        }

        let g_point = g();
        let h_point = h();
        let n = statement.pedvss.len();

        let mut transcript = Self::transcript(statement);

        let lambdas = Self::derive_lambdas(&mut transcript, n + 1);

        transcript.append_point(b"T", &self.t);
        let e = transcript.challenge_scalar(b"e");

        // Combined response scalars (pure scalar arithmetic).
        let combined_z_a = self
            .z_a
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (z, l)| acc + z * l)
            + lambdas[n] * self.z_omega;
        let combined_z_b = self
            .z_b
            .iter()
            .zip(&lambdas)
            .fold(Scalar::ZERO, |acc, (z, l)| acc + z * l)
            + lambdas[n] * self.z_r;

        let mut scalars = Vec::with_capacity(n + 4);
        let mut points = Vec::with_capacity(n + 4);

        scalars.extend_from_slice(&[combined_z_a, combined_z_b, -Scalar::ONE]);
        points.extend_from_slice(&[g_point, h_point, self.t]);

        for (k, c_k) in statement.pedvss.iter().enumerate() {
            scalars.push(-e * lambdas[k]);
            points.push(*c_k.point());
        }
        scalars.push(-e * lambdas[n]);
        points.push(*statement.d.point());

        RistrettoPoint::vartime_multiscalar_mul(scalars, points) == RistrettoPoint::identity()
    }
}

#[derive(Clone, Debug)]
pub struct SchnorrDecomProof;

#[derive(Clone, Debug)]
pub struct SchnorrDecomProofParams;

impl DecomProofScheme for SchnorrDecomProof {
    type Statement = DecomStatement;
    type Witness = DecomWitness;
    type Proof = DecomProof;
    type Params = SchnorrDecomProofParams;

    fn prove(
        _params: &Self::Params,
        statement: &Self::Statement,
        witness: &Self::Witness,
    ) -> Self::Proof {
        let mut rng = rand::thread_rng();
        DecomProof::prove(&mut rng, statement, witness)
    }

    fn verify(_params: &Self::Params, statement: &Self::Statement, proof: &Self::Proof) -> bool {
        proof.verify(statement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn decom_proof_roundtrip() {
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
        let mut rng = thread_rng();

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
