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

### Slice 1 record (2026-11-21)

**Identity mapping.** There are 15 behavior profiles and 28 shipped placements.
The probe built each placement with `BossClusterScratch::new` on the shipped
pack and rooms. Each placement is listed once per room.

| Behavior | Character (row) | Form of | Placements (room) | Notes |
| --- | --- | --- | --- | --- |
| `clockwork_warden` | none, needs `npc_clockwork_warden` | | `basement_boss`, `first_system_boss` ("System Boss"), `hall_warden_arena` | Sentinel art: `clockwork_warden` sheet |
| `mockingbird` | `npc_mockingbird_boss` | | `hall_mockingbird_sky`, `mockingbird_sky` | |
| `gnu_ton_rider` | `npc_gnu_ton_boss` | `npc_gnu_ton` | `gnu_ton_arena`, `gnu_ton_shoulders` | Already the encounter's `voice`. The row's manifest is the scholar and the giant as one picture, but the fight wears `gnu_ton_scholar_boss` (the fighter's drawing). Slice 3 makes these one art resolution. Rides `npc_giant_gnu` (a mount character with no boss profile) and conducts its fists (`npc_giant_gnu_hands`) |
| `gnu2` | `npc_giant_gnu2` | | `gnu_ton_arena`, `gnu_ton_shoulders` | Second encounter subject, called through a portal |
| `bossofthegame` | `npc_bossofthegame` | | `boss_main_stage`, `hall_bossofthegame_stage` | Rides a giant (`pilotable_mount_classes: ["giant"]`) |
| `smirking_behemoth_boss` | `npc_smirking_behemoth_boss` | | `you_have_to_cut_the_rope` | Hall duplicate; see below |
| `flying_spaghetti_monster_boss` | `npc_flying_spaghetti_monster_boss` | | `flying_spaghetti_monster_arena`, `noodly_sanctum` | Articulated (limb routing) |
| `trex_boss` | `npc_trex_enemy` | | `primeval_tyrant_hall`, `trex_arena` | |
| `mode_collapse_boss` | none, needs `npc_mode_collapse` | | `mode_collapse_arena`, `overfit_annex_boss` | Sentinel art: `hooded_sentinel` |
| `gram_reaper_boss` | none, needs `npc_gram_reaper` | | `hall_gram_reaper_arena`, `math_hotel_gram_reaper` | |
| `hunny_horror_boss` | `npc_hunny_horror_boss` | | `hunny_horror_arena` | |
| `overflow_boss` | none, needs `npc_overflow` | | `overflow_arena` | Wears the shared `gradient_sentinel` sheet; `npc_boss` ("Boss") wears the same art and is a different character |
| `wolfenstein_boss` | `npc_wolfenstein_boss` | | `castle_godwin_throne`, `hall_wolfenstein_arena` | |
| `wolfenstein_inugami` | `npc_wolfenstein_inugami` | `npc_wolfenstein_boss` | `castle_godwin_throne`, `hall_wolfenstein_arena` | Transformation: his head, risen; a second placement in the same room |
| `marshal_gallus` | none, needs `npc_marshal_gallus` | | `frontier_ore_corral`, `hall_marshal_gallus_corral` | |

Five behaviors have no character row: Clockwork Warden, Mode Collapse, Gram
Reaper, Overflow and Marshal Gallus. Their art is published
(`published.json` lists every sheet). Overflow is not `npc_boss`. They share a
drawing, and the drawing does not decide identity. `npc_boss` is the nameless
"Boss" whose own barks say it is still waiting for a name.

**Hall duplicates.** Eight Hall rows carry a `standing_height` that was copied
by hand from the fight body ("its size in its fight, its primary context",
2026-11-11). This is the duplicate the plan names: the profile's
`combat_size`, or the sheet body at the placement box, is a second copy of the
same fact. The copies already disagree. The Flying Spaghetti Monster's Hall
row says 79.1 and its fight body is 77.25 tall. For the Behemoth, the row's
`standing_height` (266) and the profile's `combat_size` (208 × 266) agree. The
Hall instance is the same character with a passive controller. Its
`peaceful` action set and `stand_still` brain are placement facts, not a
second identity.

**Body dimensions at construction** (width × height in world units):

| Behavior | Placement box | Collision body | Render basis | Encounter HP | Corpse |
| --- | --- | --- | --- | --- | --- |
| clockwork_warden (basement, hall) | 150 × 185 | 228.1 × 248.0 | 150 × 185 | 36 | default |
| clockwork_warden (first_system_boss) | 128 × 160 | 197.3 × 214.5 | 128 × 160 | 36 | default |
| bossofthegame | 54 × 96 | 29.0 × 112.2 | 54 × 96 | 60 | Stay |
| wolfenstein_boss | 84 × 140 | 38.5 × 142.8 | 84 × 140 | 110 | default |
| wolfenstein_inugami | 176 × 212 | 178.3 × 212.8 | 176 × 212 | 48 | default |
| flying_spaghetti_monster_boss | 168 × 150 | 167.4 × 77.3 | 168 × 150 | 48 | default |
| marshal_gallus | 96 × 140 | 115.5 × 168.6 | 96 × 140 | 60 | default |
| gnu_ton_rider | 54 × 96 | 59.7 × 105.2 | 54 × 96 | 42 | default |
| gnu2 | 630 × 464 | 630.5 × 464.1 | 630 × 464 | 30 | default |
| gram_reaper_boss | 220 × 340 | 210.6 × 206.7 | 220 × 340 | 52 | default |
| mockingbird | 130 × 130 | 318.4 × 150.2 | 130 × 130 | 40 | default |
| mode_collapse_boss | 140 × 132 | 176.1 × 175.2 | 140 × 132 | 34 | default |
| overflow_boss | 150 × 145 | 183.5 × 203.8 | 150 × 145 | 42 | default |
| trex_boss | 140 × 112 | 282.2 × 165.0 | 140 × 112 | 44 | default |
| hunny_horror_boss | 110 × 180 | 115.9 × 194.3 | 110 × 180 | 46 | default |
| smirking_behemoth_boss | 240 × 288 | 208.0 × 266.0 | 240 × 288 | 9999 (environmental kill only) | Vanish |

The placement box is the scale of the boss today. The same Warden profile is
228 × 248 in one room and 197 × 214 in another, because the render basis is
the box and the sheet body is drawn into it. When the character owns its
geometry, this becomes an explicit occurrence scale.

**Provisional values found.**
* `BossClusterScratch::new` builds every boss with `Health::new(18)`.
  `update_boss_encounters` replaces that health on the first active tick, using
  the encounter's `max_hp` or the placement override. The constructed body
  should carry the occurrence's health from the start.
* `boss_actor_cluster` grants flight, direct-velocity steering and the body's
  contact damage beside the body. These belong in the common seed, with the
  boss controller's locomotion stated as an override before construction.

**Slice 2 order.** Each step leaves one authority for the facts it moves.
1. *Identity (2a).* Each profile names its `character`, and the pack refuses
   a name that matches no character. The five missing rows are added, so the
   Hall and the art export show every boss character, and the inspector bridge
   is deleted. The encounter's `voice` names a character row for barks only.
   Three encounters state it, and each states the character the profile
   names. Slice 3 removes `voice`, because the boss speaks as its character.
   That gives the other bosses their rows' `on_hit` and `idle` barks in the
   fight, which is a deliberate change.
2. *Geometry, death and health (2b).* The boss body is sized from its
   character's standing height. An occurrence that differs (the Warden in
   `first_system_boss`) states its scale. The profile's `combat_size` and
   `corpse` move to the character row, and the body is built with the
   occurrence's health.
3. *Construction (2c).* The body is built by the common seed
   (`ActorClusterSeed::new_character_in` and the prepared grant). The boss role
   is composed on it as an override, stated before construction. This removes
   `boss_actor_cluster` and the scratch's body seed, and the body carries
   `CharacterForm`.

**2b input: the character path against the fight body** (probe on
2026-11-21, width × height). The character path is
`sprite_body_collision_for_character_id_from_data`: a standing height and the
sheet's idle body. The fight body is the bounding box of the boss sheet's body
parts, drawn at the placement box.

| Character | Character path | Fight body | Reading |
| --- | --- | --- | --- |
| Warden, Hunny Horror, Mode Collapse, Overflow, Behemoth, T-Rex, Wolfenstein | equal within 0.1 | | One answer already |
| Marshal Gallus | 120.2 × 168.6 | 115.5 × 168.6 | Width: the idle body against the bounding box of all parts |
| Gram Reaper | 213.4 × 206.7 | 210.6 × 206.7 | Same |
| Mockingbird | 320.1 × 151.0 | 318.4 × 150.2 | Same, plus a rounded copy of the height |
| Flying Spaghetti Monster | 176.6 × 79.1 | 167.4 × 77.3 | The Hall's height was copied from an older fight body |
| Inugami | no standing height | 178.3 × 212.8 | The row needs its height |
| bossofthegame | 12.4 × 48 (his own small standing body) | 29.0 × 112.2 | The row says "the fight's body is the boss encounter's": two sizes of one character on purpose. The fight is an occurrence scale. |
| GNU2 | a posed body (`WorldPerPixel(1.3)`), which this probe does not resolve | 630.5 × 464.1 | Resolve it through `BuiltGeometry` |
| GNU-ton (`npc_gnu_ton_boss`) | its manifest does not resolve | 59.7 × 105.2 | The row's sheet must become `gnu_ton_scholar_boss`, the drawing the fight wears |

So 2b uses the common resolution, `BuiltGeometry` (body source, sheet and
sizing). It does not repair the boss road's own resolution. Expect small width
changes for Gallus, the Gram Reaper and the Mockingbird. An occurrence that
differs on purpose (bossofthegame's fight, the Warden in `first_system_boss`)
states a scale on the placement. The profile's `combat_size` is a dead fallback
for every shipped boss, because each sheet publishes a body, so it is deleted
rather than moved.

**2b decisions (2026-11-21).**
* *One resolver.* `BuiltGeometry::resolve` moves out of `ambition_body_seed`
  into `ambition_sprite_sheet::character::catalog_join`. It reads only sprite
  and prepared-character types, which the boss crate already depends on. The
  seed and the boss scratch both call it, so no crate gains a dependency.
* *Scale is encounter tuning.* `BossEncounterSpec` gains `scale` (default 1),
  as the plan's override order allows. bossofthegame's fight states it,
  because his character's standing body is the green room's. Both of his
  placements share the encounter. The Warden in `first_system_boss` takes his
  character's size. That is 15% larger than the old box: the room is
  1280 × 768, and his 248-unit body fits.
* *The spawn anchor stays the box centre* for a boss in 2b. Moving bosses to
  the character road's feet-on-the-box-bottom rule shifts every boss and needs
  a visible check. It belongs to 2c, where the common seed places the body.
* *Health* is built at the encounter's `max_hp`, or the placement override.
  The activation write then repeats the same value; it is no longer a
  provisional 18.
* *Death:* the profile's `corpse` moves to the character's `death_traits`.

**2b record (2026-11-21).** Construction
(`spawn_boss_with_overrides_into`) sizes a boss through its character with
`BossClusterScratch::size_from_character`. That method calls
`CharacterBodyGeometry::resolve`, the resolver the seed uses, at the
encounter's `scale`. Then a placement's authored `combat_size` and `max_hp`
apply. The corpse is read from the character's `death_traits`
(bossofthegame `Stay`, the Behemoth `Vanish`). The profile's `combat_size` and
`corpse` are deleted. A character whose sheet states no body falls back to the
placement box, and in that case the sheet body the scratch already resolved is
kept, scaled. That is the GNU-ton rider today: his row's manifest does not
resolve, and slice 3 makes his art one resolution.

Probe of the scratch before and after `size_from_character`, at each
placement box of the dimensions table:

| Boss | Body after | Drawn height (max of render basis) |
| --- | --- | --- |
| Warden (basement, hall), Wolfenstein, Mode Collapse, Overflow, T-Rex, Hunny Horror, Behemoth, GNU2, GNU-ton | unchanged | unchanged |
| Warden (`first_system_boss`) | 228.1 × 248.0 (was 197.3 × 214.5) | 185 (was 160), as decided |
| Inugami | 178.4 × 212.8, from `standing_height: 141.9` at `scale: 1.5` | 212.1 |
| bossofthegame | 29.0 × 112.2, from 48 at `scale: 2.3375` | 96 |
| Marshal Gallus | 120.2 × 168.6 | unchanged |
| Gram Reaper | 213.4 × 206.7 | 335.3 (was 340): the idle body, not the bounding box of all parts, is the 206.7 |
| Mockingbird | 320.1 × 151.0 | 130.7 (was 130) |
| Flying Spaghetti Monster | 176.6 × 79.1 (was 167.4 × 77.3) | unchanged: the idle body is taller than the old box |

A box no longer sizes a boss, so a fixture that spawned a small Warden on top
of the player (`boss_replay_retraction`) now spawns him beside the player.
His full body would push the player out at speed. Lanes run:
`ambition_boss_encounter`, `ambition_characters`, `ambition_body_seed`,
`ambition_sprite_sheet`, `ambition_platformer2d_publication`,
`ambition_platformer2d_construction`, `ambition_platformer2d_actor_spawn`
and the `ambition_content` lib, plus every app_it module that touches a boss.
The failures that remain are environmental: the music registry, v3's art, and
`gnu_ton_in_a_match`, whose match does not start on HEAD either.

`CharacterForm` is added only in 2c. It brings the body into every
character-keyed system, and before 2c the boss body is still built by the boss
road.

**2c record (2026-11-21).** The boss body is built by the common seed.
`spawn_boss_with_overrides_into` calls `ActorClusterSeed::new_character_in`
with `PreparedCharacterDefinition::role_blueprint`, which is the character's
body plan with the boss's locomotion (free flight at `BOSS_FLIGHT_SPEED`).
`compose_boss_role` then states the boss role over the seed before anything is
spawned. That role is:
* the occurrence's body and health, sized by the scratch;
* a fly-only kit with `fly_toggle` false;
* direct-velocity flight;
* contact damage from the profile's `body_damage` and no `is_hostile`;
* the inert `Passive` brain.

The body then takes the prepared grant with `KitOwnership::CallerResolved`,
because its repertoire is the encounter's moveset, so it carries
`CharacterForm`. `boss_actor_cluster` is deleted. `BossClusterScratch` keeps its
encounter components (config, status) and its sizing. Its kinematics and health
are handed to the seed, so the body has one builder.

*Decision: the anchor stays the box centre.* The seed's rule puts the feet on
the placement box's bottom. `compose_boss_role` overrides it with the
occurrence's kinematics, which are centred on the box, so no boss moves in 2c.
Every boss room is authored for the centre, and the boss body convention
(`BossRef::aabb` is the kinematic box, `frame_center` places the drawing) reads
it. A move to feet-on-the-floor would be a separate change, made with a
capture of each room.

*What `CharacterForm` brings.* Character-keyed systems now see bosses, so
direction, rally, aggression, attack VFX resolution and the actor render index
give a boss its character id. Drawing is unchanged: the actor sprite path
yields to the boss path by identity (`actor_sprite_path_owns`), and the worn
presentation binder is gated on `PlayerVisual`.

Lanes run: the lib tests of `ambition_platformer2d_actor_spawn`,
`ambition_platformer2d_construction`, `ambition_characters`,
`ambition_platformer2d_publication`, `ambition_body_seed` and
`ambition_boss_encounter`, plus an app_it lane over every boss fight,
`hall_of`, `rollback_coverage` and `two_peers`: 261 passed, 3 failed. The 3
failures were already red before 2c: `gnu_ton_in_a_match` and the two T-rex
drawing tests.

**3a record (2026-11-21).** A boss speaks as its character. The encounter's
`voice` is deleted. `BossConfig::bark` reads the profile's `character`, which
the pack already refuses when it names no row. This is the deliberate change
the 2a step named: the Mockingbird, the Flying Spaghetti Monster, the T-Rex
(on hit only), the Hunny Horror and Wolfenstein now speak their rows' `on_hit`
and `idle` lines in their fights. The Warden, the Gram Reaper, Mode Collapse,
the Overflow, Marshal Gallus, the Behemoth and GNU2 have no lines and stay
silent.

GNU-ton's row now draws what his fight wears: its manifest is
`gnu_ton_scholar_boss`, the same sheet as the boss art key, and its
`standing_height` is his fight body's 105.2. The fused picture of the scholar
on the giant left the Hall. The giant has its own row and stand. Now every
boss art key names the same sheet as its character's manifest.

Four display names generated from sprite targets were wrong, and the Hall
showed them: "Trex Enemy", "Mockingbird Boss", "Flying Spaghetti Monster
Boss" and "Gnu Ton Boss". They are now "T-Rex", "Mockingbird", "Flying
Spaghetti Monster" and "GNU-ton (Boss)".

*Decision: the bill name stays the placement's.* The name card, health bar,
banner and encounter progress read `BossConfig::name`, which is the
placement's `name`. A test of billing by the character's display name failed
on two counts:
* Display names are unique in a composition (the catalog validator), and a
  form that is the same person in the fight cannot share its base's name.
  `npc_gnu_ton` is "GNU-ton", so his boss form cannot be.
* A room may bill a boss by an occurrence name: the Warden is "System Boss" in
  `first_system_boss`, which `behavior.rs` documents.

So the bill is an occurrence fact, like `scale`. The subject's identity is its
`CharacterForm`, and its health is its `BodyHealth`, so the bar keeps no second
pool and no second identity.

**3b record (2026-11-21).** A boss sheet draws its character's image.
`BossCatalogFragment::from_prepared_with_characters` names the image of each
behaviour's sheet: the `spritesheet` of the character the behaviour is an
instance of, from the pack's lowered character catalog. Fourteen of the
sixteen `sprite_filenames` entries in `boss_art_keys.ron` are deleted. Two
remain, for sheets no boss character draws: the fallback Gradient Sentinel and
the giant GNU-ton rides. Naming an image for a sheet a character draws is
refused (`SpriteFilenameBesideCharacter`). The asset-scope export lists each
boss's image by the same rule (`boss_image_of_character`), so the editor's
preflight still finds every boss's art. The sheet's frame grid and its rows
(`boss_sheets.ron`) stay boss data. A boss's row vocabulary is not a
character's, and the plan does not merge animation enums.

**Headless frame baseline** (tree `feb420d2a`). Measured with
`scripts/measure/headless_room_frame.sh`: the `profiling` build without Tracy,
2000 ticks, 3 reps, and the median of the census windows (ms per frame). The
binary is kept as `target/notrace/ambition_game_bin.baseline-feb420d2a`, so
slice 4 can re-run the same build on the same machine.

| Room | Rep 1 | Rep 2 | Rep 3 | PreUpdate | Update |
| --- | --- | --- | --- | --- | --- |
| `hall_of_characters` (ordinary characters, roster scale) | 29.66 | 29.79 | 28.14 | 15.5–16.5 | 9.1–9.5 |
| `you_have_to_cut_the_rope` (Smirking Behemoth) | 3.94 | 3.95 | 3.87 | 1.73 | 1.07–1.12 |
| `frontier_ore_corral` (Marshal Gallus) | 4.42 | 4.41 | 4.57 | 1.89–2.00 | 1.16–1.25 |

Not yet recorded: hurt volumes and action repertoire per pose. These come from
the app-level scenarios in slice 4, through the real encounter paths.

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

**Done 2026-11-21 (slice 2a).** Every boss behavior names a catalog character,
so the ordinary export covers the bosses. The module, its assembly call, its
tests, the origin badges and the boss date handling are deleted. The role
filter now reads the common records' `boss` tag. No review notes were filed on
`provider:boss/id` keys.

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
