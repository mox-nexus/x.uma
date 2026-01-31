---
name: check-breaking
description: "Check for breaking changes in proto and Rust APIs"
---

# /check-breaking

Check for breaking changes against the main branch.

## Execution

1. **Proto breaking changes:**
   ```bash
   cd ~/mox/x.uma && buf breaking proto/ --against .git#branch=main
   ```

2. **Rust API changes:**
   - Use `cargo semver-checks` if available
   - Otherwise, review public API surface manually:
     - Check `pub` items in rumi-core/src/lib.rs
     - Check trait definitions
     - Check struct fields

3. **Report findings:**
   - List any breaking changes detected
   - Classify as: Addition, Removal, Modification
   - Recommend versioning action (major/minor/patch)
