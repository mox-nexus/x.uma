//! Action sites numbered in document order, so a binding returns an index
//! and holds the actions itself (D-056).
//!
//! The walk is the same for every consumer of this crate, which is what
//! makes the PyO3 and wasm bindings number a document identically: a
//! pre-order walk of the IR, list entries in written order, tree entries by
//! sorted key, and each matcher's fallback after its entries. Two loads of
//! one document produce one table.

use rumi::{
    FieldMatcherIr, MatcherIr, MatcherKindIr, MatcherTreeIr, OnMatchIr, TreeTypeIr,
    TypedExtensionConfig,
};

/// The position of an action site in the document's action table.
///
/// The engine's `A` for a binding: what `evaluate` returns across the FFI,
/// with the action itself looked up host-side (D-056).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionIndex(pub u32);

impl std::fmt::Display for ActionIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A document with its action sites numbered.
#[derive(Debug, Clone)]
pub struct Indexed {
    /// The IR with every action replaced by its index.
    pub ir: MatcherIr<ActionIndex>,
    /// The actions, in index order: `table[i.0]` is what `ActionIndex(i)`
    /// stands for. One entry per site, so the mapping is a bijection over the
    /// document and identical for every load of it.
    pub table: Vec<TypedExtensionConfig>,
    /// The first index under the root matcher's `on_no_match`, if it has one.
    /// The root fallback is walked last, so every index from here on is a
    /// fallback decision (D-056's `fallback` flag).
    pub fallback_start: Option<ActionIndex>,
}

/// Replace every action in the IR with its index, returning the actions in
/// index order.
#[must_use]
pub fn index_actions(ir: MatcherIr<TypedExtensionConfig>) -> Indexed {
    let mut table = Vec::new();
    let kind = walk_kind(ir.kind, &mut table);
    let fallback_start = ir
        .on_no_match
        .as_ref()
        .map(|_| ActionIndex(u32::try_from(table.len()).expect("fewer than 2^32 action sites")));
    let on_no_match = ir.on_no_match.map(|om| walk_on_match(om, &mut table));
    Indexed {
        ir: MatcherIr { kind, on_no_match },
        table,
        fallback_start,
    }
}

fn walk_matcher(
    ir: MatcherIr<TypedExtensionConfig>,
    table: &mut Vec<TypedExtensionConfig>,
) -> MatcherIr<ActionIndex> {
    let kind = walk_kind(ir.kind, table);
    let on_no_match = ir.on_no_match.map(|om| walk_on_match(om, table));
    MatcherIr { kind, on_no_match }
}

fn walk_kind(
    kind: MatcherKindIr<TypedExtensionConfig>,
    table: &mut Vec<TypedExtensionConfig>,
) -> MatcherKindIr<ActionIndex> {
    match kind {
        MatcherKindIr::List(matchers) => MatcherKindIr::List(
            matchers
                .into_iter()
                .map(|fm| FieldMatcherIr {
                    predicate: fm.predicate,
                    on_match: walk_on_match(fm.on_match, table),
                })
                .collect(),
        ),
        MatcherKindIr::Tree(tree) => MatcherKindIr::Tree(MatcherTreeIr {
            input: tree.input,
            tree: match tree.tree {
                TreeTypeIr::ExactMatchMap(entries) => {
                    TreeTypeIr::ExactMatchMap(walk_entries(entries, table))
                }
                TreeTypeIr::PrefixMatchMap(entries) => {
                    TreeTypeIr::PrefixMatchMap(walk_entries(entries, table))
                }
            },
        }),
    }
}

fn walk_entries(
    mut entries: Vec<(String, OnMatchIr<TypedExtensionConfig>)>,
    table: &mut Vec<TypedExtensionConfig>,
) -> Vec<(String, OnMatchIr<ActionIndex>)> {
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
        .into_iter()
        .map(|(key, om)| (key, walk_on_match(om, table)))
        .collect()
}

fn walk_on_match(
    om: OnMatchIr<TypedExtensionConfig>,
    table: &mut Vec<TypedExtensionConfig>,
) -> OnMatchIr<ActionIndex> {
    match om {
        OnMatchIr::Action { action } => {
            let index = u32::try_from(table.len()).expect("fewer than 2^32 action sites");
            table.push(action);
            OnMatchIr::Action {
                action: ActionIndex(index),
            }
        }
        OnMatchIr::Matcher { matcher } => OnMatchIr::Matcher {
            matcher: Box::new(walk_matcher(*matcher, table)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumi::{PredicateIr, SinglePredicateIr, StringMatchSpec, ValueMatchIr};

    fn named(name: &str) -> TypedExtensionConfig {
        TypedExtensionConfig {
            type_url: "xuma.core.v1.NamedAction".into(),
            config: serde_json::json!({ "name": name }),
        }
    }

    fn action(name: &str) -> OnMatchIr<TypedExtensionConfig> {
        OnMatchIr::Action {
            action: named(name),
        }
    }

    fn rule(
        name: &str,
        on_match: OnMatchIr<TypedExtensionConfig>,
    ) -> FieldMatcherIr<TypedExtensionConfig> {
        FieldMatcherIr {
            predicate: PredicateIr::Single(SinglePredicateIr {
                input: TypedExtensionConfig {
                    type_url: "xuma.core.v1.MapInput".into(),
                    config: serde_json::json!({ "key": name }),
                },
                matcher: ValueMatchIr::BuiltIn {
                    spec: StringMatchSpec::Exact(name.into()),
                    ignore_case: false,
                },
            }),
            on_match,
        }
    }

    fn names(table: &[TypedExtensionConfig]) -> Vec<String> {
        table
            .iter()
            .map(|t| t.config["name"].as_str().unwrap().to_owned())
            .collect()
    }

    fn actions_of(ir: &MatcherIr<ActionIndex>) -> Vec<u32> {
        fn on_match(om: &OnMatchIr<ActionIndex>, out: &mut Vec<u32>) {
            match om {
                OnMatchIr::Action { action } => out.push(action.0),
                OnMatchIr::Matcher { matcher } => matcher_(matcher, out),
            }
        }
        fn matcher_(ir: &MatcherIr<ActionIndex>, out: &mut Vec<u32>) {
            match &ir.kind {
                MatcherKindIr::List(ms) => ms.iter().for_each(|fm| on_match(&fm.on_match, out)),
                MatcherKindIr::Tree(t) => match &t.tree {
                    TreeTypeIr::ExactMatchMap(e) | TreeTypeIr::PrefixMatchMap(e) => {
                        e.iter().for_each(|(_, om)| on_match(om, out));
                    }
                },
            }
            if let Some(om) = &ir.on_no_match {
                on_match(om, out);
            }
        }
        let mut out = Vec::new();
        matcher_(ir, &mut out);
        out
    }

    #[test]
    fn a_list_is_numbered_in_written_order_with_the_fallback_last() {
        let ir = MatcherIr::list(vec![
            rule("a", action("first")),
            rule("b", action("second")),
        ])
        .with_fallback(Some(action("fallback")));

        let Indexed {
            ir,
            table,
            fallback_start,
        } = index_actions(ir);

        assert_eq!(names(&table), ["first", "second", "fallback"]);
        assert_eq!(actions_of(&ir), [0, 1, 2]);
        assert_eq!(fallback_start, Some(ActionIndex(2)));
    }

    #[test]
    fn a_nested_matcher_is_numbered_where_it_appears() {
        let nested = MatcherIr::list(vec![rule("x", action("inner"))])
            .with_fallback(Some(action("inner_fallback")));
        let ir = MatcherIr::list(vec![
            rule("a", action("before")),
            rule(
                "b",
                OnMatchIr::Matcher {
                    matcher: Box::new(nested),
                },
            ),
            rule("c", action("after")),
        ]);

        let indexed = index_actions(ir);

        assert_eq!(
            names(&indexed.table),
            ["before", "inner", "inner_fallback", "after"]
        );
        assert_eq!(
            indexed.fallback_start, None,
            "a nested fallback is not the root's"
        );
    }

    #[test]
    fn tree_entries_are_numbered_by_sorted_key_whatever_order_they_arrived_in() {
        let tree = |entries: Vec<(&str, &str)>| MatcherIr {
            kind: MatcherKindIr::Tree(MatcherTreeIr {
                input: TypedExtensionConfig {
                    type_url: "xuma.core.v1.MapInput".into(),
                    config: serde_json::json!({ "key": "role" }),
                },
                tree: TreeTypeIr::ExactMatchMap(
                    entries
                        .into_iter()
                        .map(|(k, a)| (k.to_owned(), action(a)))
                        .collect(),
                ),
            }),
            on_no_match: Some(action("deny")),
        };

        let one = index_actions(tree(vec![("viewer", "read"), ("admin", "allow")])).table;
        let two = index_actions(tree(vec![("admin", "allow"), ("viewer", "read")])).table;

        assert_eq!(names(&one), ["allow", "read", "deny"]);
        assert_eq!(names(&one), names(&two));
    }

    #[test]
    fn every_index_names_exactly_one_site() {
        let ir = MatcherIr::list(vec![rule("a", action("same")), rule("b", action("same"))]);

        let indexed = index_actions(ir);

        assert_eq!(indexed.table.len(), 2, "two sites, even with one name");
        assert_eq!(actions_of(&indexed.ir), [0, 1]);
    }
}
