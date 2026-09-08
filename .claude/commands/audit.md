---
name: audit
description: "Run constraint auditor agent to validate arch-guild constraints"
---

# /audit

Run the constraint-auditor agent to validate all arch-guild constraints.

## Execution

Launch the constraint-auditor agent to perform a comprehensive audit of the x.uma codebase:

1. Use the Task tool to launch the constraint-auditor agent
2. The agent will check all 9 arch-guild constraints
3. Report findings with PASS/WARN/FAIL for each constraint

## Constraints Checked

1. ReDoS Protection (no fancy-regex)
2. Max 32 Depth
3. Type Registry Immutable
4. Send + Sync + Debug
5. Iterative Evaluation
6. DataInput None → false
7. OnMatch is Exclusive
8. Action: 'static
9. Action: Clone + Send + Sync
