"""Evaluate benchmarks for puma (Pure Python).

Measures the hot path: predicate evaluation, first-match-wins scanning,
miss-heavy workloads, and trace overhead.

Run: cd puma && uv run pytest tests/bench/test_bench_evaluate.py --benchmark-only
"""

from __future__ import annotations

from dataclasses import dataclass

from puma import (
    Action,
    And,
    ContainsMatcher,
    DataInput,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    Or,
    PrefixMatcher,
    RegexMatcher,
    SinglePredicate,
)


# ── Test fixtures ────────────────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class Ctx:
    value: str


class ValueInput(DataInput["Ctx"]):
    def get(self, ctx: Ctx) -> str | None:
        return ctx.value


def field_matcher(expected: str, action: str) -> FieldMatcher[Ctx, str]:
    return FieldMatcher(
        predicate=SinglePredicate(input=ValueInput(), matcher=ExactMatcher(expected)),
        on_match=Action(action),
    )


def prefix_field_matcher(prefix: str, action: str) -> FieldMatcher[Ctx, str]:
    return FieldMatcher(
        predicate=SinglePredicate(input=ValueInput(), matcher=PrefixMatcher(prefix)),
        on_match=Action(action),
    )


def regex_field_matcher(pattern: str, action: str) -> FieldMatcher[Ctx, str]:
    return FieldMatcher(
        predicate=SinglePredicate(input=ValueInput(), matcher=RegexMatcher(pattern)),
        on_match=Action(action),
    )


# ── Core scenarios ───────────────────────────────────────────────────────────


def test_bench_exact_match_hit_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(field_matcher("/api", "api_backend"),),
        on_no_match=Action("default"),
    )
    ctx = Ctx(value="/api")
    benchmark(matcher.evaluate, ctx)


def test_bench_exact_match_miss_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(field_matcher("/api", "api_backend"),),
        on_no_match=Action("default"),
    )
    ctx = Ctx(value="/other")
    benchmark(matcher.evaluate, ctx)


def test_bench_prefix_match_hit_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(prefix_field_matcher("/api/", "api"),),
        on_no_match=Action("default"),
    )
    ctx = Ctx(value="/api/v2/users/123")
    benchmark(matcher.evaluate, ctx)


def test_bench_regex_match_hit_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(regex_field_matcher(r"^/api/v\d+/users/\d+$", "user_route"),),
        on_no_match=Action("default"),
    )
    ctx = Ctx(value="/api/v2/users/12345")
    benchmark(matcher.evaluate, ctx)


def test_bench_regex_match_miss_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(regex_field_matcher(r"^/api/v\d+/users/\d+$", "user_route"),),
        on_no_match=Action("default"),
    )
    ctx = Ctx(value="/other/path")
    benchmark(matcher.evaluate, ctx)


# ── Predicate composition ───────────────────────────────────────────────────


def test_bench_predicate_and_all_match_evaluate(benchmark):
    pred = And(
        predicates=(
            SinglePredicate(input=ValueInput(), matcher=ContainsMatcher("hello")),
            SinglePredicate(input=ValueInput(), matcher=ContainsMatcher("world")),
        )
    )
    matcher = Matcher(
        matcher_list=(FieldMatcher(predicate=pred, on_match=Action("matched")),),
        on_no_match=None,
    )
    ctx = Ctx(value="hello world")
    benchmark(matcher.evaluate, ctx)


def test_bench_predicate_or_first_matches_evaluate(benchmark):
    pred = Or(
        predicates=(
            SinglePredicate(input=ValueInput(), matcher=ExactMatcher("hello")),
            SinglePredicate(input=ValueInput(), matcher=ExactMatcher("world")),
        )
    )
    matcher = Matcher(
        matcher_list=(FieldMatcher(predicate=pred, on_match=Action("matched")),),
        on_no_match=None,
    )
    ctx = Ctx(value="hello")
    benchmark(matcher.evaluate, ctx)


# ── Scaling: rule count ─────────────────────────────────────────────────────


def _make_n_rule_matcher(n: int, *, include_target: bool) -> Matcher[Ctx, str]:
    rules = tuple(
        field_matcher(f"rule_{i}", f"action_{i}") for i in range(n - (1 if include_target else 0))
    )
    if include_target:
        rules = (*rules, field_matcher("target", "found"))
    return Matcher(matcher_list=rules, on_no_match=Action("fallback"))


def test_bench_rule_count_10_last_match_evaluate(benchmark):
    matcher = _make_n_rule_matcher(10, include_target=True)
    ctx = Ctx(value="target")
    benchmark(matcher.evaluate, ctx)


def test_bench_rule_count_50_last_match_evaluate(benchmark):
    matcher = _make_n_rule_matcher(50, include_target=True)
    ctx = Ctx(value="target")
    benchmark(matcher.evaluate, ctx)


def test_bench_rule_count_100_last_match_evaluate(benchmark):
    matcher = _make_n_rule_matcher(100, include_target=True)
    ctx = Ctx(value="target")
    benchmark(matcher.evaluate, ctx)


def test_bench_rule_count_200_last_match_evaluate(benchmark):
    matcher = _make_n_rule_matcher(200, include_target=True)
    ctx = Ctx(value="target")
    benchmark(matcher.evaluate, ctx)


def test_bench_rule_count_10_miss_evaluate(benchmark):
    matcher = _make_n_rule_matcher(10, include_target=False)
    ctx = Ctx(value="no_match")
    benchmark(matcher.evaluate, ctx)


def test_bench_rule_count_100_miss_evaluate(benchmark):
    matcher = _make_n_rule_matcher(100, include_target=False)
    ctx = Ctx(value="no_match")
    benchmark(matcher.evaluate, ctx)


# ── Miss-heavy workload ─────────────────────────────────────────────────────


def test_bench_miss_heavy_10_rules_evaluate(benchmark):
    rules = tuple(
        field_matcher(f"/blocked/{i}", f"block_{i}") for i in range(10)
    )
    matcher = Matcher(matcher_list=rules, on_no_match=Action("allow"))
    ctx = Ctx(value="/api/v1/users")
    benchmark(matcher.evaluate, ctx)


# ── Trace overhead ───────────────────────────────────────────────────────────


def test_bench_trace_overhead_evaluate(benchmark):
    matcher = Matcher(
        matcher_list=(
            field_matcher("miss1", "a1"),
            field_matcher("miss2", "a2"),
            field_matcher("hit", "a3"),
        ),
        on_no_match=None,
    )
    ctx = Ctx(value="hit")
    benchmark(matcher.evaluate, ctx)


# NOTE: puma (pure Python) does not implement evaluate_with_trace.
# Trace overhead comparison is measured in puma-crusty (Rust bindings)
# where both evaluate() and evaluate_with_trace() are available.
