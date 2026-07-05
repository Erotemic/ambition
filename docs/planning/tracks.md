# TRACKS — the live work queue + execution log

**This is the execution front-end.** Every open work item in the project
is here or in a doc this file points to; the archived review docs
(`docs/archive/reviews/`) are history, not tasking. Executor rules:
[`vision.md`](vision.md) §7 (grades, the deviation rule);
[`decision-principles.md`](decision-principles.md) for autonomous calls.
Append execution-log entries at the bottom; keep statuses current; Jon
can only read, not ask.

Standing verification gate: `cargo build -p ambition_app --features
rl_sim` + gameplay_core lib + content + app rl_sim suites + boundary
tests. Known feel-reserved RED: `unified_melee::a_hostile_actor…`.

**Living-plan discipline (README.md, binding): every work commit updates
this file's statuses in the SAME commit; DONE detail compresses to one
line + hash; drift between a planning doc and the code is fixed in the
doc immediately. Execute in dependency order: decomposition (D-A) cards
first; demos/netcode/combat/brain/boss tracks start only when their
listed preconditions are DONE here.**

---

## ⚡ THE FABLE WINDOW (spend remaining frontier days here, in order)

1. **E4 design + the L5 message inversion** — the sim_view scout ruling
   and read-model shape (decomposition.md E4). The riskiest cut.
2. **`SimSnapshot` design** (netcode N3.1 design only) — shapes every
   future sim component; cheap now, expensive to retrofit.
3. **CM4 cancel-table advancer spec** (combat-model) — the one moveset
   edit that wants frontier care.
4. **CC5 `PortalFrame`** — or leave opus the parity-gated version.
5. **FB6 rollout architecture note** — one page so opus can't
   mis-build it later.
Everything else on this page is opus-or-below by design.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | ACTIVE — E5 first slice landed (`3c70d827`); W/E1/E2/E3/E6/E7/E8 open | E5-finish steps 1–5 [opus, fable-specced] |
| Decomposition D-B/D-C | same | queued behind D-A | mode-scope seam can land early (demos want it) |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | NEW — §7.6 swept transit + blocks-as-surfaces landed | CC1 cast consolidation [opus] |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | NEW | CM1 knockback growth (parity-pinned) [opus] |
| Netcode ladder | [engine/netcode.md](engine/netcode.md) | NEW | N0.2 input-stream type; N0.3 lint set [opus] |
| Fighter brain | [engine/fighter-brain.md](engine/fighter-brain.md) | NEW | FB1 view audit [opus] (CM7 first) |
| Boss pipeline | [engine/boss-design.md](engine/boss-design.md) | NEW | BD4 seed extraction [opus/sonnet]; BD1 after |
| Falling sand | [engine/falling-sand.md](engine/falling-sand.md) | NEW; low priority | FS1 single-owner + conservation [opus] |
| S — Sanic | [demos/sanic.md](demos/sanic.md) | S1–S3 landed; Sanic-in-normal-rooms fixed (`0189338b`) | S4 proofs; ball-dash technique [opus] |
| M — Super Mary-O | [demos/super-mary-o.md](demos/super-mary-o.md) | gated on E5-finish | M1+A3 powerup-equipment [opus] |
| F — Super Smash Siblings | [demos/super-smash-siblings.md](demos/super-smash-siblings.md) | gated on CM/N1/FB | F1 rules crate once CM1–CM2 land |
| H — Hollow Lite | [demos/hollow-lite.md](demos/hollow-lite.md) | gated on BD pipeline | after BD7 pilot |
| Slower light | [engine/slower-light.md](engine/slower-light.md) | Tier-0 rides E4; L1–L4 in P5 | — |
| Docs refresh (mechanics/concepts/systems currency) | — | P5; safe for [opus] once this stack is north star | — |

## The actor-policy slice: respawn unification + ADR 0022 — [opus, fable-specced]

Jon's model: default = **dead stays dead**; respawning is an AUTHORED
choice (`Mob`); the sandbag in-place timer folds into the same enum.
Scouted 2026-07-05; exact edit points:

1. **One authored field replaces two derived bools.** `EnemyRespawnPolicy`
   (combat/components/mod.rs:457) grows `InPlace(f32)`; `#[default]`
   flips `OnRoomReenter` → `Never` (rename the variant `DeadStaysDead`).
   Archetype RON: replace `respawn_on_rest: bool` +
   `respawn_in_place_seconds: Option<f32>` with `respawn:
   RespawnPolicy` (serde default = DeadStaysDead); `respawn_policy()`
   (features/enemies/mod.rs:630) becomes a field read. Sandbag rows
   author `InPlace(0.85)`; `never_dies`/`is_sandbag` stay orthogonal.
2. **One kill path.** Merge actor_hit.rs:254 (in-place timer) into the
   :307 policy match; `DeadStaysDead` WRITES `enemy_{id}_dead` (today the
   default writes nothing — that's the whole bug); `OnRest` keeps its
   suffix flag; `OnRoomReenter` writes nothing (the Mob choice);
   `InPlace` sets the timer (no flag).
3. **Fix the peaceful-NPC fall-through** (save_sync.rs:69–107): a killed
   unprovoked NPC matches NEITHER branch — restructure so `dead_on_load`
   zeroes HP for interaction-bearing actors regardless of the hostile
   flag. Add the missing liveness tests (there are none — that's why it
   survived).
4. **Identity is already solid:** `config.id` == LDtk iid; flags via
   `SetFlagRequested`; no new machinery.
5. **Content triage (Q29):** author `OnRoomReenter` on trash-mob rows;
   named/unique actors take the new default. List the rows in the PR for
   Jon's read.
6. **ADR 0022 — engine respawn policy** records the model (0021 is
   reserved by W4).

## The bug/polish queue (Jon's 2026-07-05 play reports, homed)

| Item | Home | Grade |
|---|---|---|
| Slash VFX renders as a black square | investigate under CM5's per-move presentation slice; root-cause first (see log note) | [opus] |
| Per-attack VFX/SFX (not one generic swing) | CM5 | [opus] |
| Morph ball still draws the robot; generalize modal body morphs | E3 (mode→sprite-state row) | [opus] |
| Shrine + glider sprites broken | E3 (rect drift; sprite pipeline) | [opus] |
| All bosses render the generic sheet | E3/E6 — needs a RUN with `boss_sprites.len()` logging; do NOT apply the disproven sprite_target dispatch | [opus] |
| NPCs infinitely respawn | the respawn slice above | [opus] |
| Kernel-guide NPC should patrol a home base when peaceful | a `patrol` brain preset (waypoints/home-radius) in the brain vocabulary — small, body-generic | [opus] |
| Dialogs don't adapt to WHO is talking (possessed actor gets self-dialogue) | dialog context slice: the interact seam passes speaker/subject identities as Yarn variables; self-interaction gets a default branch | [opus, small design note first] |
| Sanic ball-dash special | demos/sanic.md (the one new technique) | [opus] |
| Portal gun should be a normal item (portal crate forgets the gun; one gun = one pair) | decontamination near A2/items; portal exposes `spawn portal of pair P on surface` primitive | [opus, low priority] |
| Smells journal (dev/journals/code_smells.md) | C4-style sweep rides each related track; the journal stays the intake | — |

## Oracle-violation log (demos file here; engine work exits through tracks)

*(empty — the discipline: demo commits never touch engine crates; each
violation gets a row here + a slice in the right doc.)*

## Porting audit (every open item from the archived reviews → here)

- 07-05 demo plan §0 crit 1 (crate map) → decomposition D-A. crit 2 tail
  (E6) → decomposition E6. crit 3 (named-content residue: speech_sfx,
  projectile visual kinds, boss fixtures) → decomposition E3/E6/E7-E8
  notes. crit 4 (demos) → demos/. crit 5 (stretch seams) → E4 Tier-0 +
  collision CC5–CC7. crit 6 (full green gate) → standing gate above.
- §2 specs Q16–Q26 → executed (S/G tracks) or ported verbatim into
  decomposition.md (Q23/Q24/Q25/Q26) and demos/.
- §5 Jon items → roadmap Q-register (Q2-name) + the feel queue (below).
- §7 defects: 7.1/7.2/7.5/7.6 FIXED; 7.3/7.4/7.7/7.8/7.9 → the bug/polish
  queue + respawn slice above.
- Feel queue (standing, Jon-only): the BLIND ledger — sanic area layout
  (`d620a230`), sanic sheet/params, G3 limb arcs + G5 verb bindings
  (`a5d15247`), moveset slash VFX placement (`05a32378`), swept-transit
  feel (`31342e6f`), + the `unified_melee` RED.
- 07-04/07-02 docs: fully executed or absorbed (their audits said so);
  no live items remain outside this page.

---

# EXECUTION LOG (append newest last)

## 2026-07-05 (fable) — Sanic-in-normal-rooms + wear semantics (`0189338b`)
Blocks are surfaces (SurfaceRef::Block, boundary chains, interior-face
occlusion, load-bearing landing rule); wear = possession semantics (no
kit fallback); blink gated off momentum bodies. engine_core 236 /
gameplay_core 1156 / app rl_sim 140 green. M14/M16 recorded.

## 2026-07-05 (fable) — the planning consolidation (this commit)
docs/planning rebuilt as the single source of truth: vision,
decision-principles (Jon's, relocated), roadmap, tracks (this file),
engine/{decomposition, collision-and-ccd, combat-model, netcode,
fighter-brain, boss-design, falling-sand}, demos/{README, sanic,
super-mary-o, super-smash-siblings, hollow-lite}; reviews archived;
docs/current retired. Slash-VFX black-square root-cause investigation
was still in flight at consolidation time — its findings append here.
