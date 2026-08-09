# Janus – Adaptive DKG Prototype

[![CI](https://github.com/pGerhart/Janus/actions/workflows/ci.yml/badge.svg)](https://github.com/pGerhart/Janus/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.89-blue.svg)](Cargo.toml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

This is a research prototype of **Janus**, a family of distributed key generation (DKG) protocols for discrete-logarithm-based threshold cryptosystems, based on the work:

> **Adaptive Distributed Key Generation for Discrete-Log Cryptosystems**  
> Ruben Baecker, Paul Gerhart, Stanislaw Jarecki, Phillip Nazarian, Daniel Rausch, and Dominique Schröder

Janus achieves full adaptive security in the dishonest-majority setting ($t_c < t \leq n$) and provides identifiable aborts and proactive key refresh. The construction is designed to serve as the foundational setup layer for DLog-based threshold signing schemes such as FROST and threshold BLS.

> [!CAUTION]
> **Research prototype, not for production.**
> This implementation is a proof of concept. It has **not** undergone any security
> audit, it is **not** hardened against side-channel attacks, and it **must not** be
> used in production. Run it only in controlled research or testing settings.

# Protocols

Janus comprises two complementary protocols.

**Janus-1** is a one-round shifted DKG. Each party broadcasts a single message containing a polynomial commitment, per-party encrypted shares, and a proof of polynomial well-formedness. It is round-optimal when a bounded shift on the output key is tolerable.

**Janus-2** is a two-round unshifted DKG. In round 1, each party additionally commits to a blinding polynomial and proves that both polynomials are correctly structured. In round 2, parties aggregate their received shares and prove consistency of their partial verification key with the round-1 commitments. This two-round structure eliminates the shift a rushing adversary can inject into single-round setups and is round-optimal among protocols producing an unshifted public key in the dishonest-majority setting.

Both protocols produce a joint public key $\mathsf{pk} = g^{\mathsf{sk}}$ and per-party verification keys of the form $\mathsf{vk}_i = g^{\mathsf{sk}_i} \cdot h^{\omega_i}$. These are perfectly hiding Pedersen commitments to the share, not unique commitments. This structural choice is what permits adaptive security proofs to proceed without committing to share values before the adversary has decided which parties to corrupt.

# Proof Systems

The NIZKs for polynomial well-formedness (Janus-1) and decomposition consistency (Janus-2) are instantiated with two proof systems, one for each security notion:

| Proof system | Security | Proof size | Prover cost | Verifier cost |
|---|---|---|---|---|
| Chaum–Pedersen / Schnorr | Simulation-secure (rewinding) | Linear | Low | Low |
| Fischlin | UC-secure (straight-line extractable) | Linear (configurable constant) | Configurable | Configurable |

The Fischlin instantiation achieves straight-line extractability, which is required for the UC realization. The Schnorr instantiation offers better concrete efficiency under the weaker simulation-based security notion.

The Fischlin transform is parameterized by the number of repetitions $\rho$, the score difficulty $b$, and the challenge space size $t_{\mathit{bits}}$. Two operating points are evaluated, both satisfying $\rho \cdot b \approx 128$ for 128-bit security:

| Profile | ρ | b | t_bits | Character |
|---|---|---|---|---|
| Small proof | 16 | 8 | 13 | Smaller proof, high prover work, low verifier work |
| Large proof | 43 | 3 | 8 | Larger proof, low prover work, high verifier work |

# Repository Structure

| Path | Description |
|---|---|
| `src/one_round.rs` | Janus-1: initiation, output, and a parallel output variant |
| `src/two_round.rs` | Janus-2: round 1, round 2, and output |
| `src/abort.rs` | Identifiable abort: signed complaint and its verdict, shared by both variants |
| `src/one_round_proofs/` | Well-formedness proofs (Schnorr, Fischlin) |
| `src/two_round_proofs/` | Decomposition and consistency proofs (Schnorr, Fischlin) |
| `src/encryption/` | Hashed ElGamal encryption and decryption proofs |
| `src/primitives/` | Group generators, Pedersen commitments, polynomials, transcripts |
| `src/party.rs` | Party state and EdDSA channel keys |
| `src/error.rs` | Protocol and wire error types |
| `src/wire.rs` | Canonical byte encodings and the signed-message envelope |
| `src/main.rs` | Runs both protocols under both proof systems |
| `eval/` | Benchmark sources, the runner, the generated results file, and `compact-encoding`, an alternative proof encoding kept for the measurement in that file |

Tests live in `tests/`, split by purpose:

| File | Covers |
|---|---|
| `dkg_full_run.rs` | End-to-end runs for every combination of protocol variant and proof system |
| `one_round_proofs.rs` | Well-formedness proofs, including forgeries that commitments off a low-degree polynomial must not pass |
| `two_round_proofs.rs` | Decomposition, equality, and public key proofs |
| `error_paths.rs` | Every error variant of both protocols, each asserting that the party it names is the one that misbehaved |
| `abort.rs` | A malicious dealer is convicted, while a false complaint and a malformed proof fall back on the reporter |
| `wire_path.rs` | The channel path agrees with the in-memory one, and re-labelled, tampered, or truncated messages are rejected |
| `key_refresh.rs` | Contributing zero re-randomizes every share and leaves the public key untouched |
| `parallel_output.rs` | The parallel output matches the sequential one |

# Usage

The shared value each party contributes is an argument of the initiate procedure. Passing a fresh secret generates a key, passing zero refreshes an existing one, since a sharing of zero re-randomizes the shares and leaves the public key where it was. `tests/key_refresh.rs` pins that property.

**Run the protocol** (both variants, both proof systems, parameters defined in `main.rs`):

```
cargo run --release
```

**Run the tests:**

```
cargo test
```

The tests exercise full protocol runs for both variants and verify that all parties produce consistent public keys, matching partial verification keys, and share openings that validate against those keys.

# Benchmarks

The repository includes [Criterion](https://github.com/bheisler/criterion.rs) benchmarks across seven suites:

| Suite | File | What is measured |
|---|---|---|
| Janus-1 DKG | `eval/benches/one_round_dkg_run.rs` | Initiation (proof generation) and output (verification + share aggregation) per party |
| Janus-1 components | `eval/benches/one_round_components.rs` | Individual operations (proving, encryption, decryption, VSS checks, message authentication and decoding) in isolation at Fischlin small with $(t=32, n=64)$ |
| Janus-2 DKG | `eval/benches/two_round_dkg_run.rs` | Round 1 (initiate), round 2 (finalize), and output per party |
| Janus-2 components | `eval/benches/two_round_components.rs` | Individual operations in isolation at Fischlin small with $(t=32, n=64)$ |
| Identifiable abort | `eval/benches/abort_path.rs` | Building and verifying a complaint, and the worst case where every other party complains |
| Optimizations | `eval/benches/optimizations.rs` | Batch proof verification and a multi-threaded output phase at large committees |
| End-to-end run | `eval/benches/full_run.rs` | One party's whole run with messages encoded and decoded as they are on a channel, plus the bytes it sends and receives |

All DKG benchmarks run over seven parameter sets: **(t=4, n=16)**, **(t=8, n=32)**, **(t=16, n=64)**, **(t=32, n=64)**, **(t=64, n=128)**, **(t=128, n=256)**, **(t=256, n=512)** at 128-bit security on Ristretto (Curve25519). The large Fischlin profile is benchmarked up to $n < 256$; Schnorr and the small Fischlin profile cover the full range.

```
cargo bench
```

The pinned results, together with the machine they were measured on, are in [`eval/eval_results.md`](eval/eval_results.md).

# Reproducing the Published Numbers

The paper's numbers were measured on an AWS `c8i.4xlarge` (16 vCPU on 8 physical cores, 30 GB, x86-64, Granite Rapids). 
The full benchmarks take roughly two and a half hours, and the largest points take several minutes each. 
All benchmarks can be run with the script

```
./eval/scripts/run_bench.sh
```

This script runs all benchmarks. It records the machine, OS, toolchain, architecture, core count and date, runs the gate battery (build, test, fmt, clippy) over the workspace and stops on any failure, runs every benchmark suite plus the encoding comparison into `eval/bench_raw/`, and finally combines those raw logs into `eval/eval_results.md`.