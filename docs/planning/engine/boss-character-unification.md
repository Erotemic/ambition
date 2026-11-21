# Boss characters: one identity and one construction path

**State:** ready for handoff. Scope set by Jon on 2026-11-20.
**Queue:** [BOSS-CHARACTER](../queue.md#boss-character--one-character-path-for-bosses).
**Related:** [boss system](boss-system.md),
[one body, one path](../../concepts/one-body-one-path.md),
[actor content](../../systems/actors-brains-and-character-content.md).

## Maintainer intent

A boss is an instance of a character, with extra attributes composed onto that
instance. These attributes can request a title card, a health bar, phase rules,
and a boss controller. A Boss Encounter is a separate composition. It can hold
several boss character instances. GNU-ton and the Gnu are the concrete example.
The Gnu has no independent health policy selected by this task; do not give it
one as a side effect of the migration.

Keep ordinary characters small. Do not put optional boss state on every
character, enlarge every actor bundle, or add a boss test to each ordinary
actor's tick. This work must reduce runtime complexity and runtime cost.
Measure the result; do not claim a speed gain from fewer source lines.

Jon authorized an inspector bridge now and a full unification by another agent.
The bridge is temporary presentation data. It does not resolve runtime identity.

## Reproducible defect

The Characters panel reads catalog identities. Marshal Gallus has boss content
and art but no catalog character row. Smirking Behemoth has both
`npc_smirking_behemoth_boss` in the character catalog and
`smirking_behemoth_boss` in the boss profiles. The Hall row states a separate
body height and peaceful action set. The fight constructs a separate body.
Shared art does not join these identities. The copies can disagree.

## Source packet

Read these owners in the assigned checkout. Recheck symbols before editing.

| Owner | Current responsibility or split |
| --- | --- |
| `crates/ambition_characters/src/prepared.rs` | Common prepared character and its registry |
| `crates/ambition_characters/src/actor/definition.rs` | Authored character composition |
| `crates/ambition_body_seed/src/lib.rs` | Common actor body seed |
| `crates/ambition_platformer2d_actor_spawn/src/character_body.rs` | Prepared character grants |
| `crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs` | `boss_actor_cluster` and `spawn_boss_with_overrides_into` build a separate boss body |
| `crates/ambition_boss_encounter/src/clusters.rs` | `BossClusterScratch` builds kinematics, health, and sheet-derived geometry separately |
| `crates/ambition_boss_encounter/src/pattern/profile.rs` | Boss behavior also carries body and death facts |
| `crates/ambition_boss_encounter/src/systems.rs` | Encounter activation writes the body's health from the encounter seed |
| `crates/ambition_platformer2d_construction/src/lib.rs` | Separate character-body and boss construction dispatch |
| `crates/ambition_render/src/rendering/actors/boss.rs` | Boss animation and bridges into character parts |
| `crates/ambition_sprite_sheet/src/boss.rs` | Boss sheet and clip vocabulary |
| `crates/ambition_content_cli/src/art.rs` | Art export enumerates only catalog character rows |
| `game/ambition_content/assets/data/` | Character catalog, boss profiles, boss sheets, art keys, encounters |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/generate_hall_of_characters.py` | Generated Hall from catalog entries |

At investigation, the pack has 15 boss behavior profiles. Recount; this number
is context, not an acceptance target. Different profiles need not mean different
characters. Different characters can also use the same art.

## Required result

One prepared character owns intrinsic identity, body geometry, art, locomotion,
abilities, moves, default vitals, and death traits. Both the Hall and a fight
consume that same value through common body construction.

A boss role is an optional composition on an instance. Keep its controller,
phase state, title-card request, and health-bar request in focused components
or capability data. Reuse existing owners where their semantics fit. Do not
make a large boss object the mandatory carrier of these independent features.
The title card and health bar observe their subject's ordinary identity and
health. They must not keep a second health pool or character identity.

An encounter owns the participating instance references, activation, arena
rules, music, rewards, and progress. One encounter can have several subjects.
Do not equate character id, behavior id, placement id, and encounter id. Preserve
placement and encounter keys used by saves, scripts, quests, and rewards.

Occurrence-specific overrides remain explicit. A Hall controller can be passive
without replacing the character's intrinsic action repertoire. Encounter tuning
can override health or scale for an occurrence without creating another
character definition. Establish an override order once, before construction;
do not build a provisional body and repair it in a later projection.

## Execution slices

1. **Establish the authoritative mapping and baseline.** Trace each authored boss
   from source through preparation, room construction, headless state, and
   rendering. Map each behavior to its character, forms, and encounter instances.
   Include shared sentinel art, GNU-ton's rider and mounts, and transformations.
   Resolve the Hall duplicates by semantics, not by names or shared images.
   Record existing body dimensions, hurt volumes, action repertoire, death
   policy, and encounter progress for the representative scenarios below.
   Measure ordinary-character component sizes and tick cost, then boss cost.
2. **Prepare and construct one character.** Fold intrinsic facts at their owner
   seam. Compose boss attributes and encounter membership beside the common
   body plan. Remove `boss_actor_cluster` and the independent body/health seed
   in `BossClusterScratch` when the common path replaces them. Keep useful
   encounter query views; remove their body-construction authority. Remove
   conflicting source facts rather than asserting that two copies agree.
   Complete this first for Gallus and Smirking Behemoth, then migrate every
   affected provider and fixture before declaring the slice complete.
3. **Consume the common identity everywhere.** Use it in title cards, health bars,
   actor observations, inspection, character selection, and the generated Hall.
   Use one geometry and art resolution for the same character and scale.
   Preserve authored boss attack clip names and their hit geometry. Do not
   merge animation enums merely because their names differ; remove duplicate
   binding and playback authorities where they exist. Delete obsolete adapters,
   provisional definitions, and the temporary inspector bridge.
4. **Verify behavior and cost, then hand off.** Compare the stable tree with the
   baseline. Preserve intentional fight mechanics. Explain deliberate changes
   and update replay baselines when needed. Report cost changes with the same
   scenarios and build settings. Investigate regressions; do not offset them
   with a Hall-only optimization.

Keep each integration slice coherent. A temporary migration seam must have a
short `TODO(compat-remove)` with its replacement and deletion condition. Do not
leave the old body builder as a fallback after the replacement works.

## Verification and acceptance

Use the real headless provider and construction paths. Add behavioral tests at
the owner of each important regression, not a source-text or migration matrix.

- Gallus appears through the ordinary prepared character export. A missing
  character reference is refused during preparation, before construction.
- Smirking Behemoth's Hall and fight instances have the same character identity
  and the same intrinsic geometry at equal scale. The Hall does not run the
  fight controller or claim encounter rewards. The rope kill still works.
- GNU-ton and the Gnu remain separate character instances with an explicit mount
  relation. Encounter membership does not add health or damageability to the
  Gnu. Keep the rider's possession, authored specials, and GNU2 portal sequence.
- Check Gallus's phases, a shared-art sentinel, the articulated Flying Spaghetti
  Monster, the T-Rex, and Wolfenstein's transformation through their actual
  encounter paths. Preserve pose-derived hurt geometry and death policy.
- Two occurrences of one character keep independent health, control, phase,
  encounter progress, and reward state. Rewind and restore retract only the
  affected occurrence's consequences.
- A character without boss attributes has no new boss components, per-tick
  lookup, allocation, or enlarged mandatory runtime record. Report component
  sizes, allocations where relevant, and comparable headless timing samples.
  Show which duplicate runtime work was removed and its measured cost. Ordinary
  actor performance must not regress; demonstrate a runtime gain on the affected
  paths without adding a permanent timing threshold to the test suite.
- The completed tree has one body construction authority. No duplicated Hall
  definition or inspector-generated boss identity is needed.

Before Cargo, inspect prescribed target storage. Run narrow owner tests first,
then `cargo check -p ambition_app`, affected `app_it` modules (construction,
`unified_body_movement`, `boss_possession_specials`, and
`boss_replay_retraction`), `cargo test -p ambition_demo_smash_app`, and
`cargo test -p ambition_workspace_policy`. Follow the verification recipe for
any broader gate the final scope requires. Use a visible capture for rendering
and scale after headless checks pass. Do not claim visual parity from a compile.

## Temporary inspector bridge: deletion contract

`tools/ambition_editor/asset_index/boss_roster.py` derives inspection-only entries
from exported boss art keys, boss references, and room placements. It keeps boss
entries separate from catalog entries, even when their art matches. The browser
labels the entries and links shared art. It states no common runtime identity,
no authored body capability, and no boss action coverage.

Delete this module, its assembly call, origin-specific UI, and origin-specific
date handling when the ordinary prepared character export covers bosses. Keep
the useful role filter, art links, date sorting, and normal review features on
the common records. Migrate review-note subjects deliberately if notes exist on
the temporary `provider:boss/id` keys. Remove migration-only tests with the
bridge; keep behavioral coverage of the resulting common export.
