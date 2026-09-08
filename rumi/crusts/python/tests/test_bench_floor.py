"""The boundary's floor: one no-op crossing per shape (D-066).

Each row is the least a call of that shape can cost. A real path's number is
read against the floor of the shape it uses, not against zero.

Run:
  cd rumi/crusts/python
  maturin develop --release --features bench
  uv run pytest tests/test_bench_floor.py --benchmark-only
"""

from __future__ import annotations

import pytest

native = pytest.importorskip("xuma_crust.xuma_crust")
if not hasattr(native, "boundary_floor_u32"):
    pytest.skip("built without --features bench", allow_module_level=True)

# Six fields: the width of a hook context.
FLAT = {
    "event": "PreToolUse",
    "tool_name": "Bash",
    "session_id": "s-1",
    "cwd": "/work",
    "git_branch": "main",
    "command": "ls",
}


def test_bench_floor_u32(benchmark):
    benchmark(native.boundary_floor_u32)


def test_bench_floor_option_string(benchmark):
    benchmark(native.boundary_floor_option_string)


def test_bench_floor_cached_string(benchmark):
    benchmark(native.boundary_floor_cached_string)


def test_bench_floor_flat_map(benchmark):
    benchmark(native.boundary_floor_flat_map, FLAT)


PACKED = "\x1f".join(FLAT.values())


def test_bench_floor_packed(benchmark):
    benchmark(native.boundary_floor_packed, PACKED)
