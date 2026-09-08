#!/usr/bin/env python3
"""One cold launch of both crust benchmark suites, normalised to rows.

Each runtime's suite runs in a fresh process and reports JSON; this script
turns that into `{runtime, name, median_ns}` rows and writes them to
`bench/raw/launch-<n>.json`. `bench_artifact.py` runs it LAUNCHES times and
reads the noise floor off the spread (D-066).

Usage:
    python3 scripts/bench_collect.py [--launch N] [--no-build]
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY = ROOT / "rumi/crusts/python"
WASM = ROOT / "rumi/crusts/wasm"
RAW = ROOT / "bench/raw"

PY_SUITES = ["tests/test_bench_crusty.py", "tests/test_bench_config.py", "tests/test_bench_floor.py"]
WASM_SUITES = ["bench/crusty.bench.ts", "bench/config.bench.ts", "bench/floor.bench.ts"]


def sh(cmd: list[str], cwd: Path, env: dict[str, str] | None = None) -> str:
    merged = {**os.environ, **(env or {})}
    res = subprocess.run(cmd, cwd=cwd, env=merged, capture_output=True, text=True)
    if res.returncode != 0:
        sys.stderr.write(f"bench_collect: {' '.join(cmd)} failed in {cwd}\n{res.stderr[-2000:]}\n")
        sys.exit(1)
    return res.stdout


def build() -> None:
    sh(["maturin", "develop", "--release", "--features", "bench"], PY)
    sh(["wasm-pack", "build", "--target", "web", "--", "--features", "bench"], WASM)


def launch_python() -> list[dict]:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "py.json"
        sh(["uv", "run", "pytest", *PY_SUITES, "--benchmark-only", "--benchmark-disable-gc", "--benchmark-warmup=on",
            "-q", f"--benchmark-json={out}"], PY)
        data = json.loads(out.read_text())
    rows = []
    for b in data["benchmarks"]:
        name = b["name"].removeprefix("test_bench_")
        rows.append({"runtime": "python", "name": name, "median_ns": b["stats"]["median"] * 1e9})
    return rows


def launch_wasm() -> list[dict]:
    rows = []
    for suite in WASM_SUITES:
        stdout = sh(["bun", "run", suite], WASM, {"BENCH_JSON": "1"})
        data = json.loads(stdout[stdout.index("{"):])
        for b in data["benchmarks"]:
            runs = [r for r in b["runs"] if r.get("stats")]
            if not runs:
                continue
            rows.append({"runtime": "typescript", "name": b["alias"], "median_ns": runs[0]["stats"]["p50"]})
    return rows


def launch(n: int) -> Path:
    rows = launch_python() + launch_wasm()
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / f"launch-{n}.json"
    path.write_text(json.dumps({"launch": n, "rows": rows}, indent=1) + "\n")
    return path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--launch", type=int, default=0)
    ap.add_argument("--no-build", action="store_true")
    args = ap.parse_args()
    if not args.no_build and not os.environ.get("NO_BUILD"):
        build()
    path = launch(args.launch)
    rows = json.loads(path.read_text())["rows"]
    print(f"bench_collect: {len(rows)} rows -> {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
