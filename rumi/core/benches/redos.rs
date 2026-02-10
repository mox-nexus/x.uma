//! ReDoS safety demonstration.
//!
//! Proves that Rust's `regex` crate (linear time, RE2 semantics) is immune
//! to catastrophic backtracking on pathological patterns.
//!
//! Pattern: `(a+)+$` against `"a" * N + "X"`
//!
//! - Backtracking engines (Python `re`, JS `RegExp`): O(2^N) — hangs at N=25+
//! - Rust `regex` crate: O(N) — microseconds even at N=100
//!
//! This benchmark runs up to N=100. Python/TS benchmarks cap at N=20 for safety.

use rumi::prelude::*;

fn main() {
    divan::main();
}

/// The classic ReDoS pattern: nested quantifier with anchor.
const REDOS_PATTERN: &str = r"(a+)+$";

/// Build a pathological input: N 'a's followed by 'X' (forces full backtrack attempt).
fn pathological_input(n: usize) -> String {
    "a".repeat(n) + "X"
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReDoS via InputMatcher (StringMatcher::Regex)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [10, 20, 25, 30, 50, 100])]
fn redos_regex_matcher(bencher: divan::Bencher, n: usize) {
    let matcher = StringMatcher::regex(REDOS_PATTERN).unwrap();
    let input = MatchingData::String(pathological_input(n));

    bencher.bench_local(|| matcher.matches(&input));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReDoS via full Matcher pipeline (DataInput → InputMatcher → action)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
struct Ctx {
    value: String,
}

#[derive(Debug)]
struct ValueInput;

impl DataInput<Ctx> for ValueInput {
    fn get(&self, ctx: &Ctx) -> MatchingData {
        MatchingData::String(ctx.value.clone())
    }
}

#[divan::bench(args = [10, 20, 50, 100])]
fn redos_full_pipeline(bencher: divan::Bencher, n: usize) {
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(StringMatcher::regex(REDOS_PATTERN).unwrap()),
            )),
            OnMatch::Action("blocked".to_string()),
        )],
        Some(OnMatch::Action("allowed".to_string())),
    );

    let ctx = Ctx {
        value: pathological_input(n),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Safe regex for comparison (shows regex compile cost is amortized)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [10, 50, 100])]
fn safe_regex_match(bencher: divan::Bencher, n: usize) {
    let matcher = StringMatcher::regex(r"^a+X$").unwrap();
    let input = MatchingData::String(pathological_input(n));

    // This DOES match (unlike the ReDoS pattern which doesn't due to the $)
    bencher.bench_local(|| matcher.matches(&input));
}
