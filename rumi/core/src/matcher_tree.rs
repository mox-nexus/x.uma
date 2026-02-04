//! `MatcherTree` — Map-based matching with O(1) exact and O(k) prefix lookup.
//!
//! Implements the xDS `MatcherTree` semantics: extract a key via `DataInput`,
//! then look up in either an exact map (hash) or prefix map (radix tree).

use crate::{radix_tree::RadixTree, DataInput, OnMatch};
use std::collections::HashMap;
use std::fmt::Debug;

/// Tree-based matcher using map lookups instead of predicate evaluation.
///
/// This is the xDS `MatcherTree` pattern: extract a key from the context,
/// then look it up in a map. More efficient than linear predicate scanning
/// when routing on a single key.
///
/// # Variants
///
/// - `ExactMatch` — O(1) hash map lookup
/// - `PrefixMatch` — O(k) radix tree lookup, longest prefix wins
///
/// # Example
///
/// ```ignore
/// let tree = MatcherTree::exact(
///     Box::new(PathInput),
///     [
///         ("/health", OnMatch::Action("health_check")),
///         ("/ready", OnMatch::Action("readiness")),
///     ],
///     Some(OnMatch::Action("default")),
/// );
/// ```
pub enum MatcherTree<Ctx, A: Clone + Send + Sync + 'static> {
    /// O(1) exact string lookup.
    ExactMatch {
        /// Extracts the lookup key from context.
        input: Box<dyn DataInput<Ctx>>,
        /// Map from exact key to action.
        map: HashMap<String, OnMatch<Ctx, A>>,
        /// Fallback when no key matches.
        on_no_match: Option<OnMatch<Ctx, A>>,
    },

    /// O(k) prefix lookup, longest matching prefix wins.
    PrefixMatch {
        /// Extracts the lookup key from context.
        input: Box<dyn DataInput<Ctx>>,
        /// Radix tree mapping prefixes to actions.
        tree: RadixTree<OnMatch<Ctx, A>>,
        /// Fallback when no prefix matches.
        on_no_match: Option<OnMatch<Ctx, A>>,
    },
}

impl<Ctx, A: Clone + Send + Sync + 'static> MatcherTree<Ctx, A> {
    /// Create an exact-match tree.
    ///
    /// Uses O(1) hash lookup. Keys must match exactly.
    pub fn exact<K, I>(
        input: Box<dyn DataInput<Ctx>>,
        entries: I,
        on_no_match: Option<OnMatch<Ctx, A>>,
    ) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, OnMatch<Ctx, A>)>,
    {
        let map = entries
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();

        Self::ExactMatch {
            input,
            map,
            on_no_match,
        }
    }

    /// Create a prefix-match tree.
    ///
    /// Uses O(k) radix tree lookup. Longest matching prefix wins.
    pub fn prefix<K, I>(
        input: Box<dyn DataInput<Ctx>>,
        entries: I,
        on_no_match: Option<OnMatch<Ctx, A>>,
    ) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, OnMatch<Ctx, A>)>,
    {
        let mut tree = RadixTree::new();
        for (key, on_match) in entries {
            tree.insert(&key.into(), on_match);
        }

        Self::PrefixMatch {
            input,
            tree,
            on_no_match,
        }
    }

    /// Evaluate the tree against a context.
    ///
    /// Returns the action from the matching entry, or from `on_no_match`,
    /// or `None` if nothing matches.
    pub fn evaluate(&self, ctx: &Ctx) -> Option<A> {
        match self {
            Self::ExactMatch {
                input,
                map,
                on_no_match,
            } => {
                let data = input.get(ctx);
                if let Some(key) = data.as_str() {
                    if let Some(on_match) = map.get(key) {
                        return on_match.evaluate(ctx);
                    }
                }
                on_no_match.as_ref().and_then(|m| m.evaluate(ctx))
            }

            Self::PrefixMatch {
                input,
                tree,
                on_no_match,
            } => {
                let data = input.get(ctx);
                if let Some(key) = data.as_str() {
                    if let Some(on_match) = tree.find_longest_prefix(key) {
                        return on_match.evaluate(ctx);
                    }
                }
                on_no_match.as_ref().and_then(|m| m.evaluate(ctx))
            }
        }
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for MatcherTree<Ctx, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExactMatch { input, map, .. } => f
                .debug_struct("ExactMatch")
                .field("input", input)
                .field("entries", &map.len())
                .finish(),
            Self::PrefixMatch { input, tree, .. } => f
                .debug_struct("PrefixMatch")
                .field("input", input)
                .field("tree", tree)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MatchingData;

    #[derive(Debug, Clone)]
    struct TestContext {
        path: String,
    }

    #[derive(Debug)]
    struct PathInput;

    impl DataInput<TestContext> for PathInput {
        fn get(&self, ctx: &TestContext) -> MatchingData {
            MatchingData::String(ctx.path.clone())
        }
    }

    #[test]
    fn test_exact_match() {
        let tree: MatcherTree<TestContext, String> = MatcherTree::exact(
            Box::new(PathInput),
            [
                ("/health", OnMatch::Action("health".into())),
                ("/ready", OnMatch::Action("ready".into())),
            ],
            Some(OnMatch::Action("default".into())),
        );

        let ctx = TestContext { path: "/health".into() };
        assert_eq!(tree.evaluate(&ctx), Some("health".into()));

        let ctx = TestContext { path: "/ready".into() };
        assert_eq!(tree.evaluate(&ctx), Some("ready".into()));

        let ctx = TestContext { path: "/other".into() };
        assert_eq!(tree.evaluate(&ctx), Some("default".into()));
    }

    #[test]
    fn test_prefix_match_longest_wins() {
        let tree: MatcherTree<TestContext, String> = MatcherTree::prefix(
            Box::new(PathInput),
            [
                ("/", OnMatch::Action("root".into())),
                ("/api", OnMatch::Action("api".into())),
                ("/api/v2", OnMatch::Action("api_v2".into())),
            ],
            None,
        );

        // Longest prefix wins
        let ctx = TestContext { path: "/api/v2/users".into() };
        assert_eq!(tree.evaluate(&ctx), Some("api_v2".into()));

        let ctx = TestContext { path: "/api/v1/users".into() };
        assert_eq!(tree.evaluate(&ctx), Some("api".into()));

        let ctx = TestContext { path: "/other".into() };
        assert_eq!(tree.evaluate(&ctx), Some("root".into()));

        // No match
        let ctx = TestContext { path: "nope".into() };
        assert_eq!(tree.evaluate(&ctx), None);
    }

    #[test]
    fn test_exact_no_match_fallback() {
        let tree: MatcherTree<TestContext, String> = MatcherTree::exact(
            Box::new(PathInput),
            [("/a", OnMatch::Action("a".into()))],
            None,
        );

        let ctx = TestContext { path: "/b".into() };
        assert_eq!(tree.evaluate(&ctx), None);
    }

    #[test]
    fn test_non_string_input_returns_no_match() {
        #[derive(Debug)]
        struct IntInput;

        impl DataInput<TestContext> for IntInput {
            fn get(&self, _ctx: &TestContext) -> MatchingData {
                MatchingData::Int(42)
            }
        }

        let tree: MatcherTree<TestContext, String> = MatcherTree::exact(
            Box::new(IntInput),
            [("42", OnMatch::Action("matched".into()))],
            Some(OnMatch::Action("fallback".into())),
        );

        // Int data can't be looked up in string map
        let ctx = TestContext { path: "ignored".into() };
        assert_eq!(tree.evaluate(&ctx), Some("fallback".into()));
    }
}
