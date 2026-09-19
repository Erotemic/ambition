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

| ID | Status | Area | Current defect | Smallest sound direction |
| --- | --- | --- | --- | --- |
| W001 | TRACKED | actor resources | Every body still carries `BodyMana`, and Smash still uses it as Limit. | Do not create a second task here. `docs/planning/engine/composable-actor-resources.md` owns removal of universal Mana and separation of Mana from Limit. |
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
| W012 | CONFIRMED | body lifecycle | `BodyLifetime` mixes diagnostic counters (`time_alive`, `resets`, `max_speed`) with the real replay latch `restart_pending`. Its comment says the latch lives there because the component was already snapshotted. | Split the replay latch from diagnostics. Then decide independently which counters need rollback semantics. |
| W013 | CONFIRMED | boss presentation | `BossPatternTimer` is documented as a presentation-side mirror of the boss runtime timer. A sync system writes it and rollback registers it, but production code has no reader. | Delete the mirror if no consumer remains. If presentation needs it later, derive it through the view layer instead of making it gameplay rollback state. |
| W014 | CONFIRMED | actor AI state | `ActorStatus::ai_mode` is written by enemy integration and provocation and is rollback encoded, but production code has no read of the field. The same component also owns the respawn countdown. | Delete the unused projection or give it a real owner. Keep respawn lifecycle separate from an AI read-model. |
| W015 | CONFIRMED | respawn policy | Encounter and boss-spawn roads encode "never auto-respawn" as `respawn_timer = 999_999.0` even though `RespawnPolicy` already exists elsewhere. | Represent the lifecycle policy explicitly. Do not encode policy as a very large timer. |
| W016 | STRUCTURAL | actor identity | `ActorConfig` owns `id`, `name`, and `sprite_override_npc_name`. `ActorIdentity` owns the same facts. `sync_actor_components_from_cluster` copies config into identity when they differ. Both components are rollback registered. | Choose one authority. Make the other representation derived and non-authoritative, or remove the duplicate fields. |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` contains reusable body facts, controller-policy projections, placement/session policy, presentation facts, and mutable runtime state. Its exhaustive test explicitly classifies these different authority groups. `ActorConfig` rolls the whole projection back. | Split by owner when a real consumer boundary exists. Do not add more unrelated fields to this bag. |
| W018 | CONFIRMED | combat tuning | `CombatTuning::attack_cooldown_mult` is populated from `BrainProfile::attack_cooldown_mult`, but production combat has no read of the projection. The brain profile remains the live policy source. | Remove the duplicate field or move the consumer to one explicit owner. |
| W019 | CONFIRMED | actor pose | `ActorPose::feet` is derived from center and half-size at construction. Production code has no `.feet` read, but snapshot code stores it. | Remove the unused field and snapshot bytes unless a real consumer is introduced. |
| W020 | STRUCTURAL | simulation time | `SimDt` is explicitly a mirror of `WorldTime::sim_dt()`. The host copies the value every frame, and both `WorldTime` and `SimDt` are rollback resources. | Prefer one canonical mechanical clock fact. Move the neutral time primitive to a dependency-safe owner or make the mirror derived rather than a second rollback authority. |
| W021 | CONFIRMED | encounter music | `EncounterMusicRequest::last_applied` is written by the music-intent adapter for diagnostics/tests. Its accessor has no production caller, but the field lives inside gameplay encounter state. | Delete the field if transition detection does not need it, or move adapter history to presentation/audio state. |
| W022 | STRUCTURAL | combat component ownership | `BodyMelee::ranged_cooldown` is the live ranged fire-rate floor. It is actively used by ranged acceptance and prompts even though the owner type is `BodyMelee`. | Move ranged cooldown state to a weapon/ranged/action owner without changing the one-body fire-rate invariant. |
| W023 | STRUCTURAL | combat/presentation boundary | `BodyCombat::hit_flash` is a visual flash timer, but gameplay and AI use it as a recent-hit signal for bark suppression and hostility/behavior gates. | Introduce a semantic recent-hit/reaction fact if gameplay needs one. Keep visual flash lifetime as presentation state. |
| W024 | STRUCTURAL | projectile intent | `ActorFireRequest::speed` is still a live input to projectile spawn, while its own TODO says speed is redundant with resolved `RangedActionSpec`. `dir_to_world` also accepts unresolved `ScreenSpace`, logs that the result is wrong under rotated gravity, and then uses the screen vector as world space. | Make one speed authority. Make unresolved screen-space direction invalid at the gameplay seam instead of continuing with a known-wrong fallback. |
| W025 | STRUCTURAL | combat capability fallback | `default_fighting_kit()` gives a generic melee kit to bodies whose character authored no repertoire. Peaceful NPC construction and dismounted-rider logic can therefore gain an attack because content omitted one. The source says this default belongs to session/ruleset policy and has a `TODO(compat-remove)`. | Require an explicit character or ruleset fighting kit. Absence of authored combat should not silently invent a capability. |
| W026 | STRUCTURAL | provocation policy | `default_provoked_policy()` supplies an engine-default hostile brain when a provoked actor has no explicit policy. The source already says this is a ruleset-level answer. | Move the choice to explicit ruleset/content policy, or make the default a documented product rule with one owner. |
| W027 | STRUCTURAL | ability defaults | `ActorBody::new()` says it creates a locomotion-only mask with no capability verbs, but `locomotion_abilities()` enables double jump and inherits `interact` from `AbilitySet::basic()`. `ActorBody::from_kit()` also forces `attack = true` for every combat body. | Make the base capability contract match its name. Author or ruleset-select verbs that are not universal locomotion. |
| W028 | STRUCTURAL | body shape | `AncillaryMovementBundle` and the central actor query make `BodyMana`, `BodyOffense`, `BodyLifetime`, and `BodyComboTrace` part of what structurally counts as a complete body. Mana has an active migration plan; the other passengers do not. | Keep hot movement state dense where that is useful, but remove non-movement and diagnostic passengers from the mandatory body shape. |

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

### W016 and W017 — actor projections have become authority containers

`ActorIdentity` describes itself as an actor-facing read-model. The actor update
road compares it to `ActorConfig` and clones `id`, `name`, and sprite override
when they differ. The rollback registry includes both `actor.identity` and
`actor.config`.

`ActorTuning` is broader. Its exhaustive test groups fields into reusable
character facts, controller policy, placement/session facts, presentation facts,
and mutable runtime state. That test is useful as a census, but it also shows
that the type does not have one semantic owner.

### W020 — dependency inversion created a second clock fact

`ambition_platformer2d_shared_tangle::time::SimDt` says it is a neutral mirror of
`WorldTime::sim_dt()`. `mirror_sim_dt_into_runtime` copies the value after world
time refresh. `ambition_time` registers `WorldTime` for rollback and
`ambition_platformer2d_shared_tangle` registers `SimDt` for rollback. The
inversion seam is useful; the open question is whether the mirror itself should
also be authoritative state.

### W023 — a presentation timer is also a gameplay signal

Damage systems arm `BodyCombat::hit_flash`, render/view code consumes it as a
visual flash, and gameplay code also checks it in boss banter and actor bark /
hostility paths. A visual duration therefore controls semantic behavior. A
future visual-timing adjustment can change AI/dialogue behavior without an
explicit gameplay policy change.

### W025-W027 — missing authoring can grant behavior

The current source has three related fallback shapes:

- `default_fighting_kit()` supplies melee when no repertoire was authored;
- `default_provoked_policy()` supplies a hostile brain when no explicit policy
  was selected;
- `ActorBody` grants double jump, interaction, and attack through defaults even
  where comments describe a locomotion-only or kit-derived capability set.

These can be valid product rules, but they should be ruleset/content decisions.
They should not be accidental consequences of missing data.

## Already-owned related work

Do not duplicate these campaigns in this index:

- Universal `BodyMana` and the Mana/Limit alias are owned by
  `docs/planning/engine/composable-actor-resources.md` and its queue row.
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
