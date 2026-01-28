//! OnMatch — What to do when a predicate matches
//!
//! Per xDS proto semantics, `OnMatch` is **exclusive**: either an action OR
//! a nested matcher, never both. This is enforced at the type level with an enum.

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;

use crate::Matcher;
use core::fmt::Debug;

/// Defines what to do when a predicate matches.
///
/// **xDS Semantics**: OnMatch is a `oneof` in the xDS proto — you can have
/// an action OR a nested matcher, but never both. This Rust enum enforces
/// that constraint at compile time.
///
/// # Type Parameters
///
/// - `Ctx`: The context type for nested matchers
/// - `A`: The action type (must be `Clone + Send + Sync + 'static`)
///
/// # Example
///
/// ```ignore
/// // Action only
/// let on_match = OnMatch::Action("route_to_backend".to_string());
///
/// // Nested matcher only
/// let on_match = OnMatch::Matcher(Box::new(nested_matcher));
/// ```
pub enum OnMatch<Ctx, A: Clone + Send + Sync + 'static> {
    /// Execute this action when matched.
    Action(A),

    /// Continue evaluation into nested matcher.
    /// If nested matcher returns no match, this OnMatch also returns no match.
    Matcher(Box<Matcher<Ctx, A>>),
}

impl<Ctx, A: Clone + Send + Sync + 'static> OnMatch<Ctx, A> {
    /// Create an OnMatch with an action.
    pub fn action(action: A) -> Self {
        Self::Action(action)
    }

    /// Create an OnMatch with a nested matcher.
    pub fn matcher(nested: Matcher<Ctx, A>) -> Self {
        Self::Matcher(Box::new(nested))
    }

    /// Returns `true` if this is an action.
    pub fn is_action(&self) -> bool {
        matches!(self, Self::Action(_))
    }

    /// Returns `true` if this is a nested matcher.
    pub fn is_matcher(&self) -> bool {
        matches!(self, Self::Matcher(_))
    }

    /// Get the action if this is an `Action` variant.
    pub fn as_action(&self) -> Option<&A> {
        match self {
            Self::Action(a) => Some(a),
            Self::Matcher(_) => None,
        }
    }

    /// Get the nested matcher if this is a `Matcher` variant.
    pub fn as_matcher(&self) -> Option<&Matcher<Ctx, A>> {
        match self {
            Self::Action(_) => None,
            Self::Matcher(m) => Some(m),
        }
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for OnMatch<Ctx, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Action(a) => f.debug_tuple("Action").field(a).finish(),
            Self::Matcher(_) => f.debug_tuple("Matcher").field(&"...").finish(),
        }
    }
}

impl<Ctx, A: Clone + Send + Sync + 'static> Clone for OnMatch<Ctx, A>
where
    Matcher<Ctx, A>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Action(a) => Self::Action(a.clone()),
            Self::Matcher(m) => Self::Matcher(m.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestCtx;

    #[test]
    fn test_on_match_action() {
        let on_match: OnMatch<TestCtx, String> = OnMatch::action("test".to_string());
        assert!(on_match.is_action());
        assert!(!on_match.is_matcher());
        assert_eq!(on_match.as_action(), Some(&"test".to_string()));
    }

    #[test]
    fn test_on_match_matcher() {
        let nested = Matcher::<TestCtx, String>::empty();
        let on_match: OnMatch<TestCtx, String> = OnMatch::matcher(nested);
        assert!(!on_match.is_action());
        assert!(on_match.is_matcher());
        assert!(on_match.as_matcher().is_some());
    }

    #[test]
    fn test_on_match_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OnMatch<TestCtx, String>>();
    }
}
