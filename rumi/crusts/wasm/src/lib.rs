//! xuma-crust — TypeScript bindings for rumi via `wasm-bindgen`.
//!
//! Exposes rumi's matcher engine to TypeScript as opaque compiled matchers.
//! Config in → compile in Rust → evaluate in Rust → simple types out.
//!
//! # Matcher Types
//!
//! - [`HookMatcher`] — Claude Code hook matching (plain JS objects)
//! - [`HttpMatcher`] — HTTP request matching (JSON config via registry)
//! - [`KvMatcher`] — key-value matching over `xuma.kv.v1` inputs; the conformance domain

mod config;
mod convert;
#[cfg(feature = "fixtures")]
mod fixtures;
mod http_matcher;
mod kv_matcher;
pub(crate) mod matcher;
mod protojson;
