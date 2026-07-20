# Closeout review follow-ups — ownership, shipping, and measured scale

> **State (2026-07-20): OPEN gap ledger.** This file records the important
> source-backed follow-ups from the July 19–20 holistic review that were not
> already owned by a canonical subsystem plan. It is intentionally small:
> participant input belongs to [`participant-action-system.md`](participant-action-system.md),
> confirmed external effects to [`../tracks.md`](../tracks.md) Track 1,
> provider construction to
> [`immutable-content-and-transactional-construction.md`](immutable-content-and-transactional-construction.md),
> falling sand to [`falling-sand.md`](falling-sand.md), and cutscene authority
> to the design-before-code card in [`../tracks.md`](../tracks.md).
>
> **Executor:** Opus-level implementation unless a card says measure/decide.
> Land one deletion-producing vertical slice at a time. Do not create a generic
> registry, resource census, profiling framework, or compatibility facade.

## 1. Session retirement completeness — LANDED in closeout overlay

### Evidence

`crates/ambition_actors/src/session/teardown.rs::SessionScopedResources` resets
an explicit list of eight process-global live-session mirrors. At least two
other resources contain session-live mutable state and are not in that list:

- `SlotInteractionState`: buffered/double-tap/interaction gesture state keyed by
  participant slot. Simulation sleeps while the launcher owns input, so stale
  gestures can survive a retired session unless cleared explicitly.
- `SwitchActivationQueue`: a deliberately one-frame-late FIFO and registered
  rollback resource. A retirement between production and consumption can carry
  an activation into the next session.

The existing teardown test can only poison resources already named in the
`SystemParam`; it does not prove the list is complete.

### Landed fix

The closeout overlay adds both resources to the existing teardown authority and
extends the targeted poison fixture. A sequential provider/session switch can
no longer observe gestures or pending switch activations from the retired
scope.

### Remaining rule

Audit the same-session reset path only when gameplay semantics require it. For
future session-live state, prefer session-root components or scoped entities
over another process-global mirror. Do not build a global resource census.

## 2. Portal mapping convention is session authority, not a process global

### Evidence

`ambition_platformer_primitives::math` stores the active mapping convention in
`static PORTAL_MAP_ROTATION: AtomicBool`. Placement and mapping helpers read it
implicitly; `ambition_portal::tuning` mutates it from live tuning. The pure
engine-core map functions already accept an explicit `MapConvention`.

The static prevents two Apps/providers in one process from choosing independent
conventions, contaminates tests, and leaves a simulation rule outside the
session/rollback identity. `portal_reverses_facing` belongs to the same policy
family.

### Work

1. Introduce an App/session-local portal convention authority in the portal
   domain; do not put policy back into `engine_core`.
2. Thread the convention explicitly through placement, facing, input-warp, and
   transit/mapping consumers.
3. Seed it from provider/session rules, not directly from local settings.
4. Include the effective convention/facing policy in the prepared-session or
   synchronized-rules fingerprint before P2P is accepted.
5. Delete the atomic setter/getter and tests that depend on process order.

### Exit

Two independent Apps/providers can use different portal conventions in one
process, and identical synchronized session rules produce identical portal
mapping regardless of local settings.

## 3. Honest shipping and fresh-clone configurations

### Evidence

`ambition_app` defaults to `desktop_dev`, which includes developer tools,
mobile touch, RL simulation, and falling sand. Runtime/actors still have other
production dependencies on `ambition_dev_tools` after K1a, so naming a
`desktop_game` feature today would not by itself create a lean shipping build.

The supported fresh-clone route is also the full authoring-workstation setup:
system/audio packages, Rust utilities, submodules, multiple Python
environments, and regeneration of every untracked runtime asset class. That is
valid for contributors, but not a minimal build/play experience.

There is deliberately no CI initiative.

### Work

1. Continue the K1 authority removals until developer editing/inspection is an
   optional adapter rather than runtime setup authority (`SandboxDevState`,
   `EditableAbilitySet`, dev-owned schedule sets, and profiling hooks remain).
2. Only then define explicit supported app configurations:
   - desktop development;
   - desktop game/shipping;
   - headless simulation;
   - Android;
   - web.
3. Make simulation-host choice explicit in each configuration rather than a
   side effect of `dev_tools`.
4. Split local setup into:
   - build/play asset hydration and compile;
   - full authoring/regeneration workstation setup.
5. Choose the asset-hydration/distribution mechanism only when implementing
   that split; do not invent a hosted cache or GitHub workflow preemptively.
6. Periodically validate with a manual clean clone/new-machine drill.

### Exit

A fresh machine has a documented minimal command that hydrates runtime assets
and builds/plays the game without installing the full authoring stack. A
shipping-like desktop build excludes inspector/editor machinery by actual
dependency shape, not merely by feature name.

## 4. Measured runtime-scale pass

These are observed avoidable costs, but their runtime rank is unmeasured. Do
small unconditional wins first; measure before architectural optimization.

### 4.1 Cheap bounded fixes

- Cache `SnapshotSchemaFingerprint` when `RollbackRegistry` registrations
  change instead of cloning/string-dumping/hashing the full registry every
  active-session render frame. Keep the verbose dump lazy for diagnostics.
- Stop cloning `ProjectileView.visual_id: String` for every projectile every
  simulation tick; update stable identity only on creation/change or use an
  existing cheap shared/stable identifier.
- Make rich gameplay trace capture opt-in in ordinary execution after its
  predicted-vs-confirmed policy is decided. Retain a cheap anomaly trigger if
  automatic OOB capture is valuable.

Each item already has a source-backed smell entry in
`dev/journals/code_smells.md`; this card supplies ordering and exit discipline.

### 4.2 Collision composition measurement

`world_with_sandbox_solids` clones/composes authored geometry, moving
platforms, overlay solids, gates/liquids/subtractions, and portal carving. It
has many independent production call sites in body integration, actor/boss
updates, combat/damage, and both rich trace recorders.

Instrument representative authored rooms with disposable or narrowly owned
counters for:

- composed-world constructions per simulation tick/phase;
- blocks cloned, added, subtracted, and carved;
- wall-clock time by composition and major simulation phase;
- raycast candidate visits;
- surface-momentum face comparisons.

First gate/remove unnecessary trace constructions. If repeated composition is
material after that, publish one deterministic derived collision view at an
explicit phase boundary after its dynamic inputs are finalized. If candidate
scans dominate at realistic room sizes, then evaluate indexing. Do not build a
broadphase or permanent profiling subsystem from theory.

### Exit

Every retained optimization has before/after measurements from representative
authored rooms and preserves deterministic collision behavior. Temporary probes
are removed unless repeated use justifies a small maintained diagnostic seam.

## 5. Delete the dormant `GravityFlipSwitch` cluster or give it a real owner

### Evidence

The generic authored hub/C4 gravity controls are LDtk `Switch` entities handled
through the normal switch-action path. They do not use
`ambition_actors::gravity::GravityFlipSwitch`.

`GravityPlugin` explicitly does not install `gravity_flip_switch_system` because
nothing spawns the component in-game. The dormant type nevertheless retains:

- the component/system and unit test;
- sim-view extraction;
- renderer support;
- rollback registration;
- primitive documentation implying it is live.

### Work

Prefer deletion unless a near-term authored overlap-plate requirement is named:

1. Reconfirm no authored/runtime spawn or plugin installation exists.
2. Delete the component, dormant system, view/render support, rollback
   registration, and dedicated test together.
3. Keep the live generic LDtk switch/gravity-action path unchanged.

Do not synthesize a fixture solely to justify pre-paid generality.

### Exit

The repository has one live gravity-switch mechanism. If an overlap plate is
needed later, implement it as an authored feature through the normal placement
and action authorities.

## 6. Deferred provider-boundary slices: persistence and items

These are strategically important but wait until K2 provider ownership and the
single activation lifecycle are stable.

### Persistence

Reusable actor/dialog/cutscene systems currently depend on Ambition-specific
`SandboxSave`. `SandboxSaveData.version` exists, but loading does not yet form a
provider-owned migration/rejection contract suitable for another game.

When a real external provider needs persistence:

1. keep file/storage I/O reusable;
2. move Ambition's payload/schema to Ambition content/provider ownership;
3. give each provider explicit save identity/version/migration responsibility;
4. remove one reusable domain's direct `SandboxSave` dependency per slice;
5. reject unsupported future versions rather than silently accepting them.

### Items

`ambition_items::Item` is a fixed Ambition roster; catalog overrides alter
metadata for those variants but cannot define another game's item identities.
When a second provider needs items, introduce provider-stable item IDs/catalog
authority and migrate one real consumer. Do not create a type-erased universal
inventory before that need exists.

### Exit

One external provider can own a persistence payload or item identity without
editing Ambition content enums, and the slice deletes the corresponding direct
engine dependency. No generic persistence framework is built ahead of a real
consumer.

## 7. Execution order

These cards are not a new overriding wave. Apply them when adjacent work makes
them cheap:

1. dormant `GravityFlipSwitch` deletion (small convergence fix);
2. portal convention during deterministic-session-authority work;
3. cheap measured performance fixes;
4. collision measurement only when runtime scale is being investigated;
5. shipping/bootstrap after remaining dev-tool authority is optional;
6. persistence/items only with a real external-provider consumer.
