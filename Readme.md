# Janus – Adaptive DKG Prototype

This is a research prototype of **Janus**, a family of distributed key generation (DKG) protocols for discrete-logarithm-based threshold cryptosystems, based on the work:

> **Adaptive Distributed Key Generation for Discrete-Log Cryptosystems**
> Ruben Baecker, Paul Gerhart, Stanislaw Jarecki, Phillip Nazarian, Daniel Rausch, and Dominique Schröder

Janus achieves full adaptive security in the dishonest-majority setting ($t_c < t \leq n$) and provides identifiable aborts and proactive key refresh. The construction is designed to serve as the foundational setup layer for DLog-based threshold signing schemes such as FROST and threshold BLS.

⚠️ **Prototype Warning**
This implementation is **only a proof-of-concept**. It has **not** undergone any security audits, is **not** hardened against side-channel attacks, and **must not** be used in production environments. Run it only in controlled, research, or testing settings.

# Protocols

Janus comprises two complementary protocols.

**Janus-1** is a one-round shifted DKG. Each party broadcasts a single message containing a polynomial commitment, per-party encrypted shares, and a proof of polynomial well-formedness. It is round-optimal when a bounded shift on the output key is tolerable.

**Janus-2** is a two-round unshifted DKG. In round 1, each party additionally commits to a blinding polynomial and proves that both polynomials are correctly structured. In round 2, parties aggregate their received shares and prove consistency of their partial verification key with the round-1 commitments. This two-round structure eliminates the shift a rushing adversary can inject into single-round setups and is round-optimal among protocols producing an unshifted public key in the dishonest-majority setting.

Both protocols produce a joint public key $\mathsf{pk} = g^{\mathsf{sk}}$ and per-party verification keys of the form $\mathsf{vk}_i = g^{\mathsf{sk}_i} \cdot h^{\omega_i}$. These are perfectly hiding Pedersen commitments to the share, not unique commitments. This structural choice is what permits adaptive security proofs to proceed without committing to share values before the adversary has decided which parties to corrupt.

# Proof Systems

The NIZKs for polynomial well-formedness (Janus-1) and decomposition consistency (Janus-2) can be instantiated with multiple proof systems:

| Proof system | Security | Proof size | Prover cost | Verifier cost |
|---|---|---|---|---|
| Chaum–Pedersen / Schnorr | Simulation-secure (rewinding) | Linear | Low | Low |
| Bulletproof | Simulation-secure (rewinding) | Logarithmic | Medium | Medium |
| Fischlin | UC-secure (straight-line extractable) | Linear (configurable constant) | Configurable | Configurable |

The Fischlin instantiation achieves straight-line extractability, which is required for the UC realization. The Schnorr and Bulletproof instantiations offer better concrete efficiency under the weaker simulation-based security notion. Bulletproofs are only available for Janus-1.

The Fischlin transform is parameterized by the number of repetitions $\rho$, the score difficulty $b$, and the challenge space size $t_{\mathit{bits}}$. Two operating points are evaluated, both satisfying $\rho \cdot b \approx 128$ for 128-bit security:

| Profile | ρ | b | t_bits | Character |
|---|---|---|---|---|
| Small proof | 16 | 8 | 13 | Smaller proof, high prover work, low verifier work |
| Large proof | 43 | 3 | 8 | Larger proof, low prover work, high verifier work |

# Repository Structure

| Path | Description |
|---|---|
| `src/one_round.rs` | Janus-1: initiation and output |
| `src/two_round.rs` | Janus-2: round 1, round 2, and output |
| `src/one_round_proofs/` | Well-formedness proofs (Schnorr, Fischlin, Bulletproof) |
| `src/two_round_proofs/` | Decomposition and consistency proofs (Schnorr, Fischlin) |
| `src/encryption/` | Hashed ElGamal encryption and decryption proofs |
| `src/pedersen.rs` | Pedersen commitments over Ristretto |
| `src/poly.rs` | Polynomial evaluation |
| `src/party.rs` | Party state and EdDSA channel keys |
| `src/group.rs` | Group generators `g` and `h` |
| `src/main.rs` | Runs both protocols across all proof systems |

Integration tests covering end-to-end protocol runs for all combinations of protocol variant and proof system are in `tests/dkg_full_run.rs`.

# Usage

**Run the protocol** (both variants, all proof systems, parameters defined in `main.rs`):

```
cargo run --release
```

**Run the tests:**

```
cargo test
```

The tests exercise full protocol runs for both variants and verify that all parties produce consistent public keys, matching partial verification keys, and share openings that validate against those keys.

# Benchmarks

The repository includes [Criterion](https://github.com/bheisler/criterion.rs) benchmarks across four suites:

| Suite | File | Results | What is measured |
|---|---|---|---|
| Janus-1 DKG | `benches/one_round_dkg_run.rs` | [results](benchmark_outputs/one_round_dkg_run.txt) | Initiation (proof generation) and output (verification + share aggregation) per party |
| Janus-1 components | `benches/one_round_components.rs` | [results](benchmark_outputs/one_round_components.txt) | Individual operations (proving, encryption, decryption, VSS checks) in isolation at Fischlin small with $(t=32, n=64)$ |
| Janus-2 DKG | `benches/two_round_dkg_run.rs` | [results](benchmark_outputs/two_round_dkg_run.txt) | Round 1 (initiate), round 2 (finalize), and output per party |
| Janus-2 components | `benches/two_round_components.rs` | [results](benchmark_outputs/two_round_components.txt) | Individual operations in isolation at Fischlin small with $(t=32, n=64)$ |

All DKG benchmarks run over four parameter sets: **(t=4, n=16)**, **(t=8, n=32)**, **(t=16, n=64)**, **(t=32, n=64)** at 128-bit security on Ristretto (Curve25519).

```
cargo bench
```

Benchmark results measured on an Apple M3 Pro MacBook Pro with 36 GB RAM are stored in `benchmark_outputs/`.
