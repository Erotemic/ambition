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

1. **E4 — the observation-boundary carve** — ✅ **SLICES 1–20 + THE
   MINT DONE (fable 2026-07-06 evening)** except slice 8 (the
   `BossAnimator` render-insert — E6(a) territory by design).
   `crates/ambition_sim_view` is real: pose/anim/feature/boss/
   nameplate/hud/item/prop/camera read-models, rebuilt in the sim
   tail; render is a pure consumer and the
   `observation_boundary` test forbids ~45 live sim-state type names
   in render sources forever. Remaining E4 tail: the full dep-flip
   (render drops gameplay_core) is gated on E1/E3/E-assets/W — the
   remaining imports are vocabulary+assets, not sim state.
2. ✅ **`SimSnapshot` design** — identity + scope pinned in netcode.md
   N3.1 (SimId vocabulary, include/exclude lists, derived-state rule),
   2026-07-06.
3. ✅ **CM4 cancel tables** — LANDED (fable, 2026-07-06).
4. ✅ **CC5 `PortalFrame`** — LANDED + CC1 COMPLETE (fable, 2026-07-06).
5. ✅ **FB6 rollout architecture** — budget contract pinned in
   fighter-brain.md §5 (2 ms cap, scratch-world seeding, calibration
   instrument), 2026-07-06.
**Post-fable escalation protocol (fable's availability has ended —
2026-07-06 night).** The old valve ("escalate to fable at the first
ambiguous item") is CLOSED; its two named consumers are pre-answered
(E2's back-edges are classified verdict-by-verdict in decomposition.md;
W3's ambiguities are covered by the W-a…W-e rulings + the 5-step W
queue). When an executor still hits a genuine design ambiguity (a case
the rulings don't cover, not a case that's merely hard):
1. do NOT improvise doctrine and do NOT block — park that slice;
2. write a DECISION BRIEF for Jon in this file (options, consequences,
   one recommendation — the Q4 brief in netcode.md is the template);
3. continue with the nearest unambiguous work.
Everything on this page is opus-or-below by design.

### 📋 LAST-CHANCE FABLE QUESTION REGISTER — ✅ ALL RULED (fable, 2026-07-06 night)

Every fable-owned design decision is now closed. **[W-a]–[W-e] are RULED
in decomposition.md's W-track block** (Tier-0 home = entity_catalog —
serde-only, NEVER deps engine_core: pure enums move whole,
`KinematicPath` → engine_core with the geometry vocabulary,
`DamageVolume`/`Damage` dissolve into `PlacementRecord`+`HazardSpec` at
lowering, `HazardRespawn` rename; two-stage registry with the pinned
`PlacementRecord`/`register_placement_interpreter` API; `WorldDelta` =
ordered ops, SimView sees composited only + `WorldGeometryVersion`;
placement ids REQUIRED at the record layer; unknown placement = hard
error). The W execution queue (5 ordered OPUS-SAFE steps) is written
there. **No open fable design questions remain on this page.** Remaining
fable-graded material is EXECUTION-hard, not design-open — see the
"remaining fable-tier surface" note in the 2026-07-06-night execution-log
entry.

**Decomposition risk map + the fable-reserve recommendation (fable,
2026-07-06 night — answering "is any of the 64k carve list still
fable-hard?"):** NO item is design-hard anymore; risk now concentrates in
EXECUTION at exactly two places, both mitigated by sequencing (the D2
rule: cycles die in place first, atomic moves second):
1. **W2's `RoomEmission`/`PlacementRecord` payload reshaping** — the one
   commit whose shape propagates into every converter. Sequence: W-queue
   step 1 + the E2 in-place verdicts FIRST (small compiling commits),
   then W2, **then pause for ONE review of the W2 commit (Jon or any
   remaining fable budget — a review, not a work session)** before W3/E2
   atomic moves build on it.
2. **E2's atomic move of `combat/`** — near-mechanical once its in-place
   verdict commits land; do not attempt it before them.
Everything else on the carve list is compiler-driven. Estimated total:
~10–14 focused opus sessions for the full 64k, one dedicated session per
crate mint (never the tail of a multi-item run).

**Opus-sequencing notes (unchanged):** the **E4 full dep-flip** is
OPUS-SAFE-once-gated (Q32) behind E1/E3/E-assets/W. **E5 step 5 is DONE**
(see the log; step 6 is DONE — the demo shell passes). The
`KinematicPath`→Tier-0 move is W-queue step 1.

### 🟢 OPUS SESSION 2026-07-06 (b) — where the CLEAN opus surface bottomed out

A second opus session cleared the remaining *cleanly-unblocked, autonomously-
safe* opus work (gnu-ton RED, CC2-complete, CM5, progression de-weave, patrol
close — see the execution log). It then evaluated every other open opus item
and found the surface is now uniformly **not** cleanly-opus. The accounting
(so the next agent / fable doesn't re-investigate):

- **E4 slices 1–16** — the identity-bearing ones (esp. #16 `ControlledSubject`
  → `SimView.controlled_body`) CASCADE into render's raw-`Entity`-join identity
  model (nameplates/hud/fx/items compare `entity == controlled.0`). Converting
  to stable-id views is the "riskiest cut" fable is graded for — escalated, not
  forced (matches Jon's "hardest decompositions are fable").
- **E5 step 5** (mint `ambition_host`) — ⚠️ **this bullet was DISPROVEN (opus
  2026-07-06): NOT gated on the dev/menu split.** The menu-input systems are
  `gameplay_core::schedule::input_systems` (host may dep gameplay_core → clean
  lift), and the only app-local pieces are the ones the card already keeps
  app-side + two `dev_runtime` systems. Scaffold + readiness brief landed; the
  only careful part is the `wire_portal_schedule` ordering pins (fable-graded).
  See the "E5 step-5 de-risked" execution-log entry below.
- **Progression move-to-runtime-group** — ✅ DONE at E5 step 5 (the
  plugin moved into `ambition_runtime` wholesale; the old "deeper de-weave"
  concern was moot — its chain was already content-free and names only
  gameplay_core systems, which the runtime may name).
- **CC3 hard oracle** — the diagnostic exists (`collision_invariant_oracle`);
  the hard-asserting form is Jon-deferred (it would RED on the deferred
  embed/OOB bugs). **CC6 moving portals** — substantial guarded portal-physics.
- **Netcode N0.2/N0.3** — carry the "pending Q4 confirmation" gate (Jon's call).
- **E1a–e / E3 / E6 / E7-E8 / E-enc / E-assets** — real opus D-A but each is a
  multi-hour crate-MINT (move 2–4k LOC + repoint every consumer + delete
  facade); starting one without landing it compiling violates "only gate is it
  compiles" (a broken tree is worse than a checkpoint). These want a dedicated
  focused session each, not the tail of a multi-item run.
- **SurfaceRamp converter** — 4-orientation quarter-arc winding in +y-down
  coords; "masquerades as a physics bug in play" if subtly wrong and wants
  VISUAL verification (spatial-authoring discipline) + no room needs it yet.
- **FS1** — bridges the external `bevy_falling_sand` crate, feature-gated (off
  in the gate), low-pri. **BD4** — cataloging/doc. **FB1** — read-only audit.
- **Sanic ball-dash** — ~~gated on E5-finish~~ E5-finish landed later the
  same night → needs only the new release→velocity technique +
  hurtbox-resize seam (sketch in demos/sanic.md), inside the S5 content
  crate.
- **Slash-VFX black square / E3 sprite bugs** — render-side, need a visual run
  (E3-gated). **Dialog-context slice** — "design note first" per its card.

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

3. ✅ **DONE (fable, 2026-07-06 evening)** — CM1 `launch_dir` is consumed:
   the resolver honors a volume-authored launch DIRECTION (normalized,
   victim-gravity-frame, `x` mirrored away-from-source, `y` = up) while
   preserving the feel-tuned launch SPEED (`hypot(knock_x, knock_y) ·
   strength`), so an authored angle can never out-throw the default. The
   angle rides `Hitbox.launch_dir` → `HitKnockback.launch_dir` from the
   moveset volume; every un-authored path stays `None` = byte-parity.
   Composes with CM1 growth (scales `strength`) and CM2 DI (rotates the
   result). Tests: fixed-angle + speed-invariant + side-mirror, C4
   conjugation-under-gravity, degenerate-vector fallback.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | ACTIVE — **E5-finish COMPLETE (fable 2026-07-06 night): step 5 executed (amended: shared sim wiring → `ambition_runtime` per-domain plugins; `ambition_host` = leafwing bindings + camera cluster) + step 6 executed (SimCoreResourcesPlugin split + the demo smoke shell PASSES) — THE DEMO GATE IS OPEN**; W1 STATE-inversion (opus); **W-a…W-e RULED — the 5-step OPUS-SAFE W queue is in decomposition.md**; **E2 back-edges PRE-CLASSIFIED (fable) — verdict list in the E2 card**; **W-queue step 1 DONE + E2 IN-PLACE VERDICTS DONE (opus 2026-07-06 night): entity_catalog::placements + engine_core::kinematic_path minted; all 7 E2 back-edge verdicts landed in-place (CombatTuning minted, banner→message, CenteredAabb/HitEvent/overlay repointed, FeatureSimEntity→lifecycle) — combat's atomic move is now near-mechanical** | **W queue steps 2–5 [opus]** (W2 = the ONE review-point); E2 atomic move (reserved); E1a next crate-mint; E3/E6/E7/E8 open |
| Decomposition D-B/D-C | same | queued behind D-A | mode-scope seam can land early (demos want it) |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | **CC1 COMPLETE + CC5 LANDED (fable) + CC2 COMPLETE (opus, 2026-07-06)** — engine_core::frame vocabulary + cast family registry real in code; CC2 first pass (hazards swept) + completion (§3.3 every reader classified: loading-zone Door/Walk/EdgeExit now swept via `transition_for_player`; water/climbable annotated discrete-OK + `thin_region_warnings` authoring validator; ledge audited; auto-collect N/A) parity suites green | CC3 fuzz rig (§6.1 oracle) [opus]; CC6 moving portals (§5-P2 spec) [opus] |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | CM1 (incl. **launch_dir consumption, fable 2026-07-06 evening** `c695cd9c`)+CM2+CM3+CM7+CM4+CM5 LANDED — per-move presentation authored; smash axes complete (growth, DI, charge, cancel tables, fixed launch angles) | CM6 grab/throw/shield-stun (brings OnBlock) [opus, with SSB] |
| Netcode ladder | [engine/netcode.md](engine/netcode.md) | NEW | N0.2 input-stream type; N0.3 lint set [opus] |
| Fighter brain | [engine/fighter-brain.md](engine/fighter-brain.md) | NEW | FB1 view audit [opus] (CM7 first) |
| Boss pipeline | [engine/boss-design.md](engine/boss-design.md) | NEW | BD4 seed extraction [opus/sonnet]; BD1 after |
| Falling sand | [engine/falling-sand.md](engine/falling-sand.md) | NEW; low priority | FS1 single-owner + conservation [opus] |
| S — Sanic | [demos/sanic.md](demos/sanic.md) | S1–S3 landed; Sanic-in-normal-rooms fixed (`0189338b`); **S5 UNBLOCKED (E5-finish complete 2026-07-06 night; copy the demo-shell fixture)** | S4 proofs; ball-dash technique; S5 demo app [opus] |
| M — Super Mary-O | [demos/super-mary-o.md](demos/super-mary-o.md) | **UNBLOCKED (E5-finish complete 2026-07-06 night)** | M1+A3 powerup-equipment [opus] |
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
| `SurfaceRamp` quarter-circle marker entities (Q27 ruling): generated quarter-arc chain for floor↔wall momentum transitions — **arc math + the 4-case winding-oracle test protocol PINNED in [spatial-model.md](engine/spatial-model.md) §SurfaceRamp** | [the space IR] converters / sanic demo | [opus/sonnet] |
| Per-attack VFX/SFX (not one generic swing) | ✅ CM5 landed (opus 2026-07-06): `swing_sfx`/`swing_vfx` prefab params + `Vfx{effect}` timed event — each authored move sounds/looks distinct | done |
| Morph ball still draws the robot; generalize modal body morphs | E3 (mode→sprite-state row) | [opus] |
| Shrine + glider sprites broken | E3 (rect drift; sprite pipeline) | [opus] |
| All bosses render the generic sheet | E3/E6 — needs a RUN with `boss_sprites.len()` logging; do NOT apply the disproven sprite_target dispatch | [opus] |
| NPCs infinitely respawn | ✅ FIXED — the respawn slice above (ADR 0022) | done |
| Kernel-guide NPC should patrol a home base when peaceful | ✅ ALREADY DONE (verified opus 2026-07-06 — stale item / TODO drift): the `patrol_peaceful` brain preset (`Patrol` radius 64 / speed 28 / **aggressiveness 0** = peaceful home-lane pacer) exists in the brain vocabulary; the kernel-guide catalog row defaults to it (`brain=None` in its central_hub NpcSpawn → catalog default), body-generic. Tests: `patrol_paces_horizontally_around_spawn`, `peaceful_patrol_in_talk_range_holds_and_faces_target`. | done |
| Dialogs don't adapt to WHO is talking (possessed actor gets self-dialogue) | dialog context slice — **design note WRITTEN (fable): at interact-dispatch the seam sets three Yarn variables before running the node — `$speaker_id` (the CONTROLLED body's `config.id` / worn character id), `$listener_id` (the interact target's id), `$speaker_is_self` (= ids equal). Content branches on them (`<<if $speaker_is_self>>`). Engine-side default: when `$speaker_is_self` and the dialogue declares no `self` branch/node (convention: node `<name>__self` if present), the interaction is SUPPRESSED (no dialog opens) — talking to your own body is a no-op unless content authors it. Ids, not display names (entity-id-matches-label rule); display names resolve content-side.** | [opus] |
| Sanic ball-dash special | demos/sanic.md (the one new technique) | [opus] |
| Portal gun should be a normal item (portal crate forgets the gun; one gun = one pair) | decontamination near A2/items; portal exposes `spawn portal of pair P on surface` primitive | [opus, low priority] |
| Smells journal (dev/journals/code_smells.md) | C4-style sweep rides each related track; the journal stays the intake | — |

## 🅿️ PARKED SLICE — DECISION BRIEF: SweepSample's ECS-integration seam (opus 2026-07-06 night)

**Status: PARKED per the post-fable protocol.** The `SweepSample` §3.1 spec
pins the TYPE, the four contract rules, and the does-NOT-carry list exactly —
but it does NOT pin the ECS-INTEGRATION seam, and the code (read this session)
shows that seam carries genuine unresolved design content with high blast
radius on the guarded collision heart. Opus declined to improvise doctrine
here. Steps 1/2/4 of the W-queue/E2/GeoId run landed around it; this is the one
slice that needs a ruling before execution.

**Why it's not "merely hard" (the concrete entanglement, from the code):**
1. **In-kernel discontinuities.** `integrate_home_body`
   (`player/body_integration.rs`) calls `update_body_with_tuning_clusters`
   (the AABB kernel) and then, *in the same call*, does an engine-level
   teleport-to-spawn when `events.reset` fires (`reset_body_clusters(clusters,
   world.spawn)`). BLINK and the momentum solver's snaps also happen INSIDE the
   kernel. So "write the sample AFTER the kernel" (§3.1) would capture a
   prev→curr segment that SPANS a mid-kernel teleport/blink — a spurious path
   the hazard reader would then sweep. §3.2 lists Class-B as portal/room/
   death/scripted-teleport and EXCLUDES blink (kernel Class-A), so blink has no
   pinned reset — yet it is a discontinuity.
2. **The reset surface spans three crates and ~20 sites** (`.pos =` writers:
   engine_core blink, `ambition_portal` transfer, gameplay_core respawn ×2 via
   `reset_body_clusters`, boss reset, mount rider-positioning, mark-recall,
   enemy corner-slide, room transition). Missing ANY one makes the migrated
   hazard reader fire spuriously after that teleport. The spec names only four
   ("portal transfer, teleport, respawn, room transition").
3. **The verification gap.** The reset protocol is only EXERCISED once a reader
   consumes the sample (the hazard migration). So a type+writers+resets mint
   with the reader left on `vel·dt` ships an UNVERIFIED reset protocol (dead
   code CC6 would then trust); migrating the reader is the byte-parity-risky
   change on guarded hazard behavior (the sample's true prev→curr differs from
   `vel·dt` exactly on wall-stop and teleport frames).

**The core question to rule:** does `SweepSample` (a) live in
`BodyClustersMut` so the kernels write/reset it directly at every internal
discontinuity (blink, `events.reset`, momentum snap), or (b) stay a standalone
component written by the two integration SYSTEMS after the kernel returns, with
an explicit rule for in-kernel discontinuities? And: **is blink a sample-reset
(a "scripted teleport") or Class-A motion (no reset, its path is real)?**

**Options:**
- **(A) Cluster-member.** `SweepSample` joins `BodyClustersMut`; the kernels set
  `prev = curr; curr = new_pos` at step end and `prev = curr = new_pos` at every
  internal teleport/blink/reset. PRO: one write site per discontinuity, provably
  complete (the kernel owns every pos write). CON: widens the cluster view +
  every scratch/test constructor; touches the hottest struct in engine_core.
- **(B) System-layer component + blink-is-Class-A.** Standalone component; the
  two integration systems write it post-kernel; the four named Class-B writers
  reset it; blink/`events.reset` are NOT reset (their path is "real"). PRO:
  minimal cluster churn. CON: hazard sweep will graze along blink/reset paths —
  a behavior change that may or may not be byte-parity vs `vel·dt` (depends on
  whether blink zeroes vel); likely trips a portal/gravity/hazard suite, and if
  it does the fix is ad-hoc per writer.
- **(C) System-layer component + blink/reset ARE resets.** Like (B) but the
  integration system detects an in-tick teleport (blink fired / `events.reset`)
  and resets the sample same-tick. PRO: no cluster churn, correct paths. CON:
  the "did a discontinuity happen this tick" signal must be plumbed out of the
  kernel (it is not today — `events` reports `reset` but not `blinked`), so it's
  a new kernel output.

**Recommendation: (A) cluster-member.** The sample IS body motion state; putting
it where every pos write already lives is the only option that makes the reset
protocol PROVABLY complete (the §3.1 completeness requirement) rather than a
grep-audited hope across 20 sites and 3 crates. The cluster-view widening is
mechanical and one-time; it is the same place `BodyKinematics` already lives, so
the kernel writes are local. Do it as: 3a mint + cluster field + kernel writes
(zero readers, byte-parity); 3b reset at internal discontinuities incl. blink
(still zero readers); 3c migrate the hazard reader (full hazard/portal/gravity
suites are the byte-parity gate). If 3c reveals a blink-vs-`vel·dt` parity
delta, THAT is the feel call for Jon (a blinking body grazing a spike), logged
separately. **Needs Jon's (or fable-budget's) ruling on (A) vs the blink
classification before execution — it rewrites the hottest engine struct.**

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

**✅ RED FIXED 2026-07-06 (opus):**
`ambition_content bosses::gnu_ton::tests::arena_spawns_the_adr0020_linked_pair`
— root cause was `68943d28` "Commit loose data": the sandbox.ldtk re-authoring
nulled the rider `BossSpawn-6837`'s `mounted_on` EntityRef (iids preserved,
value → null), so the arena emitted 0 mount links. Restored the EntityRef quad
(→ the still-present `EnemySpawn-6836` giant mount) via `entity set-field`
(tool, not hand-edit); the repair pass reproduces G4's exact `realEditorValues`
shape. gnu_ton module 11/11 green; the ADR-0020 linked pair authors again.

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

## 2026-07-06 (opus) — CC2 COMPLETE: the §3.3 trigger-classification pass
Every path-dependent reader in the §3.3 table now declares its verb.
**Converted to swept:** loading-zone entry — `transition_for_player` takes the
body's frame `delta` and routes through `cast::aabb_path_contacts`, so a fast
body (Sanic/dash/blink) can no longer tunnel an overlap-fire `Walk` band
between frames (tunnel test pins it); the discrete standing-in-it case is
`delta == 0`, byte-preserved; `Door` stays interact-gated so the sweep only
ever helps it. **Annotated discrete-OK + authoring-swept:** water/climbable
region entry — the per-frame `water_at`/`climbable_at` reads stay discrete
(ENTER/EXIT state, RL-hot), with `World::thin_region_warnings` (floor =
`MAX_EXPECTED_BODY_SPEED/60 = 26px`) wired into room `layout_warnings` to flag
tunnelable thin strips at authoring time. **Audited:** ledge grab fires off a
resolved wall contact (Class-A kernel output), swept by construction —
annotated at the probe. **N/A:** no auto-collect token pickup exists yet
(GroundItem is the only pickup, button-gated); the swept-pattern recipe is
documented in `items/pickup/mod.rs` for whoever adds one. engine_core 249/249
(+thin-region test), rooms 31/31 (+tunnel test), app rl_sim gate build clean.
Next CC-ladder opus work: CC3 fuzz rig (§6.1 oracle), CC6 moving portals.

## 2026-07-06 (opus) — CM5: per-move presentation, authored not hardcoded
The jonnotes "one generic swing everywhere" is dead: presentation is DATA.
`MoveEventKind::Vfx { effect }` (entity_catalog) is a timed COSMETIC burst
resolved through the content-registered `ambition_vfx::move_vfx_kind`
vocabulary (the shared `ExplosionKind` set — a jab authors `burst_round`, a
smash `shockwave`, a launcher `starburst`). The `simple_melee`/`simple_charge`
prefabs grew `swing_sfx: Option<String>` + `swing_vfx: Option<String>` params
(default `None` → byte-parity; the authored-melee adapter keeps the engine
default). Validation: `MoveSpec::presentation_problems(vfx_known)` — the vfx
vocabulary oracle is INJECTED so entity_catalog stays render-free — runs inside
`MovePrefabRegistry::expand`, so a typo'd cue/effect id fails at the SAME
startup gate a bad prefab key hits (never a silent missing effect). The
content-free dispatcher emits `VfxMessage::Explosion` at the owner. 3 new tests
(authored-vs-parity, typo rejected, dispatch→burst); moveset 28/28,
entity_catalog 14/14, vfx green, app rl_sim gate build clean. NOT closed: the
slash-VFX black-square is a separate render-side sprite-source quirk needing a
visual run. Next CM-ladder opus work: CM6 (grab/throw/shield-stun, with SSB).

## 2026-07-06 (opus) — App residue: the progression-schedule content de-weave
`ProgressionSchedulePlugin` (app-side) was interleaving ENGINE boss-encounter/
save/room systems with CONTENT (cut-rope setup+victory, quest-completion
rewards, the gnu-ton arena gate, quest-registry populate). De-woven on the
E5-step-4 pattern: three engine labeled slots minted in `boss_encounter`
(`ContentEncounterScriptSet` mid-boss-tick, `ContentEncounterVictorySet` after
the boss chain, `ContentQuestRewardSet` after the quest pump), host-anchored via
`configure_sets` at the EXACT former positions; the content systems moved to
`AmbitionBossContentPlugin` / `AmbitionQuestContentPlugin` and register
`.in_set(slot)`. The engine chain now names NO content (anti-god rule 3). The
quest event pump (push/apply) stayed engine — it was a `content::` RE-EXPORT of
gameplay_core systems, not content. Ordering preserved byte-for-byte:
replay-fixture determinism guard + boss_lifecycle 8/8 + all app integration
tests green (only the documented `unified_melee::a_hostile_actor` feel-RED
fails, unrelated); content 64/64, app lib 139/139, gate build clean. Follow-up
(trivial): relocate the now-content-free engine plugin into the runtime group.

## 2026-07-06 (fable, evening) — E4 EXECUTED: slices 1–20 + the `ambition_sim_view` mint
The observation-boundary carve ran to completion in one session, six
commits, each gate-green (`d5675f27`, `07a57205`, `68fd5534`,
`9e1c852d`, `e29c9a90`, `971bb41a`, + the mint):
- **Pose (1–4):** `BodyPoseView` per player-bodied entity (pos/vel/size/
  base/facing/roll/stance/gravity/anim/flash/hp/morph/charge — AJ14
  velocity fields live); `pick_player_anim` runs sim-side only;
  `ShieldRingsView` pools every raised shield. animate_player's
  17-component query is 5.
- **Facts (5–7, 9, 11–16, 18):** `FeatureView` grew alive/flash/hp/
  dummy facts; `BossFrameIndex` (anim state + combat AABB + the
  hazard lane from the SAME volume math as damage); `NameplateIndex`
  (controlled-subject suppression is a sim fact; plates key by id, not
  Entity); `sim_view` facts for hud/held-item/ground-items/shots/
  marks/shrines/gravity-switches/gun-swords/projectiles/dynamic
  features/blink-preview. `ControlledSubject` never appears in render.
- **Back-edges killed:** `regen_player_mana` + the shrine-pulse tick
  moved sim-side; render's `FeatureName` inserts died (`PropVisual.
  name` is the render-local naming fact); the four gate-portal
  presentation systems moved OUT of `gameplay_core::rooms` into
  render; the portal host-adapter glue self-registers in
  `PortalObservationPlugin`/`PortalObservationSet` (slice 20 executed
  as pinned; the `tag_portal_scene_bodies` audit ruled its
  `sync_visuals` pin stale — it tags SIM bodies).
- **The mint (step 4):** `crates/ambition_sim_view` [the observation
  boundary] with camera_snapshot/view_index/anim_index/pose_view/facts
  + both view plugins; runtime group adds them; contract tests moved
  with it. RULING: camera-EASE stays sim-side (boss shake +
  portal-continuity write it; only the resolve observes).
- **The boundary is enforced:** `ambition_render/tests/
  observation_boundary.rs` fails on any render source naming a live
  sim-state type (whole-identifier, comment-stripped). The full
  dep-flip (step 5) awaits E1/E3/E-assets/W (vocabulary+assets moves).
Remaining E4-adjacent: slice 8 (`BossAnimator` insert) rides E6(a).

## 2026-07-06 (fable, evening) — CM1 COMPLETE: authored launch angles (`c695cd9c`)
Hard-problems #3 closed. `launch_dir` semantics pinned IN the resolver:
direction-only (normalized), victim-gravity-frame, x mirrored
away-from-source, y = up; the authored angle replaces the default feel
diagonal while PRESERVING its speed (`hypot(knock_x, knock_y)·strength`),
so an authored angle can never out-throw the feel launch. Threaded
volume→Hitbox→HitKnockback; every other constructor is `None` =
byte-parity. Composes with growth (strength, upstream) and DI (rotation,
downstream). C4 conjugation + speed-invariant + mirror + degenerate-
vector tests. With this, the fable window list is fully ✅ and the
remaining open fable-tier item is E5 step 5 (gated on E1d/E1e) + the
W3/E2 standing escalations.

## 2026-07-06 (opus) — W1 STATE-inversion (`rooms/load.rs`) + W3 vocab-arrow escalated to fable
Read docs/planning to unblock fable. Findings: the fable window is 100% ✅;
the named-next fable item E5 step 5 is genuinely blocked (E1d `dev_tools`
reads deep `actor`/`player`/`world`/`features` sim state, so it can't extract
before E7/W — confirmed the opus accounting). Pivoted to the cleanest fable-
unblocking opus work: the **W-track**. Measured every `world/` upward dep and
found they split by KIND — runtime STATE (a clean W1 invert) vs authored
VOCABULARY (the genuine W3 classification).
- **Landed (this commit):** `world/rooms/load.rs` `load_room_geometry` dropped
  its `PlayerSafetyState`/`PlayerBlinkCameraState`/`DialogState`/`BodyCombat`
  params + `ROOM_DOOR_CAMERA_SNAP_TIME` — the space IR now returns geometry +
  arrival facts only; the composition tier (`ambition_app`
  `room_flow::apply_room_transition_resets`) applies the four cross-domain
  resets from `arrival_pos`+`edge_exit` (anti-god rule 6: no single domain owns
  a transition, so the composer does). Byte-identical: gameplay_core rooms 32/32,
  app `fixture_replays_with_zero_divergence` + `room_spatial_integrity` green,
  app rl_sim gate build clean (only the documented `unified_melee::a_hostile_actor`
  feel-RED fails, unrelated).
- **Escalated to fable (docs, not forced):** the `world → ambition_characters +
  ambition_combat` VOCAB arrow (`RoomEmission`/room-graph carry
  `Authored<CharacterBrain/BossBrain/DamageVolume>` + `KinematicPath`) is an
  undrawn Tier-2 sideways arrow (anti-god rule 4) — the W3 cut can't resolve it
  mechanically (the LDtk backend can't name those types either, `ldtk_map→world`
  only). Full pre-solved option matrix (opaque-IR / draw-the-arrow / sink-specs-
  to-Tier-0 / KinematicPath-is-mis-homed) with an opus recommendation written
  into decomposition.md's "W-track FEEDBACK FOR FABLE" block; W2 is blocked on
  this ruling. Also flagged: `rooms/systems.rs detect_room_transition_system` is
  a sim-tier system that MOVES at W3, not a W1 invert.

## 2026-07-06 (opus) — E5 step-5 de-risked for fable: NOT gated on E1d/E1e + `ambition_host` scaffold
"Do what you can to make fable's work easier." Investigated the E5-step-5 host
mint (the DEMO GATE — the single most valuable fable carve, it unblocks all five
demos) and found the accounting's "gated on E1d/E1e + menu entanglement" was
STALE/overstated. Classified every system in the five movable `register_*` fns
(`ambition_app/src/app/plugins.rs`): the menu-INPUT systems are
`gameplay_core::schedule::input_systems` (host MAY dep gameplay_core → clean
lift, NOT the app menu crate), and the only genuinely app-local systems are the
handful the card already says "stay app-side" (`player_clone`/`home_reset`/
`sync_player_presentation`/`parallax`/`apply` + two `app/dev_runtime.rs` dev
systems). So E5 step 5 is a well-bounded lift with ONE careful part
(`wire_portal_schedule`'s three named-system ordering pins). Landed to make it
easier:
- **`crates/ambition_host` scaffold** — `PlatformerHostPlugins` empty group +
  `HostSeamPlugin` + the per-domain `.add(...)` skeleton commented in, workspace-
  wired, `tests/host_names_no_content.rs` LOCKING the no-content boundary from
  step 0. Builds green, boundary tests pass. Fable's carve is now a pure
  system-move, not crate ceremony.
- **Readiness brief** in decomposition.md (E5 step-5 card): the exact
  host-generic vs app-local split per register-fn + the three portal pins to
  preserve + the note that the parity harness ALREADY EXISTS (the portal/gravity/
  continuity suites catch any ordering break — port boldly).

## 2026-07-06 (fable, night) — E5 STEP 5 EXECUTED + the W-a…W-e last-chance rulings

**E5 step 5 (THE DEMO GATE carve) — done, with one card amendment.** The
card's "move the five register fns to `ambition_host`" was wrong for four
of them: `add_simulation_plugins` is added by headless/RL too (`headless.rs`,
`rl_sim/runtime.rs`, every parity suite), so the shared per-frame SIM wiring
belongs in the ENGINE group — "a headless entry point adds only the engine
group" is the doctrine that decides runtime-vs-host, now recorded in
architecture.md Tier 5. Landed:
- **`ambition_runtime` grew four per-domain plugins** (anti-god rule 2):
  `PlayerSchedulePlugin` (time-control pipeline → input → controlled-subject
  → brains → body-mode → possession → hit events → presentation write-back,
  + the brain-emitter block + required-components registrations),
  `RoomTransitionSchedulePlugin` (detect + feature reset + ContentRoomResetSet
  anchor), `PortalSchedulePlugin` (PortalPlugin + the three ordering
  landmines, behind `portal`), `ProgressionSchedulePlugin` (file moved from
  the app — the documented follow-up).
- **`ambition_host` is real:** `HostInputBindingsPlugin` (leafwing map,
  device→ControlFrame/MenuControlFrame bridge, active-input-kind tracking;
  startup attach anchored on the NEW `SimulationSetupSet` label instead of
  naming the app's setup system) + `HostCameraPlugin` (viewport publish →
  shake → camera_follow; portal camera continuity + PortalObservationPlugin
  under `portal_render`). Boundary test kept.
- **The app keeps only true residue**, pinned into documented ordering SLOTS
  the engine chains leave open: the reset/replay consumers (they call
  app-only `reset_sandbox`; the replay consumer also names content — logged
  as a smell), the player-clone block, home-reset policy + home
  presentation, the room-transition APPLY composer, `sync_preset_input_map`,
  and the debug-overlay chain.
- **Parity:** app rl_sim FULL suite green (portal_bridge/floor-bounce/
  continuity/reset-preserves-authored/lab-usable, gravity trio, phase
  splits, replay-fixture determinism, architecture boundaries ×32 — two
  boundary greps retargeted to the new runtime files, same invariants),
  gameplay_core lib 1175, content 64, observation_boundary, host 2/2.
  Zero behavior change.

**E5 STEP 6 + THE DEMO GATE (same night, second commit):**
`SimCoreResourcesPlugin` minted in the engine group — every engine sim
message + resource default moved out of the app (which keeps only the
catalog/roster install + the setup Startup chain); domain plugins init
their own state (spine indexes, hot-reload watcher-off default, cutscene
channels, trace's portal message); boss/encounter registries are
content-OPTIONAL (empty roster short-circuit + `Option<SandboxLdtkProject>`
— the missing-install panics stay live on paths that resolve real
content); `RoomSpec::new` public. **`ambition_host/tests/
demo_shell_smoke.rs` PASSES: a demo-shaped app (foundation + engine group
+ host group + a fixture content plugin) boots and ticks.** The fixture
is the reference demo assembly (demos/README.md). S5 + the M track
unblock.

**THE OPUS-PROOFING DETAIL PASS (same night, third commit).** A doc
audit swept every planning file for staleness + underspecification; all
findings folded in:
- *Staleness:* P3 demo wave marked UNBLOCKED (roadmap); the retired
  fable-escalation valve scrubbed everywhere (roadmap Q30/Q31,
  architecture §4b, decomposition E4 note); the host-scaffold and
  progression-follow-up notes marked historical/done; demo prereq
  tables show E5-finish ✅; README's fable note updated.
- *Every underspecified gate now carries a pinned design:* **CM6**
  (shield component + held verb in the ONE resolver,
  grab-beats-shield-beats-damage, holds REUSE the ADR-0020
  `ControlGrant`, throws = `throw`-verb family, `HitOutcome::Blocked` is
  CM4's OnBlock fact — combat-model.md §8); **A3 equipment→params** (the
  missing card, written: `ParamModifier{param, op, scope}` folded at
  trigger-resolve, behavioral `grants:`, `ConsumeAsArmor` on-hit policy —
  combat-model.md); **N0.1** (the two-clocks review RULED: fixed 60 Hz
  tick = canonical timeline, bullet-time scales INSIDE the tick, per-tick
  input latching with OR'd edges, FixedUpdate hosting via a threaded
  schedule label — netcode.md); **BD1** (Select/Stance/InterruptRule
  data sketch extending the real `BossPatternStep` — boss-design.md);
  **BD6** (the `FightReport` RON schema + in-band assertions);
  **FB4** (profiles/humanity-checks/ladder rig made concrete);
  **E6(d)** ("cheap" bounded: ≤200 LOC net, no new seams, suites
  unmodified, half-session box); **E7** (Q2 default: `ambition_actors`
  stands if unanswered); **SurfaceRamp** (arc parametrization in
  +y-down + the 4-case winding-oracle test protocol — spatial-model.md);
  **dialog-context** (the `$speaker_id`/`$listener_id`/
  `$speaker_is_self` Yarn contract + suppress-by-default self-talk —
  this file's bug queue); **falling-sand spout** (RULED
  falling-sand-specific `SpoutSpec`, no generic emitter — the W-queue
  step-3 proof case).
No `QUESTION FOR FABLE` markers remain anywhere in docs/planning.

**GPT-5.5 FOLLOW-UP QUESTIONS RESOLVED (same night, fourth commit —
geometry identity + collision-contract status).** Ten identity/status
questions answered IN the docs:
- **`GeoId` — the durable geometry-identity model is RULED**
  (collision-and-ccd.md §3.6): two-level `{source, index}` with sources
  Placement / TileLayer (row-major-deterministic merge ordinal) /
  Generator (marker placement + emission ordinal) / Delta (op sequence
  number) / Anon (fixtures only; rejected in deltas). `Block` gains
  `id: GeoId` (name stays the label). Faces/local positions =
  `GeoFaceRef { geo, face (Top/Bottom/Left/Right | Segment(k)), along:
  px-from-face-center }`. Carve pieces/split blocks are DERIVED —
  frame-local identity only, never persisted. Incremental introduction:
  CC6 mints it; W2 adds the IR sources.
- **CC6 `PortalHostRef` = `GeoFaceRef`** — explicitly NOT an entity,
  NOT a composed-world index, NOT a bare placement id; static and
  moving hosts share the one representation (§5-P2 updated).
- **`WorldDelta` ops name authored `GeoId`s** (`RemoveBlock(GeoId)`;
  `AddBlock` mints `GeoSource::Delta{op_index}`) — decomposition W-c +
  architecture §5 updated.
- **`SweepSample` status pinned:** NOT in code (spec-only; nearest
  machinery = `PortalSweepAnchor` + the CC2 `vel·dt` tolerance);
  CC2-completion mints it. Its fields are CLOSED (prev/curr/vel/half)
  with an explicit does-NOT-carry list (no kernel tag, no chart/portal
  context, no contact context) + the two-consumer growth rule.
- **CC3 status pinned:** the 3-check diagnostic
  (`collision_invariant_oracle.rs`) EXISTS; the CC3 slice is the
  enumerated DELTA to the six-invariant oracle (carve-aware embed,
  straddle rule, Class-B counter, one-way rule, all-rooms matrix).
- **Minimum diagnostic trace payload pinned:** `(seed, room, tick,
  invariant #, SimId+archetype+MotionModel)` + the existing
  BodyKinematics ring + the involved `GeoId`; richer channels join with
  the machinery that creates them; every field uses durable ids, never
  `Entity`.

**The last-chance register is CLOSED:** W-a…W-e all ruled (see
decomposition.md W-track). Highlights: the Tier-0 catalog stays
serde-only (NEVER deps engine_core — the HitVolume plain-fields idiom is
the law): pure enums (`CharacterBrain`/`BossBrain`/`HazardRespawn`/
`DamageKind`/`DamageTeam`) move whole; `KinematicPath` → engine_core
(geometry vocabulary); `DamageVolume`/`Damage` DISSOLVE into
`PlacementRecord{id, aabb, schema}` + Tier-0 `HazardSpec` when the hazard
interpreter lands; two-stage registry with the pinned
`register_placement_interpreter(kind, fn)` API (engine AND content
register; duplicate = panic); `WorldDelta` = ordered persisted ops,
SimView observes composited-only + `WorldGeometryVersion`; placement ids
REQUIRED now (LDtk iid / bake-synth — W-c's RemovePlacement forces them);
unknown placement = hard error naming the registered kinds. A 5-step
ordered OPUS-SAFE W execution queue replaces the questions.
