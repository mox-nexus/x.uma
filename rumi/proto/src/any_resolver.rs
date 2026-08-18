//! `AnyResolver` — bridges `google.protobuf.Any` to `TypedConfig`.
//!
//! In xDS, extension configs are carried as `Any` payloads: a `type_url` string
//! plus serialized proto bytes. Our registry expects `TypedConfig`: a `type_url`
//! string plus `serde_json::Value`. The `AnyResolver` decodes `Any` bytes into
//! the known proto message type, then serializes it to JSON via prost-serde.
//!
//! # Pattern (axum monomorphization → type erasure)
//!
//! At registration time, the concrete `Message` type is monomorphized into a
//! closure that captures `Message::decode` and `serde_json::to_value`. The
//! closure is erased behind `Box<dyn Fn>`. At resolve time, only the `type_url`
//! is needed to look up and invoke the correct decoder.

use prost::Message;
use rumi::MatcherError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

/// Type-erased decoder: proto bytes → `serde_json::Value`.
type BoxedDecoder = Box<dyn Fn(&[u8]) -> Result<serde_json::Value, MatcherError> + Send + Sync>;

/// Type-erased encoder: `serde_json::Value` → proto bytes.
///
/// The inverse of [`BoxedDecoder`], and registered by the same call, so the set
/// of readable types and the set of writable types cannot drift apart.
type BoxedEncoder = Box<dyn Fn(serde_json::Value) -> Result<Vec<u8>, MatcherError> + Send + Sync>;

/// Builder for constructing an [`AnyResolver`].
///
/// Register known proto message types, then call [`build()`](Self::build)
/// to produce an immutable resolver.
pub struct AnyResolverBuilder {
    codecs: HashMap<String, Codec>,
}

/// Both directions for one registered message type.
struct Codec {
    decode: BoxedDecoder,
    encode: BoxedEncoder,
}

impl AnyResolverBuilder {
    /// Create a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            codecs: HashMap::new(),
        }
    }

    /// Register a proto message type with its type URL.
    ///
    /// The type must implement `prost::Message` (binary) and both serde halves
    /// (JSON, via prost-serde). One call installs both directions: bytes → JSON
    /// for reading an `Any` off the wire, and JSON → bytes for packing a
    /// canonical protojson `@type` object. A type is therefore never readable
    /// but unwritable, or the reverse.
    ///
    /// The URL may be given with or without the `type.googleapis.com/` prefix;
    /// lookups normalize both.
    #[must_use]
    pub fn register<T>(mut self, type_url: &str) -> Self
    where
        T: Message + Serialize + DeserializeOwned + Default + 'static,
    {
        let url = strip_type_prefix(type_url).to_owned();
        let decode_url = url.clone();
        let encode_url = url.clone();
        self.codecs.insert(
            url,
            Codec {
                decode: Box::new(move |bytes: &[u8]| {
                    let msg = T::decode(bytes).map_err(|e| MatcherError::InvalidConfig {
                        source: format!("proto decode failed for {decode_url}: {e}"),
                    })?;
                    serde_json::to_value(&msg).map_err(|e| MatcherError::InvalidConfig {
                        source: format!("proto-to-json failed for {decode_url}: {e}"),
                    })
                }),
                encode: Box::new(move |value: serde_json::Value| {
                    let msg: T =
                        serde_json::from_value(value).map_err(|e| MatcherError::InvalidConfig {
                            source: format!("json-to-proto failed for {encode_url}: {e}"),
                        })?;
                    Ok(msg.encode_to_vec())
                }),
            },
        );
        self
    }

    /// Freeze the resolver.
    #[must_use]
    pub fn build(self) -> AnyResolver {
        AnyResolver {
            codecs: self.codecs,
        }
    }
}

impl Default for AnyResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable resolver for `google.protobuf.Any` payloads.
///
/// Converts proto binary `Any` into `TypedConfig` (type_url + JSON value)
/// for consumption by the rumi [`Registry`](rumi::Registry).
pub struct AnyResolver {
    codecs: HashMap<String, Codec>,
}

impl AnyResolver {
    /// Resolve a `TypedExtensionConfig` into a [`TypedConfig`](rumi::TypedConfig).
    ///
    /// Extracts the `type_url` from the `Any` payload (stripping the
    /// `type.googleapis.com/` prefix if present), decodes the bytes using
    /// the registered decoder, and returns a `TypedConfig`.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::UnknownTypeUrl`] if no decoder is registered
    /// - [`MatcherError::InvalidConfig`] if decoding or JSON conversion fails
    pub fn resolve(
        &self,
        config: &crate::xds::core::v3::TypedExtensionConfig,
    ) -> Result<rumi::TypedConfig, MatcherError> {
        let any = config
            .typed_config
            .as_ref()
            .ok_or_else(|| MatcherError::InvalidConfig {
                source: format!("TypedExtensionConfig '{}' has no typed_config", config.name),
            })?;

        // Strip type.googleapis.com/ prefix if present
        let type_url = strip_type_prefix(&any.type_url);

        let json_value = (self.codec(type_url)?.decode)(&any.value)?;
        Ok(rumi::TypedConfig {
            type_url: type_url.to_owned(),
            config: json_value,
        })
    }

    /// Encode a canonical protojson payload into proto binary.
    ///
    /// Used by [`pack`](Self::pack) to turn an `@type` object into the bytes an
    /// `Any` carries.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::UnknownTypeUrl`] if no type is registered
    /// - [`MatcherError::InvalidConfig`] if the JSON does not fit the message
    pub(crate) fn encode_json(
        &self,
        type_url: &str,
        value: serde_json::Value,
    ) -> Result<Vec<u8>, MatcherError> {
        (self.codec(strip_type_prefix(type_url))?.encode)(value)
    }

    fn codec(&self, type_url: &str) -> Result<&Codec, MatcherError> {
        self.codecs
            .get(type_url)
            .ok_or_else(|| MatcherError::UnknownTypeUrl {
                type_url: type_url.to_owned(),
                registry: "any_resolver",
                available: self.codecs.keys().cloned().collect(),
            })
    }

    /// Returns the number of registered types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.codecs.len()
    }

    /// Returns `true` if no types are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.codecs.is_empty()
    }
}

/// Strip the `type.googleapis.com/` prefix from a type URL.
///
/// xDS type URLs can be fully qualified (`type.googleapis.com/xuma.http.v1.HeaderInput`)
/// or short (`xuma.http.v1.HeaderInput`). We normalize to the short form.
pub(crate) fn strip_type_prefix(url: &str) -> &str {
    url.strip_prefix("type.googleapis.com/").unwrap_or(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn resolve_header_input() {
        let resolver = AnyResolverBuilder::new()
            .register::<crate::xuma::http::v1::HeaderInput>("xuma.http.v1.HeaderInput")
            .build();

        // Encode a HeaderInput proto to bytes
        let header_input = crate::xuma::http::v1::HeaderInput {
            name: "content-type".into(),
        };
        let bytes = header_input.encode_to_vec();

        // Wrap in TypedExtensionConfig
        let config = crate::xds::core::v3::TypedExtensionConfig {
            name: "test".into(),
            typed_config: Some(prost_types::Any {
                type_url: "type.googleapis.com/xuma.http.v1.HeaderInput".into(),
                value: bytes.into(),
            }),
        };

        let typed = resolver.resolve(&config).unwrap();
        assert_eq!(typed.type_url, "xuma.http.v1.HeaderInput");
        assert_eq!(typed.config["name"], "content-type");
    }

    #[test]
    fn resolve_short_type_url() {
        let resolver = AnyResolverBuilder::new()
            .register::<crate::xuma::test::v1::StringInput>("xuma.test.v1.StringInput")
            .build();

        let input = crate::xuma::test::v1::StringInput {
            value: "key".into(),
        };

        let config = crate::xds::core::v3::TypedExtensionConfig {
            name: "test".into(),
            typed_config: Some(prost_types::Any {
                type_url: "xuma.test.v1.StringInput".into(),
                value: input.encode_to_vec().into(),
            }),
        };

        let typed = resolver.resolve(&config).unwrap();
        assert_eq!(typed.type_url, "xuma.test.v1.StringInput");
        assert_eq!(typed.config["value"], "key");
    }

    #[test]
    fn resolve_empty_message() {
        let resolver = AnyResolverBuilder::new()
            .register::<crate::xuma::http::v1::PathInput>("xuma.http.v1.PathInput")
            .build();

        let config = crate::xds::core::v3::TypedExtensionConfig {
            name: "path".into(),
            typed_config: Some(prost_types::Any {
                type_url: "xuma.http.v1.PathInput".into(),
                value: vec![].into(), // empty message = zero bytes
            }),
        };

        let typed = resolver.resolve(&config).unwrap();
        assert_eq!(typed.type_url, "xuma.http.v1.PathInput");
    }

    #[test]
    fn resolve_unknown_type_url_errors() {
        let resolver = AnyResolverBuilder::new().build();

        let config = crate::xds::core::v3::TypedExtensionConfig {
            name: "test".into(),
            typed_config: Some(prost_types::Any {
                type_url: "unknown.Type".into(),
                value: vec![].into(),
            }),
        };

        let err = resolver.resolve(&config).unwrap_err();
        assert!(matches!(err, MatcherError::UnknownTypeUrl { .. }));
    }

    #[test]
    fn resolve_missing_typed_config_errors() {
        let resolver = AnyResolverBuilder::new().build();

        let config = crate::xds::core::v3::TypedExtensionConfig {
            name: "test".into(),
            typed_config: None,
        };

        let err = resolver.resolve(&config).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }));
    }

    #[test]
    fn strip_type_prefix_works() {
        assert_eq!(
            strip_type_prefix("type.googleapis.com/xuma.http.v1.HeaderInput"),
            "xuma.http.v1.HeaderInput"
        );
        assert_eq!(
            strip_type_prefix("xuma.http.v1.HeaderInput"),
            "xuma.http.v1.HeaderInput"
        );
    }
}
