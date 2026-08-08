# Janus benchmark results

Median runtimes per party, measured with `cargo bench` (Criterion). Regenerate with `scripts/run_bench.sh` followed by `scripts/make_results.py`.

| | |
|---|---|
| Machine | Intel(R) Xeon(R) 6975P-C |
| Instance | c8i.4xlarge |
| Cores | 16 logical on 8 physical with SMT |
| Memory | 30 GB |
| Architecture | x86_64 |
| AVX-512 IFMA | yes |
| Curve backend | `avx512` |
| OS | Linux ip-172-31-42-6 7.0.0-1006-aws #6-Ubuntu SMP PREEMPT Tue May 26 12:04:34 UTC 2026 x86_64 GNU/Linux |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| RUSTFLAGS | `-C target-cpu=native` |
| Date | 2026-08-01T20:33:04Z |

The curve backend row is what `curve25519-dalek` actually compiled, read from its build script rather than inferred from the CPU. `avx512` is the IFMA path, `simd` is AVX2, and `serial` is the portable fallback, so that row states which arithmetic these numbers measure.

The parallel rows are wall-clock times of the same work spread over all cores, so read their speedup against the physical core count above, not the logical one.

`(t, n)` = (threshold, parties). Initiate and output are the phases each party runs; output verifies the other `n - 1` proofs, so it grows quadratically in the committee size. The abort rows run only when a dealer sends a share that does not open its commitment, so they are off the honest path.

## Janus-1 (one round)

### Schnorr

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 1.3 ms | 8.3 ms |
| (8, 32) | 2.5 ms | 29.7 ms |
| (16, 64) | 5.1 ms | 114.6 ms |
| (32, 64) | 5.3 ms | 122.7 ms |
| (64, 128) | 11.4 ms | 509.5 ms |
| (128, 256) | 26.4 ms | 2.46 s |
| (256, 512) | 66.7 ms | 13.52 s |

### Fischlin small

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 5.1 ms | 52.3 ms |
| (8, 32) | 8.9 ms | 195.4 ms |
| (16, 64) | 16.7 ms | 751.8 ms |
| (32, 64) | 18.6 ms | 765.4 ms |
| (64, 128) | 36.6 ms | 3.03 s |
| (128, 256) | 75.1 ms | 12.42 s |
| (256, 512) | 165.7 ms | 54.16 s |

### Fischlin large

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (4, 16) | 6.5 ms | 126.6 ms |
| (8, 32) | 12.3 ms | 477.8 ms |
| (16, 64) | 24.1 ms | 1.85 s |
| (32, 64) | 24.9 ms | 1.88 s |
| (64, 128) | 50.8 ms | 7.45 s |

## Janus-2 (two rounds)

### Schnorr

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 887 µs | 8.4 ms | 4.7 ms |
| (8, 32) | 1.5 ms | 25.5 ms | 9.5 ms |
| (16, 64) | 2.9 ms | 85.9 ms | 18.9 ms |
| (32, 64) | 3.6 ms | 155.0 ms | 19.0 ms |
| (64, 128) | 7.7 ms | 589.0 ms | 38.0 ms |
| (128, 256) | 18.0 ms | 2.29 s | 76.1 ms |
| (256, 512) | 47.7 ms | 8.99 s | 152.2 ms |

### Fischlin small

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 6.3 ms | 21.5 ms | 4.7 ms |
| (8, 32) | 9.3 ms | 52.6 ms | 9.5 ms |
| (16, 64) | 16.1 ms | 145.7 ms | 19.0 ms |
| (32, 64) | 26.9 ms | 225.9 ms | 18.9 ms |
| (64, 128) | 51.4 ms | 774.0 ms | 38.0 ms |
| (128, 256) | 106.2 ms | 2.84 s | 76.1 ms |
| (256, 512) | 208.4 ms | 10.86 s | 151.6 ms |

### Fischlin large

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (4, 16) | 3.2 ms | 43.3 ms | 4.7 ms |
| (8, 32) | 4.2 ms | 97.7 ms | 9.5 ms |
| (16, 64) | 6.3 ms | 241.8 ms | 19.0 ms |
| (32, 64) | 8.3 ms | 334.9 ms | 18.9 ms |
| (64, 128) | 15.0 ms | 1.04 s | 38.1 ms |

## Identifiable abort

| (t, n) | J1 build | J1 verify | J1 worst case | J2 build | J2 verify | J2 worst case |
|:---|---:|---:|---:|---:|---:|---:|
| (4, 16) | 112 µs | 183 µs | 2.7 ms | 112 µs | 318 µs | 4.8 ms |
| (8, 32) | 112 µs | 183 µs | 5.7 ms | 112 µs | 427 µs | 13.2 ms |
| (16, 64) | 112 µs | 183 µs | 11.5 ms | 112 µs | 642 µs | 40.5 ms |
| (32, 64) | 112 µs | 183 µs | 11.6 ms | 112 µs | 1.1 ms | 67.8 ms |
| (64, 128) | 112 µs | 183 µs | 23.3 ms | 112 µs | 1.9 ms | 245.7 ms |
| (128, 256) | 112 µs | 183 µs | 46.8 ms | 112 µs | 3.7 ms | 933.0 ms |
| (256, 512) | 112 µs | 183 µs | 93.9 ms | 112 µs | 7.1 ms | 3.63 s |

## Parallel output and batching

### Schnorr

| (t, n) | Output sequential | Output parallel |
|:---|---:|---:|
| (64, 128) | 555.9 ms | 64.4 ms |
| (128, 256) | 2.49 s | 253.3 ms |
| (256, 512) | 12.64 s | 1.20 s |

### Fischlin small

| (t, n) | Output sequential | Output parallel |
|:---|---:|---:|
| (64, 128) | 3.47 s | 339.7 ms |
| (128, 256) | 14.02 s | 1.35 s |
| (256, 512) | 59.14 s | 5.52 s |

## Component breakdown, Fischlin small, (t=32, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 10.2 ms | 23.6 ms |
| Share encryption | 1.3 ms | 2.1 ms |
| Proof verification, one | 7.6 ms | 1.4 ms |
| Proof verification, all | 485.7 ms | 89.8 ms |
| Share decryption | 2.6 ms | 3.7 ms |
| Opening checks | 1.2 ms | 57.4 ms |
| Key aggregation | 578 µs | 57.6 ms |
| Message authentication | 9.5 ms | 20.0 ms |
| Message decoding | 289.5 ms | -- |

> Message authentication checks the signature over the bytes as received. Message decoding is the one-time cost of parsing those bytes into group elements, which dominates because every point needs a decompression.

## End-to-end run

One party's whole run, from building its own message to holding the output key. Every message is encoded on the way out and decoded on the way in, which the phase tables above skip.

Compute is measured, the link columns are attributed as `max(compute, transfer) + rounds * RTT`. A party reaches that bound by verifying each message as it arrives; one that waits for the whole round pays the sum instead. Broadcast is counted as point-to-point fan-out on a full-duplex link, so a party uploads once per peer.

The rounds include one echo round on top of the protocol, since the protocol rounds only disseminate: every party sends a digest of what it received, which catches a dealer that told two parties different things. That gives broadcast with abort, which suits a protocol that already identifies the party at fault. Naming the culprit needs per-dealer hashes and is counted with the abort path.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 10.1 ms | 43.1 KB | 12.1 ms | 60.1 ms | 310.1 ms |
| (8, 32) | 31.1 ms | 170.9 KB | 33.1 ms | 81.1 ms | 331.1 ms |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 55.0 ms | 311.3 KB | 57.0 ms | 105.0 ms | 355.0 ms |
| (8, 32) | 189.9 ms | 1.2 MB | 191.9 ms | 239.9 ms | 489.9 ms |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 13.7 ms | 33.4 KB | 16.7 ms | 88.7 ms | 463.7 ms |
| (8, 32) | 35.0 ms | 112.0 KB | 38.0 ms | 110.0 ms | 485.0 ms |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (4, 16) | 29.0 ms | 125.6 KB | 32.0 ms | 104.0 ms | 479.0 ms |
| (8, 32) | 68.8 ms | 418.9 KB | 71.8 ms | 143.8 ms | 518.8 ms |

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
[fischlin-small initiate t4_n16] proof=19532 (19.07 KB), broadcast=21215 (20.72 KB)
[fischlin-small initiate t8_n32] proof=37961 (37.07 KB), broadcast=41196 (40.23 KB)
[fischlin-small initiate t16_n64] proof=74826 (73.07 KB), broadcast=81165 (79.26 KB)
[fischlin-small initiate t32_n64] proof=83019 (81.07 KB), broadcast=89358 (87.26 KB)
[fischlin-small initiate t64_n128] proof=164975 (161.11 KB), broadcast=177524 (173.36 KB)
[fischlin-small initiate t128_n256] proof=328826 (321.12 KB), broadcast=353920 (345.62 KB)
[fischlin-small initiate t256_n512] proof=656509 (641.12 KB), broadcast=706691 (690.13 KB)
[fischlin-small output t4_n16] received=318199 (310.74 KB)
[fischlin-small output t8_n32] received=1277145 (1.22 MB)
[fischlin-small output t16_n64] received=5113415 (4.88 MB)
[fischlin-small output t32_n64] received=5629535 (5.37 MB)
[fischlin-small output t64_n128] received=22544974 (21.50 MB)
[fischlin-small output t128_n256] received=90249724 (86.07 MB)
[fischlin-small output t256_n512] received=361117927 (344.39 MB)
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
[two-round fischlin-small initiate t4_n16] proof=6715 (6.56 KB), broadcast=8046 (7.86 KB)
[two-round fischlin-small initiate t8_n32] proof=10815 (10.56 KB), broadcast=13314 (13.00 KB)
[two-round fischlin-small initiate t16_n64] proof=19003 (18.56 KB), broadcast=23838 (23.28 KB)
[two-round fischlin-small initiate t32_n64] proof=35388 (34.56 KB), broadcast=40735 (39.78 KB)
[two-round fischlin-small initiate t64_n128] proof=68158 (66.56 KB), broadcast=78690 (76.85 KB)
[two-round fischlin-small initiate t128_n256] proof=133722 (130.59 KB), broadcast=154752 (151.12 KB)
[two-round fischlin-small initiate t256_n512] proof=264797 (258.59 KB), broadcast=306819 (299.63 KB)
[two-round fischlin-small finalize t4_n16] round1-received=120687 (117.86 KB)
[two-round fischlin-small finalize t8_n32] round1-received=412594 (402.92 KB)
[two-round fischlin-small finalize t16_n64] round1-received=1501759 (1.43 MB)
[two-round fischlin-small finalize t32_n64] round1-received=2566225 (2.45 MB)
[two-round fischlin-small finalize t64_n128] round1-received=9993231 (9.53 MB)
[two-round fischlin-small finalize t128_n256] round1-received=39461925 (37.63 MB)
[two-round fischlin-small finalize t256_n512] round1-received=156783418 (149.52 MB)
[two-round fischlin-small output t4_n16] round1-received=120685 (117.86 KB), round2-received=7230 (7.06 KB), total=127915 (124.92 KB)
[two-round fischlin-small output t8_n32] round1-received=412601 (402.93 KB), round2-received=14942 (14.59 KB), total=427543 (417.52 KB)
[two-round fischlin-small output t16_n64] round1-received=1501775 (1.43 MB), round2-received=30366 (29.65 KB), total=1532141 (1.46 MB)
[two-round fischlin-small output t32_n64] round1-received=2566224 (2.45 MB), round2-received=30366 (29.65 KB), total=2596590 (2.48 MB)
[two-round fischlin-small output t64_n128] round1-received=9993256 (9.53 MB), round2-received=61215 (59.78 KB), total=10054471 (9.59 MB)
[two-round fischlin-small output t128_n256] round1-received=39461976 (37.63 MB), round2-received=123039 (120.16 KB), total=39585015 (37.75 MB)
[two-round fischlin-small output t256_n512] round1-received=156783301 (149.52 MB), round2-received=246687 (240.91 KB), total=157029988 (149.76 MB)
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
