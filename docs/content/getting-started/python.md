# Python Quick Start

Build an HTTP route matcher with `xuma-crust`: the Rust engine, bound through PyO3.

## Install

> **Not yet on registries.** No release has been cut, so these packages do not
> resolve yet. Until the first release, clone the repo and run `just build`.
> See the [README](https://github.com/mox-nexus/x.uma#install).

```bash
uv add xuma-crust
```

Requires Python 3.12+.

## Write a Config

Create `routes.yaml`:

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: path
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.PathInput
                valueMatch:
                  prefix: /api
            - singlePredicate:
                input:
                  name: method
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.MethodInput
                valueMatch:
                  exact: GET
      onMatch:
        action:
          name: api_read
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: api_read

    - predicate:
        singlePredicate:
          input:
            name: path
            typedConfig:
              "@type": type.googleapis.com/xuma.http.v1.PathInput
          valueMatch:
            exact: /health
      onMatch:
        action:
          name: health
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: health

onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
```

## Validate with the CLI

```bash
$ rumi check http routes.yaml
Config valid
  rules:     2
  inputs:    xuma.http.v1.MethodInput, xuma.http.v1.PathInput
  fallback:  yes
```

## Run with the CLI

```bash
$ rumi run http routes.yaml --method GET --path /api/users
api_read

$ rumi run http routes.yaml --method GET --path /health
health

$ rumi run http routes.yaml --method DELETE --path /other
not_found
```

## Load in Your App

`from_config` takes the same document the CLI read:

```python
import json

import yaml
from xuma_crust import HttpMatcher

# from_config takes canonical protojson as a string, so the YAML becomes JSON.
with open("routes.yaml") as f:
    matcher = HttpMatcher.from_config(json.dumps(yaml.safe_load(f)))

# Evaluate with method + path
assert matcher.evaluate("GET", "/api/users") == "api_read"
assert matcher.evaluate("DELETE", "/other") == "not_found"
```

## Safety

- **ReDoS protection** -- regex runs in Rust's `regex` crate: linear time, no backtracking.
- **Immutable** -- a compiled matcher cannot be changed after `from_config`.
- **Bounded** -- nesting depth, rule count and pattern length are limited at load, not at evaluation.
- **Missing is not a match** -- a missing header or query parameter makes its predicate `False`. Under `not` that becomes a match, so mark an input `required` when its absence must not decide anything: the result then says `undecided`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) -- how data flows through the matcher
- [CLI Reference](../reference/cli.md) -- all commands and domains
- [Config Format](../reference/config.md) -- full config schema and type URL tables
- [API Reference](../reference/api.md) -- generated docs for all languages
