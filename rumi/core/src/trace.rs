//! Evaluation trace types for debugging matcher behavior.
//!
//! Trace types mirror the runtime types ([`Predicate`](crate::Predicate),
//! [`Matcher`](crate::Matcher)) but capture evaluation results instead of
//! inputs. Use `evaluate_with_trace()` to get full visibility into the
//! matcher engine's decision path.
//!
//! # Two Levels of Trace
//!
//! - [`PredicateTrace`] — Per-predicate: which sub-expressions matched?
//! - [`EvalTrace`] — Per-matcher: which field matchers fired, what path was taken?
//!
//! # Example
//!
//! ```ignore
//! let trace = matcher.evaluate_with_trace(&ctx);
//! println!("Result: {:?}", trace.result);
//! for step in &trace.steps {
//!     println!("  field_matcher[{}]: matched={}", step.index, step.matched);
//! }
//! ```

use crate::MatchResult;
use std::fmt;

/// Trace of a predicate evaluation.
///
/// Mirrors [`Predicate`](crate::Predicate) structure but captures results.
///
/// In And/Or, ALL children are evaluated (no short-circuit) for maximum
/// debugging value. The `matched` result is still correct.
pub enum PredicateTrace {
    /// A single predicate evaluation.
    Single {
        /// What this predicate decided.
        result: MatchResult,
        /// Debug description of the `DataInput` (e.g., `"ToolNameInput"`).
        input: String,
        /// The `MatchingData` extracted from context (Debug format).
        data: String,
        /// Debug description of the `InputMatcher` (e.g., `"ExactMatcher(\"Bash\")"`).
        matcher: String,
    },
    /// AND: all children must match.
    And {
        /// The first child that is not a match, or a match (D-069).
        result: MatchResult,
        /// Trace of each child (all evaluated, no short-circuit).
        children: Vec<PredicateTrace>,
    },
    /// OR: any child must match.
    Or {
        /// A match if any child matched, else undecided if any was (D-069).
        result: MatchResult,
        /// Trace of each child (all evaluated, no short-circuit).
        children: Vec<PredicateTrace>,
    },
    /// NOT: inverts inner result.
    Not {
        /// The inner result negated; undecided stays undecided (D-069).
        result: MatchResult,
        /// Trace of the inner predicate.
        inner: Box<PredicateTrace>,
    },
}

impl PredicateTrace {
    /// What this predicate decided.
    #[must_use]
    pub fn result(&self) -> MatchResult {
        match self {
            Self::Single { result, .. }
            | Self::And { result, .. }
            | Self::Or { result, .. }
            | Self::Not { result, .. } => *result,
        }
    }

    /// Whether this predicate matched; an undecided predicate did not.
    #[must_use]
    pub fn matched(&self) -> bool {
        self.result() == MatchResult::Matched
    }
}

impl fmt::Debug for PredicateTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single {
                result,
                input,
                data,
                matcher,
            } => f
                .debug_struct("Single")
                .field("result", result)
                .field("input", input)
                .field("data", data)
                .field("matcher", matcher)
                .finish(),
            Self::And { result, children } => f
                .debug_struct("And")
                .field("result", result)
                .field("children", children)
                .finish(),
            Self::Or { result, children } => f
                .debug_struct("Or")
                .field("result", result)
                .field("children", children)
                .finish(),
            Self::Not { result, inner } => f
                .debug_struct("Not")
                .field("result", result)
                .field("inner", inner)
                .finish(),
        }
    }
}

/// Trace of a full [`Matcher`](crate::Matcher) evaluation.
///
/// Contains the same result as `evaluate()` plus the full evaluation path:
/// which field matchers were checked, which predicates fired, and whether
/// the fallback was used.
///
/// # INV: `result` == `evaluate()` result
///
/// The `result` field always equals what [`Matcher::evaluate()`](crate::Matcher::evaluate)
/// would return for the same input.
pub struct EvalTrace<A> {
    /// The final result (identical to what `evaluate()` returns).
    pub result: Option<A>,
    /// What the matcher actually did, shaped like the matcher itself.
    pub steps: EvalSteps<A>,
    /// Whether the `on_no_match` fallback was used.
    pub used_fallback: bool,
    /// Whether a required value was absent, so nothing was decided (D-069).
    /// When set, `result` is `None`.
    pub undecided: bool,
}

impl<A: Clone> EvalTrace<A> {
    /// The outcome this trace records, identical to
    /// [`Matcher::outcome`](crate::Matcher::outcome).
    #[must_use]
    pub fn outcome(&self) -> crate::Outcome<A> {
        match (&self.result, self.undecided) {
            (_, true) => crate::Outcome::Undecided,
            (Some(a), false) => crate::Outcome::Action(a.clone()),
            (None, false) => crate::Outcome::NoMatch,
        }
    }
}

/// The traced work, mirroring `MatcherKind`.
///
/// A tree does a map lookup, not a predicate evaluation, so there is no
/// `Predicate` to trace. Synthesizing one would put a `SinglePredicate` in the
/// output that is not in the config — a trace that invents structure is a
/// second, wrong source of truth. Reporting no steps at all would be equally
/// dishonest, and would break the property that a non-fallback result implies
/// at least one recorded step.
pub enum EvalSteps<A> {
    /// Each field matcher evaluated, in order. Stops at the first match.
    List(Vec<EvalStep<A>>),
    /// The single lookup a tree performed.
    Tree(Box<TreeLookupTrace<A>>),
}

impl<A> EvalSteps<A> {
    /// The field-matcher steps, if this was a list matcher.
    #[must_use]
    pub fn as_list(&self) -> Option<&[EvalStep<A>]> {
        match self {
            Self::List(steps) => Some(steps),
            Self::Tree(_) => None,
        }
    }
}

impl<A: fmt::Debug> fmt::Debug for EvalSteps<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(steps) => f.debug_tuple("List").field(steps).finish(),
            Self::Tree(t) => f.debug_tuple("Tree").field(t).finish(),
        }
    }
}

/// Which lookup rule a tree applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKind {
    /// Exact key equality.
    Exact,
    /// Longest matching prefix.
    Prefix,
}

/// One tree lookup, recorded.
pub struct TreeLookupTrace<A> {
    /// Which rule was applied.
    pub kind: TreeKind,
    /// The `DataInput` that produced the lookup key.
    pub input: String,
    /// The key extracted from the context.
    ///
    /// `None` means the input produced nothing usable as a string — absent
    /// data, or an `Int`/`Bool`/`Bytes`. Distinguishing that from a key that
    /// simply found no entry is the whole point of recording it: otherwise
    /// three different config faults collapse into one silent miss.
    pub key: Option<String>,
    /// Every entry the key selected, in the order tried, with what each did.
    ///
    /// Empty when the key selected nothing. Evaluation stops at the first
    /// entry that produces an action, so the last attempt is the winner unless
    /// every attempt missed; for a prefix tree, an earlier attempt is a longer
    /// prefix whose nested matcher produced nothing (D-055).
    pub attempts: Vec<TreeAttempt<A>>,
}

impl<A> TreeLookupTrace<A> {
    /// The attempt that produced the result, if one did.
    #[must_use]
    pub fn winner(&self) -> Option<&TreeAttempt<A>> {
        self.attempts.last().filter(|a| a.on_match.produced())
    }
}

impl<A: fmt::Debug> fmt::Debug for TreeLookupTrace<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeLookupTrace")
            .field("kind", &self.kind)
            .field("input", &self.input)
            .field("key", &self.key)
            .field("attempts", &self.attempts)
            .finish()
    }
}

/// One entry a tree lookup selected, and what it did.
pub struct TreeAttempt<A> {
    /// The entry key. For a prefix tree this is the matched prefix rather than
    /// the lookup key, which is the non-obvious step.
    pub matched_key: String,
    /// What the entry did.
    pub on_match: OnMatchTrace<A>,
}

impl<A: fmt::Debug> fmt::Debug for TreeAttempt<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeAttempt")
            .field("matched_key", &self.matched_key)
            .field("on_match", &self.on_match)
            .finish()
    }
}

impl<A: fmt::Debug> fmt::Debug for EvalTrace<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalTrace")
            .field("result", &self.result)
            .field("steps", &self.steps)
            .field("used_fallback", &self.used_fallback)
            .field("undecided", &self.undecided)
            .finish()
    }
}

/// One field matcher's evaluation in a trace.
pub struct EvalStep<A> {
    /// Index in `matcher_list` (0-based).
    pub index: usize,
    /// Did the predicate match?
    pub matched: bool,
    /// Full predicate evaluation trace.
    pub predicate_trace: PredicateTrace,
    /// If predicate matched, what happened with `OnMatch`?
    pub on_match: Option<OnMatchTrace<A>>,
}

impl<A: fmt::Debug> fmt::Debug for EvalStep<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalStep")
            .field("index", &self.index)
            .field("matched", &self.matched)
            .field("predicate_trace", &self.predicate_trace)
            .field("on_match", &self.on_match)
            .finish()
    }
}

/// What happened when a matched predicate resolved its `OnMatch`.
pub enum OnMatchTrace<A> {
    /// Returned an action directly.
    Action(A),
    /// Delegated to a nested matcher (recursive trace).
    Nested(Box<EvalTrace<A>>),
}

impl<A> OnMatchTrace<A> {
    /// Whether this resolution produced an action.
    #[must_use]
    pub fn produced(&self) -> bool {
        match self {
            Self::Action(_) => true,
            Self::Nested(trace) => trace.result.is_some(),
        }
    }
}

impl<A: fmt::Debug> fmt::Debug for OnMatchTrace<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(a) => f.debug_tuple("Action").field(a).finish(),
            Self::Nested(t) => f.debug_tuple("Nested").field(t).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_trace_matched_single() {
        let trace = PredicateTrace::Single {
            result: MatchResult::Matched,
            input: "TestInput".into(),
            data: "String(\"hello\")".into(),
            matcher: "ExactMatcher(\"hello\")".into(),
        };
        assert!(trace.matched());
    }

    #[test]
    fn predicate_trace_matched_and() {
        let trace = PredicateTrace::And {
            result: MatchResult::NoMatch,
            children: vec![],
        };
        assert!(!trace.matched());
    }

    #[test]
    fn predicate_trace_matched_or() {
        let trace = PredicateTrace::Or {
            result: MatchResult::Matched,
            children: vec![],
        };
        assert!(trace.matched());
    }

    #[test]
    fn predicate_trace_matched_not() {
        let trace = PredicateTrace::Not {
            result: MatchResult::Matched,
            inner: Box::new(PredicateTrace::Single {
                result: MatchResult::NoMatch,
                input: String::new(),
                data: String::new(),
                matcher: String::new(),
            }),
        };
        assert!(trace.matched());
    }

    #[test]
    fn predicate_trace_debug_format() {
        let trace = PredicateTrace::Single {
            result: MatchResult::Matched,
            input: "PathInput".into(),
            data: "String(\"/api\")".into(),
            matcher: "ExactMatcher(\"/api\")".into(),
        };
        let debug = format!("{trace:?}");
        assert!(debug.contains("PathInput"));
        assert!(debug.contains("/api"));
    }

    #[test]
    fn eval_trace_debug_format() {
        let trace: EvalTrace<String> = EvalTrace {
            result: Some("matched".into()),
            steps: EvalSteps::List(vec![]),
            undecided: false,
            used_fallback: false,
        };
        let debug = format!("{trace:?}");
        assert!(debug.contains("matched"));
    }
}
