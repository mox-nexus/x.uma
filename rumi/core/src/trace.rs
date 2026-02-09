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
        /// Whether this predicate matched.
        matched: bool,
        /// Debug description of the `DataInput` (e.g., `"ToolNameInput"`).
        input: String,
        /// The `MatchingData` extracted from context (Debug format).
        data: String,
        /// Debug description of the `InputMatcher` (e.g., `"ExactMatcher(\"Bash\")"`).
        matcher: String,
    },
    /// AND: all children must match.
    And {
        /// Whether all children matched.
        matched: bool,
        /// Trace of each child (all evaluated, no short-circuit).
        children: Vec<PredicateTrace>,
    },
    /// OR: any child must match.
    Or {
        /// Whether any child matched.
        matched: bool,
        /// Trace of each child (all evaluated, no short-circuit).
        children: Vec<PredicateTrace>,
    },
    /// NOT: inverts inner result.
    Not {
        /// Whether the NOT predicate matched (i.e., inner did NOT match).
        matched: bool,
        /// Trace of the inner predicate.
        inner: Box<PredicateTrace>,
    },
}

impl PredicateTrace {
    /// Get the overall match result of this predicate.
    #[must_use]
    pub fn matched(&self) -> bool {
        match self {
            Self::Single { matched, .. }
            | Self::And { matched, .. }
            | Self::Or { matched, .. }
            | Self::Not { matched, .. } => *matched,
        }
    }
}

impl fmt::Debug for PredicateTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single {
                matched,
                input,
                data,
                matcher,
            } => f
                .debug_struct("Single")
                .field("matched", matched)
                .field("input", input)
                .field("data", data)
                .field("matcher", matcher)
                .finish(),
            Self::And { matched, children } => f
                .debug_struct("And")
                .field("matched", matched)
                .field("children", children)
                .finish(),
            Self::Or { matched, children } => f
                .debug_struct("Or")
                .field("matched", matched)
                .field("children", children)
                .finish(),
            Self::Not { matched, inner } => f
                .debug_struct("Not")
                .field("matched", matched)
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
    /// Trace of each field matcher that was evaluated (in order).
    /// Stops after the first match (preserves first-match-wins).
    pub steps: Vec<EvalStep<A>>,
    /// Whether the `on_no_match` fallback was used.
    pub used_fallback: bool,
}

impl<A: fmt::Debug> fmt::Debug for EvalTrace<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalTrace")
            .field("result", &self.result)
            .field("steps", &self.steps)
            .field("used_fallback", &self.used_fallback)
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
            matched: true,
            input: "TestInput".into(),
            data: "String(\"hello\")".into(),
            matcher: "ExactMatcher(\"hello\")".into(),
        };
        assert!(trace.matched());
    }

    #[test]
    fn predicate_trace_matched_and() {
        let trace = PredicateTrace::And {
            matched: false,
            children: vec![],
        };
        assert!(!trace.matched());
    }

    #[test]
    fn predicate_trace_matched_or() {
        let trace = PredicateTrace::Or {
            matched: true,
            children: vec![],
        };
        assert!(trace.matched());
    }

    #[test]
    fn predicate_trace_matched_not() {
        let trace = PredicateTrace::Not {
            matched: true,
            inner: Box::new(PredicateTrace::Single {
                matched: false,
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
            matched: true,
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
            steps: vec![],
            used_fallback: false,
        };
        let debug = format!("{trace:?}");
        assert!(debug.contains("matched"));
    }
}
