"""ReDoS safety demonstration for puma (Pure Python).

Python's `re` module uses a backtracking NFA engine — vulnerable to
catastrophic backtracking on pathological patterns like `(a+)+$`.

Pattern: `(a+)+$` against `"a" * N + "X"`
- Python `re`: O(2^N) — hangs at N=25+
- Rust `regex`: O(N) — microseconds even at N=100

SAFETY: Benchmarks capped at N=20. Do NOT increase without understanding
that N=25 can take seconds and N=30 may hang indefinitely.

Run: cd puma && uv run pytest tests/bench/test_bench_redos.py --benchmark-only
"""

from __future__ import annotations

from dataclasses import dataclass

from puma import (
    Action,
    DataInput,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    RegexMatcher,
    SinglePredicate,
)

# ── Fixtures ─────────────────────────────────────────────────────────────────

REDOS_PATTERN = r"(a+)+$"

# Safe equivalent that actually matches the pathological input
SAFE_PATTERN = r"^a+X$"


@dataclass(frozen=True, slots=True)
class Ctx:
    value: str


class ValueInput(DataInput["Ctx"]):
    def get(self, ctx: Ctx) -> str | None:
        return ctx.value


def _pathological_input(n: int) -> str:
    return "a" * n + "X"


# ── Raw regex matcher (ReDoS pattern) ────────────────────────────────────────


def test_bench_redos_regex_n5(benchmark):
    matcher = RegexMatcher(REDOS_PATTERN)
    value = _pathological_input(5)
    benchmark(matcher.matches, value)


def test_bench_redos_regex_n10(benchmark):
    matcher = RegexMatcher(REDOS_PATTERN)
    value = _pathological_input(10)
    benchmark(matcher.matches, value)


def test_bench_redos_regex_n15(benchmark):
    matcher = RegexMatcher(REDOS_PATTERN)
    value = _pathological_input(15)
    benchmark(matcher.matches, value)


def test_bench_redos_regex_n20(benchmark):
    """N=20 is the SAFE MAXIMUM for Python's backtracking re engine."""
    matcher = RegexMatcher(REDOS_PATTERN)
    value = _pathological_input(20)
    benchmark(matcher.matches, value)


# ── Full pipeline (ReDoS pattern through Matcher) ────────────────────────────


def test_bench_redos_pipeline_n10(benchmark):
    m = Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(
                    input=ValueInput(),
                    matcher=RegexMatcher(REDOS_PATTERN),
                ),
                on_match=Action("blocked"),
            ),
        ),
        on_no_match=Action("allowed"),
    )
    ctx = Ctx(value=_pathological_input(10))
    benchmark(m.evaluate, ctx)


def test_bench_redos_pipeline_n20(benchmark):
    """Full pipeline at N=20 — shows ReDoS cost compounds with pipeline overhead."""
    m = Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(
                    input=ValueInput(),
                    matcher=RegexMatcher(REDOS_PATTERN),
                ),
                on_match=Action("blocked"),
            ),
        ),
        on_no_match=Action("allowed"),
    )
    ctx = Ctx(value=_pathological_input(20))
    benchmark(m.evaluate, ctx)


# ── Safe regex for comparison ────────────────────────────────────────────────


def test_bench_redos_safe_regex_n10(benchmark):
    """Safe pattern — shows regex is fast when not pathological."""
    matcher = RegexMatcher(SAFE_PATTERN)
    value = _pathological_input(10)
    benchmark(matcher.matches, value)


def test_bench_redos_safe_regex_n20(benchmark):
    matcher = RegexMatcher(SAFE_PATTERN)
    value = _pathological_input(20)
    benchmark(matcher.matches, value)
