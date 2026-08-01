# Janus – Adaptive DKG Prototype

[![CI](https://github.com/pGerhart/Janus/actions/workflows/ci.yml/badge.svg)](https://github.com/pGerhart/Janus/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](Cargo.toml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

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
| `src/main.rs` | Runs both protocols across all proof systems |
| `scripts/` | Benchmark runner and the results-file generator |

Tests live in `tests/`, split by purpose:

| File | Covers |
|---|---|
| `dkg_full_run.rs` | End-to-end runs for every combination of protocol variant and proof system |
| `one_round_proofs.rs` | Well-formedness proofs, including forgeries that commitments off a low-degree polynomial must not pass |
| `two_round_proofs.rs` | Decomposition, equality, and public key proofs |
| `abort.rs` | A malicious dealer is convicted, while a false complaint and a malformed proof fall back on the reporter |
| `wire_path.rs` | The channel path agrees with the in-memory one, and re-labelled, tampered, or truncated messages are rejected |
| `parallel_output.rs` | The parallel output matches the sequential one |

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

The repository includes [Criterion](https://github.com/bheisler/criterion.rs) benchmarks across six suites:

| Suite | File | What is measured |
|---|---|---|
| Janus-1 DKG | `benches/one_round_dkg_run.rs` | Initiation (proof generation) and output (verification + share aggregation) per party |
| Janus-1 components | `benches/one_round_components.rs` | Individual operations (proving, encryption, decryption, VSS checks, message authentication and decoding) in isolation at Fischlin small with $(t=32, n=64)$ |
| Janus-2 DKG | `benches/two_round_dkg_run.rs` | Round 1 (initiate), round 2 (finalize), and output per party |
| Janus-2 components | `benches/two_round_components.rs` | Individual operations in isolation at Fischlin small with $(t=32, n=64)$ |
| Identifiable abort | `benches/abort_path.rs` | Building and verifying a complaint, and the worst case where every other party complains |
| Optimizations | `benches/optimizations.rs` | Batch proof verification and a multi-threaded output phase at large committees |

All DKG benchmarks run over seven parameter sets: **(t=4, n=16)**, **(t=8, n=32)**, **(t=16, n=64)**, **(t=32, n=64)**, **(t=64, n=128)**, **(t=128, n=256)**, **(t=256, n=512)** at 128-bit security on Ristretto (Curve25519). The large Fischlin profile is benchmarked up to $n < 256$; Schnorr and the small Fischlin profile cover the full range.

```
cargo bench
```

The pinned results, together with the machine they were measured on, are in [`benches/bench_results.md`](benches/bench_results.md).

# Reproducing the Published Numbers

The paper's numbers come from a single run on a dedicated cloud machine, so that no other load interferes and the parameter sets up to $(t=256, n=512)$ have enough memory. The procedure below is the whole of it.

**Machine.** An AWS `c7i.4xlarge` (16 vCPU, 32 GB, x86-64). Three properties matter:

- `curve25519-dalek` selects its arithmetic backend in its build script from the target architecture, and detects the CPU features at run time. On x86-64 with a Rust toolchain of 1.89 or newer it uses the AVX-512 backend, on 64-bit ARM the serial one. The architecture therefore changes the results.
- Burstable instance families such as `t3` and `t4g` throttle once their CPU credits run out, which corrupts a run of this length. Use a fixed-performance family.
- The largest parameter set holds all $n$ broadcasts decompressed in memory, which needs well over 16 GB at $(t=256, n=512)$.


**Run.** The full benchmarks takes over an hour, and the largest points take several minutes each.
All benchmarks can be run with the script

```
./scripts/run_bench.sh
```

`scripts/run_bench.sh` first records the machine, OS, toolchain, architecture, core count and date, then runs the whole gate battery (build, test, fmt, clippy) and stops on any failure. It then runs all six suites into `bench_raw/`.

**Collect.**
We built a script that writes the raw benches into `benches/bench_results.md`.
```
python3 scripts/make_results.py
```
