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

use std::sync::OnceLock;

use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::convert_matcher;
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

/// `NamedAction` -> the string the engine returns.
///
/// An empty name is refused. Every other empty identifier makes a predicate
/// false — no decision. This one makes the rule *fire* and return `""`.
struct NamedActionFactory;

impl rumi::ActionFactory<String> for NamedActionFactory {
    type Config = rumi_proto::xuma::core::v1::NamedAction;

    fn from_config(config: Self::Config) -> Result<String, rumi::MatcherError> {
        if config.name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "action name",
            });
        }
        Ok(config.name)
    }
}

/// The action table every crust matcher resolves through. Built on first
/// use; every call after that returns the same instance.
pub fn actions() -> &'static rumi::ActionRegistry<String> {
    static ACTIONS: OnceLock<rumi::ActionRegistry<String>> = OnceLock::new();
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
    let document: serde_json::Value =
        serde_json::from_str(json_config).map_err(|e| format!("invalid config JSON: {e}"))?;
    let resolver = resolver();
    let proto = parse_matcher(resolver, document).map_err(|e| e.to_string())?;
    convert_matcher(&proto, resolver).map_err(|e| e.to_string())
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
