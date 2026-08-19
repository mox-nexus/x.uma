//! `MatcherTree` — map-based matching with O(1) exact and O(k) prefix lookup.
//!
//! Implements the xDS `MatcherTree` semantics: extract a key via `DataInput`,
//! then look it up in either an exact map (hash) or a prefix map (radix tree).
//!
//! # No fallback lives here
//!
//! The proto `MatcherTree` has exactly two members, `input` and the `tree_type`
//! oneof. `on_no_match` belongs to the enclosing `Matcher`. This type carried
//! its own until 2026-08-18, which was harmless only because nothing could
//! construct one from config; wiring it up would have put two fallbacks on one
//! condition with no stated precedence. See `DECISIONS.md` D-044.

use crate::{radix_tree::RadixTree, DataInput, MatcherError, OnMatch};
use std::collections::HashMap;
use std::fmt::Debug;

/// Tree-based matcher using map lookups instead of predicate evaluation.
///
/// This is the xDS `MatcherTree` pattern: extract a key from the context, then
/// look it up in a map. Cheaper than a linear predicate scan when routing on a
/// single key, and the only way to express longest-prefix-wins — a
/// `MatcherList` is first-match-wins in written order, so it returns `/api` for
/// `/api/v2` whenever `/api` is listed first.
///
/// # Variants
///
/// - `ExactMatch` — O(1) hash map lookup
/// - `PrefixMatch` — O(k) radix tree lookup, longest prefix wins
pub enum MatcherTree<Ctx, A: Clone + Send + Sync + 'static> {
    /// O(1) exact string lookup.
    ExactMatch {
        /// Extracts the lookup key from context.
        input: Box<dyn DataInput<Ctx>>,
        /// Map from exact key to outcome.
        map: HashMap<String, OnMatch<Ctx, A>>,
    },

    /// O(k) prefix lookup, longest matching prefix wins.
    PrefixMatch {
        /// Extracts the lookup key from context.
        input: Box<dyn DataInput<Ctx>>,
        /// Radix tree mapping prefixes to outcomes.
        tree: RadixTree<OnMatch<Ctx, A>>,
    },
}

impl<Ctx, A: Clone + Send + Sync + 'static> MatcherTree<Ctx, A> {
    /// Create an exact-match tree. O(1) hash lookup; keys must match exactly.
    ///
    /// # Errors
    ///
    /// [`MatcherError::DuplicateTreeKey`] if two entries share a key, and
    /// [`MatcherError::IncompatibleTypes`] if the input cannot produce a
    /// string — a tree keyed on an `Int` input can never match anything, so it
    /// is rejected here rather than silently never firing.
    pub fn exact<K, I>(input: Box<dyn DataInput<Ctx>>, entries: I) -> Result<Self, MatcherError>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, OnMatch<Ctx, A>)>,
    {
        Self::require_string_input(input.as_ref())?;

        let mut map = HashMap::new();
        for (key, on_match) in entries {
            let key = key.into();
            if map.insert(key.clone(), on_match).is_some() {
                return Err(MatcherError::DuplicateTreeKey { key });
            }
        }

        Ok(Self::ExactMatch { input, map })
    }

    /// Create a prefix-match tree. O(k) radix lookup, longest prefix wins.
    ///
    /// # Errors
    ///
    /// As [`exact`](Self::exact).
    pub fn prefix<K, I>(input: Box<dyn DataInput<Ctx>>, entries: I) -> Result<Self, MatcherError>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, OnMatch<Ctx, A>)>,
    {
        Self::require_string_input(input.as_ref())?;

        let mut tree = RadixTree::new();
        for (key, on_match) in entries {
            let key = key.into();
            // `insert` returns the displaced value. Discarding it is how a
            // second rule for the same prefix used to disappear silently.
            if tree.insert(&key, on_match).is_some() {
                return Err(MatcherError::DuplicateTreeKey { key });
            }
        }

        Ok(Self::PrefixMatch { input, tree })
    }

    /// A tree looks its key up as a string, so a non-string input never matches.
    fn require_string_input(input: &dyn DataInput<Ctx>) -> Result<(), MatcherError> {
        if input.data_type() == "string" {
            return Ok(());
        }
        Err(MatcherError::IncompatibleTypes {
            input_type: input.data_type().to_string(),
            matcher_types: vec!["string".to_string()],
        })
    }

    /// The lookup key for this context, if the input produced a usable string.
    fn key(&self, ctx: &Ctx) -> Option<String> {
        let input = match self {
            Self::ExactMatch { input, .. } | Self::PrefixMatch { input, .. } => input,
        };
        input.get(ctx).as_str().map(ToOwned::to_owned)
    }

    /// The entry a key selects, and which key won.
    ///
    /// Split out from [`evaluate`](Self::evaluate) so the traced path can reuse
    /// the same decision rather than reimplement it — that is where the two
    /// would otherwise drift apart.
    fn lookup<'a>(&'a self, key: &'a str) -> Option<(&'a str, &'a OnMatch<Ctx, A>)> {
        match self {
            Self::ExactMatch { map, .. } => map
                .get_key_value(key)
                .map(|(k, on_match)| (k.as_str(), on_match)),
            Self::PrefixMatch { tree, .. } => tree.find_longest_prefix_entry(key),
        }
    }

    /// Evaluate the tree against a context.
    ///
    /// Returns `None` both when no key is available and when the key selects no
    /// entry. The enclosing [`Matcher`](crate::Matcher) owns the fallback, so
    /// there is exactly one place a miss can be handled.
    pub fn evaluate(&self, ctx: &Ctx) -> Option<A> {
        let key = self.key(ctx)?;
        let (_, on_match) = self.lookup(&key)?;
        on_match.evaluate(ctx)
    }

    /// Which rule this tree applies, and what feeds it — for traces.
    pub(crate) fn trace_identity(&self) -> (crate::TreeKind, String) {
        match self {
            Self::ExactMatch { input, .. } => (crate::TreeKind::Exact, format!("{input:?}")),
            Self::PrefixMatch { input, .. } => (crate::TreeKind::Prefix, format!("{input:?}")),
        }
    }

    pub(crate) fn trace_key(&self, ctx: &Ctx) -> Option<String> {
        self.key(ctx)
    }

    pub(crate) fn trace_lookup<'a>(
        &'a self,
        key: &'a str,
    ) -> Option<(&'a str, &'a OnMatch<Ctx, A>)> {
        self.lookup(key)
    }

    /// Number of configured entries.
    pub fn len(&self) -> usize {
        match self {
            Self::ExactMatch { map, .. } => map.len(),
            Self::PrefixMatch { tree, .. } => tree.len(),
        }
    }

    /// Returns `true` if the tree has no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every outcome this tree can dispatch to.
    fn outcomes(&self) -> Box<dyn Iterator<Item = &OnMatch<Ctx, A>> + '_> {
        match self {
            Self::ExactMatch { map, .. } => Box::new(map.values()),
            Self::PrefixMatch { tree, .. } => Box::new(tree.values()),
        }
    }

    /// Deepest nesting reachable through this tree's entries.
    ///
    /// A tree entry can hold a nested `Matcher`, which can hold another tree.
    /// While this did not exist, `Matcher::depth()` walked past tree entries
    /// entirely and such a config reported depth 1 — see `DECISIONS.md` D-045.
    pub(crate) fn depth(&self) -> usize {
        self.outcomes()
            .map(|om| match om {
                OnMatch::Action(_) => 0,
                OnMatch::Matcher(m) => m.depth(),
            })
            .max()
            .unwrap_or(0)
    }

    /// Check this tree's width, and recurse into nested matchers.
    pub(crate) fn validate_widths(&self) -> Result<(), MatcherError> {
        if self.len() > crate::MAX_TREE_ENTRIES {
            return Err(MatcherError::TooManyTreeEntries {
                count: self.len(),
                max: crate::MAX_TREE_ENTRIES,
            });
        }

        for om in self.outcomes() {
            if let OnMatch::Matcher(nested) = om {
                nested.validate_widths()?;
            }
        }

        Ok(())
    }
}

impl<Ctx, A: Clone + Send + Sync + Debug + 'static> Debug for MatcherTree<Ctx, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExactMatch { input, map } => f
                .debug_struct("ExactMatch")
                .field("input", input)
                .field("entries", &map.len())
                .finish(),
            Self::PrefixMatch { input, tree } => f
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

    fn exact(entries: Vec<(&str, &str)>) -> MatcherTree<TestContext, String> {
        MatcherTree::exact(
            Box::new(PathInput),
            entries
                .into_iter()
                .map(|(k, v)| (k, OnMatch::Action(v.to_string()))),
        )
        .expect("valid tree")
    }

    fn prefix(entries: Vec<(&str, &str)>) -> MatcherTree<TestContext, String> {
        MatcherTree::prefix(
            Box::new(PathInput),
            entries
                .into_iter()
                .map(|(k, v)| (k, OnMatch::Action(v.to_string()))),
        )
        .expect("valid tree")
    }

    fn ctx(path: &str) -> TestContext {
        TestContext { path: path.into() }
    }

    #[test]
    fn exact_lookup_hits_its_own_entry() {
        let tree = exact(vec![("/health", "health"), ("/ready", "ready")]);

        assert_eq!(tree.evaluate(&ctx("/health")), Some("health".into()));
        assert_eq!(tree.evaluate(&ctx("/ready")), Some("ready".into()));
    }

    #[test]
    fn exact_means_exact() {
        let tree = exact(vec![("/a", "a")]);

        // A miss is `None`. The fallback belongs to the enclosing `Matcher`,
        // so there is nothing here to fall back to — see D-044.
        assert_eq!(tree.evaluate(&ctx("/b")), None);
        assert_eq!(tree.evaluate(&ctx("/ab")), None);
    }

    #[test]
    fn prefix_lookup_takes_the_longest_match() {
        let tree = prefix(vec![("/", "root"), ("/api", "api"), ("/api/v2", "api_v2")]);

        assert_eq!(tree.evaluate(&ctx("/api/v2/users")), Some("api_v2".into()));
        assert_eq!(tree.evaluate(&ctx("/api/v1/users")), Some("api".into()));
        assert_eq!(tree.evaluate(&ctx("/other")), Some("root".into()));
        assert_eq!(tree.evaluate(&ctx("nope")), None);
    }

    #[test]
    fn a_duplicate_exact_key_is_rejected() {
        let err = MatcherTree::<TestContext, String>::exact(
            Box::new(PathInput),
            [
                ("/a", OnMatch::Action("first".to_string())),
                ("/a", OnMatch::Action("second".to_string())),
            ],
        )
        .unwrap_err();

        assert!(
            matches!(err, MatcherError::DuplicateTreeKey { ref key } if key == "/a"),
            "expected DuplicateTreeKey, got {err:?}"
        );
    }

    #[test]
    fn a_duplicate_prefix_key_is_rejected() {
        // `RadixTree::insert` returns the displaced value; discarding it is how
        // the second rule used to disappear without a word.
        let err = MatcherTree::<TestContext, String>::prefix(
            Box::new(PathInput),
            [
                ("/api", OnMatch::Action("first".to_string())),
                ("/api", OnMatch::Action("second".to_string())),
            ],
        )
        .unwrap_err();

        assert!(
            matches!(err, MatcherError::DuplicateTreeKey { ref key } if key == "/api"),
            "expected DuplicateTreeKey, got {err:?}"
        );
    }

    #[test]
    fn an_input_declaring_a_non_string_type_is_rejected_at_construction() {
        #[derive(Debug)]
        struct BoolInput;

        impl DataInput<TestContext> for BoolInput {
            fn get(&self, _ctx: &TestContext) -> MatchingData {
                MatchingData::Bool(true)
            }
            fn data_type(&self) -> &'static str {
                "bool"
            }
        }

        let err = MatcherTree::<TestContext, String>::exact(
            Box::new(BoolInput),
            [("true", OnMatch::Action("yes".to_string()))],
        )
        .unwrap_err();

        assert!(
            matches!(err, MatcherError::IncompatibleTypes { .. }),
            "expected IncompatibleTypes, got {err:?}"
        );
    }

    #[test]
    fn an_input_that_lies_about_its_type_still_silently_misses() {
        // This is finding F17, not a gap in the check above. `data_type()`
        // defaults to `"string"` (`data_input.rs`), so an input that returns
        // `Int` without overriding it *declares* itself a string. Construction
        // cannot catch that, and the lookup then finds nothing.
        //
        // The test exists to pin the residue: the construction check is worth
        // having, and it is bounded by the default. Closing F17 is what would
        // make this case impossible, and that is not this change.
        #[derive(Debug)]
        struct UndeclaredIntInput;

        impl DataInput<TestContext> for UndeclaredIntInput {
            fn get(&self, _ctx: &TestContext) -> MatchingData {
                MatchingData::Int(42)
            }
        }

        let tree = MatcherTree::<TestContext, String>::exact(
            Box::new(UndeclaredIntInput),
            [("42", OnMatch::Action("matched".to_string()))],
        )
        .expect("declares itself a string, so it loads");

        assert_eq!(tree.evaluate(&ctx("ignored")), None);
    }

    #[test]
    fn len_counts_entries_for_both_kinds() {
        assert_eq!(exact(vec![("/a", "a"), ("/b", "b")]).len(), 2);
        assert_eq!(prefix(vec![("/a", "a"), ("/b", "b"), ("/c", "c")]).len(), 3);
        assert!(exact(vec![]).is_empty());
    }
}
