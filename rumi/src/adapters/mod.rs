//! Domain adapters
//!
//! Each adapter provides context types and `DataInput` implementations
//! for a specific domain. Adapters are feature-gated.

#[cfg(feature = "test-domain")]
pub mod test;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "claude")]
pub mod claude;
