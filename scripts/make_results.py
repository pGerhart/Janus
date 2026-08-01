#!/usr/bin/env python3
"""Turns bench_raw/ into benches/bench_results.md.

Run after scripts/run_bench.sh. Parses the Criterion medians and the size lines
the benches print, and emits one table per variant.
"""

import os
import re
import sys

RAW = "bench_raw"
OUT = "benches/bench_results.md"

TIME = re.compile(
    r"time:\s*\[\s*[\d.]+\s*(?:ns|µs|ms|s)\s+([\d.]+)\s*(ns|µs|ms|s)"
)
BENCH_ID = re.compile(r"^([A-Za-z0-9_]+(?:/[A-Za-z0-9_.=]+)+)$")
SIZE_LINE = re.compile(r"^\[(.+?)\]\s+(.*)$")

SETS = ["t4_n16", "t8_n32", "t16_n64", "t32_n64", "t64_n128", "t128_n256", "t256_n512"]
PRETTY = {
    "t4_n16": "(4, 16)",
    "t8_n32": "(8, 32)",
    "t16_n64": "(16, 64)",
    "t32_n64": "(32, 64)",
    "t64_n128": "(64, 128)",
    "t128_n256": "(128, 256)",
    "t256_n512": "(256, 512)",
}


def to_ms(value, unit):
    return {"s": value * 1000, "ms": value, "µs": value / 1000, "ns": value / 1e6}[unit]


def fmt(ms):
    if ms is None:
        return "--"
    if ms >= 1000:
        return f"{ms / 1000:.2f} s"
    if ms >= 1:
        return f"{ms:.1f} ms"
    return f"{ms * 1000:.0f} µs"


def parse(path):
    """Returns (timings by benchmark id, list of printed size lines)."""
    timings, sizes, current = {}, [], None
    if not os.path.exists(path):
        return timings, sizes
    for line in open(path):
        stripped = line.strip()
        size = SIZE_LINE.match(stripped)
        if size:
            sizes.append(stripped)
            continue
        if BENCH_ID.match(stripped):
            current = stripped
            continue
        hit = TIME.search(line)
        if hit and current:
            timings[current] = to_ms(float(hit.group(1)), hit.group(2))
            current = None
    return timings, sizes


def machine():
    info = {}
    path = f"{RAW}/machine.txt"
    if os.path.exists(path):
        for line in open(path):
            if "=" in line:
                key, value = line.strip().split("=", 1)
                info[key] = value
    return info


def table(timings, groups, columns):
    """One row per parameter set, one column per measured operation."""
    head = "| (t, n) | " + " | ".join(columns) + " |"
    rule = "|:---|" + "---:|" * len(columns)
    rows = [head, rule]
    for param in SETS:
        cells = []
        present = False
        for group in groups:
            value = timings.get(f"{group}/{param}")
            if value is not None:
                present = True
            cells.append(fmt(value))
        if present:
            rows.append(f"| {PRETTY[param]} | " + " | ".join(cells) + " |")
    return "\n".join(rows) if len(rows) > 2 else ""


def main():
    if not os.path.isdir(RAW):
        sys.exit(f"{RAW}/ not found, run scripts/run_bench.sh first")

    one_run, one_sizes = parse(f"{RAW}/one_round_dkg_run.txt")
    one_comp, _ = parse(f"{RAW}/one_round_components.txt")
    two_run, two_sizes = parse(f"{RAW}/two_round_dkg_run.txt")
    two_comp, _ = parse(f"{RAW}/two_round_components.txt")
    abort, abort_sizes = parse(f"{RAW}/abort_path.txt")
    opt, _ = parse(f"{RAW}/optimizations.txt")

    info = machine()
    parts = []
    parts.append("# Janus benchmark results\n")
    parts.append(
        "Median runtimes per party, measured with `cargo bench` (Criterion). "
        "Regenerate with `scripts/run_bench.sh` followed by "
        "`scripts/make_results.py`.\n"
    )
    parts.append("| | |")
    parts.append("|---|---|")
    parts.append(f"| Machine | {info.get('cpu', '?')} |")
    if "instance" in info:
        parts.append(f"| Instance | {info['instance']} |")
    parts.append(f"| Cores | {info.get('nproc', '?')} |")
    parts.append(f"| Memory | {info.get('mem', '?')} |")
    parts.append(f"| Architecture | {info.get('arch', '?')} |")
    parts.append(f"| OS | {info.get('uname', '?')} |")
    parts.append(f"| Rust | {info.get('rustc', '?')} |")
    parts.append(f"| RUSTFLAGS | `{info.get('rustflags', '<unset>')}` |")
    parts.append(f"| Date | {info.get('date', '?')} |\n")
    parts.append(
        "The `curve25519-dalek` backend is chosen in its build script from the "
        "architecture and toolchain above, and the CPU features are detected at "
        "run time, so the numbers depend on those rows rather than on RUSTFLAGS.\n"
    )
    parts.append(
        "`(t, n)` = (threshold, parties). Initiate and output are the phases each "
        "party runs; output verifies the other `n - 1` proofs, so it grows "
        "quadratically in the committee size. The abort rows run only when a "
        "dealer sends a share that does not open its commitment, so they are off "
        "the honest path.\n"
    )

    parts.append("## Janus-1 (one round)\n")
    for label, prefix in [
        ("Schnorr", "schnorr"),
        ("Fischlin small", "fischlin_small"),
        ("Fischlin large", "fischlin_large"),
    ]:
        body = table(
            one_run,
            [f"one_round_initiate_{prefix}/initiate", f"one_round_output_{prefix}/output"],
            ["Initiate", "Output"],
        )
        if body:
            parts.append(f"### {label}\n")
            parts.append(body + "\n")

    parts.append("## Janus-2 (two rounds)\n")
    for label, prefix in [
        ("Schnorr", "schnorr"),
        ("Fischlin small", "fischlin_small"),
        ("Fischlin large", "fischlin_large"),
    ]:
        body = table(
            two_run,
            [
                f"two_round_initiate_{prefix}/initiate",
                f"two_round_finalize_{prefix}/finalize",
                f"two_round_output_{prefix}/output",
            ],
            ["Initiate", "Finalize", "Output"],
        )
        if body:
            parts.append(f"### {label}\n")
            parts.append(body + "\n")

    parts.append("## Identifiable abort\n")
    body = table(
        abort,
        [
            "abort_one_round_report_create/report_create",
            "abort_one_round_report_verify/report_verify",
            "abort_one_round_verify_worstcase/verify_n_minus_1",
            "abort_two_round_report_create/report_create",
            "abort_two_round_report_verify/report_verify",
            "abort_two_round_verify_worstcase/verify_n_minus_1",
        ],
        [
            "J1 build",
            "J1 verify",
            "J1 worst case",
            "J2 build",
            "J2 verify",
            "J2 worst case",
        ],
    )
    if body:
        parts.append(body + "\n")

    parts.append("## Parallel output and batching\n")
    for label, prefix in [("Schnorr", "schnorr"), ("Fischlin small", "fischlin_small")]:
        body = table(
            opt,
            [
                f"opt_one_round_{prefix}/output_seq",
                f"opt_one_round_{prefix}/output_parallel",
            ],
            ["Output sequential", "Output parallel"],
        )
        if body:
            parts.append(f"### {label}\n")
            parts.append(body + "\n")

    parts.append("## Component breakdown, Fischlin small, (t=32, n=64)\n")
    parts.append("| Component | One round | Two rounds |")
    parts.append("|:---|---:|---:|")
    pairs = [
        ("Proof generation", "initiate/poly_prove_fischlin", "initiate/decom_prove_fischlin"),
        ("Share encryption", "initiate/encrypt_shares_batch", "initiate/encrypt_shares_batch"),
        ("Proof verification, one", "output/poly_verify_fischlin_single", "finalize/decom_verify_fischlin_single"),
        ("Proof verification, all", "output/poly_verify_fischlin_batch_n64", "finalize/decom_verify_fischlin_batch_n64"),
        ("Share decryption", "output/decrypt_batch_n63", "finalize/decrypt_batch_n63"),
        ("Opening checks", "output/pedvss_opening_check_batch_n63", "finalize/pedvss_eval_check_batch_n63"),
        ("Key aggregation", "output/vk_aggregation_n64", "finalize/cstar_aggregate_and_eval_n64"),
        ("Message authentication", "output/wire_verify_batch_n64", "finalize/signature_verify_batch_n64"),
        ("Message decoding", "output/wire_decode_batch_n64", None),
    ]
    for label, one_key, two_key in pairs:
        left = fmt(one_comp.get(one_key)) if one_key else "--"
        right = fmt(two_comp.get(two_key)) if two_key else "--"
        parts.append(f"| {label} | {left} | {right} |")
    parts.append("")
    parts.append(
        "> Message authentication checks the signature over the bytes as received. "
        "Message decoding is the one-time cost of parsing those bytes into group "
        "elements, which dominates because every point needs a decompression.\n"
    )

    parts.append("## Communication\n")
    parts.append("```")
    parts.extend(one_sizes)
    parts.extend(two_sizes)
    parts.extend(abort_sizes)
    parts.append("```")

    os.makedirs("benches", exist_ok=True)
    with open(OUT, "w") as handle:
        handle.write("\n".join(parts) + "\n")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
