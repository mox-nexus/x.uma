#!/usr/bin/env python3
"""The protocol behind every published number (D-066).

LAUNCHES cold launches of both crust suites at one commit on a clean tree.
Per benchmark: the median of each launch, the median of those medians, and
the coefficient of variation across launches. The gate is
G = max(1.20, 1 + 3·CV): two numbers whose ratio lies inside [1/G, G] are
indistinguishable. The artifact carries the commit, hardware, toolchains and
build profile it was measured under, and is refused on a dirty tree.

Usage:
    python3 scripts/bench_artifact.py [--launches 10] [--allow-dirty]
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
from collections import defaultdict
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bench_collect  # noqa: E402

ROOT = bench_collect.ROOT
ARTIFACTS = ROOT / "bench/artifacts"
CV_CEILING = 0.15


def out(cmd: list[str], cwd: Path = ROOT) -> str:
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=False).stdout.strip()


def hardware() -> dict:
    cpu = out(["sysctl", "-n", "machdep.cpu.brand_string"]) if platform.system() == "Darwin" else platform.processor()
    return {"os": f"{platform.system()} {platform.release()}", "arch": platform.machine(),
            "cpu": cpu, "cores": os.cpu_count()}


def toolchains() -> dict:
    return {
        "rustc": out(["rustc", "-V"]),
        "wasm-pack": out(["wasm-pack", "-V"]),
        "maturin": out(["maturin", "-V"]),
        "bun": out(["bun", "-v"]),
        "python": out(["uv", "run", "python", "-V"], bench_collect.PY),
    }


def gate(cv: float) -> float:
    return max(1.20, 1 + 3 * cv)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--launches", type=int, default=10)
    ap.add_argument("--allow-dirty", action="store_true")
    args = ap.parse_args()

    dirty = bool(out(["git", "status", "--porcelain", "--untracked-files=no"]))
    sha = out(["git", "rev-parse", "HEAD"])
    if dirty and not args.allow_dirty:
        sys.stderr.write("bench_artifact: the tree is dirty; an artifact names one commit. Commit or stash first.\n")
        sys.exit(1)

    bench_collect.build()
    series: dict[tuple[str, str], list[float]] = defaultdict(list)
    for n in range(args.launches):
        path = bench_collect.launch(n)
        for row in json.loads(path.read_text())["rows"]:
            series[(row["runtime"], row["name"])].append(row["median_ns"])
        print(f"launch {n + 1}/{args.launches} done", flush=True)

    benchmarks = []
    for (runtime, name), medians in sorted(series.items()):
        mean = statistics.fmean(medians)
        cv = statistics.stdev(medians) / mean if len(medians) > 1 and mean else 0.0
        benchmarks.append({
            "runtime": runtime, "name": name, "launches": len(medians),
            "medians_ns": [round(m, 1) for m in medians],
            "median_ns": round(statistics.median(medians), 1),
            "cv": round(cv, 4), "gate": round(gate(cv), 3),
        })

    worst = max((b["cv"] for b in benchmarks), default=0.0)
    artifact = {
        "protocol": "D-066",
        "date": date.today().isoformat(),
        "sha": sha, "dirty": dirty,
        "launches": args.launches,
        "profile": "release",
        "hardware": hardware(),
        "toolchains": toolchains(),
        "cv_ceiling": CV_CEILING,
        "worst_cv": round(worst, 4),
        "gates_nothing": worst > CV_CEILING,
        "benchmarks": benchmarks,
    }

    if dirty:
        path = bench_collect.RAW / "artifact-dirty.json"
    else:
        ARTIFACTS.mkdir(parents=True, exist_ok=True)
        path = ARTIFACTS / f"{artifact['date']}-{sha[:8]}.json"
    path.write_text(json.dumps(artifact, indent=1) + "\n")
    print(f"bench_artifact: {len(benchmarks)} benchmarks, worst CV {worst:.3f} -> {path.relative_to(ROOT)}")
    if artifact["gates_nothing"]:
        print(f"bench_artifact: worst CV exceeds {CV_CEILING}; publish the CV and gate nothing (D-066)")


if __name__ == "__main__":
    main()
