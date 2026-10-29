# Actor residual-kernel decomposition

**2026-10-26 review boundary:** D33's carve receipts describe historical and current
implementation work, not a permanent prerequisite for reconsidering package names or using a
non-rollback game profile. Before renaming `ambition_platformer2d_actor_monolith`, validate its
actual control/body/action responsibility, exports and consumers; preserve content-owner
identifiers, scripts and asset paths independently. Crate merges require higher evidence than a
successful carve; see [ADR 0039](../../adr/0039-evidence-based-crate-boundaries.md) and
[architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).
**Scope:** the rules for carving responsibilities out of
`ambition_platformer2d_actor_monolith` (track D33), the carve safety map and the
exit condition.
**Doctrine and target authorities:** [engine architecture](architecture.md).
**Packets:** [ownership migration packets](actor-monolith-work-frontier.md).
**Priority:** [the queue](../queue.md).

## Goal

Keep a coherent body, control and action execution kernel. Move session
restoration, preparation, world-object behavior and content integration to
their owners. The residual crate is not the only mixed container: `combat`,
`characters`, `core` and `runtime` also hold mixed responsibilities.

## Current graph

Measure the intra-crate module graph with:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

The module graph is a DAG after the October 24 vocabulary and ability cuts.
The session lifecycle and room-crossing policy now have a separate owner,
`ambition_platformer2d_session`, above the actor kernel. Publication and runtime
call that owner directly. The actor kernel has no production dependency on it.

The remaining work is responsibility separation. Keep device input and global
schedule assembly at runtime. Control owns participant input and held-frame
history. Move the remaining cross-domain adapters to their owners. Keep accepted
control, body custody, body motion, and action execution together. A dependency
on a lower domain is valid when the kernel needs that domain's facts.

The instrument counts textual `crate::<module>` paths inside one crate. It
misses many imported names, cross-crate dependencies, shared writers, message
timing, plugin prerequisites, macros and re-exports. Use
`scripts/module_graph.py --crate ambition_platformer2d_actor_monolith` to
inspect `use` edges as a second view. The generated ECS and Cargo indexes
show system registrations and declared/resolved crate dependencies, but the
resolved Cargo graph is feature-unified across the workspace. A consumer's
closure still needs its own check. A lower SCC count is evidence, not a goal.
Keep a cycle only when its shared invariant is stated.

## Landed carves

- Settlement state and the corrected actor-spawn extraction
  (`ambition_platformer2d_actor_spawn`, builders only). Do not move live actor
  views, provocation, fighter-ladder projection or dismounted-rider rebuild back
  into spawning. The crate boundary and construction behavior tests hold this split.
- `ambition_conversation`, `ambition_encounter_features` and
  `ambition_abilities` (the wielded kit, the gravity grenade, traversal verbs and
  the shared cooldown).
- Checkpoint restoration lives in `session::checkpoint`; `ambition_shrine`
  owns the shrine interaction only (K3).
- Actor construction planning is its own crate, `ambition_platformer2d_construction`
  (K6); placement lowering lives in its `placements` module.

## Deliberately not carved

- `possession` (`control::possession`), `teleport`, `trapdoor` and `flyline`
  (`abilities/traversal/`) are control authority. The runtime registers their
  systems, and `body_custody`, `control::authority` and
  `control::input_systems` use them. Moving them would rename the coupling.

## Rules for every carve

1. **One packet, one ownership claim.** Separate a correctness fix from the
   mechanical move that follows it. List non-goals.
2. **The old internal path disappears.** Move consumers to the owner. Do not
   keep internal re-exports to avoid editing imports. A curated public facade
   re-export is a different, deliberate thing.
3. **State and lifetime travel together.** For each moved value, account for
   construction, writers, scope, retraction, rollback and checksum,
   reconstruction and observation. An unknown writer or lifetime holds that part
   of the packet.
4. **Preserve behavior and wire identity in a pure move.** Keep wire IDs and
   encoded meaning. A format change is a separate reviewed commit.
5. **Preserve scheduling semantics.** Record phase membership, ancestor gates,
   ordering, deferred flushes and the population that runs. Startup checkpoint
   restore must not move under a gameplay gate.
6. **A carved crate never depends back on the monolith.** Cargo forbids the
   production edge; the workspace policy also forbids the dev-dependency edge,
   which would drag the monolith's closure into the carved crate's tests.
7. **Put integration above the domains it joins.** For example, `interact`
   owns the interaction fact and asks a dialogue port for the dialogue fact, so
   `features` names no dialogue type.
8. **Tests witness consequences.** Use a production-path behavior fixture plus
   an absence or external-consumer fixture. A zero-test filtered run is not
   acceptance.
9. **Refresh citations in the same commit.** Update absence contracts (see the
   safety map), policy arguments and source citations for moved code.

A host that avoids a direct monolith dependency is a goal, not a ratchet. The
monolith still reaches the host through `ambition_platformer2d_runtime`.

### Edge dispositions

When a packet reviews a monolith edge, give it one disposition:

- **KEEP_DIRECT:** the consumer legitimately depends on this lower domain.
- **MOVE_AUTHORITY:** move behavior, state and installation together.
- **MOVE_ADAPTER:** keep the integration but move it to the integration owner.
- **GROUP_PACKAGE:** an internal cycle is part of one authority.
- **HOLD:** a named proof prerequisite is missing. No generic callback stands in.
- **REMOVE_AFTER_REPLACEMENT:** delete an obsolete projection only after its
  replacement covers every customer.

Standing examples: interactive objects reading `ActingParticipant` and
perception reading `ProjectileAllegiance` are KEEP_DIRECT. Session assembling
world and physics services is ordinary composition. `ActiveContentBinding`
stays with session; it is not a reason to add world IDs everywhere.

## Target package and integration boundary

Keep `ambition_platformer2d` as the public game-facing facade.
`ambition_platformer2d_runtime::PlatformerEnginePlugins` already composes
plugins and simulation phases. Do not add a second plugin-assembly crate.
Separate two kinds of integration work:

| Layer | Owns | Must not own |
| --- | --- | --- |
| `ambition_platformer2d_runtime` (exists) | Profile selection, installation, public phase ordering, schedule policy | Body algorithms and every feature transaction |
| `ambition_platformer2d_integration` (candidate) | Cross-domain simulation adapters and coordinated room-publication operations | General service lookup, named game content, domain state machines |
| `ambition_platformer2d_session` | Session construction, checkpoint admission, restoration, retirement, and room-crossing policy | Body algorithms, item transition rules, or publication receipts |
| Actor/body kernel | Accepted control, body custody, movement and action execution | Host load covers, checkpoint routing, content publishing |
| Construction, world, session, item and combat domains | Each domain's facts, transitions, validation and rollback state | Knowledge of the higher integration crate |
| Game providers and hosts | Selected content and device/presentation work | Engine-domain algorithms |

The name `ambition_platformer2d_integration` is **proposed**, not a crate
to create before it has a real owner packet. An `ambition_engine` catch-all
would not make the responsibilities clearer. Keep integration code in the
existing runtime where that is the narrowest present owner, then add the
integration crate **only if** the room-publication move gives it a stable
independent implementation and dependency direction. **It did (K4 step 3,
2026-10-22), and the crate is named for that one packet,
`ambition_platformer2d_publication`, not for the layer: a second packet
that belongs at this level earns its own crate or a deliberate rename.**

The target dependency direction is host/provider -> runtime -> integration
-> domains -> foundations. Runtime may also install domain plugins directly.
Domains must not depend upward on runtime or integration. Dependency direction
does not dictate message flow or simulation order. A crate move must not add
scheduled systems by default. It may move a system body and leave its one
registration in the same phase.

## Dependency-first extraction sequence

These are **design packets**, not a new priority queue. AP14 in
[the execution queue](../queue.md#ap14--actor-kernel-decomposition)
selects when the work proceeds. The A-packet
[catalog](actor-monolith-work-frontier.md) owns the existing behavior proofs.

### K1 — Move transition custody interpretation upward — DONE 2026-10-21

**Receipt:** `CustodyEndingAtCommit` now lives in
`ambition_platformer2d_runtime/src/room_transition/custody.rs`, beside its
one consumer (`room_transition/loading.rs`). `body_whereabouts` keeps the
durable whereabouts state and its readers and names no session type; the
`body_whereabouts -> session` edge is gone. No system, resource or state
moved: the type is a `SystemParam` the loader already took. The record below
is the plan it executed.

The adapter depended on
`session::lifecycle_commit::LifecycleIntent` and
`rooms::another_player_stays`; the runtime transition loader is its
production consumer (`room_transition/loading.rs`). Move the adapter and
its tests near the room-transition integration path. Keep durable
whereabouts state and its ordinary readers in the domain. Do not make the
low-level body-location module import session operation types.

**Proof:** the same away-from-home population is released or retained after
crossing, checkpoint restore, same-room replay, and a crossing where another
player stays. Verify cancellation, rollback and two live rooms. Remove the
qualified edge and ensure no new registration or hidden query work is added.

### K2 — Move commit-facts *selection* above construction — DONE 2026-10-21

**Receipt:** `CommitFactsSource` is defined with the room publication
transaction (`publication/transaction.rs`, exported as
`rooms::CommitFactsSource`). Construction still receives one resolved
`PersistedFates` per commit and no longer names
`session::checkpoint::CheckpointOperationKey`; the `construction -> session`
edge is gone. The selector's resolution in `publication/stage.rs` is
unchanged, so no checkpoint consequence is evaluated earlier than before.
The record below is the plan it executed.

The selector `CommitFactsSource` contained
`AfterTheRestore(CheckpointOperationKey)`. `publication/stage.rs` resolves
that selector, while construction owns the resulting `PersistedFates`.
Move the *choice of when and where to get facts* to the publication/lifecycle
coordinator. Construction must still receive one resolved value for a commit.
Do not read a candidate session's save from the currently live session.
Do not evaluate checkpoint consequences before the operation is admitted.

**Proof:** a session prepared while another game runs; accepted and refused
checkpoint reset; replay after save changes; candidate failure; first-tick
spawn state. Remove `construction -> session` without adding a general facts
provider registry, new global resource, or duplicate saved state.

### K3 — Prepare actor construction for a domain owner — DONE 2026-10-21

**Receipt:** construction now names no other module of the monolith.

- **Death and provocation flags.** `ambition_persistence::save_data` owns
  their spellings (`enemy_dead_flag`, `enemy_dead_until_rest_flag`,
  `npc_hostile_flag`) beside `DEAD_UNTIL_REST_SUFFIX`, which it already owned
  because a rest clears those flags. The catalog's
  `RespawnPolicy::death_record` says which record a policy keeps (an
  exhaustive match). The policy-to-flag glue stays with the one writer
  (`body/damage/actor_hit.rs`). The `fate_flags` module is deleted.
- **Shrine.** `HealShrine`, `heal_save_shrine_system` and its tests moved to
  the new crate `ambition_shrine`. The crate publishes a `ShrineRest` set;
  the checkpoint resume orders after the set, not after the function. The
  monolith still installs the system in the same sets and order, and still
  registers the component for rollback. `ShrineActivationPulse` stays in
  `shared_tangle` because the renderer reads it.
- **Held-item namespace.** `HeldItemId` moved from the monolith's
  `world::rooms::binding` to `ambition_characters`, beside
  `held_item_ids()`, the list it resolves against. This was the last
  `construction -> world` edge.
- **Instrument.** `measure_kernel_module_graph.py` did not resolve
  crate-root re-exports, so `crate::rooms::…` (`pub use world::rooms`)
  counted as no edge. With the fix, K1 and K2 left `construction` in the
  cycle (nine modules, not the eight first reported). After K3 it is out.

**Can `ambition_platformer2d_actor_spawn` own construction? Not as is.**
Construction lowers every placement kind, including ground items, chests and
shrines. Pickup and chest construction belong to item integration. Planning therefore stays separate
from spawn materialization. Construction is now a leaf of the monolith graph
and can become its own owner. Its callers are `features` (50 references),
`world` (5) and `session` (5), so do the move with K4's publication
coordinator, not before it.

The record below is the plan it executed.

Recheck `construction -> fate_flags` and `construction -> shrine`, plus its
callers in `features` and `world`. Give death flags and shrine components a
semantic owner. Do not move a value to `shared_tangle` just to satisfy the
compiler. Check whether `ambition_platformer2d_actor_spawn` can own more
planning work without importing the monolith back into itself. If it cannot,
keep planning separate from spawn materialization. Reuse A3's prepared-plan,
`SimId`, provenance and preflight tests.

### K4 — Room publication owns the transaction — DONE

The room coordinator left `world::rooms` and then the actor kernel in commits
`348bca4e0`, `b7c8ffd75`, and `bbe442392`. The owner is
`ambition_platformer2d_publication`. The runtime, provider, and rollback lifecycle
compose it through the public publication API.

Publication prepares a hidden candidate, checks its receipt, and publishes or
refuses the result as one operation. Session owns admitted operations, restore
facts, and retirement. World owns geometry and live-instance identity. Item
custody stays with its domain.

K15 co-locates the typed room plan, receipt, placement adapters, and transaction.
Commit and verification operations are private to the publication crate. The
old actor exports and temporary source guard are deleted. Construction fixtures
are local test support. No production API is widened for a test.

Keep candidate visibility protection, deferred flushes, rollback identity, and
lifecycle cancellation intact. Publication tests cover normal load, content
reload, checkpoint replay, failed candidates, confirmed transitions, and live
room identity. Independent capability profiles remain an A9 obligation.

### K5 — Continue existing domain packets, then assess the residual kernel

Apply A7 to the `items <-> session` loop; keep item identities, custody and
accounting item-owned while session coordinates restore. Apply A2/A5 to
projectile contact arbitration; keep `ProjectileAllegiance` a legitimate
perception input. Apply A6 to session-specific character activation and
prepared definitions. Keep `avatar`, accepted control and body integration
together unless A4 finds an independent authority. Handle the
`assets <-> character_sprites` presentation cycle separately. Do not split
one coherent actor kernel only to make its internal module graph acyclic.

**Done 2026-10-21 — the asset cycle.** Its two edges were two different
things. `assets -> character_sprites` was the asset catalog's row builder
calling `all_character_sprite_filenames_in`, a projection of the character
catalog into manifest rows that had one production consumer: it moved to
`assets::platformer_assets` with its two tests. The other reads were
`game_assets` (it loads every kind of art into `GameAssets`) calling the
sheet builders; that is the higher layer, so `game_assets` is now a
top-level module above both. `character_sprites -> assets` (the catalog
ids) is the remaining, downward edge. No system or resource moved;
`PropSheets`' rollback exemption follows its new type path.

**Done — A2 contact helpers.** The swept contact pair and its ordering are
`crates/ambition_projectile_combat/src/feature_contact.rs`.
The shot-birth rule is
`ambition_combat::shot_birth::clear_of_the_feet`. Both projectile and brain
callers use the shared rule. `target_is_ignored` stays with damage resolution.
No system, resource, or message moved.

**Done 2026-10-22 — the rules reading.** `avatar -> session` was the
body control gate reading `session::governing_rules::RulesOf`: which rules
govern the live room an entity is in. Its owner is the rule declarations'
owner, not the session. The module (`CurrentRoom`, `LiveRuleRooms`,
`GoverningRules`, `RulesOf`) names nothing else in the monolith: it reads
`DeclaredRules` and `ActiveRoom` (`ambition_combat::scoped_rules`), the
world crate's rooms and shared_tangle's lifecycle, all of which
`ambition_combat` already depends on. So it is
`ambition_combat::governing_rules`, with its tests; its two-game session
fixture is `governing_rules::test_fixtures` under combat's `test-support`
feature, which the monolith's tests take as a dev-dependency. Readers in
the monolith (avatar, features, world, session's death rules), the
runtime, the sim view and the Smash and Sanic crates name it there. They
are SystemParams: no system, resource or message moved. The cycle went from
seven modules to four.

**Done 2026-10-22 — the room sweep.** `features -> world` was one
reference: room feature construction calling `RoomBindings::sweep`, which
resolves a room's patrol and motion path references before construction
mutates the world. The sweep reads only `RoomSpec` and the binding ledger,
both below the monolith, so it is the world crate's
(`ambition_platformer2d_world::rooms::binding`), with its tests; any content
backend can now run it without the monolith. No system, resource or message
moved. Its unused `CharacterId` namespace marker, left over from AC6, was
deleted first. The component is seven: `world` is out of it.

**Done 2026-10-23 — audio and music leave the crate.** `audio` (the Kira
install, the channels, the acoustic environment) and `music` (the intent from
encounter, boss, room, conversation and radio state) named nothing else in the
monolith in production; only two tests reached `session::data`'s registry
fixtures and the asset catalog. They are `ambition_platformer2d_audio`, a
crate below the facade that depends on `ambition_audio` (with `kira`), the
world, encounter, boss, conversation and character crates the intent reads,
and `bevy_kira_audio`. The facade's `audio` feature installs it
(`ambition_platformer2d::platformer_audio::Platformer2dAudioPlugin`); the
monolith lost its `audio`, `authored_audio` and `web_audio` features and its
`bevy_kira_audio` dependency, and its `visible` no longer implies audio, so a
windowed game that does not select `audio` (Outlander, the minimal game) no
longer links it. The catalog test (`every_live_music_track_resolves_under_web_served_assets`)
stayed in the monolith with the catalog it tests. No system, resource or
message changed; `AudioEnvironment`'s rollback-coverage row follows its new
path. Receipt (MEASURED, `wc -l`): about 1,200 production and 1,400 test lines moved; the
monolith's tests 1,338 → 1,289 lib tests, the new crate 49.

**Done 2026-10-23 — the gameplay trace recorder leaves the crate.**
`dev::trace` (the per-frame player and body recorders, OOB and teleport
detection, the F8 hotkey, the dump flush and the headless driver's
`record_simulation_frame`) named nothing else in the monolith in production;
one test helper wrapped a core constructor. It is `ambition_platformer2d_trace`,
below the runtime, beside the format crate `ambition_gameplay_trace`. The
runtime installs its `TraceSchedulePlugin` as before and its `portal` turns on
the recorder's (`BodyTeleported` as a frame fact); the facade names it
`trace_recorder`, and the app's hotkey and the headless binary use that path.
The monolith loses its `dev` module. No system, resource or message changed.
Receipt (MEASURED, `wc -l`): about 1,340 production and 910 test lines moved;
36 tests run in the new crate.
The same day, the room-entered quest producer (`quest`,
`push_room_entered_quest_events`) moved to the runtime that installs it. Its
note said it lived in the monolith "because `RoomSet` does"; `RoomSet` is the
world crate's now, and the runtime depends on both that and the quest
registry's crate, which do not depend on each other.
The cutscene player (`cutscene`: room-entry triggers, playback, the skip
hold and the seen-flag save effect, `CutsceneSchedulePlugin`) followed for
the same reason: it read nothing else of the monolith, `ambition_cutscene`
must stay gameplay-free, and the runtime installs it and is where a profile
without the `Cutscenes` capability leaves it out. About 400 lines.
The time module followed too, as the runtime's `sim_clock` (time control:
intents, smoothing, reporting; and the `SimDt` mirror with its set): only
the runtime named it. Its two source scans of the monolith's clock
discipline stayed in the monolith (`clock_discipline_tests`), because what
they scan is the monolith's source; left with the time module, they would
have scanned the runtime's.

**Done 2026-10-23 — the debris physics adapter leaves the crate.**
`world::physics` (the Avian install, paused while no debris exists; each live
room's floor; debris bursts, their lifetimes and safe retirement) named
nothing else in the monolith. It is `ambition_platformer2d_physics_debris`, a
leaf below the monolith; its `physics_debris` feature carries avian2d, and
without it only `PhysicsRoomEntity` and a no-op `retire_physics_entity` remain
(room transitions call it unconditionally). The monolith re-exports it as
`world::physics`, so no consumer path changed, and its `physics_debris`
forwards to the crate's. The clock-discipline scan loses its allowance for the
file. No system, resource or message changed. Receipt (MEASURED, `wc -l`):
about 580 production and 460 test lines moved; 11 tests run in the new crate
with avian and 2 without. `measure_kernel_module_graph.py`: `world` 2,557
production lines.

**Assessed 2026-10-21, not cut — A6 and A7.** `character_runtime -> avatar`
(1) is match preparation reading the session root's `InitialBodyPolicy`, to
refuse a local seat in a session that also lowers a home avatar; that is a
real read of home-avatar policy, and moving the declaration out of `avatar`
would only reshape the graph. `items -> session` (4 counted, 3 production)
is item systems joining session-declared boundary sets
(`DurableHorizonSet`, `DurableRestoreSet`, `ContentRoomReplayResetSet`) and
reading `SaveRestored`: session coordinates the boundary and the item
domain places itself in it, which is A7's stated destination. A7's open
work is the atomic durable horizon (one writer for occurrences, custody and
minted items) in [item custody](item-custody-and-accounting.md#open-work),
a save-semantics change with its own proof, not a module move.

### K6 — Actor construction becomes its own crate — DONE 2026-10-23

K3 left `construction` a leaf of the monolith graph and named K4 as the
precondition for giving it an owner; K4 landed on 10-22.

**Receipt:** `ambition_platformer2d_construction` is the planner (the
recipes, the relations, `PersistedFates`, placement lowering's registry and
context), moved out of the monolith's `construction` module below it. The
monolith's features, world and session name it directly; the publication
coordinator, the provider and the runtime name it; the facade exports it as
`construction` (`actors::construction` is gone, no re-export under the old
path). It installs no system: zero ECS invocations added or moved.

- **The tests stay with the lowering they drive.** The 55 tests of
  `construction/tests.rs` build rooms through the monolith's features
  (the staging registry, the static lowerers, summons, character runtime),
  so they are the monolith's `construction_tests`; their fixtures are its
  `construction_fixtures`, which the coordinator's tests still enable through
  `test-support`. The 6 persisted-fate tests touch only the planner and moved
  with it. Same population: 55 + 6.
- **Kept as is:** the registry owner label (`"ambition_platformer2d_actor_monolith"`)
  that the recipes register under, and the log target, so no dump or receipt
  changed.
- **Measured** (`measure_kernel_module_graph.py`): the monolith's production
  lines 63,879 → 61,303; `features -> construction` (50), `session ->
  construction` (5) and `world -> construction` (5) left the graph with the
  module. The cycle is the four of K3 (`abilities`, `features`, `items`,
  `session`), unchanged. Capability footprint 62 → 63 nodes, a carve
  (`construction_entered_the_closure_2026_10_23`).

**The gravity-zone mechanic moves up into the runtime (2026-10-23).**
`GravityPlugin`, `GravitySet`, the body motion-frame resolution and the reset on
a room reset are `ambition_platformer2d_runtime::gravity`: the runtime was
the only installer and nothing else in the monolith named the module (its one
edge, `ContentRoomResetSet`, the runtime names from above). The zone types and
systems stay in `shared_tangle::gravity`. No system moved schedule or set.
Monolith production lines 61,303 → 60,612.

### K7 — Session lifecycle leaves the actor kernel

Commit: `1f7f01c39`.

Checkpoint admission, restore, reset, session construction and teardown, and
room-crossing policy are `ambition_platformer2d_session`. The runtime installs
its checkpoint offer beside the item-domain offer. The combined actor wrapper
is deleted. The session owns its snapshot codec and rollback declaration;
schema keys, field order, phase membership, and gates stay the same. Callers use
the session crate or the facade's `session` surface. The actor crate exports no
session or room-crossing mirror.

The moved tests use the public body constructor. No production API is widened
for a test. The remaining kernel graph is a DAG. This cut does not complete D33.

### K8 — World services leave the actor kernel

Commit: `d22c2c18e`.

The world crate owns authored room commands and condition-gated lock walls.
`ambition_encounter_features` owns authored switch commands. The runtime owns
world-fact command and condition registration. The combined gate fixtures stay
in runtime because they join flags, inventory, abilities, encounters, and room
construction through public APIs. The room-command tests move with their owner.
No production API is widened for tests.

Derived rollback declarations move to the world and encounter owners. The
runtime calls both declarations in the existing registration pass. Phase
membership, gates, ordering, and the active system population stay the same.
The actor crate's `world` and `world_facts` paths are deleted. The debris
feature belongs to runtime composition. The display-state forwarding module,
empty platform module, and duplicate color helper are deleted.

### K9 — Match time and health effects belong to their domains

Commit: `33b070f86`.

`ambition_match` owns `LiveMatchTicks`, its wire codec, and its rollback
registration. Match identity and outcome declarations move to the same owner.
`ambition_damage::healing` owns heal requests and their consumer; its two routing
tests move with it. The message key, target policy, and wire meaning stay the same.
The runtime calls both domain declarations in its existing registration pass.

### K10 — Item lifecycle integration leaves the actor kernel

Commit: `33b070f86`.

`ambition_platformer2d_items` owns inventory commands and conditions, starting
bags, match item cadence, custody restoration, persistence, and checkpoint
baselines. It composes lower item capabilities and names no actor kernel type.
The session coordinates the same admitted capture and restore boundaries.
The facade publishes this integration as `item_runtime`; the inventory model
stays at `items`. The old actor `items` module is deleted.

### K11 — Projectile combat execution leaves the actor kernel

Commit: `33b070f86`.

`ambition_projectile_combat` owns charging, combat allegiance, swept contacts,
interception, and hit delivery. The reusable model stays at `ambition_projectiles`.
The allegiance codec and declaration move with the executor without a wire change.
Body construction and damage fixtures stay in the actor crate's `projectile_tests`.
Private contact tests and the module-entity bolt fixture move with their owner.
The old actor `projectile` module and its shot-birth forwarding export are deleted.

### K12 — Match activation belongs to session lifecycle

Commit: `33b070f86`.

The session owns match preparation admission, body seating, opening holds, match
object retirement, and the cast view declaration. `MatchLifecyclePlugin` installs
the same systems with the same phases, gates, chains, and projection edge.
The runtime composes it beside character materialization. Match activation tests
move with the session and use public body construction. No production API is
widened for a test. The actor character module exports no match activation path.

### K13 — Character materialization leaves the body kernel

Commit: `92c8dc8c4`.

`ambition_platformer2d_assets` owns character declarations, asset demand and
readiness, atlas residency, source attribution, catalog assembly, and startup
asset loading. The body kernel owns live mechanical projection and deterministic
pose resolution in `character_projection`. The runtime composes both installers
with the session's match lifecycle installer. The same systems retain their
phases, gates, chains, and ordering edges.

The asset owner has no dependency on the body kernel. The kernel uses the asset
owner only in integration tests. Core asset catalog types stay at
`ambition_asset_manager`; forwarding type exports and the source-plugin wrapper
are deleted. The public facade owns asset profile features. The actor crate's
asset, character-sprite, and character-runtime modules are deleted.

### K14 — World-object transitions leave the body kernel

Commit: `d48699d2a`.

`ambition_breakables` owns destructible state, accepted damage, stand collapse,
return timers, collision volumes, and destruction loot. State reads are public.
The damage operation changes health and starts the return timer together.
Hit dispatch keeps its recipient, room, and repeat-hit rules.

`ambition_platformer2d_items` owns pickup and chest grants, collection locks,
consumed occurrence horizons, and loot construction. `ambition_world_items`
owns physical chest state and falling motion. Boss rewards read that lower
state; item grant integration consumes boss lifecycle facts without a cycle.
`ambition_world_returns` owns admitted return times and their horizon owners.
Each domain keeps its own transition rules.

`ambition_time` owns elapsed session time and its wire codec. Shared lifecycle
advances the clock. `ambition_characters::control` owns acting participant
queries. Live-room reach and gestures belong to `ambition_platformer2d_interaction`.
Attempt residue is lifecycle vocabulary. Rollback keys and bytes stay the same.
The old actor object modules and forwarding exports are deleted.

### K15 — Room construction belongs to publication

The typed room plan, construction context, placement adapters, content staging,
receipt, and their tests move into `ambition_platformer2d_publication::construction`.
`RoomConstructionPlugin` installs the same placement interpreters. The runtime
composes it beside body simulation. Commit and verification operations are
private again. The forwarding spawn function and temporary source guard are
deleted. The actor kernel is a dev dependency of publication.

`ambition_encounter_features::spawn_service` owns wave spawn requests and their
body-construction tests. It calls the existing actor builder directly. The
request/service order stays unchanged. The old actor spawn forwarding function
and test-support feature are deleted. The old raw-spawn file inventory is
deleted; typed scopes and live receipt checks hold the construction boundary.
Shared data-only cast fixtures belong to
character test support. No scheduled invocation or rollback state is added.

### K16 — Device input and phase order belong to runtime

Commit: `f06fc1f87`.

Device decoding, participant seating, context claims, frame adapters, and their
behavior tests move to `ambition_platformer2d_runtime::input`. Runtime owns the
concrete phase graph. The actor `schedule` module and runtime `host_input`
forwarding module are deleted. Menu labels belong to `ambition_input`; the
slot-publication label belongs to `ambition_characters::control`.

The body kernel keeps raw-frame publication and its authority guard. Fixed-tick,
rollback, and frame-step hosts retain their publication rules. Chains, gates,
flushes, wire keys, and active system counts stay the same. Device dependencies
move out of the actor manifest. Runtime still selects device input, as it did
before this cut. Independent input profiles remain a separate exit.

### K17 — Cross-domain schedules belong to runtime

Commit: `feb474c84`.

Runtime owns the world-object, match, collection, interaction, and effect
schedule composition. `BodySimulationPlugin` keeps accepted control, body
execution, and their private state in the actor kernel. Existing chains, gates,
phase membership, and deferred visibility stay the same. No invocation is added.

Runtime owns the world overlay, combat-rule projection, match outcomes, and
sudden-death announcement. The announcement keeps its rollback key and bytes.
Persistence owns the shared flag-and-quest write. The actor effect-bus and
world-overlay paths are deleted.

Schedule, silhouette, and projectile integration fixtures move to runtime and
use the public prepared-body builder. The actor and view crates have no runtime
dev dependency. Private body constructors remain private to their tests.

### K18 — Startup policy and rollback declarations follow their owners

Session owns `initial_body`: startup character selection, whether to build a
home body, and its prepared resource bank. The kernel keeps live character
execution and body ability grants. The old startup module is deleted.

Each domain declares its own rollback state. Runtime composes the offers from
abilities, held items, body seeds, actor construction, shrines, and the existing
domain owners. Registration keys, probes, codecs, and wire versions stay the same.
The kernel registrar names only kernel state.

Persistence owns durable NPC provocation facts, their reducer, and the saved
NPC dialogue flag name. Runtime keeps their existing ordering. No system
invocation is added.

The public `body` API owns body execution. The old `features::ecs` tree,
controller-named movement wrapper, and crate-root forwarding exports are gone.
Foreign types are read from their owners.

Global moving-platform advancement and collision-cache retirement now belong
to the world owner. Runtime installs that step between body rescue and the
body-contact snapshot. The phase, deferred visibility, and invocation count
stay the same.

Observation captures trail geometry on the presentation clock. Rendering owns
its colors, retained gizmo assets, room bands, and retirement. A system pipe
passes plain drawing facts across that boundary in one Update invocation.
Trail creation and advancement stay in the body kernel.

Runtime owns fall-rescue integration. It combines room policy, boss activity,
body contacts, and world platform creation. Rescue stays after mount steering
and before the world platform step. The empty avatar compatibility modules
are deleted.

Observation initializes the default-view cast (`FramedCast`). Session/match
code publishes it for camera framing. Possession setup initializes control
state only.

**Remaining D33 work:** audit residual responsibilities and public operations
against the exit below and close the A4 acceptance arms. Possession stores
one record for each participant seat. Two participants can possess, drive,
and release separate bodies through one control path, including under rollback.
Boss bodies now use the common integration phase. Their optional envelope,
authored tuning, contact field, and movement cues use the same data path as
other actor bodies. Boss phase maintenance runs after integration.
Body reactions use one early phase with proper-time recovery and unscaled
hitstop expiry. Observer requests follow the body that holds the driving seat.
The empty actor-maintenance phase and the body-role clock paths are deleted.
Real simulation arms now cover removal of one possessed body while another
participant keeps control, and active-move continuity when a seat is released.
A constrained-motion witness now covers home and actor riders, both carrier
roles and timed dismount on eager and rollback hosts. It checks one tentative
kernel step before the saddle pin and one free step after release.
Control acceptance now owns script holds, commanded movement, and action
schemes. Room technique grants follow each body's driving relation, including
secondary participants. Rule reads borrow declarations; the primary observation
projection is not an eligibility authority.
Keep accepted control, body custody, motion, and action execution together.
Capability profiles remain a separate C2/SDK exit.

### K19 — Owned body API

`3efa243ff`: public `body` operations replace the legacy `features::ecs` tree, enemy wrapper, and root forwarding exports.
State fields, wire keys, phase order, and invocation counts stay the same.
Guards: 600 actor, 262 runtime, 116 app, 58 Smash, and five profile tests; 35 workspace policies and 53 absence contracts.
Do not restore compatibility aliases or foreign type forwarding.

### K20 — World-owned platform step

`7cf962e92` fixes lift detection for ordinary bodies, collision frames, planes,
and live rooms. `e952081cf` moves platform advancement and collision-cache
retirement to the world owner. Runtime keeps rescue before the step and contact
snapshots after it. The portal pose anchor still has one member. State fields,
wire keys, deferred visibility, and active invocation counts stay the same.
Guards: 158 world, 597 actor, 262 runtime, 156 app, 58 Smash, and five profile
tests; 35 workspace policies and 53 absence contracts. The app compiles and the
kernel graph remains a DAG. Android performance was not measured.
Do not restore a body-owned global platform driver or gate lift contact by
controller kind.

`495734ea2`: room roots now keep their room identity when a second room opens.
Both room-spec readers recognize roots; resident stamps and the single-room
fallback keep their priority. Guards: 339 shared, 159 world, and 63 app tests;
the app compiles. No state, wire key, or system invocation changes.

### K21 — Trail observation and drawing owners

`0fd8c7757`: observation captures plain trail geometry; rendering owns retained
gizmo assets, colors, room bands, caching, and retirement. One Update system
pipe replaces the kernel's drawing system. Trail creation, advancement,
messages, and simulation ordering stay the same.
Guards: 595 actor, 481 render, 140 observation, 15 app, and five profile tests;
35 workspace policies and 53 absence contracts. The app compiles and the
kernel graph remains a DAG. Android performance was not measured.
Do not restore render assets or drawing systems in the body kernel.

### K22 — Runtime-owned fall rescue

`66317caff`: runtime owns room-policy rescue, boss checks, and carrier creation.
Rescue stays after mount steering and before platform advancement. Body contact
snapshots stay after the world step. Empty avatar compatibility modules and the
old rescue export are deleted. State, wire keys, and invocation counts stay the
same. The causal fixture now includes the owner of its observers.
Guards: 595 actor, 270 runtime, six causal, 35 app, and five profile tests;
35 workspace policies and 53 absence contracts. The app compiles and the
kernel graph remains a DAG. No Android performance measurement was made.
Do not move room rescue policy back into the body kernel.

### K23 — Observation-owned default cast

`90e68b736`: observation initializes `FramedCast`; session/match code publishes
it for camera framing. Possession initializes control state only. State, wire
keys, and scheduling stay the same.
Guards: 595 actor and 140 observation tests, five profiles, 35 workspace
policies, and 53 absence contracts. The app compiles.
Do not make possession setup a prerequisite for camera cast projection.

### K24 — Per-participant possession

`7afeb3a0a`: one possession record held only the primary seat. Per-seat records
now use one trigger, custody projection, and driving projection. Contested
targets grant neither seat. Each claim names its home body's stable identity.
Rollback schema v385 maps every record and probes its seat, timer, and input edge.
The probe is for localization; this remains clone snapshot state.
Guards: 603 actor, 270 runtime, 112 app, 10 session teardown, eight Mary-O/Sanic,
and 52 Smash tests; six app and six Smash tests stay ignored. All 35
workspace policies and 53 absence contracts pass. The default-feature app
compiles with no warnings. The full workspace feature union was not run.
Do not restore a separate primary-seat possession path. A4 still owes its
remaining body-tick acceptance arm.

### K25 — Shared boss body integration

`d23d6f0b9`: bosses called the common integrator from a separate scheduled
loop. Boss bodies now use `integrate_sim_bodies`, with optional envelope,
tuning, contacts, and movement cues. Boss phase maintenance follows integration.
The old file and public entry are deleted: one fewer active ECS invocation.
State, rollback registrations, and wire rows stay the same.
Guards: 603 actor, 270 runtime, 145 app, and 52 Smash tests; six app and six
Smash tests stay ignored. All 35 workspace policies and 53 absence contracts
pass. The default-feature app compiles with no warnings. The kernel graph is
a DAG and the resource-writer census stays at 131 adjudicated resources.
The full feature union and Android performance were not measured.
Do not restore a boss-only motion loop.

### K26 — Shared body reaction clocks

`bf2e60c8f`: home, actor, and boss reactions used different clocks and phases.
One early body phase now advances recovery in proper time and hitstop expiry
in unscaled simulation time. Observer requests follow the driven body. The
actor-only maintenance phase is deleted: one fewer active ECS invocation.
Schema v386 records the new semantics; codec layouts stay the same.
Guards: 2,005 owner library tests, 146 app tests, eight Mary-O/Sanic host tests,
70 Smash tests, 35 workspace policies, and 53 absence contracts pass. Six app
and six Smash tests remain ignored. The default app compiles with no warnings;
the kernel graph is a DAG. The full feature union was not run.
Do not restore role-specific reaction clocks or use home identity to select the observer.

### K27 — Shared body control acceptance

`956ae7472`: the action gate used the primary observation projection to grant
room techniques. Each driven body now reads its own room rule. Control owns
hold blanking, commanded movement, and action acceptance. Rule readers borrow
values; stored projections make their own copies. Invocations, phases, state
rows, and wire layouts stay the same.
Guards: 3,153 owner library tests, 47 app tests, 14 mode-scope tests, 17
Mary-O/Sanic host tests, 70 Smash tests, and 35 workspace policies pass. Six
owner tests and six Smash tests remain ignored. The default app compiles with
no warnings; the kernel graph is a DAG. The full feature union was not run.
Sprite-publication census and fixture-lockfile findings remain with that work.
Do not use the primary observation projection to grant body techniques.

### K28 — Body-owned movement output

`e3f9f237b`: body integration depended on avatar-owned movement facts and public
per-body helpers. `body::BodyFrameOutput` and private integration helpers now
own those facts. Avatar policy consumes the output. The app forwarding export
is deleted. Required components, the derived rollback key, phases and codec
layouts stay the same. No systems were added.
Guards: 61 selected library tests, 31 app tests, 11 power-loop tests, 12 Mary-O
host tests, 70 Smash tests and 35 workspace policies pass. Six Smash tests stay
ignored. The default app compiles with no warnings; the kernel graph is a DAG.
The sprite-publication fixture lockfile remains stale (52 of 53 absence
contracts pass). The full feature union and Android performance were not run.
Do not expose a home-body helper as a second public movement route.

### K29 — Control-owned participant input

`f941e5547`: generic participant input and held-frame history lived under
avatar. Control now owns their producer, state and ordering set. Runtime
registers the same persona, input and body-mode phases directly. The avatar
forwarding installer is deleted. State fields, clone probe, key, checksum and
phase edges stay the same. No systems were added.
Guards: 16 selected owner tests, six real-app control and rollback tests, and
35 workspace policies pass. The default app compiles with no warnings; codec
layouts stay the same and the kernel graph is a DAG. The full feature union
and Android performance were not run.
Do not make the primary home identity the owner of participant input history.

### K30 — Mana policy at its ability owner

`d694e20fc`: avatar held the named Mana rate and refill system. The ability
owner now provides `ManaRegen` and `regenerate_driven_mana` beside its pool and
spending API. The old systems module and exports are deleted. Selection, rate,
phase, resource gate and bank rollback identity stay the same. No systems or
dependencies were added.
Guards: 17 selected owner tests, eight app tests, 17 Mary-O/Sanic host tests,
70 Smash tests and 35 workspace policies pass. Six Smash tests stay ignored.
The default app compiles with no warnings; codec layouts stay the same and
the kernel graph is a DAG. The known sprite fixture lockfile remains stale.
The full feature union and Android performance were not run.
Do not put named ability resource policy back in the body kernel.

### K31 — Conversation-owned entry selection

`752655df3`: the body kernel selected dialogue entry IDs through a temporary
combat-domain request. Conversation opening now owns the selector. Interaction
and content validation call that rule. The old helper, forwarding export and
request carrier are deleted. Blank and authored IDs keep their meaning. The
selector borrows its result; it adds no system, dependency or rollback state.
Guards: 37 selected owner tests, four app conversation tests and all
35 workspace policies pass. The default app compiles with no warnings; codec
layouts stay the same and the kernel graph is a DAG. The global doc-link check
reports five links in untouched docs. The full feature union and Android
performance were not run. Keep dialogue entry policy at the conversation owner.

### K32 — Projectile-combat clash integration

`513679f29`: projectile combat takes the kernel's melee/projectile collector
and ordered arbitration/rebound unit. Combat keeps the pure rule. No new systems,
dependencies or state. Guards: 28 owner, six app, 70 Smash and 35 policy tests pass;
six tests ignored; default app and strict owner rustdoc pass. The old module is gone.
Keep this adapter outside the kernel. Full feature union and Android were not run.

### Constrained-body release acceptance

`bbcdf67ff`: actor-baseline queries skipped home riders. Both release paths now
release those bodies and keep their size. The app guard covers tentative motion,
timed dismount and later rollback. Guards: 14 mount, 14 app, 70 Smash and 35 policy
tests; six Smash diagnostics ignored; default app compiles with no warnings.
Keep release independent of controller kind. No production systems or state added.

### K33 — Runtime identity diagnostics

`6efa03591`: damageable-body identity observation moved from the kernel to runtime.
Both identity censuses remain. The old module and forwarding API are gone.
Guards: 14 runtime, five app and 35 policy tests; default app and strict rustdoc pass.
Schedule, gates, state lifetime and codec shapes stay the same. Keep diagnostics
outside body execution. Full feature union and Android were not run.

### K34 — Runtime repertoire composition

`ddaf2311f`: runtime now schedules repertoire reconciliation; the kernel wrapper is gone.
The cut keeps the system count, phase, gates, state and dependencies.
Guards at that cut: three kernel, six app, 58 Smash and 35 policy tests; default app passes.
Six diagnostics ignored. At merge `271bf1821`, three kernel guards pass; app checks remain.
Keep global composition at runtime. Full feature union and Android were not run.

### Migration acceptance for each packet

Capture a before/after record with: removed file paths and edges, resolved
imports, API callers, type and state writers, construction/defaults,
rollback and checksum, ownership of plugin installation, phase order and
`apply_deferred` barriers, run conditions, teardown, tests and external
consumers. Re-run the graph and declared Cargo closure. A completed crate
move adds **zero active ECS invocations** unless a measured behavior need
justifies one. Keep normal and minimal headless consumer tests and compare
Android numbers if a hot system moved. See the
[system fusion plan](system-composition-audit.md#s7--consolidate-coherent-active-ecs-systems-proposed-2026-10-20).

## Index follow-up

The October 20 index is enough to select K1 and K2 and identify likely
owners for K3/K4. It does not yet prove the reason for each source dependency.
Extend the existing `scripts/module_graph.py` / agent index when a packet
needs it. A useful edge record has source and target module and symbol,
file and line, reference kind (`use`, type, function call, registration),
Cargo feature or cfg condition, and the active schedule/set for installed
systems. Human-reviewed edge disposition remains in this plan, not in a
machine classifier. Use the generation stamp to reject stale reports.

Do not write a new full-repository parser to start K1. Confirm its three
references and production consumer in source, implement the narrow move,
then use the result to decide if a durable edge report is needed. The
current `measure_kernel_module_graph.py` is a text-reference graph, and
`module_graph.py` only resolves `use` declarations; neither alone certifies
a Cargo carve.

## Packet receipt

Record the base and new head, the owner and moved operations, removed source
paths, the new dependency direction, state and registration changes, preserved
phase visibility, the tests that ran and the remaining limits. Record SCC
changes as diagnostics without a target.

## Exit

D33 exits when the residual crate owns only the body, control and action kernel,
every other responsibility has an explicit owner and no consumer needs the
crate's internal topology. A retained cycle has a stated shared invariant.
Independent capability profiles are a separate C2/SDK exit.

## Post-carve safety map

A carve that moves one of these owner files must update the corresponding absence
contract in the same commit. This table belongs here rather than in `queue.md`
because it is durable decomposition doctrine.

| If your carve moves… | Update these absence contracts |
|---|---|
| `crates/ambition_combat/src/moveset/mod.rs` | `ending-a-move-goes-through-the-one-teardown-path` |
| `crates/ambition_characters/src/brain/fighter`, `crates/ambition_characters/src/brain/state_machine/mod.rs`, `crates/ambition_characters/src/snapshot_impls.rs`, or `crates/ambition_characters/src/brain/mod.rs` | `the-generic-brain-does-not-grow-new-platform-fighter-edges` |
| `crates/ambition_platformer2d_session/src/match_activation.rs` or `game/ambition_app/src/app/versus.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
| `crates/ambition_platformer2d_runtime/src/input.rs` or `game/ambition_app/src/dev/rollback_observatory.rs` | `the-seat-topology-has-one-engine-side-creator` |
| `game/ambition_app/src/app/versus.rs` or `game/ambition_demo_smash/src/lib.rs` | `the-global-roster-is-retired-only-by-its-owner` |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py` or `tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py` | `the-worlds-path-is-confined-to-ldtk-paths` |
| `crates/ambition_characters/src/smash_fighter/` | `the-smash-fighter-facet-is-read-only-by-its-owner` |
| `crates/ambition_platformer2d_assets/src/materialization/presentation.rs` | `the-provider-resolver-is-confined-to-one-file` |
| `crates/ambition_characters/src/actor/character_catalog/registry.rs`, `crates/ambition_platformer2d_assets/src/materialization/audit.rs`, or `game/ambition_app/tests/app_local_catalog_composition.rs` | `the-catalog-owners-map-is-not-a-provider-authority` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_platformer2d_actor_monolith/src/avatar/character_execution.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-axis-tuning-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/character_execution.rs` or `crates/ambition_platformer2d_actor_monolith/src/avatar/mod.rs` | `the-movement-tuning-resolver-is-confined-to-one-file` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs`, or `crates/ambition_platformer2d_publication/src/stage.rs` | `only-the-candidate-builder-hides-a-root` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_publication/src/transaction.rs`, or `crates/ambition_platformer2d_provider/src/lifecycle.rs` | `only-the-publication-authority-publishes-a-candidate` |
| `crates/ambition_platformer2d_provider/src/authoring.rs` or `crates/ambition_game_shell/` | `one-registrar-installs-the-session-teardown` |
| `crates/ambition_platformer2d_world/src/collision.rs`, `crates/ambition_platformer2d_world/src/platforms/mod.rs`, or `game/ambition_demo_mary_o/src/bricks.rs` | `only-the-collision-world-composes-collision` |
