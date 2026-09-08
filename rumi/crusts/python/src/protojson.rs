//! Loading a config over FFI: canonical protojson, in.
//!
//! `from_config` used to accept the terse dialect via `MatcherIr<String>`'s
//! `Deserialize` impl. That dialect is retired — DECISIONS.md D-026 — so this
//! is the only door now: protojson in, `Matcher` out, using the same
//! `AnyResolver` and `convert_matcher` rumi and the CLI use.
//!
//! Byte-identical between `crusts/python` and `crusts/wasm`, deliberately, for
//! the same reason `fixtures.rs` is: the wheel and the npm package must not
//! disagree about what a config means.

use std::collections::HashMap;
use std::sync::OnceLock;

use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::convert_matcher;
use rumi_proto::index::{index_actions, ActionIndex};
use rumi_proto::protojson::parse_matcher;

/// Everything a crust config can name.
///
/// An `AnyResolver` decodes `Any` payloads and never touches a matching
/// context, so one resolver serves both `KvMatcher` and `HttpMatcher`. Built
/// on first use; every call after that returns the same instance.
pub fn resolver() -> &'static AnyResolver {
    static RESOLVER: OnceLock<AnyResolver> = OnceLock::new();
    RESOLVER.get_or_init(build_resolver)
}

fn build_resolver() -> AnyResolver {
    use rumi_proto::xuma;

    AnyResolverBuilder::new()
        .register::<xuma::kv::v1::MapInput>("xuma.kv.v1.MapInput")
        .register::<xuma::http::v1::HeaderInput>("xuma.http.v1.HeaderInput")
        .register::<xuma::http::v1::PathInput>("xuma.http.v1.PathInput")
        .register::<xuma::http::v1::MethodInput>("xuma.http.v1.MethodInput")
        .register::<xuma::http::v1::QueryParamInput>("xuma.http.v1.QueryParamInput")
        .register::<xuma::http::v1::AuthorityInput>("xuma.http.v1.AuthorityInput")
        .register::<xuma::http::v1::SchemeInput>("xuma.http.v1.SchemeInput")
        .register::<xuma::core::v1::BoolMatcher>("xuma.core.v1.BoolMatcher")
        .register::<xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
        .build()
}

/// One action as the binding holds it: the name the caller sees, and the
/// metadata a policy uses to name its processor (D-056, D-065).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionEntry {
    pub name: String,
    pub metadata: HashMap<String, String>,
}

impl ActionEntry {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metadata: HashMap::new(),
        }
    }
}

/// `NamedAction` -> the entry the binding holds.
///
/// An empty name is refused. Every other empty identifier makes a predicate
/// false — no decision. This one makes the rule *fire* and return `""`.
struct NamedActionFactory;

impl rumi::ActionFactory<ActionEntry> for NamedActionFactory {
    type Config = rumi_proto::xuma::core::v1::NamedAction;

    fn from_config(config: Self::Config) -> Result<ActionEntry, rumi::MatcherError> {
        if config.name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "action name",
            });
        }
        Ok(ActionEntry {
            name: config.name,
            metadata: config.metadata,
        })
    }
}

/// The action table every crust matcher resolves through. Built on first
/// use; every call after that returns the same instance.
pub fn actions() -> &'static rumi::ActionRegistry<ActionEntry> {
    static ACTIONS: OnceLock<rumi::ActionRegistry<ActionEntry>> = OnceLock::new();
    ACTIONS.get_or_init(|| {
        rumi::ActionRegistryBuilder::new()
            .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
            .build()
    })
}

/// Parse a JSON string as canonical protojson into the runtime `MatcherIr`
/// the registry consumes.
///
/// # Errors
///
/// If the JSON does not parse, or is not a valid `xds.type.matcher.v3.Matcher`.
pub fn load(json_config: &str) -> Result<rumi::MatcherIr<rumi::TypedExtensionConfig>, String> {
    rumi_proto::protojson::check_document_size(json_config).map_err(|e| e.to_string())?;
    let document: serde_json::Value =
        serde_json::from_str(json_config).map_err(|e| format!("invalid config JSON: {e}"))?;
    let resolver = resolver();
    let proto = parse_matcher(resolver, document).map_err(|e| e.to_string())?;
    convert_matcher(&proto, resolver).map_err(|e| e.to_string())
}

/// A loaded matcher and the actions it decides between, held by the binding.
///
/// The engine returns an [`ActionIndex`]; `actions[i]` is what it means.
/// Built once at load, so nothing about an action crosses the boundary per
/// evaluate (D-056).
pub struct Loaded<Ctx> {
    pub matcher: rumi::Matcher<Ctx, ActionIndex>,
    pub actions: Vec<ActionEntry>,
    pub fallback_start: Option<ActionIndex>,
}

/// Number a document's action sites, resolve each through the action table,
/// and load the matcher with indices as its actions.
///
/// This is the one door every binding matcher and the conformance runner
/// share, so the numbering a caller sees is the numbering the fixtures prove.
///
/// # Errors
///
/// An unknown action type URL, a factory refusal, or a matcher that fails
/// validation.
pub fn load_indexed<Ctx: 'static>(
    registry: &rumi::Registry<Ctx>,
    ir: rumi::MatcherIr<rumi::TypedExtensionConfig>,
) -> Result<Loaded<Ctx>, rumi::MatcherError> {
    let indexed = index_actions(ir);
    let actions = indexed
        .table
        .iter()
        .map(|entry| actions().resolve(entry))
        .collect::<Result<Vec<_>, _>>()?;
    let matcher = registry.load_matcher(indexed.ir)?;
    matcher.validate()?;
    Ok(Loaded {
        matcher,
        actions,
        fallback_start: indexed.fallback_start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_is_built_once() {
        let first = resolver();
        assert!((0..100).all(|_| std::ptr::eq(first, resolver())));
    }

    #[test]
    fn actions_are_built_once() {
        let first = actions();
        assert!((0..100).all(|_| std::ptr::eq(first, actions())));
    }
}
