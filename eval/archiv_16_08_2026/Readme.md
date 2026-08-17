# Archived benchmark run

`eval/bench_raw/` and `eval/eval_results.md` as they stood before the switch to
n-out-of-n, archived 2026-08-16. Measured 2026-08-10 on an AWS `c8i.4xlarge`,
Intel Xeon 6975P-C, `avx512` dalek backend.

Every parameter set here is `t = n/2`: (8, 16), (16, 32), (32, 64), (64, 128),
(128, 256), (256, 512), components at (32, 64), sweep at t in {16, 32, 64, 128,
192} for n = 256. The current run uses `t = n - 1` throughout and extends the
sweep to 255.

Nothing regenerates this directory.
