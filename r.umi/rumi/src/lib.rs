//! rumi - Rust implementation of xDS Unified Matcher API
//!
//! This is the facade crate that re-exports the public API from all r.umi crates.
//!
//! # Quick Start
//!
//! ```ignore
//! use rumi::prelude::*;
//!
//! // Create a matcher
//! let matcher = Matcher::new(/* ... */);
//!
//! // Evaluate against a context
//! let result = matcher.evaluate(&context);
//! ```

// Re-export core types
pub use rumi_core as core;

// Re-export domains
pub use rumi_domains as domains;

// Re-export proto (registry, conversions)
pub use rumi_proto as proto;

/// Prelude for convenient imports
pub mod prelude {
    // Core types will be re-exported here after Phase 1
}
