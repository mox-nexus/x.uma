# Policy Landscape

Where x.uma fits in the broader policy engine ecosystem, and why `xuma.act` targets the agent tool control gap.

## x.uma IS a Policy Engine

At its core, x.uma evaluates structured input against composable rules and returns a decision. This is the fundamental contract of every policy engine:

```text
Input → Rules → Decision
```

x.uma implements the xDS Unified Matcher API — the same matching engine Envoy uses for RBAC enforcement, rate limiting, and routing at Google scale. The architecture is proven in production at companies running millions of requests per second.

The mechanics:
1. Structured input (HTTP request, agent context, etc.)
2. Evaluation against a matcher tree (AND/OR/NOT predicates)
3. Return a decision (generic action `A`)

This is policy evaluation. The domain-agnostic core means the same engine handles HTTP routing decisions and agent tool control decisions.

## The Established Standards

The policy engine landscape has mature, well-adopted standards. Each emerged to solve specific problems.

### OPA (Open Policy Agent)

**What it is:** CNCF graduated project maintained by Styra. Domain-agnostic policy evaluation using the Rego language (Datalog-based).

**Model:** Input→Rules→Decision. You write Rego policies that query input documents and produce allow/deny decisions (or complex JSON responses).

**Adoption:** Netflix (microservice authorization), Goldman Sachs (compliance), Kubernetes Gatekeeper (admission control).

**Strength:** Highly flexible. Works for any domain because Rego is a general-purpose query language.

### Cedar

**What it is:** AWS-developed authorization language, open-sourced in 2023. Purpose-built for access control with formal verification.

**Model:** PARC (Principal-Action-Resource-Context). Policies explicitly state which principals can perform which actions on which resources, given context.

```cedar
permit(
  principal == User::"alice",
  action == Action::"readFile",
  resource in Folder::"docs"
);
```

**Adoption:** Amazon Verified Permissions (GA), AWS Verified Access.

**Strength:** Formal verification via Dafny — provably correct policies. The language design prevents entire classes of authorization bugs.

### XACML

**What it is:** OASIS standard since 2001. XML-based policy language that coined the PEP/PDP/PAP/PIP vocabulary the entire industry uses.

**Model:** Policy Decision Point (PDP) evaluates requests from Policy Enforcement Points (PEP). Policies authored in Policy Administration Points (PAP), context from Policy Information Points (PIP).

**Adoption:** Government, healthcare, finance (HIPAA compliance workflows).

**Strength:** Standardization. When two systems need to interoperate on authorization, XACML is the lingua franca.

### Zanzibar and Relationship Engines

**What they are:** Google's Zanzibar (2019 paper), OpenFGA, SpiceDB. Relationship-based access control (ReBAC) via graph traversal.

**Model:** Not rule evaluation. Instead: "Does a path exist in the authorization graph from user to resource?"

Example: "Can alice view doc123?" → Check if graph contains `alice -[member]→ team -[viewer]→ doc123`.

**Adoption:** Google (Drive, Calendar, Cloud), Airbnb (OpenFGA), Auth0 (FGA product).

**Strength:** Scales to billions of relationships. Natural fit for social graphs and hierarchical permissions.

**Key difference:** ReBAC engines don't evaluate predicates. They traverse graphs. Different problem space.

## The Agent Policy Gap

As of February 2026, there is no established standard for AI agent tool control policies. The space is fragmented.

### What Exists Today

**AWS Bedrock AgentCore** (re:Invent 2025): Cedar-based policy engine for agent-to-tool interactions. Closest production system. Uses Cedar's PARC model: agent is principal, tool is action, parameters are context.

**OpenAI Agents SDK**: "Guardrails" with binary tripwires. Simple allow/deny on tool invocations based on pattern matching.

**NVIDIA NeMo Guardrails**: "Rails" DSL (Colang) for conversation flow control. Includes tool call filtering but tightly coupled to NeMo.

**Claude Code**: Hooks system with matcher patterns. `PreToolUse` hooks support allow/deny/ask (ternary decision) plus parameter mutation.

**MCP (Model Context Protocol)**: No built-in policy layer. Delegates to host implementation. The protocol itself is policy-agnostic.

### Industry Recognition of the Gap

**OWASP Top 10 for Agentic Applications (2026)** identifies "Tool Misuse" as risk #2. Recommendations:
- Policy controls at tool boundaries
- Principle of Least Agency (minimize tool access)
- Audit logging of tool invocations

**MIT Technology Review** (Feb 2026, "From Guardrails to Governance"): Advocates treating agents as semi-autonomous users and enforcing rules at boundaries, not just prompting.

The pattern is clear: the industry recognizes the need for policy control but lacks a unified standard. Current approaches are vendor-specific.

## Where x.uma Fits

x.uma is uniquely positioned because it was designed domain-agnostic from day one.

### Why This Matters

Most policy engines are built for one domain and later adapted:
- OPA: started as microservice authorization, generalized
- Cedar: built for AWS resource authorization
- Guardrails: built for LLM conversation control

x.uma started with the insight that **matching is matching**, whether you're routing HTTP traffic or controlling agent tool access. The same AND/OR/NOT predicate logic, the same first-match-wins semantics, the same depth limits and ReDoS protection.

The architecture makes this natural:

```text
┌─────────────────────────────────┐
│       Domain Adapters           │
│  xuma.http  xuma.act  xuma.grpc │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│            PORTS                │
│   DataInput      ActionPort     │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│            CORE                 │
│        Matcher Engine           │
│    (domain-agnostic)            │
└─────────────────────────────────┘
```

Adding agent tool control is just another domain adapter. Core unchanged.

### Cross-Language Reality

Real systems are polyglot:
- ML inference pipelines: Python
- Edge workers: TypeScript (Cloudflare, Deno)
- High-throughput services: Rust

x.uma works natively in all three:
- **rumi** (Rust): Zero-overhead, Send+Sync for concurrency
- **puma** (Python): Pure Python, no FFI for simple cases
- **bumi** (TypeScript): Edge-native, runs in V8 isolates

Plus Rust-backed bindings (`puma-crusty`, `bumi-crusty`) when performance matters.

This isn't just convenience. Agent systems span languages. Your agent runtime might be Python, your ext_proc filter might be Rust, your edge worker might be TypeScript. One policy standard that works everywhere.

### Battle-Tested Semantics

x.uma doesn't invent new policy semantics. It implements xDS Unified Matcher API — the protocol Envoy uses. This means:

- **Proven at scale**: Google runs this in production
- **Well-specified**: xDS proto definitions are the source of truth
- **Debugged edge cases**: nested matcher failure semantics, on_no_match fallback behavior

You're not betting on a new policy language. You're using Google's battle-tested approach in a new domain.

## xuma.act — Agent Control of Tools

The `xuma.act` domain applies x.uma's matcher architecture to agent tool control.

### Mapping the PARC Model

Cedar's PARC model (Principal-Action-Resource-Context) maps naturally to agent scenarios:

| PARC Element | Agent Equivalent | Example |
|--------------|------------------|---------|
| **Principal** | Agent (or user delegating to agent) | `agent:claude-sonnet-4` |
| **Action** | Tool being invoked | `tools:bash`, `tools:read_file` |
| **Resource** | What the tool operates on | `/etc/passwd`, `s3://bucket/data` |
| **Context** | Parameters, session state | `{depth: 5, recursive: true}` |

### Decision Model Spectrum

`xuma.act` supports the full decision spectrum:

**Binary:** Allow or deny.
```yaml
action: { allow: true }
```

**Ternary:** Allow, deny, or escalate to human.
```yaml
action: { escalate: { reason: "Sensitive file access" } }
```

**Mutation:** Modify tool input before execution.
```yaml
action: {
  modify: {
    max_depth: 3,  # Override requested depth
    exclude_patterns: ["*.key", "*.pem"]
  }
}
```

This mirrors Claude Code's hook model (allow/deny/ask + modify) but generalizes to any agent framework.

### Proto Namespace

`xuma.act.v1` is the proto package for agent tool control.

Type URLs follow the standard pattern:
- `type.googleapis.com/xuma.act.v1.ToolInvocation`
- `type.googleapis.com/xuma.act.v1.AgentContext`
- `type.googleapis.com/xuma.act.v1.Decision`

First implementation target: Claude Code hooks. The adapter translates Claude's hook context into `ToolInvocation`, evaluates via matcher tree, returns `Decision`.

## Comparison to Established Engines

| | OPA | Cedar | x.uma |
|---|---|---|---|
| **Language** | Rego (Datalog) | Cedar (custom) | Matcher trees (compiled from proto) |
| **Model** | Input→Rules→JSON | PARC→permit/forbid | Context→Predicates→Action |
| **Domain focus** | Agnostic | Authorization | Agnostic |
| **Agent-native** | No | Partial (AgentCore uses it) | Yes (`xuma.act` first-class) |
| **Cross-language** | Go + WASM bindings | Rust + Java bindings | Rust, Python, TypeScript (native + bindings) |
| **Formal verification** | No | Yes (Dafny proofs) | Type safety + depth limits + linear-time regex |
| **Extension model** | Rego functions | Cedar extensions (limited) | `TypedExtensionConfig` (open) |
| **Evaluation model** | Query language | Policy search + decision | Tree traversal (first-match-wins) |

### When to Use What

**Use OPA if:**
- You need a general-purpose policy language
- Complex queries over input data
- Integration with Kubernetes (Gatekeeper)

**Use Cedar if:**
- AWS ecosystem
- Formal verification is critical
- Classic authorization (PARC model is natural)

**Use x.uma if:**
- Agent tool control is your domain
- Polyglot codebase (Rust + Python + TypeScript)
- You want Envoy-proven semantics
- Domain adapters fit your architecture

They're not mutually exclusive. Some systems use OPA for high-level policy and x.uma for low-level matcher logic (HTTP routing, tool filtering).

## The Vision

x.uma is alpha (v0.1). This is the roadmap:

**Phase 6-7 (current):** Complete `bumi` (TypeScript) and Rust bindings (`puma-crusty`, `bumi-crusty`).

**Phase 8:** `xuma.act` domain with Claude Code adapter.

**Phase 9:** Benchmarks. Prove performance against native implementations.

**v1.0:** Lock core traits. Extension ecosystem opens.

The goal is simple: make x.uma the policy engine for agent tool control. Not by inventing new semantics, but by bringing battle-tested xDS matchers to the agent domain.

The gap exists. The need is recognized (OWASP, MIT Tech Review, AWS AgentCore). x.uma fills it with production-proven architecture, cross-language support, and domain-agnostic design.

## Sources

**Policy Standards:**
- [OPA Documentation](https://www.openpolicyagent.org/) — CNCF graduated project
- [Cedar Language Guide](https://www.cedarpolicy.com/) — AWS open-source authorization language
- [XACML v3.0 Specification](https://www.oasis-open.org/committees/xacml/) — OASIS standard
- [Zanzibar: Google's Authorization System](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/) — ReBAC research paper

**Agent Policy Landscape:**
- [AWS Bedrock AgentCore Policy Engine](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/policy.html) — Cedar-based agent authorization
- [OpenAI Agents SDK Guardrails](https://openai.github.io/openai-agents-python/guardrails/) — Binary tripwire model
- [NVIDIA NeMo Guardrails](https://github.com/NVIDIA-NeMo/Guardrails) — Rails DSL for conversation control
- [Claude Code Hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) — PreToolUse matcher patterns
- [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25) — Protocol specification

**Security & Governance:**
- [OWASP Top 10 for Agentic Applications (2026)](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) — Tool Misuse as risk #2
- [MIT Technology Review: From Guardrails to Governance](https://www.technologyreview.com/2026/02/04/1131014/from-guardrails-to-governance-a-ceos-guide-for-securing-agentic-systems/) — Treating agents as semi-autonomous users

**xDS & Envoy:**
- [xDS Protocol](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol) — Service mesh configuration protocol
- [Envoy Matcher Implementation](https://github.com/envoyproxy/envoy/tree/main/source/common/matcher) — Reference C++ implementation
