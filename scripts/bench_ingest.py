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
    stddev_ns   DOUBLE,
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
            "stddev_ns": time.get("stddev"),
            "min_ns": time.get("min"),
            "max_ns": time.get("max"),
            "iterations": entry.get("iterations"),
        })
    return results


def parse_pytest_benchmark_json(data: dict[str, Any], variant: str) -> list[dict[str, Any]]:
    """Parse pytest-benchmark JSON output into normalized results."""
    results = []
    for bench in data.get("benchmarks", []):
        name = bench.get("name", "")
        # Convention: test_bench_{scenario}_{phase} or test_bench_{scenario}
        clean = name.removeprefix("test_bench_")
        parts = clean.rsplit("_", 1)

        # Try to separate scenario from phase
        known_phases = {"compile", "evaluate", "trace"}
        if len(parts) == 2 and parts[1] in known_phases:
            scenario, phase = parts
        else:
            scenario = clean
            phase = "evaluate"

        stats = bench.get("stats", {})
        # pytest-benchmark reports in seconds, convert to nanoseconds
        to_ns = 1_000_000_000
        results.append({
            "variant": variant,
            "scenario": scenario,
            "phase": phase,
            "mean_ns": stats.get("mean", 0) * to_ns,
            "stddev_ns": (stats.get("stddev") or 0) * to_ns,
            "min_ns": stats.get("min", 0) * to_ns,
            "max_ns": stats.get("max", 0) * to_ns,
            "iterations": stats.get("iterations"),
        })
    return results


def parse_mitata_json(data: dict[str, Any], variant: str) -> list[dict[str, Any]]:
    """Parse mitata JSON output into normalized results."""
    results = []
    for bench in data.get("benchmarks", []):
        name = bench.get("name", "")
        parts = name.rsplit("/", 1)
        if len(parts) == 2:
            scenario = parts[0]
            phase = parts[1]
        else:
            scenario = parts[0]
            phase = "evaluate"

        stats = bench.get("stats", {})
        results.append({
            "variant": variant,
            "scenario": scenario,
            "phase": phase,
            "mean_ns": stats.get("avg", 0),
            "stddev_ns": stats.get("p75", 0) - stats.get("p25", 0),  # IQR as proxy
            "min_ns": stats.get("min", 0),
            "max_ns": stats.get("max", 0),
            "iterations": stats.get("samples"),
        })
    return results


FILE_PARSERS: dict[str, tuple[str, Any]] = {
    "rust.json": ("rumi", parse_divan_json),
    "python.json": ("puma", parse_pytest_benchmark_json),
    "typescript.json": ("bumi", parse_mitata_json),
    "xuma-crust-python.json": ("xuma-crust-python", parse_pytest_benchmark_json),
    "xuma-crust-wasm.json": ("xuma-crust-wasm", parse_mitata_json),
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

    for json_file in sorted(json_files):
        fname = json_file.name
        if fname not in FILE_PARSERS:
            click.echo(f"  Skipping unknown file: {fname}")
            continue

        variant, parser = FILE_PARSERS[fname]
        data = json.loads(json_file.read_text())

        # Parsers have different signatures
        if fname == "rust.json":
            rows = parser(data)
        else:
            rows = parser(data, variant)

        for row in rows:
            con.execute(
                """INSERT INTO bench_results
                   (run_id, variant, scenario, phase, mean_ns, stddev_ns, min_ns, max_ns, iterations)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                [
                    run_id,
                    row["variant"],
                    row["scenario"],
                    row["phase"],
                    row["mean_ns"],
                    row["stddev_ns"],
                    row["min_ns"],
                    row["max_ns"],
                    row["iterations"],
                ],
            )
            total += 1

        click.echo(f"  Ingested {len(rows)} results from {fname} ({variant})")

    con.close()
    click.echo(f"\nRun #{run_id}: {total} results ingested into {db}")


if __name__ == "__main__":
    main()
