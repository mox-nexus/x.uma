"""Head-to-head: puma (pure Python) vs puma-crusty (PyO3 Rust bindings).

Compares identical workloads through both implementations to isolate:
1. FFI overhead — crusty's boundary crossing cost
2. Regex engine — Python `re` (backtracking) vs Rust `regex` (linear time)
3. Compile cost — Python dataclass construction vs Rust struct compilation

Run:
  cd rumi/crusts/python
  maturin develop
  uv run pytest tests/test_bench_crusty.py --benchmark-only
"""

from __future__ import annotations

from dataclasses import dataclass

import pytest

from puma_crusty import HookMatch, HookMatcher, StringMatch

# Also import xuma (pure Python) for comparison
from xuma import (
    Action,
    ContainsMatcher,
    DataInput,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    PrefixMatcher,
    RegexMatcher,
    SinglePredicate,
)


# ── Pure Python fixtures (mirror the hook matcher domain) ─────────────────────


@dataclass(frozen=True, slots=True)
class HookCtx:
    event: str
    tool_name: str | None = None
    command: str | None = None


class EventInput(DataInput["HookCtx"]):
    def get(self, ctx: HookCtx) -> str | None:
        return ctx.event


class ToolNameInput(DataInput["HookCtx"]):
    def get(self, ctx: HookCtx) -> str | None:
        return ctx.tool_name


class CommandInput(DataInput["HookCtx"]):
    def get(self, ctx: HookCtx) -> str | None:
        return ctx.command


# ── Compile benchmarks ────────────────────────────────────────────────────────


def test_bench_crusty_compile_simple(benchmark):
    """Crusty: compile a single event match rule."""

    def go():
        return HookMatcher.compile([HookMatch(event="PreToolUse")], action="matched")

    benchmark(go)


def test_bench_puma_compile_simple(benchmark):
    """Puma: construct equivalent matcher tree."""

    def go():
        return Matcher(
            matcher_list=(
                FieldMatcher(
                    predicate=SinglePredicate(input=EventInput(), matcher=ExactMatcher("PreToolUse")),
                    on_match=Action("matched"),
                ),
            ),
            on_no_match=None,
        )

    benchmark(go)


def test_bench_crusty_compile_complex(benchmark):
    """Crusty: compile a multi-field rule with regex."""

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


def test_bench_puma_compile_complex(benchmark):
    """Puma: construct equivalent complex matcher tree."""

    def go():
        from xuma import And

        return Matcher(
            matcher_list=(
                FieldMatcher(
                    predicate=And(
                        predicates=(
                            SinglePredicate(input=EventInput(), matcher=ExactMatcher("PreToolUse")),
                            SinglePredicate(
                                input=ToolNameInput(),
                                matcher=RegexMatcher(r"^(Write|Edit|Bash)$"),
                            ),
                            SinglePredicate(
                                input=CommandInput(),
                                matcher=ContainsMatcher("rm"),
                            ),
                        )
                    ),
                    on_match=Action("blocked"),
                ),
            ),
            on_no_match=Action("allowed"),
        )

    benchmark(go)


# ── Evaluate benchmarks (exact match) ────────────────────────────────────────


@pytest.fixture
def crusty_exact():
    return HookMatcher.compile([HookMatch(event="PreToolUse")], action="matched")


@pytest.fixture
def puma_exact():
    return Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(input=EventInput(), matcher=ExactMatcher("PreToolUse")),
                on_match=Action("matched"),
            ),
        ),
        on_no_match=None,
    )


def test_bench_crusty_evaluate_exact_hit(benchmark, crusty_exact):
    """Crusty: evaluate exact match (hit)."""
    benchmark(crusty_exact.evaluate, event="PreToolUse")


def test_bench_puma_evaluate_exact_hit(benchmark, puma_exact):
    """Puma: evaluate exact match (hit)."""
    ctx = HookCtx(event="PreToolUse")
    benchmark(puma_exact.evaluate, ctx)


def test_bench_crusty_evaluate_exact_miss(benchmark, crusty_exact):
    """Crusty: evaluate exact match (miss)."""
    benchmark(crusty_exact.evaluate, event="PostToolUse")


def test_bench_puma_evaluate_exact_miss(benchmark, puma_exact):
    """Puma: evaluate exact match (miss)."""
    ctx = HookCtx(event="PostToolUse")
    benchmark(puma_exact.evaluate, ctx)


# ── Evaluate benchmarks (regex — the key differentiator) ──────────────────────


@pytest.fixture
def crusty_regex():
    return HookMatcher.compile(
        [HookMatch(tool_name=StringMatch.regex(r"^mcp__\w+__\w+$"))],
        action="matched",
    )


@pytest.fixture
def puma_regex():
    return Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(
                    input=ToolNameInput(),
                    matcher=RegexMatcher(r"^mcp__\w+__\w+$"),
                ),
                on_match=Action("matched"),
            ),
        ),
        on_no_match=None,
    )


def test_bench_crusty_evaluate_regex_hit(benchmark, crusty_regex):
    """Crusty: evaluate regex match (hit) — Rust linear-time regex."""
    benchmark(crusty_regex.evaluate, event="PreToolUse", tool_name="mcp__db__query")


def test_bench_puma_evaluate_regex_hit(benchmark, puma_regex):
    """Puma: evaluate regex match (hit) — Python backtracking re."""
    ctx = HookCtx(event="PreToolUse", tool_name="mcp__db__query")
    benchmark(puma_regex.evaluate, ctx)


def test_bench_crusty_evaluate_regex_miss(benchmark, crusty_regex):
    """Crusty: evaluate regex match (miss)."""
    benchmark(crusty_regex.evaluate, event="PreToolUse", tool_name="Write")


def test_bench_puma_evaluate_regex_miss(benchmark, puma_regex):
    """Puma: evaluate regex match (miss)."""
    ctx = HookCtx(event="PreToolUse", tool_name="Write")
    benchmark(puma_regex.evaluate, ctx)


# ── Evaluate benchmarks (complex multi-field) ────────────────────────────────


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


@pytest.fixture
def puma_complex():
    from xuma import And

    return Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=And(
                    predicates=(
                        SinglePredicate(input=EventInput(), matcher=ExactMatcher("PreToolUse")),
                        SinglePredicate(input=ToolNameInput(), matcher=PrefixMatcher("mcp__")),
                        SinglePredicate(input=CommandInput(), matcher=ContainsMatcher("drop")),
                    )
                ),
                on_match=Action("blocked"),
            ),
        ),
        on_no_match=Action("allowed"),
    )


def test_bench_crusty_evaluate_complex_hit(benchmark, crusty_complex):
    """Crusty: evaluate complex multi-field match (hit)."""
    benchmark(
        crusty_complex.evaluate,
        event="PreToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )


def test_bench_puma_evaluate_complex_hit(benchmark, puma_complex):
    """Puma: evaluate complex multi-field match (hit)."""
    ctx = HookCtx(event="PreToolUse", tool_name="mcp__db__exec", command="DROP TABLE users")
    benchmark(puma_complex.evaluate, ctx)


def test_bench_crusty_evaluate_complex_miss(benchmark, crusty_complex):
    """Crusty: evaluate complex multi-field match (miss — event fails)."""
    benchmark(
        crusty_complex.evaluate,
        event="PostToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )


def test_bench_puma_evaluate_complex_miss(benchmark, puma_complex):
    """Puma: evaluate complex multi-field match (miss — event fails)."""
    ctx = HookCtx(event="PostToolUse", tool_name="mcp__db__exec", command="DROP TABLE users")
    benchmark(puma_complex.evaluate, ctx)


# ── Trace overhead (crusty only — puma doesn't have trace) ────────────────────


def test_bench_crusty_trace(benchmark, crusty_complex):
    """Crusty: trace evaluation (opt-in debugging)."""
    benchmark(
        crusty_complex.trace,
        event="PreToolUse",
        tool_name="mcp__db__exec",
        arguments={"command": "DROP TABLE users"},
    )
