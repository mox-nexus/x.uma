---
name: validate
description: "Run x.uma's check suite, covering what CI runs, and report the result"
---

# /validate

From the repository root, run:

```bash
set -o pipefail; just ci 2>&1 | tail -60
set -o pipefail; just crust-check 2>&1 | tail -40
just gen && git diff --exit-code -- rumi/proto/src/gen
```

- **`just ci`** covers most of CI.
- **`just crust-check`** runs the binding test suites, which CI runs as separate jobs.
- **The last line** is CI's codegen drift check: regenerating must change nothing.

`pipefail` matters. Without it, the pipe's exit status is `tail`'s, and a failing suite reports
success.

Report pass or fail for each line. On failure, name the first failing recipe and quote its
error. Do not summarise a failure as a pass with warnings. If a tool is missing, run
`just doctor` and report what it says.
