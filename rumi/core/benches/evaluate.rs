//! Evaluate benchmarks — the hot path.
//!
//! Measures: SinglePredicate, Predicate (And/Or/Not), Matcher first-match-wins,
//! miss-heavy workloads, and trace overhead.

use rumi::prelude::*;

fn main() {
    divan::main();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test fixtures
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

fn field_matcher(expected: &str, action: &str) -> FieldMatcher<Ctx, String> {
    FieldMatcher::new(
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new(expected)),
        )),
        OnMatch::Action(action.to_string()),
    )
}

fn prefix_field_matcher(prefix: &str, action: &str) -> FieldMatcher<Ctx, String> {
    FieldMatcher::new(
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(PrefixMatcher::new(prefix)),
        )),
        OnMatch::Action(action.to_string()),
    )
}

fn regex_field_matcher(pattern: &str, action: &str) -> FieldMatcher<Ctx, String> {
    FieldMatcher::new(
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(StringMatcher::regex(pattern).unwrap()),
        )),
        OnMatch::Action(action.to_string()),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core scenario: exact match (baseline)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn exact_match_hit(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![field_matcher("/api", "api_backend")],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = Ctx {
        value: "/api".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn exact_match_miss(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![field_matcher("/api", "api_backend")],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = Ctx {
        value: "/other".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core scenario: prefix match
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn prefix_match_hit(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![prefix_field_matcher("/api/", "api")],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = Ctx {
        value: "/api/v2/users/123".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core scenario: regex match
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn regex_match_hit(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![regex_field_matcher(r"^/api/v\d+/users/\d+$", "user_route")],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = Ctx {
        value: "/api/v2/users/12345".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn regex_match_miss(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![regex_field_matcher(r"^/api/v\d+/users/\d+$", "user_route")],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = Ctx {
        value: "/other/path".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core scenario: predicate AND (multiple conditions)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn predicate_and_all_match(bencher: divan::Bencher) {
    let pred = Predicate::And(vec![
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ContainsMatcher::new("hello")),
        )),
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ContainsMatcher::new("world")),
        )),
    ]);
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            pred,
            OnMatch::Action("matched".to_string()),
        )],
        None,
    );
    let ctx = Ctx {
        value: "hello world".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn predicate_and_first_fails(bencher: divan::Bencher) {
    let pred = Predicate::And(vec![
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ContainsMatcher::new("nope")),
        )),
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ContainsMatcher::new("world")),
        )),
    ]);
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            pred,
            OnMatch::Action("matched".to_string()),
        )],
        None,
    );
    let ctx = Ctx {
        value: "hello world".to_string(),
    };

    // Short-circuit: first fails → returns false immediately
    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core scenario: predicate OR
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn predicate_or_first_matches(bencher: divan::Bencher) {
    let pred = Predicate::Or(vec![
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("hello")),
        )),
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("world")),
        )),
    ]);
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            pred,
            OnMatch::Action("matched".to_string()),
        )],
        None,
    );
    let ctx = Ctx {
        value: "hello".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scaling: rule count (first-match-wins scan cost)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [1, 10, 50, 100, 200])]
fn rule_count_last_match(bencher: divan::Bencher, n: usize) {
    let mut rules: Vec<FieldMatcher<Ctx, String>> = (0..n - 1)
        .map(|i| field_matcher(&format!("rule_{i}"), &format!("action_{i}")))
        .collect();
    rules.push(field_matcher("target", "found"));

    let matcher = Matcher::new(rules, None);
    let ctx = Ctx {
        value: "target".to_string(),
    };

    // Worst case: match is at the end → scans all rules
    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench(args = [1, 10, 50, 100, 200])]
fn rule_count_miss(bencher: divan::Bencher, n: usize) {
    let rules: Vec<FieldMatcher<Ctx, String>> = (0..n)
        .map(|i| field_matcher(&format!("rule_{i}"), &format!("action_{i}")))
        .collect();

    let matcher = Matcher::new(rules, Some(OnMatch::Action("fallback".to_string())));
    let ctx = Ctx {
        value: "no_match".to_string(),
    };

    // Full scan: nothing matches
    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scaling: predicate depth (nesting overhead)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [1, 4, 8, 16])]
fn depth_nested_and(bencher: divan::Bencher, depth: usize) {
    fn build_nested(depth: usize) -> Predicate<Ctx> {
        if depth <= 1 {
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ContainsMatcher::new("x")),
            ))
        } else {
            Predicate::And(vec![
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ContainsMatcher::new("x")),
                )),
                build_nested(depth - 1),
            ])
        }
    }

    let pred = build_nested(depth);
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            pred,
            OnMatch::Action("matched".to_string()),
        )],
        None,
    );
    let ctx = Ctx {
        value: "x".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scaling: AND width (many conditions)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench(args = [1, 5, 10, 20])]
fn and_width(bencher: divan::Bencher, width: usize) {
    let preds: Vec<Predicate<Ctx>> = (0..width)
        .map(|_| {
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ContainsMatcher::new("x")),
            ))
        })
        .collect();

    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            Predicate::And(preds),
            OnMatch::Action("matched".to_string()),
        )],
        None,
    );
    let ctx = Ctx {
        value: "x".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Miss-heavy workload (production pattern: 90% misses)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn miss_heavy_10_rules(bencher: divan::Bencher) {
    let rules: Vec<FieldMatcher<Ctx, String>> = (0..10)
        .map(|i| field_matcher(&format!("/blocked/{i}"), &format!("block_{i}")))
        .collect();

    let matcher = Matcher::new(rules, Some(OnMatch::Action("allow".to_string())));

    // 90% of contexts are misses (allowed through)
    let miss_ctx = Ctx {
        value: "/api/v1/users".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&miss_ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Trace overhead: evaluate vs evaluate_with_trace
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn trace_overhead_evaluate(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![
            field_matcher("miss1", "a1"),
            field_matcher("miss2", "a2"),
            field_matcher("hit", "a3"),
        ],
        None,
    );
    let ctx = Ctx {
        value: "hit".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn trace_overhead_with_trace(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![
            field_matcher("miss1", "a1"),
            field_matcher("miss2", "a2"),
            field_matcher("hit", "a3"),
        ],
        None,
    );
    let ctx = Ctx {
        value: "hit".to_string(),
    };

    bencher.bench_local(|| matcher.evaluate_with_trace(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Nested matcher evaluation
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn nested_matcher_2_levels(bencher: divan::Bencher) {
    let inner = Matcher::new(vec![field_matcher("hello", "inner_action")], None);

    let outer = Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(PrefixMatcher::new("hel")),
            )),
            OnMatch::Matcher(Box::new(inner)),
        )],
        None,
    );
    let ctx = Ctx {
        value: "hello".to_string(),
    };

    bencher.bench_local(|| outer.evaluate(&ctx));
}
