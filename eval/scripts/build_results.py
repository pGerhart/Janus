#!/usr/bin/env python3
"""Turns eval/bench_raw/ into eval/eval_results.md.

Run after eval/scripts/run_bench.sh. Parses the Criterion medians and the size lines
the benches print, and emits one table per variant.
"""

import os
import re
import sys

# Paths are anchored at the repository root so the script runs from anywhere.
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RAW = os.path.join(ROOT, "eval", "bench_raw")
OUT = os.path.join(ROOT, "eval", "eval_results.md")

TIME = re.compile(
    r"time:\s*\[\s*[\d.]+\s*(?:ns|µs|ms|s)\s+([\d.]+)\s*(ns|µs|ms|s)"
)
BENCH_ID = re.compile(r"^([A-Za-z0-9_]+(?:/[A-Za-z0-9_.=]+)+)$")
# Criterion keeps a short id on the same line as its timing and puts a long one
# on the line above, so both shapes have to be read.
INLINE_ID = re.compile(r"^([A-Za-z0-9_]+(?:/[A-Za-z0-9_.=]+)+)\s+time:")
SIZE_LINE = re.compile(r"^\[(.+?)\]\s+(.*)$")

# Link profiles for the end-to-end composition. The parties of a threshold
# deployment are servers on wired links, so these are datacentre and inter-region
# figures rather than consumer or mobile ones.
PROFILES = [
    ("One region", 1.0, 10_000.0),
    ("Cross-region", 25.0, 1_000.0),
    ("Intercontinental", 150.0, 1_000.0),
]

RUN_LINE = re.compile(
    r"^\[(janus[12]) (\w+) (t\d+_n\d+)\]\s+sent=([\d.]+) (\w+)\s+received=([\d.]+) (\w+)"
)
RSS = re.compile(r"peak_rss=([\d.]+) (\w+)")
ENC_SIZE = re.compile(r"^\[(t\d+_n\d+)\] verbose=(\d+) B compact=(\d+) B")
ECHO_BYTES = 32

# Two network rounds per broadcast round, which is what a broadcast costs in the
# optimistic case and with a trusted dealer. Janus-1 has one broadcast round,
# Janus-2 has two.
ROUNDS_PER_BROADCAST = 2
BROADCASTS = {"janus1": 1, "janus2": 2}
UNIT = {"B": 1.0, "KB": 1024.0, "MB": 1024.0**2, "GB": 1024.0**3}

SETS = [f"t{n - 1}_n{n}" for n in (16, 32, 64, 128, 256, 512)]

# The threshold sweep holds the committee at 256 and moves t alone.
TSWEEP = [f"t{t}_n256" for t in (16, 32, 64, 128, 192, 255)]

PRETTY = {p: f"({p[1:].split('_n')[0]}, {p.split('_n')[1]})" for p in SETS + TSWEEP}

COMPONENT_SET = "t63_n64"


def to_ms(value, unit):
    return {"s": value * 1000, "ms": value, "µs": value / 1000, "ns": value / 1e6}[unit]


def bytes_pretty(b):
    for unit, size in (("GB", 1024**3), ("MB", 1024**2), ("KB", 1024)):
        if b >= size:
            return f"{b / size:.1f} {unit}"
    return f"{int(b)} B"


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
        inline = INLINE_ID.match(stripped)
        if inline:
            hit = TIME.search(line)
            if hit:
                timings[inline.group(1)] = to_ms(float(hit.group(1)), hit.group(2))
            continue
        if BENCH_ID.match(stripped):
            current = stripped
            continue
        hit = TIME.search(line)
        if hit and current:
            timings[current] = to_ms(float(hit.group(1)), hit.group(2))
            current = None
    return timings, sizes


def parse_run_sizes(path):
    """Per-party sent and received bytes printed by the full-run benchmark."""
    out = {}
    if not os.path.exists(path):
        return out
    for line in open(path):
        m = RUN_LINE.match(line.strip())
        if m:
            proto, scheme, param, sent, su, recv, ru = m.groups()
            out[(proto, scheme, param)] = (
                float(sent) * UNIT[su],
                float(recv) * UNIT[ru],
            )
    return out


def transfer_ms(byte_count, mbit_per_s):
    """Time to move byte_count over a link, in milliseconds."""
    return byte_count * 8.0 / (mbit_per_s * 1e6) * 1000.0


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
        sys.exit(f"{RAW} not found, run eval/scripts/run_bench.sh first")

    one_run, one_sizes = parse(f"{RAW}/one_round_dkg_run.txt")
    one_comp, _ = parse(f"{RAW}/one_round_components.txt")
    two_run, two_sizes = parse(f"{RAW}/two_round_dkg_run.txt")
    two_comp, _ = parse(f"{RAW}/two_round_components.txt")
    abort, abort_sizes = parse(f"{RAW}/abort_path.txt")
    opt, _ = parse(f"{RAW}/optimizations.txt")
    par, _ = parse(f"{RAW}/parallel_scaling.txt")
    run, _ = parse(f"{RAW}/full_run.txt")
    enc, _ = parse(f"{RAW}/encoding_compare.txt")
    enc_sizes = {}
    enc_path = f"{RAW}/encoding_compare.txt"
    if os.path.exists(enc_path):
        for line in open(enc_path):
            m = ENC_SIZE.match(line.strip())
            if m:
                enc_sizes[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    peak = 0.0
    run_path = f"{RAW}/full_run.txt"
    if os.path.exists(run_path):
        for line in open(run_path):
            m = RSS.search(line)
            if m:
                peak = max(peak, float(m.group(1)) * UNIT.get(m.group(2), 0.0))
    run_sizes = parse_run_sizes(f"{RAW}/full_run.txt")

    info = machine()
    parts = []
    parts.append("# Janus benchmark results\n")
    parts.append(
        "Median runtimes per party, measured with `cargo bench` (Criterion). "
        "Regenerate with `eval/scripts/run_bench.sh` followed by "
        "`eval/scripts/build_results.py`.\n"
    )
    parts.append("| | |")
    parts.append("|---|---|")
    parts.append(f"| Machine | {info.get('cpu', '?')} |")
    if "instance" in info:
        parts.append(f"| Instance | {info['instance']} |")
    cores = info.get("nproc", "?")
    sockets = info.get("sockets")
    per_socket = info.get("cores_per_socket")
    per_core = info.get("threads_per_core")
    if sockets and per_socket and per_core:
        physical = int(sockets) * int(per_socket)
        smt = " with SMT" if int(per_core) > 1 else ", no SMT"
        cores = f"{cores} logical on {physical} physical{smt}"
    parts.append(f"| Cores | {cores} |")
    parts.append(f"| Memory | {info.get('mem', '?')} |")
    parts.append(f"| Architecture | {info.get('arch', '?')} |")
    if "avx512ifma" in info:
        parts.append(f"| AVX-512 IFMA | {info['avx512ifma']} |")
    if "dalek_backend" in info:
        parts.append(f"| Curve backend | `{info['dalek_backend']}` |")
    parts.append(f"| OS | {info.get('uname', '?')} |")
    parts.append(f"| Rust | {info.get('rustc', '?')} |")
    parts.append(f"| RUSTFLAGS | `{info.get('rustflags', '<unset>')}` |")
    parts.append(f"| Date | {info.get('date', '?')} |\n")
    parts.append(
        "`(t, n)` = (degree, parties), every setting n-out-of-n with `t = n - 1`. "
        "Previous run at `t = n/2` in `eval/archiv_16_08_2026/`.\n"
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

    parts.append("## One core against all cores\n")
    schemes = [
        ("Schnorr", "schnorr"),
        ("Fischlin small", "fischlin_small"),
        ("Fischlin large", "fischlin_large"),
    ]
    for label, prefix in schemes:
        body = table(
            par,
            [
                f"par_one_round_{prefix}/output_seq",
                f"par_one_round_{prefix}/output_par",
            ],
            ["Output, one core", "Output, all cores"],
        )
        if body:
            parts.append(f"### Janus-1, {label}\n")
            parts.append(body + "\n")
    for label, prefix in schemes:
        body = table(
            par,
            [
                f"par_two_round_{prefix}/finalize_seq",
                f"par_two_round_{prefix}/finalize_par",
                f"par_two_round_{prefix}/output_seq",
                f"par_two_round_{prefix}/output_par",
            ],
            [
                "Finalize, one core",
                "Finalize, all cores",
                "Output, one core",
                "Output, all cores",
            ],
        )
        if body:
            parts.append(f"### Janus-2, {label}\n")
            parts.append(body + "\n")

    parts.append("## Batch verification\n")
    for label, prefix in [("Schnorr", "schnorr"), ("Fischlin small", "fischlin_small")]:
        body = table(
            opt,
            [
                f"opt_one_round_{prefix}/proofverify_loop",
                f"opt_one_round_{prefix}/proofverify_batch",
            ],
            ["Proofs, one by one", "Proofs, batched"],
        )
        if body:
            parts.append(f"### {label}\n")
            parts.append(body + "\n")
    body = table(
        opt,
        [
            "opt_one_round_schnorr/sigverify_loop",
            "opt_one_round_schnorr/sigverify_batch",
        ],
        ["Signatures, one by one", "Signatures, batched"],
    )
    if body:
        parts.append("### Channel signatures\n")
        parts.append(body + "\n")

    t_comp, n_comp = COMPONENT_SET[1:].split("_n")
    parts.append(
        f"## Component breakdown, Fischlin small, (t={t_comp}, n={n_comp})\n"
    )
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
        ("Message decoding", "output/wire_decode_batch_n64", "finalize/wire_decode_batch_n64"),
    ]
    for label, one_key, two_key in pairs:
        left = fmt(one_comp.get(one_key)) if one_key else "--"
        right = fmt(two_comp.get(two_key)) if two_key else "--"
        parts.append(f"| {label} | {left} | {right} |")
    parts.append("")

    if run_sizes:
        parts.append("## End-to-end run\n")
        parts.append(
            "Link columns are `max(compute, transfer) + rounds * RTT`, with "
            f"{ROUNDS_PER_BROADCAST} rounds charged per broadcast round, so "
            f"{BROADCASTS['janus1'] * ROUNDS_PER_BROADCAST} for Janus-1 and "
            f"{BROADCASTS['janus2'] * ROUNDS_PER_BROADCAST} for Janus-2.\n"
        )
        if peak:
            parts.append(f"Peak resident memory: {bytes_pretty(peak)}.\n")
        header = "| (t, n) | Compute | Received | " + " | ".join(
            f"{name} ({int(rtt)} ms, {int(bw / 1000)} Gbit/s)" for name, rtt, bw in PROFILES
        ) + " |"
        for proto, label in [("janus1", "Janus-1"), ("janus2", "Janus-2")]:
            broadcasts = BROADCASTS[proto]
            rounds = broadcasts * ROUNDS_PER_BROADCAST
            for scheme, sname in [("schnorr", "Schnorr"), ("fischlin_small", "Fischlin small")]:
                rows = []
                for param in SETS:
                    key = (proto, scheme, param)
                    compute = run.get(f"full_run_{proto}_{scheme}/critical_path/{param}")
                    if compute is None or key not in run_sizes:
                        continue
                    _sent, received = run_sizes[key]
                    n = int(param.split("_n")[1])
                    received += ECHO_BYTES * (n - 1) * broadcasts
                    cells = [PRETTY[param], fmt(compute), bytes_pretty(received)]
                    for _n, rtt, bw in PROFILES:
                        t = transfer_ms(received, bw)
                        cells.append(fmt(max(compute, t) + rounds * rtt))
                    rows.append("| " + " | ".join(cells) + " |")
                if rows:
                    parts.append(f"### {label}, {sname}\n")
                    parts.append(header)
                    parts.append("|:---|" + "---:|" * (2 + len(PROFILES)))
                    parts.extend(rows)
                    parts.append("")

    if run_sizes:
        ns = sorted({int(p.split("_n")[1]) for p in SETS})
        parts.append("### What the second round of a broadcast costs\n")
        parts.append(
            "Included above once per broadcast round. Subtract a row to read a "
            "protocol charged one round per broadcast instead.\n"
        )
        parts.append(
            "| n | Extra bytes | "
            + " | ".join(name for name, _r, _b in PROFILES)
            + " |"
        )
        parts.append("|:---|---:|" + "---:|" * len(PROFILES))
        for n in ns:
            extra = ECHO_BYTES * (n - 1)
            cells = [str(n), bytes_pretty(extra)]
            for _name, rtt, bw in PROFILES:
                cells.append(fmt(rtt + transfer_ms(extra, bw)))
            parts.append("| " + " | ".join(cells) + " |")
        parts.append("")

    if run_sizes:
        parts.append("## Threshold sweep at a fixed committee\n")
        parts.append(
            "Committee fixed at 256, threshold alone moving, up to the n-out-of-n "
            "point the tables above are measured at.\n"
        )
        for proto, label in [("janus1", "Janus-1"), ("janus2", "Janus-2")]:
            broadcasts = BROADCASTS[proto]
            rounds = broadcasts * ROUNDS_PER_BROADCAST
            for scheme, sname in [("schnorr", "Schnorr"), ("fischlin_small", "Fischlin small")]:
                rows = []
                for param in TSWEEP:
                    key = (proto, f"{scheme}_tsweep", param)
                    compute = run.get(f"full_run_{proto}_{scheme}_tsweep/critical_path/{param}")
                    if compute is None or key not in run_sizes:
                        continue
                    _sent, received = run_sizes[key]
                    n = int(param.split("_n")[1])
                    received += ECHO_BYTES * (n - 1) * broadcasts
                    cells = [PRETTY.get(param, param), fmt(compute), bytes_pretty(received)]
                    for _n, rtt, bw in PROFILES:
                        t = transfer_ms(received, bw)
                        cells.append(fmt(max(compute, t) + rounds * rtt))
                    rows.append("| " + " | ".join(cells) + " |")
                if rows:
                    parts.append(f"### {label}, {sname}\n")
                    parts.append(header)
                    parts.append("|:---|" + "---:|" * (2 + len(PROFILES)))
                    parts.extend(rows)
                    parts.append("")

    if enc_sizes:
        parts.append("## An encoding we measured and did not adopt\n")
        parts.append(
            "Rebuilding the first-round commitments from the challenge and the "
            "responses instead of receiving them. Code in `eval/compact-encoding`.\n"
        )
        parts.append("| (t, n) | Proof sent | Proof rebuilt | Saved | Verify sent | Verify rebuilt | Cost |")
        parts.append("|:---|---:|---:|---:|---:|---:|---:|")
        savings, crossover = [], None
        for param in SETS:
            if param not in enc_sizes:
                continue
            vb, cb = enc_sizes[param]
            v = enc.get(f"encoding/verbose/{param}")
            c = enc.get(f"encoding/compact/{param}")
            if v is None or c is None:
                continue
            savings.append(100.0 * (vb - cb) / vb)
            if c > v:
                crossover = (vb - cb) * 8.0 / ((c - v) * 1000.0)
            parts.append(
                f"| {PRETTY[param]} | {bytes_pretty(vb)} | {bytes_pretty(cb)} | "
                f"{100.0 * (vb - cb) / vb:.0f}% | {fmt(v)} | {fmt(c)} | {c / v:.2f}x |"
            )
        parts.append("")
        if savings:
            span = (
                f"about {savings[0]:.0f} percent"
                if max(savings) - min(savings) < 2.0
                else f"between {min(savings):.0f} and {max(savings):.0f} percent"
            )
            slowest = min(bw for _n, _r, bw in PROFILES)
            if crossover and crossover < slowest:
                tail = (
                    f"worth the arithmetic only below roughly {crossover:.0f} "
                    "Mbit/s, under every link profile above.\n"
                )
            elif crossover:
                tail = (
                    f"worth the arithmetic below roughly {crossover:.0f} Mbit/s, "
                    "which reaches into the link profiles above.\n"
                )
            else:
                tail = "not worth the arithmetic at any measured setting.\n"
            parts.append(f"Saving {span}, and at the largest setting " + tail)

    parts.append("## Communication\n")
    parts.append("```")
    parts.extend(one_sizes)
    parts.extend(two_sizes)
    parts.extend(abort_sizes)
    parts.append("```")

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as handle:
        handle.write("\n".join(parts) + "\n")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
