---
name: check-breaking
description: "Check the current branch for breaking changes to x.uma's schema and public surfaces"
---

# /check-breaking

Nothing is published yet, so a breaking change is allowed. But every one must be deliberate and
recorded in `DECISIONS.md`, because each will freeze at first publish.

1. **Schema:** run `just breaking` (`buf breaking` against `main`). Report each finding.
2. **Rust public API:** if `cargo semver-checks` or `cargo public-api` is installed, run it on
   the five published crates (`rumi-core`, `rumi-proto`, `rumi-kv`, `rumi-http`, `rumi-cli`).
   If neither is installed, say so. Then review the diff for changed or removed `pub` items,
   trait signatures, enum variants and error types.
3. **Binding surfaces:** review the diff in `rumi/crusts/python/` and `rumi/crusts/wasm/` for
   changed classes, methods, result fields or exception types.
4. **CLI:** review the diff in `rumi/cli/` for changed subcommands, flags, output or exit
   codes.
5. **Type URLs and config:** review the diff in `proto/xuma/` for renamed messages or changed
   field meanings.

For each change found, classify it (addition, removal, modification) and say whether a D-entry
covers it. A breaking change with no D-entry is the main thing to report.
