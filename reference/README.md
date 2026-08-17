# reference/

Evidence `PLAN.md` and `DECISIONS.md` cite. **Tracked on purpose.**

`scratch/` is gitignored (`.gitignore:63`), which is correct for working notes
and wrong for anything a plan tells a reader to go and read. A plan that claims
to be self-contained cannot depend on files absent from a fresh clone — and it
did, twice, until 2026-08-17.

| File | What it is |
|---|---|
| `security-review-2026-08-16.md` | Pre-publication security review. Produced by a `ci-scaffolds:mudge` subagent against commit `e90429e`; never written to disk, recovered from the session transcript. Phase S's falsifying tests are in it. Contains at least one error — see its header. |
| `prior-art-2025-design.md` | Design conclusions from 2025-04 to 2025-11 conversations, recovered from memex 2026-08-14, so Phase 12 work does not rediscover them. Raw hits stay in `scratch/phase-12/memex-raw.json`, which is bulk and stays untracked. |

Read these as **evidence, not truth**, the same way `PLAN.md` §4 asks you to read
itself. Each was accurate when written and is dated so you can tell how far it
has drifted.
