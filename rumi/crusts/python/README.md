# xuma-crust

Rust-backed bindings for the [x.uma](https://github.com/mox-nexus/x.uma) matcher
engine, built with PyO3.

Same API surface as the pure-Python `xuma` package, and the same conformance
suite. Reach for this when evaluation is hot enough that the Rust engine pays
for the native dependency.

```bash
uv add xuma-crust
```

```python
from xuma_crust import load_http_matcher

matcher = load_http_matcher(open("routes.yaml").read())
matcher.evaluate(method="GET", path="/api/users")
```

Documentation: https://mox-nexus.github.io/x.uma/

License: MIT OR Apache-2.0
