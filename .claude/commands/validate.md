---
name: validate
description: "Run x.uma's check suite, covering what CI runs, and report the result"
---

# /validate

From the repository root, run:

```bash
set -o pipefail; just ci 2>&1 | tail -60
```

`just ci` runs everything CI runs: the engine's suites, both bindings' suites, the codegen
drift check, the docs checks and the audit.

`pipefail` matters. Without it, the pipe's exit status is `tail`'s, and a failing suite reports
success.

Report pass or fail. On failure, name the first failing recipe and quote its
error. Do not summarise a failure as a pass with warnings. If a tool is missing, run
`just doctor` and report what it says.
