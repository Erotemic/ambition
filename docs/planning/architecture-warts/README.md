# Local architecture wart index

Verified against `99fff8d9a85dd607367bb1f099ded313c12a36b8` on 2026-09-19.

## Purpose

This page records small and medium architecture defects that are easy to miss
when review starts from the named architecture campaigns. These defects are not
a second execution queue. They are a re-measurement index for local semantic
debt.

Before implementation, re-check the row against the current tree. If the work is
selected, move the executable task to `docs/planning/queue.md` and put durable
design in the correct owner page. Remove the row here when another live planning
page owns it or when the defect is fixed.

This index focuses on four failure shapes:

1. authored controls that the runtime does not consume;
2. derived, diagnostic, or presentation state promoted to rollback authority;
3. fields or components owned by the wrong subsystem;
4. fallback policy that silently invents capabilities or behavior.

`CONFIRMED` means the source at the verified commit directly shows the stated
shape. `STRUCTURAL` means the source shows the ownership problem, but the final
repair can depend on intended product behavior. `TRACKED` means another planning
page already owns the repair; the row is present only to connect this inventory
to the motivating example.

## Index

Closed rows are removed; Git history and the
[AUTHORITY-POLISH](../queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel)
row record them. Open rows that AUTHORITY-POLISH owns carry their AP number
there. After the W003 deletion, only `extra_phase_triggers` can enter
`BossEncounterPhase::Stagger`, and no shipped boss authors it.

| ID | Status | Area | Current defect | Smallest sound direction |
| --- | --- | --- | --- | --- |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` contains reusable body facts, controller-policy projections, placement/session policy, presentation facts, and mutable runtime state. Its exhaustive test explicitly classifies these different authority groups. `ActorConfig` rolls the whole projection back. ⭐ Re-measured 2026-09-24: the "mutable runtime state" has narrowed to ONE field. After AP22 moved the Mary-O shell's contact threat out, the only runtime writer of `ActorConfig` is `brain_profile`, at provocation (`provoke.rs`) and brain command (`brain_command.rs`). Those are real transitions, and it is read every tick (`turns_at_walls`) and by brain rebuilds. Every `tuning` field is written only at construction (the `config.tuning.*` writes in `autonomous_reconcile.rs` are its test module). | Split by owner when a real consumer boundary exists. Do not add more unrelated fields to this bag. The runtime half, if it is split, is `brain_profile` alone. |

## Detailed evidence notes

### W002 — boss self-dodge does not move the boss

`crates/ambition_characters/src/brain/boss_pattern/mod.rs` says
`self_dodge_amp` and `self_dodge_freq` drive a horizontal oscillator. Shipped
`game/ambition_content/assets/data/boss_profiles.ron` gives GNU-ton
`self_dodge: Some((70.0, 1.6))`. In
`crates/ambition_boss_encounter/src/pattern/tick.rs`, the active branch only
executes `let _ = state.movement_timer;`. It does not use the frequency and does
not change the target or body motion.

### W017 — actor tuning has become an authority container

`ActorTuning` is broad. Its exhaustive test groups fields into reusable
character facts, controller policy, placement/session facts, presentation facts,
and mutable runtime state. That test is useful as a census, but it also shows
that the type does not have one semantic owner.

## Already-owned related work

Do not duplicate these campaigns in this index:

- Actor resources (the former universal `BodyMana` and its Mana/Limit alias)
  are owned by `docs/planning/engine/composable-actor-resources.md`.
- Known rollback-presence and deterministic-ingress defects remain in their
  existing owner documents and decision rows. This page is intentionally about
  local semantic debt that those indexes do not cover.
- Derived state such as `Dormant` already appears in active rollback planning.
  It is not repeated here even though it has the same "derived projection became
  rollback state" smell as several rows above.

## Re-measurement rule

Do not implement a row from this file by trusting the 2026-09-19 measurement.
Search the current source for the named field and its readers first. For an
"unused" claim, inspect both direct field reads and adapters that translate the
containing type. For an authoring claim, inspect shipped content as well as Rust.
For rollback-state cleanup, verify the real restore and checksum contract before
deleting a registration.
