# Arch-Guild Constraints

Mandatory invariants from the 13-agent architecture review. These are **non-negotiable** and must be enforced by hooks and pre-commit checks.

## The Constraints

| # | Constraint | Source | Rationale |
|---|-----------|--------|-----------|
| 1 | **ReDoS Protection** | Vector, Taleb | Use `regex` crate only, never `fancy-regex` |
| 2 | **Max 32 Depth** | Vector, Taleb | Prevent stack overflow from nested matchers |
| 3 | **Type Registry Immutable** | Vector, Lamport | Thread-safety: no runtime registration |
| 4 | **Send + Sync + Debug** | Lamport, Wolf | FFI requirement for all public types |
| 5 | **Iterative Evaluation** | Taleb, Dijkstra | Prevent recursion-induced stack overflow |
| 6 | **DataInput None → false** | Dijkstra | Missing data fails fast, doesn't panic |
| 7 | **OnMatch is Exclusive** | xDS spec, Dijkstra | Action XOR Matcher, not both |
| 8 | **Action: 'static** | Wolf | Lifetime simplicity for FFI |
| 9 | **Action: Clone + Send + Sync** | Wolf | Clone needed for first-match-wins |

---

## Enforcement Details

### 1. ReDoS Protection

**Check:** Grep for `fancy_regex` or unsafe regex patterns.

```bash
# Should find nothing
grep -r "fancy.regex" rumi/
grep -r "regex!" rumi/  # macro that might use fancy-regex
```

**Why:** Catastrophic backtracking in regex can cause DoS. The `regex` crate guarantees linear-time matching.

### 2. Max 32 Depth

**Check:** Validate `MAX_DEPTH` constant exists and is ≤ 32.

```rust
// In rumi-core/src/lib.rs or similar
pub const MAX_MATCHER_DEPTH: usize = 32;
```

**Why:** Unbounded nesting causes stack overflow. Envoy uses similar limits.

### 3. Type Registry Immutable

**Check:** Registry methods use `&self`, not `&mut self`.

```rust
// Good
impl Registry {
    pub fn get(&self, name: &str) -> Option<&TypeInfo> { ... }
}

// Bad - would allow runtime mutation
impl Registry {
    pub fn register(&mut self, name: &str, info: TypeInfo) { ... }
}
```

**Why:** Concurrent access without locks. Build registry at startup, never modify.

### 4. Send + Sync + Debug

**Check:** All public types have these trait bounds.

```rust
// Marker test in rumi-core
fn assert_send_sync_debug<T: Send + Sync + std::fmt::Debug>() {}

#[test]
fn public_types_are_ffi_safe() {
    assert_send_sync_debug::<Matcher<(), ()>>();
    assert_send_sync_debug::<Predicate<()>>();
    // ... all public types
}
```

**Why:** Required for FFI (Python, WASM). Without these, can't share across threads or print for debugging.

### 5. Iterative Evaluation

**Check:** No recursive `evaluate()` calls.

```bash
# Should find no recursion
grep -r "\.evaluate(" rumi/rumi-core/src/
# Review each hit - should be iterative with explicit stack
```

**Why:** Deep recursion causes stack overflow. Use explicit stack/queue for tree traversal.

### 6. DataInput None → false

**Check:** Conformance tests verify this behavior.

```yaml
# In spec/tests/data_input/none_returns_false.yaml
cases:
  - description: "missing data returns false, not panic"
    matcher: { ... }
    input: null
    expected: { matches: false }
```

**Why:** Fail-closed security. Missing data should never match.

### 7. OnMatch is Exclusive

**Check:** `OnMatch` is an enum, not a struct with optional fields.

```rust
// Good - type-level guarantee
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}

// Bad - runtime check required
pub struct OnMatch<Ctx, A> {
    action: Option<A>,
    matcher: Option<Box<Matcher<Ctx, A>>>,
}
```

**Why:** Type system enforces exclusivity. Can't have both action AND nested matcher.

### 8. Action: 'static

**Check:** Trait bounds include `'static`.

```rust
pub trait DataInput<Ctx> {
    type Output: 'static;  // Required
    fn get(&self, ctx: &Ctx) -> Option<Self::Output>;
}
```

**Why:** Simplifies lifetime management for FFI. No borrowed data escaping.

### 9. Action: Clone + Send + Sync

**Check:** `Matcher` has bounds on action type.

```rust
pub struct Matcher<Ctx, A>
where
    A: Clone + Send + Sync,  // Required
{ ... }
```

**Why:** First-match-wins needs to clone the action. Send/Sync for thread safety.

---

## Validation Script

```bash
#!/bin/bash
# check-constraints.sh

set -e

echo "Checking arch-guild constraints..."

# 1. ReDoS
if grep -rq "fancy.regex" rumi/; then
    echo "FAIL: fancy-regex detected"
    exit 1
fi

# 2. Max depth constant
if ! grep -q "MAX.*DEPTH.*=.*[0-9]" rumi/rumi-core/src/; then
    echo "FAIL: MAX_DEPTH constant not found"
    exit 1
fi

# 3-9: Run marker tests
cargo test --manifest-path rumi/Cargo.toml -p rumi-core ffi_safe

echo "All constraints validated!"
```
