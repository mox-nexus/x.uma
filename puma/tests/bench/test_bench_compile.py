"""Compile benchmarks for puma (Pure Python).

Measures matcher construction cost: string matcher creation,
predicate tree building, and HTTP route compilation.

Run: cd puma && uv run pytest tests/bench/test_bench_compile.py --benchmark-only
"""

from __future__ import annotations

from dataclasses import dataclass

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
    SuffixMatcher,
)

# ── Fixtures ─────────────────────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class Ctx:
    value: str


class ValueInput(DataInput["Ctx"]):
    def get(self, ctx: Ctx) -> str | None:
        return ctx.value


# ── StringMatcher construction ───────────────────────────────────────────────


def test_bench_compile_exact_compile(benchmark):
    benchmark(ExactMatcher, "/api/v1/users")


def test_bench_compile_prefix_compile(benchmark):
    benchmark(PrefixMatcher, "/api/")


def test_bench_compile_suffix_compile(benchmark):
    benchmark(SuffixMatcher, ".json")


def test_bench_compile_contains_case_insensitive_compile(benchmark):
    benchmark(ContainsMatcher, "Content-Type", True)


def test_bench_compile_regex_simple_compile(benchmark):
    benchmark(RegexMatcher, r"^/api/v\d+/users$")


def test_bench_compile_regex_complex_compile(benchmark):
    benchmark(
        RegexMatcher,
        r"^/api/v[1-3]/(users|orders|products)/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$",
    )


# ── Matcher tree construction at scale ───────────────────────────────────────


def _build_n_exact_rules(n: int) -> Matcher[Ctx, str]:
    rules = tuple(
        FieldMatcher(
            predicate=SinglePredicate(
                input=ValueInput(),
                matcher=ExactMatcher(f"/route/{i}"),
            ),
            on_match=Action(f"action_{i}"),
        )
        for i in range(n)
    )
    return Matcher(matcher_list=rules, on_no_match=None)


def _build_n_regex_rules(n: int) -> Matcher[Ctx, str]:
    rules = tuple(
        FieldMatcher(
            predicate=SinglePredicate(
                input=ValueInput(),
                matcher=RegexMatcher(f"^/route/{i}/\\d+$"),
            ),
            on_match=Action(f"action_{i}"),
        )
        for i in range(n)
    )
    return Matcher(matcher_list=rules, on_no_match=None)


def test_bench_compile_10_exact_rules_compile(benchmark):
    benchmark(_build_n_exact_rules, 10)


def test_bench_compile_50_exact_rules_compile(benchmark):
    benchmark(_build_n_exact_rules, 50)


def test_bench_compile_100_exact_rules_compile(benchmark):
    benchmark(_build_n_exact_rules, 100)


def test_bench_compile_200_exact_rules_compile(benchmark):
    benchmark(_build_n_exact_rules, 200)


def test_bench_compile_10_regex_rules_compile(benchmark):
    benchmark(_build_n_regex_rules, 10)


def test_bench_compile_50_regex_rules_compile(benchmark):
    benchmark(_build_n_regex_rules, 50)


def test_bench_compile_100_regex_rules_compile(benchmark):
    benchmark(_build_n_regex_rules, 100)
