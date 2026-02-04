//! `DataInput` implementations for `ProcessingRequest`.
//!
//! These extract HTTP data from `ext_proc` `ProcessingRequest` for matching.

use crate::context::{get_query_param, parse_path_only, parse_query_string, ProcessingRequestExt};
use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::ProcessingRequest;
use rumi::prelude::*;

/// Extracts the request path (without query string) from `ProcessingRequest`.
///
/// Maps to the `:path` pseudo-header, with query string stripped.
#[derive(Debug, Clone, Default)]
pub struct PathInput;

impl DataInput<ProcessingRequest> for PathInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_path().map_or(MatchingData::None, |p| {
            MatchingData::String(parse_path_only(p).to_string())
        })
    }
}

/// Extracts the HTTP method from `ProcessingRequest`.
///
/// Maps to the `:method` pseudo-header.
#[derive(Debug, Clone, Default)]
pub struct MethodInput;

impl DataInput<ProcessingRequest> for MethodInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_method()
            .map_or(MatchingData::None, |m| MatchingData::String(m.to_string()))
    }
}

/// Extracts a header value from `ProcessingRequest`.
///
/// Header names are matched case-insensitively.
#[derive(Debug, Clone)]
pub struct HeaderInput {
    name: String,
}

impl HeaderInput {
    /// Create a new header input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<ProcessingRequest> for HeaderInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_request_header(&self.name)
            .map_or(MatchingData::None, |v| MatchingData::String(v.to_string()))
    }
}

/// Extracts a query parameter value from `ProcessingRequest`.
///
/// Parses the query string from the `:path` pseudo-header.
#[derive(Debug, Clone)]
pub struct QueryParamInput {
    name: String,
}

impl QueryParamInput {
    /// Create a new query parameter input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<ProcessingRequest> for QueryParamInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_path()
            .and_then(parse_query_string)
            .and_then(|q| get_query_param(q, &self.name))
            .map_or(MatchingData::None, |v| MatchingData::String(v.to_string()))
    }
}

/// Extracts the :scheme pseudo-header from `ProcessingRequest`.
#[derive(Debug, Clone, Default)]
pub struct SchemeInput;

impl DataInput<ProcessingRequest> for SchemeInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_scheme()
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Extracts the :authority pseudo-header from `ProcessingRequest`.
#[derive(Debug, Clone, Default)]
pub struct AuthorityInput;

impl DataInput<ProcessingRequest> for AuthorityInput {
    fn get(&self, ctx: &ProcessingRequest) -> MatchingData {
        ctx.get_authority()
            .map_or(MatchingData::None, |a| MatchingData::String(a.to_string()))
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require constructing ProcessingRequest,
    // which needs the full protobuf structure. See integration tests.
}
