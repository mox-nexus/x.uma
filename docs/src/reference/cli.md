# CLI Reference (rumi)

`rumi` is the command-line interface for running and validating matcher configs across three domains.

## Installation

```bash
cargo install --path rumi/cli
```

## Usage

```
rumi <command> [domain] [options]
```

## Domains

The CLI supports three matching domains. Each domain has its own registry of type URLs and context type.

| Domain | Context | Description |
|--------|---------|-------------|
| *(default)* | Key-value pairs | Test domain (`xuma.test.v1.*`) |
| `http` | HTTP request | Method, path, headers, query params (`xuma.http.v1.*`) |
| `claude` | Hook event | Claude Code hook events (`xuma.claude.v1.*`) |

## Commands

### run

Run a config file against a context and print the resulting action.

**Test domain (default):**

```bash
rumi run config.yaml --context method=GET path=/api
```

| Flag | Description |
|------|-------------|
| `--context key=value...` | Context key-value pairs |

**HTTP domain:**

```bash
rumi run http routes.yaml --method GET --path /api/users
rumi run http routes.yaml --method POST --path /api --header content-type=application/json
```

| Flag | Description |
|------|-------------|
| `--method METHOD` | HTTP method (required) |
| `--path PATH` | Request path (required) |
| `--header key=value` | Header (repeatable) |
| `--query key=value` | Query parameter (repeatable) |

**Claude domain:**

```bash
rumi run claude hooks.yaml --event PreToolUse --tool Bash --arg command="ls -la"
rumi run claude hooks.yaml --event SessionStart --cwd /home/user --branch main
```

| Flag | Description |
|------|-------------|
| `--event EVENT` | Hook event name (required) |
| `--tool NAME` | Tool name |
| `--arg key=value` | Tool argument (repeatable) |
| `--cwd PATH` | Working directory |
| `--branch NAME` | Git branch |
| `--session ID` | Session ID |

Valid events: `PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, `UserPromptSubmit`, `SessionStart`, `SessionEnd`, `PreCompact`, `Notification`.

Prints the matched action string, or `(no match)` if nothing matched.

### check

Validate a config file without evaluating:

```bash
rumi check config.yaml
rumi check http routes.yaml
rumi check claude hooks.yaml
```

Loads the config against the domain's registry (including type URL resolution and depth validation). Catches: unknown type URLs, invalid regex patterns, depth limit violations, malformed config.

Prints `Config valid` on success. Exits with non-zero status on error.

### info

List all registered type URLs for a domain:

```bash
$ rumi info
Registered inputs:
  xuma.test.v1.StringInput

Registered matchers:
  xuma.core.v1.StringMatcher
  xuma.core.v1.BoolMatcher

$ rumi info http
Registered inputs:
  xuma.http.v1.PathInput
  xuma.http.v1.MethodInput
  xuma.http.v1.HeaderInput
  xuma.http.v1.QueryParamInput

Registered matchers:
  xuma.core.v1.StringMatcher
  xuma.core.v1.BoolMatcher

$ rumi info claude
Registered inputs:
  xuma.claude.v1.EventInput
  xuma.claude.v1.ToolNameInput
  xuma.claude.v1.ArgumentInput
  xuma.claude.v1.SessionIdInput
  xuma.claude.v1.CwdInput
  xuma.claude.v1.GitBranchInput

Registered matchers:
  xuma.core.v1.StringMatcher
  xuma.core.v1.BoolMatcher
```

### help

```bash
rumi help
rumi --help
rumi -h
```

## Config File Format

The CLI accepts the same config format used by all implementations. See [Config Format](config.md) for the full schema. Files can be YAML (`.yaml`, `.yml`) or JSON (`.json`).

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (invalid config, unknown command, missing flags, etc.) |

## Design

The CLI has zero runtime dependencies beyond `rumi`, `rumi-http`, and `rumi-test`. No `clap` -- argument parsing is hand-written. The binary is small and builds fast.

Each domain registers its own `Registry<Ctx>`:
- Test: `rumi_test::register()` -> `Registry<TestContext>`
- HTTP: `rumi_http::register_simple()` -> `Registry<HttpRequest>`
- Claude: `rumi::claude::register()` -> `Registry<HookContext>`
