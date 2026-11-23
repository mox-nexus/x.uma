//! Matcher — Top-level matcher with first-match-wins semantics
//!
//! The `Matcher` is the entry point for evaluation. It contains a list of
//! field matchers and evaluates them in order, returning the first match.

use crate::{
    EvalStep, EvalSteps, EvalTrace, FieldMatcher, MatchResult, MatcherError, MatcherTree, OnMatch,
    OnMatchTrace, Predicate, TreeAttempt, TreeLookupTrace, MAX_DEPTH,
};
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
/// let matcher = Matcher::list(
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
    /// A list of field matchers, or a lookup tree. Never both.
    pub kind: MatcherKind<Ctx, A>,

    /// Fallback when nothing matched.
    /// Note: per xDS, this is at the Matcher level, not per-OnMatch.
    pub on_no_match: Option<OnMatch<Ctx, A>>,

    _phantom: PhantomData<Ctx>,
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for MatcherKind<Ctx, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::List(l) => f.debug_struct("List").field("len", &l.len()).finish(),
            Self::Tree(t) => f.debug_tuple("Tree").field(t).finish(),
        }
    }
}

/// What one evaluation decided (D-069).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<A> {
    /// A rule, or the fallback, produced this action.
    Action(A),
    /// No rule matched, and there was no fallback or it matched nothing.
    NoMatch,
    /// A required value was absent: the evaluation stopped without an action,
    /// and without its fallback.
    Undecided,
}

impl<A> Outcome<A> {
    /// The action, if one was produced.
    pub fn into_action(self) -> Option<A> {
        match self {
            Self::Action(a) => Some(a),
            Self::NoMatch | Self::Undecided => None,
        }
    }
}

/// How a matcher selects an outcome — xDS `oneof matcher_type`.
///
/// A closed enum over a closed oneof: a third arm would be an upstream xDS
/// change. Deliberately not a trait object, which would open a dispatch point
/// where xDS has a fixed choice and invite matcher kinds that no protojson can
/// spell and no other implementation can execute.
pub enum MatcherKind<Ctx, A: Clone + Send + Sync + 'static> {
    /// Field matchers evaluated in order, first match wins.
    List(Vec<FieldMatcher<Ctx, A>>),
    /// A single map lookup on a key extracted from the context.
    Tree(MatcherTree<Ctx, A>),
}

impl<Ctx, A: Clone + Send + Sync + 'static> Matcher<Ctx, A> {
    /// Create a list matcher — field matchers evaluated in order, first match
    /// wins.
    ///
    /// Named `list` rather than `new` because `Matcher` is an xDS
    /// `oneof matcher_type`: this builds one of its two shapes, and
    /// [`tree`](Self::tree) builds the other. A `new` here would read as *the*
    /// way to construct a `Matcher`, which is true of neither.
    pub fn list(
        matcher_list: Vec<FieldMatcher<Ctx, A>>,
        on_no_match: Option<OnMatch<Ctx, A>>,
    ) -> Self {
        Self {
            kind: MatcherKind::List(matcher_list),
            on_no_match,
            _phantom: PhantomData,
        }
    }

    /// Create a tree matcher — a single map lookup rather than a linear scan.
    pub fn tree(tree: MatcherTree<Ctx, A>, on_no_match: Option<OnMatch<Ctx, A>>) -> Self {
        Self {
            kind: MatcherKind::Tree(tree),
            on_no_match,
            _phantom: PhantomData,
        }
    }

    /// The field matchers, if this is a list matcher.
    pub fn matcher_list(&self) -> Option<&[FieldMatcher<Ctx, A>]> {
        match &self.kind {
            MatcherKind::List(l) => Some(l),
            MatcherKind::Tree(_) => None,
        }
    }

    /// Create a matcher from a single predicate with action and optional fallback.
    ///
    /// This is the common pattern for domain compilers: a predicate tree
    /// (built with [`Predicate::from_all`] / [`Predicate::from_any`])
    /// paired with a single action and optional fallback.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let matcher = Matcher::from_predicate(
    ///     Predicate::from_any(rule_predicates, catch_all),
    ///     "allow",
    ///     Some("deny"),
    /// );
    /// ```
    pub fn from_predicate(predicate: Predicate<Ctx>, action: A, on_no_match: Option<A>) -> Self {
        Self::list(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            on_no_match.map(OnMatch::Action),
        )
    }

    /// Create an empty matcher (no field matchers, no fallback).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            kind: MatcherKind::List(Vec::new()),
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
        self.outcome(ctx).into_action()
    }

    /// Evaluate, keeping "a required value was absent" apart from "nothing
    /// matched" (D-069).
    ///
    /// A list stops at the first rule whose predicate is undecided, and a tree
    /// at an absent required key, as Envoy's matchers stop on insufficient
    /// data; neither a later rule nor `on_no_match` fires. A nested matcher
    /// that is undecided makes its parent undecided.
    pub fn outcome(&self, ctx: &Ctx) -> Outcome<A> {
        let decided = match &self.kind {
            MatcherKind::List(list) => Self::list_outcome(list, ctx),
            MatcherKind::Tree(tree) => tree.outcome(ctx),
        };
        match decided {
            Outcome::NoMatch => self
                .on_no_match
                .as_ref()
                .map_or(Outcome::NoMatch, |om| om.outcome(ctx)),
            decided => decided,
        }
    }

    fn list_outcome(list: &[FieldMatcher<Ctx, A>], ctx: &Ctx) -> Outcome<A> {
        for field_matcher in list {
            match field_matcher.predicate.match_result(ctx) {
                MatchResult::NoMatch => {}
                MatchResult::Undecided => return Outcome::Undecided,
                // xDS: a nested matcher that matches nothing hands evaluation
                // on to the next rule.
                MatchResult::Matched => match field_matcher.on_match.outcome(ctx) {
                    Outcome::NoMatch => {}
                    decided => return decided,
                },
            }
        }
        Outcome::NoMatch
    }

    /// Evaluate with full trace for debugging.
    ///
    /// Returns the same result as [`evaluate()`](Self::evaluate) plus
    /// the full evaluation path: which field matchers were checked, which
    /// predicates fired, and whether the fallback was used.
    ///
    /// # INV: `result` == `evaluate()` result
    ///
    /// The returned [`EvalTrace::result`] always equals what `evaluate()`
    /// would return for the same context.
    #[must_use]
    pub fn evaluate_with_trace(&self, ctx: &Ctx) -> EvalTrace<A> {
        let (decided, steps) = match &self.kind {
            MatcherKind::List(list) => {
                let (r, s) = Self::trace_list(list, ctx);
                (r, EvalSteps::List(s))
            }
            MatcherKind::Tree(tree) => {
                let (r, t) = Self::trace_tree(tree, ctx);
                (r, EvalSteps::Tree(Box::new(t)))
            }
        };

        let (decided, used_fallback) = match decided {
            // `used_fallback` records that the fallback was *consulted*, so it
            // can be true while the result is still None: a fallback holding a
            // nested matcher may itself match nothing.
            Outcome::NoMatch => (
                self.on_no_match
                    .as_ref()
                    .map_or(Outcome::NoMatch, |om| om.outcome(ctx)),
                self.on_no_match.is_some(),
            ),
            decided => (decided, false),
        };

        EvalTrace {
            undecided: matches!(decided, Outcome::Undecided),
            result: decided.into_action(),
            steps,
            used_fallback,
        }
    }

    fn trace_list(list: &[FieldMatcher<Ctx, A>], ctx: &Ctx) -> (Outcome<A>, Vec<EvalStep<A>>) {
        let mut steps = Vec::new();

        for (index, field_matcher) in list.iter().enumerate() {
            let predicate_trace = field_matcher.predicate.evaluate_with_trace(ctx);
            let predicate_result = predicate_trace.result();

            if predicate_result != MatchResult::Matched {
                steps.push(EvalStep {
                    index,
                    matched: false,
                    predicate_trace,
                    on_match: None,
                });
                if predicate_result == MatchResult::Undecided {
                    return (Outcome::Undecided, steps);
                }
                continue;
            }

            let (decided, on_match) = match &field_matcher.on_match {
                OnMatch::Action(action) => (
                    Outcome::Action(action.clone()),
                    OnMatchTrace::Action(action.clone()),
                ),
                OnMatch::Matcher(nested) => {
                    let nested_trace = nested.evaluate_with_trace(ctx);
                    (
                        nested_trace.outcome(),
                        OnMatchTrace::Nested(Box::new(nested_trace)),
                    )
                }
            };
            steps.push(EvalStep {
                index,
                matched: true,
                predicate_trace,
                on_match: Some(on_match),
            });

            // First-match-wins: INV-2 requires stopping here, and INV-3
            // requires the trace agree with `outcome`, so this early return is
            // load-bearing rather than an optimisation.
            if !matches!(decided, Outcome::NoMatch) {
                return (decided, steps);
            }
        }

        (Outcome::NoMatch, steps)
    }

    /// The four outcomes of a tree lookup, recorded.
    ///
    /// | key | lookup | entry | result |
    /// |---|---|---|---|
    /// | `None` | — | — | `None`, falls to `on_no_match` |
    /// | `Some` | miss | — | `None`, falls to `on_no_match` |
    /// | `Some` | hit | action | that action |
    /// | `Some` | hit | nested -> `None` | `None`, falls to `on_no_match` |
    ///
    /// The last row is where this and `evaluate` would drift if written
    /// independently, so both read the same lookup through
    /// `MatcherTree::lookup`.
    fn trace_tree(tree: &MatcherTree<Ctx, A>, ctx: &Ctx) -> (Outcome<A>, TreeLookupTrace<A>) {
        let (kind, input) = tree.trace_identity();
        let key = tree.trace_key(ctx);

        let Some(key) = key else {
            return (
                tree.absent_key(),
                TreeLookupTrace {
                    kind,
                    input,
                    key: None,
                    attempts: Vec::new(),
                },
            );
        };

        let mut decided = Outcome::NoMatch;
        let mut attempts = Vec::new();
        for (matched_key, on_match) in tree.trace_candidates(&key) {
            let (d, on_match_trace) = match on_match {
                OnMatch::Action(a) => (Outcome::Action(a.clone()), OnMatchTrace::Action(a.clone())),
                OnMatch::Matcher(nested) => {
                    let nested_trace = nested.evaluate_with_trace(ctx);
                    (
                        nested_trace.outcome(),
                        OnMatchTrace::Nested(Box::new(nested_trace)),
                    )
                }
            };
            attempts.push(TreeAttempt {
                matched_key: matched_key.to_owned(),
                on_match: on_match_trace,
            });
            if !matches!(d, Outcome::NoMatch) {
                decided = d;
                break;
            }
        }

        (
            decided,
            TreeLookupTrace {
                kind,
                input,
                key: Some(key),
                attempts,
            },
        )
    }

    /// Number of alternatives this matcher chooses between — field matchers
    /// for a list, entries for a tree.
    pub fn len(&self) -> usize {
        match &self.kind {
            MatcherKind::List(l) => l.len(),
            MatcherKind::Tree(t) => t.len(),
        }
    }

    /// Returns `true` if there is nothing to match against.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if there is an `on_no_match` fallback.
    pub fn has_fallback(&self) -> bool {
        self.on_no_match.is_some()
    }

    /// Calculate the maximum depth of this matcher tree.
    ///
    /// Used for depth limit validation at config time.
    pub fn depth(&self) -> usize {
        // A tree's entries hold `OnMatch`, which can hold a `Matcher`, which
        // can hold another tree. Walking only the list arm here is what let
        // `matcherTree -> onMatch.matcher -> matcherTree -> ...` nest without
        // bound, report depth 1, pass `validate()`, and then overflow the stack
        // in the recursive `evaluate()`. See DECISIONS.md D-045.
        let field_depth = match &self.kind {
            MatcherKind::List(list) => list
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
                .unwrap_or(0),
            MatcherKind::Tree(tree) => tree.depth(),
        };

        let no_match_depth = self.on_no_match.as_ref().map_or(0, |om| match om {
            OnMatch::Action(_) => 0,
            OnMatch::Matcher(m) => m.depth(),
        });

        1 + field_depth.max(no_match_depth)
    }

    /// Validate this matcher against every structural safety limit.
    ///
    /// Checks, over the whole tree including nested matchers and `on_no_match`
    /// chains:
    /// - nesting depth against [`MAX_DEPTH`]
    /// - entries per tree against [`MAX_TREE_ENTRIES`](crate::MAX_TREE_ENTRIES)
    /// - the worst path's cost, [`path_cost`](Self::path_cost), against
    ///   [`MAX_PREDICATES_PER_EVALUATION`](crate::MAX_PREDICATES_PER_EVALUATION)
    ///
    /// The limits live here, on the type that holds the rules, so that
    /// whatever builds a matcher (the registry, a domain compiler, a binding,
    /// a hand) inherits the same guarantees rather than reimplementing them.
    ///
    /// Pattern-length limits are not here: a compiled pattern is held by
    /// [`StringMatchSpec`](crate::StringMatchSpec), which enforces its own in
    /// its constructor, for the same reason.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::DepthExceeded`] if nesting is too deep
    /// - [`MatcherError::TooManyTreeEntries`] if one tree holds too many entries
    /// - [`MatcherError::TooManyPredicatesPerEvaluation`] if one evaluation can
    ///   reach too many predicates
    pub fn validate(&self) -> Result<(), MatcherError> {
        let depth = self.depth();
        if depth > MAX_DEPTH {
            return Err(MatcherError::DepthExceeded {
                depth,
                max: MAX_DEPTH,
            });
        }
        self.validate_widths()?;
        let cost = self.path_cost();
        if cost > crate::MAX_PREDICATES_PER_EVALUATION {
            return Err(MatcherError::TooManyPredicatesPerEvaluation {
                count: cost,
                max: crate::MAX_PREDICATES_PER_EVALUATION,
            });
        }
        Ok(())
    }

    /// The most predicates one evaluation can reach.
    ///
    /// A list costs every rule's predicate and nested matcher, since a nested
    /// miss moves on to the next rule; a tree costs its dearest entry, or for a
    /// prefix tree its dearest chain of prefixes (D-055); the fallback adds its
    /// own. An upper bound: it assumes every predicate can hold.
    #[must_use]
    pub fn path_cost(&self) -> usize {
        let own = match &self.kind {
            MatcherKind::List(list) => list
                .iter()
                .map(|fm| fm.predicate.leaf_count() + fm.on_match.path_cost())
                .sum(),
            MatcherKind::Tree(tree) => tree.path_cost(),
        };
        own + self.on_no_match.as_ref().map_or(0, OnMatch::path_cost)
    }

    /// The tree-width half of [`validate`](Self::validate), recursing through
    /// nested matchers. Depth is checked once at the root because `depth()` is
    /// already a whole-tree measure.
    pub(crate) fn validate_widths(&self) -> Result<(), MatcherError> {
        match &self.kind {
            MatcherKind::List(list) => {
                for fm in list {
                    if let OnMatch::Matcher(nested) = &fm.on_match {
                        nested.validate_widths()?;
                    }
                }
            }
            MatcherKind::Tree(tree) => tree.validate_widths()?,
        }

        if let Some(OnMatch::Matcher(nested)) = &self.on_no_match {
            nested.validate_widths()?;
        }

        Ok(())
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for Matcher<Ctx, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matcher")
            .field("kind", &self.kind)
            .field("has_fallback", &self.on_no_match.is_some())
            .finish()
    }
}

// `Send` and `Sync` are derived, never `unsafe impl`'d, so the FFI bound is
// proved by the compiler rather than promised.

#[cfg(test)]
mod tests {
    /// The list steps of a trace, for assertions. Panics if the trace came
    /// from a tree — every caller here builds a list matcher.
    fn steps_of<A>(trace: &EvalTrace<A>) -> &[EvalStep<A>] {
        trace.steps.as_list().expect("list matcher")
    }

    use super::*;
    use crate::{DataInput, ExactMatcher, MatchingData, Predicate, SinglePredicate};

    #[derive(Debug, Clone)]
    struct TestCtx {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput;

    impl DataInput<TestCtx> for ValueInput {
        fn get(&self, ctx: &TestCtx) -> Option<MatchingData> {
            Some(MatchingData::String(ctx.value.clone()))
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
        let matcher = Matcher::list(
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
        let matcher = Matcher::list(
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
            Matcher::list(vec![create_field_matcher("hello", "first")], None);

        let ctx = TestCtx {
            value: "world".to_string(),
        };

        // No match, no fallback: should return None
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn test_matcher_multiple_rules() {
        let matcher = Matcher::list(
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
        let nested = Matcher::list(
            vec![create_field_matcher("will_not_match", "nested_action")],
            None, // No fallback in nested
        );

        // Parent matcher: predicate matches, but OnMatch is a nested matcher that fails
        let parent = Matcher::list(
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
        let simple = Matcher::<TestCtx, String>::list(vec![create_field_matcher("x", "y")], None);
        // Matcher depth 1 + predicate depth 1 = 2
        assert_eq!(simple.depth(), 2);
    }

    #[test]
    fn test_validate_shallow_matcher_ok() {
        let matcher = Matcher::<TestCtx, String>::list(vec![create_field_matcher("x", "y")], None);
        assert!(matcher.validate().is_ok());
    }

    #[test]
    fn test_validate_deeply_nested_matcher_fails() {
        // Build a matcher chain deeper than MAX_DEPTH
        let mut current =
            Matcher::<TestCtx, String>::list(vec![create_field_matcher("leaf", "action")], None);

        // Nest MAX_DEPTH + 1 times to exceed the limit
        // Each nesting adds 1 to depth (the wrapping Matcher)
        for _ in 0..crate::MAX_DEPTH {
            current = Matcher::list(
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
            Matcher::<TestCtx, String>::list(vec![create_field_matcher("leaf", "action")], None);

        // depth starts at 2 (1 matcher + 1 predicate), each nesting adds 1
        // We need total depth == MAX_DEPTH
        for _ in 0..(crate::MAX_DEPTH - 2) {
            current = Matcher::list(
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

    // ========== from_predicate Tests ==========

    #[test]
    fn from_predicate_with_action_and_fallback() {
        let pred = Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("hello")),
        ));
        let matcher = Matcher::from_predicate(pred, "hit".to_string(), Some("miss".to_string()));

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "hello".into()
            }),
            Some("hit".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "other".into()
            }),
            Some("miss".to_string())
        );
    }

    #[test]
    fn from_predicate_no_fallback() {
        let pred = Predicate::Single(SinglePredicate::new(
            Box::new(ValueInput),
            Box::new(ExactMatcher::new("hello")),
        ));
        let matcher: Matcher<TestCtx, String> = Matcher::from_predicate(pred, "hit".into(), None);

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "hello".into()
            }),
            Some("hit".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "other".into()
            }),
            None
        );
    }

    // ========== Trace Tests ==========

    #[test]
    fn trace_first_match_wins() {
        let matcher = Matcher::list(
            vec![
                create_field_matcher("hello", "first"),
                create_field_matcher("hello", "second"),
            ],
            None,
        );

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        assert_eq!(trace.result, Some("first".to_string()));
        assert!(!trace.used_fallback);
        // Only one step: stopped at first match
        assert_eq!(steps_of(&trace).len(), 1);
        assert!(steps_of(&trace)[0].matched);
        assert_eq!(steps_of(&trace)[0].index, 0);
    }

    #[test]
    fn trace_second_match() {
        let matcher = Matcher::list(
            vec![
                create_field_matcher("nope", "first"),
                create_field_matcher("hello", "second"),
            ],
            None,
        );

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        assert_eq!(trace.result, Some("second".to_string()));
        assert_eq!(steps_of(&trace).len(), 2);
        assert!(!steps_of(&trace)[0].matched); // first didn't match
        assert!(steps_of(&trace)[1].matched); // second matched
    }

    #[test]
    fn trace_no_match_with_fallback() {
        let matcher = Matcher::list(
            vec![create_field_matcher("nope", "first")],
            Some(OnMatch::action("fallback".to_string())),
        );

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        assert_eq!(trace.result, Some("fallback".to_string()));
        assert!(trace.used_fallback);
        assert_eq!(steps_of(&trace).len(), 1);
        assert!(!steps_of(&trace)[0].matched);
    }

    #[test]
    fn trace_no_match_no_fallback() {
        let matcher: Matcher<TestCtx, String> =
            Matcher::list(vec![create_field_matcher("nope", "first")], None);

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        assert_eq!(trace.result, None);
        assert!(!trace.used_fallback);
        assert_eq!(steps_of(&trace).len(), 1);
    }

    #[test]
    fn trace_nested_matcher_success() {
        let nested = Matcher::list(vec![create_field_matcher("hello", "nested_action")], None);

        let parent = Matcher::list(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("hello")),
                )),
                OnMatch::matcher(nested),
            )],
            None,
        );

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = parent.evaluate_with_trace(&ctx);

        assert_eq!(trace.result, Some("nested_action".to_string()));
        assert_eq!(steps_of(&trace).len(), 1);
        assert!(steps_of(&trace)[0].matched);

        // Verify nested trace exists
        match &steps_of(&trace)[0].on_match {
            Some(OnMatchTrace::Nested(nested_trace)) => {
                assert_eq!(nested_trace.result, Some("nested_action".to_string()));
                assert_eq!(steps_of(nested_trace).len(), 1);
            }
            _ => panic!("expected nested trace"),
        }
    }

    #[test]
    fn trace_nested_matcher_failure_propagates() {
        let nested = Matcher::list(
            vec![create_field_matcher("will_not_match", "nested_action")],
            None,
        );

        let parent = Matcher::list(
            vec![
                FieldMatcher::new(
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ValueInput),
                        Box::new(ExactMatcher::new("hello")),
                    )),
                    OnMatch::matcher(nested),
                ),
                create_field_matcher("hello", "second_action"),
            ],
            None,
        );

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = parent.evaluate_with_trace(&ctx);

        // Nested failed, fell through to second field matcher
        assert_eq!(trace.result, Some("second_action".to_string()));
        assert_eq!(steps_of(&trace).len(), 2);

        // First step: predicate matched but nested returned None
        assert!(steps_of(&trace)[0].matched);
        match &steps_of(&trace)[0].on_match {
            Some(OnMatchTrace::Nested(nested_trace)) => {
                assert_eq!(nested_trace.result, None);
            }
            _ => panic!("expected nested trace"),
        }

        // Second step: matched with action
        assert!(steps_of(&trace)[1].matched);
    }

    #[test]
    fn trace_result_matches_evaluate() {
        // Various matchers — trace result must always match evaluate result
        let cases: Vec<(Matcher<TestCtx, String>, TestCtx)> = vec![
            // Match
            (
                Matcher::list(vec![create_field_matcher("hello", "hit")], None),
                TestCtx {
                    value: "hello".into(),
                },
            ),
            // No match, no fallback
            (
                Matcher::list(vec![create_field_matcher("nope", "hit")], None),
                TestCtx {
                    value: "hello".into(),
                },
            ),
            // No match, with fallback
            (
                Matcher::list(
                    vec![create_field_matcher("nope", "hit")],
                    Some(OnMatch::action("fallback".into())),
                ),
                TestCtx {
                    value: "hello".into(),
                },
            ),
            // Empty matcher
            (
                Matcher::empty(),
                TestCtx {
                    value: "hello".into(),
                },
            ),
        ];

        for (matcher, ctx) in &cases {
            let eval_result = matcher.evaluate(ctx);
            let trace = matcher.evaluate_with_trace(ctx);
            assert_eq!(
                eval_result, trace.result,
                "trace result diverged from evaluate result"
            );
        }
    }

    #[test]
    fn trace_on_match_action_captured() {
        let matcher = Matcher::list(vec![create_field_matcher("hello", "the_action")], None);

        let ctx = TestCtx {
            value: "hello".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        match &steps_of(&trace)[0].on_match {
            Some(OnMatchTrace::Action(a)) => assert_eq!(a, "the_action"),
            _ => panic!("expected Action in on_match trace"),
        }
    }

    #[test]
    fn nesting_through_a_tree_counts_toward_the_depth_limit() {
        // The regression this exists for: `MatcherTree` had no `depth()`, and
        // `Matcher::depth()` walked only the list arm, so a chain of
        // tree -> onMatch.matcher -> tree reported depth 1 and validated
        // clean. Evaluation is recursive, so that was a config-triggerable
        // stack overflow behind the check meant to prevent it. D-045.
        fn tree_chain(levels: usize) -> Matcher<TestCtx, String> {
            let mut inner = Matcher::list(vec![], Some(OnMatch::Action("leaf".to_string())));
            for _ in 0..levels {
                let tree = crate::MatcherTree::exact(
                    Box::new(ValueInput),
                    [("k", OnMatch::Matcher(Box::new(inner)))],
                )
                .expect("valid tree");
                inner = Matcher::tree(tree, None);
            }
            inner
        }

        // A shallow chain is fine and its depth reflects the nesting.
        let shallow = tree_chain(3);
        assert!(
            shallow.depth() > 3,
            "depth {} did not count the trees",
            shallow.depth()
        );
        shallow
            .validate()
            .expect("3 levels is well under the limit");

        // A deep one is rejected rather than overflowing the stack later.
        let deep = tree_chain(crate::MAX_DEPTH + 5);
        let err = deep.validate().unwrap_err();
        assert!(
            matches!(err, MatcherError::DepthExceeded { .. }),
            "expected DepthExceeded, got {err:?}"
        );
    }

    #[test]
    fn a_tree_trace_reports_the_lookup_not_a_fabricated_predicate() {
        let tree = crate::MatcherTree::prefix(
            Box::new(ValueInput),
            [
                ("/api", OnMatch::Action("api".to_string())),
                ("/api/v2", OnMatch::Action("api_v2".to_string())),
            ],
        )
        .expect("valid tree");
        let matcher = Matcher::tree(tree, Some(OnMatch::Action("fallback".to_string())));

        let ctx = TestCtx {
            value: "/api/v2/users".into(),
        };
        let trace = matcher.evaluate_with_trace(&ctx);

        // INV-3: the trace's result is what `evaluate` returns.
        assert_eq!(trace.result, matcher.evaluate(&ctx));
        assert_eq!(trace.result, Some("api_v2".to_string()));

        let lookup = match &trace.steps {
            EvalSteps::Tree(t) => t,
            EvalSteps::List(_) => panic!("a tree matcher traced as a list"),
        };
        assert_eq!(lookup.kind, crate::TreeKind::Prefix);
        assert_eq!(lookup.key.as_deref(), Some("/api/v2/users"));
        // The winning prefix, not the lookup key — the non-obvious step.
        assert_eq!(
            lookup.winner().map(|a| a.matched_key.as_str()),
            Some("/api/v2")
        );
    }

    #[test]
    fn a_tree_trace_separates_no_key_from_no_entry() {
        #[derive(Debug)]
        struct AbsentInput;
        impl DataInput<TestCtx> for AbsentInput {
            fn get(&self, _ctx: &TestCtx) -> Option<MatchingData> {
                None
            }
        }

        let no_key = Matcher::tree(
            crate::MatcherTree::exact(
                Box::new(AbsentInput),
                [("a", OnMatch::Action("a".to_string()))],
            )
            .expect("valid tree"),
            None,
        );
        let ctx = TestCtx {
            value: "anything".into(),
        };
        match &no_key.evaluate_with_trace(&ctx).steps {
            EvalSteps::Tree(t) => {
                assert!(t.key.is_none(), "no usable key should be reported as such");
            }
            EvalSteps::List(_) => panic!("expected a tree trace"),
        }

        let no_entry = Matcher::tree(
            crate::MatcherTree::exact(
                Box::new(ValueInput),
                [("a", OnMatch::Action("a".to_string()))],
            )
            .expect("valid tree"),
            None,
        );
        let ctx = TestCtx { value: "z".into() };
        match &no_entry.evaluate_with_trace(&ctx).steps {
            EvalSteps::Tree(t) => {
                assert_eq!(t.key.as_deref(), Some("z"));
                assert!(t.attempts.is_empty());
            }
            EvalSteps::List(_) => panic!("expected a tree trace"),
        }
    }

    #[test]
    fn path_cost_adds_a_lists_rules_and_takes_a_trees_dearest_entry() {
        let leaf = || {
            FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("x")),
                )),
                OnMatch::Action("leaf".to_string()),
            )
        };
        let nested = |n: usize| Matcher::list((0..n).map(|_| leaf()).collect(), None);
        let under = |n: usize| OnMatch::Matcher(Box::new(nested(n)));
        let rule = |n: usize| {
            FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("t")),
                )),
                under(n),
            )
        };

        let list = Matcher::list(vec![rule(100), rule(100), rule(100)], None);
        assert_eq!(list.path_cost(), 303);
        assert!(matches!(
            list.validate(),
            Err(MatcherError::TooManyPredicatesPerEvaluation { count: 303, max })
                if max == crate::MAX_PREDICATES_PER_EVALUATION
        ));

        let exact = Matcher::tree(
            MatcherTree::exact(
                Box::new(ValueInput),
                [("a", under(100)), ("b", under(200)), ("c", under(100))],
            )
            .unwrap(),
            Some(under(50)),
        );
        assert_eq!(exact.path_cost(), 250, "dearest entry plus the fallback");
        assert!(exact.validate().is_ok());

        let siblings = Matcher::tree(
            MatcherTree::prefix(
                Box::new(ValueInput),
                [("/x", under(200)), ("/y", under(200)), ("/z", under(200))],
            )
            .unwrap(),
            None,
        );
        assert_eq!(siblings.path_cost(), 200);

        let chain = Matcher::tree(
            MatcherTree::prefix(
                Box::new(ValueInput),
                [
                    ("/", under(100)),
                    ("/a", under(100)),
                    ("/a/b", under(100)),
                    ("/z", under(200)),
                ],
            )
            .unwrap(),
            None,
        );
        assert_eq!(chain.path_cost(), 300, "/a/b falls through to /a and /");
    }

    #[test]
    fn a_prefix_tree_falls_through_to_the_next_shorter_prefix_and_traces_every_attempt() {
        let nested = Matcher::list(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(ValueInput),
                    Box::new(ExactMatcher::new("never")),
                )),
                OnMatch::Action("api_admin".to_string()),
            )],
            None,
        );
        let tree = crate::MatcherTree::prefix(
            Box::new(ValueInput),
            [
                ("/", OnMatch::Action("root".to_string())),
                ("/api", OnMatch::Matcher(Box::new(nested))),
            ],
        )
        .expect("valid tree");
        let matcher = Matcher::tree(tree, Some(OnMatch::Action("fallback".to_string())));
        let ctx = TestCtx {
            value: "/api/users".into(),
        };

        assert_eq!(matcher.evaluate(&ctx), Some("root".to_string()));

        let trace = matcher.evaluate_with_trace(&ctx);
        assert_eq!(trace.result, matcher.evaluate(&ctx));
        assert!(!trace.used_fallback);
        let EvalSteps::Tree(lookup) = &trace.steps else {
            panic!("expected a tree trace");
        };
        let tried: Vec<&str> = lookup
            .attempts
            .iter()
            .map(|a| a.matched_key.as_str())
            .collect();
        assert_eq!(tried, ["/api", "/"]);
        assert!(!lookup.attempts[0].on_match.produced());
        assert_eq!(lookup.winner().map(|a| a.matched_key.as_str()), Some("/"));
    }

    #[test]
    fn a_tree_hit_whose_nested_matcher_fails_reaches_on_no_match() {
        // Row 4 of the lookup table: a key hit is not the same as a result, so
        // it must still fall through — the same rule a list follows when a
        // nested matcher returns None.
        let dead_end = Matcher::list(vec![], None);
        let tree = crate::MatcherTree::exact(
            Box::new(ValueInput),
            [("hit", OnMatch::Matcher(Box::new(dead_end)))],
        )
        .expect("valid tree");
        let matcher = Matcher::tree(tree, Some(OnMatch::Action("fallback".to_string())));

        let ctx = TestCtx {
            value: "hit".into(),
        };
        assert_eq!(matcher.evaluate(&ctx), Some("fallback".to_string()));

        let trace = matcher.evaluate_with_trace(&ctx);
        assert_eq!(trace.result, matcher.evaluate(&ctx), "INV-3");
        assert!(trace.used_fallback);
    }
}
