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
| W002 | CONFIRMED | boss behavior | `BossPatternCfg::self_dodge_amp` and `self_dodge_freq` are authored for GNU-ton, but the active self-dodge branch in `ambition_boss_encounter::pattern::tick` only reads `movement_timer` and applies no movement. | Implement the advertised dodge or delete the authoring fields and content value. Add a behavior witness. |
| W004 | CONFIRMED | melee authoring | `LungeSpec::step_px` and `SlamSpec::hop_height_px` are authored fields, but `MeleeActionSpec::timeline` explicitly drops them when it builds the current moveset path. | Carry the self-motion data into the move runtime or delete the unsupported controls. |
| W005 | CONFIRMED | interaction | `ActorInteraction::talk_radius` is populated by spawn roads and described as the range that stops a patrol for dialogue, but production code has no field read. | Either make interaction or patrol logic consume the range, or delete it and state the real geometry rule. |
| W008 | CONFIRMED | Smash observation | `CrowdingSignal` supports `other_faction_count`, and `compute_pressure` has behavior for it, but the shipped crowd producer always writes `other_faction_count: 0`. | Either observe other factions or delete the unreachable branch and field. |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` contains reusable body facts, controller-policy projections, placement/session policy, presentation facts, and mutable runtime state. Its exhaustive test explicitly classifies these different authority groups. `ActorConfig` rolls the whole projection back. | Split by owner when a real consumer boundary exists. Do not add more unrelated fields to this bag. |
| W026 | STRUCTURAL | provocation policy | `default_provoked_policy()` supplies an engine-default hostile brain when a provoked actor has no explicit policy. The source already says this is a ruleset-level answer. | Move the choice to explicit ruleset/content policy, or make the default a documented product rule with one owner. |

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

### W026 — missing authoring can grant behavior

`default_provoked_policy()` supplies a hostile brain when no explicit policy was
selected. That can be a valid product rule, but it should be a ruleset/content
decision rather than an accidental consequence of missing data.

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
