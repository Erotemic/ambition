# Engine architecture

How the engine is structured so it compiles fast, navigates well, pluginizes
idiomatically, and is reusable by another game (the four goals). The actor model
itself is [`unified-actors.md`](unified-actors.md); this is the crate/plugin shell
around it, and the **keystone refactor** that unblocks the whole thing.

> Agent-navigability is the real goal. ~150k LOC is too hard to navigate; the point
> is the right abstractions + getting NAMED content (bosses, abilities, rooms) out of
> the foundation crates into content, generalised where possible.

---

## The target stack

Three tiers, imports flowing one way:

- **Foundation** — `ambition_engine_core`, `ambition_platformer_primitives`. Pure
  spatial / kinematic / collision primitives. No gameplay, no content, no Bevy app
  concerns. Anything here must be usable by *any* platformer.
- **Runtime domains** — one crate per subsystem. Each **owns its vocabulary** (its
  components / resources / messages), **owns its authoritative state** (what it
  mutates), and exposes **local schedule sets**. A domain imports the foundation and
  talks to peer domains **via messages**, never by reaching up into the app or content.
  The target crate lineup (canonical names — use these):
  - `ambition_actor_control` — the control seam (`ActorControlFrame`, brains' output).
  - `ambition_actor_runtime` — the body: kinematics, movement, capabilities, the spine.
  - `ambition_combat_runtime` — hitboxes, damage, factions, the relational model.
  - `ambition_projectiles` — projectile bodies + lifecycle (generic; named kit is content).
  - `ambition_world_runtime` — `RoomGeometry`, the collision view, overlays, gates.
  - `ambition_encounter_runtime` — encounter entities, wave books, the scripted-beat VM.
  - `ambition_carryable_items` — held/carryable items + pickup.
  - `ambition_cutscene_runtime` — the cutscene library + schedule (render leak already closed).
  - `ambition_game_runtime` — collects the domains and maps their sets into the schedule.
- **Composition root** — `ambition_app` hosts platform / assets / windowing / the
  binaries, and is the *only* crate allowed to name both machinery and content.
  `ambition_game` (the composition-collector half of `ambition_game_runtime`) is a
  *direction* — introduce it when the app file clearly splits host-vs-composition
  jobs, not before.

**Boundary rule (enforced by the `architecture_boundaries` test):** machinery must
not import named content. If a rule needs content data to run, it lives in a content
install-seam, not in a foundation/runtime crate.

## The keystone: collapse the `Player*` / `Actor*` dual hierarchy

This is the prerequisite that unblocks crate-splitting (goals 1 + 2) and **subsumes**
the `ControlFrame`→intent and projectile source/faction work. It is the same cut as
[`unified-actors.md`](unified-actors.md) step 4; this section is the *execution
blueprint*.

### Why it's THE blocker (the dependency analysis, not taste)

A crate-extraction spike tried to pull `world` out as a clean leaf and **failed**:
`world` depends on a ~6.4k-LOC LDtk fan-out which depends on every gameplay domain. But
the real blocker is singular — **`crate::player` is a universal dependency sink: 20 of
28 non-player top-level modules import it.** That is why no clean leaf exists. Move the
shared sim-state off `Player*` onto the `Actor*` vocabulary and those **20 back-edges
dissolve**; modules like `time`, `body_mode` become extractable leaf crates and compile
time drops. *That* is why this is ranked above every other cleanup.

### The component families (the work breakdown, by bucket)

By approximate reference count: `BodyKinematics` (~35), `PlayerEntity` (~34),
`PrimaryPlayer` (~17), `PlayerCombatState` (~14), `PlayerSafetyState` (~12),
`PlayerInteractionState`, `PlayerEnvironmentContact`, `PlayerWallet`, `PlayerShieldState`,
`PlayerGroundState`, `PlayerFlightState`, `PlayerDodgeState`, `PlayerLedgeState`,
`PlayerOffense`, `PlayerBaseSize`, … Assign each to one bucket:

- **Bucket 1 — namespace-only moves** (cheap, zero behavior, byte-stable):
  already-shared types to neutral homes — `BodyKinematics`, `PlayerEntity` /
  `PrimaryPlayer`, the primary-player scratch.
- **Bucket 2 — sim-state collapses** (the real convergence): `PlayerCombatState` →
  `ActorCombatState`; the movement/ability states (`PlayerShieldState`/`Dash`/`Flight`/…)
  fold onto the shared body; `PlayerWallet`/economy onto `Actor*`. This is where less
  code actually happens.
- **Bucket 3 — keep player-only** (genuinely not shared): `PlayerHudRoot`,
  `PlayerBlinkCameraState`, `PlayerDemoCfg`, camera / aim / device-input / HUD.

### The slice plan (order matters)

- **Slice 0** — a namespace move only (Bucket 1). Zero behavior; the differential
  headless trace must be **byte-stable**. Proves the harness + the move mechanic.
- **Slices 1..k** — one sim-state family per slice (Bucket 2), ordered **low → high
  feel-risk**: (1) economy / interaction first, (2) combat state next, (3)
  **movement / ability state LAST** (the feel-sensitive `ProjectileSpawner` / shield /
  dash / flight fold). Feel-risky slices get an in-game check between them; cheap ones
  iterate fast.
- **Slice final** — bank the compile-time win: extract the now-unblocked leaf crates.

Each slice gated on *it compiles* + the differential trace; behavior may change (often
*better*) — re-baseline canary traces, don't preserve unpolished feel. Commit each
slice as a checkpoint.

## World geometry

`RoomGeometry` is **authored** and swapped at room boundaries — never mutated
mid-room. All mid-room dynamics (moving platforms, gates, portal carves, ECS solids)
compose through a **derived `CollisionWorld` view** layered on top.

> An authored `RoomGeometry` + a derived collision view is replay/RL-friendly — a
> frame's collision truth is a pure function of room id + overlay state — and naturally
> supports per-player world variants later, without a mutable monolith that tempts
> content to reach in and mutate the base.

**Write-map (who may mutate `RoomGeometry`):**

- *Boundary swaps (legitimate):* `session/reset`, `app/world_flow/room_flow`,
  `app/dev_runtime`.
- *Mid-room mutators (must move to the overlay):* `encounter/systems` (`gate_solids`,
  done), `content/intro/route_state` (`gate_solids`, done), **`content/bosses/gnu_ton`
  (subtractive carve — REMAINS; needs the overlay model extended with carve +
  climbable-region overlay)**, `falling_sand` (feature-gated, open).

**Collision-view guard (a silent-feel-regression trap):** route only the **collision**
readers through the composited view. Do NOT route render / layout / metadata readers
(`water_regions`, `spawn_point`) or projectiles (projectiles pass through platforms →
`carves_only`). A reader that suddenly sees moving-platform / carve geometry is a silent
feel regression.

## Bevy-plugin shape

Each domain is a plugin exposing four things:

1. **Owned vocabulary** — components / resources / messages native to the domain.
2. **Authoritative state** — exactly what it mutates (no other domain writes it).
3. **Local schedule sets** — a consistent rhythm: `BuildIntent → Simulate → Resolve →
   EmitFacts → ProjectPresentation`. A domain creates a *local* set when its ordering
   is internal; it maps into a global set only at the composition root.
4. **Public extension points** — content attaches systems to named slots
   (`CombatSet::ContentSpecials`, `PortalSet`, …) **without reaching into privates**.

**`ambition_portal` is the exemplar** — copy its shape to extract a domain (runtime
core crate + optional presentation crate + optional `content/adapter` package + the app
maps the schedule). Why it's the exemplar: (1) the reusable mechanic is not buried in
`gameplay_core`; (2) presentation is separate from runtime (the sim runs headless —
[`headless-verification.md`](headless-verification.md)); (3) the Ambition-specific glue
is a *visible adapter*, not pretending to be generic; (4) it exposes a local set
(`PortalSet`) rather than forcing callers to know the whole schedule; (5) its remaining
impurities (still uses `ControlFrame`, a few gameplay-core shims) are concrete adapter
migration work, **not reasons to recollapse it**.

## The reusable-engine principle

The engine is a stack of domain plugins that accept **content-installable data** —
named archetype rows, boss specs, encounter waves, audio cues, item/ability registries
— through `OnceLock`-backed **pure-function resolvers**: install once at startup,
immutable after, read from non-system code with no `World` access. **The `install_*`
fn + the `cfg(test)` fixture together already ARE the test-override seam** — no Bevy
`Resource` coupling needed.

Classify each `OnceLock` so a new one lands in the right pattern:

- **Content registries (install seams):** `BOSS_PROFILE_OVERRIDE`,
  `BOSS_SPECIAL_ANIM_KEYS`, `BOSS_ENCOUNTER_SPEC_OVERRIDE`, `ENCOUNTER_WAVE_BOOK`,
  `ENEMY_ROSTER_OVERRIDE` — install-once, immutable, read from pure helpers
  (`BossBehaviorProfile::from_data`, etc.). Content installs them; a second game
  installs its own.
- **Immutable asset caches (no install seam):** `file_root_registry`, the
  `player_render_size` SPEC, `record_index` — derived once from compile-time-baked
  tables, pure.

> **The design oracle:** *could another platformer be built by ADDING a content crate
> without editing core?* If a change makes the answer "no", it's the wrong change.
>
> **The reusability test:** extract one domain crate clean — zero `gameplay_core`
> imports — and use its collision semantics / projectile lifecycle / actor intent
> untethered to Ambition's narrative.

## Live structural debts (ranked)

1. **KEYSTONE — collapse `Player*` / `Actor*`** (above). Unblocks everything.
2. **Portal shot-placement adapter** — a **design fork, not a sweep**: decide whether a
   portal may be placed on a moving platform / ECS solid, *and* whether carving the
   aperture into the world the shot raycasts against creates feedback into the shot
   trace. Decide that first; add a test; then route the reader onto the collision view.
3. **gnu_ton's subtractive arena gate** — the last mid-room base-mutator; needs the
   overlay model extended (carve + climbable-region overlay) beyond additive `gate_solids`.
4. **`ControlFrame` → entity-local `ActorIntent`** — ~46 systems read global input; the
   sim should read the *body's* intent. Rendering / input-sources stay presentation
   consumers. (Subsumed by the keystone collapse.)
5. **`falling_sand` → overlay** (feature-gated, open question).

## Validation strategy

Structural refactors may land **ahead of** feel/visual verification; a feel regression
is fixed afterward and is not a blocker. The gate is the **differential headless
harness** ([`headless-verification.md`](headless-verification.md)) — much is
headless-testable (e.g. collision: a ladder/floor-gate is present-or-not), and what
isn't (subjective feel) is Jon's in-game check, done on a marked commit.

## Done (drop from any old plan — pure history)

The compat shims are gone (`architecture_boundaries` enforces canonical paths);
`GameWorld` → `RoomGeometry` + the `CollisionWorld` view; the app drain (room
transitions, attack phases, damage, movement SFX/VFX, boss specials, death attribution
now in runtime domains); the generic/named projectile split; world gates → overlay;
cutscenes extracted to `ambition_cutscene` with the render leak closed; the OnceLock
registries classified.

## The discipline

> **Delete, don't bridge. Rename in place, don't alias. Add seams when the second use
> case lands.** Pre-release, single-commit replacement beats a two-step bridge. Overlap
> old and new consumers only briefly when changing authority, then delete the old path
> in the same branch.
