# rumi-cli

CLI for the [rumi](https://crates.io/crates/rumi-core) matcher engine.

## Install

```sh
cargo install rumi-cli
```

This installs the `rumi` binary.

## Usage

```sh
# Run a config against key=value context
rumi run config.yaml --context method=GET path=/api

# Validate a config file
rumi check config.yaml

# List registered type URLs
rumi info
```

## Config format

Configs are YAML or JSON files, written as canonical protojson — protobuf's own
JSON mapping of `xds.type.matcher.v3.Matcher`. See the
[x.uma documentation](https://github.com/mox-nexus/x.uma) for the full schema.

```yaml
matcherList:
  matchers:
    - predicate:
        singlePredicate:
          input:
            name: method
            typedConfig:
              "@type": type.googleapis.com/xuma.kv.v1.MapInput
              key: method
          valueMatch:
            exact: GET
      onMatch:
        action:
          name: route-get
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: route-get
onNoMatch:
  action:
    name: fallback
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: fallback
```
