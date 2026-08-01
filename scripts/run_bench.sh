#!/usr/bin/env bash
# Runs every benchmark suite and records the machine context alongside the raw
# Criterion output. Produces bench_raw/, which make_results.py turns into
# benches/bench_results.md.
#
# Usage:  ./scripts/run_bench.sh
# The (256,512) points take several minutes each, so run this under tmux.
#
# No RUSTFLAGS needed: curve25519-dalek picks its backend in build.rs from the
# target architecture (x86_64 gets avx512 on rustc 1.89 or newer) and detects
# the CPU features at run time. The architecture and toolchain are recorded
# below because they, not RUSTFLAGS, decide which backend runs.

set -euo pipefail
cd "$(dirname "$0")/.."

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
        echo "flags_avx512=$(lscpu | grep -c avx512 || true)"
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
