//! Predicate — Boolean expressions over DataInputs
//!
//! Predicates combine [`DataInput`] and [`InputMatcher`] to create boolean
//! conditions that can be composed with AND/OR/NOT.

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{boxed::Box, vec::Vec};

use crate::{DataInput, InputMatcher, MatchingData};
use core::fmt::Debug;

/// A single predicate: combines a [`DataInput`] with an [`InputMatcher`].
///
/// This is where domain-specific (DataInput) meets domain-agnostic (InputMatcher).
/// The SinglePredicate extracts data from the context using the input, then
/// passes it to the matcher for evaluation.
///
/// # INV (Dijkstra): None → false
///
/// If the DataInput returns [`MatchingData::None`], the predicate evaluates to `false`.
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
    /// Create a new single predicate from a DataInput and InputMatcher.
    pub fn new(input: Box<dyn DataInput<Ctx>>, matcher: Box<dyn InputMatcher>) -> Self {
        Self { input, matcher }
    }

    /// Get a reference to the input.
    pub fn input(&self) -> &dyn DataInput<Ctx> {
        &*self.input
    }

    /// Get a reference to the matcher.
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
}

impl<Ctx> Debug for SinglePredicate<Ctx> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

    /// Returns `true` if this is a `Single` predicate.
    pub fn is_single(&self) -> bool {
        matches!(self, Predicate::Single(_))
    }

    /// Returns `true` if this is an `And` predicate.
    pub fn is_and(&self) -> bool {
        matches!(self, Predicate::And(_))
    }

    /// Returns `true` if this is an `Or` predicate.
    pub fn is_or(&self) -> bool {
        matches!(self, Predicate::Or(_))
    }

    /// Returns `true` if this is a `Not` predicate.
    pub fn is_not(&self) -> bool {
        matches!(self, Predicate::Not(_))
    }

    /// Calculate the depth of this predicate tree.
    ///
    /// Used for depth limit validation at config time.
    pub fn depth(&self) -> usize {
        match self {
            Predicate::Single(_) => 1,
            Predicate::And(ps) | Predicate::Or(ps) => {
                1 + ps.iter().map(|p| p.depth()).max().unwrap_or(0)
            }
            Predicate::Not(p) => 1 + p.depth(),
        }
    }
}

impl<Ctx> Debug for Predicate<Ctx> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
        let pred = SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("hello")),
        );
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn test_single_predicate_none_returns_false() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };
        let pred = SinglePredicate::new(
            Box::new(NoneInput),
            Box::new(ExactMatcher::new("hello")),
        );
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

        let nested = Predicate::Not(Box::new(Predicate::And(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("x")),
            )),
        ])));
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
}
