# Share one config across languages

You want the routing rules authored once and enforced identically by a Rust data
plane, a Python service, and a TypeScript worker.

## The config is the contract

A matcher config is data. Nothing in it is language-specific. The same file loads
in every implementation:

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
          name: read-handler
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: read-handler
onNoMatch:
  action:
    name: reject
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: reject
```

```rust
let matcher = registry.load_matcher(config)?;
matcher.evaluate(&ctx)
```

<!-- doc-sample: fragment -->
```python
matcher = registry.load_matcher(config)
matcher.evaluate(ctx)
```

<!-- doc-sample: fragment -->
```typescript
const matcher = registry.loadMatcher(config);
matcher.evaluate(ctx);
```

## What makes them agree

Agreement is not a promise in a README. It is a test suite. Every implementation
runs the same fixtures in `spec/tests/`, and a fixture is a config plus contexts
plus expected decisions. An implementation that disagrees fails its own build.

If you extend the engine, add a fixture first. That is the only way the guarantee
stays true for your extension.

## Keep the type URLs identical

The registry resolves inputs and matchers by type URL. A config referencing
`myapp.v1.RegionInput` loads only where that URL is registered. Register the same
URLs in every runtime, or the config becomes portable in name only.

A load failure here is the good outcome. The alternative is a silent difference
in behaviour between services.

## What is not shared

Actions are yours. The engine treats the action as an opaque value and hands it
back on match. Two runtimes agreeing that a request matched `read-handler` does
not make them agree on what `read-handler` does. That mapping lives in your code,
above the matcher.

This is the boundary the engine holds deliberately. It matches. It does not
decide policy.

## The trade-off

One shared config means one shared blast radius. A change lands everywhere at
once, and every consumer needs the inputs it references. Version the config
alongside the services that read it, and roll it like you roll code.

## Next

- [Add a custom input](custom-input.md) when a runtime needs to read something new.
- [Debug why something matched](debug-a-match.md) when two runtimes disagree, which should mean a missing fixture.
