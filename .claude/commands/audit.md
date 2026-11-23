---
name: audit
description: "Review the current branch against x.uma's protocol obligations with the constraint-auditor agent"
---

# /audit

Launch the `constraint-auditor` agent on the current branch's changes against `main`
(`git diff main...HEAD`). If the user named files or a pull request, audit those instead.

Relay its report as it gives it: each obligation the change touches, marked PASS, FAIL or NOT
CHECKED, with the evidence for each. Do not upgrade a NOT CHECKED to PASS.
