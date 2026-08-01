use super::*;
use crate::pedersen::PedersenCommitment;
use crate::poly::eval_poly_at;
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedStatement {
    pub x_points: Vec<Scalar>,                // evaluation points x_1, ..., x_n
    pub commitments: Vec<PedersenCommitment>, // C_j = g^{f(x_j)} h^{omega_j}
    pub f0_commitment: RistrettoPoint,        // pk = g^{a_0}
    pub degree: usize, // f has degree `degree`; the proof binds exactly degree+1 coefficients
}

#[derive(Clone, Debug, zeroize::ZeroizeOnDrop)]
pub struct PolyWellFormedWitness {
    pub coeffs: Vec<Scalar>,    // a_0, ..., a_degree
    pub blindings: Vec<Scalar>, // omega_1, ..., omega_n
}

// One first-round commitment per equation: T_0 for f0, T_1..T_n for the n
// commitments. Verification checks every equation, so a malicious response can no
// longer satisfy a single aggregate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedProof {
    pub t_commitments: Vec<RistrettoPoint>, // length n+1: [T_0, T_1, ..., T_n]
    pub z_coeffs: Vec<Scalar>,              // length degree+1
    pub z_blindings: Vec<Scalar>,           // length n
}

pub fn prove(
    statement: &PolyWellFormedStatement,
    witness: &PolyWellFormedWitness,
) -> PolyWellFormedProof {
    let n = statement.x_points.len();
    assert_eq!(
        statement.commitments.len(),
        n,
        "commitments length mismatch"
    );
    assert_eq!(witness.blindings.len(), n, "blindings length mismatch");
    assert_eq!(
        witness.coeffs.len(),
        statement.degree + 1,
        "coeffs length must be degree+1"
    );

    let mut rng = UnwrapErr(SysRng);
    let r_coeffs: Zeroizing<Vec<Scalar>> = Zeroizing::new(
        (0..=statement.degree)
            .map(|_| Scalar::random(&mut rng))
            .collect(),
    );
    let r_blindings: Zeroizing<Vec<Scalar>> =
        Zeroizing::new((0..n).map(|_| Scalar::random(&mut rng)).collect());

    let mut t_commitments = Vec::with_capacity(n + 1);
    t_commitments.push(g_mul_scalar(r_coeffs[0])); // T_0: f0 = g^{a_0}, no blinding
    for j in 0..n {
        let e = eval_poly_at(&r_coeffs, statement.x_points[j]);
        t_commitments.push(g_mul_scalar(e) + h_mul_scalar(r_blindings[j]));
    }

    let mut transcript = Transcript::new(b"poly-well-formedness");
    append_statement_to_transcript(&mut transcript, statement);
    for t in &t_commitments {
        transcript.append_point(b"t", t);
    }
    let c = transcript.challenge_scalar(b"challenge");

    let z_coeffs: Vec<Scalar> = r_coeffs
        .iter()
        .zip(witness.coeffs.iter())
        .map(|(r, a)| *r + c * *a)
        .collect();
    let z_blindings: Vec<Scalar> = r_blindings
        .iter()
        .zip(witness.blindings.iter())
        .map(|(r, omega)| *r + c * *omega)
        .collect();

    PolyWellFormedProof {
        t_commitments,
        z_coeffs,
        z_blindings,
    }
}

pub fn verify(statement: &PolyWellFormedStatement, proof: &PolyWellFormedProof) -> bool {
    let n = statement.x_points.len();
    if statement.commitments.len() != n
        || proof.t_commitments.len() != n + 1
        || proof.z_blindings.len() != n
        || proof.z_coeffs.len() != statement.degree + 1
    {
        return false;
    }

    let mut transcript = Transcript::new(b"poly-well-formedness");
    append_statement_to_transcript(&mut transcript, statement);
    for t in &proof.t_commitments {
        transcript.append_point(b"t", t);
    }
    let c = transcript.challenge_scalar(b"challenge");

    // Fold weights from the same transcript, binding the responses. Deriving them
    // after c avoids re-absorbing the statement and T commitments a second time.
    transcript.append_message(b"dom-sep", b"poly-well-formedness-batch-verify");
    for z in &proof.z_coeffs {
        transcript.append_scalar(b"za", z);
    }
    for z in &proof.z_blindings {
        transcript.append_scalar(b"zw", z);
    }
    let deltas: Vec<Scalar> = (0..=n)
        .map(|i| {
            transcript.append_message(b"delta_index", &(i as u64).to_le_bytes());
            transcript.challenge_scalar(b"delta")
        })
        .collect();

    let round = RoundView {
        t_commitments: &proof.t_commitments,
        challenge: c,
        z_coeffs: &proof.z_coeffs,
        z_blindings: &proof.z_blindings,
    };
    batched_point_check_rank1(statement, &[round], &deltas, &[Scalar::ONE])
}

pub fn batch_verify(
    statements: &[PolyWellFormedStatement],
    proofs: &[PolyWellFormedProof],
) -> bool {
    if statements.len() != proofs.len() {
        return false;
    }
    statements
        .iter()
        .zip(proofs.iter())
        .all(|(s, p)| verify(s, p))
}

#[derive(Clone, Debug)]
pub struct SchnorrPolyProof;

impl PolyProofScheme for SchnorrPolyProof {
    type Proof = PolyWellFormedProof;
    type Params = ();

    fn prove(
        _params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof {
        prove(statement, witness)
    }

    fn verify(
        _params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool {
        verify(statement, proof)
    }
}
