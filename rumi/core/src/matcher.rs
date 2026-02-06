//! Matcher — Top-level matcher with first-match-wins semantics
//!
//! The `Matcher` is the entry point for evaluation. It contains a list of
//! field matchers and evaluates them in order, returning the first match.

use crate::{FieldMatcher, MatcherError, OnMatch, MAX_DEPTH};
use std::fmt::Debug;
use std::marker::PhantomData;

/// Top-level matcher with first-match-wins semantics.
///
/// A Matcher contains:
/// - A list of field matchers (predicate + action pairs)
/// - An optional `on_no_match` fallback
///
/// Evaluation iterates through the field matchers in order and returns
/// the action from the first matching predicate. If no predicate matches,
/// the `on_no_match` action is returned (if present).
///
/// # Type Parameters
///
/// - `Ctx`: The context type to match against
/// - `A`: The action type (must be `Clone + Send + Sync + 'static`)
///
/// # INV (Dijkstra): First-match-wins
///
/// Field matchers are evaluated in order. The first matching predicate
/// terminates evaluation, even if later predicates would also match.
///
/// # xDS Semantics: Nested Matcher Failure Propagates
///
/// When an `OnMatch` contains a nested matcher and that nested matcher
/// returns no match, the ENTIRE `OnMatch` fails — there is no fallback
/// to a sibling action (because `OnMatch` is exclusive per xDS proto).
///
/// # Example
///
/// ```ignore
/// let matcher = Matcher::new(
///     vec![
///         FieldMatcher::new(api_path_predicate, OnMatch::action("api".to_string())),
///         FieldMatcher::new(static_path_predicate, OnMatch::action("static".to_string())),
///     ],
///     Some(OnMatch::action("default".to_string())),
/// );
///
/// let action = matcher.evaluate(&request);
/// ```
pub struct Matcher<Ctx, A: Clone + Send + Sync + 'static> {
    /// The list of field matchers to evaluate.
    pub matcher_list: Vec<FieldMatcher<Ctx, A>>,

    /// Fallback when no field matcher matches.
    /// Note: per xDS, this is at the Matcher level, not per-OnMatch.
    pub on_no_match: Option<OnMatch<Ctx, A>>,

    _phantom: PhantomData<Ctx>,
}

impl<Ctx, A: Clone + Send + Sync + 'static> Matcher<Ctx, A> {
    /// Create a new matcher.
    pub fn new(
        matcher_list: Vec<FieldMatcher<Ctx, A>>,
        on_no_match: Option<OnMatch<Ctx, A>>,
    ) -> Self {
        Self {
            matcher_list,
            on_no_match,
            _phantom: PhantomData,
        }
    }

    /// Create an empty matcher (no field matchers, no fallback).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            matcher_list: Vec::new(),
            on_no_match: None,
            _phantom: PhantomData,
        }
    }

    /// Evaluate this matcher against the given context.
    ///
    /// Returns the action from the first matching field matcher,
    /// or the `on_no_match` action if nothing matches.
    ///
    /// # First-match-wins semantics (INV)
    ///
    /// Field matchers are evaluated in order. Evaluation stops at the
    /// first matching predicate.
    ///
    /// # xDS Nested Matcher Semantics
    ///
    /// If a field matcher's `OnMatch` contains a nested matcher:
    /// - Evaluation continues into the nested matcher
    /// - If the nested matcher returns Some(action), that's returned
    /// - If the nested matcher returns None, this field matcher is considered
    ///   NOT matched, and we continue to the next field matcher
    ///
    /// This matches xDS semantics where nested matcher failure propagates up.
    pub fn evaluate(&self, ctx: &Ctx) -> Option<A> {
        // First-match-wins: iterate through matchers, return first match
        for field_matcher in &self.matcher_list {
            if field_matcher.matches(ctx) {
                // OnMatch is now an enum: either Action or Matcher
                match &field_matcher.on_match {
                    OnMatch::Action(action) => return Some(action.clone()),
                    OnMatch::Matcher(nested) => {
                        // xDS semantics: nested matcher failure propagates
                        // If nested returns None, continue to next field_matcher
                        if let Some(action) = nested.evaluate(ctx) {
                            return Some(action);
                        }
                        // Nested returned None → this field_matcher is NOT a match
                        // Continue to next field_matcher
                    }
                }
            }
        }

        // No match: return on_no_match action if present
        self.on_no_match.as_ref().and_then(|om| match om {
            OnMatch::Action(a) => Some(a.clone()),
            OnMatch::Matcher(nested) => nested.evaluate(ctx),
        })
    }

    /// Returns the number of field matchers.
    pub fn len(&self) -> usize {
        self.matcher_list.len()
    }

    /// Returns `true` if there are no field matchers.
    pub fn is_empty(&self) -> bool {
        self.matcher_list.is_empty()
    }

    /// Returns `true` if there is an `on_no_match` fallback.
    pub fn has_fallback(&self) -> bool {
        self.on_no_match.is_some()
    }

    /// Calculate the maximum depth of this matcher tree.
    ///
    /// Used for depth limit validation at config time.
    pub fn depth(&self) -> usize {
        let field_depth = self
            .matcher_list
            .iter()
            .map(|fm| {
                let pred_depth = fm.predicate.depth();
                let nested_depth = match &fm.on_match {
                    OnMatch::Action(_) => 0,
                    OnMatch::Matcher(m) => m.depth(),
                };
                pred_depth.max(nested_depth)
            })
            .max()
            .unwrap_or(0);

        let no_match_depth = self.on_no_match.as_ref().map_or(0, |om| match om {
            OnMatch::Action(_) => 0,
            OnMatch::Matcher(m) => m.depth(),
        });

        1 + field_depth.max(no_match_depth)
    }

    /// Validate this matcher against safety constraints.
    ///
    /// Checks:
    /// - Nesting depth does not exceed [`MAX_DEPTH`]
    ///
    /// Call this at config load time to catch errors early.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::DepthExceeded`] if nesting is too deep.
    pub fn validate(&self) -> Result<(), MatcherError> {
        let depth = self.depth();
        if depth > MAX_DEPTH {
            return Err(MatcherError::DepthExceeded {
                depth,
                max: MAX_DEPTH,
            });
        }
        Ok(())
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for Matcher<Ctx, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matcher")
            .field("matcher_list_len", &self.matcher_list.len())
            .field("has_fallback", &self.on_no_match.is_some())
            .finish()
    }
}

impl<Ctx, A: Clone + Send + Sync + 'static> Clone for Matcher<Ctx, A>
where
    FieldMatcher<Ctx, A>: Clone,
    OnMatch<Ctx, A>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            matcher_list: self.matcher_list.clone(),
            on_no_match: self.on_no_match.clone(),
            _phantom: PhantomData,
        }
    }
}

// Note: No unsafe impl needed — compiler derives Send/Sync automatically
// because all fields (Vec<FieldMatcher>, Option<OnMatch>, PhantomData) are Send/Sync
// when their type parameters are.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataInput, ExactMatcher, MatchingData, Predicate, SinglePredicate};

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

    fn create_field_matcher(expected: &str, action: &str) -> FieldMatcher<TestCtx, String> {
        FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new(expected)),
            )),
            OnMatch::action(action.to_string()),
        )
    }

    #[test]
    fn test_matcher_first_match_wins() {
        let matcher = Matcher::new(
            vec![
                create_field_matcher("hello", "first"),
                create_field_matcher("hello", "second"), // Also matches, but won't be reached
            ],
            None,
        );

        let ctx = TestCtx {
            value: "hello".to_string(),
        };

        // First-match-wins: should return "first"
        assert_eq!(matcher.evaluate(&ctx), Some("first".to_string()));
    }

    #[test]
    fn test_matcher_no_match_fallback() {
        let matcher = Matcher::new(
            vec![create_field_matcher("hello", "first")],
            Some(OnMatch::action("fallback".to_string())),
        );

        let ctx = TestCtx {
            value: "world".to_string(),
        };

        // No match: should return fallback
        assert_eq!(matcher.evaluate(&ctx), Some("fallback".to_string()));
    }

    #[test]
    fn test_matcher_no_match_no_fallback() {
        let matcher: Matcher<TestCtx, String> =
            Matcher::new(vec![create_field_matcher("hello", "first")], None);

        let ctx = TestCtx {
            value: "world".to_string(),
        };

        // No match, no fallback: should return None
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn test_matcher_multiple_rules() {
        let matcher = Matcher::new(
            vec![
                create_field_matcher("hello", "hello_action"),
                create_field_matcher("world", "world_action"),
            ],
            Some(OnMatch::action("default".to_string())),
        );

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "hello".into()
            }),
            Some("hello_action".to_string())
        );

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "world".into()
            }),
            Some("world_action".to_string())
        );

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "other".into()
            }),
            Some("default".to_string())
        );
    }

    #[test]
    fn test_nested_matcher_failure_propagates() {
        // Create a nested matcher that will NOT match
        let nested = Matcher::new(
            vec![create_field_matcher("will_not_match", "nested_action")],
            None, // No fallback in nested
        );

        // Parent matcher: predicate matches, but OnMatch is a nested matcher that fails
        let parent = Matcher::new(
            vec![
                FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(ExactMatcher::new("hello")),
                    )),
                    OnMatch::matcher(nested),
                ),
                create_field_matcher("hello", "second_action"), // Fallthrough to this
            ],
            None,
        );

        let ctx = TestCtx {
            value: "hello".to_string(),
        };

        // xDS semantics: nested failure propagates, so we continue to next field_matcher
        assert_eq!(parent.evaluate(&ctx), Some("second_action".to_string()));
    }

    #[test]
    fn test_matcher_depth() {
        let simple = Matcher::<TestCtx, String>::new(vec![create_field_matcher("x", "y")], None);
        // Matcher depth 1 + predicate depth 1 = 2
        assert_eq!(simple.depth(), 2);
    }

    #[test]
    fn test_matcher_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Matcher<TestCtx, String>>();
    }

    #[test]
    fn test_validate_shallow_matcher_ok() {
        let matcher = Matcher::<TestCtx, String>::new(vec![create_field_matcher("x", "y")], None);
        assert!(matcher.validate().is_ok());
    }

    #[test]
    fn test_validate_deeply_nested_matcher_fails() {
        // Build a matcher chain deeper than MAX_DEPTH
        let mut current =
            Matcher::<TestCtx, String>::new(vec![create_field_matcher("leaf", "action")], None);

        // Nest MAX_DEPTH + 1 times to exceed the limit
        // Each nesting adds 1 to depth (the wrapping Matcher)
        for _ in 0..crate::MAX_DEPTH {
            current = Matcher::new(
                vec![FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(ExactMatcher::new("x")),
                    )),
                    OnMatch::matcher(current),
                )],
                None,
            );
        }

        let result = current.validate();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::MatcherError::DepthExceeded { .. })
        ));
    }

    #[test]
    fn test_validate_at_max_depth_ok() {
        // Build exactly at MAX_DEPTH — should pass
        let mut current =
            Matcher::<TestCtx, String>::new(vec![create_field_matcher("leaf", "action")], None);

        // depth starts at 2 (1 matcher + 1 predicate), each nesting adds 1
        // We need total depth == MAX_DEPTH
        for _ in 0..(crate::MAX_DEPTH - 2) {
            current = Matcher::new(
                vec![FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(ExactMatcher::new("x")),
                    )),
                    OnMatch::matcher(current),
                )],
                None,
            );
        }

        assert_eq!(current.depth(), crate::MAX_DEPTH);
        assert!(current.validate().is_ok());
    }
}
