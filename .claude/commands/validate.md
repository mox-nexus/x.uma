---
name: validate
description: "Run full validation suite for x.uma"
---

# /validate

Run the complete validation suite for x.uma.

## Execution

Run these validations in sequence:

1. **Proto validation:**
   ```bash
   cd ~/mox/x.uma && buf lint proto/ && buf breaking proto/ --against .git#branch=main
   ```

2. **Rust validation:**
   ```bash
   cd ~/mox/x.uma && cargo fmt --manifest-path rumi/Cargo.toml --all -- --check
   cd ~/mox/x.uma && cargo clippy --manifest-path rumi/Cargo.toml --workspace -- -W clippy::pedantic -D warnings
   cd ~/mox/x.uma && cargo test --manifest-path rumi/Cargo.toml --workspace
   cd ~/mox/x.uma && cargo build --manifest-path rumi/Cargo.toml -p rumi-core --no-default-features --features alloc
   ```

3. **Constraint check:**
   ```bash
   cd ~/mox/x.uma && .claude/plugins/x.uma-maintainer/scripts/check-constraints.sh
   ```

Report results to the user with pass/fail for each step.
