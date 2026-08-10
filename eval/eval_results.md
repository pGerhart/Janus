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
| OS | Linux ip-172-31-0-206 7.0.0-1006-aws #6-Ubuntu SMP PREEMPT Tue May 26 12:04:34 UTC 2026 x86_64 GNU/Linux |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| RUSTFLAGS | `-C target-cpu=native` |
| Date | 2026-08-09T16:31:33Z |

The curve backend row is what `curve25519-dalek` actually compiled, read from its build script rather than inferred from the CPU. `avx512` is the IFMA path, `simd` is AVX2, and `serial` is the portable fallback, so that row states which arithmetic these numbers measure.

The parallel rows are wall-clock times of the same work spread over all cores, so read their speedup against the physical core count above, not the logical one.

`(t, n)` = (threshold, parties). Initiate and output are the phases each party runs; output verifies the other `n - 1` proofs, so it grows quadratically in the committee size. The abort rows run only when a dealer sends a share that does not open its commitment, so they are off the honest path.

## Janus-1 (one round)

### Schnorr

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 1.3 ms | 8.4 ms |
| (8, 32) | 2.5 ms | 29.8 ms |
| (16, 64) | 5.0 ms | 113.5 ms |
| (32, 64) | 5.3 ms | 121.0 ms |
| (64, 128) | 11.4 ms | 500.4 ms |
| (128, 256) | 26.0 ms | 2.38 s |
| (256, 512) | 66.1 ms | 12.86 s |

### Fischlin small

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 4.8 ms | 52.5 ms |
| (8, 32) | 8.5 ms | 195.5 ms |
| (16, 64) | 16.2 ms | 758.6 ms |
| (32, 64) | 17.7 ms | 772.4 ms |
| (64, 128) | 35.2 ms | 3.03 s |
| (128, 256) | 72.0 ms | 12.50 s |
| (256, 512) | 162.6 ms | 53.69 s |

### Fischlin large

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 6.4 ms | 127.7 ms |
| (8, 32) | 12.1 ms | 482.9 ms |
| (16, 64) | 23.5 ms | 1.85 s |
| (32, 64) | 24.1 ms | 1.89 s |
| (64, 128) | 49.3 ms | 7.45 s |

## Janus-2 (two rounds)

### Schnorr

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 652 µs | 5.7 ms | 3.1 ms |
| (8, 32) | 1.1 ms | 16.9 ms | 6.3 ms |
| (16, 64) | 2.2 ms | 56.2 ms | 12.5 ms |
| (32, 64) | 2.8 ms | 100.2 ms | 12.6 ms |
| (64, 128) | 6.4 ms | 378.8 ms | 25.0 ms |
| (128, 256) | 16.0 ms | 1.48 s | 49.8 ms |
| (256, 512) | 46.0 ms | 5.73 s | 99.8 ms |

### Fischlin small

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 6.9 ms | 14.6 ms | 3.1 ms |
| (8, 32) | 10.2 ms | 36.1 ms | 6.3 ms |
| (16, 64) | 18.2 ms | 99.9 ms | 12.5 ms |
| (32, 64) | 31.5 ms | 154.7 ms | 12.5 ms |
| (64, 128) | 57.9 ms | 530.6 ms | 24.9 ms |
| (128, 256) | 120.3 ms | 1.94 s | 49.8 ms |
| (256, 512) | 257.4 ms | 7.34 s | 99.7 ms |

### Fischlin large

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 2.5 ms | 29.5 ms | 3.1 ms |
| (8, 32) | 3.3 ms | 67.0 ms | 6.3 ms |
| (16, 64) | 5.1 ms | 168.6 ms | 12.5 ms |
| (32, 64) | 7.4 ms | 235.4 ms | 12.6 ms |
| (64, 128) | 14.0 ms | 738.6 ms | 25.0 ms |

## Identifiable abort

| (t, n) | J1 build | J1 verify | J1 worst case | J2 build | J2 verify | J2 worst case |
|:---|---:|---:|---:|---:|---:|---:|
| (4, 16) | 87 µs | 134 µs | 2.0 ms | 87 µs | 214 µs | 3.2 ms |
| (8, 32) | 87 µs | 134 µs | 4.2 ms | 87 µs | 277 µs | 8.5 ms |
| (16, 64) | 87 µs | 134 µs | 8.6 ms | 87 µs | 400 µs | 25.2 ms |
| (32, 64) | 87 µs | 134 µs | 8.5 ms | 87 µs | 652 µs | 40.8 ms |
| (64, 128) | 87 µs | 135 µs | 17.3 ms | 87 µs | 1.1 ms | 145.5 ms |
| (128, 256) | 87 µs | 134 µs | 34.6 ms | 87 µs | 2.1 ms | 544.3 ms |
| (256, 512) | 87 µs | 134 µs | 69.4 ms | 87 µs | 4.1 ms | 2.10 s |

## One core against all cores

The same work, run sequentially and spread over every core. Read the speedup against the physical core count in the machine table, not the logical one. The initiate phase has no parallel counterpart: it is a party's own message, and the Fischlin prover already spreads its repetitions over the cores inside the sequential call.

## Batch verification

Verifying the received proofs and channel signatures one by one against verifying them in a single batched check, at the three largest settings.

### Schnorr

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (64, 128) | 357.1 ms | 356.8 ms |
| (128, 256) | 1.83 s | 1.82 s |
| (256, 512) | 10.72 s | 10.71 s |

### Fischlin small

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (64, 128) | 1.99 s | 2.00 s |
| (128, 256) | 8.28 s | 8.27 s |
| (256, 512) | 36.48 s | 36.43 s |

### Channel signatures

| (t, n) | Signatures, one by one | Signatures, batched |
|:---|---:|---:|
| (64, 128) | 131.0 ms | 129.4 ms |
| (128, 256) | 514.6 ms | 521.2 ms |
| (256, 512) | 2.07 s | 2.05 s |

## Component breakdown, Fischlin small, (t=32, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 10.3 ms | 23.0 ms |
| Share encryption | 1.3 ms | 1.3 ms |
| Proof verification, one | 7.6 ms | 1.0 ms |
| Proof verification, all | 484.7 ms | 65.7 ms |
| Share decryption | 2.6 ms | 2.6 ms |
| Opening checks | 1.2 ms | 33.7 ms |
| Key aggregation | 575 µs | 33.0 ms |
| Message authentication | 9.6 ms | 19.0 ms |
| Message decoding | 288.3 ms | -- |

> Message authentication checks the signature over the bytes as received. Message decoding is the one-time cost of parsing those bytes into group elements, which dominates because every point needs a decompression.

## End-to-end run

One party's whole run, from building its own message to holding the output key. Every message is encoded on the way out and decoded on the way in, which the phase tables above skip.

Compute is measured, the link columns are attributed as `max(compute, transfer) + rounds * RTT`. A party reaches that bound by verifying each message as it arrives; one that waits for the whole round pays the sum instead. Broadcast is counted as point-to-point fan-out on a full-duplex link, so a party uploads once per peer.

The benchmark process peaked at 1.3 GB resident while holding every party of the largest setting at once, which bounds what one party needs to keep a round in memory.

The rounds include one echo round on top of the protocol, since the protocol rounds only disseminate: every party sends a digest of what it received, which catches a dealer that told two parties different things. That gives broadcast with abort, which suits a protocol that already identifies the party at fault. Naming the culprit needs per-dealer hashes and is counted with the abort path.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 9.8 ms | 43.1 KB | 11.8 ms | 59.8 ms | 309.8 ms |
| (8, 32) | 32.8 ms | 170.9 KB | 34.8 ms | 82.8 ms | 332.8 ms |
| (16, 64) | 121.0 ms | 680.0 KB | 123.0 ms | 171.0 ms | 421.0 ms |
| (32, 64) | 128.6 ms | 711.5 KB | 130.6 ms | 178.6 ms | 428.6 ms |
| (64, 128) | 514.4 ms | 2.8 MB | 516.4 ms | 564.4 ms | 814.4 ms |
| (128, 256) | 2.42 s | 11.1 MB | 2.42 s | 2.47 s | 2.72 s |
| (256, 512) | 13.01 s | 44.5 MB | 13.01 s | 13.06 s | 13.31 s |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 59.0 ms | 311.3 KB | 61.0 ms | 109.0 ms | 359.0 ms |
| (8, 32) | 209.1 ms | 1.2 MB | 211.1 ms | 259.1 ms | 509.1 ms |
| (16, 64) | 784.4 ms | 4.9 MB | 786.4 ms | 834.4 ms | 1.08 s |
| (32, 64) | 804.1 ms | 5.4 MB | 806.1 ms | 854.1 ms | 1.10 s |
| (64, 128) | 3.12 s | 21.5 MB | 3.12 s | 3.17 s | 3.42 s |
| (128, 256) | 12.84 s | 86.1 MB | 12.84 s | 12.89 s | 13.14 s |
| (256, 512) | 54.76 s | 344.4 MB | 54.76 s | 54.81 s | 55.06 s |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 11.4 ms | 33.4 KB | 14.4 ms | 86.4 ms | 461.4 ms |
| (8, 32) | 28.7 ms | 112.0 KB | 31.7 ms | 103.7 ms | 478.7 ms |
| (16, 64) | 82.6 ms | 402.9 KB | 85.6 ms | 157.6 ms | 532.6 ms |
| (32, 64) | 131.3 ms | 497.4 KB | 134.3 ms | 206.3 ms | 581.3 ms |
| (64, 128) | 462.4 ms | 1.9 MB | 465.4 ms | 537.4 ms | 912.4 ms |
| (128, 256) | 1.72 s | 7.3 MB | 1.72 s | 1.79 s | 2.17 s |
| (256, 512) | 6.56 s | 28.8 MB | 6.57 s | 6.64 s | 7.01 s |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 27.1 ms | 125.6 KB | 30.1 ms | 102.1 ms | 477.1 ms |
| (8, 32) | 58.6 ms | 418.9 KB | 61.6 ms | 133.6 ms | 508.6 ms |
| (16, 64) | 146.9 ms | 1.5 MB | 149.9 ms | 221.9 ms | 596.9 ms |
| (32, 64) | 220.8 ms | 2.5 MB | 223.8 ms | 295.9 ms | 670.9 ms |
| (64, 128) | 696.5 ms | 9.6 MB | 699.5 ms | 771.5 ms | 1.15 s |
| (128, 256) | 2.44 s | 37.8 MB | 2.45 s | 2.52 s | 2.89 s |
| (256, 512) | 8.94 s | 149.8 MB | 8.94 s | 9.02 s | 9.39 s |

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

## An encoding we measured and did not adopt

A verifier can rebuild the first-round commitments from the challenge and the responses instead of receiving them, the way a Schnorr signature carries the challenge. Both columns run the same path, encode then decode then verify, so the parsing the shorter encoding avoids is counted in its favour. The code is in `eval/compact-encoding`.

| (t, n) | Proof sent | Proof rebuilt | Saved | Verify sent | Verify rebuilt | Cost |
|:---|---:|---:|---:|---:|---:|---:|
| (4, 16) | 19.1 KB | 10.6 KB | 45% | 3.1 ms | 6.6 ms | 2.11x |
| (8, 32) | 37.1 KB | 20.6 KB | 45% | 5.9 ms | 13.4 ms | 2.26x |
| (16, 64) | 73.1 KB | 40.6 KB | 44% | 11.5 ms | 27.3 ms | 2.38x |
| (32, 64) | 81.1 KB | 48.6 KB | 40% | 11.6 ms | 29.2 ms | 2.52x |
| (64, 128) | 161.1 KB | 96.6 KB | 40% | 23.7 ms | 65.2 ms | 2.75x |
| (128, 256) | 321.1 KB | 192.6 KB | 40% | 48.2 ms | 157.6 ms | 3.27x |
| (256, 512) | 641.1 KB | 384.6 KB | 40% | 103.2 ms | 425.9 ms | 4.13x |

The saving holds at about 40 percent while the cost climbs from 1.7x to 3.9x, so the trade gets worse exactly where a smaller message would help most. At the largest setting the bytes are worth the arithmetic only below roughly 8 Mbit/s, which is far under any link these parties run on, so the protocol keeps the longer encoding.

## Communication

```
[schnorr initiate t4_n16] proof=1219 (1.19 KB), broadcast=2902 (2.83 KB)
[schnorr initiate t8_n32] proof=2371 (2.32 KB), broadcast=5606 (5.47 KB)
[schnorr initiate t16_n64] proof=4675 (4.57 KB), broadcast=11014 (10.76 KB)
[schnorr initiate t32_n64] proof=5187 (5.07 KB), broadcast=11526 (11.26 KB)
[schnorr initiate t64_n128] proof=10309 (10.07 KB), broadcast=22858 (22.32 KB)
[schnorr initiate t128_n256] proof=20550 (20.07 KB), broadcast=45644 (44.57 KB)
[schnorr initiate t256_n512] proof=41030 (40.07 KB), broadcast=91212 (89.07 KB)
[schnorr output t4_n16] received=43530 (42.51 KB)
[schnorr output t8_n32] received=173786 (169.71 KB)
[schnorr output t16_n64] received=693882 (677.62 KB)
[schnorr output t32_n64] received=726138 (709.12 KB)
[schnorr output t64_n128] received=2902966 (2.77 MB)
[schnorr output t128_n256] received=11639220 (11.10 MB)
[schnorr output t256_n512] received=46609332 (44.45 MB)
[fischlin-small initiate t4_n16] proof=19534 (19.08 KB), broadcast=21217 (20.72 KB)
[fischlin-small initiate t8_n32] proof=37965 (37.08 KB), broadcast=41200 (40.23 KB)
[fischlin-small initiate t16_n64] proof=74826 (73.07 KB), broadcast=81165 (79.26 KB)
[fischlin-small initiate t32_n64] proof=83019 (81.07 KB), broadcast=89358 (87.26 KB)
[fischlin-small initiate t64_n128] proof=164970 (161.10 KB), broadcast=177519 (173.36 KB)
[fischlin-small initiate t128_n256] proof=328829 (321.12 KB), broadcast=353923 (345.63 KB)
[fischlin-small initiate t256_n512] proof=656508 (641.12 KB), broadcast=706690 (690.13 KB)
[fischlin-small output t4_n16] received=318210 (310.75 KB)
[fischlin-small output t8_n32] received=1277123 (1.22 MB)
[fischlin-small output t16_n64] received=5113405 (4.88 MB)
[fischlin-small output t32_n64] received=5629551 (5.37 MB)
[fischlin-small output t64_n128] received=22545024 (21.50 MB)
[fischlin-small output t128_n256] received=90249781 (86.07 MB)
[fischlin-small output t256_n512] received=361117975 (344.39 MB)
[fischlin-large initiate t4_n16] proof=52461 (51.23 KB), broadcast=54144 (52.88 KB)
[fischlin-large initiate t8_n32] proof=101997 (99.61 KB), broadcast=105232 (102.77 KB)
[fischlin-large initiate t16_n64] proof=201069 (196.36 KB), broadcast=207408 (202.55 KB)
[fischlin-large initiate t32_n64] proof=223085 (217.86 KB), broadcast=229424 (224.05 KB)
[fischlin-large initiate t64_n128] proof=443331 (432.94 KB), broadcast=455880 (445.20 KB)
[fischlin-large output t4_n16] received=812160 (793.12 KB)
[fischlin-large output t8_n32] received=3262192 (3.11 MB)
[fischlin-large output t16_n64] received=13066704 (12.46 MB)
[fischlin-large output t32_n64] received=14453712 (13.78 MB)
[fischlin-large output t64_n128] received=57896760 (55.21 MB)
[two-round schnorr initiate t4_n16] proof=418 (418 B), broadcast=1749 (1.71 KB)
[two-round schnorr initiate t8_n32] proof=674 (674 B), broadcast=3173 (3.10 KB)
[two-round schnorr initiate t16_n64] proof=1186 (1.16 KB), broadcast=6021 (5.88 KB)
[two-round schnorr initiate t32_n64] proof=2210 (2.16 KB), broadcast=7557 (7.38 KB)
[two-round schnorr initiate t64_n128] proof=4258 (4.16 KB), broadcast=14790 (14.44 KB)
[two-round schnorr initiate t128_n256] proof=8356 (8.16 KB), broadcast=29386 (28.70 KB)
[two-round schnorr initiate t256_n512] proof=16548 (16.16 KB), broadcast=58570 (57.20 KB)
[two-round schnorr finalize t4_n16] round1-received=26235 (25.62 KB)
[two-round schnorr finalize t8_n32] round1-received=98363 (96.06 KB)
[two-round schnorr finalize t16_n64] round1-received=379323 (370.43 KB)
[two-round schnorr finalize t32_n64] round1-received=476091 (464.93 KB)
[two-round schnorr finalize t64_n128] round1-received=1878330 (1.79 MB)
[two-round schnorr finalize t128_n256] round1-received=7493430 (7.15 MB)
[two-round schnorr finalize t256_n512] round1-received=29929270 (28.54 MB)
[two-round schnorr output t4_n16] round1-received=26235 (25.62 KB), round2-received=7230 (7.06 KB), total=33465 (32.68 KB)
[two-round schnorr output t8_n32] round1-received=98363 (96.06 KB), round2-received=14942 (14.59 KB), total=113305 (110.65 KB)
[two-round schnorr output t16_n64] round1-received=379323 (370.43 KB), round2-received=30366 (29.65 KB), total=409689 (400.09 KB)
[two-round schnorr output t32_n64] round1-received=476091 (464.93 KB), round2-received=30366 (29.65 KB), total=506457 (494.59 KB)
[two-round schnorr output t64_n128] round1-received=1878330 (1.79 MB), round2-received=61215 (59.78 KB), total=1939545 (1.85 MB)
[two-round schnorr output t128_n256] round1-received=7493430 (7.15 MB), round2-received=123039 (120.16 KB), total=7616469 (7.26 MB)
[two-round schnorr output t256_n512] round1-received=29929270 (28.54 MB), round2-received=246687 (240.91 KB), total=30175957 (28.78 MB)
[two-round fischlin-small initiate t4_n16] proof=6717 (6.56 KB), broadcast=8048 (7.86 KB)
[two-round fischlin-small initiate t8_n32] proof=10812 (10.56 KB), broadcast=13311 (13.00 KB)
[two-round fischlin-small initiate t16_n64] proof=19000 (18.55 KB), broadcast=23835 (23.28 KB)
[two-round fischlin-small initiate t32_n64] proof=35386 (34.56 KB), broadcast=40733 (39.78 KB)
[two-round fischlin-small initiate t64_n128] proof=68156 (66.56 KB), broadcast=78688 (76.84 KB)
[two-round fischlin-small initiate t128_n256] proof=133721 (130.59 KB), broadcast=154751 (151.12 KB)
[two-round fischlin-small initiate t256_n512] proof=264794 (258.59 KB), broadcast=306816 (299.62 KB)
[two-round fischlin-small finalize t4_n16] round1-received=120676 (117.85 KB)
[two-round fischlin-small finalize t8_n32] round1-received=412599 (402.93 KB)
[two-round fischlin-small finalize t16_n64] round1-received=1501762 (1.43 MB)
[two-round fischlin-small finalize t32_n64] round1-received=2566218 (2.45 MB)
[two-round fischlin-small finalize t64_n128] round1-received=9993224 (9.53 MB)
[two-round fischlin-small finalize t128_n256] round1-received=39461942 (37.63 MB)
[two-round fischlin-small finalize t256_n512] round1-received=156783374 (149.52 MB)
[two-round fischlin-small output t4_n16] round1-received=120671 (117.84 KB), round2-received=7230 (7.06 KB), total=127901 (124.90 KB)
[two-round fischlin-small output t8_n32] round1-received=412610 (402.94 KB), round2-received=14942 (14.59 KB), total=427552 (417.53 KB)
[two-round fischlin-small output t16_n64] round1-received=1501799 (1.43 MB), round2-received=30366 (29.65 KB), total=1532165 (1.46 MB)
[two-round fischlin-small output t32_n64] round1-received=2566232 (2.45 MB), round2-received=30366 (29.65 KB), total=2596598 (2.48 MB)
[two-round fischlin-small output t64_n128] round1-received=9993237 (9.53 MB), round2-received=61215 (59.78 KB), total=10054452 (9.59 MB)
[two-round fischlin-small output t128_n256] round1-received=39461886 (37.63 MB), round2-received=123039 (120.16 KB), total=39584925 (37.75 MB)
[two-round fischlin-small output t256_n512] round1-received=156783364 (149.52 MB), round2-received=246687 (240.91 KB), total=157030051 (149.76 MB)
[two-round fischlin-large initiate t4_n16] proof=18018 (17.60 KB), broadcast=19349 (18.90 KB)
[two-round fischlin-large initiate t8_n32] proof=29026 (28.35 KB), broadcast=31525 (30.79 KB)
[two-round fischlin-large initiate t16_n64] proof=51042 (49.85 KB), broadcast=55877 (54.57 KB)
[two-round fischlin-large initiate t32_n64] proof=95074 (92.85 KB), broadcast=100421 (98.07 KB)
[two-round fischlin-large initiate t64_n128] proof=183138 (178.85 KB), broadcast=193670 (189.13 KB)
[two-round fischlin-large finalize t4_n16] round1-received=290235 (283.43 KB)
[two-round fischlin-large finalize t8_n32] round1-received=977275 (954.37 KB)
[two-round fischlin-large finalize t16_n64] round1-received=3520251 (3.36 MB)
[two-round fischlin-large finalize t32_n64] round1-received=6326523 (6.03 MB)
[two-round fischlin-large finalize t64_n128] round1-received=24596090 (23.46 MB)
[two-round fischlin-large output t4_n16] round1-received=290235 (283.43 KB), round2-received=7230 (7.06 KB), total=297465 (290.49 KB)
[two-round fischlin-large output t8_n32] round1-received=977275 (954.37 KB), round2-received=14942 (14.59 KB), total=992217 (968.96 KB)
[two-round fischlin-large output t16_n64] round1-received=3520251 (3.36 MB), round2-received=30366 (29.65 KB), total=3550617 (3.39 MB)
[two-round fischlin-large output t32_n64] round1-received=6326523 (6.03 MB), round2-received=30366 (29.65 KB), total=6356889 (6.06 MB)
[two-round fischlin-large output t64_n128] round1-received=24596090 (23.46 MB), round2-received=61215 (59.78 KB), total=24657305 (23.52 MB)
[one-round abort t4_n16] report=259 bytes
[one-round abort t8_n32] report=259 bytes
[one-round abort t16_n64] report=259 bytes
[one-round abort t32_n64] report=259 bytes
[one-round abort t64_n128] report=259 bytes
[one-round abort t128_n256] report=259 bytes
[one-round abort t256_n512] report=259 bytes
[two-round abort t4_n16] report=259 bytes
[two-round abort t8_n32] report=259 bytes
[two-round abort t16_n64] report=259 bytes
[two-round abort t32_n64] report=259 bytes
[two-round abort t64_n128] report=259 bytes
[two-round abort t128_n256] report=259 bytes
[two-round abort t256_n512] report=259 bytes
```
