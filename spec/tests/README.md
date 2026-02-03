# Conformance Test Fixtures

YAML fixtures that all x.uma implementations must pass.

## Fixture Format

```yaml
# Required: unique fixture name
name: "exact_string_match"

# Required: what this fixture tests
description: "Tests exact string equality matching"

# Required: the matcher configuration
matcher:
  # List of field matchers (predicate + on_match pairs)
  matchers:
    - predicate:
        # Single predicate: input + value_match
        single:
          input: { key: "field_name" }
          value_match: { exact: "expected_value" }
      on_match:
        action: "action_name"

  # Optional: fallback when no matcher matches
  on_no_match:
    action: "default_action"

# Required: test cases to run against the matcher
cases:
  - name: "case_name"
    # Context: key-value pairs for TestContext
    context:
      field_name: "some_value"
    # Expected result: action name or null
    expect: "action_name"
```

## Predicate Types

### Single Predicate
```yaml
predicate:
  single:
    input: { key: "field_name" }
    value_match:
      exact: "value"      # or
      prefix: "val"       # or
      suffix: "ue"        # or
      contains: "alu"
```

### AND Predicate
```yaml
predicate:
  and:
    - single: { input: { key: "a" }, value_match: { exact: "1" } }
    - single: { input: { key: "b" }, value_match: { exact: "2" } }
```

### OR Predicate
```yaml
predicate:
  or:
    - single: { input: { key: "a" }, value_match: { exact: "1" } }
    - single: { input: { key: "a" }, value_match: { exact: "2" } }
```

### NOT Predicate
```yaml
predicate:
  not:
    single: { input: { key: "a" }, value_match: { exact: "blocked" } }
```

## OnMatch Types

### Action
```yaml
on_match:
  action: "action_name"
```

### Nested Matcher
```yaml
on_match:
  matcher:
    matchers:
      - predicate: ...
        on_match: { action: "nested_action" }
```

## Test Case Format

```yaml
cases:
  - name: "descriptive_name"
    context:
      key1: "value1"
      key2: "value2"
    expect: "expected_action"  # or null for no match
```

## File Organization

```
spec/tests/
├── README.md           # This file
├── 01_string_matchers/ # Basic string matching
│   ├── exact.yaml
│   ├── prefix.yaml
│   ├── suffix.yaml
│   └── contains.yaml
├── 02_predicates/      # Boolean logic
│   ├── and.yaml
│   ├── or.yaml
│   └── not.yaml
├── 03_semantics/       # Matcher behavior
│   ├── first_match_wins.yaml
│   ├── on_no_match.yaml
│   └── nested_matcher.yaml
└── 04_invariants/      # Critical invariants
    ├── none_returns_false.yaml
    └── empty_and_or.yaml
```
