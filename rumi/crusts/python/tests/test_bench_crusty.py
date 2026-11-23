"""Boundary cost of the PyO3 binding on the Claude hook path.

Every call here crosses from Python into Rust and back. What the numbers
isolate:
1. Compile cost — HookMatch objects in, a compiled matcher out
2. Evaluate cost — keyword arguments marshalled into a HookContext, one
   matcher walk, an owned action string back
3. Regex — Rust `regex`, linear time, behind the same boundary
4. Trace — the opt-in debugging path, against plain evaluate

Run:
  cd rumi/crusts/python
  maturin develop --release   # a debug build inverts every verdict drawn from this
  uv run pytest tests/test_bench_crusty.py --benchmark-only
"""

from __future__ import annotations

import pytest

from xuma_crust import HookMatch, HookMatcher, StringMatch

# ── Compile ───────────────────────────────────────────────────────────────────


def test_bench_crusty_compile_simple(benchmark):
    """Compile a single event match rule."""

    def go():
        return HookMatcher.compile([HookMatch(event="PreToolUse")], action="matched")

    benchmark(go)


def test_bench_crusty_compile_complex(benchmark):
    """Compile a multi-field rule with a regex."""

    def go():
        return HookMatcher.compile(
            [
                HookMatch(
                    event="PreToolUse",
                    tool_name=StringMatch.regex(r"^(Write|Edit|Bash)$"),
                    arguments=[("command", StringMatch.contains("rm"))],
                )
            ],
            action="blocked",
            fallback="allowed",
        )

    benchmark(go)


# ── Evaluate: exact match ─────────────────────────────────────────────────────


@pytest.fixture
def crusty_exact():
    return HookMatcher.compile([HookMatch(event="PreToolUse")], action="matched")


def test_bench_crusty_evaluate_exact_hit(benchmark, crusty_exact):
    benchmark(crusty_exact.evaluate, event="PreToolUse")


def test_bench_crusty_evaluate_exact_miss(benchmark, crusty_exact):
    benchmark(crusty_exact.evaluate, event="PostToolUse")


# ── Evaluate: regex ───────────────────────────────────────────────────────────


@pytest.fixture
def crusty_regex():
    return HookMatcher.compile(
        [HookMatch(tool_name=StringMatch.regex(r"^mcp__\w+__\w+$"))],
        action="matched",
    )


def test_bench_crusty_evaluate_regex_hit(benchmark, crusty_regex):
    benchmark(crusty_regex.evaluate, event="PreToolUse", tool_name="mcp__db__query")


def test_bench_crusty_evaluate_regex_miss(benchmark, crusty_regex):
    benchmark(crusty_regex.evaluate, event="PreToolUse", tool_name="Write")


# ── Evaluate: complex multi-field ─────────────────────────────────────────────


@pytest.fixture
def crusty_complex():
    return HookMatcher.compile(
        [
            HookMatch(
                event="PreToolUse",
                tool_name=StringMatch.prefix("mcp__"),
                arguments=[("command", StringMatch.contains("drop"))],
            )
        ],
        action="blocked",
        fallback="allowed",
    )


def test_bench_crusty_evaluate_complex_hit(benchmark, crusty_complex):
    benchmark(
        crusty_complex.evaluate,
        event="PreToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )


def test_bench_crusty_evaluate_complex_miss(benchmark, crusty_complex):
    """The event fails, so the AND short-circuits on its first predicate."""
    benchmark(
        crusty_complex.evaluate,
        event="PostToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )


# ── Trace ─────────────────────────────────────────────────────────────────────


def test_bench_crusty_trace(benchmark, crusty_complex):
    """Trace evaluation: the same walk, plus the step list crossing back."""
    benchmark(
        crusty_complex.trace,
        event="PreToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )
