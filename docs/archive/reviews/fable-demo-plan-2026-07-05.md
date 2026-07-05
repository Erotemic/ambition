# THE ENGINE PLAN — the full architecture vision, proven by Sanic + SMB1

**THIS IS THE SINGLE GOAL DOCUMENT.** Hand this doc to an executing agent and
"complete it" is the tasking: it carries the ENTIRE remaining engine-
architecture vision — the unification tails, the decomposition (every carve),
the content evictions and cleanups, the ability-model completion, and the two
demo games that PROVE the architecture. The demos are the acceptance test,
not the whole goal. §0 states the goal; §6 is the completeness audit showing
every open item from the three historical reviews is either DONE in code or
re-homed here — nothing else is outstanding anywhere.

**Authored by fable, 2026-07-05 (evening; completeness audit appended the
same night),** consolidating and superseding the review docs at Jon's
direction:

- [`fable-review-2026-07-04.md`](fable-review-2026-07-04.md) — now
  **HISTORICAL** (the R1–R6 record, adjudications AJ1–AJ7, its E-log frozen).
- [`fable-review-2026-07-05.md`](fable-review-2026-07-05.md) — now
  **HISTORICAL** (AJ8–AJ12, the R7–R10 specs, the FABLE-queue execution log
  frozen).

**Everything still OPEN from both docs is re-homed here** under a track
structure aimed at ONE destination: **the two first demo games, Sanic and
SMB1** (Q12+Q13, Jon 2026-07-05) — each a content crate + a thin app against
`ambition_runtime`, built adversarially against the oracle (*could another
platformer be built by ADDING a content crate without editing core?*). Slice
specs below answer every question opus posed in the two frontier audits
(Q16–Q26) — read §2 before executing anything.

**The binding stack** (unchanged + one addition):
[`../planning/engine/architecture.md`](../planning/engine/architecture.md)
(crate map) · [`../planning/engine/spatial-model.md`](../planning/engine/spatial-model.md)
(authoring-backend-agnostic space) ·
**[`../planning/engine/frame-awareness.md`](../planning/engine/frame-awareness.md)
(NEW — Jon's frame manifesto, adjudicated in §1)** · ADR 0020 (mounts) ·
[`../planning/roadmap.md`](../planning/roadmap.md) (phases + the demo matrix).

**Executor rules** (unchanged from the historical docs, distilled): every
slice carries **[opus]** / **[★fable]** / **[opus, fable-specced]** (opus
executes exactly the spec in §2/§3; stop and queue for fable at the first
sign the spec doesn't fit the code). Commit each verified slice, explicit
paths, BLIND-marked feel commits, C4 scenario for every new reaction seam,
keep THIS doc's log current, wall-clock table for multi-phase runs. Known
standing RED: `unified_melee::a_hostile_actor…` (feel-reserved).

---

## 0. THE GOAL STATE (what "done" means)

When this document is complete, ALL of the following hold:

1. **The crate map is real.** Every crate in
   [`architecture.md`](../planning/engine/architecture.md)'s 6-tier target
   stack exists with imports flowing strictly downward:
   `ambition_world` + `ambition_ldtk_map` (W), `ambition_persistence` /
   `ambition_menu` / `ambition_audio` / `ambition_dialog` /
   `ambition_dev_tools` (E1), `ambition_combat` + `ambition_projectiles`
   (E2), `ambition_sprite_sheet` (E3), `ambition_sim_view` with the render
   edge cut (E4), `ambition_runtime::PlatformerEnginePlugins` (E5), and the
   gameplay_core residue renamed `ambition_actors` with the `features/` hub
   facade dissolved (E7).
2. **The unification has no tails.** The ability model is COMPLETE (three
   tiers live: data + prefab registry + techniques-with-params — track A);
   the A1 boss tail is closed (E6); every `Without<BossConfig>` is documented
   policy; movement identity, perception, and motion are typed per-body
   policies in ONE pipeline.
3. **The engine names no content.** The named-content grep
   (`rg 'gnu_ton|pca|mockingbird|shark|duel_arena|noether|pirate|sanic|hall_of_characters'`
   over engine crates) hits test fixtures only, then zero; the residual
   cleanups (track C) are gone; content enters ONLY through install
   registries, converters, and manifests.
4. **The two demos pass the oracle.** `demos/demo_sanic` and `demos/demo_smb`
   each = one content crate + a ~100-line app on `ambition_runtime`, and each
   demo's `git log --stat` touches zero engine crates. The boundary test
   suite enforces app-thinness and machinery-names-no-content permanently.
5. **The stretch seams exist** (not the stretch features): AJ13 frame
   discipline in review practice, AJ14's Tier-0 obligations in E4, the
   knight-on-chains and angled-portal seams intact for post-1.0.
6. **The full workspace gate is green** (`cargo test --workspace
   --all-targets --features rl_sim`) with the one documented feel-reserved
   RED; the C4 rigs, replay fixtures, and boss/duel suites all pass.

Post-1.0 concerns (roadmap P4/P5: further clone tiers, semver/docs/template,
Q1–Q11) deliberately stay in [`roadmap.md`](../planning/roadmap.md) — they
are NOT this document's scope.

---

## 1. AJ13 — Frame awareness: the discipline (Jon's manifesto, adjudicated)

The manifesto (`frame-awareness.md`, binding) is an **architectural bias, not
a subsystem**. Nothing in this plan builds a frame graph. What changes is how
we write and review code, effective immediately:

1. **"Relative to what?" is now a review question.** Any API that takes or
   returns a position, velocity, normal, or axis NAMES its frame in its doc
   comment — `world-frame`, `body-local (x=facing)`, `gravity-local
   (y=toward-feet)`, `surface-tangent`, `screen/presentation`. The codebase
   already has good exemplars (`ActorControlFrame.locomotion` — "local frame:
   x is local side"; `AccelerationFrame`; `Contact.normal`); the rule makes
   the exemplars the norm. Ungoverned raw `Vec2`s in new signatures are a
   review flag, same tier as `AMBITION_REVIEW(spatial)`.
2. **The engine already speaks pieces of this language — name them as such.**
   `Contact { normal, tangent, surface_velocity }` IS a surface frame;
   `Block.velocity`/`SurfaceChain.velocity` IS "a support frame in motion"
   (the emergent platform-carry rule is frame composition, not a feature);
   `AccelerationFrame` IS the body's gravity frame; portal piece-mapping IS a
   frame transform. Docs/comments touching these should use the frame
   vocabulary so the mental model accretes.
3. **The camera is an observer.** The sim never reads presentation frames;
   the E-track's `ambition_sim_view` (E5) is the observer boundary made
   structural. Any sim system found consuming camera/screen state is a bug of
   this discipline.
4. **World frame stays the default; specialization stays cheap.** No new
   abstraction is required to write an ordinary AABB room or body. The
   manifesto's rule — *use the world frame by default; do not make the world
   frame sacred* — means we stop writing code that would make a second frame
   IMPOSSIBLE, not that we make every call site generic.
5. **Non-goals (explicit):** no frame-graph type, no per-entity frame
   component, no rewrite of working cardinal code. The next real pressure
   (angled portals / portal-on-moving-platform — the `PortalFrame`
   `FIXME(portal-api)` arc) will be the first consumer allowed to introduce a
   shared frame TYPE, and only for what it needs.

### AJ14 — Slower light (Jon's stretch directive; seams now, mechanic later)

Jon wants a **reduced-speed-of-light mechanic** (lower `c`; shader-driven
visuals; "the trick is how to warp space") as a stretch goal the core must
make easy. Full design + feasibility:
[`../planning/engine/slower-light.md`](../planning/engine/slower-light.md).
The adjudication in one paragraph: **you don't warp sim space — you warp the
view.** One honest Galilean sim + a `c` speed cap + per-body time dilation
(which is a γ written through the ALREADY-BUILT proper-time seam, ADR
0010/0011) + light-limited information (a `Perception::LightLimited` policy
reading retarded state); the "warp" (aberration/contraction/Doppler) is an
observer-frame post pass at the camera boundary — the AJ13 camera-as-observer
made literal. Staged as tiers L1 (sim: `LightZone` + γ + the twin test) → L2
(retarded perception) → L3 (the shaders, BLIND) → L4 (the relativity biome),
all POST-demo. **The only obligations on the live plan are Tier 0** — they
cost ≈ nothing and are folded into the slices below: E4 carries per-body
VELOCITY + the observer's velocity in the read-model and keeps one
full-screen post seam; view/snapshot builders stay functions-of-inputs (no
live-state aliasing); speed caps stay seam-shaped.

---

## 2. ANSWERS — every open question from the two frontier audits (binding)

*(Numbering continues the historical docs'. Jon's rulings are restated where
they gate a spec; everything marked **fable:** in the old docs is answered
here.)*

### Q16 — the home-path momentum branch (Jon ruled: "Sanic is BOTH") — SPEC

`integrate_home_body` (`player/body_integration.rs`) gains the same
`MotionModel` dispatch `integrate_actor_body` got in R9.1. Exact shape:

- The `players` query in `integrate_sim_bodies` (`actors/update.rs`) gains
  `Option<&mut MotionModel>`; `integrate_home_body` gains
  `motion_model: Option<&mut MotionModel>`.
- Branch point: AFTER `engine_input_from_actor_control` + the `sim_dt`
  hitstop gate, BEFORE the ledge-platform carry. On
  `Some(MotionModel::SurfaceMomentum(m))`:
  1. Build the composited view exactly as the AABB path does
     (`world_with_sandbox_solids`).
  2. Drive the R9.1 pure core `step_momentum_body` with the **GATED** input
     (`run = input.axis_x`, `jump_pressed = input.jump_pressed` — taking them
     from the gated `InputState`, NOT raw `actor_control`, keeps
     hitstun/recoil authority-reduction uniform with every other body), the
     body's `clusters.kinematics` + `clusters.ground.on_ground`, a LOCAL
     `surface_normal` var (the home body has no `ActorSurfaceState`), gravity
     `tuning.gravity_dir * tuning.gravity`, and `sim_dt`.
  3. **Hazard/OOB parity (do not skip):** after the step, apply the SAME
     gravity-relative gate the engine sim phase applies
     (`touching_hazard_aabb` + the "fell 200px past the world AABB along
     gravity" rule from `update_body_simulation_with_clusters`); on trigger
     set `events.hazard/reset`, `ae::reset_body_clusters(clusters,
     world.spawn)`, and `m.state = Airborne` (never respawn "riding" a chain
     you're no longer on). Sanic must die in pits.
  4. Write `frame_out` as usual (`was_grounded`, `pre_sim_fall_speed`,
     `reset`, and the momentum step's contacts into `events.contacts`);
     publish the hurtbox with `frame_down = -surface_normal` (falls back to
     `gravity_dir` airborne) — the §B2 rule; sprite tilt-on-slope is a
     presentation follow-up, BLIND.
  5. Skip entirely: ledge carry (no ledge grab on a momentum body v1),
     jump-buffer/dash/blink machinery (capabilities absent v1).
- **Wearing:** the StartingCharacter / `from_scratch_as_character` path
  inserts `MotionModel::SurfaceMomentum(params)` when the worn catalog row
  authors momentum params, and **REMOVES the component** when wearing a
  non-momentum character (explicit removal — remember the render-refresh
  clobber gotcha; a stale MotionModel after re-wear is the bug to test for).
- **Tests:** (a) home body worn as Sanic on a chain world rides/jumps
  (scratch-level, mirrors the R9.1 motion tests); (b) hazard reset returns to
  spawn Airborne; (c) wear-then-unwear restores the AABB path (pin: a
  non-momentum home body on `test_world` is byte-identical — positions equal
  across N frames vs a control run); (d) the possession e2e (S4) covers the
  actor side.
- Classification: **[opus, fable-specced]** — this section IS the fable pass
  Q16 asked for; opus executes it verbatim and stops if the seam disagrees.

### Q17 — R9.3 vs R7 ordering + who owns the chains channel — RULED

Jon's sequencing holds: **do NOT gate Sanic on the world carve.** Binding
split:

- **S3 (Sanic track) OWNS the `chains` emission channel now**:
  `RuntimeEntityEmission` gains `chains: Vec<ae::SurfaceChain>`;
  `compose_runtime_area` folds it into `RoomSpec.world.chains` (one field +
  one fold arm — additive). **W-track (R7.2) REBASES on it** and must not
  reinvent it; the relocation of `RuntimeEntityEmission` carries the field
  along. (Coordination note for executors: if both slices are in flight
  simultaneously, S3 lands first — it is hours, R7 is sessions.)
- **Slopes** author as the LDtk `SurfaceChain` entity (point-array field →
  the engine-registered converter; `fields.rs::parse_points` exists;
  `ambition_ldtk_tools` gains `surface add/validate`).
- **The LOOP** does NOT wait for `ron-room` and does NOT hand-edit geometry:
  author a content-registered **`SurfaceLoop` marker entity converter**
  (fields: `radius`, `segments` default 24, winding fixed interior-rideable)
  that EMITS the generated polygon chain into the new chains channel at
  conversion time. This is strictly better than the script-injection escape
  hatch (uses the landed R3.1a converter seam, respects the RoomGeometry
  write-map, exercises "content-registered converter" — the second real
  consumer of that seam). The script hatch (`World::with_chains` from a
  content plugin at staging) remains the documented fallback if the marker
  converter fights.
- **`ron-room` + baked-`RoomSpec` serde stay in W2 (R7.2)** — when they land,
  S3's room gains a `ron-room` twin as the native-IR proof; nothing in S3 is
  thrown away.

### Q18 — the profile→limb routing seam (R10.4 / G3) — SPEC

Two decisions:

1. **The translation is a NEW gameplay-core system, not a brain change.**
   `route_boss_strikes_to_limbs` reads the host's live attack state (the
   `BossAttackState` projection + `MovePlayback` phase — sim-owned since
   R1.3) and writes `LimbIntents` on the host; `tick_boss_pattern` stays
   limb-ignorant (the brain keeps emitting ONE body's frame — the
   coordinator stays "whatever brain drives the host", which is what makes
   the player-piloted giant (G5) free). Registration (deferred from R10.1):
   `route_boss_strikes_to_limbs` then `fan_out_limb_intents`, chained after
   `tick_boss_brains_system`/`steer_mount_from_rider`, before
   `integrate_sim_bodies` (features/mod.rs).
2. **The slot map is RON data on the behavior profile.**
   `BossBehaviorProfile.limb_routing: Vec<(String, LimbRoute)>` where
   `LimbRoute = { slots: Vec<String> /* "hand_left"|"hand_right" */, motion:
   LimbMotion }` and `LimbMotion = Raise | SweepAcross | SlamDown | Hold`
   (a tiny verb set the router turns into `velocity_target` arcs during the
   move's Startup/Active/Recovery phases + a `melee_pressed` edge at Active
   onset; anything richer is authored later as limb `MoveSpec`s). Unrouted
   strike keys behave exactly as today (host-body strike). Default authoring
   for gnuton (BLIND, Jon's taste pass later — his call was "don't care
   yet"): `hand_slam → { both, SlamDown }`, `hand_sweep → { facing-side hand,
   SweepAcross }` (facing-side = deterministic from host facing),
   `converging_shockwave → { both, SlamDown }`; `head_descent` stays a
   host-body move; `apple_rain` stays a Special.
3. **Limb station-keeping:** `Limb` gains `home_offset: Vec2` (host-local,
   body-frame); when a limb has no routed intent, the router writes a
   hold-station frame (`velocity_target` steering toward
   `host.pos + frame.to_world(home_offset)`). This replaces the deleted
   per-frame hand-part animation as the idle pose source.
- Classification: **[opus, fable-specced]**; the router + verbs above are the
  spec. Escalate if `BossAttackState` phases prove insufficient to time the
  arcs.

### Q19 — mount-death → the boss fights on foot (R10.3 / G2) — SPEC

1. **Bridge (a):** new message `MountDied { mount: Entity, rider: Entity }`
   written by `enforce_mount_rider_link` at dissolution. A small
   boss_encounter system consumes it and calls
   `notify_external("mount_died")` on the RIDER's `BossPhaseState` — giving
   `PhaseTriggerCondition::External` its first production caller. Do NOT
   route through the `EncounterGate` script bus — that bus is
   script-vocabulary; this is a body fact crossing into encounter state, and
   the direct bridge keeps it honest (the script bus can subscribe to the
   same message later if a set-piece wants it).
2. **Dismount brain rule (b):** the dissolution rebuild applies ONLY to
   riders whose brain identity is derived from their kit — the rule stated
   generally: **a rider carrying `BossConfig` keeps its `Brain` untouched on
   dismount** (its identity is authored, not derived). No new flag; the
   component IS the marker. Gnuton therefore lands on foot still running his
   `BossPattern`, the `mount_died` gate flips him into the authored on-foot
   mini-phase (one RON phase block, Jon kept it), and the encounter HP/HUD
   continue uninterrupted.
- Classification: **[opus, fable-specced]**.

### Q20 — `ron-room` shape + Tier-1 serde — CONFIRMED (both defaults)

`ron-room` is intentionally the **BAKED form** (a serialized `RoomSpec`,
IntGrid already compiled to `Block`s): the native format is the engine's own
spatial IR, not a new authoring schema — authored sources remain backend
files (LDtk today); `ron-room` is for generated rooms, fixtures, and
IR-level proofs. And yes, **Tier-1 serde is acceptable**: plain-data
`Serialize`/`Deserialize` derives on `World`/`Block`/`SurfaceChain`/AABB
wrappers (no Reflect, no hot-path cost, trivial compile impact). Lands in W2.

### Q21 — `MomentumParams` authoring type — RULED: gameplay-side mirror

The kernel struct stays serde-free (its doc's contract). S2 adds a
`MomentumParamsSpec` (serde) to the archetype/catalog schema with
`fn to_kernel(&self) -> ae::surface::MomentumParams` and per-field defaults
matching the kernel defaults — authored RON omits what it doesn't tune.

### Q22 — A1 tail vs the giant (coordination) — RULED: G-track first, residue after

Fold nothing preemptively. The G-track (gnuton mount split) DELETES the
hardest boss-specific machinery on its own (split overlay, per-frame hand
geometry, `StationaryGiant`) and puts the last per-frame-geometry boss onto
the moveset runtime. THEN the A1-tail residue — the boss brain-tick fold
question, `BrainSnapshot.target_pos`, `BossAnim` rows → `CharacterAnim` for
the REMAINING bosses — becomes ONE standalone slice (**E6** below), executed
after G3 with the shape G-track discoveries inform. Doing the boss-fold once,
after the boss that most distorts it is gone, beats doing it twice.

### Q23 — R4c: the persistence↔menu layering — SPEC (five ordered slices)

R4c is a SEQUENCE, menu last:

- **E1a `ambition_persistence`**: save I/O + settings **MODEL/schema** +
  `host/` + `quest/`. The boundary: persistence owns *what is stored and its
  serde shape*; anything that renders, pages, curates, or navigates settings
  is NOT persistence. The current upward reach into menu is exactly the
  settings **IR** — it stays behind (moves in E1e), and persistence exposes
  plain typed settings the IR reads. Exit: `ambition_persistence` has zero
  imports from menu/UI code.
- **E1b audio** → `ambition_audio` (mechanical, per arch.md).
- **E1c dialog runtime** → `ambition_dialog` (bindings stay sim-side).
- **E1d dev_tools** → `ambition_dev_tools` (core dev/ + app dev/).
- **E1e menu LAST**: gameplay_core menu IR + the app's 10k host stack →
  `ambition_menu` (deps: `ambition_persistence` — the layering that
  dissolves the god-dep). The `ambition_touch_input` upward-dep inversion
  (AJ7) rides THIS slice (its upward edges are menu-bridge edges).
- Classification: E1a needs care (**[opus, fable-specced]** — the boundary
  above is the spec; escalate on any type that won't classify); E1b–E1d
  **[opus]**; E1e **[opus]** after E1a.

### Q24 — R4d: breaking the combat↔features cycle — SPEC

Direction ruled: **`features` (the actor sim) depends on `combat` (the
kit); never the reverse.** No new lower crate (grow-don't-mint). Execution
shape:

1. Inventory the ~23 combat→features back-edge refs (mechanical grep).
2. Classify each: (a) a COMBAT type that historically lived features-side →
   move it combat-ward (most of them — hitbox/hit-event/volume vocabulary);
   (b) a genuine sim fact combat consumes → invert to a parameter or a
   read-model the sim passes in (the `Contact`/`FrameEvents` pattern —
   combat receives facts, it doesn't reach up for them).
3. Land (a)+(b) as compiling, committable steps INSIDE gameplay_core first
   (the cycle dies while everything still lives in one crate — cheap to
   iterate).
4. Only then the atomic move: `combat/` + the moveset runtime →
   `ambition_combat` (facade-then-delete, D2 template), then repoint.
- Classification: steps 1–3 **[opus]** (the classification rule above
  decides each ref; escalate genuinely ambiguous ones with the list in
  hand); step 4 **[opus]** mechanical.

### Q25 — R4e ordering — RULED: G1 first, then the carve

G1 (gnu sprite split) goes through the EXISTING generator + `boss_sheets.ron`
pipeline first, so all boss art is uniform BEFORE the carve. Then **E3
(R4e)** carves `ambition_sprite_sheet` (the M7 ONE pipeline) and CARRIES the
asset-root flip with it (they were always the same blocked cluster:
`ParallaxTheme` #6, `pirate_weapon` #7, projectile visual kinds, the 6
`BossSheetSpec` statics #5 — all land here, per the historical R3.4/R3.6
notes).

### Q26 — R4f readiness — CONDITIONAL GREEN

Opus runs a bounded scout FIRST (one session-hour): enumerate every
`ambition_render`/`portal_presentation` import of gameplay_core that is not
already a read-model/view/index type. If the list is view-shaped → carve
`ambition_sim_view` + fire D3.7 (**[opus]**). If real sim internals appear →
file them as D3-prep slices (each an inversion in gameplay_core) and land
those first. Do not start the carve with an unclassified list.

### Residuals

- **Q14 class string → `"giant"`** (shareable). The mount ACTOR id is
  `giant_gnu` (Jon), its `mount_class` is `"giant"`, gnuton authors
  `pilotable_mount_classes: ["giant"]` — a future mech/colossus rider joins
  the class instead of minting one. (Jon's "giant" framing leaned shareable;
  flip to `"giant_gnu"` only if he objects on the feel pass.)
- **Q2 (Jon, still open):** endorse `ambition_actors` as the R4g rename of
  the gameplay_core residue, or supply a name. Pure churn; scheduled last;
  nothing blocks on it.

---

## 3. THE TRACKS

Seven tracks. S leads (Jon: Sanic jumps the queue); W and G run parallel to
S; A and C are small independent fillers; E runs behind demo needs — **E5
(`ambition_runtime`) is the gate for both demo games** (S5, M-track). Old
R-numbers cited for traceability; §6 proves nothing was dropped.

### Track S — Sanic (LEADS)

- **S1 [opus, fable-specced]** — the home-path momentum branch (the Q16 spec,
  §2, verbatim). Was R9.2's kernel-adjacent half.
- **S2 [opus]** — Sanic the character: `MomentumParamsSpec` (Q21) on the
  archetype/catalog schema → `MotionModel::SurfaceMomentum` inserted at actor
  spawn AND on wearing (S1's seam); catalog row `sanic` (parody blue —
  original silhouette, Idle row per the sprite invariants, draw-blind, ship
  the sheet); playable via `AMBITION_START_CHARACTER=sanic`. Was R9.2's
  content half.
- **S3 [opus]** — the sandbox room (needs S2; owns the chains channel per
  Q17): `RuntimeEntityEmission.chains` + fold; the LDtk `SurfaceChain`
  entity converter + `ambition_ldtk_tools surface add/validate`; the
  `SurfaceLoop` marker converter (generated 24-gon, interior winding); a
  `sanic_sandbox` area in sandbox.ldtk (slopes, valley, loop, one knight NPC
  for coexistence); and the **debug overlay** (gizmos: chain segments,
  normals, tangents, the ridden frame, support state — deferred here from
  R8.2; draw-blind, ships with the room). Was R9.3.
- **S4 [opus]** — proofs (was R9.4): scripted reachability (loop at speed /
  fail below threshold / slope round-trip), the possession e2e (possess the
  Sanic actor → controlled body still rides — movement identity travels),
  coexistence (knight fights Sanic; combat stays AABB), overlay screenshot
  artifact for Jon (BLIND-marked).
- **S5 [senior; opus slices after E5]** — **the Sanic DEMO GAME** (was the
  Q13 promotion): `demos/demo_sanic` = one content crate + a ~100-line app
  against `ambition_runtime`. Scope v1: one momentum ZONE (multi-room LDtk
  world: slopes/loops/springs-analog), a rings-analog pickup (this is where
  the deferred `Item`-enum SET opens — violation #2 lands on real demand),
  2–3 patrol enemies reusing engine archetypes, a goal gate, title/results
  via the cutscene kit. Adversarial: every needed core edit files an
  `oracle-violation`. Exit: the demo's `git log --stat` touches zero engine
  crates.

### Track M — SMB1 (the second demo; starts once E5 lands)

Was R6's other half + the Tier-1 matrix gaps. All **[opus]** slices with a
**[senior]** assembly:

- **M1** powerup-as-equipment chain (mushroom/flower as equipment rows on the
  C1 metadata; size-change = the existing `BodyBaseSize` seam).
- **M2** camera scroll-policy knob (one-way forward scroll — a
  `CameraZoneSpec`/clamp-mode extension, authored data).
- **M3** level-end sequencing (flagpole → score walk-off) on the cutscene
  kit + `RoomLoaded`/gate vocabulary.
- **M4** `demos/demo_smb`: 2–3 levels, goomba/koopa archetypes (stomp-kill =
  the landed pogo/on-hit vocabulary), flag, adversarial log. Exit identical
  to S5's.

### Track G — the mounted giant (parallel; independent of S/W)

- **G1 [opus]** (was R10.2) — gnu sprite split via the Python generator:
  `giant_gnu` body+head sheets, hand sheets, `gnu_ton` scholar-rider sheet
  (the `scholar` anchor = `rider_offset`); actor RONs; parity baselines;
  delete per-frame hand hit-geometry from the sheet RON. **Precedes E3
  (Q25).**
- **G2 [opus, fable-specced]** (was R10.3) — archetype split + dismount: the
  `giant_gnu` mount row (`mount_class: "giant"`, big HP, real mover —
  `StationaryGiant` + `body_damage: 0` die) + `gnu_ton` rider row
  (`pilotable_mount_classes: ["giant"]`, boss identity, encounter = rider
  HP); the Q19 spec (MountDied message → `notify_external("mount_died")`
  bridge; BossConfig-keeps-Brain dismount rule); the on-foot mini-phase RON
  block.
- **G3 [opus, fable-specced]** (was R10.4) — choreography port: the Q18 spec
  (`route_boss_strikes_to_limbs` + RON `limb_routing` + `home_offset`
  station-keeping + schedule registration of both limb systems); limb
  `MoveSpec`s for `hand_slam`/`hand_sweep`; delete the
  `HAND_SLAM`/`HAND_SWEEP` StrikeRect tables + `sync_boss_split_overlay` +
  `BossOverlayLayer` + split z-consts. Boss suites retargeted; expression
  arcs BLIND.
- **G4 [opus]** (was R10.5) — authoring: `BossSpawn` gains `mounted_on`
  (mirror of the landed EnemySpawn converter); `ambition_ldtk_tools mount
  split` extension; `gnu_ton_arena` reauthored as the linked pair; roundtrip
  + validate.
- **G5 [★fable]** (was R10.6; M5 landed `328c25ce`, so this is UNBLOCKED) —
  the payoff: possess gnuton / board the giant and drive the limbs (a
  controller→limb verb map through the directional-verb resolution). Design
  slice; scheduled with fable.

### Track W — the world carve (parallel; was R7; all [opus])

- **W1** the ~13 `rooms` upward-dep inversions (unchanged scout in the 07-04
  doc's R4b section).
- **W2** IR naming in place: `RuntimeEntityEmission` (→ `RoomEmission`) +
  fold relocate IR-side **carrying S3's chains channel** (Q17);
  `SpatialSource` provenance (kills render's `"ldtk "` name-sniff);
  baked-`RoomSpec` serde + the `ron-room` manifest format (Q20 — Tier-1
  serde OK); S3's room gains its `ron-room` twin here.
- **W3** the TWO-crate carve: `ambition_world` (IR, no LDtk dep) +
  `ambition_ldtk_map` (backend; game-side dep). Compile-time before/after.
- **W4** the leakage ratchet (encounter loading → emissions; menu-map /
  session / settings inversions; schedule-set rename) + **ADR 0021**
  (authoring-backend-agnostic space, citing `spatial-model.md` +
  `frame-awareness.md`).

### Track E — the engine face (behind demo needs)

- **E1a–E1e [opus, E1a fable-specced]** — the R4c sequence (Q23 spec).
- **E2 [opus]** — R4d combat/projectiles (Q24 spec: kill the cycle in-crate
  first, then the atomic move).
- **E3 [opus]** — R4e `ambition_sprite_sheet` + the asset-root flip (after
  G1, per Q25; absorbs ParallaxTheme/#5/#7/projectile-visual residue).
- **E4 [opus]** — R4f: the Q26 scout, then `ambition_sim_view` + D3.7 if
  clean. **AJ14 Tier-0 requirements bind here:** the read-model carries
  per-rendered-body position AND velocity (world-frame, named per AJ13) plus
  the OBSERVER's velocity in the camera snapshot, and the render stack keeps
  ONE registered full-screen post-pass seam — the slower-light shaders (L3)
  and any future observer-frame effect plug in there without a schema break.
- **E5 [opus]** — R5: `ambition_runtime::PlatformerEnginePlugins` — **the
  demo gate.** Pull this forward aggressively; S5/M-track cannot start
  without it, and it needs E1e/E2/E3/E4 only to the extent the plugin groups
  reference their crates (assemble with what exists; tighten as carves land).
- **E6 [opus, after G3]** — the A1-tail residue (Q22), fully enumerated:
  (a) the `BossAnimator` frame-state split fully sim-side (a sim
  `BossAnimFrame` component; the sim stops reading a render-inserted
  component — the R1.3 follow-up); (b) remaining `BossAnim` rows →
  `CharacterAnim` rows for the non-gnuton bosses (BLIND visuals, mechanics
  pinned by frame-sample tests); (c) `BrainSnapshot.target_pos` retirement
  (the boss brain consumes its view/target directly); (d) the DECISIONS on
  the two optional deep folds recorded in the R1 close — the "no boss arm"
  integrate fold (needs the chain reorder, BLIND one-frame pose lag) and
  `BossAttackIntent` → a general move-intent (which would let the boss
  brain-tick truly fold into `tick_actor_brains`) — execute them if G-track
  left them cheap, or document them as permanent policy with rationale;
  either closes the item.
- **E7 [Jon + opus]** — R4g rename (`ambition_actors`, pending Q2) + the
  features-hub facade dissolution sweep.
- **E8 [opus]** — the last R4a near-leaf: `inventory_ui/` → `ambition_items`
  (arch.md: items owns "item/inventory/equipment machinery, shop,
  inventory-UI state"). The `time/` residue (feel / time_control /
  camera_ease) stays in gameplay_core by measurement (depends on
  player/combat/features); `camera_ease` moves WITH E4's sim_view.

### Track A — the ability-model completion (the R2 deferred trio; consumers arrived)

The 07-04 doc deferred these until a consumer existed. The demos ARE the
consumers — this track completes the JD1/AJ1 three-tier model:

- **A1 [opus]** (was R2.2) — thread `EffectRef.params` through the
  `Effect→ActorActionMessage::Special` dispatch (params ride along; today the
  bridge drops them) + the install-time param-schema validation hook from
  AJ1 (each registered technique/prefab may register a check the
  content-validation pass runs — typos fail at startup, not mid-fight).
  First consumer: any G3 limb technique or S5/M4 demo move that authors
  params.
- **A2 [opus]** (was R2.3) — the PREFAB registry: string-keyed constructors
  `(params) -> MoveSpec` expanded at roster install; generalize
  `attack_move_from_melee` / `fire_move_from_ranged` into the engine-shipped
  `simple_melee` / `simple_ranged` (+ `simple_charge` as the demos need
  them). `sword_slash` = `simple_melee` + params, zero new code. First
  consumer: the demo rosters (S5/M4 author kits as prefab rows).
- **A3 [opus, with M1]** (was R2.6) — the equipment→params merge: numeric
  equipment modifiers MERGE into the params value at trigger-resolve;
  behavioral overrides are components the technique reads. Lands WITH M1
  (SMB1 powerups are literally equipment rows modifying the body/moves) —
  the adjudicated consumer.

### Track C — residual content/cleanup sweep (small, independent, all [opus])

The last named-content/hygiene residuals from the historical audits, none
blocked:

- **C1** — `HALL_OF_CHARACTERS_AREA` (`actors/update.rs`): the bark-pool
  switch matches a room-id string. Fix as adjudicated: a room-metadata
  `gallery: bool` (LDtk level field + `RoomMetadata` + loader wiring, edited
  via `ambition_ldtk_tools level set-field`); the const dies.
- **C2** — `StartingCharacter::DEFAULT_ID = "player"` (+
  `PLAYER_CHARACTER_ID`/`PLAYER_FILE_ROOT`): the default-character SEAM
  hardcodes a content id; content injects the default (it owns
  `PLAYABLE_ROSTER[0]`) through the existing install pattern.
- **C3** — the in-game character-select follow-up from the
  starting-character feature (menu row driving the wear seam; the
  `AMBITION_START_CHARACTER` env var stays as the dev path) + the
  AbilitySet-unify note from that feature's landing — fold or explicitly
  close during E1e (menu consolidation touches the same surface).
- **C4** — sweep `dev/journals/code_smells.md`: close entries these tracks
  resolve; keep the journal honest (docs-describing-dead-things rule).

## 4. SEQUENCING (the short version)

```text
NOW  (parallel): S1 → S2 → S3 → S4        [the Sanic playable track]
                 W1 → W2 → W3 → W4        [the carve; W2 rebases on S3's chains channel]
                 G1 → G2 → G3 → G4        [the giant; G5 with fable later]
                 C1–C4, E8, A1–A2         [small, independent — good fillers]
NEXT:            E5 pulled forward (+ E1–E4 as they're ready), E6 after G3
THEN:            S5 (demo_sanic) and M1–M4+A3 (demo_smb) — adversarial, on E5
POST-DEMO:       L1–L4 slower light (AJ14; Tier-0 seams already riding E4)
                 G5 player-drives-the-giant; angled portals (frame-type arc);
                 knight-likes-on-chains (Q15: not in 1.0, seam kept)
```

Compile discipline, verification gates, BLIND rules: unchanged from the
historical docs (§6 of 07-04). The 07-04/07-05 execution logs are FROZEN;
new entries append HERE.

## 5. JON'S OPEN ITEMS (short)

- **Q2** — the `ambition_actors` rename: endorse or rename (E7).
- Feel-pass queue (standing): `unified_melee` RED, the BLIND commits ledger,
  the G3 limb-arc taste pass (Q18's slot map is fable-BLIND until then).
- **BLIND commits ledger** (opus 2026-07-05 eve): `d620a230` sanic_sandbox
  area layout — is the valley/ramp/loop placement rideable + fun? (headless
  geometry verified; play-feel unchecked). Sanic sprite (`sanic`) draws blind —
  spot-check the silhouette. Sanic momentum params (top_speed 1200 etc.) are a
  first guess; tune on the feel pass.
- **BLIND additions (fable 2026-07-05 night):** `a5d15247` G5 possessed-verb
  BINDINGS (attack→hand_sweep, down→hand_slam, up→converging_shockwave,
  special→apple_rain) — retune in `boss_profiles.ron possessed_verbs`;
  `05a32378` moveset slash VFX placement/size + the authored-blade swap on
  every moveset melee (incl. enemies) — check swings read right in-game;
  `31342e6f` swept portal transit at loop speeds — feel the c135/c134 dive.

## 6. COMPLETENESS AUDIT — every open item from the three reviews, accounted for

*(Swept 2026-07-05 night, against CODE, not just the docs. If it isn't in
this table, it was verified DONE.)*

**fable-review-2026-07-02** (E1–E66): fully executed; its adjudications were
absorbed by the 07-04 doc's AJ1–AJ7. Sole survivors: the deferred-tuning /
BULK REVIEW QUEUE items = **Jon's standing feel queue** (§5) — not agent
work.

**fable-review-2026-07-04** — open items → here:

| Historical item | Status / new home |
|---|---|
| A1 tail (brain fold, target_pos, BossAnim rows, animator split, deep folds) | **E6** (fully enumerated; verified live in code: `BrainSnapshot.target_pos` exists, animator sample flow unchanged) |
| R2.2 params-through-dispatch + validation | **A1** (verified: zero `hydrate` call sites in moveset — genuinely open) |
| R2.3 prefab registry | **A2** |
| R2.6 equipment→params | **A3** (rides M1) |
| R2.5 BLIND feel deltas; `unified_melee` RED | Jon's feel queue (§5) |
| R3.4 blocked residue: ParallaxTheme #6, pirate_weapon #7, projectile visual kinds, BossSheetSpec statics #5, asset-root flip | **E3** (Q25) |
| R3.4 residue: `HALL_OF_CHARACTERS_AREA` | **C1** (verified live at `actors/update.rs:1347`) |
| R3.4 residue: `StartingCharacter::DEFAULT_ID` | **C2** (verified live) |
| R4a leftovers: `inventory_ui/`→items; `time/` residue; camera_ease/camera_snapshot | **E8** (inventory_ui); time residue stays by measurement; camera pieces ride **E4** |
| R4b world carve | **W1–W4** (reshaped two-crate form) |
| R4c/R4d/R4e/R4f/R4g, R5 | **E1a–e / E2 / E3 / E4 / E7 / E5** (Q23–Q26 specs in §2) |
| R6 proof clones + the deferred `Item` SET | **S5 + M1–M4** (Item SET opens in S5) |
| Q2 rename; Q12 | §5; RULED (Sanic+SMB1) |
| mount M-slices | ALL LANDED (M1–M5, cutover A+B; G5 = the remaining payoff) |
| stale-doc sweeps, boundary tests, exit greps | §0 exit criteria (3, 4, 6) |

**fable-review-2026-07-05** — open items → here:

| Historical item | Status / new home |
|---|---|
| R7.1–R7.4 | **W1–W4** |
| R8.1–R8.4 | **DONE** (committed 2026-07-05: `9f13a7b8`…`30010fcf`) |
| R9.1 | **DONE** (`7041d1d0`) |
| R9.2–R9.4 | **S1 DONE** (`75f7bf8f`) **+ S3a DONE** (`8ab942ed`); rest = **S2/S3b/S4** |
| R10.1 | **DONE** (`c9b9dd02`) |
| R10.2–R10.6 | **G1–G5** (Q18/Q19 specs in §2) |
| debug overlay (deferred from R8.2) | **S3b** |
| ADR 0021 | **W4** |
| angled portals; knight-on-chains | post-1.0 seams (§0 item 5; Q15 ruled) |
| Q13–Q21 | RULED/answered (§2) |

---

## 7. IN-GAME BUG RECON — triage + deferred re-adds (opus 4.8, 2026-07-05)

A play session surfaced eight in-game defects (plus one refactor Jon wants on
the list). opus 4.8 ran a five-agent recon pass — root cause, file anchors,
regression classification — and Jon triaged each. Three cheap regressions are
fixed THIS session (see the SESSION TODO log at the bottom); the rest are
re-homed here with breadcrumbs so fable can prioritise the elegant re-add.
**The governing rule (Jon):** deleting a bespoke hacky path is fine — the
obligation is to re-add the capability *elegantly* once the unification unblocks
it. Each deferred item below is that obligation made explicit. Numbering matches
Jon's original bug list.

### 7.1 — Authored polygon melee hitboxes lost in the moveset fold — [FIXED, fable `05a32378`]
**Symptom:** the controlled robot's attack is a small square that never orients
up/down/side; the authored per-direction blade polygons are gone, and that
square (not the polygon) is the hitbox.
**Root cause:** `6806c16b` folded player melee onto the moveset runtime;
`simple_melee` synthesises a hardcoded forward `VolumeShape::Rect`
(`combat/moveset.rs:144-149`) and `advance_move_playback` spawns hitboxes purely
from `window.volumes` — it never calls `manifest_attack_hitbox_world`
(`character_sprites/attack_hitbox.rs:52-117`), which still reads the authored
convex polygon per animation and covaries with gravity/facing.
`directional_attack_variants` (`moveset.rs:488-555`) only ROTATES the synthetic
rect; it carries no authored geometry.
**Classification:** deliberate-removal regression — the authored-polygon code
still exists on the now-skipped bespoke path. The moveset volume vocabulary
(`VolumeShape::{Rect,Circle}`) has no way to say "use the sprite-manifest
authored hitbox for this move/direction."
**Elegant re-add (fable's call on shape + priority):** let a `MoveWindow` volume
REFERENCE the manifest hitbox — e.g. `VolumeShape::Manifest { animation }` (or a
per-window `authored` flag) that `advance_move_playback` resolves by calling the
existing `actor_attack_hitbox_world(character_id, move-derived-animation, …)`.
Directional resolution already exists (variant keying by `attack_axis`), so
keying the manifest lookup by move id (`attack`/`attack_up`/`attack_down`)
restores per-direction authored polygons for free. NOT urgent; fable decides
where this lands (natural sibling to Track A / the E3 moveset-presentation
surface).

### 7.2 — Attack VFX/SFX gone — [FIXED: SFX T2; VFX fable `05a32378`]
**VFX root cause (DEFER):** `MoveEventKind` has only `Sfx`/`Effect`/`Ranged`
(`ambition_entity_catalog/src/lib.rs:233-246`) — no `Vfx`/`Slash` variant — and
`dispatch_move_events` (`moveset.rs:947-1015`) has no VFX branch, so a move has
no seam to draw its slash. The old bespoke path emitted `VfxMessage::Slash` via
`spawn_melee_strike` (`combat/attack.rs:320-333`). **Missing vocabulary**, blocked
on the moveset gaining a presentation-event hook; pairs with 7.1 (both are "the
moveset can't yet express the authored strike") — do them together.
**SFX root cause (FIXED, T2):** `simple_melee` emitted the phantom cue
`"melee_swing"`, registered nowhere → `SfxMessage::Play` silently no-ops. Fixed
this session by routing to the typed `Slash` cue.

### 7.3 — All bosses render the generic sheet ("goblin") — [DEFER — needs a run; recon fix was WRONG]
**Symptom:** gnuton, smirking behemoth, and every boss render one shared generic
placeholder body (`assets.boss` / `ai_slop_zeta`, which reads as "goblin").
**Investigated this session (opus 4.8) — the recon hypothesis was disproven.**
The recon proposed dispatching boss render on `sprite_target` instead of
`behavior_id`. A direct trace of the three tables shows that would REGRESS the
bosses that currently work. Reconciliation (render key = `behavior_id.lower()`;
registered sprite keys from `dedicated_boss_sheets()`; scale keys from
`sprite_render_size_for`):

| Boss | `behavior.id` | `sprite_target` | registered sprite key(s) | current render |
|---|---|---|---|---|
| mockingbird | `mockingbird` | `mockingbird_boss` | `mockingbird` | **hits** (on `behavior_id`) |
| gnu_ton | `gnu_ton` | `gnu_ton_boss` | `gnu_ton`,`gnu_ton_body`,`gnu_ton_hands` | **hits split** (on `behavior_id`) |
| smirking_behemoth_boss | `smirking_behemoth_boss` | (none→id) | `smirking_behemoth_boss` | **hits** |
| clockwork_warden | `clockwork_warden` | `boss` | (generic) | generic (by design) |

Dispatching on `sprite_target` would look up `mockingbird_boss` / `gnu_ton_boss`
— NOT registered — and break mockingbird + gnuton. So the render key dispatch is
NOT the bug, and must NOT be "fixed" per the recon. **Corrected leading
hypotheses for the uniform symptom (need a RUN to confirm which):**
  1. **Uniform load failure** — `load_named_boss_sprite_via_catalog` returns
  `None` for every boss under the active asset profile/source (the game://-vs-
  default-source split; the boss sheet files DO exist on disk under
  `gameplay_core/assets/sprites/`), so `GameAssets.boss_sprites` is empty and
  every boss falls to `assets.boss`. This is the only mechanism that hits ALL
  bosses at once. `game_assets/mod.rs:411-423` swallows the per-boss `None`
  silently; the `MissingAssetPolicy::SilentPlaceholder` upstream hides the cause.
  2. **Regenerated boss art itself** looks generic/wrong (a sprite-pipeline
  regression, sibling to §7.8).
  A separate, real, PER-boss bug: **smirking_behemoth** — if its BossSpawn name
  resolves through `encounter_id_from_name` to `smirking_behemoth` (strips
  `_boss`) but the profile id is `smirking_behemoth_boss`, the profile lookup
  misses → `generic("smirking_behemoth")` → render key `smirking_behemoth` → the
  registered key `smirking_behemoth_boss` misses → generic. That one IS a
  slug-reconciliation fix, independent of the uniform failure.
**Classification / disposition:** DEFERRED. The fix requires (a) a run to confirm
whether `boss_sprites` is empty at runtime (add a startup log of
`boss_sprites.len()` + downgrade `SilentPlaceholder` to a logging policy), then
(b) fix the asset-source resolution if that's the cause, and (c) the
`smirking_behemoth` slug reconciliation. Home: overlaps E3 (`ambition_sprite_
sheet` / asset-root flip) + E6 (boss tail). Do NOT apply the recon's
`sprite_target` dispatch change.

### 7.4 — Morph ball draws the robot behind the ball — [DEFER → modal body morphs]
**Symptom:** entering the morph ball still renders the robot sprite behind the
procedural ball.
**Root cause:** morph has NO sprite representation — `BodyMode::MorphBall` falls
through `compact_from_mode` to a standing locomotion row
(`character_sprites/anim/mod.rs:601-608`), so the character pipeline always draws
a robot while morphed; the ball look leans entirely on a procedural circle
overlay + a single fragile `Visibility::Hidden` toggle on the player sprite
(`ambition_render/.../morph_ball.rs:136-188`). Any scheduling/possession/worn-body
edge that misses that one write leaves the robot showing.
**Jon's ruling:** a morph BALL is not a first-class engine mechanic — if codified
anywhere it's per-game logic. The first-class engine mechanic is **modal body
morphs** (a body mode owns a sprite-state supplied by the character sheet). Two
obligations: (a) **[content/asset]** add a MorphBall/roll row to the robot sprite
sheet so the character's OWN render path shows the ball (deletes the hide-toggle +
procedural overlay — the visibility race disappears); (b) **[engine]** generalise
"a `BodyMode` selects a sprite-state" so any game authors modal morphs as sheet
rows + a mode→row map, no bespoke overlay. Home: sprite pipeline /
actor-geometry-unification (Track E3 / M7). Not this session.

### 7.5 — Crouch sinks the sprite under the floor — [FIX THIS SESSION, T5]
**Symptom:** crouching drops the sprite below the collision box (feet unplanted).
**Root cause:** the crouch stance-scale block (`ambition_render/.../actors/mod.rs:
80-102`) is correct but immediately clobbered — for trimmed sheets,
`apply_character_frame` restores the STANDING render height + feet-anchor from the
first-frame `render_basis` (`actors/animation.rs:61-67`, `animator.current_render`)
at the crouched, lowered pos. Landed with `c66649fd0` (trimmed-atlas render);
slipped past `crouch_stability.rs` which only asserts engine `pos.y`, never render
size/anchor.
**Classification:** clean unintended regression. **Fix:** fold `stance_ratio_y`
into the trimmed per-frame basis so trimmed sheets respect crouch/crawl/slide AABB
shrink. See SESSION TODO.

### 7.6 — Portal high-speed tunneling (c135→c134 accelerating fall loop) — [FIXED, fable `31342e6f`]
**Symptom:** falling through the c135(floor)→c134(ceiling) translation pair builds
speed each 680px cycle; eventually the body clips PAST the aperture and lands
embedded in the floor.
**Root cause (pre-existing, NOT a refactor regression):** portal transit uses
discrete per-frame position sampling with fixed guard windows
(`APPROACH_CARVE_REACH = 96`, `CARVE_DEPTH = 60`; `ambition_portal/.../placement.rs:
306`, `pieces.rs:266`) sized for ONE worst-case per-frame step (~63px). The
relaxed fall cap (`engine_core/.../integration.rs:435-450`) lets the loop exceed
that, so in one frame the body jumps from in-front-of-plane to past the carve hole;
`transit_step` fires neither Begin nor rescue, the carve re-seals, and (under Jon's
no-pushout rule) the swept solid-sweep grounds it embedded. Solid blocks already
sweep (no tunneling); only the PORTAL trigger is discrete.
**Jon's steer:** the fix is a swept/CCD trigger (sweep `pos → pos+vel·dt` against
walls AND portal apertures), NOT a speed cap (that would kill the signature "speedy
thing comes out" mechanic). Primitives already exist: `raycast_through_portals`
(`placement.rs:25`, portal-aware segment cast) + `first_body_sweep`
(`engine_core/.../world.rs:687`) + the momentum follower's proven 500px/frame
no-tunnel CCD (R8.3). Scope = the transit TRIGGER only, not a physics rewrite; keep
the discrete carve as the low-speed fast path; add a headless regression (fling the
pair at ~500px/frame, assert transit every cycle, never embed). Independent of the
angled-portal arc (post-1.0). **fable determines the elegant shape + schedules.**

### 7.7 — Respawn policy unification: "dead stays dead" as the default — [DEFER + ADR 0022]
**Symptom:** killed NPCs infinitely respawn on room re-entry.
**Root cause:** the spawn/despawn machinery is SOUND (every actor is
`RoomScopedEntity`; all four spawners despawn-then-respawn). The defect is policy:
the default `respawn_policy()` is `OnRoomReenter` (`features/enemies/mod.rs:630-635`)
and the kill-flag writer skips it (`features/ecs/damage/actor_hit.rs:309-320`), so
default actors re-instantiate alive on re-entry; separately the peaceful-NPC
dead-flag is never READ (`features/ecs/save_sync.rs:69-94` — peaceful branch needs
the hostile flag, enemy branch needs `interaction.is_none()`, so a killed peaceful
NPC falls through both). Unification-era gap, not a spawn-code bug.
**Jon's model (the elegant unification):** fold the sandbag/training-dummy
respawn-in-place special-case INTO the respawn policy — one enum that GROWS as
mechanics land. The DEFAULT for a unique actor is **dead stays dead forever**. This
is *architecturally* less simple than room-respawn — but that is the point of
building an engine: make the intuitively-correct default the easy one for
downstream games. Later a **`Mob`** actor concept authors respawn-on-save /
respawn-on-reenter as an AUTHOR choice.
**Killable important NPCs (Jon — 100% yes, "Morrowind rules"):** killing a
questline-critical NPC is allowed; the full effect (the reality-rift "a thread of
major consequence has been cut" dialogue) is a per-GAME choice codified in
`docs/storylines/cannon.md` §Story Continuity. The ENGINE only makes that per-game
choice EASY — the default "dead stays dead" + author-selected policies are that
seam.
**Deliverables:** (1) unify respawn-in-place + respawn-policy into one grown enum;
(2) default unique actor = dead-stays-dead; (3) wire the peaceful-NPC dead-flag
read; (4) the `Mob` actor concept; (5) **ADR 0022 — engine respawn policy** (0021
is reserved by W4). *Model note for fable: this item + the ADR proposal were added
by opus 4.8.* Home: a dedicated actor-policy slice (Track E-adjacent). Not this
session.

### 7.8 — Shrine + glider sprites broken — [DEFER — sprite authoring in flux]
**Symptom:** shrine and glider render broken.
**Root cause (inferred; unverified without a run):** assets + code paths intact;
likely RON↔PNG rect drift under the measure-by-default label-gutter packing (shrine
rects start x:112, glider x:100 — offset by `label_width`, not a zero grid). A regen
that changed the packer/label width desyncs the committed RON from the emitted PNG.
(Regeneration IS possible in this env — `rectpack`+`python3` present — the recon
agents simply lacked run access.)
**Jon's ruling:** DEFER. Sprite authoring itself is likely to change; no point
fixing something that may break again or get fixed as correctness emerges from the
sprite-pipeline refactor. Revisit after Track E3 / sprite-renderer-refactor.

### 7.9 — Portal gun → a normal item (portal crate should not know the gun) — [DEFER, refactor, low priority]
*Not a bug — a decontamination Jon wants on the refactor list.*
**Today:** the portal crate knows too much about "the portal gun."
**Target:** the portal gun is a NORMAL item (like the laser sword); `ambition_portal`
knows ~nothing about the gun. The gun's only special property: the projectiles it
fires spawn portals on the surfaces they land on. **A single gun spawns exactly ONE
portal PAIR (2 modes/colours)** — not four. Each gun INSTANCE carries a small bit of
identity: which portal-pair colour ids it owns, so multiple guns don't interfere
with each other or with level-authored portals. (Jon never intended one gun to shoot
four pairs.) Otherwise it behaves like any other item.
**Shape (fable/opus to detail):** move the gun into the item/weapon vocabulary; the
portal crate exposes a "spawn a portal of pair-id P on this surface" primitive; the
gun item carries an owned pair-id (2 endpoints) and, on projectile-land, calls that
primitive. Aligns with the crit-3 "engine names no content" thrust. Home: Track
C-like decontamination / the item vocabulary (near A2 prefab items). Low priority /
good filler.

---

# EXECUTION LOG (live — newest last)

## S1 — the home body rides momentum ✅ (`75f7bf8f`, fable, 2026-07-05)
The Q16 spec executed verbatim: `integrate_home_body` gained the `MotionModel`
dispatch (players query + `Option<&mut MotionModel>`); the momentum branch
drives the R9.1 pure core over the composited view from the GATED input;
hazard/OOB parity via the kernel predicate — now exported ONCE as
`ae::movement::touching_hazard_aabb` (never a duplicated near-copy) — plus
the gravity-relative fell-out rule; reset → spawn + follower state Airborne.
`step_momentum_body` returns its contacts → the home `FrameEvents.contacts`.
`MotionModel`/`MomentumMotion` re-exported through the features hub.
Probe-verified full arc (lands, 133→900px/s cap, runs 1300px, launches off
the open chain end, falls out, resets — all correct); tests pin the ride/jump
and the pit-death-respawns-Airborne rule. gameplay_core --lib 1135,
engine_core 231.

## S3a — the chains emission channel + SurfaceChain converter ✅ (`8ab942ed`, fable)
The Q17 ruling executed: S-track owns the channel;
`RuntimeEntityEmission.chains` → `compose_runtime_area` fold →
`RoomSpec.world.chains`. New standard converter `SurfaceChain` (32nd
identifier; drift pin green): `points` semicolon pairs + optional `closed`;
the engine validator runs AT CONVERSION (bad geometry fails the loader
loudly). End-to-end test pins LDtk→World.chains + the winding convention;
degenerate chains fail conversion. gameplay_core --lib 1137.
**W2 REBASES on this** — the emission relocation carries the field.

## S2 — Sanic the character (momentum on the catalog row) ✅ (`ea632936`, opus)
The Q21/S2 spec executed: `MomentumParamsSpec` (serde) mirrors the serde-free
kernel `MomentumParams` field-for-field (every field `#[serde(default)]` off
the kernel `Default`, so authored RON omits what it doesn't tune); `to_kernel`
hydrates; `momentum: Option<_>` on the catalog entry. `momentum_params_for_
character_id` is the ONE roster lookup both seams read — `apply_worn_motion_
model` INSERTS `SurfaceMomentum` for a momentum character and REMOVES it
otherwise (the render-refresh clobber gotcha in reverse), wired into the player
spawn; the peaceful-NPC spawn resolves momentum from the Npc interaction id, so
NPC-Sanic rides too. Catalog row `sanic` (fast profile: top_speed 1200 /
ground_accel 900 / jump_speed 700) on the existing draw-blind `sanic` sheet
(verified renders — blue meme speedster, run streaks, spin-ball, full moveset);
playable via `AMBITION_START_CHARACTER=sanic`. Tests: partial/empty spec →
kernel defaults; roster resolves the fast profile + None for axis-swept/unknown;
wear-then-unwear inserts-then-removes. gameplay_core --lib + characters green.
(env: `rectpack` was missing — installed user-level so the sheet renders.)

## A1 — params through the Special dispatch + AJ1 param-schema seam ✅ (`dd62a8b6`, opus)
`ActionRequest::Special` carries `params: ParamValue` beside `spec`; the moveset
`Effect` bridge threads `effect.params` (was dropped) so a keyed technique
hydrates its own typed params — clean downward dep `characters → entity_catalog`
(leaf, no cycle). AJ1: `ParamSchemaRegistry` + `check_hydrates::<T>` in
entity_catalog — a technique registers a check; the content pass runs authored
EffectRefs through it (typo → startup error, not mid-fight). Bridge test hydrates
`(rise: 320.0)` on the far side; registry test names the offending key. Green.

## A2 — the move-prefab registry ✅ (`c4bdb516`, opus)
`attack_move_from_melee`/`fire_move_from_ranged` generalized into params-driven
engine prefabs `simple_melee`/`simple_ranged` (+ new `simple_charge`); the
authored-spec builders are byte-identical adapters (pinned). `MovePrefabRegistry`
expands `key + params → MoveSpec` at roster install (`sword_slash = simple_melee`
+ params, zero new code); unknown key / bad params fail at install. moveset 21
green. **A-track ability model is now COMPLETE (§0 crit 2): data + prefab
registry + techniques-with-params.** A3 rides M1.

## C2 — the engine names no default character ✅ (`f9392130`, opus)
`StartingCharacter::DEFAULT_ID = "player"` deleted. `Default` is now EMPTY = "no
override"; `effective_id()` resolves the concrete row LAZILY at spawn from a
content-installed default (`character_roster::install_default_character_id`,
fallback = first catalog row, never an engine literal). Content injects
`PLAYABLE_ROSTER[0]` at the catalog choke point. Consumers (wear seam,
scene_setup, render-refresh) read `effective_id()`. Green; app builds. (Residue:
`PLAYER_CHARACTER_ID`/`PLAYER_FILE_ROOT` in attack_hitbox.rs — a separate
worn-sheet-geometry concern, left for a dedicated slice.)

## C1 — gallery is room metadata, not a hardcoded id ✅ (engine `30265b75` + LDtk `27f81e66`, opus)
Engine: `RoomMetadata::gallery: bool` (merge ORs; level `field_bool`); the
ambient-bark ticker switches Hall vs Idle pool off `active_metadata().gallery`;
`HALL_OF_CHARACTERS_AREA` const deleted. LDtk: built the missing
`ambition_ldtk_tools level add-field-def` subcommand (registers a levelField
def; idempotent, type-guarded) → `gallery` def in sandbox + hall, hall authors
`gallery: true`. En route fixed two pre-existing generator bugs: stale
CATALOG/HALL paths (pointed at dead pre-R3.2 `gameplay_core/` locations — the
generator only "worked" by scaffolding a fresh dead file) and a
`--replace-existing` self-overlap (the overlap check counted the level being
replaced). `generate hall-of-characters` now cleanly regens the real content
file. Green.

**C-track status:** C1 ✅, C2 ✅. C3 (in-game character-select) → fold into E1e
(menu). C4 (code_smells sweep): no entry is resolved by S2/A1/A2/C1/C2 yet — the
Special-consumer half-vocabulary smell needs G3/E6; left honestly open.

**INTEGRATION VERIFIED:** `cargo build -p ambition_app --features rl_sim` green
after S2+A1+A2+C2 — they integrate across the whole app. (C1 engine+LDtk verified
via gameplay_core + content test suites.)

## S3b (converter) — the SurfaceLoop marker → generated rideable loop ✅ (`e5261468`, opus)
Q17's content-registered `SurfaceLoop` marker converter (the 2nd consumer of the
S3a converter seam): a `radius`+`segments` marker GENERATES a closed polygon loop
into the `chains` channel — no hand-plotted points. Winding fixed INTERIOR-
rideable (segment normals point to center → ride the inside). Registered in the
converter map; added `SurfaceChain` (an S3a gap) + `SurfaceLoop` to the Python
validator's KNOWN_ENTITIES. Tests pin the radius/closed/24-vertex shape + the
inward-normal proof. **S3b REMAINING:** `ambition_ldtk_tools surface add/validate`
subcommand (+ SurfaceLoop/SurfaceChain entity DEFS via `def register-entity`),
the `sanic_sandbox` area, the debug-overlay gizmos.

## S3b (defs) — SurfaceChain + SurfaceLoop entity defs registered ✅ (`ff4e6fa1`, opus)
The S3a/S3b converters were only reachable from synthetic tests (no LDtk
`defs.entities` entry). Registered both entity defs into sandbox.ldtk via `def
register-entity` — a level can now PLACE a `SurfaceChain` slope or a
`SurfaceLoop` marker. Content graph validates.

## S3b (area) — the sanic_sandbox momentum playground ✅ (`d620a230`, opus, BLIND)
Authored a real momentum level via `area create` from
`specs/sanic_sandbox_area.ron`: a rideable floor `SurfaceChain` (valley + launch
ramp), one `SurfaceLoop` marker (radius 200) on the floor, a HazardBlock pit +
Solid end walls, a viking-warrior knight (coexistence), PlayerStart + CameraZone;
at a verified-free world region (x=9600). Headless-verified: content graph
validates (every chain + generated loop converts), ldtk roundtrips, diff purely
additive (59→60 levels). **BLIND** — layout/feel are Jon's (see feel queue §5);
reachability (LoadingZone) + spawn-into-sanic are S4/follow-up.

## S3b (overlay) — SurfaceChain debug gizmos ✅ (`d7e7c762`, opus, draw-blind)
`draw_surface_chains` gizmo: every chain's segments (cyan) + per-segment normal
(yellow, ridden side) + tangent (green) + vertex dots, under the world-blocks
toggle. Ships with sanic_sandbox so the ride geometry + the loop's interior
winding read without playing. Mirrors the existing `draw_portals` normal/tangent
pattern; app builds.

**S3b is DONE** (converter + defs + area + overlay). The `ambition_ldtk_tools
surface` convenience subcommand is dropped as redundant — `entity add` + the
registered defs already author SurfaceChain/SurfaceLoop directly.

## C-residue — evict GNU_TON_APPLE_OWNER_PREFIX ✅ (`bdca6f56`, opus)
The engine exported a named boss-projectile prefix `"gnu_ton_apple"` used only by
content; moved it into `gradient_sentinel`, deleted the stale `is_apple_owner`
doc (that sniff no longer exists — art is `ProjectileVisualKind::Apple`), dropped
the re-export. **crit-3 grep audit (this session):** the boss PROFILES are
`cfg(test)`-only fixtures (production installs from `boss_profiles.ron`) →
"test-fixtures-only" milestone MET. Remaining PRODUCTION named-content in the
engine, each a focused slice, NOT a quick eviction:
- **`dialog/speech_sfx.rs`** — a large hardcoded `character-name → DIALOGUE_BLIP_*`
  voice table (alice/bob/pirate/ninja/goblin/… dozens). Needs a content voice-
  profile registry (the character-catalog eviction pattern). **[crit-3 slice]**
- **`projectile/visual_kind.rs`** — `ProjectileVisualKind::{Apple,Glider,Lasersword}`
  + their sprite-path art descriptors name content. This is the E3 "projectile
  visual kinds" residue. **[E3]**
- boss test fixtures (`BossBehaviorProfile::gnu_ton()` etc.) — allowed by the
  milestone; full-zero is E6/G-track.

**SESSION TALLY (opus, 2026-07-05 eve):** S2, A1, A2, C2, C1, S3b (converter +
defs + area + overlay), C-residue (GNU_TON prefix) — **11 feature commits**, all
green, ZERO regressions (gameplay_core --lib 1143, content 61, app+rl_sim build).
§0 crit 2 (ability model) COMPLETE; the whole S3 Sanic-authoring chain landed;
THREE crit-3 evictions (C1, C2, GNU_TON) + boss profiles confirmed at the
allowed "test-fixtures-only" milestone.

**HONEST §0 STATUS — every remaining item is a LARGE multi-session track, not a
bounded slice** (all the quick evictions are now done):
- **crit 1** (crate map real): the W-track (`ambition_world`/`ambition_ldtk_map`)
  + E-track carves (`ambition_persistence/menu/audio/dialog/dev_tools/combat/
  projectiles/sprite_sheet/sim_view/runtime`, `ambition_actors` rename) — none
  minted yet. This is the bulk of the remaining work.
- **crit 3** full-zero: the `speech_sfx` voice system (a multi-crate refactor —
  `DIALOGUE_BLIP_*` ids in `ambition_sfx` foundation + the engine table + a new
  content voice-profile registry), E3 projectile visual kinds, E6 boss tail.
- **crit 2** tail: E6 (boss animator split / target_pos / BossAnim→CharacterAnim).
- **crit 4** (the demos): gated on **E5** (`ambition_runtime`); neither demo
  crate exists.
- **crit 5/6**: the slower-light Tier-0 seams (ride E4), the full green gate.

## E5 (first slice) — ambition_runtime + PlatformerEnginePlugins ✅ (`3c70d827`, opus)
**The demo gate exists.** Minted the `ambition_runtime` crate (§0 crit 1: one
target-stack crate now real) with `PlatformerEnginePlugins`, a Bevy PluginGroup
bundling the 16 unconditional/unentangled engine sim plugins (world-prep,
universal brain, gravity, abilities, item pickups, feature collection/
interaction/effects/view-sync, LDtk spine, encounters, cutscenes, room reset,
traces, affordances). Solved gameplay_core's headless feature coherence
(`headless` + `input` + `portal_ldtk`). `ambition_app` now composes its sim
through the group + its app-local host wiring (input/sim/room-transition/
presentation-sync system registrations, combat/progression schedules, portal
placement — these stay app-side, tighten in later). **Parity VERIFIED:** app
builds; plugin_minimal_app + movement_axis + boss_lifecycle + collision oracle +
gravity_symmetry + architecture_boundaries (32) all green — the extraction did
not change the resolved schedule (set-based ordering). **E5 REMAINING:** migrate
the app-local host wiring + content-neutral resources into the group (or a thin
host adapter); then S5/M demo apps build on `PlatformerEnginePlugins` + their own
content crate.

**RECOMMENDED NEXT SESSION ORDER:** finish E5 (migrate the host wiring; then the
demos can start); the W-track carve (crit 1); the speech_sfx + E3/E6 crit-3
decontaminations; G1→G4 (unblock G5 [★fable]). Several need app-run / visual
verification best done interactively.

**NEXT HEADS (the bigger dents):** **G1→G4 (gnu split) unblock G5 [★fable]**;
**E5 (`ambition_runtime`) — demo gate for S5 [senior] + M-track — needs a careful
dedicated pass (guts the app boot)**; S3b→S4 continue Sanic; W1–W4 world carve
independent. The remaining §0 is the W/E crate carves + the two demos — all
multi-session.

## G2 (Q19 architecture) — mount death → the boss fights on foot ✅ (`af589a32`, opus)
The Q19 spec executed verbatim — the reusable, sprite-independent HALF of G2,
landed ahead of G1 so the G-track advances without waiting on the sprite split:
- `MountDied { mount, rider }` Bevy Message, written by `enforce_mount_rider_link`
  at the (dead-mount, still-mounted) dissolution — a body fact, NOT the
  `EncounterGate` script bus.
- Q19b dismount rule: a rider carrying `BossConfig` keeps its authored `Brain`
  untouched on dismount (no new flag — the component IS the marker); it still
  re-grounds + emits `MountDied` but lands on foot running its own brain.
- `notify_bosses_on_mount_death` — the direct bridge → `notify_external("mount_died")`,
  **`PhaseTriggerCondition::External`'s first production caller.** No duplicate
  publish path: the phase swap on `BossEncounter.encounter` is picked up by
  `update_boss_encounters` (music, level-triggered) + `boss_phase_transition_feedback`
  (feedback, edge-triggered) automatically; registered ahead of the phase driver
  so a `Combat`-set `MountDied` lands same-frame.
- Tests: boss rider keeps Brain + emits MountDied; bridge flips the boss to its
  on-foot phase; unrelated MountDied is a no-op. gameplay_core --lib green (18
  mount + 70 boss_encounter), app+rl_sim builds.
**G2 REMAINING (rides G1):** the `giant_gnu` mount row + `gnu_ton` rider row
archetype RONs (`mount_class:"giant"`, `pilotable_mount_classes:["giant"]`, big
HP, `StationaryGiant`+`body_damage:0` die) and the on-foot mini-phase RON block —
all reference the G1 split sheets, so they wait on G1.

## G1 — the gnu sprite split ✅ (`3031464b` + submodule `8bd7548`, opus, draw-blind)
The ADR 0020 sprite split, ADDITIVE (gnu_ton_boss full/body/hands byte-identical):
- **`giant_gnu`** MOUNT — the giant WITHOUT the scholar: a scholar-less `giant_body`
  layer (`_draw_body_layer(draw_man=False)` across all 6 anim rows) lockstep-packed
  with the shared `hands` layer. Sheets giant_gnu body/hands/full (3850×3850) + RON.
- **`gnu_ton_rider`** RIDER — the scholar drawn ALONE + centered (`_pack_scholar`,
  own tight trim, 362×378) + RON (no body_metrics).
- Registered in Rust (`sprites/mod.rs`: `GIANT_GNU_SHEET`=GNU_TON_SHEET clone +
  `GNU_TON_RIDER_SHEET`; 7→11 keys) + `boss_sheets.ron` (byte-consistent pin green).
- **rider_offset recorded: design-space x=44.0, y=-20.0** (`_MAN_CENTER_X/Y`) in
  `giant_gnu_actor.ron` + the `GIANT_GNU_SHEET` doc — G2 authors `Mountable::rider_offset`
  from it. Per-frame hand hit-geometry LEFT in place (coupled to G3's StrikeRect
  teardown). Verified: sprites 22 + content bosses 22 green; regen clean. Previews
  shipped BLIND for Jon's silhouette/feel check; rider `collision_scale` is a
  first-pass placeholder for G2 to tune.
**G1 DONE.** G2-archetypes rides these keys.

### G2-ARCHETYPES — DESIGN FORKS surfaced for fable (opus 2026-07-05; [opus, fable-specced] → escalated)
G1 (sprites) + G2/Q19 (dismount/bridge) landed. Authoring the `giant_gnu` mount +
`gnu_ton` rider ROWS hits three genuine boss-vs-actor taxonomy forks the executor
rules say to surface, not invent (each distorts the architecture if guessed wrong):
1. **Is `giant_gnu` a boss or a character actor?** The plan's "big HP, real mover —
   `StationaryGiant`" is BOSS vocabulary (`BossMovementProfile`, `boss_profiles.ron`),
   but a MOUNT wants `Mountable`/`mount_class`, which `attach_mount_role` grants ONLY
   in the ENEMY spawn path (`spawn_actors.rs:931`, `spawn_solo_enemy`) from
   `CharacterArchetypeSpec` — never in `spawn_boss`. AND the giant's sprite is
   registered as a BOSS sheet (`giant_gnu`/`_body`/`_hands` in boss_sheets), but
   character-archetype (enemy) actors resolve sprites by id→CHARACTER-sheet
   convention (no sprite field on `CharacterArchetypeSpec`). So "giant as an enemy
   archetype" can't find its boss-registered sprite, and "giant as a boss" has no
   mount-role attachment. → **fable: is the carried giant a brainless MOUNT actor
   (needs boss-sheet render for a non-boss + a stationary character mover) or a
   second boss? This is the Boss/NPC/prop taxonomy call.**
2. **A boss as a rider needs `CanPilot`.** `gnu_ton` stays a boss (BossConfig +
   encounter HP), but `spawn_boss`/`BossBehaviorProfile` carry no
   `pilotable_mount_classes` and `attach_mount_role` isn't called for bosses. →
   **fable/opus: extend `spawn_boss` to attach a mount role from a new authored
   field (BossBehaviorProfile or BossSpawn), OR author the rider via the enemy path
   with boss identity grafted. Bounded once the shape is chosen.**
3. **G2-archetypes is INERT without G3.** A split giant is a brainless carried body;
   the scholar-boss's scripted `hand_slam`/`hand_sweep`/`head_descent` only reach the
   giant's hands via G3's limb routing (Q18). So a *functional* split needs G2-arch
   + G3 together — G2-arch alone spawns a giant that stands doing nothing. The clean
   landing is G2-arch (rows + boss mount-role + the on-foot mini-phase, tested
   headlessly via the Q19 bridge) THEN G3 (limbs make it fight), THEN G4 (LDtk pair).
**Q19's bridge is ready to receive**: once the pair is authored + linked, killing the
`giant_gnu` mount fires `gnu_ton`'s `External("mount_died")` → on-foot mini-phase, and
the BossConfig-keeps-Brain rule lands him on foot — all landed in `af589a32`, waiting
only on these authoring forks.

### G2-ARCHETYPES — DECISIONS MADE + fork#2 LANDED (opus 2026-07-05, overruling "queue for fable" per Jon's run-and-land directive)
Jon's live directive (land it; planning-doc executor rules are overrulable) → the
taxonomy is DECIDED, boldly, per the actors-vs-props model:
- **DECISION (fork 1): `giant_gnu` is a MOUNT ACTOR** — a prop-like carried body
  (no brain of its own, `Mountable`, its own HP, `body_contact_damage: false`).
  It renders via the CHARACTER-sprite path (a catalog entry `npc_giant_gnu` →
  target `giant_gnu`, loading the G1 `giant_gnu_spritesheet.ron`), NOT the boss
  split-overlay — the giant's HANDS become limb ACTORS in G3, which retires the
  split-overlay render entirely. `gnu_ton` (the scholar) stays the BOSS.
- **fork 2 LANDED** (`{this session}`): `BossBehaviorProfile.pilotable_mount_classes`
  + `spawn_boss` attaches `CanPilot` (symmetric with the enemy `attach_mount_role`).
  A boss can now pilot a mount. Regression-pinned (existing profiles stay empty).
- **fork 3** is a sequencing fact: the split giant is inert until G3's limb routing
  makes the scholar-boss's strikes drive the giant's hand limbs.

**G-TRACK EXECUTION (opus, driving the full chain to G5):**
- **fork#2** ✅ `7873010d` — a boss can pilot a mount (`CanPilot` from `spawn_boss`).
- **slices 1+2** ✅ `3f8f03e1` — `giant_gnu` mount actor + `gnu_ton_rider` boss pair;
  the mount-death→on-foot-phase bridge (new `BossEncounterSpec.extra_phase_triggers`
  seam authoring `External("mount_died")`) is verified end-to-end on the real pair.
- **G3** ✅ `cb694744` — the giant's hands are drivable limb ACTORS; `route_boss_strikes_to_limbs`
  bridges the RidingOn/MountSlot link (rider `BossAttackState` + `limb_routing` →
  mount `LimbIntents`), `LimbMotion`/`LimbRoute` verbs, `home_offset` station-keeping,
  `spawn_giant_hand_limbs`. ADDITIVE (live fused split-overlay untouched). Test green.
- **G4 (1/2)** ✅ `<converter>` — `convert_boss_spawn` honors `mounted_on` (a boss rider).
- **G4 (2/2)** ✅ `13d5e3a2` — gnu_ton_arena reauthored as the linked pair (giant_gnu
  EnemySpawn mount + gnu_ton_rider BossSpawn `mounted_on` it) via the area spec + regen;
  BossSpawn `mounted_on` field def (cross-type EntityRef tooling in mount_split/area_authoring);
  arena-gate recognizes the rider; roundtrip/validate clean; `arena_spawns_the_adr0020_linked_pair`
  test green. Teardown of the fused profile + split-overlay DEFERRED (still test-referenced;
  logged in code_smells.md).

**✅✅ THE G-TRACK IS COMPLETE — G5 [★fable] IS UNBLOCKED.** The gnu_ton_arena now spawns
the drivable `giant_gnu` mount + `gnu_ton_rider` boss pair: the boss pilots the giant
(`CanPilot`/`ControlGrant`), the giant's hands are limb actors the boss's strikes route to
(G3), and killing the giant drops the boss to fight on foot (Q19). G5 (possess gnuton /
board the giant / drive the limbs — the controller→limb verb map) is a **fable design slice**
that now has every prerequisite in code. Remaining G-track tail: **G5 [★fable]** (design,
scheduled with fable) + the deferred fused-gnuton/split-overlay teardown (cleanup).

**PRECISE REMAINING PATH to unblock G5** (each a bounded slice under the decisions):
1. **giant_gnu mount ACTOR render** — a `character_catalog.ron` entry
   (`npc_giant_gnu` → `giant_gnu`) so `sheet_for_character_id` loads the G1 sheet
   (verify build.rs bakes `gnu_ton_boss/giant_gnu_spritesheet.ron`); a `giant_gnu`
   row in `character_archetypes.ron` (`mount_class: "giant"`, big HP,
   `body_contact_damage: false`, stationary mover, high `mass`, `default_size` ≈ the
   giant). Headless test: spawns as a `Mountable` mount actor, resolves a sheet.
2. **gnu_ton rider pairing** — author `pilotable_mount_classes: ["giant"]` +
   `sprite_target: "gnu_ton_rider"` on the gnu_ton profile (the on-foot mini-phase:
   a `BossOverrides.phase_triggers` / profile trigger `External("mount_died")` →
   on-foot phase). Headless test via the Q19 bridge: link a giant_gnu+gnu_ton pair,
   kill the mount → gnu_ton dismounts (keeps Brain) + fires mount_died phase.
   KEEP the live fused gnuton until G4 reauthors the arena (no mid-flight regression).
3. **G3** — the Q18 limb router (`route_boss_strikes_to_limbs` + RON `limb_routing`
   + `home_offset` station-keeping) + a `LimbRig` of hand limb actors on the giant;
   delete `sync_boss_split_overlay`/`BossOverlayLayer`/HAND_SLAM|SWEEP StrikeRect.
4. **G4** — `BossSpawn.mounted_on` + `ldtk_tools mount split`; reauthor gnu_ton_arena
   as the linked pair; delete the old fused gnu_ton profile. THEN **G5 [★fable]** —
   possess gnuton / drive the limbs — is executable.

### G1 READINESS (scouted, opus 2026-07-05 — headlessly TRACTABLE, next dedicated pass)
Env confirmed: PIL 12.2.0 + rectpack present; `PYTHONPATH=tools/ambition_sprite2d_renderer
python3` imports the renderer; `./regen_sprites.sh --target gnu_ton_boss` is the
wired invocation (no GPU/display). Sheets/RON are gitignored (generated) — parity
is against freshly-regenerated artifacts; the byte-identical-to-builtin invariant
already exists (`boss_sheets_ron_matches_builtin_defaults`). Precise entry points:
- **Generator** (2119 lines): `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/gnu_ton_boss/sprite_generator.py`.
  Today "split" is only intra-sheet z-LAYERS, not separate actors: the `body`
  layer (`_draw_body_layer:954`) draws body+neck+head **+ scholar fused in** (via
  `draw_gnu_ton_man`); the `hands` layer (`_draw_hands_layer:980`) is hands+VFX.
  G1 = pull the scholar OUT of the body layer → a scholar-less `giant_gnu`
  body/head(+hands) sheet + a separate `gnu_ton` scholar-rider sheet; expose the
  shoulder anchor (`_MAN_CENTER_X/Y ≈ (44,-20)`, `:940-941`) as the `rider_offset`.
- **Delete per-frame hand geometry:** `_hand_hit_frames`/`gnu_hand_*` in
  `_gnu_ton_body_metrics` (`:1546-1707`). **COUPLING CHECK (fable/executor):** the
  live hand hitbox source today is the Rust-side hardcoded StrikeRect (SLAM_STRIKE_Y
  comment `:951`), which G3 deletes when hand_slam/hand_sweep become limb MoveSpecs.
  Confirm whether the sheet-RON `body_metrics` hand frames are a live source before
  deleting standalone in G1 vs. sequencing with G3 (queue for fable if it bites).
- **Rust/RON registration:** `boss_encounter/sprites/mod.rs` (`BossSheetSpec`,
  `GNU_TON_SHEET:544`, filename consts `:514-522`, `dedicated_boss_sheets:820`) +
  `ambition_content/assets/data/boss_sheets.ron` (`gnu_ton`/`gnu_ton_body`/
  `gnu_ton_hands` keys) — add `giant_gnu` keys; author actor RONs. Render-side split
  (`ambition_render/.../actors/boss.rs`, `sync_boss_split_overlay`, `BossOverlayLayer`)
  is CONVENTION-driven (`{key}_body`+`{key}_hands`) — G1 does NOT touch it; that
  teardown is G3's.
- **StationaryGiant** is a `BossMovementProfile` arm (`boss_pattern/mod.rs:53`),
  not yet a component; gnuton spawns fused via `boss_profiles.ron:284`
  (`movement: StationaryGiant`). `giant_gnu` target/key + `scholar`/`rider_offset`
  authoring are net-new (G1/G2).

## SESSION TODO — in-game bug triage (opus 4.8, 2026-07-05) 🔧
Recon → §7. Two cheap regressions fixed this session; bug 3 downgraded after a
deeper trace disproved the recon hypothesis; the rest deferred (§7.1, 7.3, 7.4,
7.6, 7.7, 7.8, 7.9). Checklist:
- [x] **T2 (SFX, §7.2)** — `simple_melee`/`simple_charge` phantom cue
  `"melee_swing"` → the real `player.slash` procedural cue (`SWING_SFX_CUE`);
  the `Play { id }` audio path now prefers a procedural `SoundCue` when the id
  names one (`SoundCue::from_sfx_id`), falling back to the bank. **DONE.**
- [~] **T3 (boss sprites, §7.3)** — **DOWNGRADED to deferred.** Traced the
  id↔target↔registered-key tables: the current `behavior_id` dispatch is
  consistent and the recon's `sprite_target` swap would REGRESS mockingbird +
  gnuton. Uniform symptom points to a load failure / regenerated art — needs a
  run to confirm. No code changed. See §7.3 for the corrected finding.
- [x] **T5 (crouch, §7.5)** — folded `stance_ratio_y` (current/base AABB height)
  into the trimmed per-frame basis in `apply_character_frame`
  (`actors/animation.rs`); trimmed sheets now respect the crouch/crawl/slide
  shrink instead of restoring standing height/anchor at the lowered pos. **DONE.**

## G5 — possess gnuton, drive the giant's limbs ✅ (`a5d15247`, fable, 2026-07-05)
**THE G-TRACK IS FULLY COMPLETE.** The payoff slice, designed + landed as the
controller→verb map, maximally reusing what already existed:
- `BossBehaviorProfile.possessed_verbs: Vec<(verb, move_key)>` — sibling of
  `limb_routing`, authored RON. The possession arm of `tick_boss_brains_system`
  (`possessed_attack_choice`) reduces the controller's body-local aim with the
  SAME `attack_dir_from_axis` + `directional_verb_chain` every actor melee
  resolves (attack_down → attack; the boss floats, no air split), publishes the
  winning move key as fire intent via `from_move_id` — the same id
  `limb_routing` keys on, so aboard the giant the verbs ARE the limb verbs with
  zero new plumbing. No authored verbs → legacy slot(0)/signature mapping,
  byte-identical (pinned).
- gnu_ton_rider binds attack→hand_sweep, down→hand_slam,
  up→converging_shockwave, special→apple_rain (BLIND; typo-guarded by test —
  every verb must name an authored attack).
- e2e (mount tests): SlotControls down+attack on a possessed rider aboard the
  real rig → hand_slam plays → projects → routes across RidingOn/MountSlot →
  BOTH giant hands slam with the strike edge; release → move completes, hands
  re-station. Every system in the chain is the production one.

## HOTFIX — the game paniced at startup (G3 schedule cycle) ✅ (`32f42cb9`, fable)
Jon's live run hit `Error when initializing schedule Update: … before/after
cycle`. Root cause: the G3 registration demanded `.after(tick_boss_brains_system)`
AND `.before(integrate_sim_bodies)`, but the app schedule already orders
`integrate_sim_bodies < tick_npc_idle_barks < tick_boss_brains_system` —
unsatisfiable; the full composition only exists in the real app, so headless
gameplay_core suites stayed green. Fix: the limb router stays in the movement
phase and reads `BossAttackState` as the read-model it is (previous frame's
projection — the standard lag every consumer of it accepts); dropped the
impossible edge, documented the contract at the registration. **The rl_sim
headless app tests reproduce this class of failure and are the regression
guard** (they were failing; now 140 green + all integration suites; sole RED =
the documented feel-reserved `unified_melee::a_hostile_actor…`).

## §7.6 — swept (CCD) portal transit ✅ (`31342e6f`, fable)
Jon's steer executed (sweep the TRIGGER, never a speed cap): `SweptSample` +
a swept tier in `transit_step_with_tuning` — if the prev→now segment crossed a
paired portal's plane through its opening, the body fell through this frame;
transfer at any depth (`map_point` is continuous in depth). Runs in the
unlatched arm AND the post-transfer latch arm (on a fast loop the exit→entry
flight drops below one frame). Momentum honesty: carries the velocity that
PRODUCED the crossing (the grounded-at-carve-bottom embed zeroes the live one).
Teleport guard: segments longer than one frame of prior ballistic motion never
sweep. `PortalSweepAnchor` records true post-step samples in `portal_transit`.
Regression: the exact 680px floor→ceiling pair flung to ~630px/frame — transit
every cycle, never embeds. Documented bound: one crossing per step (>680px/frame
out-runs it — several times terminal). portal 46 green.

## §7.1 + §7.2 — authored blades + slash VFX on the moveset path ✅ (`05a32378`, fable)
One vocabulary, both regressions: `HitVolume.vfx: Option<String>`
("slash_arc"/"slash_poke") marks a volume as the character's BLADED swing —
(a) the slash VFX draws from the SAME spawned volume at the Active edge (the
`spawn_melee_strike` can't-diverge invariant, restored), down-tilt pokes;
(b) a tagged volume prefers the sprite-manifest's authored hit polygon for the
move's CLIP (directional variants rebind clip → per-direction rows key
themselves; attack_up already resolves a real upward hull), resolved body-local
and mirrored/rotated by the hitbox's own `place_at` — the bespoke path's
gravity covariance. Actors key by `ActorConfig.sprite_character_id`; home body
by the player root (C2 worn-sheet residue stands). Miss → synthetic rect,
payload intact. Boss geometry volumes stay silent (`vfx: None`) — no boss
slashes, no manifest overrides. `MoveEventKind` needed no new variant: the
volume tag IS the presentation seam, geometry-coupled (recorded as the §7.2
resolution). Pins: Convex blade + one Arc slash; fallback rect still hits +
slashes. gameplay_core 1157 / content 64 / app rl_sim 140 green.

**FABLE SESSION TALLY (2026-07-05 night):** G5 (`a5d15247`), startup-panic
hotfix (`32f42cb9`), §7.6 swept transit (`31342e6f`), §7.1+7.2 authored
blades/VFX (`05a32378`) — every [★fable] item on the plan is now executed;
the G-track is closed end-to-end. Remaining §7 defers: 7.3 (needs a run),
7.4 (modal body morphs, E3/M7), 7.7 (respawn policy + ADR 0022), 7.8 (sprite
authoring in flux), 7.9 (portal gun → item, low priority). Remaining §0 is
the W/E crate carves + the two demos (multi-session, [opus]).
