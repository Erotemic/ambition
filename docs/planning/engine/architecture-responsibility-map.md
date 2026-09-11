# Current responsibilities and target logical authorities

**Source baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
This is the semantic map behind the [reassessment](architecture-reassessment.md).
It is a target for incremental implementation, not the current package diagram.
Package/line inventories and scope limits are in
[review coverage](architecture-review-coverage.md).

## How to read this map

Current ownership is inferred from state, mutation, callers and lifecycle, not
from a crate's name. An ECS component's declaration site alone does not prove
that site owns the capability. Each target authority needs an identifiable writer
or coordinator, an explicit lifetime and an output other domains can consume
without importing its policy.

The package boundary follows later. Start with modules and restricted visibility
inside existing crates; use a crate when a real consumer, independent install,
compile closure or build measurement justifies it. Internal cycles within one
coherent authority are acceptable. No engine-wide cycle is justified merely by
putting everything into one crate.

## Current major containers

Paths below are relative to `crates/` unless stated otherwise. Each row describes
source responsibility, not a certification of architectural completeness.

| Container | State, behavior and current authority | Important consumers / invariant | Target disposition |
| --- | --- | --- | --- |
| `ambition_platformer2d_actor_monolith` | Live actor queries; control and possession integration; per-body execution; room objects; item/persistence adapters; construction; session startup and reconstitution; sprite/materialization integration | Runtime and facade install its plugins; actor update, damage and custody share live body state | Retain a coherent actor execution/control kernel; move lifecycle, world-object and preparation adapters by authority, not by directory size |
| `ambition_combat` | Strike geometry and events; reactions/capture; moveset execution; brain execution; stocks/death rules; breakables/falling chests/path motion; worn/held equipment; hit camera effects, banners and broad tuning | Consumers include damage, characters' prepared schemas, actor features, bosses and demos; one hit must not produce duplicate reactions | Keep contact/reaction and closely coupled action mechanics; separate intent producers, destructibles, ruleset outcomes and presentation. A new crate called combat is not needed to establish these internal owners |
| `ambition_platformer2d_core` | A large movement/contact/motion-model implementation plus geometry, identity/protocol and timing vocabulary | Body and world users rely on compatible geometry, transit, time and state | Keep platformer body motion cohesive; separate small general foundations only for actual consumers. Do not call platformer motion the universal engine kernel |
| `ambition_characters` | Actor catalogs/runtime vocabulary; brain policy/schema; prepared definitions; equipment/action schemes; moveset and technique authoring, including many `smash_*` schemas | Body seed, spawn, combat, content providers and character materialization consume it | Distinguish prepared character/action definition, live actor execution vocabulary and intent policy. Reusable mechanics are not game content merely because their name mentions Smash |
| `ambition_platformer2d_runtime` | Schedule/host coordination, input/rollback integration, prepared-content identity, external effects, room loading and confirmed commit | Owns real state transitions, not just plugin wiring | Split logical lifecycle coordinator from host assembly and backend adapter. Do not relocate all inconvenient gameplay here |
| `ambition_platformer2d_world` | Room definitions/set, geometry/world query support, moving-platform and feature-overlay related integration | Construction and simulation query an active world; some inputs are Bevy ECS state | Separate spatial definition/query authority from content lowering and active-session selection; do not claim the whole current crate is backend-neutral |
| `ambition_interaction` | Authoring/value vocabulary for chests, pickups, breakables, switches and interactions | Spawn lowering copies values into ECS; some fields have no runtime consumer (F4) | Vocabulary needs a semantic owner per object family and a support contract. Avoid an interaction registry that owns all world gameplay |
| `ambition_projectiles` | Projectile models/specifications, motion/world-hit policy and supporting vocabulary | The monolith still performs significant spawning/contact integration | Keep projectile lifetime/travel policy together; consume common geometry/contact outcomes rather than boss/breakable families |
| `ambition_platformer2d_actor_spawn` | Construction requests, spawn routines, body/brain builders, spawn-time policy | Corrected consumers use builders; no live actor query authority | Preserve the construction-only contract and its six Python guard tests; do not use it as the lower home for actor mutation |
| `ambition_world_items` | World-item motion, touch collection and their scheduled installation | Consumes body/custody settlement and item acquisition paths | Own world presence/collection, independent of hold/use/throw when that capability is absent; reward policy is a consumer |
| `ambition_held_items` | Release/pickup/use/throw/settlement/physics/residency phases | Coupled to body custody and held-object relation state | Own held-object lifecycle with relation writers and teardown; do not make checkpoint restoration a prerequisite |
| `ambition_platformer2d_shared_tangle` | IDs, scopes, markers, construction protocols, prepared logic and miscellaneous shared state | Very broad dependency reach; typed construction and neutral scope vocabulary have real customers | Retain the warning label. Move only proven semantic protocols to named owners; no new catch-all slot, command enum or shared context |
| `ambition_platformer2d` | Public SDK/app builders, curated namespaces, legacy module mirrors and feature wiring | External games and all demos; many mandatory internal dependencies | Keep one ergonomic facade; remove internal mirrors as consumers migrate and separately fix optional dependency closure |
| `ambition_damage` | Victim/body hit resolution plus boss/projectile, persistence and effect integration | Implements a central hit-consumption path; tests dominate its physical size | Separate reaction authority from death/reward and presentation adapters; do not duplicate contact resolution in a second combat facade |
| `ambition_boss_encounter` | Encounter patterns, phase/state machines, authored geometry, ECS runtime, sprites and rewards | Bosses, actor feature damage and content providers | Distinguish encounter orchestration and authored assets from body/action execution. Preserve encounter/cutscene separation and characterize existing boss paths before convergence |
| `ambition_body_seed` / `ambition_mount` / `ambition_match` | Shared body construction; mount relations; match/seat/settlement semantics, respectively | One-body construction and control, plus ruleset/match lifecycle | These are useful bounded directions, but each remains subject to its own writer, scope and absence tests |

### Source entry points for corroboration

The following are sufficient starting points, not a claim to enumerate every
symbol in these packages:

* `crates/ambition_combat/src/lib.rs`, `crates/ambition_combat/src/rollback_registration.rs`,
  `crates/ambition_combat/src/breakables.rs`, `crates/ambition_combat/src/moveset/mod.rs`,
  `crates/ambition_combat/src/brain/mod.rs`.
* `crates/ambition_characters/src/lib.rs`, `crates/ambition_characters/src/prepared.rs`,
  `crates/ambition_characters/src/actor/mod.rs`,
  `crates/ambition_entity_catalog/src/authoring.rs` (moved out of
  `ambition_characters` by fast-iteration I1, 2026-09-11: authoring a move is a
  pure value computation and must not link Bevy).
* `crates/ambition_platformer2d_runtime/src/lib.rs`,
  `crates/ambition_platformer2d_runtime/src/content_identity.rs`,
  `crates/ambition_platformer2d_runtime/src/external_effects.rs`,
  `crates/ambition_platformer2d_runtime/src/room_transition/commit.rs`.
* `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`,
  `crates/ambition_platformer2d_shared_tangle/src/sim_id.rs`,
  `crates/ambition_platformer2d/Cargo.toml`,
  `crates/ambition_platformer2d_host/Cargo.toml`.

## Residual monolith regions

All paths in this table are under
`crates/ambition_platformer2d_actor_monolith/src/`.

| Region / entry point | Real responsibility | Boundary decision and evidence needed |
| --- | --- | --- |
| `actor_clusters.rs` | Live query/mutation views for body execution | Keep with live actor authority; never move to spawning or schema to reduce an import |
| `features/ecs/mod.rs` | Shared actor and room-object integration, not optional engine feature toggles | Dissolve as a category. Separate by rows below rather than extract a features crate |
| `features/ecs/actors/update.rs` | Live decision/action/movement execution and adapters to shared body integration | Retain body execution; audit human/brain/home-body filters before declaring one-path convergence |
| `features/ecs/damage` and `features/ecs/damage_predicates.rs` | Family-specific hit admission and application, including bosses and destructibles | A2 aligns geometry and contact identity; A5 then moves destructible reaction with its state |
| `features/ecs/target_volumes.rs` | Publication of simulation hurt/damageable geometry | Keep one authoritative interpretation per tick; renderer quality must not alter it |
| `features/feature_bundles.rs`, `features/ecs/spawn_static.rs` | Room-object construction and authoring-to-runtime projection | Domain construction adapter, distinct from actor spawn and from runtime object behavior |
| `features/ecs/fighter_ladder.rs`, `features/ecs/dismounted_rider.rs` | Live policy projection/rebuild of existing actors | Retained by the spawn correction; later move with the relevant live owner, not builders |
| `items` | Physical/held item integration plus persistence, horizons and plugin wiring | Custody, collection and persistent accounting need separate ownership; preserve occurrence accounting |
| `items/pickup/mod.rs` | Held/collection installation and misplaced checkpoint startup installation | A1 removes checkpoint restoration from item installation; do not move all item code with it |
| `session/lifecycle_commit.rs` | First-admitted pending room lifecycle intent and subject/retraction rules | Session/lifecycle authority; not dependency-neutral foundation |
| `session` | Setup, active content/session integration, save restoration, teardown and horizon coordination | Retain coordinator; domain-specific cleanup stays with domain owners under named boundaries |
| `world/placements.rs` | Character/object-aware authored placement lowering context and provider bridge | A3 moves the bridge toward construction; room definitions remain in world |
| Re-exported `ambition_platformer2d_world` collision service, integrated through `world/mod.rs` | Active-room collision assembly with dynamic overlays and specialized query views | Spatial read authority; cache only against explicit generations and measured costs |
| `world` (remaining staging/replay roads) | Mixed world query, room preparation and lifecycle adapters | Inventory each operation; room activation is not the same as immutable room geometry |
| `construction` | Typed assembly of actors, objects, summons and encounter-specific parts | The actor label overstates coherence; preserve typed lanes and separate adapters when a real domain warrants it |
| `projectile/systems.rs` | Projectile body stepping, contact selection/consumption and world collision | A2, then projectile owner extraction. Do not move this full integration function into combat just to close a cycle |
| `abilities/traversal/possession.rs` and related control consumers | Eligibility policy plus accepted control/custody projection | Co-locate the relation authority; ability-specific acquisition rules remain optional policy |
| `abilities` (remaining) | Several independent abilities and summoning/traversal adapters | Split by actual customer. No universal ability service needed for a single summoning call |
| `control/authority.rs`, `control/input_systems.rs` | Accepted driver projection, participant intents, possession integration, stray animation advancement | Keep accepted control with possession relation; move animation advancement to its actual frame owner |
| `shrine.rs` | Rest/heal/checkpoint capture trigger plus startup/reset restoration | A1 separates content behavior from lifecycle restoration; keep heal/save semantics unchanged |
| `character_runtime` | Prepared character loading/materialization, sprite residency and live match activation | Separate prepared definition, device preparation and activation; not an actor simulation kernel |
| `avatar` | Home/avatar creation policy and shared movement integration | Shared motor execution belongs with bodies; starting-character/home-avatar policy belongs with session/game adapters |
| `schedule`, `checkpoint_horizon.rs`, `body_custody.rs` | Cross-owner ordering and custody/checkpoint integration | Preserve phase ancestry and deferred visibility. Integration can stay together without acquiring every algorithm |
| `assets` and `character_sprites` | Coupled visual preparation and runtime sprite integration | Separate two-module SCC; shared asset-preparation authority may legitimately move together. Not a forced early carve |

## Target dependency shape

```text
host / game composition
  selects providers, capabilities, profiles, backends and lifecycle policy
       |
       +--> domain-owned preparation adapters --> immutable prepared definitions
       +--> domain installers --> deterministic simulation authorities
       +--> lifecycle coordinator --> explicit domain checkpoint/retire operations
       +--> presentation adapters <-- simulation read models / confirmed effects

simulation authority A --> lower semantic vocabulary or authority B
  is a normal direct dependency when A genuinely depends on B

spatial/motion/math/time primitives
  do not depend on game content, named bosses, UI, device residency or the host
```

This remains 2D-first and platformer-capable. General engine services such as
asset identity, preparation diagnostics, input identity, lifecycle scoping and
inspection should not require platformer body motion. A future non-platformer or
3D game needs an explicit customer and its own simulation/world implementation;
do not parameterize all present code over spatial dimension to advertise parity.

## Target authority contracts

Names below describe logical responsibilities. They are not approved crate names.
Each capability must publish the actual supported configuration, not imply that
all listed combinations work today.

### T1. Simulation primitives and tick protocol

**Owns:** geometric values and deterministic primitive operations, tick/frame
identity, explicit time domains, stable identity vocabulary and confirmed-frame
protocol. **Does not own:** active session selection, room transitions, stocks,
input-device policy or particular movement tuning.

**Depends on:** narrow mathematical/protocol libraries, not gameplay registries.
**Exposes:** typed values and pure operations, with documented units and ordering.
**Class:** reusable infrastructure. **Lifetime:** immutable values or host-owned
clock state; distinguish simulation tick, presentation time and confirmed frame.
**Witness:** deterministic operations and frame-domain misuse tests; no game or
render dependency in a consumer that only needs geometry/timing.

### T2. Platformer body motion and contacts

**Owns:** body kinematics, motion model, contacts, attachment reconciliation,
transit and collision response for the supported platformer world.
**Does not own:** player identity, brain strategy, score, save files or animation
assets. **Depends on:** T1 and spatial query contracts.

**Exposes:** accepted movement/control inputs and body state/contact results.
**Class:** reusable genre capability, not universal engine foundation.
**Lifetime:** live body; rollback and reconstruction include contact/attachment
state. **Witness:** same body under human, brain and remote control traverses the
same integration path; teleport, launch, portal and mount detachment reconcile
contacts instead of writing position through a second authority.

### T3. Accepted control and custody relations

**Owns:** accepted participant-to-body driver relation and invariant-preserving
handoff; coordinates body claims with mount/possession and held relations.
**Does not own:** raw device bindings, brain policy, ability eligibility, camera
ownership or persistent entitlement. **Depends on:** body identity and participant
identity, narrow claim/result vocabulary.

**Exposes:** accepted driver/acting-participant facts and explicit handoff results.
**Class:** reusable simulation authority with game policy above it.
**Lifetime:** participant/session and body occurrence; retraction on disappearance
is part of the contract. **Witness:** competing claims have deterministic results;
release/re-entry removes stale drivers; a view changing target does not change
control; no missing optional possession plugin requires a dummy resource.

### T4. Action execution and victim reactions

**Owns:** action instance progression, cancellation/teardown, action-local state,
authored attack/hurt geometry publication and application of accepted reactions.
**Does not own:** strategic action selection, stock rules, world residency,
sprite-quality selection or save persistence. **Depends on:** body execution,
prepared action definitions and contact facts.

**Exposes:** a bounded action menu, accepted action starts/cancels, reaction and
execution facts. **Class:** reusable genre/gameplay capability.
**Lifetime:** action instance/body; rollback includes in-flight action state.
**Witness:** exactly one teardown path, common action semantics for player and
CPU, missing authored action fails validation rather than selecting an arbitrary
fallback. Closely coupled schema/executor modules may share a package.

### T5. Projectile travel and contact resolution

**Owns:** projectile sequence/lifetime, path segments, world-hit policy, reflection,
returning/piercing behavior and selection of physical contact outcomes.
**Does not own:** boss phases, chest rewards, family-string dispatch or stock loss.
**Depends on:** geometry, spatial queries and simulation target geometry/identity.

**Exposes:** resolved contacts and projectile state; reaction consumers determine
damage effects. **Class:** reusable simulation capability.
**Lifetime:** projectile occurrence plus any explicit already-hit set; stable
ordering is independent of ECS allocation. **Witness:** nearest blocking contact,
finite-size sweep, authored empty hurt geometry, reflection ownership transfer,
return behavior and deterministic tie handling. These are staged requirements,
not a claim that all are presently implemented.

### T6. Destructible and interactive world objects

**Owns:** a destructible's health/broken/respawn/collision state, accepted damage
and break transitions; separately, mechanism state for switches/doors/platforms
where it forms a coherent machine. **Does not own:** every item, NPC, checkpoint,
encounter, camera effect or arbitrary object called a feature.
**Depends on:** spatial/body/contact primitives and domain-authored definitions.

**Exposes:** simulation geometry, interaction affordances, broken/state-change
facts and scoped lifecycle operations. **Class:** reusable object capabilities;
particular shrine behavior, reward tables and named objects are content/adapters.
**Lifetime:** live world-object occurrence, with explicit attempt/save policy.
**Witness:** projectile and melee see the same destructible shape and transition;
respawn re-enables collision once; pure standing geometry does not become a hurt
volume. Whether all bodies may break a stand is a product rule, not a refactor.

### T7. Physical items, custody and accounting

**Owns:** physical occurrence lifecycle, world collection, custody transfers,
held use/throw and reconciliation of physical identity with inventory accounting.
These are related subauthorities, not a requirement for one indivisible plugin.
**Does not own:** checkpoint startup routing, boss reward policy or UI navigation.
**Depends on:** body/occurrence identity and explicit persistence milestones.

**Exposes:** accepted acquire/release/use outcomes, occurrence records and read-only
inventory/held state. **Class:** reusable gameplay capability with game accounting
policy. **Lifetime:** occurrence versus durable inventory entry must be explicit;
checkpoint capture waits for custody settlement. **Witness:** collection without
held-item installation; no duplication/loss after throw, death, checkpoint or
room retirement; persisted held weapons remain per-item rather than entitlements.

### T8. Spatial world and residency

**Owns:** immutable room/world geometry, live-instance spatial membership,
dynamic collision contributions and query views; residency owns loaded/active
instance sets and their activation/retirement protocol.
**Does not own:** character catalogs, sprite preparation, checkpoint choice or
participant camera targeting. **Depends on:** geometry, prepared world definitions
and scoped instance identity when multiple live instances are supported.

**Exposes:** domain-appropriate collision, hostable-surface and observation views,
not a bag of Boolean query switches with inconsistent semantics.
**Class:** reusable world service. **Lifetime:** content revision, live room
instance and query epoch are distinct. **Witness:** overlays invalidate the right
view; inactive candidates cannot collide; two copies of one room do not alias
once multi-instance support is introduced. Current single-active-room behavior
remains supported without forcing every game to stream.

### T9. Lifecycle and reconstruction coordinator

**Owns:** session generation, admitted lifecycle intent, confirmation/loading/
construction/activation state machine, checkpoint startup/reset routing and
retirement orchestration. **Does not own:** item internals, character AI,
shrine healing or a universal command interpreter.
**Depends on:** typed construction, scope/content identity, body subject identity,
world activation and explicit domain lifecycle adapters.

**Exposes:** admission outcome, transition progress/readiness, semantic reset/
retire boundaries and completion/cancellation result when required.
**Class:** runtime coordinator with policy supplied by the game/profile.
**Lifetime:** session/transaction; confirmed transitions rebase rollback rather
than snapshotting across rooms. **Witness:** first-admitted slot policy, denied
admission remains retryable when required, subject disappearance cancels instead
of retargeting, startup/reset work without held items, no partial candidate is
published as a ready room.

### T10. Definitions, preparation and construction adapters

**Owns:** authored schema, semantic validation, stable references, prepared
immutable values and domain-specific lowering into typed construction plans.
**Does not own:** arbitrary live-world mutation, active participant selection,
device texture ownership or dynamic behavior discovery.
**Depends on:** schema-specific values and small identity/diagnostic protocols.

**Exposes:** prepared domain values, typed plans and structured diagnostics.
**Class:** reusable authoring infrastructure plus domain/game adapters.
**Lifetime:** content revision; running simulations hold an explicit revision.
**Witness:** invalid references/unsupported semantics fail before publication;
provider conflict does not partially replace content; a no-render headless
preparation path does not require textures. Trusted raw-Commands recipes retain
the explicitly limited transaction guarantee in F6.

### T11. Observation and intent producers

**Owns:** perception budget, attention, memory and decision policy for each agent;
input adapters own translation of device/remote actions into semantic intents.
**Does not own:** body integration, canonical world mutation or changing action
semantics to compensate for selection deficiencies.
**Depends on:** bounded observation views and public action capability facts.

**Exposes:** tick-stamped intents and inspectable decision diagnostics.
**Class:** optional reusable policy capability plus game-specific behavior.
**Lifetime:** actor/participant memory with explicit rollback/determinism rules.
**Witness:** brain reads only authorized observations, deterministic AI never
waits on a model/network call, absence of an authored difficulty ladder is
reported rather than disguised as a valid level.

### T12. Ruleset outcomes and game content

**Owns:** stock/round/match outcome policy, score/reward semantics, named character
content, boss designs, shrine rest behavior and story-specific orchestration.
**Does not own:** alternate body physics or family-specific collision kernels.
**Depends on:** reusable gameplay authorities and their outcomes.

**Exposes:** authored catalogs, scoped rules and bounded domain requests.
**Class:** game content/ruleset, with reusable match bookkeeping below it where
supported. **Lifetime:** match/attempt/session/save as explicitly selected.
**Witness:** health and versus modes share body/action mechanics while differing
in outcome policy; rewards cannot be duplicated by repeated presentation or
speculative rollback events. Encounters and cutscenes remain distinct domains.

### T13. Presentation, assets and external effects

**Owns:** rendering/animation materialization, body-owned drawable geometry,
per-view composition, audio/VFX delivery, asset preparation/device residency and
confirmed external-effect delivery. These are separate services connected by
explicit facts, not one presentation crate mandate.
**Does not own:** simulation damage geometry, stock state, authoritative input
or checkpoint admission. **Depends on:** prepared visual/audio data and stable
simulation read models; effects crossing confirmation use the existing journal.

**Exposes:** views and diagnostics, resource readiness and presentation controls.
**Class:** optional engine services plus game styling.
**Lifetime:** content/device/view/session scopes; externally delivered effects
must be idempotent at their real sink when retried across process boundaries.
**Witness:** changing resolution/quality does not change replay state; missing
camera readiness is represented as missing, not a plausible default; portal
composition runs after body-owned drawable geometry is finalized.

### T14. Host, capability composition and public SDK

**Owns:** explicit provider selection, profile prerequisites, backend/window/input
selection, cross-capability ordering and ergonomic public entry points.
**Does not own:** body algorithms, private world-object behavior, a global
service locator or automatic discovery of arbitrary installed plugins.
**Depends on:** selected capability installers and their public phases/contracts.

**Exposes:** a small supported set of app profiles, explicit advanced composition,
inspection and failure diagnostics. **Class:** host/composition/API.
**Lifetime:** app/session install and teardown, with named re-entry semantics.
**Witness:** external minimal headless, windowed platformer and richer combat
profiles run without source-tree privilege; manifest closure and runtime absence
are measured separately. Default/full composition is convenience, not mandatory
engine foundation.

## Ambiguous domains resolved

### Breakables

The authored breakable model, combat components/runtime, monolith hit consumer,
projectile family probe, world placement and simulation view currently describe
pieces of one destructible object. Co-locate the object's state and transitions
under T6. Keep authoring adapters at T10, projectile travel at T5, generic contact
vocabulary at the T4/T5 boundary and view extraction at T13. Do not move everything
into `interaction` merely because its schema already lives there. First unify
contact geometry (A2), then relocate destructible state and writers (A5).

### Shrines

A shrine combines T12 rest behavior, T7/T9 checkpoint capture ordering and T9
session restoration. A1 separates restoration now. A reusable rest-point
capability may later expose heal/capture requests with game-selected policy;
there is no need to promote the existing shrine implementation wholesale into
an engine foundation.

### Combat, characters and core

A component used by combat is not necessarily combat authority. Banner/camera
cues are presentation; stocks are ruleset outcomes; falling chests/path motion
are world-object behavior; held state is custody; AI strategy is an intent
producer. Conversely, move-local capture/charge/launch primitives can be coherent
parts of action execution even when their first customers are Smash fighters.

Character definitions and live execution share a real protocol but have different
lifetimes and dependency needs. Split when that distinction removes knowledge or
unnecessary asset/host dependencies, not merely because one side is called data.
Platformer movement is a coherent specialized subsystem within core. Extracting
each movement structure into its own microcrate would make that subsystem harder
to understand without providing independent capability selection.

## Naming policy

`actor_monolith` and `shared_tangle` are useful warning labels until their exits
are met. `features` should disappear as an owner category, not become a polished
crate. `runtime`, `characters`, `combat`, `core`, and facade `actors` must not be
used as evidence of coherent boundaries. New names describe a proven authority
such as session lifecycle or body motion; the packet must first show its state,
writers, lifetime and consumers. No global rename is authorized by this map.
