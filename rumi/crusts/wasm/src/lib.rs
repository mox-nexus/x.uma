//! xuma-crust — TypeScript bindings for rumi via `wasm-bindgen`.
//!
//! Exposes rumi's matcher engine to TypeScript as opaque compiled matchers.
//! Config in → compile in Rust → evaluate in Rust → simple types out.
//!
//! # Matcher Types
//!
//! - [`HookMatcher`] — Claude Code hook matching (plain JS objects)
//! - [`HttpMatcher`] — HTTP request matching (JSON config via registry)
//! - [`TestMatcher`] — Key-value matching for conformance testing (JSON config via registry)

mod config;
mod convert;
mod http_matcher;
pub(crate) mod matcher;
mod test_matcher;
