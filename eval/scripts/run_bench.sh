#!/usr/bin/env bash
# Runs every benchmark suite and records the machine context alongside the raw
# Criterion output. Produces eval/bench_raw/, which build_results.py turns into
# eval/eval_results.md.
#
# Usage:  ./scripts/run_bench.sh
# The (256,512) points take several minutes each, so run this under tmux.
#
# target-cpu=native is set on purpose. curve25519-dalek decides its backend in
# build.rs from the compile-time target features: without avx512ifma and
# avx512vl it falls back to the AVX2 path, and only with them does it build the
# AVX-512 backend. Export RUSTFLAGS yourself to override this.

set -euo pipefail
cd "$(dirname "$0")/../.."

export RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}"

OUT=eval/bench_raw
mkdir -p "$OUT"

# Record what the numbers depend on. Without this the run is not reproducible.
{
    echo "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "uname=$(uname -a)"
    echo "arch=$(uname -m)"
    echo "rustc=$(rustc -V)"
    echo "cargo=$(cargo -V)"
    echo "rustflags=${RUSTFLAGS:-<unset>}"
    echo "nproc=$(nproc 2>/dev/null || sysctl -n hw.ncpu)"
    if command -v lscpu >/dev/null; then
        echo "cpu=$(lscpu | sed -n 's/^Model name: *//p' | head -1)"
        # Physical cores and threads per core, because a parallel speedup read
        # against vCPUs alone is misleading on an SMT machine.
        echo "sockets=$(lscpu | sed -n 's/^Socket(s): *//p' | head -1)"
        echo "cores_per_socket=$(lscpu | sed -n 's/^Core(s) per socket: *//p' | head -1)"
        echo "threads_per_core=$(lscpu | sed -n 's/^Thread(s) per core: *//p' | head -1)"
        # These two decide whether the AVX-512 backend is reachable at all.
        if lscpu | grep -q avx512ifma && lscpu | grep -q avx512vl; then
            echo "avx512ifma=yes"
        else
            echo "avx512ifma=no"
        fi
    else
        echo "cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
    fi
    if command -v free >/dev/null; then
        echo "mem=$(free -g | awk '/^Mem:/{print $2" GB"}')"
    else
        echo "mem=$(( $(sysctl -n hw.memsize 2>/dev/null || echo 0) / 1073741824 )) GB"
    fi
    if [ -f /sys/devices/virtual/dmi/id/product_name ]; then
        echo "instance=$(cat /sys/devices/virtual/dmi/id/product_name)"
    fi
} > "$OUT/machine.txt"

echo "== machine =="
cat "$OUT/machine.txt"

# The whole gate battery first: numbers from a failing tree are worthless.
echo "== verifying the tree =="
cargo build --release --workspace --all-targets --locked

# Which arithmetic backend was actually compiled, straight from the library's
# own build script. lscpu only says what the CPU could do, this says what runs.
# Selecting avx512 also emits simd as a fallback, so rank rather than take the
# last line: avx512 beats simd beats serial.
cfgs=$(grep -h 'curve25519_dalek_backend' target/release/build/curve25519-dalek-*/output 2>/dev/null)
if echo "$cfgs" | grep -q 'avx512'; then
    backend=avx512
elif echo "$cfgs" | grep -q 'simd'; then
    backend=simd
elif echo "$cfgs" | grep -q 'serial'; then
    backend=serial
else
    backend=unknown
fi
echo "dalek_backend=${backend:-unknown}" >> "$OUT/machine.txt"
echo "== dalek backend: ${backend:-unknown} =="
if [ "$backend" != "avx512" ]; then
    echo "   note: not the AVX-512 path. Expected on ARM or without the target-cpu flag."
fi
cargo test --release --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings \
    -A clippy::needless_range_loop \
    -A clippy::too_many_arguments \
    -A clippy::type_complexity

for suite in one_round_dkg_run one_round_components two_round_dkg_run \
             two_round_components abort_path optimizations full_run; do
    echo "== $suite =="
    cargo bench --bench "$suite" --locked 2>&1 | tee "$OUT/$suite.txt"
done

# The alternative encoding lives in its own crate, so it needs its own line.
echo "== encoding_compare =="
cargo bench -p compact-encoding --locked 2>&1 | tee "$OUT/encoding_compare.txt"

echo "== results =="
python3 eval/scripts/build_results.py

echo
echo "done. the results file is eval/eval_results.md"
