# rumi-cli

CLI for the [rumi](https://crates.io/crates/rumi-core) matcher engine.

## Install

```sh
cargo install rumi-cli
```

This installs the `rumi` binary.

## Usage

```sh
# Evaluate a config against key=value context
rumi eval config.yaml --context method=GET path=/api

# Validate a config file
rumi check config.yaml

# List registered type URLs
rumi info
```

## Config format

Configs are YAML or JSON files describing a matcher tree. See the
[x.uma documentation](https://github.com/mox-nexus/x.uma) for the full schema.

```yaml
matchers:
  - predicate:
      type: single
      input:
        type_url: "xuma.test.v1.StringInput"
        config:
          key: "method"
      value_match:
        Exact: "GET"
    on_match:
      type: action
      action: "route-get"
on_no_match:
  type: action
  action: "fallback"
```
