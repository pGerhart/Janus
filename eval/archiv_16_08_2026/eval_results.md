# Janus benchmark results

Median runtimes per party, measured with `cargo bench` (Criterion). Regenerate with `eval/scripts/run_bench.sh` followed by `eval/scripts/build_results.py`.

| | |
|---|---|
| Machine | Intel(R) Xeon(R) 6975P-C |
| Instance | c8i.4xlarge |
| Cores | 16 logical on 8 physical with SMT |
| Memory | 30 GB |
| Architecture | x86_64 |
| AVX-512 IFMA | yes |
| Curve backend | `avx512` |
| OS | Linux ip-172-31-10-0 7.0.0-1006-aws #6-Ubuntu SMP PREEMPT Tue May 26 12:04:34 UTC 2026 x86_64 GNU/Linux |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| RUSTFLAGS | `-C target-cpu=native` |
| Date | 2026-08-10T15:37:13Z |

The curve backend row is what `curve25519-dalek` actually compiled, read from its build script rather than inferred from the CPU. `avx512` is the IFMA path, `simd` is AVX2, and `serial` is the portable fallback, so that row states which arithmetic these numbers measure.

The parallel rows are wall-clock times of the same work spread over all cores, so read their speedup against the physical core count above, not the logical one.

`(t, n)` = (threshold, parties). Initiate and output are the phases each party runs; output verifies the other `n - 1` proofs, so it grows quadratically in the committee size. The abort rows run only when a dealer sends a share that does not open its commitment, so they are off the honest path.

## Janus-1 (one round)

### Schnorr

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (8, 16) | 1.4 ms | 8.5 ms |
| (16, 32) | 2.7 ms | 30.6 ms |
| (32, 64) | 5.6 ms | 121.0 ms |
| (64, 128) | 12.1 ms | 502.2 ms |
| (128, 256) | 27.5 ms | 2.40 s |
| (256, 512) | 69.0 ms | 12.97 s |

### Fischlin small

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (8, 16) | 5.1 ms | 53.5 ms |
| (16, 32) | 9.3 ms | 198.2 ms |
| (32, 64) | 17.8 ms | 765.5 ms |
| (64, 128) | 35.1 ms | 3.06 s |
| (128, 256) | 72.3 ms | 12.41 s |
| (256, 512) | 158.7 ms | 53.85 s |

### Fischlin large

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (8, 16) | 6.4 ms | 128.8 ms |
| (16, 32) | 12.2 ms | 488.0 ms |
| (32, 64) | 24.0 ms | 1.90 s |
| (64, 128) | 49.2 ms | 7.52 s |

## Janus-2 (two rounds)

### Schnorr

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (8, 16) | 781 µs | 8.4 ms | 3.1 ms |
| (16, 32) | 1.4 ms | 27.8 ms | 6.3 ms |
| (32, 64) | 2.9 ms | 99.9 ms | 12.5 ms |
| (64, 128) | 6.4 ms | 377.8 ms | 25.1 ms |
| (128, 256) | 16.0 ms | 1.47 s | 50.0 ms |
| (256, 512) | 45.8 ms | 5.70 s | 100.0 ms |

### Fischlin small

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (8, 16) | 8.2 ms | 18.0 ms | 3.2 ms |
| (16, 32) | 14.2 ms | 49.8 ms | 6.3 ms |
| (32, 64) | 25.2 ms | 154.9 ms | 12.6 ms |
| (64, 128) | 48.8 ms | 529.8 ms | 25.1 ms |
| (128, 256) | 99.6 ms | 1.95 s | 50.0 ms |
| (256, 512) | 213.1 ms | 7.34 s | 100.1 ms |

### Fischlin large

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (8, 16) | 2.9 ms | 33.6 ms | 3.1 ms |
| (16, 32) | 4.2 ms | 84.0 ms | 6.3 ms |
| (32, 64) | 6.9 ms | 235.5 ms | 12.5 ms |
| (64, 128) | 13.1 ms | 743.3 ms | 25.0 ms |

## Identifiable abort

| (t, n) | J1 build | J1 verify | J1 worst case | J2 build | J2 verify | J2 worst case |
|:---|---:|---:|---:|---:|---:|---:|
| (8, 16) | 87 µs | 135 µs | 2.3 ms | 87 µs | 277 µs | 4.1 ms |
| (16, 32) | 87 µs | 134 µs | 4.7 ms | 87 µs | 402 µs | 12.4 ms |
| (32, 64) | 87 µs | 134 µs | 9.6 ms | 87 µs | 652 µs | 40.9 ms |
| (64, 128) | 87 µs | 134 µs | 19.3 ms | 87 µs | 1.2 ms | 145.9 ms |
| (128, 256) | 88 µs | 135 µs | 38.7 ms | 87 µs | 2.2 ms | 545.5 ms |
| (256, 512) | 87 µs | 135 µs | 77.7 ms | 87 µs | 4.2 ms | 2.11 s |

## One core against all cores

The same work, run sequentially and spread over every core. Read the speedup against the physical core count in the machine table, not the logical one. The initiate phase has no parallel counterpart: it is a party's own message, and the Fischlin prover already spreads its repetitions over the cores inside the sequential call.

### Janus-1, Schnorr

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (8, 16) | 8.4 ms | 1.7 ms |
| (16, 32) | 30.4 ms | 4.9 ms |
| (32, 64) | 120.1 ms | 15.0 ms |
| (64, 128) | 498.2 ms | 53.6 ms |
| (128, 256) | 2.36 s | 227.0 ms |
| (256, 512) | 12.86 s | 1.15 s |

### Janus-1, Fischlin small

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (8, 16) | 52.7 ms | 6.1 ms |
| (16, 32) | 197.0 ms | 21.7 ms |
| (32, 64) | 761.2 ms | 79.2 ms |
| (64, 128) | 3.05 s | 295.1 ms |
| (128, 256) | 12.59 s | 1.18 s |
| (256, 512) | 53.68 s | 4.93 s |

### Janus-1, Fischlin large

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (8, 16) | 128.0 ms | 13.8 ms |
| (16, 32) | 485.1 ms | 49.1 ms |
| (32, 64) | 1.87 s | 184.6 ms |
| (64, 128) | 7.49 s | 709.2 ms |

### Janus-2, Schnorr

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (8, 16) | 8.5 ms | 1.7 ms | 3.1 ms | 1.1 ms |
| (16, 32) | 28.2 ms | 4.4 ms | 6.3 ms | 2.3 ms |
| (32, 64) | 100.7 ms | 12.4 ms | 12.6 ms | 4.5 ms |
| (64, 128) | 380.5 ms | 39.9 ms | 25.1 ms | 8.5 ms |
| (128, 256) | 1.48 s | 144.7 ms | 50.2 ms | 16.6 ms |
| (256, 512) | 5.71 s | 540.2 ms | 100.3 ms | 32.7 ms |

### Janus-2, Fischlin small

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (8, 16) | 18.0 ms | 2.6 ms | 3.1 ms | 1.1 ms |
| (16, 32) | 49.8 ms | 6.3 ms | 6.3 ms | 2.3 ms |
| (32, 64) | 153.9 ms | 17.3 ms | 12.6 ms | 4.5 ms |
| (64, 128) | 528.7 ms | 54.1 ms | 25.1 ms | 8.5 ms |
| (128, 256) | 1.95 s | 188.5 ms | 50.1 ms | 16.6 ms |
| (256, 512) | 7.31 s | 691.2 ms | 100.8 ms | 32.7 ms |

### Janus-2, Fischlin large

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (8, 16) | 33.7 ms | 4.0 ms | 3.1 ms | 1.1 ms |
| (16, 32) | 83.6 ms | 9.4 ms | 6.3 ms | 2.3 ms |
| (32, 64) | 234.9 ms | 24.6 ms | 12.6 ms | 4.5 ms |
| (64, 128) | 740.9 ms | 73.4 ms | 25.1 ms | 8.5 ms |

## Batch verification

Verifying the received proofs and channel signatures one by one against verifying them in a single batched check, at the three largest settings.

### Schnorr

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (64, 128) | 356.6 ms | 353.5 ms |
| (128, 256) | 1.84 s | 1.82 s |
| (256, 512) | 10.83 s | 10.65 s |

### Fischlin small

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (64, 128) | 2.00 s | 1.95 s |
| (128, 256) | 8.32 s | 8.26 s |
| (256, 512) | 36.61 s | 36.41 s |

### Channel signatures

| (t, n) | Signatures, one by one | Signatures, batched |
|:---|---:|---:|
| (64, 128) | 131.3 ms | 131.6 ms |
| (128, 256) | 520.9 ms | 521.8 ms |
| (256, 512) | 2.04 s | 2.07 s |

## Component breakdown, Fischlin small, (t=32, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 10.3 ms | 23.1 ms |
| Share encryption | 1.3 ms | 1.6 ms |
| Proof verification, one | 7.6 ms | 1.0 ms |
| Proof verification, all | 489.8 ms | 65.6 ms |
| Share decryption | 2.6 ms | 2.6 ms |
| Opening checks | 1.2 ms | 33.7 ms |
| Key aggregation | 581 µs | 33.1 ms |
| Message authentication | 9.6 ms | 19.2 ms |
| Message decoding | 288.1 ms | 25.9 ms |

> Message authentication checks the signature over the bytes as received. Message decoding is the one-time cost of parsing those bytes into group elements, which dominates because every point needs a decompression.

## End-to-end run

One party's whole run, from building its own message to holding the output key. Every message is encoded on the way out and decoded on the way in, which the phase tables above skip.

Compute is measured, the link columns are attributed as `max(compute, transfer) + rounds * RTT`. A party reaches that bound by verifying each message as it arrives; one that waits for the whole round pays the sum instead. Broadcast is counted as point-to-point fan-out on a full-duplex link, so a party uploads once per peer.

The benchmark process peaked at 1.4 GB resident while holding every party of the largest setting at once, which bounds what one party needs to keep a round in memory.

The rounds include one echo round on top of the protocol, since the protocol rounds only disseminate: every party sends a digest of what it received, which catches a dealer that told two parties different things. That gives broadcast with abort, which suits a protocol that already identifies the party at fault. Naming the culprit needs per-dealer hashes and is counted with the abort path.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (8, 16) | 10.0 ms | 45.0 KB | 12.0 ms | 60.0 ms | 310.0 ms |
| (16, 32) | 33.9 ms | 178.6 KB | 35.9 ms | 83.9 ms | 333.9 ms |
| (32, 64) | 129.5 ms | 711.5 KB | 131.5 ms | 179.5 ms | 429.5 ms |
| (64, 128) | 515.0 ms | 2.8 MB | 517.0 ms | 565.0 ms | 815.0 ms |
| (128, 256) | 2.42 s | 11.1 MB | 2.42 s | 2.47 s | 2.72 s |
| (256, 512) | 13.04 s | 44.5 MB | 13.05 s | 13.10 s | 13.35 s |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (8, 16) | 60.0 ms | 341.3 KB | 62.0 ms | 110.0 ms | 360.0 ms |
| (16, 32) | 212.2 ms | 1.3 MB | 214.2 ms | 262.1 ms | 512.1 ms |
| (32, 64) | 801.3 ms | 5.4 MB | 803.3 ms | 851.3 ms | 1.10 s |
| (64, 128) | 3.14 s | 21.5 MB | 3.14 s | 3.19 s | 3.44 s |
| (128, 256) | 12.91 s | 86.1 MB | 12.92 s | 12.96 s | 13.21 s |
| (256, 512) | 54.80 s | 344.4 MB | 54.81 s | 54.85 s | 55.10 s |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (8, 16) | 14.6 ms | 39.0 KB | 17.6 ms | 89.5 ms | 464.6 ms |
| (16, 32) | 41.0 ms | 135.3 KB | 44.0 ms | 116.0 ms | 491.0 ms |
| (32, 64) | 131.4 ms | 497.4 KB | 134.4 ms | 206.4 ms | 581.4 ms |
| (64, 128) | 461.0 ms | 1.9 MB | 464.0 ms | 536.0 ms | 911.0 ms |
| (128, 256) | 1.72 s | 7.3 MB | 1.72 s | 1.80 s | 2.17 s |
| (256, 512) | 6.54 s | 28.8 MB | 6.54 s | 6.61 s | 6.99 s |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (8, 16) | 33.2 ms | 187.5 KB | 36.2 ms | 108.2 ms | 483.2 ms |
| (16, 32) | 80.0 ms | 674.7 KB | 83.0 ms | 155.0 ms | 530.0 ms |
| (32, 64) | 221.9 ms | 2.5 MB | 224.9 ms | 296.9 ms | 671.9 ms |
| (64, 128) | 699.5 ms | 9.6 MB | 702.5 ms | 774.5 ms | 1.15 s |
| (128, 256) | 2.43 s | 37.8 MB | 2.44 s | 2.51 s | 2.88 s |
| (256, 512) | 8.97 s | 149.8 MB | 8.97 s | 9.04 s | 9.42 s |

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

## Threshold sweep at a fixed committee

The main sweep moves the threshold and the committee together, so it cannot separate their effects. Here the committee is fixed at 256 and only the threshold moves. Same link model as above, one echo round included.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 1.64 s | 10.2 MB | 1.64 s | 1.69 s | 1.94 s |
| (32, 256) | 1.75 s | 10.4 MB | 1.76 s | 1.80 s | 2.05 s |
| (64, 256) | 1.97 s | 10.6 MB | 1.97 s | 2.02 s | 2.27 s |
| (128, 256) | 2.42 s | 11.1 MB | 2.42 s | 2.47 s | 2.72 s |
| (192, 256) | 2.90 s | 11.6 MB | 2.90 s | 2.95 s | 3.20 s |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 11.70 s | 72.1 MB | 11.70 s | 11.75 s | 12.00 s |
| (32, 256) | 11.87 s | 74.1 MB | 11.87 s | 11.92 s | 12.17 s |
| (64, 256) | 12.16 s | 78.1 MB | 12.16 s | 12.21 s | 12.46 s |
| (128, 256) | 12.72 s | 86.1 MB | 12.72 s | 12.77 s | 13.02 s |
| (192, 256) | 13.37 s | 94.0 MB | 13.38 s | 13.42 s | 13.67 s |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 349.1 ms | 4.7 MB | 352.1 ms | 424.1 ms | 799.1 ms |
| (32, 256) | 545.1 ms | 5.0 MB | 548.1 ms | 620.1 ms | 995.1 ms |
| (64, 256) | 936.2 ms | 5.8 MB | 939.2 ms | 1.01 s | 1.39 s |
| (128, 256) | 1.73 s | 7.3 MB | 1.73 s | 1.80 s | 2.18 s |
| (192, 256) | 2.48 s | 8.8 MB | 2.49 s | 2.56 s | 2.93 s |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 572.2 ms | 9.0 MB | 575.2 ms | 647.2 ms | 1.02 s |
| (32, 256) | 837.8 ms | 13.1 MB | 840.8 ms | 912.8 ms | 1.29 s |
| (64, 256) | 1.38 s | 21.3 MB | 1.38 s | 1.45 s | 1.83 s |
| (128, 256) | 2.44 s | 37.8 MB | 2.44 s | 2.51 s | 2.89 s |
| (192, 256) | 3.48 s | 54.2 MB | 3.48 s | 3.55 s | 3.93 s |

## An encoding we measured and did not adopt

A verifier can rebuild the first-round commitments from the challenge and the responses instead of receiving them, the way a Schnorr signature carries the challenge. Both columns run the same path, encode then decode then verify, so the parsing the shorter encoding avoids is counted in its favour. The code is in `eval/compact-encoding`.

| (t, n) | Proof sent | Proof rebuilt | Saved | Verify sent | Verify rebuilt | Cost |
|:---|---:|---:|---:|---:|---:|---:|
| (32, 64) | 81.1 KB | 48.6 KB | 40% | 11.7 ms | 28.9 ms | 2.47x |
| (64, 128) | 161.1 KB | 96.6 KB | 40% | 23.5 ms | 64.9 ms | 2.76x |
| (128, 256) | 321.1 KB | 192.6 KB | 40% | 48.6 ms | 157.2 ms | 3.24x |
| (256, 512) | 641.1 KB | 384.6 KB | 40% | 104.0 ms | 426.1 ms | 4.10x |

The saving holds at about 40 percent while the cost climbs with the committee size, so the trade gets worse exactly where a smaller message would help most. At the largest setting the bytes are worth the arithmetic only below roughly 8 Mbit/s, which is far under any link these parties run on, so the protocol keeps the longer encoding.

## Communication

```
[schnorr initiate t8_n16] proof=1347 (1.32 KB), broadcast=3030 (2.96 KB)
[schnorr initiate t16_n32] proof=2627 (2.57 KB), broadcast=5862 (5.72 KB)
[schnorr initiate t32_n64] proof=5187 (5.07 KB), broadcast=11526 (11.26 KB)
[schnorr initiate t64_n128] proof=10309 (10.07 KB), broadcast=22858 (22.32 KB)
[schnorr initiate t128_n256] proof=20550 (20.07 KB), broadcast=45644 (44.57 KB)
[schnorr initiate t256_n512] proof=41030 (40.07 KB), broadcast=91212 (89.07 KB)
[schnorr output t8_n16] received=45450 (44.38 KB)
[schnorr output t16_n32] received=181722 (177.46 KB)
[schnorr output t32_n64] received=726138 (709.12 KB)
[schnorr output t64_n128] received=2902966 (2.77 MB)
[schnorr output t128_n256] received=11639220 (11.10 MB)
[schnorr output t256_n512] received=46609332 (44.45 MB)
[fischlin-small initiate t8_n16] proof=21578 (21.07 KB), broadcast=23261 (22.72 KB)
[fischlin-small initiate t16_n32] proof=42061 (41.08 KB), broadcast=45296 (44.23 KB)
[fischlin-small initiate t32_n64] proof=83019 (81.07 KB), broadcast=89358 (87.26 KB)
[fischlin-small initiate t64_n128] proof=164970 (161.10 KB), broadcast=177519 (173.36 KB)
[fischlin-small initiate t128_n256] proof=328826 (321.12 KB), broadcast=353920 (345.62 KB)
[fischlin-small initiate t256_n512] proof=656508 (641.12 KB), broadcast=706690 (690.13 KB)
[fischlin-small output t8_n16] received=348928 (340.75 KB)
[fischlin-small output t16_n32] received=1404103 (1.34 MB)
[fischlin-small output t32_n64] received=5629556 (5.37 MB)
[fischlin-small output t64_n128] received=22545010 (21.50 MB)
[fischlin-small output t128_n256] received=90249768 (86.07 MB)
[fischlin-small output t256_n512] received=361117941 (344.39 MB)
[fischlin-large initiate t8_n16] proof=57965 (56.61 KB), broadcast=59648 (58.25 KB)
[fischlin-large initiate t16_n32] proof=113005 (110.36 KB), broadcast=116240 (113.52 KB)
[fischlin-large initiate t32_n64] proof=223085 (217.86 KB), broadcast=229424 (224.05 KB)
[fischlin-large initiate t64_n128] proof=443331 (432.94 KB), broadcast=455880 (445.20 KB)
[fischlin-large output t8_n16] received=894720 (873.75 KB)
[fischlin-large output t16_n32] received=3603440 (3.44 MB)
[fischlin-large output t32_n64] received=14453712 (13.78 MB)
[fischlin-large output t64_n128] received=57896760 (55.21 MB)
[two-round schnorr initiate t8_n16] proof=674 (674 B), broadcast=2133 (2.08 KB)
[two-round schnorr initiate t16_n32] proof=1186 (1.16 KB), broadcast=3941 (3.85 KB)
[two-round schnorr initiate t32_n64] proof=2210 (2.16 KB), broadcast=7557 (7.38 KB)
[two-round schnorr initiate t64_n128] proof=4258 (4.16 KB), broadcast=14790 (14.44 KB)
[two-round schnorr initiate t128_n256] proof=8356 (8.16 KB), broadcast=29386 (28.70 KB)
[two-round schnorr initiate t256_n512] proof=16548 (16.16 KB), broadcast=58570 (57.20 KB)
[two-round schnorr finalize t8_n16] round1-received=31995 (31.25 KB)
[two-round schnorr finalize t16_n32] round1-received=122171 (119.31 KB)
[two-round schnorr finalize t32_n64] round1-received=476091 (464.93 KB)
[two-round schnorr finalize t64_n128] round1-received=1878330 (1.79 MB)
[two-round schnorr finalize t128_n256] round1-received=7493430 (7.15 MB)
[two-round schnorr finalize t256_n512] round1-received=29929270 (28.54 MB)
[two-round schnorr output t8_n16] round1-received=31995 (31.25 KB), round2-received=7230 (7.06 KB), total=39225 (38.31 KB)
[two-round schnorr output t16_n32] round1-received=122171 (119.31 KB), round2-received=14942 (14.59 KB), total=137113 (133.90 KB)
[two-round schnorr output t32_n64] round1-received=476091 (464.93 KB), round2-received=30366 (29.65 KB), total=506457 (494.59 KB)
[two-round schnorr output t64_n128] round1-received=1878330 (1.79 MB), round2-received=61215 (59.78 KB), total=1939545 (1.85 MB)
[two-round schnorr output t128_n256] round1-received=7493430 (7.15 MB), round2-received=123039 (120.16 KB), total=7616469 (7.26 MB)
[two-round schnorr output t256_n512] round1-received=29929270 (28.54 MB), round2-received=246687 (240.91 KB), total=30175957 (28.78 MB)
[two-round fischlin-small initiate t8_n16] proof=10809 (10.56 KB), broadcast=12268 (11.98 KB)
[two-round fischlin-small initiate t16_n32] proof=19003 (18.56 KB), broadcast=21758 (21.25 KB)
[two-round fischlin-small initiate t32_n64] proof=35384 (34.55 KB), broadcast=40731 (39.78 KB)
[two-round fischlin-small initiate t64_n128] proof=68158 (66.56 KB), broadcast=78690 (76.85 KB)
[two-round fischlin-small initiate t128_n256] proof=133720 (130.59 KB), broadcast=154750 (151.12 KB)
[two-round fischlin-small initiate t256_n512] proof=264797 (258.59 KB), broadcast=306819 (299.63 KB)
[two-round fischlin-small finalize t8_n16] round1-received=184067 (179.75 KB)
[two-round fischlin-small finalize t16_n32] round1-received=674499 (658.69 KB)
[two-round fischlin-small finalize t32_n64] round1-received=2566196 (2.45 MB)
[two-round fischlin-small finalize t64_n128] round1-received=9993214 (9.53 MB)
[two-round fischlin-small finalize t128_n256] round1-received=39461938 (37.63 MB)
[two-round fischlin-small finalize t256_n512] round1-received=156783340 (149.52 MB)
[two-round fischlin-small output t8_n16] round1-received=184043 (179.73 KB), round2-received=7230 (7.06 KB), total=191273 (186.79 KB)
[two-round fischlin-small output t16_n32] round1-received=674483 (658.67 KB), round2-received=14942 (14.59 KB), total=689425 (673.27 KB)
[two-round fischlin-small output t32_n64] round1-received=2566199 (2.45 MB), round2-received=30366 (29.65 KB), total=2596565 (2.48 MB)
[two-round fischlin-small output t64_n128] round1-received=9993215 (9.53 MB), round2-received=61215 (59.78 KB), total=10054430 (9.59 MB)
[two-round fischlin-small output t128_n256] round1-received=39461945 (37.63 MB), round2-received=123039 (120.16 KB), total=39584984 (37.75 MB)
[two-round fischlin-small output t256_n512] round1-received=156783253 (149.52 MB), round2-received=246687 (240.91 KB), total=157029940 (149.76 MB)
[two-round fischlin-large initiate t8_n16] proof=29026 (28.35 KB), broadcast=30485 (29.77 KB)
[two-round fischlin-large initiate t16_n32] proof=51042 (49.85 KB), broadcast=53797 (52.54 KB)
[two-round fischlin-large initiate t32_n64] proof=95074 (92.85 KB), broadcast=100421 (98.07 KB)
[two-round fischlin-large initiate t64_n128] proof=183138 (178.85 KB), broadcast=193670 (189.13 KB)
[two-round fischlin-large finalize t8_n16] round1-received=457275 (446.56 KB)
[two-round fischlin-large finalize t16_n32] round1-received=1667707 (1.59 MB)
[two-round fischlin-large finalize t32_n64] round1-received=6326523 (6.03 MB)
[two-round fischlin-large finalize t64_n128] round1-received=24596090 (23.46 MB)
[two-round fischlin-large output t8_n16] round1-received=457275 (446.56 KB), round2-received=7230 (7.06 KB), total=464505 (453.62 KB)
[two-round fischlin-large output t16_n32] round1-received=1667707 (1.59 MB), round2-received=14942 (14.59 KB), total=1682649 (1.60 MB)
[two-round fischlin-large output t32_n64] round1-received=6326523 (6.03 MB), round2-received=30366 (29.65 KB), total=6356889 (6.06 MB)
[two-round fischlin-large output t64_n128] round1-received=24596090 (23.46 MB), round2-received=61215 (59.78 KB), total=24657305 (23.52 MB)
[one-round abort t8_n16] report=259 bytes
[one-round abort t16_n32] report=259 bytes
[one-round abort t32_n64] report=259 bytes
[one-round abort t64_n128] report=259 bytes
[one-round abort t128_n256] report=259 bytes
[one-round abort t256_n512] report=259 bytes
[two-round abort t8_n16] report=259 bytes
[two-round abort t16_n32] report=259 bytes
[two-round abort t32_n64] report=259 bytes
[two-round abort t64_n128] report=259 bytes
[two-round abort t128_n256] report=259 bytes
[two-round abort t256_n512] report=259 bytes
```
