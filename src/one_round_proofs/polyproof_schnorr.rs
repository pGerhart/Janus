use super::*;
use crate::pedersen::PedersenCommitment;
use curve25519_dalek::traits::{Identity, VartimeMultiscalarMul};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedStatement {
    pub x_points: Vec<Scalar>, // evaluations of the polynomial: (1, ..., n)
    pub commitments: Vec<PedersenCommitment>, // pedersen commitments of the evaluations: C_i = g^{f(x_i)} h^{rho_i}
    pub f0_commitment: RistrettoPoint,        // pk_i = g^{a0}
}

#[derive(Clone, Debug, zeroize::ZeroizeOnDrop)]
pub struct PolyWellFormedWitness {
    pub coeffs: Vec<Scalar>,
    pub blindings: Vec<Scalar>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedProof {
    pub t: RistrettoPoint, // commitment to the random linear combination of the coefficients and blindings
    pub z_coeffs: Vec<Scalar>, // responses for the coefficients: z_j = r_j + c * a_j
    pub z_blindings: Vec<Scalar>, // responses for the blindings: z_rho_i = r_rho_i + c * rho_i
}

pub fn prove(
    statement: &PolyWellFormedStatement,
    witness: &PolyWellFormedWitness,
) -> PolyWellFormedProof {
    let n = statement.x_points.len();
    assert_eq!(
        statement.commitments.len(),
        n,
        "x_points and commitments length mismatch"
    );
    assert_eq!(witness.blindings.len(), n, "blindings length mismatch");
    assert!(
        !witness.coeffs.is_empty(),
        "coeffs must contain at least a0"
    );

    let degree = witness.coeffs.len() - 1;

    let mut transcript = Transcript::new(b"poly-well-formedness");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, degree);

    let mut rng = OsRng;

    let r_coeffs: Zeroizing<Vec<Scalar>> =
        Zeroizing::new((0..=degree).map(|_| Scalar::random(&mut rng)).collect());
    let r_blindings: Zeroizing<Vec<Scalar>> =
        Zeroizing::new((0..n).map(|_| Scalar::random(&mut rng)).collect());

    let coeff_part = r_coeffs
        .iter()
        .zip(mus.iter())
        .fold(Scalar::ZERO, |acc, (r, mu)| acc + (*r * *mu));

    let blind_part = r_blindings
        .iter()
        .zip(lambdas.iter().skip(1))
        .fold(Scalar::ZERO, |acc, (r, lambda)| acc + (*r * *lambda));

    let t = g() * coeff_part + h() * blind_part;

    transcript.append_point(b"t", &t);
    let c = transcript.challenge_scalar(b"challenge");

    let z_coeffs: Vec<Scalar> = r_coeffs
        .iter()
        .zip(witness.coeffs.iter())
        .map(|(r, a)| *r + c * *a)
        .collect();

    let z_blindings: Vec<Scalar> = r_blindings
        .iter()
        .zip(witness.blindings.iter())
        .map(|(r, rho)| *r + c * *rho)
        .collect();

    PolyWellFormedProof {
        t,
        z_coeffs,
        z_blindings,
    }
}

pub fn verify(statement: &PolyWellFormedStatement, proof: &PolyWellFormedProof) -> bool {
    let n = statement.x_points.len();

    if statement.commitments.len() != n {
        return false;
    }

    if proof.z_blindings.len() != n {
        return false;
    }

    if proof.z_coeffs.is_empty() {
        return false;
    }

    let degree = proof.z_coeffs.len() - 1;

    let mut transcript = Transcript::new(b"poly-well-formedness");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, degree);

    let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

    transcript.append_point(b"t", &proof.t);
    let c = transcript.challenge_scalar(b"challenge");

    let coeff_part = proof
        .z_coeffs
        .iter()
        .zip(mus.iter())
        .fold(Scalar::ZERO, |acc, (z, mu)| acc + (*z * *mu));

    let blind_part = proof
        .z_blindings
        .iter()
        .zip(lambdas.iter().skip(1))
        .fold(Scalar::ZERO, |acc, (z, lambda)| acc + (*z * *lambda));

    let lhs = g() * coeff_part + h() * blind_part;
    let rhs = proof.t + p_star * c;

    lhs == rhs
}

pub fn batch_verify(
    statements: &[PolyWellFormedStatement],
    proofs: &[PolyWellFormedProof],
) -> bool {
    if statements.len() != proofs.len() {
        return false;
    }

    if statements.is_empty() {
        return true;
    }

    let mut batch_transcript = Transcript::new(b"poly-well-formedness-batch");

    let mut lhs_coeff_sum = Scalar::ZERO;
    let mut lhs_blind_sum = Scalar::ZERO;
    let mut msm_scalars: Vec<Scalar> = Vec::with_capacity(2 * statements.len() + 2);
    let mut msm_points: Vec<RistrettoPoint> = Vec::with_capacity(2 * statements.len() + 2);

    for (statement, proof) in statements.iter().zip(proofs.iter()) {
        let n = statement.x_points.len();

        if statement.commitments.len() != n {
            return false;
        }
        if proof.z_blindings.len() != n {
            return false;
        }
        if proof.z_coeffs.is_empty() {
            return false;
        }

        let degree = proof.z_coeffs.len() - 1;

        let mut transcript = Transcript::new(b"poly-well-formedness");
        append_statement_to_transcript(&mut transcript, statement);

        let lambdas = derive_lambdas(&mut transcript, n);
        let mus = compute_mus(&statement.x_points, &lambdas, degree);
        let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

        transcript.append_point(b"t", &proof.t);
        let c = transcript.challenge_scalar(b"challenge");

        let coeff_part = proof
            .z_coeffs
            .iter()
            .zip(mus.iter())
            .fold(Scalar::ZERO, |acc, (z, mu)| acc + (*z * *mu));

        let blind_part = proof
            .z_blindings
            .iter()
            .zip(lambdas.iter().skip(1))
            .fold(Scalar::ZERO, |acc, (z, lambda)| acc + (*z * *lambda));

        // Deterministische Batch-Gewichte aus einem separaten Transcript
        append_statement_to_transcript(&mut batch_transcript, statement);
        batch_transcript.append_point(b"t", &proof.t);
        let alpha = batch_transcript.challenge_scalar(b"batch-weight");

        lhs_coeff_sum += alpha * coeff_part;
        lhs_blind_sum += alpha * blind_part;

        msm_scalars.push(-alpha);
        msm_points.push(proof.t);
        msm_scalars.push(-(alpha * c));
        msm_points.push(p_star);
    }

    msm_scalars.push(lhs_coeff_sum);
    msm_points.push(g());
    msm_scalars.push(lhs_blind_sum);
    msm_points.push(h());

    RistrettoPoint::vartime_multiscalar_mul(msm_scalars, msm_points) == RistrettoPoint::identity()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{g, h};
    use crate::poly::eval_poly_at;
    use curve25519_dalek::scalar::Scalar;

    fn build_instance(
        coeffs: Vec<Scalar>,
        xs: Vec<Scalar>,
        blindings: Vec<Scalar>,
    ) -> (PolyWellFormedStatement, PolyWellFormedWitness) {
        let commitments: Vec<PedersenCommitment> = xs
            .iter()
            .zip(blindings.iter())
            .map(|(x, rho)| {
                let fx = eval_poly_at(&coeffs, *x);
                PedersenCommitment::new(fx, *rho)
            })
            .collect();

        let statement = PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g() * coeffs[0],
        };

        let witness = PolyWellFormedWitness { coeffs, blindings };

        (statement, witness)
    }

    #[test]
    fn test_poly_well_formed_proof() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);
        assert!(verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_commitment_is_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);

        let modified = *statement.commitments[1].point() + g() * Scalar::ONE;
        statement.commitments[1] = PedersenCommitment::from_point(modified);

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_f0_commitment_is_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);

        statement.f0_commitment += g() * Scalar::ONE;

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_t_is_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let mut proof = prove(&statement, &witness);
        proof.t += h() * Scalar::ONE;

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_z_coeff_is_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let mut proof = prove(&statement, &witness);
        proof.z_coeffs[1] += Scalar::ONE;

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_z_blinding_is_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let mut proof = prove(&statement, &witness);
        proof.z_blindings[0] += Scalar::ONE;

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_for_wrong_witness() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, mut witness) = build_instance(coeffs, xs, blindings);

        witness.coeffs[2] += Scalar::ONE;

        let proof = prove(&statement, &witness);

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_constant_polynomial() {
        let coeffs = vec![Scalar::from(42u64)];
        let xs = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(5u64),
            Scalar::from(9u64),
        ];
        let blindings = vec![
            Scalar::from(10u64),
            Scalar::from(11u64),
            Scalar::from(12u64),
            Scalar::from(13u64),
        ];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);
        assert!(verify(&statement, &proof));
    }

    #[test]
    fn test_many_evaluation_points() {
        let coeffs = vec![
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(5u64),
            Scalar::from(7u64),
        ];

        let xs = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
            Scalar::from(5u64),
        ];

        let blindings = vec![
            Scalar::from(11u64),
            Scalar::from(12u64),
            Scalar::from(13u64),
            Scalar::from(14u64),
            Scalar::from(15u64),
        ];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);
        assert!(verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_if_x_points_are_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove(&statement, &witness);

        statement.x_points[1] = Scalar::from(3u64);

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_on_wrong_number_of_blinding_responses() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let mut proof = prove(&statement, &witness);
        proof.z_blindings.pop();

        assert!(!verify(&statement, &proof));
    }

    #[test]
    fn test_verify_fails_on_empty_coeff_responses() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let mut proof = prove(&statement, &witness);
        proof.z_coeffs.clear();

        assert!(!verify(&statement, &proof));
    }
}
