//! rumi-http: HTTP domain for request matching
//!
//! This crate provides two layers:
//!
//! 1. **User API**: Gateway API `HttpRouteMatch` for configuration
//! 2. **Data Plane API**: `HttpMessage` (indexed from `ProcessingRequest`) for runtime
//!
//! # Architecture
//!
//! ```text
//! Gateway API HttpRouteMatch (config)
//!         ↓ compile()
//! rumi Matcher<HttpMessage, A>
//!         ↓ evaluate()
//! HttpMessage (indexed from ext_proc ProcessingRequest)
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
//! // Runtime: index ProcessingRequest into HttpMessage, then evaluate
//! let msg = HttpMessage::from(&processing_request);
//! let result = matcher.evaluate(&msg);
//! ```

mod compiler;
mod context;
mod inputs;
mod message;
mod simple;

pub use compiler::*;
pub use inputs::*;
pub use message::HttpMessage;
pub use simple::{
    HttpRequest, HttpRequestBuilder, SimpleHeaderInput, SimpleMethodInput, SimplePathInput,
    SimpleQueryParamInput,
};

// Re-export ext_proc types for convenience
pub use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    ProcessingRequest, ProcessingResponse,
};

// Re-export Gateway API types for convenience
pub use k8s_gateway_api::{
    HttpHeaderMatch, HttpMethod, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        // DataInputs for HttpMessage
        AuthorityInput,
        HeaderInput,
        // Re-exports: Gateway API
        HttpHeaderMatch,
        // Indexed context
        HttpMessage,
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
        let _: Option<ProcessingRequest> = None;
        let _: Option<HttpRouteMatch> = None;
        let _: Option<HttpMessage> = None;
    }
}
