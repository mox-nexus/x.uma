//! Conformance fixture loading. Test apparatus, not a shipped domain.
//!
//! This crate used to contain the key-value matching domain as well. That made
//! it unpublishable — a YAML fixture loader has no business in a released
//! artifact — and since `rumi-cli` depended on it for the *domain*, the CLI
//! could not be published either. The domain is now `rumi-kv`; what remains
//! here is only the machinery that reads `spec/tests/*.yaml`.
//!
//! It also used to carry three more fixture dialects — a terse `config:` shape
//! and a native `matcher:` shape, each with its own loader and its own
//! `MatcherConfig` type. All three are gone; `spec/tests/07_protojson/` is the
//! format, and `proto_fixture` is the only reader.
//!
//! Stays `publish = false`, deliberately. Nothing user-facing depends on it.

#[cfg(all(feature = "fixtures", feature = "registry"))]
pub mod proto_fixture;

// The conformance suite refers to the domain by these names, so they are
// re-exported rather than duplicated. `rumi-kv` is the single definition.
pub use rumi_kv::{KvContext, StringInput};

#[cfg(feature = "registry")]
pub use rumi_kv::register;

pub mod prelude {
    pub use rumi_kv::prelude::*;
}
