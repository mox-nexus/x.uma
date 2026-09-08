#!/usr/bin/env python3
"""Render a D-066 artifact as Markdown.

Every number in the output names the artifact it came from, so a sentence
quoting it can cite `[bench:<artifact>]` and be checked against the file.

Usage:
    python3 scripts/bench_report.py bench/artifacts/<date>-<sha>.json [--output bench/RESULTS.md]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

FLOORS = ["floor_u32", "floor_option_string", "floor_flat_map"]
PATHS = {
    "python": ["crusty_evaluate_exact_hit", "crusty_evaluate_exact_miss", "crusty_config_evaluate_simple", "crusty_http_config_evaluate"],
    "typescript": ["crusty_exact_hit", "crusty_exact_miss", "crusty_config_evaluate_simple", "crusty_http_config_evaluate"],
}


def fmt_ns(ns: float) -> str:
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f} µs"
    return f"{ns:.0f} ns"


def verdict(a: dict, b: dict) -> str:
    ratio = a["median_ns"] / b["median_ns"] if b["median_ns"] else float("inf")
    g = max(a["gate"], b["gate"])
    if 1 / g <= ratio <= g:
        return f"{ratio:.2f}x, indistinguishable (G={g:.2f})"
    return f"{ratio:.2f}x (G={g:.2f})"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("artifact")
    ap.add_argument("--output")
    args = ap.parse_args()
    path = Path(args.artifact)
    art = json.loads(path.read_text())
    by = {(b["runtime"], b["name"]): b for b in art["benchmarks"]}

    lines = ["# Benchmark results", ""]
    lines.append(f"Artifact `{path.as_posix()}`, protocol {art['protocol']}, commit `{art['sha'][:8]}`"
                 f"{' (dirty)' if art['dirty'] else ''}, {art['date']}. {art['launches']} cold launches; "
                 f"per row the median of per-launch medians, the CV across launches, and the gate "
                 f"G = max(1.20, 1 + 3·CV). Two numbers whose ratio lies inside [1/G, G] are indistinguishable.")
    lines.append("")
    hw = art["hardware"]; tc = art["toolchains"]
    lines.append(f"Hardware: {hw['cpu']}, {hw['cores']} cores, {hw['os']} {hw['arch']}. "
                 f"Toolchains: {tc['rustc']}; {tc['wasm-pack']}; {tc['maturin']}; bun {tc['bun']}; {tc['python']}. "
                 f"Profile: {art['profile']}.")
    if art["gates_nothing"]:
        lines.append("")
        lines.append(f"**Worst CV {art['worst_cv']:.3f} exceeds the ceiling {art['cv_ceiling']}: "
                     "this artifact reports its noise and gates nothing (D-066).**")
    lines.append("")

    for runtime in ("python", "typescript"):
        rows = [b for b in art["benchmarks"] if b["runtime"] == runtime]
        if not rows:
            continue
        lines += [f"## {runtime}", "", "| Benchmark | Median | CV | Gate |", "|---|---|---|---|"]
        for b in rows:
            lines.append(f"| `{b['name']}` | {fmt_ns(b['median_ns'])} | {b['cv']:.3f} | {b['gate']:.2f} |")
        lines.append("")
        floors = [by[(runtime, f)] for f in FLOORS if (runtime, f) in by]
        paths = [by[(runtime, p)] for p in PATHS[runtime] if (runtime, p) in by]
        if floors and paths:
            lines += ["### Against the boundary floor", "",
                      "| Path | " + " | ".join(f"vs `{f['name']}`" for f in floors) + " |",
                      "|---|" + "---|" * len(floors)]
            for p in paths:
                lines.append(f"| `{p['name']}` | " + " | ".join(verdict(p, f) for f in floors) + " |")
            lines.append("")

    text = "\n".join(lines)
    if args.output:
        Path(args.output).write_text(text + "\n")
        print(f"bench_report: wrote {args.output}")
    else:
        print(text)


if __name__ == "__main__":
    main()
