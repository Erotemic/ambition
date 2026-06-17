# Monolith breakup — COMPLETE (the structural runs landed)

The monolith-breakup effort is done. `ambition_sandbox` is a reusable
2D-platformer **engine** with named content in `ambition_content`; the layering
(machinery ← content ← app, plus extracted `ambition_render`, `ambition_time`,
`ambition_portal*`, `ambition_combat`, `ambition_actor`, `ambition_menu`,
`ambition_platformer_runtime`, …) is enforced by the
`architecture_boundaries` guard test.

**Oracle (still the design test):** could a different platformer be built by
ADDING a content crate without editing core? Jon's four goals the work served:
(1) incremental compile time, (2) agent-navigability, (3) idiomatic Bevy plugins,
(4) audit-grade reuse.

**What's left is no longer "extract a crate."** The clean crate-leaf frontier is
exhausted; further structure is internal splits + content promotion, tracked
live in `refactor-candidates.md` and `tech-debt-log.md`. The detailed scoping and
per-phase run-logs were pruned (outcome is in the crate graph + git history).
