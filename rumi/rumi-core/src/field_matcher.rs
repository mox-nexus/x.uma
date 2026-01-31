//! `FieldMatcher` — Predicate + `OnMatch` combination
//!
//! A `FieldMatcher` binds a predicate to what happens when it matches.
//! The top-level `Matcher` contains a list of field matchers.

use crate::{OnMatch, Predicate};
use core::fmt::Debug;

/// A field matcher: predicate + `on_match`.
///
/// When the predicate evaluates to `true`, the `on_match` action is returned
/// or the nested matcher is evaluated.
///
/// # Type Parameters
///
/// - `Ctx`: The context type
/// - `A`: The action type (must be `Clone + Send + Sync + 'static`)
///
/// # Example
///
/// ```ignore
/// let field_matcher = FieldMatcher::new(
///     Predicate::Single(path_predicate),
///     OnMatch::action("route_to_api".to_string()),
/// );
/// ```
pub struct FieldMatcher<Ctx, A: Clone + Send + Sync + 'static> {
    /// The predicate to evaluate.
    pub predicate: Predicate<Ctx>,

    /// What to do when the predicate matches.
    pub on_match: OnMatch<Ctx, A>,
}

impl<Ctx, A: Clone + Send + Sync + 'static> FieldMatcher<Ctx, A> {
    /// Create a new field matcher.
    pub fn new(predicate: Predicate<Ctx>, on_match: OnMatch<Ctx, A>) -> Self {
        Self {
            predicate,
            on_match,
        }
    }

    /// Evaluate the predicate against the context.
    ///
    /// Returns `true` if the predicate matches.
    pub fn matches(&self, ctx: &Ctx) -> bool {
        self.predicate.evaluate(ctx)
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for FieldMatcher<Ctx, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FieldMatcher")
            .field("predicate", &self.predicate)
            .field("on_match", &self.on_match)
            .finish()
    }
}

// Note: No unsafe impl needed — compiler derives Send/Sync automatically
// because Predicate<Ctx> and OnMatch<Ctx, A> are Send/Sync when their
// type parameters satisfy the bounds.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataInput, ExactMatcher, MatchingData, SinglePredicate};

    #[derive(Debug, Clone)]
    struct TestCtx {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput;

    impl DataInput<TestCtx> for ValueInput {
        fn get(&self, ctx: &TestCtx) -> MatchingData {
            MatchingData::String(ctx.value.clone())
        }
    }

    #[test]
    fn test_field_matcher_matches() {
        let field_matcher: FieldMatcher<TestCtx, String> = FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("hello")),
            )),
            OnMatch::action("matched".to_string()),
        );

        let ctx = TestCtx {
            value: "hello".to_string(),
        };
        assert!(field_matcher.matches(&ctx));

        let ctx_no_match = TestCtx {
            value: "world".to_string(),
        };
        assert!(!field_matcher.matches(&ctx_no_match));
    }

    #[test]
    fn test_field_matcher_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FieldMatcher<TestCtx, String>>();
    }
}
