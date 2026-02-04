//! rumi-http: HTTP domain for request matching
//!
//! This crate provides two layers:
//!
//! 1. **User API**: Gateway API `HttpRouteMatch` for configuration
//! 2. **Data Plane API**: `ext_proc` `ProcessingRequest` for runtime
//!
//! # Architecture
//!
//! ```text
//! Gateway API HttpRouteMatch (config)
//!         ↓ compile()
//! rumi Matcher<ProcessingRequest, A>
//!         ↓ evaluate()
//! ext_proc ProcessingRequest (runtime)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use rumi_http::prelude::*;
//!
//! // Config time: compile Gateway API match to rumi matcher
//! let route_match = HttpRouteMatch {
//!     path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
//!     ..Default::default()
//! };
//! let matcher = route_match.compile::<&str>("api_backend");
//!
//! // Runtime: evaluate against ext_proc ProcessingRequest
//! let result = matcher.evaluate(&processing_request);
//! ```

mod compiler;
mod context;
mod inputs;
mod simple;

pub use compiler::*;
pub use context::*;
pub use inputs::*;
pub use simple::{
    HttpRequest, HttpRequestBuilder, SimpleHeaderInput, SimpleMethodInput, SimplePathInput,
    SimpleQueryParamInput,
};

// Re-export ext_proc types for convenience
pub use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    HttpBody, HttpHeaders, HttpTrailers, ProcessingRequest, ProcessingResponse,
};

// Re-export Gateway API types for convenience
pub use k8s_gateway_api::{
    HttpHeaderMatch, HttpMethod, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        // DataInputs for ProcessingRequest
        AuthorityInput,
        HeaderInput,
        // Re-exports: Gateway API
        HttpHeaderMatch,
        HttpMethod,
        HttpPathMatch,
        HttpQueryParamMatch,
        // Simple context + inputs (for testing)
        HttpRequest,
        HttpRequestBuilder,
        HttpRouteMatch,
        // Compiler
        HttpRouteMatchExt,
        MethodInput,
        PathInput,
        // Re-exports: ext_proc
        ProcessingRequest,
        ProcessingResponse,
        QueryParamInput,
        SchemeInput,
        SimpleHeaderInput,
        SimpleMethodInput,
        SimplePathInput,
        SimpleQueryParamInput,
    };
    pub use rumi::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // Basic smoke test that dependencies are wired correctly
        let _: Option<ProcessingRequest> = None;
        let _: Option<HttpRouteMatch> = None;
    }
}
