//! Canonical protobuf JSON (protojson) for `google.protobuf.Any`.
//!
//! The generated serde impls handle every part of protojson except one: `Any`.
//! Canonical protojson writes an `Any` **expanded** — the payload's own fields
//! inlined beside an `@type` key:
//!
//! ```json
//! { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "role" }
//! ```
//!
//! `pbjson-types` writes it **packed** instead — a type URL beside base64 of the
//! binary encoding:
//!
//! ```json
//! { "typeUrl": "type.googleapis.com/xuma.kv.v1.MapInput", "value": "CgRyb2xl" }
//! ```
//!
//! The difference is not stylistic. Expanding an `Any` requires knowing the
//! payload's schema at (de)serialization time, and static codegen has no
//! descriptor pool to look it up in. So `pbjson-types` does the only thing it
//! can, and a document written the canonical way fails to load with
//! ``unknown field `@type` ``.
//!
//! This module supplies the missing half. It rewrites between the two forms
//! using the same [`AnyResolver`] registry that already decodes `Any` payloads,
//! so the set of types that can be *written* and the set that can be *read* are
//! one set by construction rather than two that must be kept in step.
//!
//! # Why route through binary at all
//!
//! Packing means the file path and the control-plane path meet at the same proto
//! `Matcher` value and share one conversion walk ([`crate::convert`]). The
//! alternative — a second walk that reads JSON directly into config types —
//! costs a JSON→bytes→JSON round trip less, and buys two implementations of the
//! same semantics that are free to disagree. This repo has already shipped that
//! bug once. The round trip happens once per config load, never during
//! evaluation.

use crate::any_resolver::AnyResolver;
use rumi::MatcherError;
use serde_json::{Map, Value};

/// The key canonical protojson uses to name an `Any` payload's type.
const TYPE_KEY: &str = "@type";

/// How deep the packing walk will descend before giving up.
///
/// The walk runs over attacker-supplied JSON *before* any `Matcher` exists, so
/// `MAX_DEPTH` — which is checked on a constructed matcher — cannot protect it.
/// The limit belongs to the walker because the walker is what holds the stack.
///
/// 128 is `serde_json`'s own recursion limit. Measured 2026-08-18, `serde_yaml`
/// carries the same one — it accepts 128 levels of nesting and rejects 129 — so
/// neither front end can hand this walk a document deep enough to reach the
/// bound. It is a backstop for a `Value` a caller built some other way, and
/// deliberately not the only guard.
const MAX_JSON_DEPTH: usize = 128;

impl AnyResolver {
    /// Rewrite canonical protojson into the form the generated serde impls read.
    ///
    /// Descends the document top-down. An object whose `@type` key holds a
    /// string is an `Any`: its remaining fields are encoded to binary and it is
    /// replaced by `{"typeUrl": …, "value": <base64>}`. Everything else is
    /// structural — matchers, predicates, string matchers — and passes through.
    ///
    /// # The walk stops at `@type`, and that is the point
    ///
    /// Payload bodies are handed to the payload's own deserializer without
    /// being scanned. That matters because the schema's user-controlled string
    /// maps all live *inside* payloads — `xuma.core.v1.NamedAction.metadata`,
    /// `xuma.claude.v1.HookContext.tool_args` — and a rule that rewrote any
    /// object containing `@type` would corrupt a rule whose metadata happened to
    /// use that key. Descending only until the first `@type` puts them out of
    /// reach without needing a descriptor pool to say so.
    ///
    /// The mirrored hazard is structural maps with user-controlled *keys*. The
    /// matcher schema has exactly one, `MatcherTree.MatchMap.map`, and its
    /// values are messages — so a match key spelled `@type` carries an object,
    /// never a string, and the string test above excludes it. Both cases are
    /// pinned by tests; a new `map<string, string>` in a structural position
    /// would break the second guarantee, and that test is where it would show.
    ///
    /// # Nested `Any` is not supported
    ///
    /// No `xuma.*` message has an `Any` field, so the case does not arise. If
    /// one gains it, a canonically-written payload fails to load with
    /// ``unknown field `@type` `` rather than loading wrongly.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::UnknownTypeUrl`] if no type is registered under the URL
    /// - [`MatcherError::InvalidConfig`] if the payload does not fit the
    ///   registered message, or the document nests past [`MAX_JSON_DEPTH`]
    pub fn pack(&self, value: Value) -> Result<Value, MatcherError> {
        self.pack_at(value, 0)
    }

    fn pack_at(&self, value: Value, depth: usize) -> Result<Value, MatcherError> {
        if depth > MAX_JSON_DEPTH {
            return Err(MatcherError::InvalidConfig {
                source: format!("config nests deeper than {MAX_JSON_DEPTH} levels"),
            });
        }

        match value {
            Value::Object(map) => match any_type_url(&map) {
                Some(_) => self.pack_payload(map),
                None => map
                    .into_iter()
                    .map(|(k, v)| self.pack_at(v, depth + 1).map(|v| (k, v)))
                    .collect::<Result<Map<_, _>, _>>()
                    .map(Value::Object),
            },
            Value::Array(items) => items
                .into_iter()
                .map(|v| self.pack_at(v, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            scalar => Ok(scalar),
        }
    }

    /// Encode one `@type` object into a packed `Any`. Its body is not walked.
    fn pack_payload(&self, mut map: Map<String, Value>) -> Result<Value, MatcherError> {
        let url = any_type_url(&map)
            .expect("caller checked the key is present and holds a string")
            .to_owned();
        map.remove(TYPE_KEY);

        let bytes = self.encode_json(&url, Value::Object(map))?;

        let mut packed = Map::with_capacity(2);
        packed.insert("typeUrl".to_owned(), Value::String(url));
        packed.insert("value".to_owned(), Value::String(base64_encode(&bytes)));
        Ok(Value::Object(packed))
    }
}

/// The type URL, if this object is an `Any` written the canonical way.
///
/// Requires the value to be a string. See [`AnyResolver::pack`] for why that
/// test is load-bearing rather than defensive.
fn any_type_url(map: &Map<String, Value>) -> Option<&str> {
    match map.get(TYPE_KEY) {
        Some(Value::String(url)) => Some(url),
        _ => None,
    }
}

/// Base64, standard alphabet with padding — what `pbjson` reads for `bytes`.
///
/// Hand-written rather than pulled from a crate: twenty lines against a
/// dependency in the tree of every published artifact. The round-trip test
/// below decodes through `pbjson` itself, so the two cannot drift.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let idx = [n >> 18 & 63, n >> 12 & 63, n >> 6 & 63, n & 63];

        out.push(ALPHABET[idx[0] as usize] as char);
        out.push(ALPHABET[idx[1] as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[idx[2] as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[idx[3] as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Parse canonical protojson into a proto `Matcher`.
///
/// This is the only door. Everything that loads a config file — the CLI, the
/// crusts, the conformance runner — comes through here, so there is one place
/// where a document becomes a matcher and one place a bug in that step can be.
///
/// Takes an already-parsed [`Value`] rather than a string so the YAML front end
/// stays the caller's business: `serde_yaml` is archived upstream, and a library
/// in the dependency tree of every published artifact should not carry it.
/// Both syntaxes converging on `Value` here is also what makes "the same rule in
/// YAML and in JSON builds the same matcher" true by construction rather than by
/// testing.
///
/// # Errors
///
/// Returns [`MatcherError::InvalidConfig`] if the document is not a valid
/// `xds.type.matcher.v3.Matcher`, or [`MatcherError::UnknownTypeUrl`] if it
/// names an extension nobody registered. Unknown fields are rejected, not
/// ignored — a typo in a deny rule must not quietly become a catch-all.
pub fn parse_matcher(
    resolver: &AnyResolver,
    document: Value,
) -> Result<crate::xds::r#type::matcher::v3::Matcher, MatcherError> {
    let packed = resolver.pack(document)?;
    serde_json::from_value(packed).map_err(|e| MatcherError::InvalidConfig {
        source: format!("not a valid xds.type.matcher.v3.Matcher: {e}"),
    })
}

/// Parse a canonical protojson string into a proto `Matcher`.
///
/// # Errors
///
/// As [`parse_matcher`], plus a JSON syntax error.
pub fn parse_matcher_str(
    resolver: &AnyResolver,
    document: &str,
) -> Result<crate::xds::r#type::matcher::v3::Matcher, MatcherError> {
    let value: Value = serde_json::from_str(document).map_err(|e| MatcherError::InvalidConfig {
        source: format!("config is not valid JSON: {e}"),
    })?;
    parse_matcher(resolver, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_resolver::AnyResolverBuilder;
    use crate::xds::r#type::matcher::v3 as proto;

    fn resolver() -> AnyResolver {
        AnyResolverBuilder::new()
            .register::<crate::xuma::test::v1::MapInput>("xuma.test.v1.MapInput")
            .register::<crate::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
            .build()
    }

    /// The base64 this module writes is the base64 `pbjson` reads.
    ///
    /// Hand-rolled encoders drift from their decoders; this pins them together
    /// by decoding through the real thing rather than a second implementation.
    #[test]
    fn base64_matches_what_pbjson_decodes() {
        for len in 0..=32usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37)).collect();
            let json = serde_json::json!({
                "typeUrl": "x/y",
                "value": base64_encode(&bytes),
            });
            let any: prost_types::Any = serde_json::from_value(json)
                .unwrap_or_else(|e| panic!("len {len} failed to decode: {e}"));
            assert_eq!(any.value.as_ref(), bytes.as_slice(), "len {len}");
        }
    }

    /// A canonical protojson document loads. Before `pack` existed it failed
    /// with ``unknown field `@type` ``, which is the whole reason this exists.
    #[test]
    fn canonical_protojson_matcher_loads() {
        let doc: Value = serde_json::from_str(
            r#"{
              "matcherList": {
                "matchers": [{
                  "predicate": {
                    "singlePredicate": {
                      "input": {
                        "name": "role",
                        "typedConfig": {
                          "@type": "type.googleapis.com/xuma.test.v1.MapInput",
                          "key": "role"
                        }
                      },
                      "valueMatch": { "exact": "admin" }
                    }
                  },
                  "onMatch": {
                    "action": {
                      "name": "admin",
                      "typedConfig": {
                        "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                        "name": "admin"
                      }
                    }
                  }
                }]
              }
            }"#,
        )
        .unwrap();

        let packed = resolver().pack(doc).unwrap();
        let matcher: proto::Matcher = serde_json::from_value(packed).unwrap();

        let proto::matcher::MatcherType::MatcherList(list) = matcher.matcher_type.unwrap() else {
            panic!("expected a matcher list");
        };
        assert_eq!(list.matchers.len(), 1);
    }

    /// Round trip: the bytes `pack` produces decode back to the same JSON the
    /// author wrote. This is what makes the file path and the control-plane path
    /// meet at the same value.
    #[test]
    fn packed_payload_resolves_back_to_the_authored_fields() {
        let r = resolver();
        let doc = serde_json::json!({
            "name": "role",
            "typedConfig": {
                "@type": "type.googleapis.com/xuma.test.v1.MapInput",
                "key": "role"
            }
        });

        let tec: crate::xds::core::v3::TypedExtensionConfig =
            serde_json::from_value(r.pack(doc).unwrap()).unwrap();
        let typed = r.resolve(&tec).unwrap();

        assert_eq!(typed.type_url, "xuma.test.v1.MapInput");
        assert_eq!(typed.config["key"], "role");
    }

    /// An `@type` naming a type nobody registered is a load error, not a
    /// silently-ignored field. A config that names the wrong input must not
    /// quietly become a matcher that never fires.
    #[test]
    fn unregistered_type_url_is_an_error() {
        let doc = serde_json::json!({ "@type": "type.googleapis.com/nope.v1.Nope" });
        let err = resolver().pack(doc).unwrap_err();
        assert!(
            matches!(err, MatcherError::UnknownTypeUrl { .. }),
            "{err:?}"
        );
    }

    /// A field the payload's schema does not define is a load error too — the
    /// same protection `deny_unknown_fields` gave the hand-written config types.
    #[test]
    fn unknown_field_inside_a_payload_is_an_error() {
        let doc = serde_json::json!({
            "@type": "type.googleapis.com/xuma.test.v1.MapInput",
            "kye": "role"
        });
        let err = resolver().pack(doc).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }), "{err:?}");
    }

    /// Objects with no `@type` are structural and must survive untouched,
    /// including through arrays and nesting.
    #[test]
    fn structural_objects_pass_through() {
        let doc = serde_json::json!({
            "a": [{ "b": 1 }, { "c": [true, null, "s"] }],
            "d": { "e": {} }
        });
        assert_eq!(resolver().pack(doc.clone()).unwrap(), doc);
    }

    /// `MatcherTree.MatchMap.map` has user-controlled keys. A rule that matches
    /// the literal string `@type` must load, and must not be mistaken for an
    /// `Any`. It is not, because that map's values are messages and this walk
    /// only treats a *string* `@type` as a type URL.
    #[test]
    fn a_match_key_named_at_type_is_not_an_any() {
        let doc = serde_json::json!({
            "matcherTree": {
                "input": {
                    "name": "role",
                    "typedConfig": {
                        "@type": "type.googleapis.com/xuma.test.v1.MapInput",
                        "key": "role"
                    }
                },
                "exactMatchMap": {
                    "map": {
                        "@type": {
                            "action": {
                                "name": "weird",
                                "typedConfig": {
                                    "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                                    "name": "weird"
                                }
                            }
                        }
                    }
                }
            }
        });
        let packed = resolver().pack(doc).unwrap();
        let matcher: proto::Matcher = serde_json::from_value(packed).unwrap();
        let proto::matcher::MatcherType::MatcherTree(tree) = matcher.matcher_type.unwrap() else {
            panic!("expected a matcher tree");
        };
        let proto::matcher::matcher_tree::TreeType::ExactMatchMap(m) = tree.tree_type.unwrap()
        else {
            panic!("expected an exact match map");
        };
        assert!(m.map.contains_key("@type"));
    }

    /// The mirror case: `NamedAction.metadata` is `map<string, string>`, so a
    /// user may key it `@type`. The walk must not descend into a payload body
    /// and try to pack it.
    #[test]
    fn at_type_inside_a_payload_body_is_left_alone() {
        let r = resolver();
        let doc = serde_json::json!({
            "name": "a",
            "typedConfig": {
                "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                "name": "a",
                "metadata": { "@type": "not-a-type-url" }
            }
        });
        let tec: crate::xds::core::v3::TypedExtensionConfig =
            serde_json::from_value(r.pack(doc).unwrap()).unwrap();
        let typed = r.resolve(&tec).unwrap();
        assert_eq!(typed.config["metadata"]["@type"], "not-a-type-url");
    }

    /// Unbounded recursion over attacker-supplied JSON, before any `Matcher`
    /// exists for `MAX_DEPTH` to protect.
    #[test]
    fn absurdly_nested_json_is_rejected_not_overflowed() {
        let mut doc = Value::Null;
        for _ in 0..(MAX_JSON_DEPTH + 10) {
            doc = Value::Array(vec![doc]);
        }
        let err = resolver().pack(doc).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }), "{err:?}");
    }
}
