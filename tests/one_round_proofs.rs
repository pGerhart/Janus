// Well-formedness proofs of the one-round DKG: happy path, negative paths, and
// the forgeries the aggregate-equation proof used to accept.

use curve25519_dalek::scalar::Scalar;
use janus::group::{g, h};
use janus::one_round_proofs::polyproof_fischlin::*;
use janus::one_round_proofs::polyproof_schnorr::*;
use janus::pedersen::PedersenCommitment;
use janus::poly::eval_poly_at;

mod polyproof_schnorr {
    use super::*;
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

        let degree = coeffs.len() - 1;
        let statement = PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g() * coeffs[0],
            degree,
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
        proof.t_commitments[0] += h() * Scalar::ONE;

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

    // A malicious dealer commits to evaluations that lie on no degree-`degree`
    // polynomial and claims degree `degree`. No degree-`degree` witness opens all
    // commitments, so every proof it can build is rejected. This is the case the
    // old aggregate-equation proof accepted.
    #[test]
    fn forgery_non_low_degree_commitments_rejected() {
        let xs: Vec<Scalar> = (1..=5).map(|i| Scalar::from(i as u64)).collect();
        // Arbitrary values, not the evaluations of any degree-2 polynomial.
        let vals: Vec<Scalar> = [1u64, 2, 9, 1, 7]
            .iter()
            .map(|v| Scalar::from(*v))
            .collect();
        let blindings: Vec<Scalar> = (0..5).map(|i| Scalar::from(100 + i as u64)).collect();
        let commitments: Vec<PedersenCommitment> = vals
            .iter()
            .zip(blindings.iter())
            .map(|(v, b)| PedersenCommitment::new(*v, *b))
            .collect();
        let a0 = Scalar::from(3u64);
        let statement = PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g() * a0,
            degree: 2,
        };

        // The best the dealer can do: any degree-2 witness with the matching blindings.
        let witness = PolyWellFormedWitness {
            coeffs: vec![a0, Scalar::from(5u64), Scalar::from(7u64)],
            blindings,
        };
        let proof = prove(&statement, &witness);
        assert!(
            !verify(&statement, &proof),
            "non-low-degree commitments must be rejected"
        );
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

mod polyproof_fischlin {
    use super::*;
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
            degree: coeffs.len() - 1,
        };

        let witness = PolyWellFormedWitness { coeffs, blindings };

        (statement, witness)
    }

    #[test]
    fn test_fischlin_proof_valid_custom_params() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let blindings = vec![Scalar::from(7u64), Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    // Commitments that lie on no degree-2 polynomial must be rejected regardless
    // of the degree-2 witness the dealer picks.
    #[test]
    fn forgery_non_low_degree_commitments_rejected() {
        let xs: Vec<Scalar> = (1..=5).map(|i| Scalar::from(i as u64)).collect();
        let vals: Vec<Scalar> = [1u64, 2, 9, 1, 7]
            .iter()
            .map(|v| Scalar::from(*v))
            .collect();
        let blindings: Vec<Scalar> = (0..5).map(|i| Scalar::from(100 + i as u64)).collect();
        let commitments: Vec<PedersenCommitment> = vals
            .iter()
            .zip(blindings.iter())
            .map(|(v, b)| PedersenCommitment::new(*v, *b))
            .collect();
        let a0 = Scalar::from(3u64);
        let statement = PolyWellFormedStatement {
            x_points: xs,
            commitments,
            f0_commitment: g() * a0,
            degree: 2,
        };
        let witness = PolyWellFormedWitness {
            coeffs: vec![a0, Scalar::from(5u64), Scalar::from(7u64)],
            blindings,
        };
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    // Rank-1 folding shares one delta vector across rounds, so check that a single
    // corrupted equation in an interior round is still caught.
    #[test]
    fn rank1_fold_rejects_single_middle_round_corruption() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let blindings = vec![Scalar::from(7u64), Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);

        // Corrupt one coefficient response in a middle round.
        let mut p1 = prove_fischlin_with_params(&statement, &witness, 5, 4, 9);
        p1.rounds[2].z_coeffs[1] += Scalar::ONE;
        assert!(!verify_fischlin_with_params(&statement, &p1, 5, 4, 9));

        // Corrupt one blinding response in a different middle round.
        let mut p2 = prove_fischlin_with_params(&statement, &witness, 5, 4, 9);
        p2.rounds[3].z_blindings[2] += Scalar::ONE;
        assert!(!verify_fischlin_with_params(&statement, &p2, 5, 4, 9));
    }

    #[test]
    fn test_batch_verify_valid_single() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        assert!(batch_verify_fischlin_with_params(
            &[statement],
            &[proof],
            4,
            4,
            9
        ));
    }

    #[test]
    fn test_batch_verify_valid_many() {
        let mut statements = Vec::new();
        let mut proofs = Vec::new();

        for k in 0..4u64 {
            let coeffs = vec![
                Scalar::from(2 + k),
                Scalar::from(3 + k),
                Scalar::from(5 + k),
            ];
            let xs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
            let blindings = vec![
                Scalar::from(7 + k),
                Scalar::from(11 + k),
                Scalar::from(13 + k),
            ];

            let (statement, witness) = build_instance(coeffs, xs, blindings);
            let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

            statements.push(statement);
            proofs.push(proof);
        }

        assert!(batch_verify_fischlin_with_params(
            &statements,
            &proofs,
            4,
            4,
            9
        ));
    }

    #[test]
    fn test_fischlin_verify_fails_if_commitment_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        let modified = *statement.commitments[1].point() + g() * Scalar::ONE;
        statement.commitments[1] = PedersenCommitment::from_point(modified);

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_f0_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        statement.f0_commitment += g() * Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_x_points_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (mut statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        statement.x_points[1] = Scalar::from(9u64);

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_t_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].t_commitments[0] += h() * Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_e_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].e ^= 1;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_z_coeff_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_coeffs[1] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_z_blinding_modified() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_blindings[0] += Scalar::ONE;

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_round_removed() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_wrong_blinding_length() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_blindings.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_wrong_coeff_length() {
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64), Scalar::from(7u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(11u64), Scalar::from(13u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let mut proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);

        proof.rounds[0].z_coeffs.pop();

        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_if_invalid_params() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64)];
        let xs = vec![Scalar::from(1u64)];
        let blindings = vec![Scalar::from(5u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 2, 4, 9);

        assert!(!verify_fischlin_with_params(&statement, &proof, 0, 4, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 0, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 0));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 10, 9));
        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 16));
    }

    #[test]
    fn test_fischlin_verify_fails_if_invalid_round_count() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64)];
        let xs = vec![Scalar::from(1u64)];
        let blindings = vec![Scalar::from(5u64)];

        let (statement, witness) = build_instance(coeffs, xs, blindings);
        let proof = prove_fischlin_with_params(&statement, &witness, 3, 4, 9);

        assert!(!verify_fischlin_with_params(&statement, &proof, 2, 4, 9));
    }

    #[test]
    fn test_fischlin_verify_fails_for_wrong_witness() {
        let coeffs = vec![Scalar::from(2u64), Scalar::from(3u64), Scalar::from(5u64)];
        let xs = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let blindings = vec![Scalar::from(7u64), Scalar::from(11u64)];

        let (statement, mut witness) = build_instance(coeffs, xs, blindings);
        witness.coeffs[1] += Scalar::ONE;

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(!verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_constant_polynomial() {
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

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }

    #[test]
    fn test_fischlin_many_evaluation_points() {
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

        let proof = prove_fischlin_with_params(&statement, &witness, 4, 4, 9);
        assert!(verify_fischlin_with_params(&statement, &proof, 4, 4, 9));
    }
}
