#!/usr/bin/env python3
"""Compare two D-066 artifacts, benchmark by benchmark.

For each benchmark present in both: the ratio old/new of the medians, the
gate G = max(gate_old, gate_new), and whether the ratio lies outside [1/G, G].
A change is a claim only where it does.

Usage:
    python3 scripts/bench_compare.py bench/artifacts/<old>.json bench/artifacts/<new>.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load(path: str) -> tuple[dict, dict[tuple[str, str], dict]]:
    art = json.loads(Path(path).read_text())
    return art, {(b["runtime"], b["name"]): b for b in art["benchmarks"]}


def fmt_ns(ns: float) -> str:
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f} µs"
    return f"{ns:.0f} ns"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("old")
    ap.add_argument("new")
    ap.add_argument("--only", help="substring filter on benchmark name")
    args = ap.parse_args()
    old_art, old = load(args.old)
    new_art, new = load(args.new)

    print(f"old `{old_art['sha'][:8]}` -> new `{new_art['sha'][:8]}`; ratio is old/new, so > 1 is faster now.\n")
    print("| Runtime | Benchmark | Old | New | old/new | G | Verdict |")
    print("|---|---|---|---|---|---|---|")
    for key in sorted(old.keys() & new.keys()):
        if args.only and args.only not in key[1]:
            continue
        a, b = old[key], new[key]
        ratio = a["median_ns"] / b["median_ns"] if b["median_ns"] else float("inf")
        g = max(a["gate"], b["gate"])
        if ratio > g:
            verdict = "faster"
        elif ratio < 1 / g:
            verdict = "SLOWER"
        else:
            verdict = "indistinguishable"
        print(f"| {key[0]} | `{key[1]}` | {fmt_ns(a['median_ns'])} | {fmt_ns(b['median_ns'])} | {ratio:.2f}x | {g:.2f} | {verdict} |")


if __name__ == "__main__":
    main()
