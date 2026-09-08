//! Conformance fixture loading. Test apparatus, not a shipped domain.
//!
//! Only the machinery that reads `spec/tests/*.yaml`: the key-value domain is
//! `rumi-kv`, and `07_protojson/` is the one fixture format (D-042).
//!
//! Stays `publish = false`, deliberately. Nothing user-facing depends on it.

#[cfg(feature = "fixtures")]
pub mod implementations;

#[cfg(all(feature = "fixtures", feature = "registry"))]
pub mod proto_fixture;

#[cfg(all(feature = "fixtures", feature = "http"))]
pub mod http_fixture;

// The conformance suite refers to the domain by these names, so they are
// re-exported rather than duplicated. `rumi-kv` is the single definition.
pub use rumi_kv::{KvContext, StringInput};

#[cfg(feature = "registry")]
pub use rumi_kv::register;

pub mod prelude {
    pub use rumi_kv::prelude::*;
}
