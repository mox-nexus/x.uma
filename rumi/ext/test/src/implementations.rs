//! The conformance ledger's vocabulary, shared by every fixture dialect.
//!
//! Every fixture names the implementations expected to run it, and each
//! runner holds that in both directions: a listed implementation that fails
//! is a failure, and one that is not listed but succeeds is also a failure.
//! The key is required. A default would let a fixture claim coverage nobody
//! provides, which is how `spec/tests/05_http/` spent months with no runner
//! in the reference implementation (D-050).

use serde::Deserialize;

/// An implementation that may be expected to run a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Implementation {
    /// rumi, the engine.
    Rust,
    /// The Python binding, `xuma-crust` on the Python package index.
    Python,
    /// The TypeScript binding, `xuma-crust` on npm.
    Typescript,
}

/// Every implementation the suite covers.
pub const ALL: [Implementation; 3] = [
    Implementation::Rust,
    Implementation::Python,
    Implementation::Typescript,
];
