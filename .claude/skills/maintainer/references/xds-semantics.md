# xDS Matcher Semantics

The canonical behavior defined by the xDS specification. x.uma must match these semantics exactly.

## Core Types

### Matcher

The top-level container. Contains either a `MatcherList` or `MatcherTree`.

```protobuf
message Matcher {
  oneof matcher_type {
    MatcherList matcher_list = 1;
    MatcherTree matcher_tree = 2;
  }
  OnMatch on_no_match = 3;
}
```

### MatcherList (First-Match-Wins)

Evaluates predicates in order. First match wins.

```protobuf
message MatcherList {
  repeated FieldMatcher matchers = 1;
}
```

**Semantics:**
1. Iterate through matchers in order
2. For each matcher, evaluate its predicate
3. If predicate is true → return on_match result
4. If no match → return on_no_match (or UnknownMatch if none)

### MatcherTree (Map-Based)

Uses a DataInput to extract a key, then looks up in a map.

```protobuf
message MatcherTree {
  TypedExtensionConfig input = 1;
  oneof tree_type {
    ExactMatchMap exact_match_map = 2;
    PrefixMatchMap prefix_match_map = 3;
  }
}
```

**Exact Match:** O(1) hash lookup.
**Prefix Match:** Longest-prefix-wins using radix tree.

### OnMatch

What to do when a match succeeds.

```protobuf
message OnMatch {
  oneof on_match {
    Matcher matcher = 1;  // Nested evaluation
    Action action = 2;    // Terminal action
  }
}
```

**Exclusive:** Either nested matcher OR action, never both.

### Predicate

Boolean combinations of matchers.

```protobuf
message Predicate {
  oneof match_type {
    SinglePredicate single_predicate = 1;
    PredicateList or_matcher = 2;
    PredicateList and_matcher = 3;
    Predicate not_matcher = 4;
  }
}
```

**Evaluation:**
- `SinglePredicate`: Evaluate the input matcher
- `OrMatcher`: Short-circuit OR (first true wins)
- `AndMatcher`: Short-circuit AND (first false loses)
- `NotMatcher`: Logical negation

---

## Data Availability

Three-state model for streaming matching:

| State | Meaning | Action |
|-------|---------|--------|
| `NotAvailable` | Data not present | Return false immediately |
| `MoreDataMightBeAvailable` | Data incomplete | Defer decision |
| `AllDataAvailable` | Data complete | Make final decision |

**Example:** HTTP header matching before all headers received.

---

## String Matching

```protobuf
message StringMatcher {
  oneof match_pattern {
    string exact = 1;
    string prefix = 2;
    string suffix = 3;
    string contains = 6;
    RegexMatcher safe_regex = 5;
  }
  bool ignore_case = 7;
}
```

| Pattern | Behavior |
|---------|----------|
| `exact` | Byte-for-byte equality |
| `prefix` | Starts with |
| `suffix` | Ends with |
| `contains` | Substring search |
| `safe_regex` | RE2 regex match |

**ignore_case:** Case-insensitive comparison (ASCII only for performance).

---

## Key Semantic Rules

### 1. First-Match-Wins

In `MatcherList`, order matters. First matching predicate determines result.

```yaml
matchers:
  - predicate: { starts_with: "/api/" }
    on_match: { action: "api_route" }
  - predicate: { starts_with: "/" }
    on_match: { action: "default_route" }
```

`/api/users` → `api_route` (not `default_route`)

### 2. Longest-Prefix-Wins

In `PrefixMatchMap`, longest matching prefix wins.

```yaml
prefix_map:
  "/": { action: "root" }
  "/api": { action: "api" }
  "/api/v2": { action: "api_v2" }
```

`/api/v2/users` → `api_v2`

### 3. Short-Circuit Evaluation

- `AND`: Stops on first `false`
- `OR`: Stops on first `true`
- Side effects in later predicates may not execute

### 4. on_no_match Cascading

If no predicate matches:
1. Check `on_no_match`
2. If `on_no_match` has nested matcher → evaluate it
3. If no `on_no_match` → return `UnknownMatch`

---

## Type URLs

All extension types use googleapis.com prefix:

```
type.googleapis.com/xuma.http.v1.HeaderInput
type.googleapis.com/xuma.test.v1.StringInput
type.googleapis.com/xds.type.matcher.v3.StringMatcher
```

The registry resolves these at config load time, not runtime.

---

## Conformance Requirements

x.uma implementations must:

1. **Pass all xDS conformance tests** in spec/tests/
2. **Match Envoy's behavior** for edge cases
3. **Use RE2 semantics** for regex (linear time, limited features)
4. **Fail closed** on unknown types or missing data
5. **Preserve order** in MatcherList evaluation
