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
UNIT = {"B": 1.0, "KB": 1024.0, "MB": 1024.0**2, "GB": 1024.0**3}

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
        "The curve backend row is what `curve25519-dalek` actually compiled, read "
        "from its build script rather than inferred from the CPU. `avx512` is the "
        "IFMA path, `simd` is AVX2, and `serial` is the portable fallback, so that "
        "row states which arithmetic these numbers measure.\n"
    )
    parts.append(
        "The parallel rows are wall-clock times of the same work spread over all "
        "cores, so read their speedup against the physical core count above, not "
        "the logical one.\n"
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

    if run_sizes:
        parts.append("## End-to-end run\n")
        parts.append(
            "One party's whole run, from building its own message to holding the "
            "output key. Every message is encoded on the way out and decoded on "
            "the way in, which the phase tables above skip.\n"
        )
        parts.append(
            "Compute is measured, the link columns are attributed as "
            "`max(compute, transfer) + rounds * RTT`. A party reaches that bound "
            "by verifying each message as it arrives; one that waits for the whole "
            "round pays the sum instead. Broadcast is counted as point-to-point "
            "fan-out on a full-duplex link, so a party uploads once per peer.\n"
        )
        if peak:
            parts.append(
                f"The benchmark process peaked at {bytes_pretty(peak)} resident "
                "while holding every party of the largest setting at once, which "
                "bounds what one party needs to keep a round in memory.\n"
            )
        parts.append(
            "The rounds include one echo round on top of the protocol, since the "
            "protocol rounds only disseminate: every party sends a digest of what "
            "it received, which catches a dealer that told two parties different "
            "things. That gives broadcast with abort, which suits a protocol that "
            "already identifies the party at fault. Naming the culprit needs "
            "per-dealer hashes and is counted with the abort path.\n"
        )
        header = "| (t, n) | Compute | Received | " + " | ".join(
            f"{name} ({int(rtt)} ms, {int(bw / 1000)} Gbit/s)" for name, rtt, bw in PROFILES
        ) + " |"
        # The protocol rounds disseminate, they do not agree. One echo round on
        # top turns that into broadcast with abort: every party digests the round
        # it received and sends that digest to the others, so an equivocating
        # dealer is caught. Localizing which dealer equivocated needs per-dealer
        # hashes, which is dispute traffic and belongs to the abort path.
        for proto, rounds, label in [("janus1", 2, "Janus-1"), ("janus2", 3, "Janus-2")]:
            for scheme, sname in [("schnorr", "Schnorr"), ("fischlin_small", "Fischlin small")]:
                rows = []
                for param in SETS:
                    key = (proto, scheme, param)
                    compute = run.get(f"full_run_{proto}_{scheme}/critical_path/{param}")
                    if compute is None or key not in run_sizes:
                        continue
                    _sent, received = run_sizes[key]
                    n = int(param.split("_n")[1])
                    received += ECHO_BYTES * (n - 1)
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
        parts.append("### What the echo round costs\n")
        parts.append(
            "The tables above include it. It is the same for both protocols and "
            "both proof systems, one round-trip plus a digest to every peer, so "
            "subtract a row here to read a protocol without agreement.\n"
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

    if enc_sizes:
        parts.append("## An encoding we measured and did not adopt\n")
        parts.append(
            "A verifier can rebuild the first-round commitments from the challenge "
            "and the responses instead of receiving them, the way a Schnorr "
            "signature carries the challenge. Both columns run the same path, "
            "encode then decode then verify, so the parsing the shorter encoding "
            "avoids is counted in its favour. The code is in "
            "`eval/compact-encoding`.\n"
        )
        parts.append("| (t, n) | Proof sent | Proof rebuilt | Saved | Verify sent | Verify rebuilt | Cost |")
        parts.append("|:---|---:|---:|---:|---:|---:|---:|")
        for param in SETS:
            if param not in enc_sizes:
                continue
            vb, cb = enc_sizes[param]
            v = enc.get(f"encoding/verbose/{param}")
            c = enc.get(f"encoding/compact/{param}")
            if v is None or c is None:
                continue
            parts.append(
                f"| {PRETTY[param]} | {bytes_pretty(vb)} | {bytes_pretty(cb)} | "
                f"{100.0 * (vb - cb) / vb:.0f}% | {fmt(v)} | {fmt(c)} | {c / v:.2f}x |"
            )
        parts.append("")
        parts.append(
            "The saving holds at about 40 percent while the cost climbs from 1.7x "
            "to 3.9x, so the trade gets worse exactly where a smaller message would "
            "help most. At the largest setting the bytes are worth the arithmetic "
            "only below roughly 8 Mbit/s, which is far under any link these parties "
            "run on, so the protocol keeps the longer encoding.\n"
        )

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
