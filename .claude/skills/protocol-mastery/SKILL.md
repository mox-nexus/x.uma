---
name: protocol-mastery
description: >
  Activates when working with protobuf schemas, buf codegen, xDS protocol family,
  TypedExtensionConfig patterns, Any resolution, control plane integration, ECDS,
  or wire format decisions in the x.uma project.
triggers:
  - proto files (.proto)
  - buf.yaml or buf.gen.yaml
  - xDS types (Matcher, TypedExtensionConfig, Any)
  - codegen directory structure
  - type URL resolution
  - ECDS or control plane integration
  - wire format decisions (binary proto, ProtoJSON, TypedStruct)
version: 1.0.0
---

# Protocol Mastery

Knowledge for working with protobuf, buf, xDS, and wire protocols in x.uma.

## buf Codegen Layout

x.uma uses **Pattern B** (per-package gen directories), same as Connect-ES:

```
rumi/proto/src/gen/    # Rust generated code
puma/proto/src/gen/    # Python generated code
buma/proto/src/gen/    # TypeScript generated code
```

**Pattern A** (monorepo top-level `gen/`) is also valid but x.uma doesn't use it.

### buf.gen.yaml Structure

```yaml
version: v2
clean: true  # Delete old generated code before regenerating
managed:
  enabled: true  # Apply defaults (optimize_for: SPEED, etc.)
  override:
    go_package_prefix: ...  # Only if generating Go
plugins:
  - remote: buf.build/neoeinstein/prost
    out: src/gen
  - remote: buf.build/neoeinstein/prost-serde
    out: src/gen
```

**Key points:**
- `managed: enabled: true` covers most defaults — language overrides only needed for specific targets
- `clean: true` prevents stale generated files (buf.gen.yaml v2 only)
- x.uma **commits generated code** for convenience (like cncf/xds, go-control-plane)
- Alternative: gitignore generated code (like Envoy with Bazel)

## Proto Plugin Choices

| Language | Plugin | Why |
|----------|--------|-----|
| **Rust** | neoeinstein-prost + prost-serde | `prost::Message` + `serde::Deserialize` on same types |
| **Python** | betterproto | Dataclass-based, clean Python idioms, built-in JSON |
| **TypeScript** | ts-proto (current) → protobuf-es (future) | ts-proto v2 works but protobuf-es is only JS lib passing full conformance + Connect-ES native |

### TypeScript Migration Path

**Current:** ts-proto v2.x (internally depends on @bufbuild/protobuf anyway)

**Future:** protobuf-es + Connect-ES 2.0
- protobuf-es: Buf's first-party TS library, full conformance
- Connect-ES 2.0: unified codegen (message + service in single protoc-gen-es plugin)
- Migration when needed for RPC services

## xDS Protocol Family

| Service | Purpose | x.uma Relevance |
|---------|---------|-----------------|
| LDS | Listener Discovery | Future: deliver matcher configs |
| RDS | Route Discovery | Future: HTTP route matching |
| CDS | Cluster Discovery | N/A |
| EDS | Endpoint Discovery | N/A |
| SDS | Secret Discovery | N/A |
| **ECDS** | Extension Config Discovery | **Primary delivery for matcher configs** |

### ECDS for x.uma

**Extension Config Discovery Service** delivers configs independently from listener updates.

```protobuf
// Delivered via ECDS
message TypedExtensionConfig {
  string name = 1;                       // Label only
  google.protobuf.Any typed_config = 2;  // Type resolution via type_url
}
```

**Why ECDS matters:**
- Matcher configs change more frequently than listeners
- Avoids listener reloads for matcher updates
- The Unified Matcher API (xds.type.matcher.v3) is cross-cutting — delivered via any xDS service

**Industry default:** Delta xDS (Istio 1.22+) sends only changed configs, not full state.

## Any Resolution Pattern

`google.protobuf.Any` = type erasure at wire level:

```protobuf
message Any {
  string type_url = 1;  // "type.googleapis.com/xuma.http.v1.HeaderInput"
  bytes value = 2;      // Serialized message
}
```

### Resolution Flow

1. Extract `type_url`
2. Lookup factory in type registry
3. Decode `value` bytes into concrete config
4. Return typed object

### Implementation by Language

| Language | Pattern |
|----------|---------|
| **Envoy (C++)** | `Config::Utility::getFactory<DataInputFactory<T>>(config)` + `translateAnyToFactoryConfig()` |
| **rumi (Rust)** | `AnyResolver::register::<T>(url)` — monomorphization at registration, `Box<dyn Fn>` erasure |
| **prost** | `Any::from_msg()`/`to_msg()` with `Name` trait |
| **betterproto** | `to_json()`/`from_json()` on dataclass messages |
| **protobuf-es** | Built-in `pack()`/`unpack()` with schema objects |

### x.uma Type Registry

```rust
// rumi/proto/src/any_resolver.rs
let mut resolver = AnyResolver::new();
resolver.register::<HeaderInput>("type.googleapis.com/xuma.http.v1.HeaderInput");

// Later, resolve Any bytes:
let input: Box<dyn DataInput<HttpMessage>> = resolver.resolve(&any_config)?;
```

**Constraint:** Registry immutable after initialization (Arch-Guild: Vector, Lamport).

## x.uma vs Envoy Comparison

### What x.uma Matches

| Pattern | Envoy | x.uma |
|---------|-------|-------|
| Type erasure | `MatchingDataType` enum | `MatchingData` (Rust), `MatchingValue` union (Python) |
| DataInput | `DataInput<DataType>` | `DataInput<Ctx>` |
| InputMatcher | Non-generic | Non-generic |
| Predicate composition | AND/OR/NOT | AND/OR/NOT |
| OnMatch exclusivity | Action XOR matcher (proto `oneof`) | Rust `enum OnMatch<Ctx, A>` — stricter |

### What Envoy Has (Not Yet in x.uma)

| Feature | Envoy | x.uma Status |
|---------|-------|--------------|
| Data availability | 3-state: NotAvailable, MoreDataMightBeAvailable, AllDataAvailable | Deferred |
| keep_matching | Records action, returns no-match, continues | Deferred |
| CommonProtocolInput | Protocol-agnostic inputs | Deferred |
| CustomMatcherFactory | IP, domain, CEL matchers | Deferred |

**Current focus:** Core matching semantics, HTTP domain, cross-language conformance.

## Wire Format Decisions

| Format | Size | CPU | Readability | Use When |
|--------|------|-----|-------------|----------|
| **Binary proto** | Smallest | Fastest | None | Wire transport, ECDS delivery |
| **ProtoJSON** | ~3x larger | Slower | Human-readable | Config files, debugging |
| **TypedStruct** | Largest | Slowest | Human-readable | Control planes without proto schema |

### x.uma Choices

- **Config files:** ProtoJSON (prost-serde, betterproto, protobuf-es all support)
- **Wire transport (future):** Binary proto
- **TypedStruct:** Not planned — x.uma control planes will have schema

### TypedStruct vs TypedExtensionConfig

```protobuf
// Standard (has schema)
message TypedExtensionConfig {
  string name = 1;
  google.protobuf.Any typed_config = 2;  // type_url → schema
}

// For JSON-only control planes (no schema)
message TypedStruct {
  string type_url = 1;
  google.protobuf.Struct value = 2;  // Opaque JSON
}
```

**x.uma stance:** TypedExtensionConfig only — simpler, safer, better tooling.

## Codegen Directory Convention

```
<impl>/proto/
├── buf.yaml           # buf workspace config
├── buf.gen.yaml       # codegen config
└── src/
    ├── gen/           # Generated code (committed)
    │   ├── xds/
    │   └── xuma/
    ├── convert.{rs,py,ts}       # proto Matcher → native MatcherConfig
    └── any_resolver.{rs,py,ts}  # Any bytes → native config objects
```

**Pattern:**
- `gen/` = machine-written (buf codegen)
- Sibling modules = human-written (conversion, resolution)

## Package Namespace Convention

Following established patterns:

| Project | Pattern | Example |
|---------|---------|---------|
| Google | `google.<area>.<version>` | `google.protobuf` |
| xDS | `xds.<area>.<version>` | `xds.type.matcher.v3` |
| Envoy | `envoy.<area>.<version>` | `envoy.config.core.v3` |
| **x.uma** | `xuma.<area>.<version>` | `xuma.http.v1` |

### x.uma Namespaces

```
xuma.core.v1      # Base types, registry
xuma.kv.v1      # Conformance testing
xuma.http.v1      # HTTP matching
xuma.claude.v1    # Claude Code hooks
xuma.grpc.v1      # gRPC matching (future)
```

### Type URLs

```
type.googleapis.com/xuma.http.v1.HeaderInput
type.googleapis.com/xuma.http.v1.QueryParameterInput
type.googleapis.com/xuma.kv.v1.MapInput
```

**Convention:** Always `type.googleapis.com/` prefix (de facto standard).

## Proto Style Guide (x.uma)

```protobuf
syntax = "proto3";

package xuma.http.v1;

import "xds/type/matcher/v3/matcher.proto";
import "google/protobuf/wrappers.proto";

// GOOD: Clear field names, field numbers reserved blocks
message HeaderInput {
  // Name of the header to extract (case-insensitive)
  string name = 1;
}

// GOOD: Explicit about behavior
message QueryParameterInput {
  // Parameter name (case-sensitive)
  string name = 1;

  // If false, evaluates to None when parameter absent
  // If true, evaluates to empty string when parameter absent
  bool treat_missing_as_empty = 2;
}
```

### Field Numbering

- 1-15: Single-byte encoding (most common fields)
- 16-2047: Two-byte encoding
- Reserve blocks when deprecating fields

### Documentation

- Every message has a comment
- Every field has a comment
- Explain **behavior**, not just "what" — the "when" and "why"

## Common Pitfalls

| Mistake | Why It's Wrong | Fix |
|---------|----------------|-----|
| `name` in TypedExtensionConfig for type resolution | `name` is a label only | Use `type_url` inside `Any.typed_config` |
| Recursive codegen (proto imports itself) | buf errors on cycles | Extract shared types to separate package |
| Missing `managed: enabled` | Manual field numbering, package paths | Add `managed: enabled: true` |
| Regenerating without `clean: true` | Stale files from renamed/deleted messages | Add `clean: true` in buf.gen.yaml v2 |
| Hard-coding type URLs as strings | Brittle, error-prone | Use proto `Name` trait / descriptor access |

## Source Material

This skill synthesizes from:
- buf.build documentation (codegen patterns, managed mode)
- xDS protocol specification (ECDS, TypedExtensionConfig)
- Envoy source code (`~/oss/envoy/source/common/matcher/`)
- x.uma project conventions (CLAUDE.md, MEMORY.md)
- Connect-ES codegen patterns (protobuf-es + Connect-ES 2.0)

## Quick Reference: buf Commands

```bash
# Generate code for all languages
buf generate

# Generate for specific proto files only
buf generate --path proto/xuma

# Lint proto files
buf lint

# Format proto files
buf format -w

# Check breaking changes
buf breaking --against .git#branch=main

# Update dependencies
buf mod update

# Push to BSR (Buf Schema Registry)
buf push
```

## When to Update This Skill

- New xDS service integration (RDS, LDS)
- Control plane implementation starts
- Wire format requirements change (binary proto, streaming)
- Migration to protobuf-es / Connect-ES
- New language binding (e.g., Go, Java)
- Type registry patterns evolve
