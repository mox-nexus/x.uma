//! bumi-crusty — TypeScript bindings for rumi via `wasm-bindgen`.
//!
//! Exposes rumi's matcher engine to TypeScript as opaque compiled matchers.
//! Config in → compile in Rust → evaluate in Rust → simple types out.

mod config;
mod convert;
mod matcher;
