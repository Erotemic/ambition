# TRACKS — the live work queue + execution log

**This is the execution front-end.** Every open work item in the project
is here or in a doc this file points to; the archived review docs
(`docs/archive/reviews/`) are history, not tasking. Executor rules:
[`vision.md`](vision.md) §7 (grades, the deviation rule);
[`decision-principles.md`](decision-principles.md) for autonomous calls.
Append execution-log entries at the bottom; keep statuses current; Jon
can only read, not ask.

Standing verification gate: `cargo build -p ambition_app --features
rl_sim` + ambition_actors lib + content + app rl_sim suites + boundary
tests.

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
1. ✅ **W2's `RoomEmission`/`PlacementRecord` payload reshaping — EXECUTED
   BY FABLE DIRECTLY (2026-07-07, commits W2.1–W2.4)**, which satisfies
   the review clause more strongly than a review would (the clause
   existed to catch shape errors in an opus execution). See the amended
   W2 card in decomposition.md — one ruling changed in execution:
   `GeoSource` (§3.6) is the provenance enum; no `SpatialSource` was
   minted. W3 may now build on the landed shape.
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
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | ACTIVE — **E5-finish COMPLETE (fable 2026-07-06 night): step 5 executed (amended: shared sim wiring → `ambition_runtime` per-domain plugins; `ambition_host` = leafwing bindings + camera cluster) + step 6 executed (SimCoreResourcesPlugin split + the demo smoke shell PASSES) — THE DEMO GATE IS OPEN**; W1 STATE-inversion (opus); **W-a…W-e RULED — the 5-step OPUS-SAFE W queue is in decomposition.md**; **E2 back-edges PRE-CLASSIFIED (fable) — verdict list in the E2 card**; **W-queue step 1 DONE + E2 IN-PLACE VERDICTS DONE (opus 2026-07-06 night): entity_catalog::placements + engine_core::kinematic_path minted; all 7 E2 back-edge verdicts landed in-place (CombatTuning minted, banner→message, CenteredAabb/HitEvent/overlay repointed, FeatureSimEntity→lifecycle) — combat's atomic move is now near-mechanical**; **E1a persistence carve ✅ DONE (Codex 2026-07-06): `ambition_persistence` owns save/settings/quest stored shapes; actor sim retains only settings/menu IR + dev persistence adapters for E1e/E1d**; **E1b audio carve ✅ DONE (Codex 2026-07-06): reusable SFX-bank loader/drain moved to `ambition_audio`; dead encounter-music fallback deleted; remaining audio/music files are sandbox adapters**; **E-assets catalog/source carve ✅ DONE (Codex 2026-07-07): `ambition_asset_manager::sandbox_assets` owns `SandboxAssetCatalog`, ids, catalog builders, and embedded-source plugin; actor sim keeps only input assembly + `game_assets` presentation tail**; **W step 3 lowering-registry proof ✅ DONE (Codex 2026-07-07): `PlacementKind`/`LoweringCtx`/registry + [W-e] hard error landed; hazards lower through the registry while legacy channels dissolve branch-by-branch**; **W3/W4 crate split + ratchet ✅ DONE first cut (Codex 2026-07-07): `ambition_world` owns room IR/platform math/placements, `ambition_ldtk_map` owns LDtk, ADR 0021 + boundary test landed; legacy typed family lists remain as explicit placement-dissolution residue**; **E3 character sprite-sheet slice ✅ DONE (Codex 2026-07-07): `ambition_sprite_sheet::character` owns `CharacterAnim`, sheet specs/geometry, animator, baked RON/pack tables; actor sim keeps roster-aware loader + body-state pickers**; **E-enc state/vocabulary slice ✅ DONE (Codex 2026-07-07): `ambition_encounter` owns wave specs/state/events/registry/music/reward math; actor sim keeps ECS/LDtk adapters**; **E6 boss tail ✅ CLOSED (Codex 2026-07-07): sim-side frame + direct target context landed; BossAnim row fold and the two deep folds closed by code-site policy comments**; **E7 rename slice ✅ DONE (Codex 2026-07-07): `ambition_gameplay_core` package/path renamed to `ambition_actors` and workspace/import references repointed**; **E8 catalog/UI slice ✅ DONE (Codex 2026-07-07): `ambition_items` owns item catalog/shop/inventory UI state; pickup/persist adapters stay in actor sim**; **E7 workspace re-home ✅ DONE (Codex 2026-07-07): `ambition_app` + `ambition_content` moved under `game/`, with workspace/path/docs/tool references repointed**; **E4 dialog dep cleanup ✅ DONE (Codex 2026-07-07): render reads `DialogState`/`DialogChoiceSlot` directly from `ambition_dialog`**; **F1.1/F3 world-purity ratchet ✅ FIRST CUT (Codex 2026-07-07): RON-room loader/source rows moved to `ambition_world`, room-transition SFX is now a plain world cue id, and `ambition_world` has an explicit dependency allow-list test**; **F1.2 portal facade cleanup ✅ DONE (Codex 2026-07-07): `ambition_actors::portal` deleted; consumers import `ambition_portal`, `ambition_portal_presentation`, or `ambition_host::portal` directly**; **F1.3 vfx side cleanup ✅ DONE (Codex 2026-07-07): `ambition_vfx::HitSide` replaces the `ActorFaction` dep in effect/hitbox messages, mapped at combat emit/resolution edges with a boundary ratchet** | **W2 ✅ EXECUTED (fable 2026-07-07 — see the log; GeoSource subsumed SpatialSource)**; W cleanup = legacy typed family dissolution through placement registry; **F1.1 remaining arrows: dissolve hazards/interactables/portals through placement records, deleting combat/interaction/portal from `ambition_world`'s allow-list one at a time**; **E2 combat carve ✅ EXECUTED (fable 2026-07-07 — the kit IS ambition_combat; see the log)**; **projectiles model ✅ DONE (opus 2026-07-07 — `ambition_projectiles` owns the shot vocabulary; the victim/world/anim steppers stay in the sim heart blocked on boss types E6, see the log)**; E7 features-hub facade dissolution + E4 final dep-flip blockers (GameAssets/world/schedule live presentation data) |
| Decomposition D-B/D-C | same | queued behind D-A | mode-scope seam can land early (demos want it) |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | **CC1 COMPLETE + CC5 LANDED (fable) + CC2 COMPLETE + §3.6 GeoId/GeoFaceRef SUBSTRATE MINTED (opus, 2026-07-06)** — SweepSample §3.1 PARKED (decision brief above); GeoId types + Block.id (Anon default, byte-parity) real in code, first consumer = CC6 — engine_core::frame vocabulary + cast family registry real in code; CC2 first pass (hazards swept) + completion (§3.3 every reader classified: loading-zone Door/Walk/EdgeExit now swept via `transition_for_player`; water/climbable annotated discrete-OK + `thin_region_warnings` authoring validator; ledge audited; auto-collect N/A) parity suites green | CC3 fuzz rig (§6.1 oracle) [opus]; CC6 moving portals (§5-P2 spec) [opus] |
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

## ✅ RESOLVED + EXECUTED (fable 2026-07-07) — the SweepSample brief below

The parked slice is LANDED. The ruling refined option (A) into something
stronger than any of the three options: **the sample is the simulation
phase's OWN integration segment, both endpoints captured INSIDE the
kernel** (`prev` at sim-phase entry, `curr` at exit). Under those
semantics the brief's core problems dissolve:
- The **~20-site reset surface does not exist** — a position change
  outside the sim window (blink, respawn wrapper, portal, mark-recall,
  mount positioning, room transition) can never become path, so external
  writers have NO protocol to violate. Provable completeness without
  touching them.
- **Blink is ruled a teleport, never path** (its design identity is
  crossing gaps without traversing them) — and the control/sim phase
  split enforces that for free, no kernel flag needed.
- The **cluster-view widening is `Option<&mut SweepSample>`** — zero
  scratch/seed/test churn (24 scratch literals untouched); spawned bodies
  get the component via `AncillaryMovementBundle` (players AND actors,
  one edit); non-pipeline movers (surface-walker branch, home momentum
  path) write their own segments; `reset_body_clusters` leaves a
  zero-length record.
- The **hazard reader migrated** (both arms): `sample.delta()` with the
  historical `vel·dt` fallback for sample-less bodies (bosses — same
  effective behavior as before). 4 new engine contract tests; full gate
  green. CC6's relative sweep now has its substrate.
The original brief is preserved below for the reasoning record.

## 🅿️ PARKED SLICE — DECISION BRIEF: SweepSample's ECS-integration seam (opus 2026-07-06 night) — ✅ RESOLVED ABOVE

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
- **`SweepSample` status pinned:** IN CODE. Its fields are CLOSED
  (prev/curr/vel/half) with an explicit does-NOT-carry list (no kernel tag,
  no chart/portal context, no contact context) + the two-consumer growth rule.
  The old portal-local anchor has been retired; portal CCD consumes the shared
  sample when its live endpoint still matches the body.
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

## 2026-07-06 (opus) — the decomposition-unblock run: W-a + E2 in-place + GeoId (+ SweepSample parked)
Executed the ordered arc that makes the RESERVED risky carves (W2, E2's
atomic move, CC6) cheap — each step in-place/additive, gate-green, one focused
commit at a time. Landed:
- **W-queue step 1 (W-a):** minted `ambition_entity_catalog::placements`
  (`CharacterBrain`/`BossBrain`/`DamageKind`/`DamageTeam`/`HazardRespawn`) +
  `ambition_engine_core::kinematic_path` (`KinematicPath`/`KinematicPathMode`);
  repointed every consumer (bulk full-path sed + 5 grouped-brace uses + the two
  `DamageKind` sites); deleted the old defs; NO shims; entity_catalog added as a
  dep of combat/interaction/content/sim_view/app.
- **E2 in-place verdicts (all 7, one commit each, byte-parity):** CenteredAabb +
  HitEvent/HitTarget + FeatureEcsWorldOverlay off the `crate::features` hub →
  combat-local paths; FactionRelations/FriendlyFire init owned by combat (rule 5);
  minted `combat::CombatTuning{weight}` written at the two actor-spawn choke points
  + the hitbox weight read converted off `Option<&ActorConfig>`; `damage.rs` writes
  `GameplayBannerRequested` not the UI resource; `FeatureSimEntity` →
  `platformer_primitives::lifecycle`. Combat's atomic move is now near-mechanical.
- **W-queue step 4 (GeoId):** minted the `geo_id` substrate (`PlacementId`/
  `GeoSource`/`GeoId`/`Face`/`GeoFaceRef`) + `Block.id` (Anon default, inert →
  byte-parity). No consumer; real sources deferred to W2/CC6 per §3.6 rule 4.
- **CC2 filler:** GroundItem pickup now carries the grep-able
  `AMBITION_REVIEW(discrete_ok)` marker.
**PARKED — W-queue step 3 (SweepSample §3.1):** genuine ECS-integration-seam
ambiguity + guarded-code blast radius (in-kernel blink/`events.reset`
discontinuities; ~20-site reset surface across 3 crates; reset protocol
unverifiable until the byte-parity-risky hazard migration). Full decision brief
above (recommendation: SweepSample as a `BodyClustersMut` member so the reset
protocol is provably complete) — needs Jon's / fable-budget's ruling before it
touches the hottest engine struct.
**NOT STARTED (correctly) — E-assets + E1b:** re-measured as NOT mechanical
(both reach up into session/persistence/features/rooms/encounter/combat — a
cycle if moved into their Tier-1 target crates); each is a dedicated
invert-then-move session, not a filler. Ledger rows corrected.
Gate every commit: gameplay_core lib 1175, engine_core 252, content 64, full app
rl_sim suite green (only the documented `unified_melee::a_hostile_actor` feel-RED).

## 2026-07-07 (fable) — SweepSample RULED + LANDED (the parked slice closed; CC6 fully unblocked)
Opus's decision brief was exactly right to park — and the ruling that closed
it is stronger than any of its three options: **the sample is the simulation
phase's OWN integration segment, both endpoints captured INSIDE the kernel**
(`prev` at sim-phase entry, `curr` at exit, written by
`update_body_simulation_with_clusters`'s wrapper). Consequences:
- The ~20-site reset-protocol surface DOES NOT EXIST — teleports (blink,
  respawn, portal, mark-recall, mounts, room transitions) happen outside the
  sim window and can never become path. Provably complete with zero external
  cooperation.
- **Blink is ruled a teleport, never path** (crossing gaps without traversing
  them IS blink); the control/sim phase split enforces it for free.
- Plumbing: `engine_core::SweepSample` component; `Option<&mut>` members on
  `BodyClustersMut`/`ActorClusterQueryData`/`ActorMut` (zero churn on the 24
  scratch literals — scratch passes None; tests override the view field);
  `AncillaryMovementBundle` carries the component for players AND actors (one
  edit); the surface-walker branch + the home momentum path write their own
  segments (rule 2); `reset_body_clusters` leaves a zero-length record.
- The hazard reader (both victim arms) migrated to `sample.delta()` with the
  historical `vel·dt` fallback for sample-less bodies (bosses:
  `integrate_boss_bodies` is the known remaining mover — same effective
  behavior as before, a small follow-up slice).
- Tests: 4 new engine contract tests (segment recorded / zero-dt zero-length /
  control-phase blink never path / respawn leaves zero-length-at-spawn).
  Gate: engine_core 256, gameplay_core 1175, content green, app rl_sim 44
  suites (only the documented `unified_melee` feel-RED), host + demo shell
  green.
**CC6 (moving portals) is now fully unblocked for opus** — both prerequisites
(SweepSample §3.1 + GeoId §3.6) are in code; the relative swept trigger reads
`sample` and the host ref is `GeoFaceRef`.

## 2026-07-07 (fable, b) — W2 EXECUTED (the one review-point commit is IN, by fable's own hand)
The reserved risky carve landed as four compiling commits (W2.1–W2.4);
the risk-map review clause is satisfied by fable executing directly.
- **W2.1** — serde across the engine IR spine (`World`/`Block`/chains/
  water/climbable/GeoId family/`KinematicPath`/`CenteredAabb`; `Aabb` =
  `Aabb2d` already serde under bevy_math's `serialize` feature). The
  emission paths now assign REAL GeoIds: IntGrid → level-scoped
  `TileLayer{"{level}/{layer}"}` + row-major merge ordinal (§3.6
  determinism contract, pinned by test); entity-authored blocks →
  `Placement(iid)`. Render's `"ldtk "` name-sniff is DEAD — provenance
  reads `GeoSource::TileLayer`. **Ruling amendment: no `SpatialSource`
  enum — `GeoSource` IS the provenance model; never mint a second one.**
- **W2.2** — `RuntimeEntityEmission` → `RoomEmission` (pure rename; all
  four reference sites internal to `ldtk_world`).
- **W2.3** — the [W-b] record channel: Tier-0 `HazardSpec` (pinned
  plain-pairs shape) + CLOSED `PlacementSchema{Hazard}` in
  `entity_catalog::placements` (+ serde derives the module doc already
  promised); `world::placements::PlacementRecord { id: PlacementId
  ([W-d] REQUIRED), schema, aabb }`; `RoomEmission.placements` +
  `RoomSpec.placements` routed like every family;
  `convert_damage_volume` DUAL-emits (legacy hazard + record twin, same
  iid — pinned by test). Records stay inert until W-queue step 3
  registers the hazard interpreter and deletes the legacy channel.
- **W2.4** — serde across the whole `RoomSpec` tree (interaction/combat/
  portal payload types included; combat + portal grew serde deps);
  `world::ron_room` — `RonRoomDoc { spec, links }` ⇄ RON +
  `WorldManifest.ron_rooms` rows + `to_room_set` append. **THE IR
  PROOF:** the sanic area round-trips serialize∘parse as a string fixed
  point and re-enters a `RoomSet` with no LDtk in the second path; a
  pure-generated `RoomSpec` bakes/reloads (the W4 "second backend"
  seed).
Gate: engine_core 260, gameplay_core lib 1179, content 64+5, interaction/
combat/catalog/characters/portal suites 335, app `--features rl_sim`
builds + full suite green (only the documented `unified_melee` feel-RED).
**W-queue state: steps 1–2 ✅; step 3 (lowering registry) is the next opus
slice; W3's cut now builds on a landed, fable-verified shape.**


## 2026-07-07 (opus, d) — E2 tail: `ambition_projectiles` (the projectile MODEL)
Carved the projectile vocabulary out of `ambition_actors` as two
compiling commits (`feat(E2): mint ambition_projectiles` +
`refactor(E2): repoint projectile-model consumers`). The de-weave
re-measured fable's census and found the real split is model-vs-stepper,
not the 96-ref count (which was the STAYING steppers' upward refs):
- **MOVED** (deps: engine_core + platformer_primitives + portal +
  gameplay_trace + input + serde + bevy — **NOT combat**; the model names
  zero combat vocab): shot kinds + visual-kind art, kind-specific expiry VFX
  cues, `PlayerProjectileState`, the ECS components + enemy spawn state, the
  `SpawnProjectile` pool + the pure player-pool spawner, pure portal transit,
  diagnostics. Later projectile follow-ups also moved the substrate-only
  enemy-pool `Effect::Projectiles` spawn executor into
  `ambition_projectiles::enemy`.
- **STAYS** (victim/world/anim weave — combat's `damage_apply` precedent):
  `step_projectiles` (BOSS types `BossConfig`/`BossClusterRef`/
  `BossAnimationFrameSample` = the E6 blocker, + breakables/actors/`HitEvent`/
  parry-heal), `charge_projectile_input` (`BodyAnimFacts`), and the
  `ProjectileCollisionWorld` overlay param. `gameplay_core::{projectile,
  enemy_projectile}` are thin facades (`pub use ambition_projectiles::{*,enemy::*}`,
  the `ambition_combat as combat` transition-alias precedent).
- The "player = heal/anim" verdict needed NO execution — `PlayerHealRequested`
  + `BodyAnimFacts` are read only by the staying steppers, so they travel at
  E7, not now. Boundary test
  `architecture_boundaries_projectiles_crate_is_model_only`.
Gate: `-p ambition_app --features rl_sim` build green; projectile_portal_transit
+ held_projectile_portal_transit + gameplay_core projectile (54) + gradient_nova
content tests + the new boundary test all pass. **Rechecked after W3/W4
(Codex 2026-07-07): the world-IR/backend blocker is gone; the steppers still
follow E6 (boss carve) / E7 (actors), because the surviving weave is
boss/actor/victim-side sim state rather than projectile model vocabulary.**

## 2026-07-07 (fable, c) — E2 EXECUTED: the combat kit IS `ambition_combat`
The reserved atomic move landed as eleven compiling commits
(E2.8–E2.18 + THE MOVE, 727bafe6). The card's "near-mechanical" claim was
re-measured first: 184 real upward refs, ~25 distinct symbols — the
opus in-place verdicts had covered only the features-hub reads. What
happened, in order:
- **E2.8/E2.9 (mechanical):** 84 hub-path refs died (Body* → engine_core;
  markers/lifecycle/gravity → platformer_primitives; combat-owned types
  reached via `crate::features`/`crate::player` hub paths → their real
  homes); `ENEMY_ATTACK_COOLDOWN` became combat's; `room_spec_paths` →
  spawn side.
- **E2.10:** `PhysicsDebrisCue`/`DebrisBurstMessage` → `ambition_vfx`
  (effect vocabulary, the debris twin of `VfxMessage`); the Avian
  subscriber stays in `world/physics` as the adapter half.
- **E2.11:** `CombatTuning` grew `attack_cooldown_mult` +
  `sprite_character_id` (spawn projects the full actor read surface) and
  the `authored_volumes` INSTALL SEAM was minted — combat asks for
  artist-authored hit polygons through an installed resolver
  (`CombatSchedulePlugin` installs `character_sprites`'s); uninstalled =
  the existing spec-volume fallback. `Option<&ActorConfig>` is GONE from
  combat.
- **E2.12/E2.13/E2.14:** message defs re-homed (`QuestAdvanceRequested`
  → quest, `SwitchActivated`+`SwitchFeature`/`SwitchOn` → encounter);
  `HitSource::PlayerProjectile.kind` DROPPED (never read); `ActorTuning`
  + `CharacterBrainTemplate`/`CharacterBrainSpec` →
  `features::ecs::actor_tuning`; actor `RespawnPolicy` → the Tier-0
  catalog ([W-a]); `DeathPolicy` STAYS combat (CM1 meter-kill law);
  spawn bundles → `features::ecs::actor_bundles`.
- **E2.15/E2.16/E2.17:** the overlay pair (`FeatureEcsWorldOverlay` +
  `CollisionWorld`/`world_with_sandbox_solids`) → `crate::world::
  {overlay, overlay_rebuild}` (W-track material); the glue seven
  (attack=legacy flat swing, damage_apply=victim-side/E7, effect_bus,
  pickups, chests, spawn_static=lowering, boss_clusters=E6) → under
  `features::ecs`; shared predicates/emitters (`body_vulnerable`,
  `shield_blocks_hit`, `scaled_knockback`, `emit_melee_slash`) settled
  in `combat::util`. Combat's upward surface hit ZERO.
- **E2.18 + THE MOVE:** the blade test split along the seam (combat
  tests the seam with a fixture resolver; the real sprite-data assertion
  lives sprites-side), then sixteen modules moved whole into
  `ambition_combat` (joining the model half), kit visibility promoted at
  the crate boundary, deps = interaction/primitives/sfx/time/vfx. ONE
  alias remains (`pub use ambition_combat as combat;` in gameplay_core's
  lib.rs) so `crate::combat::` paths resolve until the features hub
  dissolves at E7/E8 — no other shim.
Gate: ambition_combat 100, gameplay_core lib 1095 (tests traveled with
their code; sprites-side gained the blade data test), app `--features
rl_sim` full suite green (only the documented `unified_melee` feel-RED).
**Projectiles measured: 96 upward refs — a DEDICATED session following
this exact arc (census + order in the E2 card), not a tail item.**

## 2026-07-06 (Codex) — E1a EXECUTED: `ambition_persistence` owns saved shapes

Minted `crates/ambition_persistence` and moved the persistence-owned
surface out of `ambition_actors`: `SandboxSaveData`/`SandboxSave`
I/O, typed `UserSettings` + settings I/O, display-mode vocabulary, and
quest specs/events/registry/save mirroring. Runtime/content/render/app/
touch/sim-view consumers now name `ambition_persistence` directly, so
Bevy has one canonical `SandboxSave`, `UserSettings`, and
`QuestRegistry` resource. The gameplay-core residue is intentionally
small and assigned to later E1 slices: settings/menu IR remains behind
for E1e, and `DeveloperTools` disk persistence remains beside dev tools
until E1d; the room-specific quest producer remains a gameplay-core
adapter over the generic quest event. Added the boundary test
`architecture_boundaries_persistence_crate_owns_stored_shapes_only` to
forbid menu/UI/game machinery imports in the new crate. Gate:
`cargo fmt --check`; `cargo test -p ambition_persistence` (66);
`cargo test -p ambition_actors --lib` (1029);
`cargo test -p ambition_content --all-features` (102+4+1+1);
`cargo check -p ambition_app --features rl_sim`; focused
architecture-boundary test green; `python3 scripts/check_agent_kb.py`;
`python3 scripts/check_doc_links.py`; `cargo run -p ambition_app --bin
headless -- 120`.

## 2026-07-06 (Codex) — E1b EXECUTED: `ambition_audio` owns the reusable SFX runtime

Moved the reusable SFX-bank runtime out of `ambition_actors` and
into the existing `ambition_audio` crate: the Bevy bank asset/loader,
`SfxBankResource`, async promotion, handle-cache refresh, and
`audio_play_sfx_messages` now live behind `ambition_audio/kira`. The app
keeps the sandbox-specific catalog responsibility by resolving
`audio.sfx_bank` during `init_sandbox_resources` and inserting
`SfxBankAssetPath`; sync startup loading still inserts the same
`SfxBankResource` type. Deleted the unscheduled `apply_encounter_music`
fallback, leaving the neutral music-intent/director path as the only
music application route. The remaining gameplay-core audio/music files
are explicitly sandbox adapters (environment mix, schedule assembly,
settings sync, encounter/room/radio intent). Gate: `cargo fmt`;
`cargo test -p ambition_audio --features kira` (6);
`cargo test -p ambition_actors --lib --features audio` (1029);
`cargo check -p ambition_app --features "rl_sim audio"`;
`cargo fmt --check`; `python3 scripts/check_doc_links.py`;
`python3 scripts/check_agent_kb.py`; `cargo run -p ambition_app --bin
headless -- 120`.

## 2026-07-06 (opus) — E1c EXECUTED: `ambition_dialog` owns the dialogue runtime

Minted `crates/ambition_dialog` and moved the engine-side dialogue
machinery out of `ambition_actors`: the `DialogState` view model +
typewriter/options reveal state machines, `DialogChoice`, the
typewriter-SFX selection rules, the input/reveal Bevy systems, the
`bevy_yarnspinner`↔`DialogState` bridge, and the generic Yarn binding
machinery (`YarnStateMirror`/`YarnStateMirrorData`, `YarnPresentationCue`,
the `YarnContentBindings` installer seam, `YarnBindingsPlugin`). The crate
depends only on the foundations (`engine_core`, `ui_nav`, `input`, `sfx`,
`persistence`) + `bevy_yarnspinner` (behind `ui`).

Two seams make the runtime reusable and content-free:
1. **GameMode decoupling.** The runtime flips `DialogState.active` and
   names no host session mode. All `ambition_platformer_primitives::schedule::GameMode` reads/writes
   left the bridge + input systems; the sim-side `sync_dialogue_game_mode`
   maps active→`GameMode::Playing` when a conversation ends. Entering
   `Dialogue` stays the interaction system's job (every old `set(Playing)`
   site coincided with `active=false`, so the mapping is exact).
2. **Installer-only vocabulary.** `spawn_dialogue_runner` registers no
   concrete command — it only runs `YarnContentBindings.installers`.
   Ambition's actor/save-state Yarn commands + functions
   (give_item/buy/sell/challenge/spawn_*, boss_cleared/flag/visit_count/
   inventory_has/wallet_*) and the `SandboxSave`→mirror refresh stay in
   `gameplay_core::dialog::yarn_bindings` and register through the seam via
   the new `install_game_bindings`.

`gameplay_core::dialog` is now a facade: it re-exports the runtime on the
historical `ambition_actors::dialog::*` path (render/content/app/
host need no import edits) and owns the two sim-side plugins
(`YarnBindingsPlugin`/`YarnBridgePlugin` wrap the reusable ones + schedule
the refresh/installer/sync) and `sync_dialogue_game_mode`. Content keeps
naming `dialog::yarn_bindings::{YarnStateMirror, YarnContentBindings,
refresh_yarn_state_mirror, …}` (re-exported from `ambition_dialog`).

Added boundary test
`architecture_boundaries_dialog_crate_is_runtime_only` (foundational path
deps only; no game/actor/menu/UI code refs), and fixed pre-existing E1a
drift in `architecture_boundaries_input_crate_is_extracted` (the canonical
`controls` re-export moved into `ambition_persistence` at E1a). Gate:
`cargo test -p ambition_dialog --features "ui input"` (18);
`cargo test -p ambition_actors --lib --features "ui input"` (1011);
`cargo test -p ambition_app --test architecture_boundaries` (34);
`cargo check -p ambition_content --all-features`;
`cargo check -p ambition_app --features rl_sim`;
`cargo run -p ambition_app --bin headless -- 120` (clean);
rustfmt on touched files.

## 2026-07-07 (opus) — E1d EXECUTED: `ambition_dev_tools` owns the dev-tool state

Minted `crates/ambition_dev_tools` (foundational) and moved the
content-free half of `ambition_actors::dev` into it: the
`DeveloperTools` debug/gizmo toggle resource + inspector-visibility run
conditions, the reflected editable player-tuning / ability / stats
resources + their engine conversions, the movement/debug profile enums,
the `StartupProfiler` marks, `DeveloperTools` disk persistence
(developer.ron — the resource `ambition_persistence` deliberately never
took, left "for E1d" at E1a), and `sync_live_player_dev_edits_system`.

The move is clean because every actor type the dev code touches is
already foundational: `Body*` clusters in `ambition_engine_core`,
`PrimaryPlayerOnly` in `ambition_platformer_primitives::markers`,
`BodyHealth`/`Health` in `ambition_characters`. So the inline
`crate::actor::Body*` / `crate::actor::PrimaryPlayerOnly` references in
`editable.rs` + the live-edit system repoint straight to those crates; the
new crate deps are `engine_core` + `characters` + `platformer_primitives`
+ `persistence` only (bevy with just `bevy_log` — no windowing, so it
compiles headless in isolation).

`gameplay_core::dev` is now a facade re-exporting `dev_tools` /
`profiling` / `sync_live_player_dev_edits_system` on the historical
`crate::dev::*` paths, so the WIDE consumer set (render, sim_view,
runtime, app, menu, and audio's `phase_mark`) needs zero import edits. It
keeps `pub mod trace` — the trace RECORDER samples sim-only state
(`player`/`features`/`rooms`/`portal`/`game_mode`) and stays sim-side.

**Deviation from the card** (recorded in decomposition.md): "one crate
incl. app dev/ 2.7k; DevToolsPlugin moves whole" can't be done without a
cycle — the dev STATE is consumed below app, so it must be foundational;
the egui overlays need render/egui and are app-level. The overlays stay in
`ambition_app::dev` (they read the STATE via the facade path, so no edits);
only the reusable STATE moved down.

Gate: `cargo test -p ambition_dev_tools` (9);
`cargo test -p ambition_actors --lib --features "ui input"` (1002 =
1011 − the 9 moved dev_tools tests);
`cargo test -p ambition_app --test architecture_boundaries` (35 — added the
dev_tools foundation test + updated the dev-overlays home test);
`cargo check -p ambition_app --features rl_sim`;
`cargo run -p ambition_app --bin headless -- 120` (clean);
rustfmt on touched files.

## 2026-07-07 (opus) — E1e EXECUTED: settings-IR crate + the first extension crate

Executed the E1e menu carve as two crate mints plus recorded dispositions
for the pieces the dependency graph rules out as literal moves.

**Slice 1 — `ambition_settings_menu` (the god-dep dissolution).** Moved
core `menu/ir/{settings,system}` into a new FOUNDATIONAL crate: the
renderer-agnostic `SettingsMenuModel` / `SettingsOption` /
`apply_settings_option` + the System-menu layer, built from
`ambition_persistence::settings::UserSettings`. It is pure logic — no bevy,
no renderer, no game state — so the flat grid and the lunex cube render the
same model; that is exactly the layering that stops the settings IR from
being the god-dep that forced menu presentation to reach back into
gameplay-core. To keep the move cycle-free, the two pure
`next/prev_display_mode` helpers moved down to
`ambition_persistence::host::windowing` beside `DisplayModeKind` (gameplay
model re-exports them). `gameplay_core::menu::ir` is a facade re-export, so
the `persistence::settings` IR re-export + the app-menu hosts need no edits.

**Slice 2 — `game/ambition_menu_kaleidoscope` (the FIRST extension crate).**
Split the bevy_lunex 3D cube renderer out of `ambition_menu`; the base menu
crate is now bevy_lunex-FREE (flat grid + page model). The neutral
scroll-drag channel (`MenuScrollDragged` + `ScrollbarDragState`) was
mis-homed inside the cube module but is shared by both renderers — it moved
DOWN into `ambition_menu`. Two `pub(crate)` scrollbar helpers widened to
`pub`. The app's kaleidoscope host + grid backend repoint
`ambition_menu::kaleidoscope::*` → `ambition_menu_kaleidoscope::*`; the app
gains the extension dep. Two independent renderers now drive one page model
— the extension seam works.

**Dispositions (recorded in decomposition.md).** The host stack + grid
backend couple up to items/player/sfx, so like the E1d overlays they stay
app-side (only the neutral scroll types + the kaleidoscope path repointed).
`menu/map` is sim-tier (render + runtime consume `MapMenuState` /
`track_room_visits` / `sync_map_from_save`; neither deps content), so a
move to content would cycle — it stays in `gameplay_core::menu::map`, and
`app/menu/effects.rs` is app-side host glue already out of the reusable
crates, so bucket (3) "menu content stays content-side" holds in place. The
`ambition_touch_input` inversion is re-scoped: the menu stack is IR-only now
and needs nothing from touch; touch's remaining upward gameplay-core dep
(bevy_plugin affordances/physics + the menu_bridge GameMode gate) is a
separate, larger inversion, deferred. **C3 is explicitly closed** — no
in-game character-select menu exists; "wear" is spawn-time possession
re-parametrization, not a menu.

Gate (slice 1): `cargo test -p ambition_settings_menu --features dev_tools`
(16); `cargo test -p ambition_actors --lib --features "ui input
dev_tools"` (986 = 1002 − 16 moved IR tests). Gate (slice 2):
`cargo test -p ambition_menu` (14, lunex-free); `cargo test -p
ambition_menu_kaleidoscope` (9); `cargo test -p ambition_app --features
"bevy_ui_menu kaleidoscope_menu" --lib menu::` (90). Both slices:
`cargo test -p ambition_app --test architecture_boundaries` (37 — added the
settings-IR + kaleidoscope-extension boundary tests);
`cargo check -p ambition_app --features rl_sim`;
`cargo run -p ambition_app --bin headless -- 120` (clean); rustfmt touched.

**E1 COMPLETE:** E1a (persistence) + E1b (audio) + E1c (dialog) + E1d
(dev_tools) + E1e (settings IR + kaleidoscope extension) all executed.

## 2026-07-07 (Codex) — E-assets catalog/source carve executed

Carved the reusable sandbox asset catalog/source layer out of
`gameplay_core::assets::sandbox_assets` into
`ambition_asset_manager::sandbox_assets`: `SandboxAssetCatalog`, stable
`ids`, catalog builders, scaled-id helper, embedded-core URL table, and
`AmbitionAssetSourcePlugin` now live with the asset-manager resolver/profile
vocabulary. The former upward reads (`MusicRegistry`, world manifest,
character/boss sprite registries, texture-scale variants) are inverted into
plain `SandboxCatalogInputs` rows assembled by a thin gameplay-core adapter;
the adapter also supplies embedded LDtk bytes to the moved source plugin.

The runtime asset root fallback was corrected after the move so desktop
catalog probes still fall back to `../ambition_actors/assets`.
`assets/game_assets` intentionally remains in gameplay-core for now because it
owns Bevy image handles plus gameplay/presentation vocabulary; the next real
shrinks there ride E3/E6/E7 rather than making `ambition_asset_manager` import
upward.

Gate: `cargo check -p ambition_asset_manager --features bevy,static_core_assets,static_map`;
`cargo check -p ambition_actors --lib`; `cargo test -p
ambition_asset_manager` (62); `cargo test -p ambition_actors --lib
assets` (20); `cargo test -p ambition_app --test architecture_boundaries`
(38). Checks are green with pre-existing unused/private-interface warnings in
gameplay-core/sim-view/app and one pre-existing portal-presentation unused-variable
warning; boundary grep confirmed no production
`ambition_asset_manager::sandbox_assets` references to gameplay modules.

## 2026-07-07 (Codex) — W3/W4 world + LDtk crate split first cut

Minted `ambition_world` and `ambition_ldtk_map`. The world crate now owns
room graph/metadata/loading-zone IR, authored placement records + lowering
registry, debug labels, moving-platform spec/state/math, and the generated
`ron-room` bake/reload proof. The LDtk backend crate now owns project parsing,
field/intgrid/surface conversion, manifest/loading/hot-reload state, entity
converter registry, and the `bevy_ecs_ldtk` runtime spine. Gameplay-core keeps
only sim-side room load/systems and thin compatibility facades
(`rooms`, `ldtk_world`, `world::placements`, `debug_label`, moving-platform
visual sync).

W4 ratchet: ADR 0021 plus
`architecture_boundaries_world_ir_and_ldtk_backend_are_split`. The backend no
longer carries a hidden Ambition-content test fixture; real game worlds install
through `ambition_content::worlds` as content. Remaining W residue is explicit:
legacy typed family lists on `RoomSpec` dissolve branch-by-branch through the
placement registry.

Gate: `cargo test -p ambition_world` (23); `cargo test -p ambition_ldtk_map`
(21); `cargo check -p ambition_actors --lib`; `cargo test -p
ambition_app --test architecture_boundaries` (39); `cargo check -p
ambition_app --features rl_sim`; `python3 scripts/check_doc_links.py`.

## 2026-07-07 (Codex) — E3 character sprite-sheet slice executed

Moved the character sprite-sheet authority down into
`ambition_sprite_sheet::character`: `CharacterAnim`, `CharacterSheetSpec`,
sheet atlas/geometry helpers, `CharacterAnimator`, the baked sheet RON table,
the quality-tier pack table, and the sprite-sheet build script. Gameplay-core
now keeps compatibility facades for those modules plus only the adapters that
still legitimately read sim/roster state: catalog-aware sprite loading,
body-state animation pickers, and authored melee-hitbox resolution. Render,
app, and content call into `ambition_sprite_sheet` directly for sprite
vocabulary/animator/geometry, relieving the E4 render dep-flip pressure.

The carve also fixed a W3 feature-forwarding mismatch: gameplay-core's
`portal` feature now forwards into `ambition_ldtk_map/portal`, and the LDtk
backend depends on `ambition_world` with default features off so portal fields
stay feature-aligned under Cargo unification.

Remaining residue is explicit: boss sheet statics + boss attack geometry stay
with E6, and `assets/game_assets` keeps the Bevy handle bundle until the
E3/E6/E7 visual vocabulary tails stop naming gameplay-side types.

Gate: `cargo check -p ambition_sprite_sheet`; `cargo check -p
ambition_actors --lib`; `cargo check -p ambition_render`; `cargo check
-p ambition_content --all-features`; `cargo test -p ambition_sprite_sheet`
(21); `cargo test -p ambition_actors --lib character_sprites` (46);
`python3 scripts/check_doc_links.py`. Checks are green with pre-existing
unused warnings in portal-presentation, gameplay-core, sim-view, render, and
dialog.

## 2026-07-07 (Codex) — E-enc state/vocabulary crate minted

Minted `ambition_encounter` for the reusable encounter set-piece kit:
`EncounterSpec`/waves/mobs/lock walls, `EncounterState`/phase/run state,
`EncounterRegistry`, `SwitchActivation`, encounter events, music request
resources, and reward chest position/save-flag math. Gameplay-core's matching
`encounter::{events,music,registry,rewards,spec,state}` files are now
compatibility facades. Added
`architecture_boundaries_encounter_crate_is_state_only` so the crate cannot
start naming gameplay-core, LDtk, content, render, runtime, host, or app.

The live ECS/LDtk adapters remain in gameplay-core by design:
`loading` still reads the LDtk backend and content-installed wave book;
`systems`, `lock_walls`, and `switches` still spawn mobs, query player/body
state, mutate feature overlays, and write save/quest/banner state; and
`features/ecs/encounter_rewards.rs` still spawns/mutates reward chest entities.
Those dissolve with W/E7 rather than by pulling the sim heart into
`ambition_encounter`.

Gate: `cargo test -p ambition_encounter` (16); `cargo test -p
ambition_actors --lib encounter` (118); `cargo test -p ambition_app
--test architecture_boundaries architecture_boundaries_encounter_crate_is_state_only`
(1). The LDtk-backed encounter tests now install their own minimal world
manifest fixture, matching the W3 rule that the LDtk backend ships no hidden
game content.

## 2026-07-07 (Codex) — E6(a) boss animation frame split executed

Moved the boss animation frame cursor into sim-owned state:
`BossAnimFrame` now spawns with each boss from the content-installed/built-in
boss sheet timing, `drive_boss_animators` advances that cursor and publishes
`BossAnimationFrameSample`, and render mirrors the cursor into draw-only
`BossAnimator` texture/atlas state. Same-room reset now clears the cursor with
the boss brain/read-model reset, so retries do not inherit stale attack frames.

This closes the E4 slice-8 boundary violation: gameplay no longer reads a
render-inserted `BossAnimator` to drive boss geometry. Remaining E6 work is the
non-gnuton `BossAnim` vocabulary decision, `BrainSnapshot.target_pos`
retirement, and the bounded deep-fold attempts.

Gate: `cargo test -p ambition_actors --lib boss` (121); `cargo check -p
ambition_render`; `cargo check -p ambition_app --features rl_sim`; `python3
scripts/check_doc_links.py`.

## 2026-07-07 (Codex) — E6(c) boss target snapshot carve executed

Stopped routing the autonomous boss pattern target through
`BrainSnapshot.target_pos` in production. `tick_boss_brains_system` now builds
the `BossPatternContext` directly from the already-selected `ActorTarget`, runs
`tick_boss_pattern`, and mirrors the pattern state's attack projection into
`BossAttackIntent` as before. Possessed bosses still use the player-brain
snapshot path because controller input is the point of that branch.

Gate: `cargo test -p ambition_actors --lib boss` (121).

## 2026-07-07 (Codex) — E6(b/d) boss tail policy closures

Closed the remaining E6 boss-tail decisions without adding adapters:
`BossAnim` stays as the boss-domain row vocabulary because its rows are
authored attack-geometry verbs keyed by boss hurtbox/hitbox metadata; folding
non-GNU-ton bosses through `CharacterAnim` would mislabel those rows. The
no-boss-arm integrate fold also misses the bounded cheap test: it needs a
schedule move plus boss-only `BossConfig`/`BodyEnvelope`/combat-size policy
inside the actor body movement query. The `BossAttackIntent` → general
move-intent / boss-brain fold misses for the same reason on the brain side:
`tick_actor_brains` is the swarm system, while the boss orchestrator owns
non-swarm profile intent and possession→special mapping. Permanent policy
comments now live at the enum and both fold sites.

Gate: `cargo fmt`; `cargo test -p ambition_actors --lib boss` (121);
`python3 scripts/check_doc_links.py`.

## 2026-07-07 (Codex) — E4 dep-flip cleanup: moved-vocabulary render imports

Started the final render dep-flip by deleting stale render imports whose
destination crates already exist: UI font loading now reads
`ambition_asset_manager::sandbox_assets` directly, effect atlas layout uses
`ambition_sprite_sheet::character::build_atlas_layout`, and render imports
`BoundFeatureKind` from `ambition_combat` instead of the gameplay-core facade.
This does not remove render's gameplay-core dependency yet; the remaining
references are live presentation data / scheduling / E7-E8 residue rather than
already-moved vocabulary.

Gate: `cargo fmt`; `cargo check -p ambition_render`.

## 2026-07-07 (Codex) — E8 item catalog/UI slice executed

Minted `ambition_items` for the reusable item kit: the 24-slot `Item` catalog,
`OwnedItems`, authored `ItemCatalog` install seam, shop buy/sell primitives, and
`InventoryUiState` moved out of the actor sim. App menu/content code now imports
the catalog/UI state directly from `ambition_items`; `ambition_actors` keeps only
`items::{pickup,persist}` as sim adapters because those systems mutate actor
bodies, gravity, portals, abilities, hit events, save mirrors, and projectile
state.

Added `architecture_boundaries_items_crate_is_catalog_and_ui_state_only`: the
new crate may not depend on or source-reference actors/render/content/app, and
the old actor-side `inventory_ui` module path must stay absent.

Gate: `cargo check -p ambition_items`; `cargo check -p ambition_actors --lib`;
`cargo check -p ambition_content --all-features`; `cargo check -p ambition_app
--features rl_sim`. Pre-existing unused/private-interface warnings remain.

## 2026-07-07 (Codex) — E7 workspace re-home executed

Moved the game-owned crates out of the engine crate directory:
`ambition_app` and `ambition_content` now live under `game/`. The workspace
members, app/content relative dependencies, compile-time content fixture paths,
docs, scripts, recipes, and LDtk tooling references were repointed to the new
layout. `game/ambition_app` remains the composition tier naming both machinery
and content; `game/ambition_content` remains content-owned source/assets. Demo
pairs need no move until new demo crates are minted.

The remaining E7 work is no longer filesystem layout; it is the features-hub
facade dissolution. At this point the E4 final dep flip remains blocked by live
GameAssets/world/dialog/schedule presentation data, not by stale
`crates/ambition_app` or `crates/ambition_content` paths.

Gate: `cargo check -p ambition_content --all-features`; `cargo check -p
ambition_app --features rl_sim`; `cargo test -p ambition_app --test
architecture_boundaries` (41); `cargo test -p ambition_actors --lib` (789);
`python3 scripts/check_doc_links.py`.

## 2026-07-07 (Codex) — E4 dialog render import cleanup

Removed one stale actor-facade read from render: `dialog_ui` now imports
`DialogState` and `DialogChoiceSlot` directly from `ambition_dialog`, the crate
that already owns the reusable dialogue runtime. This does not remove
`ambition_render`'s actor dependency yet, but it closes the dialog part of the
final dep-flip blocker list; the remaining live blockers are GameAssets/world
visual data and schedule labels.

Gate: `cargo check -p ambition_render`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — E7 runtime combat-message facade cleanup

`SimCoreResourcesPlugin` now registers combat-owned messages/resources
(`SetFlagRequested`, `GameplaySfxRequested`, `HitEvent`,
`ResetRoomFeaturesEvent`, `GameplayBannerRequested`, `GameplayBanner`) through
`ambition_combat` instead of the actor `features` facade. Actor/domain-specific
messages (`ActorStimulus`, switch activation, quest advance) stay on their
current owning paths.

Gate: `cargo check -p ambition_runtime`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — E7 app/content combat-message facade cleanup

App and content code now import combat-owned message/resource types directly
from `ambition_combat`: hit events, room-feature reset events/reasons, set-flag
requests, and the gameplay banner resource. The actor `features` facade remains
for live actor ECS components and systems only.

Gate: `cargo check -p ambition_content --all-features`; `cargo check -p
ambition_app --features rl_sim`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — E7 sim-view combat-vocab facade cleanup

`ambition_sim_view` now imports `FeatureVisualKind` directly from
`ambition_combat` instead of through `ambition_actors::features`. This clears
the last easy combat-vocabulary facade hits outside the actor crate; remaining
features-facade references are live actor ECS components/systems, schedule
labels, boss glue, or debug/sim-view query inputs.

Gate: `cargo check -p ambition_sim_view`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — E7 render combat-vocab facade cleanup

Repointed render's `FeatureVisualKind` imports from the actor `features` facade
to `ambition_combat::events`, which already owns that presentation-neutral
taxonomy. Render still depends on actors for live ECS components/resources, but
this removes another already-moved vocabulary read from the E7 facade.

Gate: `cargo check -p ambition_render`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — E4 render config helper cleanup

Removed the last render import of `ambition_actors::config`: `fx` now keeps its
tiny alpha-clamping `rgba` helper locally. This is intentionally small, but it
keeps the E4 dep-flip surface honest by deleting another actor edge that was
only a utility reach-through, not live sim data.

Gate: `cargo check -p ambition_render`. Pre-existing warnings remain.



## 2026-07-07 (Codex) — F1.5 render→actors first-cut ratchet

Burned down the already-moved vocabulary side of the E4 render dep-flip blocker:
`ambition_render` now reads rooms/metadata/gate portals and the respawn-visuals
message from `ambition_world`, camera layer markers + `SandboxSet` labels +
camera shake/ease state from `ambition_platformer_primitives`, baked sheet
registry data from `ambition_sprite_sheet`, and its fireball prop-art id locally.
The actor `schedule`, `session::camera_layers`, `session::RespawnRoomVisualsRequested`,
and `time::camera_ease` modules are compatibility facades over the lower crates.

Added a render-side F1.5 ratchet that counts the remaining `ambition_actors::`
references exactly. The residue is now the real work: `GameAssets`/image-handle
catalog ownership, live feature ECS inputs, dev-tool presentation toggles, boss
animator tail, shrine pulse, physics settings, and starting-character fallback.

Gate: `cargo fmt --all`; `cargo test -p ambition_render --test observation_boundary`;
`cargo check -p ambition_render`.

## 2026-07-07 (fable, FINAL) — whole-repo audit → [engine/fable-final-audit-2026-07-07.md](engine/fable-final-audit-2026-07-07.md)
The last fable pass audited the full post-decomposition repo. **READ THAT FILE
FIRST when planning any structural work — it supersedes older card details
where they conflict.** Headlines: DAG sound, 11 arrows enumerated with
prescriptions (F1); ambition_actors 68k = 43k real actor domain + misplaced/
glue/facades with disposition classes (F2); lowering/GeoId/Tier-0 rulings
verified green, with ron_room/world-purity corrections now tracked by the
Codex ratchet entry below (F3); TWO rename-fallout regressions found and FIXED
(desktop asset root silently degraded — game ran with no assets; music-tool
repo probe dead) plus clock-reset seam + determinism hazards logged (F4); the
`ambition` umbrella crate (E9) + demo-game homes proposed as the oracle made
concrete (F5); final gate 44 suites green, only the documented feel-RED (F6).
The priority-ordered next-session queue is at the end of the audit file.

## 2026-07-07 (Codex) — F1.1/F3 world-purity ratchet first cut

Moved the backend-neutral `ron-room` parser/serializer/source row from the
LDtk backend into `ambition_world::ron_room`, leaving LDtk composition as a
legal backend → IR consumer. Replaced room-transition `zone_sfx:
ambition_sfx::SfxId` with a world-owned plain cue id and converted to
`SfxId` only at the app audio emission edge, deleting `ambition_world` →
`ambition_sfx`. Added the explicit `ambition_world` dependency allow-list
test; combat/interaction/portal remain named legacy family residue to remove
one branch at a time through placement-record dissolution.

Gate: `cargo fmt --check`; `cargo test -p ambition_world`; `cargo test -p
ambition_ldtk_map`; `cargo test -p ambition_actors --lib` (789);
`cargo build -p ambition_app --features rl_sim`; `cargo test -p ambition_app
--test architecture_boundaries` (41). Pre-existing warnings remain.

## 2026-07-07 (Codex) — F1.2 portal facade cleanup

Deleted the `ambition_actors::portal` facade and removed the actor crate's
presentation dependency. Simulation/content/runtime consumers now import
portal mechanics directly from `ambition_portal`; app/render/dev UI imports
presentation resources from `ambition_portal_presentation`; the Ambition-only
world-frame/viewer/camera-continuity/dev-toggle adapter lives in
`ambition_host::portal`. The observation ordering label moved to
`ambition_portal_presentation::PortalObservationSet` so render can order after
the seam without depending on host.

Gate: `cargo fmt --check`; `cargo test -p ambition_app --test
architecture_boundaries` (41); `cargo test -p ambition_actors --lib` (789);
`cargo build -p ambition_app --features rl_sim`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — F1.3 vfx side vocabulary cleanup

Deleted the `ambition_vfx` dependency on `ambition_characters` by moving the
effect/hitbox side tag to vfx-owned `HitSide`. Combat keeps the gameplay
relationship vocabulary (`ActorFaction`) and maps at the only edges that need
both facts: melee/moveset hitbox spawn, hit/on-hit resolution, and summon
execution. DamageBox/Summon emitters now send `HitSide` directly, and the debug
overlay colors the vfx-owned tag instead of reaching back through actor
factions. The `architecture_boundaries_effects_crate_is_foundation_only` test
now forbids both `ambition_actors` and `ambition_characters` from
`ambition_vfx`.

Gate: `cargo fmt --check`; `cargo test -p ambition_vfx`; `cargo test -p
ambition_app --test architecture_boundaries` (41); `cargo test -p
ambition_actors --lib` (789); `cargo build -p ambition_app --features
rl_sim`. Pre-existing warnings remain.

## 2026-07-07 (Codex) — F1.4 EXECUTED: `GameMode` moved down to primitive schedule vocabulary

Moved the coarse session-state enum and gameplay gating run conditions from
`ambition_actors::session::game_mode` into
`ambition_platformer_primitives::schedule`, next to `PlatformerRuntimeSet`.
Runtime, content, sim-view, and touch-input schedule/run-condition callers now
name the lower crate directly; `ambition_actors::game_mode` remains a thin
compatibility facade for actor-internal and legacy paths while the residual
actor hub dissolves. Updated the F1 audit item and UI navigation doc so future
callers treat `GameMode` as shared runtime vocabulary, not actor machinery.

Gate in overlay sandbox: `cargo`/`rustfmt` unavailable, so validation was
limited to static source greps, Python syntax-free rewrite checks, and
`git diff --check`. Recipient should run `cargo fmt --all`,
`cargo test -p ambition_platformer_primitives`, `cargo test -p ambition_actors --lib`,
`cargo test -p ambition_touch_input`, and the app/content checks listed in the
overlay response.

## 2026-07-08 (Codex) — F1.5 render→actors dep flip complete

Finished the F1.5 blocker after the first-cut ratchet compiled: `ambition_render`
no longer depends on `ambition_actors`. The remaining render-facing vocabulary
was lowered to reusable crates: `ambition_sprite_sheet::game_assets` owns the
`GameAssets` resource shape, entity/parallax asset ids, and static image loader
helpers; `ambition_sprite_sheet::boss` owns boss animation/sheet render types;
`ambition_platformer_primitives` owns the shared physics settings, feature
overlay resource, and shrine pulse resource; and `ambition_dev_tools` owns
`SandboxDevState`. Render-side feature visuals now use the `FeatureView`
read-model instead of live ECS feature components, and controlled-body sprite
quality reloads preserve the selected character through a render-owned marker
written by the app composition seam.

The F1.5 observation test is now a zero-dependency boundary: the render manifest
must not depend on `ambition_actors`, and render source code must not name the
actor crate.

Gate: `cargo fmt --all`; `cargo test -p ambition_render --test observation_boundary`;
`cargo check -p ambition_render`; `cargo check -p ambition_app --features "rl_sim input mobile_touch"`.

## 2026-07-08 (Codex) — F1.6 inventory UI split complete

Finished F1.6 by splitting the menu-navigation state out of `ambition_items` and
into the new `ambition_inventory_ui` crate. `ambition_items` now owns the item
catalog and shop primitives only, and drops the `ambition_ui_nav` dependency.
The app/menu code imports `InventoryUiState` from `ambition_inventory_ui`; the
old `ambition_items::inventory_ui` path is gone. The architecture boundary test
now checks both halves: items must not depend on UI navigation or contain an
`inventory_ui` module, while inventory-ui is a small leaf over `ambition_ui_nav`
and must not import the item catalog, actor sim, render, content, or app tiers.

## 2026-07-08 (Codex) — F1.7 ControlFrame moved to engine core

Finished F1.7 by moving the device-agnostic `ControlFrame` vocabulary out of
`ambition_input` and into `ambition_engine_core`, next to `InputState` and the
reference-frame helpers. `ambition_characters` now stores/reads
`ambition_engine_core::ControlFrame` for slot controls and player-brain
snapshots, so reusable character brains no longer depend on the input adapter.

`ambition_input` still owns the device/Leafwing/settings adapter logic: it
exports `read_gameplay_control_frame*` / `read_menu_control_frame`, keeps
`PlayerDashTriggerState`, and re-exports `ambition_engine_core::ControlFrame`
for legacy app/test import paths. The architecture boundary test now ratchets
that split: `ambition_characters` must not depend on or name `ambition_input`,
while `ambition_input` must not depend upward on reusable character brains.


## 2026-07-08 (Codex) — F1.8 asset-manager/SFX adapter edge removed

Finished F1.8 by deleting the unused `ambition_asset_manager::sfx_integration`
adapter and removing the optional `ambition_sfx` dependency/`sfx` feature from
`ambition_asset_manager`. The asset manager is now backend-generic again: it
resolves logical ids such as `audio.sfx_bank` to `AssetLocation`, while the
audio/app layer owns `BankProvider` construction from local paths, embedded
bytes, or async loader paths. The app crate no longer enables an asset-manager
`sfx` feature, and the architecture boundary test ratchets the split.

Gate in overlay sandbox: `cargo`/`rustfmt` unavailable; validation was limited
to static boundary greps, doc checks, and `git diff --check`. Recipient should
run `cargo fmt --all`, `cargo test -p ambition_asset_manager`, `cargo test -p
ambition_app --test architecture_boundaries`, and `cargo check -p ambition_app
--features "rl_sim input mobile_touch"`.

## 2026-07-08 (Codex) — F1.9 runtime composition edge ratcheted

Finished F1.9 as an explicit no-move ruling. The `ambition_runtime` edges to
`ambition_actors`, `ambition_combat`, `ambition_projectiles`, adjacent
headless sim/model crates, and the foundational `ambition_dev_tools` state seam
are correct by design because runtime is the engine composition tier, not a
foundational model crate. The architecture boundary test records the allowed
headless composition surface and forbids upward drift
into app/content/host/render/touch/menu/backend ownership.

This prevents future cleanups from re-chasing the intentional runtime → sim
arrows while still catching real tier leaks.

## 2026-07-08 (Codex) — F1.10 host actor edge removed

Closed the final F1 host blocker: `ambition_host` no longer depends on
`ambition_actors`. The visible host wires input through the runtime-owned
`host_input` facade, uses lower primitive camera/controlled-subject vocabulary
for portal camera continuity, and its demo smoke fixture reaches actor setup
through `ambition_runtime::demo_fixture` rather than taking an actor-crate edge.

The architecture boundary now pins both decisions: F1.9 records
`ambition_runtime` as the intentional headless sim composition tier, and F1.10
forbids `ambition_host -> ambition_actors` from regressing.

## 2026-07-08 (Codex) — F1.11 touch overlay render edge accepted

Closed F1.11 as a no-move ruling. `ambition_touch_input` still has a direct
`ambition_render` dependency because it owns the visible touch HUD: joystick,
action-button text, glyph overlays, z-ordering, and render-aligned touch hit
regions live together so the on-screen controls and the Android raw-touch path
cannot drift. That means the crate is not a pure input crate; it is a small
presentation/input adapter with a legacy name.

The architecture boundary now records that decision: the touch adapter may depend
on `ambition_render`, but it stays out of app/content/host/backend ownership. A
future rename or re-home under a `presentation/` grouping is still allowed, but it
is LOW priority and not a blocker for closing the F1 dep-graph audit.

- 2026-07-08: F1.1 closed. `ambition_world` dropped its remaining runtime-family deps (`ambition_combat`, `ambition_interaction`, `ambition_portal`) by converting the legacy RoomSpec payload families to world-owned plain specs and moving runtime lowering to actor/portal edges.

## 2026-07-08 (Codex) — F2.1 actor compatibility facade burn-down

Burned down the first safe batch of post-F1 `ambition_actors` compatibility
facades now that their consumers name the lower crates directly. The actor crate
no longer exposes actor-side facades for `GameMode`, camera layer markers,
camera ease/shake state, `SandboxDevState`, or `ControlledSubject`; external
`FeatureEcsWorldOverlay` reads also name `ambition_platformer_primitives`
directly. Runtime/app now depend on `ambition_dev_tools` where they need the dev
state instead of reaching through `ambition_actors`.

Added/updated architecture ratchets so the deleted facade files stay gone and
new code cannot reintroduce actor paths for the moved vocabulary.

Gate: `cargo fmt --all`; `cargo test -p ambition_actors --lib`; `cargo test -p
ambition_app --test architecture_boundaries`; `cargo check -p ambition_app
--features "rl_sim input mobile_touch"`.

- 2026-07-08 — F2 misplaced character-sprites absorb: moved canonical `SheetRegistryPlugin` into `ambition_sprite_sheet`, repointed app/content plugin installs there, and removed actor-side pure facade modules for `animator`, `baked_sheet_rons`, `registry`, `sheets`, and `sprite_packs`. The remaining `ambition_actors::character_sprites` surface is now the actor/content join that still reads actor facts or character catalog data.

- 2026-07-08 — F2 misplaced assets consumer repoint: app/content/sim-view callers now use `ambition_sprite_sheet::game_assets` for `GameAssetConfig`, `GameAssets`, and entity sprite keys, and `ambition_asset_manager::sandbox_assets` for `SandboxAssetCatalog` / catalog ids. The actor `assets/` module remains only as the game-specific adapter that joins authored content registries, embedded world rows, and character/boss sprite loading.

- 2026-07-08 — F2 projectile residual-glue schedule facade: centralized the remaining actor-side projectile steppers behind `ambition_runtime::projectile_schedule`, repointed app ordering there, and added a boundary ratchet so app/content production code no longer schedules through `ambition_actors::{projectile,enemy_projectile}` directly. The projectile model remains in `ambition_projectiles`; victim-routing and charge-input steppers remain actor-side until their boss/player/world inputs split.
- 2026-07-08 — F2 developer-tools residual-glue facade burn-down: external consumers no longer reach `DeveloperTools`, editable profiles, startup profiling, or `sync_live_player_dev_edits_system` through `ambition_actors::dev`. App/runtime/sim-view now name `ambition_dev_tools` directly, and `ambition_actors::dev` retains only the sim-coupled gameplay trace recorder.

- 2026-07-08 — F2 audio residual-glue facade burn-down: app menu/radio and Android lifecycle consumers now import pure playback vocabulary (`AudioLibrary`, `MusicPlaybackState`, `RadioStationState`, `MusicChannel`, `SfxChannel`, `set_radio_track`) from `ambition_audio::library` instead of `ambition_actors::audio`. The remaining actor-side audio surface is the sandbox composition plugin/environment detector, which still bridges actor water-contact facts, settings, schedule, and music intent.

- 2026-07-08 — F2 schedule-label facade repoint: runtime/content/app/sim-view consumers now name canonical schedule labels (`SandboxSet`, `CombatSet`, `BossSteerSlot`, `PresentationSetupSet`, `SimulationSetupSet`) from `ambition_platformer_primitives::schedule` instead of `ambition_actors::schedule`. The actor schedule module remains only for the concrete sandbox set installer and input bridge systems.

- F2 menu-backend facade slice: `InventoryUiBackend` and backend availability constants moved to `ambition_menu::backend`; app menu consumers no longer name `ambition_actors::menu::backend`.

- 2026-07-08 — F2 settings/menu-IR facade repoint: app menu hosts/tests now import persisted settings vocabulary from `ambition_persistence::settings` and the shared renderer-agnostic settings/System IR from `ambition_settings_menu` instead of reaching through `ambition_actors::persistence::settings` or `ambition_actors::menu::ir`. Added a boundary ratchet so app menu code cannot reintroduce those actor facade paths.

- 2026-07-08 — F2 character animation vocabulary repoint: SimView read-model fields now name `ambition_sprite_sheet::character::CharacterAnim` directly instead of using the actor `character_sprites` facade. The actor-side character-sprites module remains only for real actor/content adapters: stateful animation pickers, authored hitbox resolution, and catalog-aware sprite loading/body collision.

- 2026-07-08 — F2 encounter vocabulary repoint: app/content/runtime/sim-view consumers now name pure encounter state/spec/music/reward vocabulary from `ambition_encounter` instead of actor encounter facades; a follow-up moved the content-installed encounter wave book there too. Actor encounter remains the adapter home for LDtk loading, ECS spawning, switch queues/indexes, lock-wall contribution, and concrete schedule systems.

- 2026-07-08 — F2 dialog/developer-persistence residual-glue repoint: app/content/runtime consumers now name reusable dialog state/input/Yarn-binding vocabulary from `ambition_dialog`, while actor dialog keeps Ambition-specific Yarn bindings plus `GameMode` sync plugins. `DeveloperPersistenceSchedulePlugin` now lives in `ambition_dev_tools`; the actor persistence compatibility alias was removed in the F2 closeout.

- 2026-07-08 — F2 closeout ratchet: moved `MapMenuState` / `MapRoomNode` to `ambition_menu::map`, removed actor dialog runtime-vocabulary/Yarn-binding reexports and the dev-persistence alias, and added boundary ratchets documenting remaining actor refs as intentional asset/LDtk/dialog/item/map adapters. F2 is now closed for audit cleanup; deeper actor decomposition moves to later world/plain-input, projectile, and unified-actor cards.

- 2026-07-08 — F2 final facade closeout top-off: deleted the actor-side `menu::ir` and map-model compatibility facades after repointing actor-local settings helpers and runtime `MapMenuState` resource initialization to `ambition_settings_menu` / `ambition_menu::map` directly. The remaining `ambition_actors::menu` surface is only the save/room hydration, hotkey, and Bevy-UI adapter layer. F2 remains closed for audit cleanup; later cards own deeper actor-domain decomposition.
- 2026-07-09 — F4.3 clock-reset seam: added `ClockResetRequest` to the time-control authority path, registered and scheduled its owner handler, and repointed respawn/transition/sandbox-reset call sites to emit reset intent instead of writing `ClockState.time_scale = 1.0` directly. The handler snaps both live `ClockState` and `RequestedClockScale` to neutral so reset behavior stays immediate while the authority boundary stays centralized.

- 2026-07-09 — F4.4 deterministic player fallback: save-load hostile-grudge restoration and actor slot-board anchoring now fall back by lowest `PlayerSlot` instead of raw Bevy query iteration order. Both sites are tagged `AMBITION_REVIEW(determinism)`, and the architecture boundary ratchets the player-fallback pattern so future RL/multiplayer work has a stable ordering seam.

- 2026-07-09 — F3.2 swept mover closeout: runtime actor/boss ECS queries now require the shared `SweepSample` component, so `integrate_boss_bodies` flows through the canonical §3.1 motion record instead of an optional fallback. Portal transit retired its `PortalSweepAnchor` component and feeds CCD from the same kernel-written sample, guarded by a live-endpoint check so teleports outside the sim step are not interpreted as swept portal crossings.
- 2026-07-09 — E9 umbrella/demo first cut: added the `crates/ambition` facade crate as the engine-for-other-games surface, re-exporting runtime/host/render/world/model/vocabulary crates plus a curated prelude. Added `game/ambition_demo_sanic` and `game/ambition_demo_smb1` as registered workspace members whose manifests depend only on `ambition`, and ratcheted that oracle in the architecture boundary test. The follow-up app-manifest collapse is tracked in the next entry; demo crates intentionally start as empty content plugins.
- 2026-07-09 — E9 app-manifest collapse: repointed `game/ambition_app` reusable engine/model/render imports through the new `ambition` umbrella crate and reduced the app manifest's direct `ambition*` deps to the facade plus app-local `ambition_content` and `ambition_menu_kaleidoscope`. Added umbrella feature forwarders and a boundary ratchet so downstream/demo app shells do not rebuild the old direct dependency wall.

- 2026-07-09 — F4.4 fallback top-up: kept the actor brain and save-sync player-slot reads optional at the query seam, preserving deterministic lowest-`PlayerSlot` fallback without making hostile AI depend on every fixture/player entity already carrying the slot component. The primary-player path still anchors directly; only the non-primary fallback sorts by slot.

- 2026-07-09 — unified-melee/warning top-up: `unified_melee` now follows the spawned hostile by `FeatureId + BodyMelee` across the observation window instead of caching the first post-spawn entity, so it observes the sim body that owns the swing lifecycle, matching the already-green `enemy_attacks_player` chain test. Also cleaned the low-risk unused/private-interface warnings surfaced by the E9/F4.4 gate log without running broad `cargo fix` churn; the follow-up private-interface warning was resolved by keeping the debug-overlay systems crate-visible.

- 2026-07-09 — unified-melee moveset observation top-up: the hostile half of `unified_melee` now serializes its two sandbox simulations and observes both accepted swing authorities: the legacy/flat `BodyMelee` projection and the moveset-backed `MovePlayback` that now owns actor melee timing. This keeps the already-green `enemy_attacks_player` test as the enemy-AI regression oracle while making `unified_melee` a convergence test for the post-E9 combat read-model.

- 2026-07-09 — projectile residual-glue substrate spawn slice: moved the canonical enemy/boss `Effect::Projectiles` drain into `ambition_projectiles::enemy::apply_enemy_projectile_effect_requests`, where it only materializes projectile entities, stamps shared sequence/owner/visual components, and consumes effect vocabulary. `ambition_runtime::projectile_schedule` still owns the scheduling name, but now routes the enemy-pool spawn executor through `ambition_projectiles`; the actor-side projectile residue is narrowed to the still-woven charge input and victim/world routing stepper.

- 2026-07-09 — projectile visual-kind expiry cue slice: moved the lasersword detonation VFX policy out of `ambition_actors::projectile::systems` and into `ambition_projectiles::visual_kind` as `ProjectileVisualKind::expiry_vfx`. The actor stepper still decides when a projectile times out or hits a solid and still owns victim/world routing, but projectile-kind-specific presentation cues now live next to the projectile art descriptor and are ratcheted by the architecture boundary test.
- 2026-07-09 — projectile test-travel slice: moved the pure projectile primitive tests (motion gestures, spawner gates, kind tuning, body flight/collision) from the actor facade into `ambition_projectiles::engine_tests`, removed the actor-side `engine_tests` module, and extended the projectile architecture ratchet so model tests travel with the projectile kit while actor tests cover only the woven charge/victim/world steppers.

- 2026-07-09 — fable F9 verification pass: independently confirmed the executed F-queue against manifests/source (all F1 arrows closed, world purity + ratchet, ron_room re-sided, F3.2/F4.3/F4.4, E9 exit met at 3 app deps, gate 44/44 green). RULED: the IR-native-family route for F1.1 is accepted; record-over-schema consolidation continues as IR-internal cleanup, one family per session, exiting when the dual-emit guard deletes. Next-phase queue (audit F9): demo content first, then family conversions, projectile steppers stay put until inputs are plain, player fold stays deferred.

- 2026-07-09 — fable: **CC6 MOVING PORTALS LANDED** (§5-P2 spec executed in full; amendments in collision-and-ccd.md §5-P2a). Host-attached frames via `GeoFaceRef` (engine_core gains `FaceAnchor` + `World::{block_by_id, resolve_face, attribute_face}`; platforms stamp their first real `GeoSource::Placement` ids), relative swept trigger (the scoop works; co-moving bodies never spuriously transit), Galilean transfer with the exit-REST-frame min-exit floor, host-carried motion exempt from eviction (close-only pushout preserved), lazy content-side attribution + per-frame frame re-derivation (a portal closes with its host face). Full parity gate green — including the `--features portal` content suite (101 tests), which the default gate was silently skipping (D2 gate list amended). Also fixed pre-existing F4.3 fallout: reset-test fixtures lacked `ClockResetRequest` registration (4 RED on main before this session). Next on the doctrine: P3a angled math [opus], CC3 fuzz rig [opus].

- 2026-07-09 — fable F9.2 IR-consolidation, family 1 (interactables): first branch conversion of the record-over-schema arc. Moved `InteractableSpec`/`InteractionKindSpec` DOWN into `ambition_entity_catalog::placements` (they are Tier-0-pure — no kernel `Vec2`, unlike `HazardVolumeSpec`'s inline `KinematicPath`) and reused them directly as the `PlacementSchema::Interactable` payload, so the schema and world IR share ONE pure type with no mirror/mapping. Deleted `RoomSpec.interactables`, `RoomEmission.interactables`, and the `RoomEmission::interactable` helper; the LDtk npc/switch converters now emit a `PlacementRecord` only; the actor sim path lowers via a registered `lower_interactable_placement`; render's authored-visual path reads the same records (both consumers on the single channel — no dual-emit guard needed for this family). `ambition_world::rooms` re-exports the moved types so `rooms::InteractableSpec` paths stayed stable. Gate green (`ambition_app --features rl_sim` all suites, actors lib 748, ldtk_map 24, world 25). Next families: pickups → chests → breakables (same Tier-0 move), portals last (`PortalSpec` carries `Vec2`). Pre-existing out-of-scope REDs left untouched: `ambition_sprite_sheet` lib-test (`mod tests;` files missing on HEAD) and `ambition_content` portal lib-test (uncommitted CC6 `host_adapter.rs` tail).

- 2026-07-09 — fable F9.2 IR-consolidation, family 2 (pickups): second branch conversion, same shape as interactables. Moved `PickupSpec`/`PickupKindSpec` into `ambition_entity_catalog::placements` as the `PlacementSchema::Pickup` payload (Vec2-free — clean Tier-0 move, no mirror); deleted `RoomSpec.pickups`, `RoomEmission.pickups`, and the `pickup` emitter helper; the LDtk `PickupSpawn` converter now emits a `PlacementRecord` only; actor sim lowers via a registered `lower_pickup_placement`; render reads pickups off `spec.placements`. Added a shared `placement_aabbs(room, kind)` helper (keyed on `PlacementRecord::kind()`) to the spatial-integrity test and the geometry-debug example so migrated-family lookups stay one-liners. Gate green (`ambition_app --features rl_sim`, actors lib 748, ldtk_map 24, world 25, entity_catalog 16). Remaining families: chests → breakables → portals (portals last; `PortalSpec` carries `Vec2`).

- 2026-07-09 — fable F9.2 IR-consolidation, family 3 (chests): third branch conversion, identical pattern. Moved `ChestSpec`/`ChestStateSpec` into `ambition_entity_catalog::placements` as `PlacementSchema::Chest` (reward reuses the already-moved `PickupKindSpec`); deleted `RoomSpec.chests`, `RoomEmission.chests`, and the `chest` emitter helper; the LDtk `ChestSpawn` converter emits a `PlacementRecord` only; actor sim lowers via `lower_chest_placement`; render reads chests off `spec.placements`. Gate green (`ambition_app --features rl_sim`, actors lib 748, ldtk_map 24, world 25, entity_catalog 16). Remaining: breakables → portals (portals last; `PortalSpec` carries `Vec2`).

- 2026-07-09 — fable F9.2 IR-consolidation, family 4 (breakables): fourth branch conversion; ALL Vec2-free families now placements-only. Moved `BreakableSpec` + `BreakableStateSpec`/`BreakableTriggerSpec`/`BreakableCollisionSpec` (with impls) into `ambition_entity_catalog::placements` as `PlacementSchema::Breakable`. Twist: breakables enter via the surface-compile pipeline (`compile_surface` → `SurfaceCompiled.breakables`), not a dedicated converter — so the placement conversion happens in `RoomEmission::from_compiled`, mapping each internal `Authored<BreakableSpec>` to a `PlacementRecord` (SurfaceCompiled keeps its typed field). Deleted `RoomSpec.breakables` + `RoomEmission.breakables`; actor sim lowers via `lower_breakable_placement`; render reads breakables off `spec.placements`. Gate green (actors lib 748, ldtk_map 24, world 25, entity_catalog 16). ONLY PORTALS REMAIN in the arc: `PortalSpec` carries `ae::Vec2` (pos/normal), so unlike the other five families it cannot move to Tier-0 — the Vec-deletion end state there needs a plain-pair (`[f32;2]`) mirror, assessed separately. Hazards keep their typed Vec + inline-motion legacy path (dual-emit guard) until the KinematicPath lift.

- 2026-07-09 — fable F9.2 IR-consolidation, family 5 (portals): the deliberate Vec2 exception, done as a Tier-0 MIRROR. Moved `PortalChannelColorSpec` (pure enum) into `ambition_entity_catalog::placements` and added `PlacementSchema::Portal(PortalSchema)` where the schema stores `normal: [f32;2]` + color/link/half_length; the runtime-facing `ambition_world::rooms::PortalSpec` keeps its `Vec2` and the actor `#[cfg(feature="portal")] lower_portal_placement` reconstructs it, DERIVING the face center from the placement record's `aabb.center()` (the converter authored `pos = box center`). The LDtk `Portal` converter emits a `PlacementRecord`; `RoomSpec.portals`/`RoomEmission.portals`/the `portal` helper deleted; the cfg(portal) spawn loop removed (portals lower via the registry now). Verified across features: content portal suite 101 green, `ambition_app --features rl_sim` exit 0, `ambition_actors --features portal,portal_ldtk --all-targets` green, non-portal `ldtk_map --no-default-features` green. Confirmed no conflict with the uncommitted CC6 `host_adapter.rs` (it operates on runtime `PlacedPortal`/`RoomGeometry`, never the world IR). FIVE of six families now placements-only; the dual-emit guard's deletion is gated only on the hazard inline-motion→KinematicPath lift (behavior-preserving; being done next).
