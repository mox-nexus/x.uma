# Security — bumi

| Package | Regex engine | ReDoS safe |
|---------|-------------|------------|
| bumi | `re2js` (pure JS RE2 port) | Yes |
| xuma-crust | Rust `regex` crate (via WASM) | Yes |

## Regex

`RegexMatcher` uses `re2js`. RE2 rejects patterns that require backtracking at compile time — specifically, backreferences (`(a)\1`) and lookahead/lookbehind (`(?=...)`, `(?<=...)`). Invalid or non-RE2-compatible patterns throw `MatcherError`, not a raw `re2js` error.

## Depth limits

Matcher trees are validated at construction. Trees deeper than `MAX_DEPTH` (32 levels) throw `MatcherError`.

## Prototype safety

HTTP request parsing uses `Object.create(null)` for header and query parameter storage. This prevents prototype pollution from user-supplied keys like `__proto__` or `constructor`.
