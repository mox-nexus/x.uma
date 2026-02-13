# When to Use x.uma

x.uma isn't the right tool for everything. Here's when it fits and when other engines are better.

## Use x.uma When

### HTTP Routing and Traffic Control

You're building HTTP routers, API gateways, or traffic shapers. x.uma implements the same matching semantics Envoy uses for routing at Google scale.

**Concrete use cases:**
- Multi-tenant API gateways (route by header, method, path)
- Feature flags based on request context
- A/B testing routing (route users to backend variants)
- Traffic splitting for canary deployments
- Rate limit rule matching before quota enforcement

**Why x.uma:** Gateway API compiler built-in. `HttpRouteMatch` → `Matcher` translation is a first-class feature. Path prefix matching, header matching, query param matching — all tested against 268 conformance fixtures.

### Event Filtering and Message Routing

You're processing event streams (Kafka, Kinesis, message queues) and need first-match-wins routing logic.

**Concrete use cases:**
- CloudEvent filtering by type, source, extensions
- Message bus routing (AMQP, MQTT)
- Webhook dispatchers (route incoming webhooks by headers, payload fields)
- Log aggregation pipelines (route by severity, source, tags)

**Why x.uma:** Domain-agnostic core. Implement `DataInput` for your event type, reuse all the predicate composition and string matching. The same AND/OR/NOT logic, the same depth limits.

### Polyglot Codebases

Your system spans Rust, Python, and TypeScript. You need consistent matching behavior across all three.

**Concrete use cases:**
- Rust ext_proc filter + Python control plane + TypeScript edge workers
- ML inference pipelines (Python) calling routing services (Rust)
- Cloudflare Workers (TypeScript) with backend services (Rust)

**Why x.uma:** Three native implementations (rumi, puma, bumi) plus Rust bindings (puma-crusty, bumi-crusty). All pass the same test suite. Write matchers once, run anywhere.

### Envoy-Proven Semantics

You want xDS Unified Matcher API semantics without running Envoy. Maybe you're building a sidecar, a policy engine, or a custom proxy.

**Concrete use cases:**
- Custom proxies that need Envoy-compatible route matching
- Policy engines that compile xDS `Matcher` proto into evaluation logic
- Testing Envoy configs without spinning up Envoy itself

**Why x.uma:** Direct implementation of the xDS spec. The proto definitions are the source of truth. Nested matcher failure propagation, on_no_match fallback, first-match-wins — all match Envoy's behavior.

## Use OPA When

### General-Purpose Policy Language

You need to query complex input documents, join data from multiple sources, or express policies that don't fit a tree structure.

**Example:** "Allow if user is in the 'engineering' team AND the resource has tag 'internal' AND it's a weekday during business hours."

OPA's Rego is Datalog — you can express arbitrary joins and aggregations. x.uma is tree traversal with first-match-wins. Different models.

### Kubernetes Admission Control

You're using Gatekeeper to enforce OPA policies as Kubernetes admission webhooks.

The CNCF ecosystem is built around OPA + Gatekeeper. x.uma has no Kubernetes-specific tooling.

### Compliance and Audit

You need to generate compliance reports showing which policies matched, which conditions were evaluated, and why a decision was made.

OPA has mature audit tooling. x.uma has trace output (evaluate + explain), but it's debugging-focused, not compliance-focused.

## Use Cedar When

### AWS Ecosystem

You're using AWS services (Verified Permissions, Verified Access, Cognito) or building authorization for AWS-integrated apps.

Cedar integrates natively with AWS IAM, Cognito user pools, and resource tags. x.uma has no AWS-specific integrations.

### Formal Verification

You need provably correct policies. Cedar uses Dafny to verify policies can't produce unintended outcomes (no confused deputy bugs, no authorization bypass).

x.uma has type safety and depth limits, but no formal verification.

### Classic Authorization (PARC Model)

Your domain maps cleanly to Principal-Action-Resource-Context. You're answering "Can this user perform this action on this resource?"

Cedar is purpose-built for this. x.uma is matcher trees — you can represent PARC, but Cedar makes it native.

## Use Zanzibar/OpenFGA When

### Graph-Based Permissions

Your authorization model is a graph. Users → teams → folders → documents. You need to answer "Can alice view doc123?" by checking if a path exists.

**Example use cases:**
- Google Drive-style sharing (folders inherit permissions)
- Organization hierarchies (managers can approve for their reports)
- Social graphs (friends-of-friends visibility)

x.uma evaluates predicates against structured input. It doesn't traverse graphs. Wrong tool.

## Comparison Table

| Feature | x.uma | OPA | Cedar | Zanzibar/OpenFGA |
|---------|-------|-----|-------|------------------|
| **Model** | Tree traversal, first-match-wins | Datalog queries | PARC authorization | Graph traversal |
| **Language** | Matcher proto (compiled) | Rego | Cedar | Tuple-based |
| **Domain** | Agnostic | Agnostic | Authorization | Authorization |
| **Evaluation** | O(depth × predicates) | O(policy complexity) | O(policy complexity) | O(graph traversal) |
| **Cross-language** | Rust, Python, TypeScript (native) | Go, WASM bindings | Rust, Java bindings | Go, varies by impl |
| **Formal verification** | No | No | Yes (Dafny) | No |
| **HTTP routing** | Built-in (Gateway API) | DIY | DIY | Not applicable |
| **Best for** | Routing, filtering, polyglot | General policy | AWS, formal correctness | Social graphs, hierarchies |

## They're Not Mutually Exclusive

Some systems combine engines:

**OPA + x.uma:** OPA makes high-level allow/deny decision. If allowed, x.uma routes to specific backend based on request attributes.

Example: OPA checks "Is this user authorized for the admin API?" (yes/no). x.uma routes to admin-v1 vs admin-v2 based on feature flags in headers.

**Cedar + x.uma:** Cedar handles "can this agent invoke this tool?" x.uma handles "which tool variant should execute given these parameters?"

Example: Cedar checks "Can agent use read_file?" (yes/no). x.uma routes to sandboxed reader vs full reader based on path prefix.

**OpenFGA + x.uma:** OpenFGA answers "does user have viewer permission on resource?" x.uma handles "which cache strategy for this request?"

Example: OpenFGA checks graph for user→resource edge. x.uma routes to CDN vs origin based on request headers.

## When x.uma Is Wrong

### Complex Joins and Aggregations

If your policy is "allow if user belongs to a team that owns a resource in a project with budget > $1000", you need Rego. x.uma is tree traversal, not SQL.

### Stateful Decisions

If your policy depends on "number of requests in the last 5 minutes" or "this is the 3rd failed attempt", x.uma doesn't help. It evaluates one request at a time with no state.

Use rate limiters (Envoy's rate limit service, Redis + Lua) or stateful policy engines (OPA with external data).

### Graph Queries

If your policy is "allow if there's a path in the org chart from user to resource owner", you need a graph database or Zanzibar-style engine.

x.uma evaluates predicates. It doesn't traverse relationships.

## Decision Framework

Answer these questions:

| Question | Answer | Recommendation |
|----------|--------|----------------|
| Is this HTTP routing or event filtering? | Yes | **x.uma** |
| Do I need graph traversal? | Yes | **OpenFGA/SpiceDB** |
| Is this classic authorization (can user X do Y on Z)? | Yes | **Cedar** (if AWS) or **OPA** |
| Do I need Datalog-style queries? | Yes | **OPA** |
| Is my codebase polyglot (Rust + Python + TypeScript)? | Yes | **x.uma** |
| Do I need formal verification? | Yes | **Cedar** |
| Am I running in Kubernetes? | Yes | **OPA + Gatekeeper** |

## Performance Considerations

x.uma is fast (sub-microsecond evaluation, linear-time regex in Rust implementations). But:

- **For complex policies with hundreds of rules:** Compiled Rego (OPA) might be faster. Benchmark your workload.
- **For high-cardinality attribute checks:** Redis + Lua or custom C++ might be faster. x.uma is general-purpose, not hyper-optimized.
- **For graph queries:** Specialized graph databases (Zanzibar, SpiceDB) will be orders of magnitude faster.

See Performance > Benchmarks for concrete numbers.

## Sources

**Policy Engines:**
- [OPA Documentation](https://www.openpolicyagent.org/) — CNCF graduated project
- [Cedar Language Guide](https://www.cedarpolicy.com/) — AWS authorization language
- [Zanzibar: Google's Authorization System](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/) — ReBAC research
- [OpenFGA](https://openfga.dev/) — Open-source Zanzibar implementation

**x.uma Positioning:**
- [xDS Unified Matcher API](https://www.envoyproxy.io/docs/envoy/latest/api-v3/xds/type/matcher/v3/matcher.proto) — Source protocol
- [Gateway API Specification](https://gateway-api.sigs.k8s.io/) — HTTP route matching semantics
- x.uma implements xDS for polyglot routing and filtering, not general authorization
