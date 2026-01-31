//! rumi-domains: Domain adapters for rumi
//!
//! Each domain provides context types and `DataInput` implementations.
//! Domains are feature-gated.
//!
//! # Available Domains
//!
//! - `test` (default): xuma.test.v1 - Testing domain
//! - `http`: xuma.http.v1 - HTTP request matching
//! - `claude`: xuma.claude.v1 - Claude Code hooks matching

#[cfg(feature = "test")]
pub mod test;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "claude")]
pub mod claude;
