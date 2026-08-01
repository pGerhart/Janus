#!/usr/bin/env bash
# Runs every benchmark suite and records the machine context alongside the raw
# Criterion output. Produces bench_raw/, which make_results.py turns into
# benches/bench_results.md.
#
# Usage:  ./scripts/run_bench.sh
# The (256,512) points take several minutes each, so run this under tmux.
#
# target-cpu=native is set on purpose. curve25519-dalek decides its backend in
# build.rs from the compile-time target features: without avx512ifma and
# avx512vl it falls back to the AVX2 path, and only with them does it build the
# AVX-512 backend. Export RUSTFLAGS yourself to override this.

set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}"

OUT=bench_raw
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
cargo build --release --all-targets --locked
cargo test --release --locked
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings \
    -A clippy::needless_range_loop \
    -A clippy::too_many_arguments \
    -A clippy::type_complexity

for suite in one_round_dkg_run one_round_components two_round_dkg_run \
             two_round_components abort_path optimizations; do
    echo "== $suite =="
    cargo bench --bench "$suite" --locked 2>&1 | tee "$OUT/$suite.txt"
done

echo
echo "done. now run:  python3 scripts/make_results.py"
echo "then bring back benches/bench_results.md"
