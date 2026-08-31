//! The migration ledger's vocabulary, shared by every fixture dialect.
//!
//! This lived inside `proto_fixture` until 2026-08-31, which is why the
//! `implementations:` key existed only for `spec/tests/07_protojson/`. The
//! other suite, `05_http/`, had no ledger at all — and no rumi runner — so the
//! claim that every implementation passes every fixture was true of one
//! directory and unexamined for the other. A ledger that covers half a suite is
//! the shape that let `http_empty_routes_matches_all` require a fail-open of
//! everyone (D-050).

use serde::Deserialize;

/// An implementation that may be expected to run a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Implementation {
    /// rumi, the Rust reference implementation.
    Rust,
    /// puma, the pure Python implementation.
    Python,
    /// bumi, the pure TypeScript implementation.
    Typescript,
}

/// Every implementation the suite covers. The migration's finish line.
pub const ALL: [Implementation; 3] = [
    Implementation::Rust,
    Implementation::Python,
    Implementation::Typescript,
];

/// Every implementation. The default for a fixture with no `implementations` key.
#[must_use]
pub fn all_implementations() -> Vec<Implementation> {
    ALL.to_vec()
}
