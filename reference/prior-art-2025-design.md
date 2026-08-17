# Phase 12 — prior art, extracted from memex

Recovered 2026-08-14 from conversations dated 2025-04 to 2025-11. These are
conclusions already reached, so Phase 12 does not rediscover them. Raw hits in
`memex-raw.json`.

---

## 1. Config ingestion — already decided, 2025

The design question was asked and answered directly:

> "No I want to only focus on config from json/yaml to proto generated bindings
> which will be **our config classes**. The actual config will come in as
> typedExtensionConfig like with envoy."

That is proto-first, stated plainly. The generated bindings *are* the config
classes. There is no second hand-written schema in the intended design.

The mechanism, also recovered:

> `google::protobuf::util::JsonStringToMessage`

Envoy's `MessageUtil::loadFromYaml` goes YAML → JSON → `JsonStringToMessage` →
typed proto. **One schema, two encodings.** YAML is not an alternative config
format; it is a serialization of the proto.

The follow-on question in the same thread is the one Phase 12 has to answer in
code:

> "Since the API only defines filter configurations, and filter configurations
> are actual proto messages and not TypedExtensionConfig, then how does envoy
> deserialize them?"

Answer: `TypedExtensionConfig.typed_config` is a `google.protobuf.Any`. The
registry resolves the type URL to a concrete message type, unpacks the `Any`
into it, and hands it to the extension factory. That resolution step is what
`rumi/proto/src/any_resolver.rs` exists to do.

**Implication for x.uma:** the hand-written `MatcherConfig` in Rust, Python and
TypeScript is not the design. It is what got built while codegen was broken.

---

## 2. Envoy's built-in matcher inputs

Six input extensions ship with Envoy for the Matcher API's `input` field:

| Input | Qualified name |
|---|---|
| HTTP request headers | `envoy.matching.inputs.request_headers` |
| HTTP request trailers | `envoy.matching.inputs.request_trailers` |
| HTTP response headers | `envoy.matching.inputs.response_headers` |
| HTTP response trailers | `envoy.matching.inputs.response_trailers` |
| Query parameters | `envoy.matching.inputs.query_params` |
| Environment variables | `envoy.matching.inputs.environment_variable` |

x.uma's `xuma.http.v1.*` inputs (Path, Method, Header, Authority, Scheme,
QueryParam) cover the request side and add pseudo-header accessors Envoy exposes
differently. Trailers and response-side inputs are absent, which matters for
ext_proc response processing.

---

## 3. Consistent hashing — it exists, one layer out

Corrects an assumption made twice, in 2025 and again in 2026-08.

It is **not** in `xds.type.matcher.v3`. It **is** an Envoy input matcher
extension:

```
envoy.extensions.matching.input_matchers.consistent_hashing.v3.ConsistentHashing
registered as: envoy.matching.matchers.consistent_hashing
```

Roughly: hash the input value with a seed, match when
`hash % modulo < threshold`. The point is stability — the same input always
lands the same way, which is what makes percentage rollouts and canaries
deterministic rather than random per request.

**Confirm the exact field names and semantics against the proto before
implementing.** This is recovered from conversation, not read from the schema.

**The pattern this establishes:** xDS core defines a small closed set of value
matchers plus the `custom_match` / `TypedExtensionConfig` seam. Envoy populates
that seam with extensions. x.uma should do the same rather than growing the core
schema. Consistent-hash and semantic matchers are `xuma.core.v1.*` extensions
registered by type URL, not new variants of `StringMatcher`.

---

## 4. Matcher tree — the shape the use case wants

> "Matcher tree from unified matching API would be perfect. For processors.
> Request can only ever be one tenant, so the first decision is always mutually
> exclusive. Then inside the tenant tree, the tree can be nested as needed."

Tenant-first dispatch, mutually exclusive at the top, nested below. That is
`exact_match_map` at the root with nested `Matcher` values, and it is the case
`MatcherTree` and `RadixTree` were built for.

The architectural principle, from the same corpus:

> "How you extract a key, how you look it up, and what action you take are all
> decoupled."

---

## 5. ExtensionWithMatcher and the delegate pattern

```protobuf
message ExtensionWithMatcher {
  xds.type.matcher.v3.Matcher xds_matcher = 3;
  config.core.v3.TypedExtensionConfig extension_config = 2;  // -> Composite
}
```

The Composite filter itself has **no configuration** — it is an empty message.
`ExtensionWithMatcher` holds both the matcher and a pointer to Composite. On
match, Envoy calls `onMatchCallback(action)`; Composite then creates the
delegate filter dynamically from the action, where `ExecuteFilterAction` carries
the filter configuration.

The contrast noted at the time: our SDK evaluated the matcher directly and
extracted the action, rather than handing it to a delegate that constructs
behaviour. Worth revisiting when x.uma grows a filter-dispatch story, because
the delegate indirection is what makes the action carry *configuration* rather
than just a name.

---

## What this changes for Phase 12

1. Proto-first is not a new decision. It was made in 2025 and never implemented.
2. YAML support does not require a second schema. It requires protojson.
3. New matchers go on the extension seam, not into the core schema.
4. Response-side and trailer inputs are a real gap for ext_proc.
5. The `Any` → type URL → concrete message resolution is the load-bearing piece,
   and `any_resolver.rs` has never compiled.
