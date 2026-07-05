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

**SESSION TALLY (opus, 2026-07-05 eve):** S2, A1, A2, C2, C1, S3b (converter +
defs + area + overlay) — **10 feature commits.** §0 crit 2 (ability model)
COMPLETE; two crit-3 residues (C1, C2) closed; the whole S3 Sanic-authoring chain
(momentum body → character → chains channel → converters → a real playground
level with a debug overlay) is landed. **S-TRACK NEXT = S4 proofs** (scripted
reachability at speed / fail-below-threshold, the possession e2e, knight
coexistence, the overlay screenshot artifact) — then S5 the demo game, gated on
**E5** (`ambition_runtime`). Parallel big dents remain: **G1→G4 unblock G5
[★fable]**, **E5 the demo gate**, **W1–W4** the world carve, and the two demos.

**NEXT HEADS (the bigger dents):** **G1→G4 (gnu split) unblock G5 [★fable]**;
**E5 (`ambition_runtime`) — demo gate for S5 [senior] + M-track — needs a careful
dedicated pass (guts the app boot)**; S3b→S4 continue Sanic; W1–W4 world carve
independent. The remaining §0 is the W/E crate carves + the two demos — all
multi-session.
