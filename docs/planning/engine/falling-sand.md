# Falling sand — correctness first, then "Don't fuck with Oiler"

**Authored by fable, 2026-07-05.** Low priority relative to the
decomposition/demo tracks, but part of the engine: the module must be
CORRECT (deterministic, conserving, settling) before any feature rides it.
Current observed defects (Jon): water and oil pool on the top platform yet
particles ALSO fall forever below; settling sand becomes world geometry on
both the top and bottom platforms — i.e., particles are being duplicated or
mis-homed between the pooling representation and the falling representation.

## 1. The correctness contract (the actual engine work)

The falling-sand room becomes a **bounded, deterministic cellular
automaton** with an explicit conservation law:

- **One representation.** A particle exists in exactly one place: the grid.
  Pooled/settled matter is grid state (or, once compiled, RoomGeometry
  overlay solids) — never simultaneously a live falling particle. The
  observed double-pooling symptom is by definition a violation of this
  invariant; the fix is structural (single owner), not a patch.
- **Conservation test:** total matter per material = spawned − despawned,
  every tick, asserted in a headless test that runs the room's real spec.
- **Settle guarantee:** any finite spawn input reaches a fixed point (no
  particle falls forever): every particle either settles, pools, or exits
  through an authored drain/kill boundary. Test: spawn N, step until
  quiescent, assert fixed point within a tick budget.
- **Fluids find level:** water/oil equalize across connected basins
  (the standard lateral-flow CA rule with a determinism-safe update order
  — stable cell ordering, double-buffered, per the query-order rule).
- **Sand→geometry compilation** stays, but only from settled grid state,
  through the RoomGeometry OVERLAY (the write-map rule: falling_sand never
  mutates authored geometry) — and compiled cells leave the grid
  (conservation moves them between owners atomically).
- **Determinism:** fixed update order, seeded RNG from world state, C4-safe
  (gravity-frame from `GravityCtx`, not −y). The module remains a
  self-gating CONTENT plugin (architecture ruling) — the engine ships the
  CA substrate, the room ships as content.

Slices FS1 (single-owner refactor + conservation test), FS2 (settle/level
rules + fixed-point test), FS3 (overlay compilation + atomic ownership
transfer). All [opus] — the spec above is the contract; the current code is
small and may be boldly restructured to meet it.

**The SPOUT is the canonical authored-placement example (Jon, 2026-07-06).**
A falling-sand spout (a source that emits matter) is an **authored PLACEMENT
in the map**, not a hardcoded runtime spawn — it rides the same
world→sim lowering seam as any other contentful placement (architecture
§4b): the map author drops a `spout` placement (a Tier-0 authored schema:
material, rate, direction), `ambition_world` carries it as an authored
record, and the falling-sand CONTENT plugin registers the interpreter that
lowers it into the runtime emitter at room-load. So the same-tier deps hold:
`ambition_world` never names the falling-sand runtime; the content plugin
depends on world + reads the schema. **QUESTION FOR FABLE:** whether the
spout schema is a general "emitter" placement or a falling-sand-specific
one rides [Q-FABLE W-a] (where authored schemas live + how specific they
get) — do not invent it before that lands.

## 2. Oiler (ideas parked here deliberately — feel-pass era work)

The character **Oiler** (Euler) weaponizes the module: a special that
sprays large volumes of oil (FS-spawner as a technique with params —
the A-track technique seam), pooled oil becomes a **surface coating** that
slows actors moving through it (a `Contact`-driven movement modifier — the
surface-coating vocabulary may also serve ice/goo later; add it when Oiler
lands, not before), and a second beat IGNITES pooled oil: flame propagates
across connected coated cells, dealing damage volumes over time and
consuming the oil (conservation: oil → fire → gone). After enough pooling,
the arena is a trap; hence the brainstorm's line. Prerequisites: FS1–FS3,
the technique params seam (landed), CM5 presentation events. Home when
built: Oiler is CONTENT (a catalog row + techniques); only the surface-
coating movement hook is engine vocabulary.
