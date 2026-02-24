# Proto API Reference

x.uma extends the xDS Unified Matcher API with domain-specific proto definitions under the `xuma` namespace.

## Namespaces

| Package | Domain | Location |
|---------|--------|----------|
| `xuma.core.v1` | Core action types | `proto/xuma/core/v1/` |
| `xuma.test.v1` | Test/conformance domain | `proto/xuma/test/v1/` |
| `xuma.http.v1` | HTTP matching | `proto/xuma/http/v1/` |
| `xuma.claude.v1` | Claude Code hooks | `proto/xuma/claude/v1/` |

## Type URL Convention

Type URLs follow the pattern: `xuma.<domain>.<version>.<TypeName>`

Examples:
- `xuma.core.v1.StringMatcher`
- `xuma.test.v1.StringInput`
- `xuma.http.v1.PathInput`
- `xuma.claude.v1.ToolNameInput`

These type URLs are used in `TypedConfig` references within the config format:

```json
{ "type_url": "xuma.http.v1.HeaderInput", "config": { "name": "authorization" } }
```

## xuma.core.v1

### NamedAction

```protobuf
message NamedAction {
  string name = 1;
  map<string, string> metadata = 2;
}
```

Generic action returned on match. `name` is the action identifier; `metadata` carries optional key-value pairs.

## xuma.test.v1

### StringInput

Input for the test domain. Extracts a named string value from a key-value context.

Config: `{ "key": "field_name" }`

### TestContext (Runtime)

A `HashMap<String, String>` context for conformance testing. Not a proto message — used only in test runners.

## xuma.http.v1

### PathInput

Extracts the HTTP request path. No config required.

### MethodInput

Extracts the HTTP method. No config required.

### HeaderInput

Extracts an HTTP header value by name (case-insensitive). Returns null when absent.

Config: `{ "name": "header_name" }`

### QueryParamInput

Extracts an HTTP query parameter by name. Returns null when absent.

Config: `{ "name": "param_name" }`

## xuma.claude.v1

### EventInput

Extracts the hook event type as a string. No config required.

### ToolNameInput

Extracts the tool name. No config required.

### ArgumentInput

Extracts a tool argument value by name. Returns null when absent.

Config: `{ "name": "argument_name" }`

### SessionIdInput

Extracts the session identifier. No config required.

### CwdInput

Extracts the current working directory. No config required.

### GitBranchInput

Extracts the git branch name. Returns null when not in a repository.

## Code Generation

Proto definitions are compiled with [buf](https://buf.build/) to three languages:

| Language | Generator | Output |
|----------|-----------|--------|
| Rust | `prost` + `prost-serde` | `rumi/proto/src/gen/` |
| Python | `betterproto` | `puma/proto/src/gen/` |
| TypeScript | `ts-proto` | `bumi/proto/src/gen/` |

Configuration: `buf.gen.yaml` at project root.

## xDS Foundation

x.uma's proto types are designed to be compatible with the xDS `TypedExtensionConfig` extension mechanism. The xDS matcher proto (`xds.type.matcher.v3.Matcher`) uses `TypedExtensionConfig` for both inputs and actions. x.uma domain types register their proto type URLs, enabling interop with xDS-native tooling.

The xDS matcher proto itself is consumed as a buf dependency, not vendored.
