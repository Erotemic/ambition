# THE VISION — a Unity/Godot-class 2D platformer engine, the Bevy way

**Authored by fable, 2026-07-05, at Jon's direction.** This is the top of the
planning stack: what we are building, why, and what "done" looks like. Every
other document in `docs/planning/` serves this one. If a task cannot be traced
to this document through [`roadmap.md`](roadmap.md) and a track in
[`tracks.md`](tracks.md), it is not the work.

---

## 1. What we are building

**A reusable, composable, ECS-native 2D platformer engine** on Bevy and Rust
that competes with Unity, Godot, and (in expressibility ambition) Unreal for
the 2D platformer / action-platformer / platform-fighter space — plus
**Ambition**, the first game built on it, and a **suite of demo games** that
prove the engine the way test vectors prove a kernel.

> A primary goal of ambition should be to create a game engine on the level of
> Unity / Unreal / Godot for 2D platformers, on top of Bevy and Rust. That
> means ECS-native and centered around the idea of composition and plugins.
> ELEGANCE and BEAUTY are first-class design constraints of the codebase.
> — Jon (binding)

**Our identity vs. the editor engines:** Unity and Godot are *editors* first.
We are **the Bevy way taken seriously**: the engine is a set of composable
crates and plugins; content is a Rust crate + RON + a spatial authoring backend
(LDtk today; Tiled/Godot-scene importers are legitimate future backends —
we may well borrow *their* authoring tools) + Yarn for dialogue. The "editor"
is best-in-class external tools speaking through validated data seams. We
compete on **architecture, expressibility, headless testability, and agent
navigability**, not on shipping an editor binary.

**The design oracle** (unchanged, permanent):

> *Could another platformer be built by ADDING a content crate, without
> editing core?*

The demo games make the oracle executable. Each demo's `git log --stat` must
touch zero engine crates; every violation files an `oracle-violation` and
becomes engine work.

## 2. The four product pillars

1. **The engine** — the crate stack of [`engine/architecture.md`](engine/architecture.md):
   frame-agnostic movement kernels (axis-swept AABB *and* surface-momentum),
   the unified actor model (one body pipeline, brains behind one interface,
   possession as brain transfer), data-driven combat (movesets, volumes,
   knockback), authored space through backend-agnostic IR, deterministic
   headless simulation, and a plugin-group bootstrap
   (`ambition_runtime::PlatformerEnginePlugins`) that makes a new game's
   `main.rs` ~100 lines.
2. **Ambition, the game** — the flagship content crate
   ([`game/`](game/)): *"Every upgrade a theorem, every boss a failed
   objective function, every biome a math world model."* The sandbox is ALSO
   the engine's integration lab: it can host every demo game inside its world
   (see §5).
3. **The demo suite** ([`demos/`](demos/)) — standalone games, each ONE
   content crate + a thin app: **Sanic**, **Super Mary-O**, **Super Smash
   Siblings**, **Hollow Lite**, and the later tiers of the matrix in
   [`roadmap.md`](roadmap.md). These are written in stone as vision; only
   their ORDER is negotiable.
4. **The intelligence stack** — headless/RL-first simulation is not a test
   convenience, it is a product surface: forward-model AI (the
   [fighter brain](engine/fighter-brain.md)), the
   [boss-design pipeline](engine/boss-design.md) that lets mid-tier agents
   author genuinely good fights, and RL training hooks. Only
   non-simulation-impacting visuals may be presentation-only; everything that
   affects outcomes must be steppable headless.

## 3. What "1.0" looks like (the goal state)

- The crate map of `engine/architecture.md` is REAL: `ambition_actors`
  no longer exists as a monolith; every crate is a well-scoped domain a small
  agent can navigate and modify safely. **This is the absolute highest
  priority** — extensibility, pluggability, and agent navigation come from
  the decomposition ([`engine/decomposition.md`](engine/decomposition.md)).
- All four named demos exist and pass the oracle; the ambition sandbox can
  host each demo in-world (§5).
- The collision doctrine of
  [`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) holds: every
  mover and every trigger is swept (no discrete sampling anywhere), the OOB
  bug class is structurally dead, non-axis-aligned geometry is a first-class
  surface, and portals may MOVE.
- The combat model ([`engine/combat-model.md`](engine/combat-model.md))
  expresses the full smash stack — knockback scaling on a damage-accumulation
  axis, directional influence, smash attacks, cancel/chain tables — as data,
  shared by every actor, headless-testable.
- Determinism is a managed contract ([`engine/netcode.md`](engine/netcode.md)):
  local-N multiplayer ships with Super Smash Siblings; the
  snapshot/rollback seams exist even if online ships post-1.0.
- Relativity mechanics ([`engine/slower-light.md`](engine/slower-light.md))
  have their seams paid for (Tier-0 obligations in the read-model), with the
  full mechanic staged behind the demos.
- The docs stack is trustworthy: `docs/planning/` is the single source of
  truth for direction; `docs/concepts|systems|mechanics` describe what exists
  (updating them is a scheduled track, executable by mid-tier agents).

## 4. The demo suite is written in stone

Each demo is a **standalone system depending only on the engine**:
`<demo>-content` (one crate: world, rosters, rules, match/level state) + a
`<demo>-app` (~100-line thin shell). No demo may edit an engine crate; no
engine crate may name a demo. Full designs live in [`demos/`](demos/):

| Demo | Inspiration | Proves |
|---|---|---|
| **Sanic** | Sonic 2, Emerald Hill Zone act 1 | the momentum/surface kernel: slopes, loops, springs, rings-analog, momentum enemies |
| **Super Mary-O** | SMB1 world 1-1 | the classic tile-platformer baseline: powerup-as-equipment, one-way camera, stomp kills, flagpole sequencing |
| **Super Smash Siblings** | SSB1 | multiple controlled bodies with DIFFERENT movement identities in one arena, shared combat semantics, retained feel; local-N input routing; match state outside engine core |
| **Hollow Lite** | Hollow Knight's first area + boss | the exploration-combat loop and, above all, the boss-design pipeline producing a FUN fight |

Later tiers (MoneySeize, Celeste-slice, Metroid-slice, Braid-slice, Dead
Cells-slice, Rain World-slice) stay in the roadmap matrix as capability test
vectors; they get full design docs when their tier opens.

## 5. Ambition hosts the demos (maximum composability, forced honesty)

The ambition sandbox app depends on the demo **content** crates and mounts
each demo inside the LDtk world: possess Sanic in the Hall of Characters, walk
into the Sanic demo zone, and that game plays *as if launched standalone* —
same rules, same systems — with only presentation differences (the standalone
app pulls fewer crates; ambition may keep its own HUD chrome). This forces the
demo crates to be genuinely scoped systems: **a demo's rules are a plugin
activated per area/room, not global app state.** The design (the "scoped game
mode" pattern) lives in [`demos/README.md`](demos/README.md). We do not have
to build all of this at once — but every demo is DESIGNED for it from day one.

## 6. How we get there (the arc, one paragraph)

Finish the engine face: complete `ambition_runtime` (the demo gate), execute
the decomposition playbook until the monolith is gone, keep the collision/
combat/netcode doctrines ahead of the demos that need them. Ship Sanic and
Super Mary-O against the oracle. Land local-N + the combat stack, ship Super
Smash Siblings. Land the boss pipeline + fighter brain, ship Hollow Lite.
Then Ambition-the-game gets built ON a finished engine — which was the plan
since the north star was written. Phases and status:
[`roadmap.md`](roadmap.md); the live queue: [`tracks.md`](tracks.md).

## 7. Who does what (the model ladder)

Jon has limited frontier-model (fable) access. The plan is deliberately
structured so that **everything below the hardest tier is executable by
opus-level agents following the written specs, with sonnet-level agents
handling mechanically-specified slices.** Every track in `tracks.md` carries
an executor grade:

- **[fable]** — genuinely hard design/kernel work; do while access lasts
  (the standing list lives at the top of `tracks.md`).
- **[opus, fable-specced]** — the spec in the planning doc IS the design;
  opus executes it verbatim and STOPS at the first sign the spec doesn't fit
  the code (surface the mismatch; do not improvise architecture).
- **[opus]** — well-bounded engineering; the doc gives shape + exit criteria.
- **[sonnet]** — mechanical: renames, moves, authored data, test scaffolds,
  doc sweeps — with exact file lists and commands.

**Deviation rule (Jon's, binding):** an agent may NOT deviate from the plan
because a step is hard, tedious, or "you aren't gonna need it" — we are
building an engine; people will need it. Deviation is legitimate ONLY when
the code contradicts the plan's factual assumptions ("fable didn't see
this"); then the agent surfaces the contradiction in the execution log and
queues the design question, taking parallel work meanwhile. Jon can always
overrule — and good agent counter-arguments are welcome; make the case, don't
silently drift.

## 8. Principles digest

The autonomous-decision criteria are Jon's own words in
[`decision-principles.md`](decision-principles.md) — read them before any
architectural choice. The compressed spine:

- **Elegance is the objective function; correctness emerges from it.**
- **Layer law:** Rust is behavior; RON is content; the world IR is space
  (authored by a backend: LDtk/Tiled/Godot); machinery never imports named
  content.
- **Relativity, not player-centrism:** mechanics are frame-agnostic and
  shared by every actor; the strongest tests are symmetry/covariance (C4
  gravity rotation, through-portal invariance).
- **Two-port body:** controllers attempt, bodies enforce — human, brain, RL,
  and (future) remote inputs are interchangeable.
- **No pushout, ever** (one exception: portal-close straddle eviction).
  Transit emerges at the face; sweep to TOI; nothing teleports.
- **Headless first:** verify against the real simulation; feel ships BLIND in
  marked commits for Jon's pass; never pause the architecture for feel.
- **Delete, don't bridge; rename in place; add seams when the second use case
  lands.** Pre-release, single-commit replacement beats compat shims.
- **Grow existing crates before minting new ones;** a crate earns existence
  by owning a coherent domain (healthy band ~1–15k LOC).
