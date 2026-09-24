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

Closed by [AUTHORITY-POLISH](../queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel) on 2026-09-23 and removed here: W013, W014, W015, W018, W020, W024; W012 on 2026-09-24. The remaining rows it owns carry their AP number there.

| ID | Status | Area | Current defect | Smallest sound direction |
| --- | --- | --- | --- | --- |
| W002 | CONFIRMED | boss behavior | `BossPatternCfg::self_dodge_amp` and `self_dodge_freq` are authored for GNU-ton, but the active self-dodge branch in `ambition_boss_encounter::pattern::tick` only reads `movement_timer` and applies no movement. | Implement the advertised dodge or delete the authoring fields and content value. Add a behavior witness. |
| W003 | CONFIRMED | boss encounter schema | `transition_to_phase2_hp`, `stagger_seconds`, `stagger_threshold`, and `stagger_window_seconds` are authored in all shipped boss encounter RON files, but production Rust has no consumer for the four values. | Implement the encounter semantics or delete the fields from the schema and shipped data. |
| W004 | CONFIRMED | melee authoring | `LungeSpec::step_px` and `SlamSpec::hop_height_px` are authored fields, but `MeleeActionSpec::timeline` explicitly drops them when it builds the current moveset path. | Carry the self-motion data into the move runtime or delete the unsupported controls. |
| W005 | CONFIRMED | interaction | `ActorInteraction::talk_radius` is populated by spawn roads and described as the range that stops a patrol for dialogue, but production code has no field read. | Either make interaction or patrol logic consume the range, or delete it and state the real geometry rule. |
| W006 | CONFIRMED | Smash AI | `SmashCfg::aerial_foray_cadence_s`, `aerial_foray_duration_s`, and `SmashState::foray_timer` describe proactive flight behavior, but production code does not read the two settings or operate the timer. | Implement the advertised hybrid-flyer cadence or remove the dormant API/state. |
| W007 | CONFIRMED | Smash AI | `DifficultyProfile::mash_speed_hz` is authored and documented as a downstream cooldown input, but no production consumer exists. | Wire it to an explicit mechanic or delete it from difficulty and character authoring. |
| W008 | CONFIRMED | Smash observation | `CrowdingSignal` supports `other_faction_count`, and `compute_pressure` has behavior for it, but the shipped crowd producer always writes `other_faction_count: 0`. | Either observe other factions or delete the unreachable branch and field. |
| W009 | CONFIRMED | combat damage | `BodyOffense::damage_multiplier` is mandatory body state and rollback state. Outside developer editing, gameplay does not read the value. Hitbox completeness still uses `With<BodyOffense>` as part of the victim shape. | Identify the real damage authority, remove the dead value if it has no owner, and use an explicit damageable/victim capability instead of offense presence. |
| W010 | CONFIRMED | combat attack spec | `AttackSpec::damage_kind`, `can_pogo`, and `damage_override` are authored, copied, tested, and snapshotted, but production gameplay does not read them. The `damage_override` comment still promises a fallback to `BodyOffense::damage_multiplier`. | Restore the intended semantics or delete the dead fields and their snapshot surface. |
| W011 | CONFIRMED | movement diagnostics | `BodyComboTrace` exists so the HUD combo trace does not blank. It is mandatory on the central body shape, rollback canonical, and threaded through movement, ledge, dodge, blink, knockdown, and other kernel APIs. The visible production reader is the HUD. | Move diagnostic history out of the core body contract. Keep gameplay events or semantic state separate from the display trace. |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` contains reusable body facts, controller-policy projections, placement/session policy, presentation facts, and mutable runtime state. Its exhaustive test explicitly classifies these different authority groups. `ActorConfig` rolls the whole projection back. | Split by owner when a real consumer boundary exists. Do not add more unrelated fields to this bag. |
| W019 | CONFIRMED | actor pose | `ActorPose::feet` is derived from center and half-size at construction. Production code has no `.feet` read, but snapshot code stores it. | Remove the unused field and snapshot bytes unless a real consumer is introduced. |
| W021 | CONFIRMED | encounter music | `EncounterMusicRequest::last_applied` is written by the music-intent adapter for diagnostics/tests. Its accessor has no production caller, but the field lives inside gameplay encounter state. | Delete the field if transition detection does not need it, or move adapter history to presentation/audio state. |
| W022 | STRUCTURAL | combat component ownership | `BodyMelee::ranged_cooldown` is the live ranged fire-rate floor. It is actively used by ranged acceptance and prompts even though the owner type is `BodyMelee`. | Move ranged cooldown state to a weapon/ranged/action owner without changing the one-body fire-rate invariant. |
| W023 | STRUCTURAL | combat/presentation boundary | `BodyCombat::hit_flash` is a visual flash timer, but gameplay and AI use it as a recent-hit signal for bark suppression and hostility/behavior gates. | Introduce a semantic recent-hit/reaction fact if gameplay needs one. Keep visual flash lifetime as presentation state. |
| W026 | STRUCTURAL | provocation policy | `default_provoked_policy()` supplies an engine-default hostile brain when a provoked actor has no explicit policy. The source already says this is a ruleset-level answer. | Move the choice to explicit ruleset/content policy, or make the default a documented product rule with one owner. |
| W028 | STRUCTURAL | body shape | `AncillaryMovementBundle` and the central actor query make `BodyOffense` and `BodyComboTrace` part of what structurally counts as a complete body. (Mana left the shape 2026-09-23: it is a declared resource in the optional bank. The `BodyLifetime` diagnostics left it 2026-09-24; the restart latch that stays is a mechanical fact.) | Keep hot movement state dense where that is useful, but remove non-movement and diagnostic passengers from the mandatory body shape. |

## Detailed evidence notes

### W002 — boss self-dodge does not move the boss

`crates/ambition_characters/src/brain/boss_pattern/mod.rs` says
`self_dodge_amp` and `self_dodge_freq` drive a horizontal oscillator. Shipped
`game/ambition_content/assets/data/boss_profiles.ron` gives GNU-ton
`self_dodge: Some((70.0, 1.6))`. In
`crates/ambition_boss_encounter/src/pattern/tick.rs`, the active branch only
executes `let _ = state.movement_timer;`. It does not use the frequency and does
not change the target or body motion.

### W003 — boss encounter controls have no runtime customer

`BossEncounterSpec` declares the four fields and the shipped boss encounter RON
files provide values for them. At this verified commit, production Rust references
for these names are limited to schema/default/test construction. Phase trigger
construction uses `phase1_to_transition_hp`, `transition_seconds`, and
`phase2_to_enrage_hp`, not `transition_to_phase2_hp` or the stagger fields.

### W009 and W010 — the old offense/damage vocabulary is still in the type system

`BodyOffense` is in the shared body bundle and rollback registry. Production
combat uses its component presence in the hitbox query, but there is no gameplay
read of `offense.damage_multiplier`. `AttackSpec` still stores three fields that
look like the consumer side of the same older design, but current production
search finds no read of those fields either. Before deletion, re-check authored
item/move adapters so the cleanup does not remove a hidden translation seam.

### W011 — a HUD trace changes movement-kernel signatures

`BodyComboTrace` says its purpose is to preserve the symbolic operation trace for
the HUD. It is part of body scratch state, body ECS queries, rollback encoding,
and many movement function signatures. The production reader outside the write
path is `game/ambition_app/src/app/hud.rs`. This is more than an unused field: a
diagnostic feature changes the required shape of every body.

### W017 — actor tuning has become an authority container

`ActorTuning` is broad. Its exhaustive test groups fields into reusable
character facts, controller policy, placement/session facts, presentation facts,
and mutable runtime state. That test is useful as a census, but it also shows
that the type does not have one semantic owner.

### W023 — a presentation timer is also a gameplay signal

Damage systems arm `BodyCombat::hit_flash`, render/view code consumes it as a
visual flash, and gameplay code also checks it in boss banter and actor bark /
hostility paths. A visual duration therefore controls semantic behavior. A
future visual-timing adjustment can change AI/dialogue behavior without an
explicit gameplay policy change.

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
