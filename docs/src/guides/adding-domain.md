# Adding a Domain

Create domain-specific matchers for HTTP, gRPC, or your custom protocol.

## Overview

A "domain" in x.uma is a context type with associated `DataInput` implementations.

```text
proto/xuma/<domain>/v1/    → Proto definitions
rumi/ext/<domain>/src/     → Rust implementations
```

## Step 1: Define Proto

```protobuf
// proto/xuma/http/v1/inputs.proto
syntax = "proto3";
package xuma.http.v1;

message HeaderInput {
  string header_name = 1;
}

message PathInput {}
```

## Step 2: Generate Bindings

```bash
just gen
```

## Step 3: Implement DataInput

```rust,ignore
// rumi/ext/http/src/lib.rs
use rumi::{DataInput, MatchingData};
use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpRequest {
    pub headers: HashMap<String, String>,
    pub path: String,
}

#[derive(Debug)]
pub struct HeaderInput {
    pub header_name: String,
}

impl DataInput<HttpRequest> for HeaderInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.headers
            .get(&self.header_name)
            .map(|v| MatchingData::String(v.clone()))
            .unwrap_or(MatchingData::None)
    }
}
```

## Step 4: Add Conformance Tests

```yaml
# spec/tests/http/header_exact.yaml
name: "HTTP header exact match"
cases:
  - description: "matches header value"
    input:
      headers: { "content-type": "application/json" }
    matcher:
      input: { header_name: "content-type" }
      matcher: { exact: "application/json" }
    expected: { matches: true }
```

## Step 5: Feature Gate

```toml
# rumi/ext/http/Cargo.toml
[features]
default = []
```

```rust,ignore
// rumi/ext/http/src/lib.rs
pub mod headers;
pub mod path;
```
