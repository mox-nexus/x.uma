//! Predicate — Boolean expressions over `DataInputs`
//!
//! Predicates combine [`DataInput`] and [`InputMatcher`] to create boolean
//! conditions that can be composed with AND/OR/NOT.

use crate::{DataInput, InputMatcher, MatchingData, PredicateTrace};
use std::fmt::Debug;

/// A single predicate: combines a [`DataInput`] with an [`InputMatcher`].
///
/// This is where domain-specific (`DataInput`) meets domain-agnostic (`InputMatcher`).
/// The `SinglePredicate` extracts data from the context using the input, then
/// passes it to the matcher for evaluation.
///
/// # INV (Dijkstra): None → false
///
/// If the `DataInput` returns [`MatchingData::None`], the predicate evaluates to `false`.
/// This is a critical invariant that simplifies reasoning about matcher behavior.
///
/// # Example
///
/// ```ignore
/// let predicate = SinglePredicate::new(
///     Box::new(HeaderInput::new("content-type")),
///     Box::new(ContainsMatcher::new("json")),
/// );
/// let result = predicate.evaluate(&request);
/// ```
pub struct SinglePredicate<Ctx> {
    input: Box<dyn DataInput<Ctx>>,
    matcher: Box<dyn InputMatcher>,
}

impl<Ctx> SinglePredicate<Ctx> {
    /// Create a new single predicate from a `DataInput` and `InputMatcher`.
    #[must_use]
    pub fn new(input: Box<dyn DataInput<Ctx>>, matcher: Box<dyn InputMatcher>) -> Self {
        Self { input, matcher }
    }

    /// Get a reference to the input.
    #[must_use]
    pub fn input(&self) -> &dyn DataInput<Ctx> {
        &*self.input
    }

    /// Get a reference to the matcher.
    #[must_use]
    pub fn matcher(&self) -> &dyn InputMatcher {
        &*self.matcher
    }

    /// Evaluate this predicate against the given context.
    ///
    /// # Returns
    ///
    /// - `true` if the input produces data and the matcher matches
    /// - `false` if the input returns `None` (INV: None → false)
    /// - `false` if the matcher doesn't match
    pub fn evaluate(&self, ctx: &Ctx) -> bool {
        let data = self.input.get(ctx);
        match data {
            MatchingData::None => false, // INV: None → false
            _ => self.matcher.matches(&data),
        }
    }

    /// Evaluate with full trace for debugging.
    ///
    /// Returns a [`PredicateTrace::Single`] capturing the input, extracted
    /// data, matcher, and whether the predicate matched.
    #[must_use]
    pub fn evaluate_with_trace(&self, ctx: &Ctx) -> PredicateTrace {
        let data = self.input.get(ctx);
        let matched = match &data {
            MatchingData::None => false,
            _ => self.matcher.matches(&data),
        };
        PredicateTrace::Single {
            matched,
            input: format!("{:?}", self.input),
            data: format!("{data:?}"),
            matcher: format!("{:?}", self.matcher),
        }
    }
}

impl<Ctx> Debug for SinglePredicate<Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SinglePredicate")
            .field("input", &self.input)
            .field("matcher", &self.matcher)
            .finish()
    }
}

// Note: No unsafe impl needed — compiler derives Send/Sync automatically
// because Box<dyn DataInput<Ctx>> requires Send + Sync in the trait bound,
// and Box<dyn InputMatcher> also requires Send + Sync.

/// Composite predicate with boolean logic.
///
/// Predicates can be composed using AND, OR, and NOT operations.
/// Evaluation uses short-circuit semantics for efficiency.
///
/// # Variants
///
/// - `Single` — A single predicate
/// - `And` — All predicates must match (short-circuit on first false)
/// - `Or` — Any predicate must match (short-circuit on first true)
/// - `Not` — Inverts the result of the inner predicate
///
/// # Example
///
/// ```ignore
/// // Match requests where content-type contains "json" AND method is "POST"
/// let predicate = Predicate::And(vec![
///     Predicate::Single(content_type_json),
///     Predicate::Single(method_post),
/// ]);
/// ```
pub enum Predicate<Ctx> {
    /// A single predicate.
    Single(SinglePredicate<Ctx>),

    /// All predicates must match (logical AND).
    /// Short-circuits on the first `false`.
    And(Vec<Predicate<Ctx>>),

    /// Any predicate must match (logical OR).
    /// Short-circuits on the first `true`.
    Or(Vec<Predicate<Ctx>>),

    /// Inverts the result of the inner predicate (logical NOT).
    Not(Box<Predicate<Ctx>>),
}

impl<Ctx> Predicate<Ctx> {
    /// Evaluate this predicate against the given context.
    ///
    /// # Safety Consideration (Taleb)
    ///
    /// This uses recursion. Production code should use iterative evaluation
    /// with an explicit stack to protect against stack overflow from deeply
    /// nested predicates. The depth limit (max 32) should be enforced at
    /// config load time, not evaluation time.
    pub fn evaluate(&self, ctx: &Ctx) -> bool {
        match self {
            Predicate::Single(p) => p.evaluate(ctx),
            Predicate::And(predicates) => predicates.iter().all(|p| p.evaluate(ctx)),
            Predicate::Or(predicates) => predicates.iter().any(|p| p.evaluate(ctx)),
            Predicate::Not(p) => !p.evaluate(ctx),
        }
    }

    /// Evaluate with full trace for debugging.
    ///
    /// Unlike [`evaluate()`](Self::evaluate), this does NOT short-circuit
    /// And/Or — all children are evaluated for maximum debugging visibility.
    /// The `matched` result is still correct.
    #[must_use]
    pub fn evaluate_with_trace(&self, ctx: &Ctx) -> PredicateTrace {
        match self {
            Self::Single(p) => p.evaluate_with_trace(ctx),
            Self::And(predicates) => {
                let children: Vec<PredicateTrace> = predicates
                    .iter()
                    .map(|p| p.evaluate_with_trace(ctx))
                    .collect();
                let matched = children.iter().all(PredicateTrace::matched);
                PredicateTrace::And { matched, children }
            }
            Self::Or(predicates) => {
                let children: Vec<PredicateTrace> = predicates
                    .iter()
                    .map(|p| p.evaluate_with_trace(ctx))
                    .collect();
                let matched = children.iter().any(PredicateTrace::matched);
                PredicateTrace::Or { matched, children }
            }
            Self::Not(p) => {
                let inner = p.evaluate_with_trace(ctx);
                PredicateTrace::Not {
                    matched: !inner.matched(),
                    inner: Box::new(inner),
                }
            }
        }
    }

    /// Compose predicates with AND semantics, optimizing for common cases.
    ///
    /// - Empty → `catch_all` (no conditions = match everything)
    /// - Single → unwrapped (no wrapping overhead)
    /// - Multiple → `And(predicates)`
    ///
    /// This eliminates the repeated `if empty / if 1 / else And` pattern
    /// in every domain compiler.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // unwrap guarded by len() == 1
    pub fn from_all(predicates: Vec<Self>, catch_all: Self) -> Self {
        match predicates.len() {
            0 => catch_all,
            1 => predicates.into_iter().next().unwrap(),
            _ => Self::And(predicates),
        }
    }

    /// Compose predicates with OR semantics, optimizing for common cases.
    ///
    /// - Empty → `catch_all` (no conditions = match everything)
    /// - Single → unwrapped (no wrapping overhead)
    /// - Multiple → `Or(predicates)`
    ///
    /// Symmetric with [`from_all`](Self::from_all).
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // unwrap guarded by len() == 1
    pub fn from_any(predicates: Vec<Self>, catch_all: Self) -> Self {
        match predicates.len() {
            0 => catch_all,
            1 => predicates.into_iter().next().unwrap(),
            _ => Self::Or(predicates),
        }
    }

    /// Returns `true` if this is a `Single` predicate.
    #[must_use]
    pub fn is_single(&self) -> bool {
        matches!(self, Predicate::Single(_))
    }

    /// Returns `true` if this is an `And` predicate.
    #[must_use]
    pub fn is_and(&self) -> bool {
        matches!(self, Predicate::And(_))
    }

    /// Returns `true` if this is an `Or` predicate.
    #[must_use]
    pub fn is_or(&self) -> bool {
        matches!(self, Predicate::Or(_))
    }

    /// Returns `true` if this is a `Not` predicate.
    #[must_use]
    pub fn is_not(&self) -> bool {
        matches!(self, Predicate::Not(_))
    }

    /// Calculate the depth of this predicate tree.
    ///
    /// Used for depth limit validation at config time.
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Predicate::Single(_) => 1,
            Predicate::And(ps) | Predicate::Or(ps) => {
                1 + ps.iter().map(Predicate::depth).max().unwrap_or(0)
            }
            Predicate::Not(p) => 1 + p.depth(),
        }
    }
}

impl<Ctx> Debug for Predicate<Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Single(p) => f.debug_tuple("Single").field(p).finish(),
            Predicate::And(ps) => f.debug_tuple("And").field(&ps.len()).finish(),
            Predicate::Or(ps) => f.debug_tuple("Or").field(&ps.len()).finish(),
            Predicate::Not(_) => f.debug_tuple("Not").finish(),
        }
    }
}

// Note: No unsafe impl needed — compiler derives Send/Sync automatically
// because SinglePredicate<Ctx>, Vec<Predicate<Ctx>>, and Box<Predicate<Ctx>>
// are all Send/Sync when their contents are.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContainsMatcher, ExactMatcher};

    #[derive(Debug, Clone)]
    struct TestContext {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput;

    impl DataInput<TestContext> for ValueInput {
        fn get(&self, ctx: &TestContext) -> MatchingData {
            MatchingData::String(ctx.value.clone())
        }
    }

    #[derive(Debug)]
    struct NoneInput;

    impl DataInput<TestContext> for NoneInput {
        fn get(&self, _ctx: &TestContext) -> MatchingData {
            MatchingData::None
        }
    }

    #[test]
    fn test_single_predicate_matches() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };
        let pred = SinglePredicate::new(Box::new(ValueInput), Box::new(ExactMatcher::new("hello")));
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn test_single_predicate_none_returns_false() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };
        let pred = SinglePredicate::new(Box::new(NoneInput), Box::new(ExactMatcher::new("hello")));
        // INV: None from DataInput → false
        assert!(!pred.evaluate(&ctx));
    }

    #[test]
    fn test_predicate_and() {
        let ctx = TestContext {
            value: "hello world".to_string(),
        };

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

        assert!(pred.evaluate(&ctx));

        // AND with one false
        let pred_fail = Predicate::And(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ContainsMatcher::new("hello")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ContainsMatcher::new("foo")),
            )),
        ]);

        assert!(!pred_fail.evaluate(&ctx));
    }

    #[test]
    fn test_predicate_or() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };

        let pred = Predicate::Or(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("foo")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("hello")),
            )),
        ]);

        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn test_predicate_not() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };

        let pred = Predicate::Not(Box::new(Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("world")),
        ))));

        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn test_predicate_depth() {
        let single = Predicate::Single::<TestContext>(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("x")),
        ));
        assert_eq!(single.depth(), 1);

        let and = Predicate::And(vec![single]);
        assert_eq!(and.depth(), 2);

        let nested = Predicate::Not(Box::new(Predicate::And(vec![Predicate::Single(
            SinglePredicate::new(Box::new(ValueInput), Box::new(ExactMatcher::new("x"))),
        )])));
        assert_eq!(nested.depth(), 3);
    }

    #[test]
    fn test_empty_and_returns_true() {
        // Empty AND should return true (vacuous truth)
        let ctx = TestContext {
            value: "anything".to_string(),
        };
        let pred = Predicate::<TestContext>::And(vec![]);
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn test_empty_or_returns_false() {
        // Empty OR should return false
        let ctx = TestContext {
            value: "anything".to_string(),
        };
        let pred = Predicate::<TestContext>::Or(vec![]);
        assert!(!pred.evaluate(&ctx));
    }

    #[test]
    fn test_predicates_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SinglePredicate<TestContext>>();
        assert_send_sync::<Predicate<TestContext>>();
    }

    // ========== Smart Constructor Tests ==========

    fn make_single(expected: &str) -> Predicate<TestContext> {
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new(expected)),
        ))
    }

    fn catch_all() -> Predicate<TestContext> {
        Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(crate::PrefixMatcher::new("")),
        ))
    }

    #[test]
    fn from_all_empty_returns_catch_all() {
        let ctx = TestContext {
            value: "anything".into(),
        };
        let pred = Predicate::from_all(vec![], catch_all());
        assert!(pred.evaluate(&ctx)); // catch-all matches everything
    }

    #[test]
    fn from_all_single_unwraps() {
        let pred = Predicate::from_all(vec![make_single("hello")], catch_all());
        assert!(pred.is_single()); // no And wrapper
    }

    #[test]
    fn from_all_multiple_wraps_and() {
        let pred = Predicate::from_all(vec![make_single("a"), make_single("b")], catch_all());
        assert!(pred.is_and());
    }

    #[test]
    fn from_any_empty_returns_catch_all() {
        let ctx = TestContext {
            value: "anything".into(),
        };
        let pred = Predicate::from_any(vec![], catch_all());
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn from_any_single_unwraps() {
        let pred = Predicate::from_any(vec![make_single("hello")], catch_all());
        assert!(pred.is_single());
    }

    #[test]
    fn from_any_multiple_wraps_or() {
        let pred = Predicate::from_any(vec![make_single("a"), make_single("b")], catch_all());
        assert!(pred.is_or());
    }

    // ========== Trace Tests ==========

    #[test]
    fn trace_single_match() {
        let ctx = TestContext {
            value: "hello".into(),
        };
        let pred = SinglePredicate::new(Box::new(ValueInput), Box::new(ExactMatcher::new("hello")));
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(trace.matched());
        if let PredicateTrace::Single {
            matched,
            input,
            data,
            matcher,
        } = &trace
        {
            assert!(matched);
            assert!(input.contains("ValueInput"));
            assert!(data.contains("hello"));
            assert!(matcher.contains("hello"));
        } else {
            panic!("expected Single trace");
        }
    }

    #[test]
    fn trace_single_no_match() {
        let ctx = TestContext {
            value: "world".into(),
        };
        let pred = SinglePredicate::new(Box::new(ValueInput), Box::new(ExactMatcher::new("hello")));
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(!trace.matched());
    }

    #[test]
    fn trace_single_none_returns_false() {
        let ctx = TestContext {
            value: "hello".into(),
        };
        let pred = SinglePredicate::new(Box::new(NoneInput), Box::new(ExactMatcher::new("hello")));
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(!trace.matched());
        if let PredicateTrace::Single { data, .. } = &trace {
            assert!(data.contains("None"));
        } else {
            panic!("expected Single trace");
        }
    }

    #[test]
    fn trace_and_all_match() {
        let ctx = TestContext {
            value: "hello world".into(),
        };
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
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(trace.matched());
        if let PredicateTrace::And { children, .. } = &trace {
            assert_eq!(children.len(), 2);
            assert!(children[0].matched());
            assert!(children[1].matched());
        } else {
            panic!("expected And trace");
        }
    }

    #[test]
    fn trace_and_partial_failure_evaluates_all() {
        let ctx = TestContext {
            value: "hello".into(),
        };
        // First fails, second would match — both should be evaluated in trace
        let pred = Predicate::And(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("nope")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("hello")),
            )),
        ]);
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(!trace.matched());
        if let PredicateTrace::And { children, .. } = &trace {
            // Both children evaluated (no short-circuit in trace)
            assert_eq!(children.len(), 2);
            assert!(!children[0].matched());
            assert!(children[1].matched()); // would have been skipped in evaluate()
        } else {
            panic!("expected And trace");
        }
    }

    #[test]
    fn trace_or_first_match() {
        let ctx = TestContext {
            value: "hello".into(),
        };
        let pred = Predicate::Or(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("hello")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("nope")),
            )),
        ]);
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(trace.matched());
        if let PredicateTrace::Or { children, .. } = &trace {
            // Both evaluated in trace mode
            assert_eq!(children.len(), 2);
            assert!(children[0].matched());
            assert!(!children[1].matched());
        } else {
            panic!("expected Or trace");
        }
    }

    #[test]
    fn trace_not() {
        let ctx = TestContext {
            value: "hello".into(),
        };
        let pred = Predicate::Not(Box::new(Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("world")),
        ))));
        let trace = pred.evaluate_with_trace(&ctx);

        assert!(trace.matched()); // NOT(false) = true
        if let PredicateTrace::Not { inner, .. } = &trace {
            assert!(!inner.matched()); // inner didn't match
        } else {
            panic!("expected Not trace");
        }
    }

    #[test]
    fn trace_empty_and_returns_true() {
        let ctx = TestContext { value: "x".into() };
        let pred = Predicate::<TestContext>::And(vec![]);
        let trace = pred.evaluate_with_trace(&ctx);
        assert!(trace.matched()); // vacuous truth
    }

    #[test]
    fn trace_empty_or_returns_false() {
        let ctx = TestContext { value: "x".into() };
        let pred = Predicate::<TestContext>::Or(vec![]);
        let trace = pred.evaluate_with_trace(&ctx);
        assert!(!trace.matched());
    }

    #[test]
    fn trace_result_matches_evaluate() {
        let ctx = TestContext {
            value: "hello world".into(),
        };

        // Complex nested predicate
        let pred = Predicate::And(vec![
            Predicate::Or(vec![
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("nope")),
                )),
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ContainsMatcher::new("hello")),
                )),
            ]),
            Predicate::Not(Box::new(Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("nope")),
            )))),
        ]);

        let eval_result = pred.evaluate(&ctx);
        let trace = pred.evaluate_with_trace(&ctx);

        assert_eq!(eval_result, trace.matched());
    }
}
