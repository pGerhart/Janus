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
| OS | Linux ip-172-31-13-243 7.0.0-1006-aws #6-Ubuntu SMP PREEMPT Tue May 26 12:04:34 UTC 2026 x86_64 GNU/Linux |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| RUSTFLAGS | `-C target-cpu=native` |
| Date | 2026-08-17T04:07:16Z |

`(t, n)` = (degree, parties), every setting n-out-of-n with `t = n - 1`. Previous run at `t = n/2` in `eval/archiv_16_08_2026/`.

## Janus-1 (one round)

### Schnorr

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (15, 16) | 1.3 ms | 8.7 ms |
| (31, 32) | 2.7 ms | 32.3 ms |
| (63, 64) | 5.7 ms | 135.1 ms |
| (127, 128) | 13.1 ms | 611.7 ms |
| (255, 256) | 33.0 ms | 3.27 s |
| (511, 512) | 93.7 ms | 20.01 s |

### Fischlin small

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (15, 16) | 18.3 ms | 53.9 ms |
| (31, 32) | 35.4 ms | 203.0 ms |
| (63, 64) | 73.3 ms | 792.3 ms |
| (127, 128) | 157.6 ms | 3.25 s |
| (255, 256) | 369.3 ms | 13.77 s |
| (511, 512) | 993.5 ms | 62.09 s |

### Fischlin large

| (t, n) | Initiate | Output |
|:---|---:|---:|
| (15, 16) | 22.5 ms | 129.9 ms |
| (31, 32) | 46.3 ms | 494.3 ms |
| (63, 64) | 100.5 ms | 1.93 s |
| (127, 128) | 238.0 ms | 7.73 s |

## Janus-2 (two rounds)

### Schnorr

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (15, 16) | 1.0 ms | 13.2 ms | 3.2 ms |
| (31, 32) | 2.0 ms | 48.6 ms | 6.3 ms |
| (63, 64) | 4.2 ms | 186.4 ms | 12.6 ms |
| (127, 128) | 9.9 ms | 731.0 ms | 25.2 ms |
| (255, 256) | 26.5 ms | 2.85 s | 50.4 ms |
| (511, 512) | 80.4 ms | 11.25 s | 100.7 ms |

### Fischlin small

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (15, 16) | 12.6 ms | 24.0 ms | 3.1 ms |
| (31, 32) | 23.1 ms | 75.7 ms | 6.3 ms |
| (63, 64) | 45.8 ms | 262.0 ms | 12.6 ms |
| (127, 128) | 91.5 ms | 967.4 ms | 25.2 ms |
| (255, 256) | 192.3 ms | 3.66 s | 50.4 ms |
| (511, 512) | 373.9 ms | 14.25 s | 100.9 ms |

### Fischlin large

| (t, n) | Initiate | Finalize | Output |
|:---|---:|---:|---:|
| (15, 16) | 3.6 ms | 41.0 ms | 3.1 ms |
| (31, 32) | 5.9 ms | 115.7 ms | 6.3 ms |
| (63, 64) | 10.8 ms | 366.5 ms | 12.6 ms |
| (127, 128) | 21.8 ms | 1.28 s | 25.1 ms |

## Identifiable abort

| (t, n) | J1 build | J1 verify | J1 worst case | J2 build | J2 verify | J2 worst case |
|:---|---:|---:|---:|---:|---:|---:|
| (15, 16) | 87 µs | 135 µs | 2.0 ms | 87 µs | 387 µs | 5.8 ms |
| (31, 32) | 87 µs | 135 µs | 4.2 ms | 87 µs | 638 µs | 19.8 ms |
| (63, 64) | 87 µs | 135 µs | 8.5 ms | 87 µs | 1.1 ms | 72.0 ms |
| (127, 128) | 87 µs | 135 µs | 17.2 ms | 87 µs | 2.1 ms | 272.4 ms |
| (255, 256) | 87 µs | 135 µs | 34.6 ms | 87 µs | 4.1 ms | 1.06 s |
| (511, 512) | 87 µs | 135 µs | 69.3 ms | 87 µs | 8.2 ms | 4.17 s |

## One core against all cores

### Janus-1, Schnorr

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (15, 16) | 8.7 ms | 1.8 ms |
| (31, 32) | 32.3 ms | 5.1 ms |
| (63, 64) | 134.9 ms | 16.4 ms |
| (127, 128) | 610.1 ms | 63.1 ms |
| (255, 256) | 3.26 s | 300.3 ms |
| (511, 512) | 19.93 s | 1.74 s |

### Janus-1, Fischlin small

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (15, 16) | 54.2 ms | 6.3 ms |
| (31, 32) | 203.8 ms | 22.2 ms |
| (63, 64) | 795.0 ms | 80.6 ms |
| (127, 128) | 3.22 s | 306.7 ms |
| (255, 256) | 13.79 s | 1.28 s |
| (511, 512) | 62.06 s | 5.58 s |

### Janus-1, Fischlin large

| (t, n) | Output, one core | Output, all cores |
|:---|---:|---:|
| (15, 16) | 130.3 ms | 13.9 ms |
| (31, 32) | 494.1 ms | 49.9 ms |
| (63, 64) | 1.93 s | 187.6 ms |
| (127, 128) | 7.74 s | 735.2 ms |

### Janus-2, Schnorr

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (15, 16) | 13.2 ms | 2.2 ms | 3.2 ms | 1.1 ms |
| (31, 32) | 48.5 ms | 6.3 ms | 6.3 ms | 2.4 ms |
| (63, 64) | 186.5 ms | 20.2 ms | 12.6 ms | 4.5 ms |
| (127, 128) | 730.0 ms | 72.9 ms | 25.3 ms | 8.6 ms |
| (255, 256) | 2.84 s | 271.4 ms | 50.5 ms | 16.7 ms |
| (511, 512) | 11.24 s | 1.04 s | 101.0 ms | 32.8 ms |

### Janus-2, Fischlin small

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (15, 16) | 24.0 ms | 3.2 ms | 3.2 ms | 1.1 ms |
| (31, 32) | 75.6 ms | 8.8 ms | 6.3 ms | 2.4 ms |
| (63, 64) | 261.9 ms | 27.4 ms | 12.6 ms | 4.5 ms |
| (127, 128) | 968.2 ms | 95.0 ms | 25.2 ms | 8.6 ms |
| (255, 256) | 3.66 s | 347.2 ms | 50.5 ms | 16.7 ms |
| (511, 512) | 14.26 s | 1.33 s | 101.0 ms | 32.9 ms |

### Janus-2, Fischlin large

| (t, n) | Finalize, one core | Finalize, all cores | Output, one core | Output, all cores |
|:---|---:|---:|---:|---:|
| (15, 16) | 41.1 ms | 4.7 ms | 3.2 ms | 1.1 ms |
| (31, 32) | 116.2 ms | 12.4 ms | 6.3 ms | 2.3 ms |
| (63, 64) | 367.6 ms | 37.3 ms | 12.6 ms | 4.6 ms |
| (127, 128) | 1.28 s | 123.6 ms | 25.2 ms | 8.5 ms |

## Batch verification

### Schnorr

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (127, 128) | 469.4 ms | 472.2 ms |
| (255, 256) | 2.73 s | 2.75 s |
| (511, 512) | 17.92 s | 18.12 s |

### Fischlin small

| (t, n) | Proofs, one by one | Proofs, batched |
|:---|---:|---:|
| (127, 128) | 2.16 s | 2.16 s |
| (255, 256) | 9.41 s | 9.42 s |
| (511, 512) | 44.81 s | 44.75 s |

### Channel signatures

| (t, n) | Signatures, one by one | Signatures, batched |
|:---|---:|---:|
| (127, 128) | 132.8 ms | 131.3 ms |
| (255, 256) | 520.6 ms | 519.7 ms |
| (511, 512) | 2.06 s | 2.07 s |

## Component breakdown

### Fiat-Shamir, (t=15, n=16)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 494 µs | 131 µs |
| Share encryption | 323 µs | 324 µs |
| Proof verification, one | 300 µs | 152 µs |
| Proof verification, all | 5.1 ms | 2.6 ms |
| Share decryption | 616 µs | 620 µs |
| Opening checks | 283 µs | 4.0 ms |
| Key aggregation | 37 µs | 4.0 ms |
| Message authentication | 433 µs | 1.6 ms |
| Message decoding | 2.7 ms | 1.7 ms |
| Public-key contributions | -- | 899 µs |
| Key-equality proofs | -- | 1.4 ms |
| **Total** | **10.0 ms** | **17.4 ms** |

### Fischlin small, (t=15, n=16)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 16.3 ms | 11.7 ms |
| Share encryption | 323 µs | 324 µs |
| Proof verification, one | 2.1 ms | 728 µs |
| Proof verification, all | 34.5 ms | 11.9 ms |
| Share decryption | 616 µs | 620 µs |
| Opening checks | 283 µs | 4.0 ms |
| Key aggregation | 37 µs | 4.0 ms |
| Message authentication | 960 µs | 3.0 ms |
| Message decoding | 19.2 ms | 3.8 ms |
| Public-key contributions | -- | 899 µs |
| Key-equality proofs | -- | 1.4 ms |
| **Total** | **72.2 ms** | **41.7 ms** |

### Fiat-Shamir, (t=63, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 2.2 ms | 400 µs |
| Share encryption | 1.3 ms | 1.3 ms |
| Proof verification, one | 1.5 ms | 528 µs |
| Proof verification, all | 95.9 ms | 35.5 ms |
| Share decryption | 2.6 ms | 2.6 ms |
| Opening checks | 1.2 ms | 64.3 ms |
| Key aggregation | 581 µs | 64.5 ms |
| Message authentication | 2.6 ms | 18.8 ms |
| Message decoding | 35.8 ms | 20.4 ms |
| Public-key contributions | -- | 3.7 ms |
| Key-equality proofs | -- | 5.6 ms |
| **Total** | **142.1 ms** | **217.1 ms** |

### Fischlin small, (t=63, n=64)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 65.1 ms | 40.3 ms |
| Share encryption | 1.3 ms | 1.3 ms |
| Proof verification, one | 8.0 ms | 1.5 ms |
| Proof verification, all | 512.0 ms | 98.5 ms |
| Share decryption | 2.6 ms | 2.6 ms |
| Opening checks | 1.2 ms | 64.3 ms |
| Key aggregation | 581 µs | 64.5 ms |
| Message authentication | 10.9 ms | 30.5 ms |
| Message decoding | 295.6 ms | 41.7 ms |
| Public-key contributions | -- | 3.7 ms |
| Key-equality proofs | -- | 5.6 ms |
| **Total** | **889.3 ms** | **353.0 ms** |

### Fiat-Shamir, (t=511, n=512)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 42.0 ms | 2.9 ms |
| Share encryption | 10.1 ms | 10.1 ms |
| Proof verification, one | 34.6 ms | 3.7 ms |
| Proof verification, all | 17.70 s | 1.91 s |
| Share decryption | 19.8 ms | 19.9 ms |
| Opening checks | 9.7 ms | 4.10 s |
| Key aggregation | 50.7 ms | 4.13 s |
| Message authentication | 86.2 ms | 1.08 s |
| Message decoding | 2.18 s | 1.18 s |
| Public-key contributions | -- | 29.3 ms |
| Key-equality proofs | -- | 45.2 ms |
| **Total** | **20.09 s** | **12.52 s** |

### Fischlin small, (t=511, n=512)

| Component | One round | Two rounds |
|:---|---:|---:|
| Proof generation | 853.6 ms | 333.2 ms |
| Share encryption | 10.1 ms | 10.1 ms |
| Proof verification, one | 87.8 ms | 8.4 ms |
| Proof verification, all | 44.96 s | 4.32 s |
| Share decryption | 19.8 ms | 19.9 ms |
| Opening checks | 9.7 ms | 4.10 s |
| Key aggregation | 50.7 ms | 4.13 s |
| Message authentication | 637.9 ms | 1.61 s |
| Message decoding | 18.54 s | 2.32 s |
| Public-key contributions | -- | 29.3 ms |
| Key-equality proofs | -- | 45.2 ms |
| **Total** | **65.08 s** | **16.92 s** |

## End-to-end run

Link columns are `max(compute, transfer) + rounds * RTT`, with 2 rounds charged per broadcast round, so 2 for Janus-1 and 4 for Janus-2.

Peak resident memory: 1.7 GB.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (15, 16) | 10.2 ms | 48.2 KB | 12.2 ms | 60.2 ms | 310.2 ms |
| (31, 32) | 35.7 ms | 193.2 KB | 37.7 ms | 85.7 ms | 335.7 ms |
| (63, 64) | 143.2 ms | 772.5 KB | 145.2 ms | 193.2 ms | 443.2 ms |
| (127, 128) | 628.8 ms | 3.0 MB | 630.8 ms | 678.8 ms | 928.8 ms |
| (255, 256) | 3.31 s | 12.1 MB | 3.31 s | 3.36 s | 3.61 s |
| (511, 512) | 20.05 s | 48.4 MB | 20.05 s | 20.10 s | 20.35 s |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (15, 16) | 74.7 ms | 393.8 KB | 76.7 ms | 124.7 ms | 374.7 ms |
| (31, 32) | 243.2 ms | 1.6 MB | 245.2 ms | 293.2 ms | 543.2 ms |
| (63, 64) | 889.5 ms | 6.3 MB | 891.5 ms | 939.5 ms | 1.19 s |
| (127, 128) | 3.42 s | 25.4 MB | 3.42 s | 3.47 s | 3.72 s |
| (255, 256) | 14.34 s | 101.9 MB | 14.34 s | 14.39 s | 14.64 s |
| (511, 512) | 63.75 s | 408.0 MB | 63.75 s | 63.80 s | 64.05 s |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (15, 16) | 19.9 ms | 49.3 KB | 23.9 ms | 119.9 ms | 619.9 ms |
| (31, 32) | 64.1 ms | 179.9 KB | 68.1 ms | 164.1 ms | 664.1 ms |
| (63, 64) | 226.4 ms | 682.5 KB | 230.4 ms | 326.4 ms | 826.4 ms |
| (127, 128) | 846.5 ms | 2.6 MB | 850.5 ms | 946.5 ms | 1.45 s |
| (255, 256) | 3.22 s | 10.2 MB | 3.22 s | 3.32 s | 3.82 s |
| (511, 512) | 12.57 s | 40.7 MB | 12.57 s | 12.67 s | 13.17 s |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (15, 16) | 44.5 ms | 296.2 KB | 48.5 ms | 144.5 ms | 644.5 ms |
| (31, 32) | 119.8 ms | 1.1 MB | 123.8 ms | 219.8 ms | 719.8 ms |
| (63, 64) | 365.8 ms | 4.4 MB | 369.8 ms | 465.8 ms | 965.8 ms |
| (127, 128) | 1.24 s | 17.7 MB | 1.24 s | 1.34 s | 1.84 s |
| (255, 256) | 4.48 s | 70.4 MB | 4.48 s | 4.58 s | 5.08 s |
| (511, 512) | 17.00 s | 281.0 MB | 17.00 s | 17.10 s | 17.60 s |

### What the second round of a broadcast costs

Included above once per broadcast round. Subtract a row to read a protocol charged one round per broadcast instead.

| n | Extra bytes | One region | Cross-region | Intercontinental |
|:---|---:|---:|---:|---:|
| 16 | 480 B | 1.0 ms | 25.0 ms | 150.0 ms |
| 32 | 992 B | 1.0 ms | 25.0 ms | 150.0 ms |
| 64 | 2.0 KB | 1.0 ms | 25.0 ms | 150.0 ms |
| 128 | 4.0 KB | 1.0 ms | 25.0 ms | 150.0 ms |
| 256 | 8.0 KB | 1.0 ms | 25.1 ms | 150.1 ms |
| 512 | 16.0 KB | 1.0 ms | 25.1 ms | 150.1 ms |

## Threshold sweep at a fixed committee

Committee fixed at 256, threshold alone moving, up to the n-out-of-n point the tables above are measured at.

### Janus-1, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 1.62 s | 10.2 MB | 1.62 s | 1.67 s | 1.92 s |
| (32, 256) | 1.73 s | 10.4 MB | 1.74 s | 1.78 s | 2.03 s |
| (64, 256) | 1.96 s | 10.6 MB | 1.96 s | 2.01 s | 2.26 s |
| (128, 256) | 2.41 s | 11.1 MB | 2.41 s | 2.46 s | 2.71 s |
| (192, 256) | 2.86 s | 11.6 MB | 2.87 s | 2.91 s | 3.16 s |
| (255, 256) | 3.31 s | 12.1 MB | 3.31 s | 3.36 s | 3.61 s |

### Janus-1, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 11.79 s | 72.1 MB | 11.79 s | 11.84 s | 12.09 s |
| (32, 256) | 11.93 s | 74.1 MB | 11.94 s | 11.98 s | 12.23 s |
| (64, 256) | 12.25 s | 78.1 MB | 12.25 s | 12.30 s | 12.55 s |
| (128, 256) | 12.88 s | 86.1 MB | 12.88 s | 12.93 s | 13.18 s |
| (192, 256) | 13.50 s | 94.0 MB | 13.50 s | 13.55 s | 13.80 s |
| (255, 256) | 14.14 s | 101.9 MB | 14.14 s | 14.19 s | 14.44 s |

### Janus-2, Schnorr

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 352.1 ms | 4.7 MB | 356.1 ms | 452.1 ms | 952.1 ms |
| (32, 256) | 547.5 ms | 5.0 MB | 551.5 ms | 647.5 ms | 1.15 s |
| (64, 256) | 938.5 ms | 5.8 MB | 942.5 ms | 1.04 s | 1.54 s |
| (128, 256) | 1.72 s | 7.3 MB | 1.72 s | 1.82 s | 2.32 s |
| (192, 256) | 2.47 s | 8.8 MB | 2.48 s | 2.57 s | 3.07 s |
| (255, 256) | 3.22 s | 10.2 MB | 3.22 s | 3.32 s | 3.82 s |

### Janus-2, Fischlin small

| (t, n) | Compute | Received | One region (1 ms, 10 Gbit/s) | Cross-region (25 ms, 1 Gbit/s) | Intercontinental (150 ms, 1 Gbit/s) |
|:---|---:|---:|---:|---:|---:|
| (16, 256) | 576.9 ms | 9.0 MB | 580.9 ms | 676.9 ms | 1.18 s |
| (32, 256) | 846.0 ms | 13.1 MB | 850.0 ms | 946.0 ms | 1.45 s |
| (64, 256) | 1.37 s | 21.3 MB | 1.38 s | 1.47 s | 1.97 s |
| (128, 256) | 2.43 s | 37.8 MB | 2.43 s | 2.53 s | 3.03 s |
| (192, 256) | 3.46 s | 54.2 MB | 3.46 s | 3.56 s | 4.06 s |
| (255, 256) | 4.50 s | 70.4 MB | 4.51 s | 4.60 s | 5.10 s |

## An encoding we measured and did not adopt

Rebuilding the first-round commitments from the challenge and the responses instead of receiving them. Code in `eval/compact-encoding`.

| (t, n) | Proof sent | Proof rebuilt | Saved | Verify sent | Verify rebuilt | Cost |
|:---|---:|---:|---:|---:|---:|---:|
| (15, 16) | 24.6 KB | 16.1 KB | 35% | 3.2 ms | 7.1 ms | 2.18x |
| (31, 32) | 48.6 KB | 32.1 KB | 34% | 6.1 ms | 14.7 ms | 2.39x |
| (63, 64) | 96.6 KB | 64.1 KB | 34% | 12.1 ms | 32.6 ms | 2.69x |
| (127, 128) | 192.6 KB | 128.1 KB | 34% | 25.0 ms | 79.2 ms | 3.17x |
| (255, 256) | 384.6 KB | 256.1 KB | 33% | 53.1 ms | 214.9 ms | 4.05x |
| (511, 512) | 768.6 KB | 512.1 KB | 33% | 120.2 ms | 655.0 ms | 5.45x |

Saving about 35 percent, and at the largest setting worth the arithmetic only below roughly 4 Mbit/s, under every link profile above.

## Communication

```
[schnorr initiate t15_n16] proof=1571 (1.53 KB), broadcast=3254 (3.18 KB)
[schnorr initiate t31_n32] proof=3107 (3.03 KB), broadcast=6342 (6.19 KB)
[schnorr initiate t63_n64] proof=6179 (6.03 KB), broadcast=12518 (12.22 KB)
[schnorr initiate t127_n128] proof=12326 (12.04 KB), broadcast=24875 (24.29 KB)
[schnorr initiate t255_n256] proof=24614 (24.04 KB), broadcast=49708 (48.54 KB)
[schnorr initiate t511_n512] proof=49190 (48.04 KB), broadcast=99372 (97.04 KB)
[schnorr output t15_n16] received=48810 (47.67 KB)
[schnorr output t31_n32] received=196602 (191.99 KB)
[schnorr output t63_n64] received=788634 (770.15 KB)
[schnorr output t127_n128] received=3159125 (3.01 MB)
[schnorr output t255_n256] received=12675540 (12.09 MB)
[schnorr output t511_n512] received=50779092 (48.43 MB)
[fischlin-small initiate t15_n16] proof=25166 (24.58 KB), broadcast=26849 (26.22 KB)
[fischlin-small initiate t31_n32] proof=49740 (48.57 KB), broadcast=52975 (51.73 KB)
[fischlin-small initiate t63_n64] proof=98892 (96.57 KB), broadcast=105231 (102.76 KB)
[fischlin-small initiate t127_n128] proof=197246 (192.62 KB), broadcast=209795 (204.88 KB)
[fischlin-small initiate t255_n256] proof=393851 (384.62 KB), broadcast=418945 (409.13 KB)
[fischlin-small initiate t511_n512] proof=787065 (768.62 KB), broadcast=837247 (817.62 KB)
[fischlin-small output t15_n16] received=402681 (393.24 KB)
[fischlin-small output t31_n32] received=1642193 (1.57 MB)
[fischlin-small output t63_n64] received=6629445 (6.32 MB)
[fischlin-small output t127_n128] received=26643530 (25.41 MB)
[fischlin-small output t255_n256] received=106830923 (101.88 MB)
[fischlin-small output t511_n512] received=427834137 (408.01 MB)
[fischlin-large initiate t15_n16] proof=67597 (66.01 KB), broadcast=69280 (67.66 KB)
[fischlin-large initiate t31_n32] proof=133645 (130.51 KB), broadcast=136880 (133.67 KB)
[fischlin-large initiate t63_n64] proof=265741 (259.51 KB), broadcast=272080 (265.70 KB)
[fischlin-large initiate t127_n128] proof=530062 (517.64 KB), broadcast=542611 (529.89 KB)
[fischlin-large output t15_n16] received=1039200 (1014.84 KB)
[fischlin-large output t31_n32] received=4243280 (4.05 MB)
[fischlin-large output t63_n64] received=17141040 (16.35 MB)
[fischlin-large output t127_n128] received=68911597 (65.72 MB)
[two-round schnorr initiate t15_n16] proof=1122 (1.10 KB), broadcast=2805 (2.74 KB)
[two-round schnorr initiate t31_n32] proof=2146 (2.10 KB), broadcast=5381 (5.25 KB)
[two-round schnorr initiate t63_n64] proof=4194 (4.10 KB), broadcast=10533 (10.29 KB)
[two-round schnorr initiate t127_n128] proof=8292 (8.10 KB), broadcast=20841 (20.35 KB)
[two-round schnorr initiate t255_n256] proof=16484 (16.10 KB), broadcast=41578 (40.60 KB)
[two-round schnorr initiate t511_n512] proof=32868 (32.10 KB), broadcast=83050 (81.10 KB)
[two-round schnorr finalize t15_n16] round1-received=42075 (41.09 KB)
[two-round schnorr finalize t31_n32] round1-received=166811 (162.90 KB)
[two-round schnorr finalize t63_n64] round1-received=663579 (648.03 KB)
[two-round schnorr finalize t127_n128] round1-received=2646807 (2.52 MB)
[two-round schnorr finalize t255_n256] round1-received=10602390 (10.11 MB)
[two-round schnorr finalize t511_n512] round1-received=42438550 (40.47 MB)
[two-round schnorr output t15_n16] round1-received=42075 (41.09 KB), round2-received=7230 (7.06 KB), total=49305 (48.15 KB)
[two-round schnorr output t31_n32] round1-received=166811 (162.90 KB), round2-received=14942 (14.59 KB), total=181753 (177.49 KB)
[two-round schnorr output t63_n64] round1-received=663579 (648.03 KB), round2-received=30366 (29.65 KB), total=693945 (677.68 KB)
[two-round schnorr output t127_n128] round1-received=2646807 (2.52 MB), round2-received=61215 (59.78 KB), total=2708022 (2.58 MB)
[two-round schnorr output t255_n256] round1-received=10602390 (10.11 MB), round2-received=123039 (120.16 KB), total=10725429 (10.23 MB)
[two-round schnorr output t511_n512] round1-received=42438550 (40.47 MB), round2-received=246687 (240.91 KB), total=42685237 (40.71 MB)
[two-round fischlin-small initiate t15_n16] proof=17979 (17.56 KB), broadcast=19662 (19.20 KB)
[two-round fischlin-small initiate t31_n32] proof=34362 (33.56 KB), broadcast=37597 (36.72 KB)
[two-round fischlin-small initiate t63_n64] proof=67131 (65.56 KB), broadcast=73470 (71.75 KB)
[two-round fischlin-small initiate t127_n128] proof=132699 (129.59 KB), broadcast=145248 (141.84 KB)
[two-round fischlin-small initiate t255_n256] proof=263775 (257.59 KB), broadcast=288869 (282.10 KB)
[two-round fischlin-small initiate t511_n512] proof=525916 (513.59 KB), broadcast=576098 (562.60 KB)
[two-round fischlin-small finalize t15_n16] round1-received=294920 (288.01 KB)
[two-round fischlin-small finalize t31_n32] round1-received=1165548 (1.11 MB)
[two-round fischlin-small finalize t63_n64] round1-received=4628576 (4.41 MB)
[two-round fischlin-small finalize t127_n128] round1-received=18446427 (17.59 MB)
[two-round fischlin-small finalize t255_n256] round1-received=73660526 (70.25 MB)
[two-round fischlin-small finalize t511_n512] round1-received=294385369 (280.75 MB)
[two-round fischlin-small output t15_n16] round1-received=294922 (288.01 KB), round2-received=7230 (7.06 KB), total=302152 (295.07 KB)
[two-round fischlin-small output t31_n32] round1-received=1165519 (1.11 MB), round2-received=14942 (14.59 KB), total=1180461 (1.13 MB)
[two-round fischlin-small output t63_n64] round1-received=4628568 (4.41 MB), round2-received=30366 (29.65 KB), total=4658934 (4.44 MB)
[two-round fischlin-small output t127_n128] round1-received=18446485 (17.59 MB), round2-received=61215 (59.78 KB), total=18507700 (17.65 MB)
[two-round fischlin-small output t255_n256] round1-received=73660486 (70.25 MB), round2-received=123039 (120.16 KB), total=73783525 (70.37 MB)
[two-round fischlin-small output t511_n512] round1-received=294385391 (280.75 MB), round2-received=246687 (240.91 KB), total=294632078 (280.98 MB)
[two-round fischlin-large initiate t15_n16] proof=48290 (47.16 KB), broadcast=49973 (48.80 KB)
[two-round fischlin-large initiate t31_n32] proof=92322 (90.16 KB), broadcast=95557 (93.32 KB)
[two-round fischlin-large initiate t63_n64] proof=180386 (176.16 KB), broadcast=186725 (182.35 KB)
[two-round fischlin-large initiate t127_n128] proof=356600 (348.24 KB), broadcast=369149 (360.50 KB)
[two-round fischlin-large finalize t15_n16] round1-received=749595 (732.03 KB)
[two-round fischlin-large finalize t31_n32] round1-received=2962267 (2.83 MB)
[two-round fischlin-large finalize t63_n64] round1-received=11763675 (11.22 MB)
[two-round fischlin-large finalize t127_n128] round1-received=46881923 (44.71 MB)
[two-round fischlin-large output t15_n16] round1-received=749595 (732.03 KB), round2-received=7230 (7.06 KB), total=756825 (739.09 KB)
[two-round fischlin-large output t31_n32] round1-received=2962267 (2.83 MB), round2-received=14942 (14.59 KB), total=2977209 (2.84 MB)
[two-round fischlin-large output t63_n64] round1-received=11763675 (11.22 MB), round2-received=30366 (29.65 KB), total=11794041 (11.25 MB)
[two-round fischlin-large output t127_n128] round1-received=46881923 (44.71 MB), round2-received=61215 (59.78 KB), total=46943138 (44.77 MB)
[one-round abort t15_n16] report=259 bytes
[one-round abort t31_n32] report=259 bytes
[one-round abort t63_n64] report=259 bytes
[one-round abort t127_n128] report=259 bytes
[one-round abort t255_n256] report=259 bytes
[one-round abort t511_n512] report=259 bytes
[two-round abort t15_n16] report=259 bytes
[two-round abort t31_n32] report=259 bytes
[two-round abort t63_n64] report=259 bytes
[two-round abort t127_n128] report=259 bytes
[two-round abort t255_n256] report=259 bytes
[two-round abort t511_n512] report=259 bytes
```
