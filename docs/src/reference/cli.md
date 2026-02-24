# CLI Reference (rumi)

`rumi` is the command-line interface for evaluating and validating matcher configs.

## Installation

```bash
cargo install --path rumi/cli
```

## Commands

### eval

Evaluate a config file against a context:

```bash
rumi eval config.yaml --context method=GET path=/api
```

Loads the config, builds the matcher, evaluates against the provided context, and prints the resulting action. Prints `(no match)` if nothing matched.

| Flag | Description |
|------|-------------|
| `--context key=value...` | Context key-value pairs |

The config file can be YAML (`.yaml`, `.yml`) or JSON (`.json`). Context values are passed as string key-value pairs.

### check

Validate a config file without evaluating:

```bash
rumi check config.yaml
```

Loads the config, builds the matcher (including registry resolution and depth validation), and reports success or failure. Catches: unknown type URLs, invalid regex patterns, depth limit violations, malformed config.

Prints `Config valid` on success. Exits with non-zero status on error.

### info

List all registered type URLs:

```bash
rumi info
```

Output:

```
Registered inputs:
  xuma.test.v1.StringInput

Registered matchers:
  xuma.core.v1.StringMatcher
  xuma.core.v1.BoolMatcher
```

Shows what types the CLI can resolve when loading configs. The CLI registers the test domain by default.

### help

```bash
rumi help
rumi --help
rumi -h
```

## Config File Format

The CLI accepts the same config format used by all implementations. See [Config Format](config.md) for the full schema.

Example `config.yaml`:

```yaml
matchers:
  - predicate:
      type: single
      input: { type_url: "xuma.test.v1.StringInput", config: { key: "method" } }
      value_match: { Exact: "GET" }
    on_match: { type: action, action: "route-get" }
  - predicate:
      type: single
      input: { type_url: "xuma.test.v1.StringInput", config: { key: "method" } }
      value_match: { Exact: "POST" }
    on_match: { type: action, action: "route-post" }
on_no_match: { type: action, action: "fallback" }
```

```bash
$ rumi eval config.yaml --context method=GET
route-get

$ rumi eval config.yaml --context method=DELETE
fallback

$ rumi check config.yaml
Config valid
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (invalid config, unknown command, etc.) |

## Design

The CLI has zero dependencies beyond `rumi` and `rumi-test`. No `clap` — argument parsing is hand-written. The binary is small and builds fast.

The CLI uses the **config path**: JSON/YAML → `MatcherConfig` → `Registry::load_matcher()` → evaluate. It registers the test domain (`xuma.test.v1.*`), which provides `StringInput` for key-value context.
