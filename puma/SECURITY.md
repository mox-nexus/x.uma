# Security — puma

| Package | Regex engine | ReDoS safe |
|---------|-------------|------------|
| puma | `google-re2` | Yes |
| xuma-crust | Rust `regex` crate (via FFI) | Yes |

## Regex

`RegexMatcher` uses `google-re2`. RE2 rejects patterns that require backtracking at compile time — specifically, backreferences (`(a)\1`) and lookahead/lookbehind (`(?=...)`, `(?<=...)`). Invalid or non-RE2-compatible patterns raise `MatcherError`, not a raw `re2.error`.

## Depth limits

Matcher trees are validated at construction. Trees deeper than `MAX_DEPTH` (32 levels) raise `MatcherError`.
