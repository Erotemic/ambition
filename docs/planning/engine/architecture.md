# Engine architecture

How the engine is structured so it compiles fast, navigates well, pluginizes
idiomatically, and is reusable by another game (the four goals). The actor model
itself is [`unified-actors.md`](unified-actors.md); this is the crate/plugin shell
around it.

---

## The target stack

Three tiers, imports flowing one way:

- **Foundation** — `ambition_engine_core`, `ambition_platformer_primitives`. Pure
  spatial / kinematic / collision primitives. No gameplay, no content, no Bevy app
  concerns. Anything here must be usable by *any* platformer.
- **Runtime domains** — one crate per subsystem: actor control, actor runtime,
  combat runtime, projectiles, world runtime, carryable items, cutscene, portal, … .
  Each **owns its vocabulary** (its components / resources / messages), **owns its
  authoritative state** (what it mutates), and exposes **local schedule sets**. A
  domain imports the foundation and talks to peer domains **via messages**, never by
  reaching up into the app or into content.
- **Composition root** — `ambition_game` (collects the domains and maps their sets
  into the app schedule) and `ambition_app` (platform, assets, windowing, the
  binaries). `ambition_app` is the *only* crate allowed to name both machinery and
  content. `ambition_game` is a *direction* — introduce it when the app file clearly
  splits host-vs-composition jobs, not before.

**Boundary rule (enforced by the `architecture_boundaries` test):** machinery must
not import named content. If a rule needs content data to run, it lives in a
content install-seam, not in a foundation/runtime crate.

## The reusable-engine principle

The engine is a composable stack of **domain plugins that accept
content-installable data** — named archetype rows, boss specs, encounter waves,
audio cues, item/ability registries — through `OnceLock`-backed *pure-function
resolvers* (install once at startup, immutable after, read from non-system code with
no `World` access). A second platformer installs **only** its content crate atop the
unmodified runtime domains; **zero core edits**.

> **The design oracle:** *could another platformer be built by ADDING a content crate
> without editing core?* If a change makes the answer "no", it's the wrong change.

> **The reusability test:** extract one domain crate clean — zero `gameplay_core`
> imports — and use its collision semantics / projectile lifecycle / actor intent
> untethered to Ambition's narrative.

## Bevy-plugin shape

Each domain is a plugin exposing four things:

1. **Owned vocabulary** — components / resources / messages native to the domain.
2. **Authoritative state** — exactly what it mutates (no other domain writes it).
3. **Local schedule sets** — a consistent rhythm:
   `BuildIntent → Simulate → Resolve → EmitFacts → ProjectPresentation`.
4. **Public extension points** — content attaches systems to named slots
   (`CombatSet::ContentSpecials`, `PortalSet`, …) **without reaching into privates**.

**`ambition_portal` is the exemplar shape**, copy it to extract a domain: runtime
core crate + optional presentation crate + optional `content/adapter` package + the
app maps the schedule. Presentation is always a separate concern — the sim runs
headless ([`headless-verification.md`](headless-verification.md)).

## World geometry

`RoomGeometry` is **authored** and swapped at room boundaries — never mutated
mid-room. All mid-room dynamics (moving platforms, gates, portal carves, ECS solids)
compose through a **derived `CollisionWorld` view** layered on top. This keeps a
frame's collision truth a *pure function of room id + overlay state* — replay- and
RL-friendly, and it supports per-player world variants later without a mutable
monolith that tempts content to reach in and mutate the base.

## The keystone: collapse the `Player*` / `Actor*` dual hierarchy

This is the structural prerequisite for goals 1 and 2, and it is the same cut as the
unified-actors work — stated here for its *architectural* payoff.

`crate::player` is today a ~20-module **dependency sink**: shared simulation state
lives under `Player*` names, so 20 modules back-edge into it and nothing extracts.
Move the shared sim-state onto the `Actor*` vocabulary in shared `body` / `actor`
modules and those back-edges dissolve — modules like `time`, `body_mode` become
extractable leaf crates, and **compile time drops**.

**Sliced, not big-bang** — one component family per slice, gated on *it compiles* +
the differential headless trace (behavior may change; that's allowed). Three buckets:

1. **Namespace-only moves** — already-shared types (`BodyKinematics`, `PlayerEntity`)
   to neutral homes.
2. **Sim-state collapses** — `PlayerCombatState` → `ActorCombatState`, the
   movement/ability states fold onto the shared body (the unified-actors phase 4 cut).
3. **Keep player-only** — genuinely player-specific (`PlayerHudRoot`, camera, device
   input, demo) stays.

## Live structural debts (roadmap items)

- **`ControlFrame` → entity-local `ActorIntent`.** ~46 systems read global input;
  the sim should read the *body's* intent. Rendering / input-sources stay
  presentation consumers. (The non-player-centric finish line.)
- **Projectile attribution** — replace the player/enemy projectile split with
  source/faction; track enemy-projectile owners by entity.
- **gnu_ton's subtractive arena gate** — the last mid-room base-mutator; needs the
  overlay model extended (carve + climbable-region overlay) beyond additive
  `gate_solids`.
- **Portal shot-placement on moving platforms** — a design fork (may portals carve
  moving platforms?); add the test before routing.

## Done (drop from any old plan)

Pure history — already landed; do not re-plan: the compat shims are gone (canonical
paths enforced by `architecture_boundaries`); `GameWorld` → `RoomGeometry` + the
`CollisionWorld` view; the app drain (room transitions, attack phases, damage,
movement SFX/VFX, boss specials, death attribution now in runtime domains); the
generic/named projectile split; world gates → overlay; cutscenes extracted to
`ambition_cutscene` with the render leak closed; the OnceLock registries classified.

## The discipline

> **Delete, don't bridge. Rename in place, don't alias. Add seams when the second use
> case lands.** Pre-release, single-commit replacement beats a two-step bridge.
