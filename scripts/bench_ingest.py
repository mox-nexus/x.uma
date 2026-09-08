#!/usr/bin/env python3
"""Ingest benchmark JSON results into DuckDB.

Usage:
    uv run scripts/bench_ingest.py [--db bench/x_uma_bench.duckdb] [--notes "initial baseline"]

Reads raw JSON from bench/raw/ (one file per variant) and inserts into DuckDB.
Each run creates a new bench_runs entry with the current commit SHA.
"""

from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import click
import duckdb


DB_DEFAULT = "bench/x_uma_bench.duckdb"
RAW_DIR = Path("bench/raw")

SCHEMA = """
CREATE TABLE IF NOT EXISTS bench_runs (
    id          INTEGER PRIMARY KEY,
    commit_sha  VARCHAR NOT NULL,
    timestamp   TIMESTAMP NOT NULL,
    machine     VARCHAR,
    notes       VARCHAR
);

CREATE TABLE IF NOT EXISTS bench_results (
    run_id      INTEGER NOT NULL REFERENCES bench_runs(id),
    variant     VARCHAR NOT NULL,
    scenario    VARCHAR NOT NULL,
    phase       VARCHAR NOT NULL,
    mean_ns     DOUBLE NOT NULL,
    -- Median is stored because it is the robust statistic: on Apple Silicon
    -- the 24MHz mach timebase quantises perf_counter at ~41.7ns, and rows the
    -- harness settles at 1 iteration carry that as pure instrument noise.
    median_ns   DOUBLE,
    -- Dispersion is NOT comparable across harnesses: divan and pytest-benchmark
    -- report a true standard deviation, mitata reports no stddev at all so we
    -- store its interquartile range. A variance gate that compares the two is
    -- comparing different statistics, so the kind travels with the number.
    dispersion_ns   DOUBLE,
    dispersion_kind VARCHAR,
    min_ns      DOUBLE,
    max_ns      DOUBLE,
    iterations  BIGINT,
    PRIMARY KEY (run_id, variant, scenario, phase)
);
"""


def get_commit_sha() -> str:
    """Get current git commit SHA."""
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def get_machine() -> str:
    """Get machine identifier."""
    import platform

    return f"{platform.node()}/{platform.machine()}"


def create_run(con: duckdb.DuckDBPyConnection, notes: str | None) -> int:
    """Create a new benchmark run entry, return its ID."""
    con.execute(SCHEMA)

    max_id = con.execute("SELECT COALESCE(MAX(id), 0) FROM bench_runs").fetchone()[0]
    run_id = max_id + 1

    con.execute(
        "INSERT INTO bench_runs (id, commit_sha, timestamp, machine, notes) VALUES (?, ?, ?, ?, ?)",
        [run_id, get_commit_sha(), datetime.now(timezone.utc), get_machine(), notes],
    )
    return run_id


def parse_divan_json(data: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Parse divan JSON output into normalized results."""
    results = []
    for entry in data:
        name = entry.get("name", "")
        # divan format: "bench_name" or "group::bench_name"
        parts = name.rsplit("::", 1)
        if len(parts) == 2:
            scenario = parts[0]
            phase = parts[1]
        else:
            scenario = parts[0]
            phase = "evaluate"

        time = entry.get("time", {})
        results.append({
            "variant": "rumi",
            "scenario": scenario,
            "phase": phase,
            "mean_ns": time.get("mean", 0),
            "median_ns": time.get("median"),
            "dispersion_ns": time.get("stddev"),
            "dispersion_kind": "stddev",
            "min_ns": time.get("min"),
            "max_ns": time.get("max"),
            "iterations": entry.get("iterations"),
        })
    return results


KNOWN_PHASES = {"compile", "evaluate", "trace", "load"}


def split_name(raw: str, variants: dict[str, str]) -> tuple[str, str, str]:
    """Split a benchmark name into (variant, scenario, phase).

    The leading token names the variant and the rest is kept intact as the
    scenario, so rows from different variants join on scenario equality.
    Stripping the variant prefix into the scenario is what once made the
    report's comparison section return no rows.
    """
    clean = raw.removeprefix("test_bench_")
    head, _, rest = clean.partition("_")
    if head in variants and rest:
        variant, scenario = variants[head], rest
    else:
        # Unprefixed name: fall back to the file's sole declared variant.
        variant, scenario = next(iter(variants.values())), clean
    phase = next((t for t in scenario.split("_") if t in KNOWN_PHASES), "evaluate")
    return variant, scenario, phase


def parse_pytest_benchmark_json(
    data: dict[str, Any], variants: dict[str, str]
) -> list[dict[str, Any]]:
    """Parse pytest-benchmark JSON output into normalized results."""
    results = []
    for bench in data.get("benchmarks", []):
        variant, scenario, phase = split_name(bench.get("name", ""), variants)
        stats = bench.get("stats", {})
        # pytest-benchmark reports in seconds, convert to nanoseconds
        to_ns = 1_000_000_000
        results.append({
            "variant": variant,
            "scenario": scenario,
            "phase": phase,
            "mean_ns": stats.get("mean", 0) * to_ns,
            "median_ns": (stats.get("median") or 0) * to_ns or None,
            "dispersion_ns": (stats.get("stddev") or 0) * to_ns,
            "dispersion_kind": "stddev",
            "min_ns": stats.get("min", 0) * to_ns,
            "max_ns": stats.get("max", 0) * to_ns,
            "iterations": stats.get("iterations"),
        })
    return results


def parse_mitata_json(
    data: dict[str, Any], variants: dict[str, str]
) -> list[dict[str, Any]]:
    """Parse mitata JSON output into normalized results."""
    results = []
    for bench in data.get("benchmarks", []):
        # mitata 1.0.34 emits `alias` (not `name`) and nests stats one level down
        # under `runs[]`. This parser read `bench["name"]` and `bench["stats"]`,
        # which are both absent — so it produced zero rows against real mitata
        # output. Shape confirmed by running mitata and reading what it printed.
        runs = bench.get("runs") or [{}]
        stats = runs[0].get("stats") or {}
        raw_name = bench.get("alias") or runs[0].get("name", "")
        variant, scenario, phase = split_name(raw_name, variants)
        results.append({
            "variant": variant,
            "scenario": scenario,
            "phase": phase,
            "mean_ns": stats.get("avg", 0),
            "median_ns": stats.get("p50"),
            # mitata reports no standard deviation. This is an IQR and is labelled
            # as one — it was previously stored in a column called stddev_ns and
            # printed as "Stddev", so any variance comparison put an IQR next to a
            # real standard deviation and called them the same thing.
            "dispersion_ns": stats.get("p75", 0) - stats.get("p25", 0),
            "dispersion_kind": "iqr",
            "min_ns": stats.get("min", 0),
            "max_ns": stats.get("max", 0),
            "iterations": stats.get("samples"),
        })
    return results


# filename -> ({leading name token: variant}, parser).
FILE_PARSERS: dict[str, tuple[dict[str, str], Any]] = {
    "rust.json": ({"rumi": "rumi"}, parse_divan_json),
    "xuma-crust-python.json": ({"crusty": "xuma-crust-python"}, parse_pytest_benchmark_json),
    "xuma-crust-wasm.json": ({"crusty": "xuma-crust-wasm"}, parse_mitata_json),
}


@click.command()
@click.option("--db", default=DB_DEFAULT, help="DuckDB database path")
@click.option("--notes", default=None, help="Notes for this benchmark run")
@click.option("--raw-dir", default=str(RAW_DIR), help="Directory with raw JSON files")
def main(db: str, notes: str | None, raw_dir: str) -> None:
    """Ingest benchmark results into DuckDB."""
    raw_path = Path(raw_dir)

    if not raw_path.exists():
        click.echo(f"Raw directory {raw_path} does not exist", err=True)
        sys.exit(1)

    json_files = list(raw_path.glob("*.json"))
    if not json_files:
        click.echo(f"No JSON files found in {raw_path}", err=True)
        sys.exit(1)

    con = duckdb.connect(db)
    run_id = create_run(con, notes)
    total = 0

    unknown: list[str] = []
    for json_file in sorted(json_files):
        fname = json_file.name
        if fname not in FILE_PARSERS:
            unknown.append(fname)
            continue

        variants, parser = FILE_PARSERS[fname]
        data = json.loads(json_file.read_text())

        # Parsers have different signatures
        if fname == "rust.json":
            rows = parser(data)
        else:
            rows = parser(data, variants)

        for row in rows:
            con.execute(
                """INSERT INTO bench_results
                   (run_id, variant, scenario, phase, mean_ns, median_ns, dispersion_ns,
                    dispersion_kind, min_ns, max_ns, iterations)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                [
                    run_id,
                    row["variant"],
                    row["scenario"],
                    row["phase"],
                    row["mean_ns"],
                    row["median_ns"],
                    row["dispersion_ns"],
                    row["dispersion_kind"],
                    row["min_ns"],
                    row["max_ns"],
                    row["iterations"],
                ],
            )
            total += 1

        found = sorted({r["variant"] for r in rows})
        click.echo(f"  Ingested {len(rows)} results from {fname} ({', '.join(found)})")

    con.close()

    # An ingest that recognises nothing used to print its skips and exit 0, so a
    # renamed or unwritten file produced a successful-looking run with no data in
    # it — and the report built from that run would be stale rows under today's
    # commit SHA. Nothing downstream can tell that apart from a clean run, so it
    # has to fail here.
    if unknown:
        click.echo(f"Unrecognised files in {raw_path}: {', '.join(unknown)}", err=True)
    if total == 0:
        click.echo(
            f"No results ingested from {raw_path}. Expected one or more of: "
            f"{', '.join(sorted(FILE_PARSERS))}",
            err=True,
        )
        sys.exit(1)
    if unknown:
        sys.exit(1)

    click.echo(f"\nRun #{run_id}: {total} results ingested into {db}")


if __name__ == "__main__":
    main()
