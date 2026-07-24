use super::*;
use bulletproofs::LinearProof;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[derive(Clone, Debug)]
pub struct SerializableLinearProof(pub LinearProof);

impl Serialize for SerializableLinearProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for SerializableLinearProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;

        let proof = LinearProof::from_bytes(&bytes).map_err(serde::de::Error::custom)?;

        Ok(SerializableLinearProof(proof))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWellFormedBulletproof {
    pub degree: usize,

    pub coeff_cf: RistrettoPoint,
    pub coeff_output: RistrettoPoint,
    pub coeff_proof: SerializableLinearProof,

    pub blind_cf: RistrettoPoint,
    pub blind_output: RistrettoPoint,
    pub blind_proof: SerializableLinearProof,
}

#[derive(Clone, Debug)]
pub struct PolyWellFormedBulletproofParams {
    pub coeff_m: usize,
    pub coeff_g_vec: Vec<RistrettoPoint>,

    pub blind_m: usize,
    pub blind_g_vec: Vec<RistrettoPoint>,
}

pub fn prove_bulletproof(
    statement: &PolyWellFormedStatement,
    witness: &PolyWellFormedWitness,
    pp: &PolyWellFormedBulletproofParams,
) -> PolyWellFormedBulletproof {
    let n = statement.x_points.len();
    assert_eq!(
        statement.commitments.len(),
        n,
        "x_points and commitments mismatch"
    );
    assert_eq!(witness.blindings.len(), n, "blindings length mismatch");
    assert!(!witness.coeffs.is_empty(), "coeffs must not be empty");

    let degree = witness.coeffs.len() - 1;

    assert!(pp.coeff_m >= witness.coeffs.len(), "coeff_m too small");
    assert!(pp.coeff_m.is_power_of_two(), "coeff_m must be power of two");
    assert_eq!(
        pp.coeff_g_vec.len(),
        pp.coeff_m,
        "coeff generator length mismatch"
    );

    assert!(pp.blind_m >= witness.blindings.len(), "blind_m too small");
    assert!(pp.blind_m.is_power_of_two(), "blind_m must be power of two");
    assert_eq!(
        pp.blind_g_vec.len(),
        pp.blind_m,
        "blind generator length mismatch"
    );

    let mut transcript = Transcript::new(b"poly-well-formedness-bp");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, degree);
    let lambda_tail: Vec<Scalar> = lambdas.iter().skip(1).copied().collect();

    let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

    // coeff proof
    let coeff_x = pad_vec(&witness.coeffs, pp.coeff_m);
    let coeff_b = pad_vec(&mus, pp.coeff_m);

    let coeff_y = witness
        .coeffs
        .iter()
        .zip(mus.iter())
        .fold(Scalar::ZERO, |acc, (a, mu)| acc + (*a * *mu));

    let coeff_output = g() * coeff_y;

    let coeff_r = Scalar::random(&mut OsRng);
    let coeff_cf = pp
        .coeff_g_vec
        .iter()
        .zip(coeff_x.iter())
        .fold(h() * coeff_r, |acc, (gi, xi)| acc + (*gi * *xi));

    let coeff_c = (coeff_cf + coeff_output).compress();

    let mut coeff_transcript = Transcript::new(b"poly-well-formedness-bp-coeff");
    let coeff_proof = LinearProof::create(
        &mut coeff_transcript,
        &mut OsRng,
        &coeff_c,
        coeff_r,
        coeff_x,
        coeff_b,
        pp.coeff_g_vec.clone(),
        &g(),
        &h(),
    )
    .expect("LinearProof::create coeff failed");

    // blinding proof
    let blind_x = pad_vec(&witness.blindings, pp.blind_m);
    let blind_b = pad_vec(&lambda_tail, pp.blind_m);

    let blind_y = witness
        .blindings
        .iter()
        .zip(lambda_tail.iter())
        .fold(Scalar::ZERO, |acc, (rho, lambda)| acc + (*rho * *lambda));

    let blind_output = h() * blind_y;

    let blind_r = Scalar::random(&mut OsRng);
    let blind_cf = pp
        .blind_g_vec
        .iter()
        .zip(blind_x.iter())
        .fold(g() * blind_r, |acc, (gi, xi)| acc + (*gi * *xi));

    let blind_c = (blind_cf + blind_output).compress();

    let mut blind_transcript = Transcript::new(b"poly-well-formedness-bp-blind");
    let blind_proof = LinearProof::create(
        &mut blind_transcript,
        &mut OsRng,
        &blind_c,
        blind_r,
        blind_x,
        blind_b,
        pp.blind_g_vec.clone(),
        &h(),
        &g(),
    )
    .expect("LinearProof::create blind failed");

    debug_assert_eq!(coeff_output + blind_output, p_star);

    PolyWellFormedBulletproof {
        degree,
        coeff_cf,
        coeff_output,
        coeff_proof: SerializableLinearProof(coeff_proof),
        blind_cf,
        blind_output,
        blind_proof: SerializableLinearProof(blind_proof),
    }
}

pub fn verify_bulletproof(
    statement: &PolyWellFormedStatement,
    proof: &PolyWellFormedBulletproof,
    pp: &PolyWellFormedBulletproofParams,
) -> bool {
    verify_bulletproof_debug(statement, proof, pp).is_ok()
}

pub fn verify_bulletproof_debug(
    statement: &PolyWellFormedStatement,
    proof: &PolyWellFormedBulletproof,
    pp: &PolyWellFormedBulletproofParams,
) -> Result<(), String> {
    let n = statement.x_points.len();

    if statement.commitments.len() != n {
        return Err("commitment length mismatch".into());
    }
    if pp.coeff_g_vec.len() != pp.coeff_m || !pp.coeff_m.is_power_of_two() {
        return Err("bad coeff params".into());
    }
    if pp.blind_g_vec.len() != pp.blind_m || !pp.blind_m.is_power_of_two() {
        return Err("bad blind params".into());
    }
    if proof.degree + 1 > pp.coeff_m {
        return Err("degree too large".into());
    }
    if n > pp.blind_m {
        return Err("too many blindings".into());
    }

    let mut transcript = Transcript::new(b"poly-well-formedness-bp");
    append_statement_to_transcript(&mut transcript, statement);

    let lambdas = derive_lambdas(&mut transcript, n);
    let mus = compute_mus(&statement.x_points, &lambdas, proof.degree);
    let lambda_tail: Vec<Scalar> = lambdas.iter().skip(1).copied().collect();

    let p_star = combine_publics(&statement.f0_commitment, &statement.commitments, &lambdas);

    if proof.coeff_output + proof.blind_output != p_star {
        return Err("public coupling failed".into());
    }

    let coeff_b = pad_vec(&mus, pp.coeff_m);
    let coeff_c = (proof.coeff_cf + proof.coeff_output).compress();
    let mut coeff_transcript = Transcript::new(b"poly-well-formedness-bp-coeff");
    proof
        .coeff_proof
        .0
        .verify(
            &mut coeff_transcript,
            &coeff_c,
            &pp.coeff_g_vec,
            &g(),
            &h(),
            coeff_b,
        )
        .map_err(|e| format!("coeff proof failed: {e:?}"))?;

    let blind_b = pad_vec(&lambda_tail, pp.blind_m);
    let blind_c = (proof.blind_cf + proof.blind_output).compress();
    let mut blind_transcript = Transcript::new(b"poly-well-formedness-bp-blind");
    proof
        .blind_proof
        .0
        .verify(
            &mut blind_transcript,
            &blind_c,
            &pp.blind_g_vec,
            &h(),
            &g(),
            blind_b,
        )
        .map_err(|e| format!("blind proof failed: {e:?}"))?;

    Ok(())
}

fn pad_vec(xs: &[Scalar], m: usize) -> Vec<Scalar> {
    let mut out = Vec::with_capacity(m);
    out.extend_from_slice(xs);
    if out.len() < m {
        out.resize(m, Scalar::ZERO);
    }
    out
}

pub fn make_bulletproof_params(
    coeff_len: usize,
    blind_len: usize,
) -> PolyWellFormedBulletproofParams {
    let coeff_m = coeff_len.next_power_of_two();
    let blind_m = blind_len.next_power_of_two();

    let mut coeff_g_vec = Vec::with_capacity(coeff_m);
    for i in 0..coeff_m {
        let mut t = Transcript::new(b"poly-bp-coeff-generators");
        t.append_u64(b"i", i as u64);
        coeff_g_vec.push(t.challenge_point(b"G"));
    }

    let mut blind_g_vec = Vec::with_capacity(blind_m);
    for i in 0..blind_m {
        let mut t = Transcript::new(b"poly-bp-blind-generators");
        t.append_u64(b"i", i as u64);
        blind_g_vec.push(t.challenge_point(b"G"));
    }

    PolyWellFormedBulletproofParams {
        coeff_m,
        coeff_g_vec,
        blind_m,
        blind_g_vec,
    }
}

#[derive(Clone, Debug)]
pub struct BulletproofPolyProof;

impl crate::one_round_proofs::PolyProofScheme for BulletproofPolyProof {
    type Proof = PolyWellFormedBulletproof;
    type Params = PolyWellFormedBulletproofParams;

    fn prove(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof {
        prove_bulletproof(statement, witness, params)
    }

    fn verify(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool {
        verify_bulletproof(statement, proof, params)
    }
}

pub trait PolyProofScheme {
    type Proof: Clone + std::fmt::Debug;
    type Params: Clone + std::fmt::Debug;

    fn prove(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        witness: &PolyWellFormedWitness,
    ) -> Self::Proof;

    fn verify(
        params: &Self::Params,
        statement: &PolyWellFormedStatement,
        proof: &Self::Proof,
    ) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pedersen::PedersenCommitment;
    use crate::poly::eval_poly_at;

    fn make_instance(
        degree: usize,
        n: usize,
    ) -> (
        PolyWellFormedStatement,
        PolyWellFormedWitness,
        PolyWellFormedBulletproofParams,
    ) {
        let mut rng = OsRng;

        let coeffs: Vec<Scalar> = (0..=degree).map(|_| Scalar::random(&mut rng)).collect();
        let blindings: Vec<Scalar> = (0..n).map(|_| Scalar::random(&mut rng)).collect();
        let x_points: Vec<Scalar> = (1..=n).map(|i| Scalar::from(i as u64)).collect();

        let commitments: Vec<PedersenCommitment> = x_points
            .iter()
            .zip(blindings.iter())
            .map(|(x, rho)| {
                let fx = eval_poly_at(&coeffs, *x);
                PedersenCommitment::new(fx, *rho)
            })
            .collect();

        let statement = PolyWellFormedStatement {
            x_points,
            commitments,
            f0_commitment: g() * coeffs[0],
        };

        let witness = PolyWellFormedWitness { coeffs, blindings };
        let pp = make_bulletproof_params(degree + 1, n);

        (statement, witness, pp)
    }

    #[test]
    fn bulletproof_roundtrip_verifies() {
        let (statement, witness, pp) = make_instance(3, 5);

        let proof = prove_bulletproof(&statement, &witness, &pp);

        match verify_bulletproof_debug(&statement, &proof, &pp) {
            Ok(()) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
