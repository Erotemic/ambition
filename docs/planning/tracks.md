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

## ⚡ THE FABLE WINDOW (order confirmed by Jon 2026-07-06 + his addition:
## the hardest decompositions are fable EXECUTION work, not just design)

1. **E4 — EXECUTE the observation-boundary carve** (steps 2–4 of the
   card; opus can pre-land step 1, the vfx-message inversion). The
   riskiest cut in the playbook.
2. **`SimSnapshot` design** (netcode N3.1 — design written into the doc;
   review/refine against code when implementing starts).
3. **CM4 cancel-table advancer** — execute or pair-review opus's cut.
4. **CC5 `PortalFrame`** — execute, or leave opus the parity-gated card.
5. **FB6 rollout architecture** — sketch is in the doc; refine at
   implementation time.
Standing escalation: W3 (the world two-crate cut) and E2 (back-edge
classification) escalate to fable at the FIRST ambiguous item.
Everything else on this page is opus-or-below by design.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | ACTIVE — E5 first slice `3c70d827`; **E5-finish steps 1–4 LANDED 2026-07-06** (sets+resources+combat schedule into the group; shared headless foundation; cut-rope de-woven via generic `RoomReplayRequested` + labeled slots; E4-prep: fx facade imports repointed, CameraViewState + cut-rope resources re-owned) | E5 step 5 (mint [the windowed host]) + step 6 (smoke shell) [opus]; W/E1/E2/E3/E6/E7/E8 open |
| Decomposition D-B/D-C | same | queued behind D-A | mode-scope seam can land early (demos want it) |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | NEW — §7.6 swept transit + blocks-as-surfaces landed | CC1 cast consolidation [opus] |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | CM1 LANDED 2026-07-06 (knockback growth + weight + DeathPolicy, parity-pinned) | CM2 launch DI [opus] |
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

## The actor-policy slice: respawn unification + ADR 0022 — ✅ DONE (fable, 2026-07-06)

Executed as specced, with one design addition discovered in the code:
**policy is a property of the PLACEMENT** — peaceful NPCs borrow a mob
archetype's spec for their provoked form, so the NPC spawn plan PINS
`DeadStaysDead` (otherwise a "guide" borrowing `medium_striker`'s row
would respawn like a mob). One enum (`RespawnPolicy`, serde, default
DeadStaysDead, `InPlace(secs)` folds the sandbag timer), ONE carrier
(`ActorTuning.respawn` — the caps/tuning triplication deleted), one
kill-path match, universal liveness-on-load (+ the missing test),
Q29 triage authored (16 mob rows OnRoomReenter incl. the staged duel
pair + the gnu arena pair to preserve encounter reset; 5 OnRest;
sandbag InPlace). ADR 0022 written. Original spec below for reference:

1. **One authored field replaces two derived bools.** `EnemyRespawnPolicy`
   (`combat/components`) grows `InPlace(f32)`; `#[default]` flips
   `OnRoomReenter` → `DeadStaysDead` (rename of `Never`). Archetype RON:
   replace `respawn_on_rest: bool` + `respawn_in_place_seconds:
   Option<f32>` with one `respawn: RespawnPolicy` field (serde default =
   DeadStaysDead); the derived `respawn_policy()` helper in
   `features/enemies` becomes a field read. Sandbag rows author
   `InPlace(0.85)`; `never_dies`/`is_sandbag` stay orthogonal.
2. **One kill path.** In `features/ecs/damage/actor_hit.rs`, merge the
   in-place-timer branch into the kill-flag policy match (the two death
   paths become one match on the enum); `DeadStaysDead` WRITES
   `enemy_{id}_dead` (today the default writes nothing — that's the whole
   bug); `OnRest` keeps its suffix flag; `OnRoomReenter` writes nothing
   (the Mob choice); `InPlace` sets the timer (no flag).
3. **Fix the peaceful-NPC fall-through** (the two liveness branches of `sync_ecs_actors_from_save`, `features/ecs/save_sync.rs`): a killed
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
| Slash VFX renders as a black square | DEPRIORITIZED (Jon 2026-07-06: leaf effect, likely a sprite-source read quirk) — fold into CM5's per-move presentation slice when it lands; root-cause there, no dedicated pass | [opus] |
| `SurfaceRamp` quarter-circle marker entities (Q27 ruling): generated quarter-arc chain for floor↔wall momentum transitions; params radius/orientation/segments; same converter pattern as `SurfaceLoop` + LDtk entity def + validator row | [the space IR] converters / sanic demo | [opus/sonnet] |
| Per-attack VFX/SFX (not one generic swing) | CM5 | [opus] |
| Morph ball still draws the robot; generalize modal body morphs | E3 (mode→sprite-state row) | [opus] |
| Shrine + glider sprites broken | E3 (rect drift; sprite pipeline) | [opus] |
| All bosses render the generic sheet | E3/E6 — needs a RUN with `boss_sprites.len()` logging; do NOT apply the disproven sprite_target dispatch | [opus] |
| NPCs infinitely respawn | ✅ FIXED — the respawn slice above (ADR 0022) | done |
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

## 2026-07-05 (fable) — the planning consolidation
docs/planning rebuilt as the single source of truth: vision,
decision-principles (Jon's, relocated), roadmap, tracks (this file),
engine/{decomposition, collision-and-ccd, combat-model, netcode,
fighter-brain, boss-design, falling-sand}, demos/{README, sanic,
super-mary-o, super-smash-siblings, hollow-lite}; reviews archived;
docs/current retired. (`c8de27d5`)

## 2026-07-06 (fable) — the refinement pass (Jon's rulings folded in)
architecture.md rewritten as the crate set with EVERGREEN ROLE handles +
the workspace push-target layout (crates/=engine, game/=ambition,
demos/=demos) + [the sim heart]'s internal module map; decomposition.md
role-anchored, de-duplicated, line numbers scrubbed for symbol anchors
(evergreen-anchor rule added), E4 re-graded [★fable executes],
workspace re-home added to E7; demos deepened with consumes-by-role /
owns tables — SSB carries Jon's scope (roster: player-robot, goblin,
PCA, mary-o, sanic; percent display; select screen; ≤4 fighters, all-CPU
to 2-local-human; NO online round 1); Q4 decision brief written
(netcode.md) for Jon's call; Q27 (backends deferred; SurfaceRamp
quarter-circle entities instead) / Q28 (parody names = policy) / Q29
(respawn triage) / Q30 (fable window + hardest-carves-are-fable)
recorded; slash-VFX deprioritized into CM5.

## 2026-07-06 (fable) — respawn unification + E4-17 (execution, later same day)
ADR 0022 landed (`23b81c99`): ONE authored `RespawnPolicy` (default
DeadStaysDead), one carrier, one kill-path match, placement-pins for NPCs,
universal liveness-on-load + the missing test, Q29 triage in the RON. The
infinite-NPC-respawn bug is dead at the root.
E4-17 landed (commit follows): the camera OBSERVATION seam —
`CameraObservationPlugin` resolves the follow snapshot sim-side as a tail
observer (the only `CameraEaseState` writer; also live headless/RL);
render's `camera_follow` is a pure consumer; `CameraViewport`/
`CameraExtraClamp` are the generic observer-input resources; portal
continuity bridges its clamp pad same-frame. Discovered + recorded:
`PresentationSync` nests inside `CoreSimulation` — post-sim observers
anchor `.after(CoreSimulation)`. Continuity suite 3/3 green.

## 2026-07-06 (fable) — E5-finish steps 1–4 + E4 prep (execution)
[the sim assembly] grew: `SandboxSetsPlugin` (sets + ShrineActivationPulse/
SlotInteractionState/StartingCharacter, FIRST in the group; host-override-
by-pre-insert preserved), `CombatSchedulePlugin` moved in wholesale
(ambition_vfx dep added; content slots + guard test intact),
`add_headless_foundation` + `init_engine_states` (the 3× copy-pasted
foundation block + 2× cli init_state all converge). De-weave: the engine
gained `session::reset::RoomReplayRequested` (generic) +
`ContentDialogueFollowupSet`; the cut-rope emitter/reset systems moved to
`AmbitionBossContentPlugin` on the labeled slots; the app's replay consumer
is the generic `apply_room_replay_request_system`; `CutRopeRoomReplayRequested`
deleted. E4 prep: all `ambition_render::fx` type imports repointed to
`ambition_vfx` (the types had already moved; the facade was the residue);
`CameraViewState` init moved to the presentation half; cut-rope resources
now initialized by the content plugin (anti-god rule 5 sweeps).
ALSO: disk-full incident — `/home/joncrall/ambition-target/debug` had
grown to 351G; deleted (pure build cache; one full rebuild), plus the
config-blessed stale repo `target/` (13G) and >1-day-old `debug_traces/`
(5.6G). Flag for Jon: consider a cron/`cargo-sweep` or periodic
`rm -rf $CARGO_TARGET_DIR/debug` to stop the target dir re-ballooning.
The extension-crate ruling (kaleidoscope) is recorded in architecture.md
Tier 6 + the E1e card.

## 2026-07-06 (opus) — CM1: the knockback-scaling axis (parity-pinned)
Goal: maximally unblock fable by walking the opus ladders (CM/CC/CM7/E4
slices/E5-finish 5–6) so CM4/CC5/E4-flip flip to ready. First slice landed:
CM1. `HitVolume` grew `kb_growth` + `launch_dir` (both serde-default); the
archetype schema + `ActorTuning` grew `weight` (default 1.0) + `death_policy`
(`DeathPolicy::{HpDepleted default, Unbounded}`); `BodyHealth::damage_taken()`
exposes the smash-percent meter off the existing pool (no parallel state). The
scaling is a pure `combat::damage::scaled_knockback(base, growth, damage_taken,
weight)` applied VICTIM-SIDE at the moveset-hitbox overlap — the ONE
growth-carrying path (aggressor/player/boss/hazard volumes stay flat), reading
the victim's `BodyHealth` + `ActorConfig.tuning.weight` (both `Option` → player
weight 1.0). `DeathPolicy::kills_at_max()` gates the actor kill path so an
`Unbounded` body never dies from its meter. All defaults are byte-parity
(`growth=0`, `HpDepleted`, `weight=1.0`); C4 conjugation-under-gravity + scaling
+ parity tests green. gameplay_core combat 104/104, vfx + entity_catalog green,
app rl_sim gate build clean. Next: CM2 (launch DI off `ActorControl`).
