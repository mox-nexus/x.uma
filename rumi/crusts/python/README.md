# xuma-crust

Rust-backed bindings for the [x.uma](https://github.com/mox-nexus/x.uma) matcher
engine, built with PyO3.

The engine itself, in a wheel: the same protojson config the `rumi` CLI reads,
and the same conformance suite (`spec/tests/07_protojson/`). A compiled matcher
is an opaque, frozen handle; evaluation never leaves Rust.

```bash
uv add xuma-crust
```

`from_config` takes canonical protojson — the same document every x.uma
implementation reads. If your config is YAML, load it and re-serialise
(`json.dumps(yaml.safe_load(...))`).

```python
from xuma_crust import HttpMatcher

CONFIG = """
{
  "matcherList": {
    "matchers": [{
      "predicate": {
        "singlePredicate": {
          "input": {
            "name": "path",
            "typedConfig": {"@type": "type.googleapis.com/xuma.http.v1.PathInput"}
          },
          "valueMatch": {"prefix": "/api"}
        }
      },
      "onMatch": {
        "action": {
          "name": "api",
          "typedConfig": {
            "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
            "name": "api"
          }
        }
      }
    }]
  }
}
"""

matcher = HttpMatcher.from_config(CONFIG)
assert matcher.evaluate(method="GET", path="/api/users") == "api"
assert matcher.evaluate(method="GET", path="/other") is None
```

Documentation: https://mox-nexus.github.io/x.uma/

License: MIT OR Apache-2.0
