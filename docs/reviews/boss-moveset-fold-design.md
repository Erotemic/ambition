# Boss → moveset fold — design proposal (for Jon's approval)

**Status:** DESIGN, awaiting Jon's approve/modify. Requested during the 2026-07-03
§A-line push (fable review §A1). Do NOT execute until approved.

---

## TL;DR — the finding that should drive the decision

I mapped both systems before designing. The headline:

> **The moveset system (`MoveSpec` / `MovePlayback`) is unproven SCAFFOLDING.**
> Nothing in the live game ever creates a `MovePlayback` — only tests do. It has
> **no trigger glue** (a move is "triggered" by inserting the component; nothing
> inserts it in production), its `MoveEventKind::Effect{key}` seam has **no
> consumer**, and its hit-volume model is **static-offset, spawned once per Active
> window** — a *downgrade* from the boss's per-tick re-derived, multi-part,
> pose-tracking geometry (GNU-ton's two hands, the gradient sentinel's animating
> strike boxes).

And the mirror finding about the source:

> **The boss attack model is already WELL-FACTORED, not a duplicated island.**
> Capability (`BossCapability`) vs policy (`BossPattern`) is split like `ActionSet`
> vs brain. `Special(key)` → content `Technique` is already the "open moveset"
> seam (the per-boss variants already collapsed to one string carrier). The phase
> machine is cleanly separable. Possession already maps input onto the repertoire
> (`slot(i)` / `signature_special()`). The ~126 `BossAttackState` references are
> mostly **legitimate consumers of a well-defined projection**, not copy-paste.

**Consequence:** folding the boss onto the moveset *right now* would ADD a layer
(greenfield trigger glue + effect dispatch + a geometry extension) and KEEP
`BossAttackState` as a projection (so the 126 consumers don't break) — i.e. **more
code, plus a geometry downgrade and a proper-time behavior change**. That fails the
acceptance test for these refactors ("convergence = one path, *less* code").

**My recommendation: don't fold the boss onto the moveset as the next step.** Pick
one of the two honest paths in §5. My top pick is **Path A (defer, with the island
re-scoped)**; if you want to invest in the moveset, **Path B (prove it on a simple
actor melee FIRST, boss second)** is the correct sequencing. **Path C (boss-first
port now)** is possible but is the worst trade and I'd advise against it.

---

## 1. The two systems, precisely

### Source — the boss attack model (3 crates)

- **`BossAttackProfile`** (`ambition_characters/src/brain/boss_pattern/mod.rs:243`):
  geometry variants (`FloorSlam`, `SideSweep`, `FullBodyPulse`, `WingSweep`,
  `DiveLane`, `Broadside`, `HandSlam`, `HandSweep`, `HeadDescent`,
  `ConvergingShockwave`, `HazardColumn`) → damage via `volumes_for_profile` →
  frame-driven hitboxes; plus `Special(String)` → damage via a content `Technique`
  off `ActorActionMessage::Special`. (The old per-boss special variants ALREADY
  collapsed to `Special(key)`.)
- **`BossPatternStep`** (`mod.rs:165`): `Telegraph{profile,dur}` | `Strike{profile,dur}`
  | `Rest{dur}`. A **`BossPattern`** is a `Vec<step>`; **`BossAttackPattern::Scripted`**
  carries one pattern per encounter phase (intro/phase1/transition/phase2/enrage).
  The cursor loops.
- **`BossAttackState`** (`mod.rs:718`, `#[derive(Component)]`): the live projection —
  `telegraph_profile/remaining/elapsed`, `active_profile/remaining/elapsed`. Produced
  inside the brain (`BossPatternState.attack_state`, `tick_boss_pattern`), **mirrored
  to the ECS component** each tick (`bosses/tick.rs:369`). **~126 refs** read it:
  attack-volume geometry, damageable-hurtbox selection, damage predicates, sprite
  anim rows, telegraph overlays, debug gizmos, possession, and the content specials.
- **`BossCapability`** (`mod.rs:768`, Component, persists across brain swap): the
  repertoire `Vec<(BossAttackProfile, strike_secs)>`; possession maps input →
  `slot(i)` / `signature_special()`. Explicitly "the boss analogue of `ActionSet`."
- **Phase machine** (`BossPhaseState`, HP/time triggers) selects the pattern; a pure
  function of `(phase, cursor)`. Cleanly separable.

### Target — the moveset (`MoveSpec` / `MovePlayback`), all greenfield

- **`MoveSpec`** (`ambition_entity_catalog/src/lib.rs:138`): `{id, clip, duration_s,
  windows: Vec<MoveWindow>, events, gates}`. **`MoveWindow`** = `{start_s, end_s,
  tag, volumes}`; **`WindowTag`** = `Startup|Active|Recovery|Invuln|Armor|Cancelable`
  (no `Telegraph` — `Startup` is the tell). **`HitVolume`** = a **static** body-local
  rect/circle + damage + knockback, only allowed on `Active` windows.
- **`MovePlayback`** (`gameplay_core/src/combat/moveset.rs:49`, Component): runtime
  `{spec, facing, t, live_boxes, fired}`. `advance_move_playback` (the ONE system)
  advances `t` on the owner's **proper-time** clock, spawns a hitbox per `HitVolume`
  **once on Active-window entry** (`FollowOwner` static local offset), despawns on
  exit, and fires `MoveEventMessage` on timed events.
- **`MovesetContract`** (`lib.rs:199`): `{verbs: map<verb,id>, moves: Vec<MoveSpec>}`
  with `move_by_id` / `move_for_verb`. **Data only — nothing selects/inserts at
  runtime.**
- **Trigger = insert `MovePlayback`.** No production code does this. The system is
  registered in the live schedule (`combat_schedule.rs:118`) but is a **no-op in the
  shipping game** (never any `MovePlayback` to act on). `MoveEventMessage` is
  registered but has **no `MessageReader` outside tests**.

---

## 2. The structural mapping (IF we port)

| Boss concept | Moveset concept |
|---|---|
| `Telegraph{profile,dur}` | `MoveWindow{tag: Startup}` (no volumes) |
| `Strike{profile,dur}` (geometry) | `MoveWindow{tag: Active, volumes}` |
| `Strike{profile: Special(key)}` | `Active` window + `MoveEvent{Effect{key}}` |
| `Rest{dur}` | a `Recovery` window / gap between moves |
| ONE attack (Tel→Strike→Rest) | ONE `MoveSpec` |
| `BossPattern` (which attack, when — the loop) | a **separate move-sequencer** (stays boss-side) |
| `BossCapability.specials` (repertoire) | `MovesetContract.moves` |
| `BossAttackProfile` / `Special(key)` | `MoveSpec.id` (+ `verbs`) |
| possession `slot(i)` / `signature_special()` | `move_by_id` / `move_for_verb` |
| `BossAttackState` timers (sim dt) | `MovePlayback.t` (proper-time dt) |

Correspondence is real — the boss's capability/policy split lines up with
`MovesetContract`/sequencer, and `Special(key)` lines up with `MoveSpec.id`. **But
the gaps below are each real engineering, not a data edit.**

---

## 3. The gaps that make "port now" a bad trade

1. **No trigger glue exists.** You must build the system that selects a move and
   inserts `MovePlayback`. Today the `BossPattern` cursor writes `BossAttackState`;
   you'd replace that with a sequencer that inserts moves on Strike entry.
2. **One `MoveSpec` = one attack; the looping multi-attack script stays a separate
   scheduler.** Don't encode a whole phase loop as one spec.
3. **Geometry DOWNGRADE (the hard one).** Moveset volumes are static-offset,
   spawned once per Active window, translated by `FollowOwner`. The boss re-derives
   geometry **every tick** so a multi-part / animating pose tracks the drawn frame
   (`sync_boss_strike_hitboxes` → `active_attack_volumes` with the live anim frame).
   GNU-ton's two hands and the gradient sentinel's pose-tracking strikes do NOT map
   onto a single static window. You'd have to extend `HitVolume` to be frame-sampled
   (or keep `sync_boss_strike_hitboxes` and feed it from move state — i.e. keep the
   boss geometry path anyway).
4. **`Special(key)` specials emit NO volumes** — they spawn projectiles / world
   hitboxes / minions via the `Special` message → a content `Technique`. `MoveSpec`
   has no equivalent; `MoveEventKind::Effect{key}` is the closest seam **but nothing
   consumes `MoveEventMessage`**. You'd wire `MoveEventMessage → the technique
   dispatch` that `ActionRequest::Special` feeds today (greenfield).
5. **Proper-time behavior change.** `MovePlayback` runs on the owner's dilatable
   clock; the boss pattern runs on plain `sim_dt`. Adopting moveset gives boss
   attacks dilation they don't have — intended long-term, but a change to validate
   against every encounter's timing.
6. **`BossAttackState`'s 126 consumers.** Unless you rewrite all of them, you keep
   `BossAttackState` as a **read-model projection written from `MovePlayback`** — so
   the fold doesn't delete it, it adds a producer in front of it.

Net of 1–6: **a new runtime layer + a kept projection + a geometry extension**, to
replace a working, well-factored system. That's not "less code."

---

## 4. Why the doc oversold this as "the single largest fork"

The original audit predates the AS4b/AS4c archetype swap and the `Special(key)`
collapse. Post-those, the boss is **already** an aerial actor for movement (shared
flight limb), ticks the **universal** `Brain::tick`, routes damage through the
**shared** `apply_hitbox_damage`, and its specials go through the **shared**
`ActorActionMessage::Special`. What's left that's genuinely boss-specific is:
the attack-STATE projection (`BossAttackState`) + its per-tick geometry path, the
separate tick/sync systems, and the separate render animator (3f). That's a
*different subsystem*, not duplicated logic — and the moveset isn't ready to absorb
it.

---

## 5. The three paths (recommendation: A, else B)

### Path A — **Defer the fold; re-scope the "island" as mostly-closed** ⭐ (recommended)
Accept that after the recon, the boss attack model is well-factored and the moveset
isn't ready. Mark the moveset migration as a FUTURE program gated on a real need
(a second game, or a character wanting decomposable/rebindable moves — the actual
use case the moveset was built for). Spend remaining A-line energy on the honest
small residuals + A7. **Pros:** no risky rewrite, no geometry downgrade, respects
"add knobs when use cases land." **Cons:** the boss keeps its own attack-state +
render animator (a real but *contained* seam); the unified-actors north star isn't
100% reached for bosses yet.

### Path B — **Prove the moveset on a simple ACTOR melee first, boss second**
Give the moveset its FIRST production consumer where static geometry SUFFICES:
migrate a basic enemy's `MeleeActionSpec` swing → a `MoveSpec` + build the trigger
glue (`ActionSet.melee`/verb → insert `MovePlayback`) and the effect dispatch
(`MoveEventMessage` → SFX/VFX). That validates the system in production on the easy
case. THEN extend it for the boss (frame-sampled multi-part `HitVolume`;
`Effect{key}` → technique dispatch) and fold the boss with confidence. **Pros:**
correct sequencing (prove infra on the simple case before the complex boss); ends
with a genuinely unified, decomposable move system; the moveset stops being dead
scaffolding. **Cons:** multi-session; Phase 0 (actor-melee migration) is itself a
feel-sensitive change to the proven `BodyMelee` path; net convergence only arrives
at the end.

### Path C — **Boss-first port now** (not recommended)
Build the trigger glue + effect dispatch + geometry extension on the BOSS as the
first/guinea-pig consumer, keep `BossAttackState` as a projection. **Pros:** directly
attacks the named item. **Cons:** the worst trade — makes the most complex actor the
proving ground for unproven infra, forces the geometry extension up front, risks all
13 boss suites + every content special, and still doesn't reduce code. This is the
"bold port" as literally scoped, and I think it's the wrong bold move.

---

## 6. If you pick B — the phased plan

- **Phase 0 — moveset gets a real consumer.** Pick one simple hostile with a plain
  melee swing. Author its swing as a `MoveSpec` (Startup/Active/Recovery windows,
  one static `HitVolume`). Build: (a) trigger glue — on the melee control edge,
  look up `move_for_verb`/`move_by_id` and insert `MovePlayback`; (b) effect
  dispatch — a `MessageReader<MoveEventMessage>` → `SfxMessage`/`EffectRequest`.
  Gate behind the existing melee suites; ship blind for feel. Deletes nothing yet —
  it's the beachhead.
- **Phase 1 — extend for boss geometry + specials.** Add frame-sampled / multi-part
  `HitVolume` (or a `Window` that defers to a geometry fn) so a pose-tracking strike
  maps; wire `MoveEventKind::Effect{key}` → the content-technique dispatch that
  `ActionRequest::Special` uses.
- **Phase 2 — fold the boss.** `BossPattern` → a move-sequencer that inserts
  `MoveSpec`s; write `BossAttackState` as a **projection** from `MovePlayback` so the
  126 consumers keep working, then migrate them off it incrementally; retire
  `sync_boss_strike_hitboxes` once the geometry extension covers every boss strike.
  Render animator (3f) folds last (it's blind — its own slice).

Each phase compiles + keeps the boss suites green; Phases 0/1 add no boss risk.

---

## 7. What I need from you

1. **A, B, or C?** (I recommend A; B if you want to invest in the moveset now.)
2. If B: OK to touch the proven `BodyMelee` path in Phase 0, or should Phase 0
   pick an enemy that currently has NO melee (pure additive, zero regression risk)?
3. Either way: the render animator fold (3f) is BLIND (presentation-unverifiable) —
   confirm you're OK feel-checking it whenever it lands.
