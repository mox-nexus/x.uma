# rumi-proto

xDS and `xuma.*` protobuf types for [rumi](https://crates.io/crates/rumi-core),
plus the two pieces that make them useful:

- **`AnyResolver`** — decodes a `google.protobuf.Any` into a registered config
  type. A closed-world allowlist: decoders are monomorphised at registration and
  an unregistered `type_url` is an error, so there is no reflective type lookup
  to steer.
- **`convert`** — turns an xDS `Matcher` message into a rumi config, which is
  the path a control plane's config takes to become a matcher.

Generated code is committed rather than produced by a `build.rs`, so consuming
this crate needs no `protoc` and no network access. The codegen plugins are
pinned in the repo's `buf.gen.yaml`.

## Scope

x.uma never speaks xDS itself — no client, no subscription, no transport. The
host owns all of that. This crate covers only the step from a proto message to a
matcher.
