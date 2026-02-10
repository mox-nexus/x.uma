//! Compile benchmarks — config → matcher construction.
//!
//! Measures the one-time cost of building matchers from configuration.
//! Includes StringMatcher compilation (especially regex) and scaling scenarios.

use rumi::prelude::*;

fn main() {
    divan::main();
}

// ═══════════════════════════════════════════════════════════════════════════════
// StringMatcher construction
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn compile_exact(bencher: divan::Bencher) {
    bencher.bench_local(|| StringMatcher::exact("/api/v1/users", false));
}

#[divan::bench]
fn compile_prefix(bencher: divan::Bencher) {
    bencher.bench_local(|| StringMatcher::prefix("/api/", false));
}

#[divan::bench]
fn compile_contains_case_insensitive(bencher: divan::Bencher) {
    bencher.bench_local(|| StringMatcher::contains("Content-Type", true));
}

#[divan::bench]
fn compile_regex_simple(bencher: divan::Bencher) {
    bencher.bench_local(|| StringMatcher::regex(r"^/api/v\d+/users$"));
}

#[divan::bench]
fn compile_regex_complex(bencher: divan::Bencher) {
    bencher.bench_local(|| {
        StringMatcher::regex(r"^/api/v[1-3]/(users|orders|products)/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$")
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// Matcher tree construction at scale
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
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

#[divan::bench(args = [1, 10, 50, 100, 200])]
fn compile_n_exact_rules(bencher: divan::Bencher, n: usize) {
    bencher.bench_local(|| {
        let rules: Vec<FieldMatcher<Ctx, String>> = (0..n)
            .map(|i| {
                FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(ExactMatcher::new(format!("/route/{i}"))),
                    )),
                    OnMatch::Action(format!("action_{i}")),
                )
            })
            .collect();
        Matcher::new(rules, None)
    });
}

#[divan::bench(args = [1, 10, 50, 100, 200])]
fn compile_n_regex_rules(bencher: divan::Bencher, n: usize) {
    bencher.bench_local(|| {
        let rules: Vec<FieldMatcher<Ctx, String>> = (0..n)
            .map(|i| {
                FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(StringMatcher::regex(&format!(r"^/route/{i}/\d+$")).unwrap()),
                    )),
                    OnMatch::Action(format!("action_{i}")),
                )
            })
            .collect();
        Matcher::new(rules, None)
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// RadixTree construction
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [10, 50, 100, 500])]
fn compile_radix_tree(bencher: divan::Bencher, n: usize) {
    let routes: Vec<String> = (0..n).map(|i| format!("/api/v1/route/{i}")).collect();

    bencher.bench_local(|| {
        let mut tree = RadixTree::new();
        for (i, route) in routes.iter().enumerate() {
            tree.insert(route, format!("action_{i}"));
        }
        tree
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// Validation (depth check)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [2, 8, 16, 30])]
fn validate_nested_depth(bencher: divan::Bencher, depth: usize) {
    // Build a matcher nested to the given depth
    let mut current = Matcher::<Ctx, String>::new(
        vec![FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("leaf")),
            )),
            OnMatch::Action("action".to_string()),
        )],
        None,
    );

    for _ in 0..depth.saturating_sub(2) {
        current = Matcher::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("x")),
                )),
                OnMatch::Matcher(Box::new(current)),
            )],
            None,
        );
    }

    bencher.bench_local(|| current.validate());
}
