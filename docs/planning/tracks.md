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

1. **E4 — the observation-boundary carve** — ⏳ IN PROGRESS: slices 17
   (camera seam) + **19 (pose read-model sim-side, fable 2026-07-06)**
   done; slice 20's set-label inversion design pinned in the card
   [opus]; slices 1–16 remain [opus with the card's table]; step 4
   (mint `ambition_sim_view`) after.
2. ✅ **`SimSnapshot` design** — identity + scope pinned in netcode.md
   N3.1 (SimId vocabulary, include/exclude lists, derived-state rule),
   2026-07-06.
3. ✅ **CM4 cancel tables** — LANDED (fable, 2026-07-06).
4. ✅ **CC5 `PortalFrame`** — LANDED + CC1 COMPLETE (fable, 2026-07-06).
5. ✅ **FB6 rollout architecture** — budget contract pinned in
   fighter-brain.md §5 (2 ms cap, scratch-world seeding, calibration
   instrument), 2026-07-06.
Standing escalation: W3 (the world two-crate cut) and E2 (back-edge
classification) escalate to fable at the FIRST ambiguous item.
Everything else on this page is opus-or-below by design.

### 🔴 HARD PROBLEMS surfaced for fable (opus hit the ambiguity boundary, 2026-07-06)

Logged while executing the CM/CC/CM7 ladders (opus). These are the items where
the clean path needed a fable-tier decision or touched the most-guarded code —
opus stopped at the safe boundary rather than risk a regression or pre-empt a
fable card. Ranked by how much they gate a fable task.

1. ✅ **RULED (fable, 2026-07-06)** — the CC1→CC5 cast-consolidation rulings
   are written into collision-and-ccd.md §3.4: **(a)** `first_circle_hit` stays
   kernel-private (no extraction, no re-export; a public swept-circle query
   waits for a real external consumer); **(b)** `ray_aabb`/`raycast_solids`/
   `SolidWorldQuery` move DOWN into `engine_core::cast` [opus, mechanical];
   **(c)** the portal-aware cast lands in `cast` WITH CC5 — engine_core owns
   aperture GEOMETRY (`PortalFrame`/`PortalAperture`), ambition_portal keeps
   portal GAMEPLAY. CC5 conventions + migration steps are pinned in §7 of that
   doc; CC1–CC3 explicitly do NOT wait on CC5 (§8 minimum-slice separation).

2. **CM2 DI + CM3 charge each left a feel/input seam that is Jon's, not opus's.**
   DI (`di_max_angle`) defaults OFF — turning it on for a fighter is a feel
   number Jon sets. Partial-charge-on-EARLY-release needs a new
   `attack_held`/`attack_released` signal on `ActorControlFrame` + input mapping
   (a feel + input-layer change); opus wired the scaling so the charge fraction
   already derives from `MovePlayback.t`, but the release TRIGGER is deferred.
   Neither is a blocker; both are one small authored change away and want Jon's
   feel eye.

3. **CM1 `launch_dir` (authored directional launch) needs the ±side knockback
   model reworked into arbitrary 2D launch angles.** `resolved_body_knockback_
   velocity` currently launches along ±`frame.side` with a fixed rise; honoring a
   volume's authored `launch_dir` (smash-style fixed launch angles) is a rework
   of that resolver, not a field read. The field is authored + carried (CM1);
   the consumption was deferred. Small but wants care (it's on the knockback path
   Jon guards) — a fable/opus-with-parity call.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | ACTIVE — E5 first slice `3c70d827`; **E5-finish steps 1–4 LANDED 2026-07-06** (sets+resources+combat schedule into the group; shared headless foundation; cut-rope de-woven via generic `RoomReplayRequested` + labeled slots; E4-prep: fx facade imports repointed, CameraViewState + cut-rope resources re-owned) | E5 step 5 (mint [the windowed host]) + step 6 (smoke shell) [opus]; W/E1/E2/E3/E6/E7/E8 open |
| Decomposition D-B/D-C | same | queued behind D-A | mode-scope seam can land early (demos want it) |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | **CC1 COMPLETE + CC5 LANDED (fable, 2026-07-06)** — engine_core::frame vocabulary + cast family registry real in code (ray tier moved down, portal-aware cast in `cast`, wrapper in portal crate), parity suites green; CC2 first pass landed earlier same day | CC2 completion (§3.3 table) [opus]; CC3 fuzz rig (§6.1 oracle) [opus]; CC6 moving portals (§5-P2 spec) [opus] |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | CM1+CM2+CM3+CM7+**CM4 (fable, 2026-07-06)** LANDED — the full cancel/chain table rides the move timeline (`Cancelable{into, condition}`), parity-pinned, connect-fact wired through the real hit path | CM5 per-move sfx/vfx [opus]; CM6 grab/throw/shield-stun (brings OnBlock) [opus, with SSB] |
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

**⚠ Pre-existing RED found 2026-07-06 (fable, during CC5 parity runs — fails
at `6e48e598` with a clean tree, unrelated to the CC work):**
`ambition_content bosses::gnu_ton::tests::arena_spawns_the_adr0020_linked_pair`
— the reauthored arena emits 0 mount links (expects 1 rider→mount). Likely
fallout of a recent content/data commit (`68943d28` "Commit loose data" or the
falling-sand room edit touching shared LDtk). Needs an [opus] look: re-run
`world validate`, check the gnu arena's mount-link entity fields.

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

## 2026-07-06 (opus) — CM2: directional influence (DI), parity-pinned
The victim's held control now rotates its OWN knockback launch. Pure
`combat::damage::di_adjust(launch, di_input_local, gravity_dir, max_angle)`
turns the resolved launch TOWARD the held `ActorControl.locomotion`, weighted by
how perpendicular the input is (you can't DI along your own launch) and by the
throttle, bounded by the new `SandboxFeelTuning.di_max_angle`. DEFAULT `0.0` =
DI off (Ambition PvE unchanged, byte-parity); Super Smash Siblings authors a
smash-like ≈0.31 (18°). Because it reads the SAME gated input every system reads,
a level-9 CPU / RL policy DIs like a human — wired via a localized
`Option<&ActorControl>` on the two knockback-consumer SYSTEM queries (player_q +
the actor-damage query), NOT the shared `BodyClustersMut`/`ActorMut` views, and
threaded through `resolved_body_knockback_velocity` → `apply_body_hit_reaction`
→ `apply_player_knockback`/`apply_actor_hit`. Tests: inert-at-zero parity,
rotate-toward-bounded, cannot-DI-along-launch, C4 conjugation-under-gravity
(confirms `AccelerationFrame` is a consistent rotation). gameplay_core lib
1165/1165 green, app rl_sim gate build clean. RL survival-extension assertion
deferred to the FB self-play rig; `launch_dir` full directional launch deferred
to CM3. Next: CM3 (smash/charge release scaling + verb-map smash class).

## 2026-07-06 (opus) — CM3: smash-charge scaling + smash verb class → CM4 UNBLOCKED
`MoveSpec.smash_charge_mult` (data, default 1.0 → parity) + `charge_fraction_at(t)`/
`charge_scale_at(t)` on `MoveSpec` — the charge state IS the move's clock
(`MovePlayback.t`), no new component (as specced). `advance_move_playback` scales
the spawned hitbox's damage + knockback by `charge_scale_at(t)` (interpolates
`1.0 → smash_charge_mult` by how far the owner's clock advanced through the
leading Startup window). `simple_charge` prefab exposes the mult. The "verb-map
smash class" is already expressible per AJ1 (MORE VERBS): the generic `verbs` map
+ `directional_verb_chain(base="smash")` resolve smash verbs distinctly from
tilt/`attack` — a test proves it; flick-vs-hold input is per-game (SSB). Tests:
charge interpolation + parity + no-startup-window + smash-verb resolution + a
runtime charged-hitbox-doubling. entity_catalog 12/12, gameplay_core lib
1165/1165 green. Partial-charge-on-early-release awaits an `attack_held/released`
control signal (input+feel, Jon's); the fraction already derives from `t`.
**With CM1–CM3 landed, CM4 (cancel tables, fable) is UNBLOCKED.** Next on the CM
ladder: CM5 (per-move sfx/vfx) [opus]. Moving to the CC ladder (→ unblocks CC5)
+ CM7.

## 2026-07-06 (opus) — CM7 frame-data + CC1 (partial) + hard-problems log for fable
CM7 landed (`800419ff`): `MoveSpec::frame_data() -> MoveFrameData` (pure
derivation in entity_catalog; startup/active/recovery/cancel windows + reach) —
feeds FB2. CC1 landed to its SAFE BOUNDARY: `ambition_engine_core::cast` minted
as the swept-primitive API surface (re-exports `AabbExt`/`AabbSweepHit` + public
`body_sweep()` entry delegating to `World::first_body_sweep`; external
platformer caller repointed; engine_core 236/236 green, no behavior change). The
FULL CC1 consolidation is NOT mechanical — the four cast primitives span three
crates at different layers and the portal-aware cast needs CC5's `PortalFrame`
aperture type to live in `cast` without inverting layering; opus stopped at the
boundary and logged the three fable rulings this needs (see the HARD PROBLEMS
section at the top of this file). Per "log hard problems for fable" (Jon,
2026-07-06): the CC-ladder cast endgame, the DI/charge feel+input seams, and
CM1's `launch_dir` resolver rework are all recorded there.
STOP POINT (Jon): CM1/CM2/CM3/CM7 fully landed + CC1 partial; combat side of
fable maximally unblocked (CM4 ready, FB2 fed); CC/E4/E5 remain and the CC
endgame is now a documented fable call.

## 2026-07-06 (fable) — THE RUNTIME-CONTRACT PASS (GPT-5.5 review folded in)
collision-and-ccd.md REWRITTEN with the pinned contracts: §3.1 canonical
`SweepSample` (true prev, reset-on-teleport protocol, one-chart rule; hazard
`vel·dt` drift flagged for CC2-completion), §3.2 authority classes A/B/C +
one-Class-B-per-frame ordering (death > transition > portal), §3.3 the
per-trigger semantics table + `AMBITION_REVIEW(discrete_ok)` convention,
§3.4 `cast` identity + family registry + the three CC1 rulings, §3.5
portal-aware cast semantics, §4 SurfacePolygon per-consumer solidity + AABB
slope rules pinned pre-implementation, §5 moving-portal object model
(host-attached, authoritative velocity, update order, edge-case rulings) +
angled-portal P3a/P3b scope split + piece-geometry ruling, §6 exact fuzz
oracle (6 illegal states) + required traces, §7 CC5 exact frame conventions
(tangent derived = rot90(normal); reflection map (s,d)→(s,−d), rotation
(s,d)→(−s,−d); convention explicit at engine layer; zero-tolerance parity)
+ the PortalFrame/PortalAperture ownership-migration ruling, §8 minimum-slice
separation. netcode.md N3.1 grew identity+scope pins (SimId vocabulary,
include/exclude lists, derived-state rule); decomposition.md E4 sketch grew
view identity/dedup/prop-ownership pins; boss-design.md grew calibration v0
(bands, arena assumptions, error-vs-warning); fighter-brain.md grew the FB6
budget contract (2ms cap, scratch-world seeding, weight-calibration
instrument). HARD PROBLEM 1 → RULED.

## 2026-07-06 (opus) — CC2 first pass: the swept trigger primitive + hazards
Added `cast::aabb_path_contacts(center, half, delta, target)` — THE trigger-tier
swept primitive (collision-and-ccd.md §2): preserves the discrete standing-in-it
case exactly (parity for slow bodies) and ADDS the swept path, so a fast body
can't tunnel a thin trigger volume. Unit-tested (tunnel caught, near-miss
ignored, discrete preserved). CONVERTED hazard touch — player AND actor victims
now sweep their frame path (both bodies, relativity principle; a Sanic-speed body
or a lured actor can't leap a spike without being spiked); a tunneling test
proves a body that ends CLEAR of a hazard but crossed it takes the hit. The
classification audit (hazards→swept done; blink→already swept; GroundItem
pickup→discrete-OK button-gated, annotated in code; remaining: auto-coins,
mid-room doors, water/climbable regions, ledge) is written into the CC2 slice row
for fable's review. engine_core 239/239 (+3 cast tests), gameplay_core 1167/1167
green. The CC2 completion pass classifies+converts the remaining readers.

## 2026-07-06 (fable) — THE FABLE-WINDOW EXECUTION RUN (contracts + CC5/CC1 + CM4 + E4-19)
One session, four commits after the contract pass (`cdb0e5c8`):
- **CC5 + CC1 COMPLETE** (`9aa4d998`, `a413f4b2`, `2c74a3f4`):
  `engine_core::frame` minted (PortalFrame {origin, normal, velocity},
  tangent DERIVED, PortalAperture, explicit MapConvention, Galilean
  map_velocity); platformer math delegates to the ONE implementation;
  ray tier moved down into `cast` (world_query.rs deleted); the old
  pieces::PortalFrame REPLACED across portal + presentation (frame-only
  vs opening-aware signatures separated); `cast::ray_through_apertures`
  landed with §3.5 segment semantics + the flush-mount tie-break
  (aperture wins t == solid_t — the old thick-box behavior, now
  explicit). Full parity: portal 46, presentation 45, engine_core 248,
  gameplay 1167→1174, app rl_sim green. CC6 may now read velocity.
- **CM4** (`ef132da9`): cancel tables ride the timeline
  (Cancelable{into, condition}); OnBlock deliberately deferred to CM6's
  shield fact; landed_hit wired through the REAL hit path; jump/dash
  end-early escapes; byte-parity reject without windows (tested, 7 new).
- **E4 slice 19** (`65606d5b`): pose read-model rebuilds sim-side in
  FeatureViewSyncSchedulePlugin; render is a pure consumer; slice 20
  design pinned for opus.
Found during parity runs: pre-existing content RED (gnu_ton mount link,
logged above). Next fable-tier work: E4 slices 1–16 are opus-per-table;
the remaining fable calls are E4's final flip (step 5) + W3/E2
escalations when opus hits them.
