# Janus benchmark results

Median runtimes per party, measured with `cargo bench` (Criterion). Regenerate with `eval/scripts/run_bench.sh` followed by `eval/scripts/build_results.py`.

| | |
|---|---|
| Machine | ? |
| Cores | ? |
| Memory | ? |
| Architecture | ? |
| OS | ? |
| Rust | ? |
| RUSTFLAGS | `<unset>` |
| Date | ? |

The curve backend row is what `curve25519-dalek` actually compiled, read from its build script rather than inferred from the CPU. `avx512` is the IFMA path, `simd` is AVX2, and `serial` is the portable fallback, so that row states which arithmetic these numbers measure.

The parallel rows are wall-clock times of the same work spread over all cores, so read their speedup against the physical core count above, not the logical one.

`(t, n)` = (threshold, parties). Initiate and output are the phases each party runs; output verifies the other `n - 1` proofs, so it grows quadratically in the committee size. The abort rows run only when a dealer sends a share that does not open its commitment, so they are off the honest path.

## Janus-1 (one round)

## Janus-2 (two rounds)

## Identifiable abort

## Parallel output and batching

## Component breakdown, Fischlin small, (t=32, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | -- | -- |
| Share encryption | -- | -- |
| Proof verification, one | -- | -- |
| Proof verification, all | -- | -- |
| Share decryption | -- | -- |
| Opening checks | -- | -- |
| Key aggregation | -- | -- |
| Message authentication | -- | -- |
| Message decoding | -- | -- |

> Message authentication checks the signature over the bytes as received. Message decoding is the one-time cost of parsing those bytes into group elements, which dominates because every point needs a decompression.

## End-to-end run

One party's whole run, from building its own message to holding the output key. Every message is encoded on the way out and decoded on the way in, which the phase tables above skip.

Compute is measured, the link columns are attributed as `max(compute, transfer) + rounds * RTT`. A party reaches that bound by verifying each message as it arrives; one that waits for the whole round pays the sum instead. Broadcast is counted as point-to-point fan-out on a full-duplex link, so a party uploads once per peer.

The rounds include one echo round on top of the protocol, since the protocol rounds only disseminate: every party sends a digest of what it received, which catches a dealer that told two parties different things. That gives broadcast with abort, which suits a protocol that already identifies the party at fault. Naming the culprit needs per-dealer hashes and is counted with the abort path.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 9.3 ms | 43.1 KB | 11.3 ms | 59.3 ms | 309.3 ms |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 54.8 ms | 311.3 KB | 56.8 ms | 104.8 ms | 354.8 ms |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 13.7 ms | 33.4 KB | 16.7 ms | 88.7 ms | 463.7 ms |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 28.3 ms | 125.6 KB | 31.3 ms | 103.3 ms | 478.3 ms |

### What the echo round costs

The tables above include it. It is the same for both protocols and both proof systems, one round-trip plus a digest to every peer, so subtract a row here to read a protocol without agreement.

| n | Extra bytes | One region | Cross-region | Intercontinental |
|:---|---:|---:|---:|---:|
| 16 | 480 B | 1.0 ms | 25.0 ms | 150.0 ms |
| 32 | 992 B | 1.0 ms | 25.0 ms | 150.0 ms |
| 64 | 2.0 KB | 1.0 ms | 25.0 ms | 150.0 ms |
| 128 | 4.0 KB | 1.0 ms | 25.0 ms | 150.0 ms |
| 256 | 8.0 KB | 1.0 ms | 25.1 ms | 150.1 ms |
| 512 | 16.0 KB | 1.0 ms | 25.1 ms | 150.1 ms |

## Communication

```
```
