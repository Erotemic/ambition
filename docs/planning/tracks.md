# TRACKS — the live work queue + execution log

**This is the execution front-end.** Every open work item in the project
is here or in a doc this file points to. Executor rules:
[`vision.md`](vision.md) §7 (grades, the deviation rule);
[`decision-principles.md`](decision-principles.md) for autonomous calls.
Jon can only read, not ask.

**Standing verification gate:** `cargo build -p ambition_app --features
rl_sim` + `ambition_actors --lib` + content + app rl_sim suites +
boundary tests. **Also run the `--features portal` content suite** — the
default gate silently skips it (found by CC6, 2026-07-09).

**Living-plan discipline (README.md, binding):** every work commit
updates this file's statuses in the SAME commit; DONE detail compresses
to one line + hash in the execution log; drift between a planning doc and
the code is fixed in the doc immediately.

**When you hit a genuine design ambiguity** (a case the rulings don't
cover — not a case that's merely hard): do NOT improvise doctrine and do
NOT block. Park that slice, write a DECISION BRIEF for Jon in this file
(options, consequences, one recommendation — the Q4 brief in
[`engine/netcode.md`](engine/netcode.md) is the template), and continue
with the nearest unambiguous work. Fable's availability ended 2026-07-06
night; every fable-tier design question it left is ruled. Remaining
fable-graded material is EXECUTION-hard, not design-open.

**Read [`engine/fable-final-audit-2026-07-07.md`](engine/fable-final-audit-2026-07-07.md)
before planning any structural work** — it supersedes older card details
where they conflict, and its tail carries the next-phase queue.

---

## ▶ NEXT UP (priority order, 2026-07-09)

P2's decomposition and combat stack are DONE. What's left divides into one
arc that closes the last agent-closable playbook exit, a handful of
player-visible bugs, and the untouched determinism ladder.

1. **The demo-shell arc — D-C mode scope + a runnable `ambition_demo_sanic`.**
   [opus] The single highest-leverage item: it closes playbook **exit 3** (the
   oracle, executable) and lands D-C, the last decomposition artifact. The
   mode-scope seam is pre-solved in
   [engine/decomposition.md](engine/decomposition.md) §D-C; the reference
   assembly is `crates/ambition_host/tests/demo_shell_smoke.rs` (already
   passing). `ambition_demo_sanic` already authors `sanic_speedway` through
   the umbrella.
   **Counter-argument to fable's F9.1 ruling, per vision §7 (make the case,
   don't silently drift):** fable ruled the demo binary "a fundamentally
   interactive build" and deferred it. That conflates the SHELL with the
   FEEL. The shell is architecture and ships now; the momentum tuning is a
   knob whose value ships BLIND in a marked commit for Jon's pass
   (decision-principles: "do not let TUNING block architecture"; Jon's
   standing rules: don't pause for feel, always ship the visual). Build the
   binary, author reasonable defaults, mark the commit `blind fix:`.
   Same shape afterward for `ambition_demo_smb1`'s 1-1 geometry [sonnet].

2. **The visible sprite bugs (E3/E6 tail).** [opus] Three player-facing
   regressions in the bug queue below: all bosses render the generic sheet,
   shrine + glider sprites are broken, morph ball still draws the robot.
   Diagnosis path is already written — do a RUN with `boss_sprites.len()`
   logging, and do NOT apply the disproven `sprite_target` dispatch. Ship the
   visual; never defer to "an interactive pass".

3. ~~**N0.1 — fixed-tick sim mode.**~~ **✅ DONE (opus, 2026-07-09).** The sim
   registers into a `SimSchedule` label (default `Update`, byte-parity);
   `PlatformerEnginePlugins::fixed_tick()` hosts it in `FixedUpdate` on
   `Time<Fixed>` at 60 Hz. `SimTick` is the canonical timeline;
   `ControlFrameLatch` is the device-owned frame→tick input latch. Exit check
   met both ways, plus a split-brain guard. **Deviation from the ruled
   plumbing (resource, not per-plugin field) argued in
   [engine/netcode.md](engine/netcode.md) N0.1**, along with three recorded
   remainders (presentation interpolation, `wall_dt` semantics, one-frame
   device latency). N0.2 and N0.4 are unblocked.

4. ~~**N0.3 — the determinism lint set.**~~ **✅ DONE (opus, 2026-07-09).** Four
   greps over the sim crates + **ADR 0023** + an auditable
   `AMBITION_REVIEW(determinism)` escape hatch; every lint poison-tested.
   **The "already true" measurement was wrong**: `start_body_melee` iterated a
   `std::collections::HashSet<Entity>` and spawned strikes + wrote messages from
   that loop — per-process hash order on the hottest combat path. Fixed.

5. ~~**The dialog speaker-context slice.**~~ **✅ DONE (opus, 2026-07-09).**
   `$speaker_id` / `$listener_id` / `$speaker_is_self` published into Yarn
   variable storage at dispatch (the FIRST Yarn-variable write path in the
   codebase — everything else content reads is a library function over the state
   mirror). `<dialogue_id>__self` is the self branch; without one, self-talk is
   suppressed before the banner, the flags, the quest pump, and the mode flip.
   Identity is CHARACTER-first: the default player wears `player` and the Hall's
   player pedestal IS `player`, so `hall_player__self` — the mirror scene — is
   authored, and a content test guards that it must be.

6. ~~**N0.2 — the input-stream type.**~~ **✅ DONE (opus, 2026-07-09).**
   `engine_core::InputStream` (versioned, serde, per-tick `SlotControls` keyed by
   `SimTick`, contiguous, validated) + `runtime::InputStreamRecorder`, the one
   capture path. `SandboxSim::step_frame` drives raw `ControlFrame`s, so replay
   stops laundering the artifact through `AgentAction`. **It was more than
   promotion:** the old fixture is 60 ticks of NEUTRAL input, which only proves a
   falling body falls the same. The new `input_stream_replay` suite records a
   moving session, round-trips it through JSON, and replays it into a fresh sim
   with zero divergence. N0.4 is now a state-hash away.

7. **CC3 — the fuzz-oracle delta** (§6.1). [opus] The collision doctrine's
   exit. Diagnostic-only by Jon's ruling; a 3-check harness already exists, so
   this is an enumerated delta, not a new rig. Ranked below the above because
   it is a test rig, not a feature.

8. **Bookkeeping**: re-baseline the ledger (or rule that the adapter floor IS the
   floor) — **STILL OPEN**; an opus pass tried and got it backwards by comparing
   production-only lines against a total-lines projection. The ledger now states
   its units. ~~reconstruct or rewrite playbook exit 5~~ **DONE** (rewritten as
   four measured, ratchetable rebuild loops). Write `MODULES.md` per crate (D-B)
   — still open [sonnet].

**Deliberately NOT next:** CM6 and N1 (both land with the SSB demo, P4);
projectile steppers (blocked by design until their inputs are plain); the
S5/S6 player fold + `features/` rename (deferred until unified-actor work);
CC4 (profile first); CC7 P3a.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | **COMPLETE** — E1–E9, W1–W4, and the F1–F9 audit queue all executed; the demo gate is open, the umbrella crate exists, `placements` is the sole authored-entity channel. **Exit 5 rewritten (2026-07-09): playbook exits 1, 2, 4, 5 all met; only exit 3 (a demo binary) is open.** The ledger drift is STILL OPEN (below) | ledger ruling; D-C mode scope; D-B `MODULES.md` |
| Decomposition D-B | same | navigability standard: no module >1.5k ✅, hub globs dissolved ✅, **`MODULES.md` missing in every crate** | write `MODULES.md` per crate [sonnet] |
| Decomposition D-C | same | **NOT STARTED** — the mode-scope seam (`RoomMetadata.mode` + `in_mode("sanic")` run-condition). Demos want it; it can land early | the room-scoped run-condition helper [opus] |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | CC1 + CC2 + CC5 + CC6 (moving portals) LANDED | **CC3** — the enumerated delta from the 3-check diagnostic to the six-invariant oracle (§6.1); diagnostic-only, Jon defers hard gating [opus]. Then CC4 (profile first; NOT a CC1–CC3 precondition), CC7 P3a angled math |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | CM1–CM5 + CM7 LANDED — smash axes complete (growth, DI, charge, cancel tables, launch angles, per-move presentation) | CM6 grab/throw/shield-stun (brings OnBlock) [opus, with SSB — a P4 slice, not a P2 exit] |
| Netcode ladder | [engine/netcode.md](engine/netcode.md) | **N0.1 + N0.2 + N0.3 LANDED** (2026-07-09): `SimSchedule` seam + `fixed_tick` knob + `SimTick` + `ControlFrameLatch`; `InputStream` + `InputStreamRecorder`; determinism lints + ADR 0023 | **N0.4 desync canary** — everything it needs now exists (a fixed timeline, a replayable input artifact, the lint set); it wants the N3.1 snapshot registry for the per-tick hash. Then N1.1–N1.3. (Presentation interpolation rides the first fixed-tick *windowed* app) |
| Fighter brain | [engine/fighter-brain.md](engine/fighter-brain.md) | NEW (CM7 fed it) | FB1 view audit [opus] |
| Boss pipeline | [engine/boss-design.md](engine/boss-design.md) | NEW | BD4 seed extraction [opus/sonnet]; BD1 after |
| Falling sand | [engine/falling-sand.md](engine/falling-sand.md) | NEW; low priority | FS1 single-owner + conservation [opus] |
| S — Sanic | [demos/sanic.md](demos/sanic.md) | S1–S3 landed; `ambition_demo_sanic` authors `sanic_speedway` through the umbrella alone (the oracle held — nothing was missing) | the FEEL half + a playable binary are **interactive work**, ruled un-shippable headless. Ball-dash technique [opus] |
| M — Super Mary-O | [demos/super-mary-o.md](demos/super-mary-o.md) | `ambition_demo_smb1` registered, empty | level 1-1 geometry; M1+A3 powerup-equipment [opus] |
| F — Super Smash Siblings | [demos/super-smash-siblings.md](demos/super-smash-siblings.md) | gated on CM6 / N1 / FB | F1 rules crate |
| H — Hollow Lite | [demos/hollow-lite.md](demos/hollow-lite.md) | gated on BD pipeline | after BD7 pilot |
| Slower light | [engine/slower-light.md](engine/slower-light.md) | Tier-0 seams rode E4; L1–L4 in P5 | — |
| Docs refresh | — | P5; safe for [opus] once this stack is north star | mechanics/concepts/systems brought current |

Jon's open questions (Q1/Q2/Q3/Q5) live in [`roadmap.md`](roadmap.md).

## Drift findings (the plan vs. the measured code)

- **The residual ledger is wrong.** [decomposition.md](engine/decomposition.md)
  projects `ambition_actors` bottoming out at ≈31–35k and calls that "the
  DELIBERATE floor". Measured 2026-07-09: **64.0k src**. Roughly half of the
  projected ~64k actually left. Each carve moved the pure half and left an
  adapter shell (`boss_encounter/` 5.5k, `character_sprites/` 2.7k, `world/`
  1.9k, `projectile/` 1.8k, `dev/` `items/` `encounter/` 4.7k), and `features/`
  grew to 25.4k against a projected 20.6k. Needs either a re-measured ledger or
  an explicit ruling that the adapter floor IS the floor.
  **UNITS (2026-07-09, the hard-won part):** all these figures — and the
  projection's — are TOTAL src lines INCLUDING TESTS. An opus attempt to
  re-baseline in production-only lines concluded the alarm was a counting error
  and **was itself the error**; it is retracted. The projection's baseline
  (101.7k) is exactly the monolith's total src at that commit, and its residual
  breakdown (`player/` 6.6k, `abilities/` 4.2k) matches today's TOTALS. The one
  durable finding: `ambition_actors` is 43% test code (27.8k of 64.0k), which is
  worth knowing when SCOPING a carve but is not the comparison. **State the
  units in every ledger.**
- ~~**Playbook exit 5 cannot be met as written.**~~ **REWRITTEN and met (opus,
  2026-07-09).** A relative criterion against a baseline nobody recorded is not
  a criterion, and a pre-D-A checkout would now time a different Bevy. Replaced
  with four measured, absolute, ratchetable rebuild loops (see
  [decomposition.md](engine/decomposition.md) exit 5). The headline: editing
  CONTENT rebuilds the app in **9.4 s** — the decomposition's actual payoff —
  editing a leaf sim module rebuilds `ambition_actors` in **3.2 s**, and the
  full play loop after a sim edit is **104 s**, which is the residual cost of
  everything above `ambition_actors` relinking.
- **Playbook exit 3 is Jon-gated, not agent-gated.** A demo app building
  from runtime+host+content with zero engine edits needs a demo binary;
  fable ruled the feel half interactive.

## The bug/polish queue (Jon's play reports, homed)

| Item | Home | Grade |
|---|---|---|
| Slash VFX renders as a black square | render-side sprite-source quirk; needs a visual run (CM5 did NOT close it) | [opus] |
| DI + smash-charge feel/input seams | `di_max_angle` defaults OFF (a feel number Jon sets); partial-charge-on-early-release needs an `attack_held`/`attack_released` signal on `ActorControlFrame` + input mapping. Scaling is wired; only the trigger is deferred | [Jon's feel eye] |
| `SurfaceRamp` quarter-circle marker entities (Q27) | arc math + the 4-case winding-oracle protocol PINNED in [spatial-model.md](engine/spatial-model.md) §SurfaceRamp | [opus/sonnet] |
| Morph ball still draws the robot; generalize modal body morphs | E3 (mode→sprite-state row) | [opus] |
| Shrine + glider sprites broken | E3 (rect drift; sprite pipeline) | [opus] |
| All bosses render the generic sheet | needs a RUN with `boss_sprites.len()` logging; do NOT apply the disproven `sprite_target` dispatch | [opus] |
| ~~Dialogs don't adapt to WHO is talking~~ **DONE 2026-07-09** | `ambition_dialog::{DialogueContext, DialogueNodeIndex}`; `interact_ecs_actors_and_switches` resolves both ids and suppresses trace-free. **Amendment to the pin:** identity is the CHARACTER id where a body has one (`InteractionKind::Npc.character_id`, or the home avatar's worn `StartingCharacter`), falling back to the placement id. The pin's literal "config.id vs target id" would make `$speaker_is_self` fire only when a possessed body interacts with its own placement — never at the Hall, which is the case that motivated the slice | — |
| Sanic ball-dash special | [demos/sanic.md](demos/sanic.md) — release→velocity technique + hurtbox-resize seam, inside the S5 content crate | [opus] |
| Portal gun should be a normal item | portal exposes `spawn portal of pair P on surface`; one gun = one pair | [opus, low priority] |
| Build cache re-balloons | `$CARGO_TARGET_DIR/debug` hit 351G once. Consider `cargo-sweep` or a periodic prune | [Jon] |
| Smells journal (`dev/journals/code_smells.md`) | C4-style sweep rides each related track; the journal stays the intake | — |

**The BLIND ledger (standing, Jon-only):** sanic area layout (`d620a230`),
sanic sheet/params, G3 limb arcs + G5 verb bindings (`a5d15247`), moveset
slash VFX placement (`05a32378`), swept-transit feel (`31342e6f`).

## Oracle-violation log (demos file here; engine work exits through tracks)

*(empty — the discipline: demo commits never touch engine crates; each
violation gets a row here + a slice in the right doc.)*

---

# EXECUTION LOG (one line per session; append newest last)

## 2026-07-05
- **fable** — the planning consolidation: `docs/planning/` rebuilt as the single source of truth; reviews archived. (`c8de27d5`)
- **fable** — Sanic-in-normal-rooms + wear semantics: blocks are surfaces (`SurfaceRef::Block`, boundary chains, load-bearing landing rule); wear = possession, no kit fallback; blink gated off momentum bodies. M14 + M16 recorded. (`0189338b`)

## 2026-07-06
- **fable** — the refinement pass: architecture.md rewritten around evergreen ROLE handles + the workspace push-target layout; Q27 (backends deferred, SurfaceRamp instead) / Q28 (parody names = policy) / Q29 (respawn triage) / Q30 (fable window) recorded; the Q4 decision brief written for Jon.
- **fable** — respawn unification, ADR 0022: ONE authored `RespawnPolicy` (default `DeadStaysDead`), one carrier (`ActorTuning.respawn`), one kill-path match, placement-pinned NPC policy, universal liveness-on-load. The infinite-NPC-respawn bug is dead at the root. Plus E4-17, the camera OBSERVATION seam. (`23b81c99`)
- **fable** — E5-finish steps 1–4: `SandboxSetsPlugin`, `CombatSchedulePlugin` moved in wholesale, `add_headless_foundation` + `init_engine_states` converge the copy-pasted foundation blocks; cut-rope content de-woven onto labeled slots.
- **opus** — CM1: the knockback-scaling axis. `HitVolume.{kb_growth, launch_dir}`, `ActorTuning.{weight, death_policy}`; `scaled_knockback` applied victim-side at the moveset-hitbox overlap. All defaults byte-parity.
- **opus** — CM2: directional influence. `di_adjust` rotates the victim's own launch toward its held input, bounded by `SandboxFeelTuning.di_max_angle` (default 0.0 = off). Reads the same gated input every system reads, so a CPU/RL policy DIs like a human.
- **opus** — CM3: smash-charge scaling. `MoveSpec.smash_charge_mult` + `charge_scale_at(t)` — the charge state IS the move's clock, no new component. Smash verbs resolve distinctly via `directional_verb_chain`.
- **opus** — CM7 frame data (`MoveSpec::frame_data()`, feeds FB2) + CC1 to its safe boundary (`engine_core::cast` minted); the three cast-consolidation rulings logged for fable. (`800419ff`)
- **fable** — THE RUNTIME-CONTRACT PASS: collision-and-ccd.md rewritten with the pinned contracts — §3.1 `SweepSample`, §3.2 authority classes A/B/C + one-Class-B-per-frame, §3.3 per-trigger semantics + `AMBITION_REVIEW(discrete_ok)`, §3.4 cast identity, §3.5 portal-aware cast, §4 SurfacePolygon, §5 moving-portal object model, §6 the six-invariant fuzz oracle, §7 CC5 frame conventions, §8 minimum-slice separation. netcode N3.1, E4's sketch, boss-design calibration, and the FB6 budget contract all grew pins. (`cdb0e5c8`)
- **opus** — CC2 first pass: `cast::aabb_path_contacts` (the trigger-tier swept primitive) + hazards converted, both victim arms. A fast body can no longer leap a spike.
- **fable** — the fable-window run: CC5 + CC1 COMPLETE (`engine_core::frame` minted, `pieces::PortalFrame` replaced with no shim, `cast::ray_through_apertures` with the flush-mount tie-break) (`9aa4d998`, `a413f4b2`, `2c74a3f4`); CM4 cancel tables on the timeline (`ef132da9`); E4 slice 19 (`65606d5b`).
- **opus** — CC2 COMPLETE: every §3.3 reader declares its verb. Loading-zone entry swept (`transition_for_player`); water/climbable annotated discrete-OK + `World::thin_region_warnings` authoring validator; ledge audited; auto-collect N/A.
- **opus** — CM5: per-move presentation is DATA. `MoveEventKind::Vfx{effect}` through the content-registered vfx vocabulary; `swing_sfx`/`swing_vfx` prefab params; a typo'd cue fails at the startup gate, never silently. "One generic swing everywhere" is dead.
- **opus** — app residue: the progression-schedule content de-weave. Three engine labeled slots; the engine chain now names NO content. Ordering preserved byte-for-byte.
- **opus** — RED fixed: `gnu_ton::arena_spawns_the_adr0020_linked_pair`. Root cause `68943d28` "Commit loose data" nulled the rider's `mounted_on` EntityRef; restored via `entity set-field` (tool, not hand-edit).
- **fable** — E4 EXECUTED: slices 1–20 + the `ambition_sim_view` mint. Pose/anim/feature/boss/nameplate/hud/item/prop/camera read-models rebuild sim-side; render is a pure consumer; `ControlledSubject` never appears in render; `observation_boundary.rs` forbids ~45 live sim-state type names in render sources forever. RULING: camera-EASE stays sim-side. (`d5675f27`…`971bb41a`)
- **fable** — CM1 COMPLETE: authored launch angles. `launch_dir` is direction-only, victim-gravity-frame, x mirrored away-from-source; it replaces the default diagonal while PRESERVING its speed, so an authored angle can never out-throw the feel launch. (`c695cd9c`)
- **opus** — W1 STATE-inversion: `load_room_geometry` dropped its four cross-domain params; the composition tier applies the transition resets (anti-god rule 6). The `world → characters + combat` VOCAB arrow escalated to fable with a pre-solved option matrix.
- **opus** — E5 step-5 de-risked: the "gated on E1d/E1e" accounting was DISPROVEN; `ambition_host` scaffold + boundary test landed so fable's carve is a pure system-move.
- **fable, night** — **E5 STEP 5 + 6 EXECUTED — THE DEMO GATE IS OPEN.** Card amended: shared per-frame sim wiring belongs in the ENGINE group (headless/RL add it too), so `ambition_runtime` grew four per-domain schedule plugins and `ambition_host` = leafwing bindings + camera cluster. `SimCoreResourcesPlugin` minted; `demo_shell_smoke.rs` PASSES — a demo-shaped app boots and ticks. Also: W-a…W-e RULED (Tier-0 catalog stays serde-only; `KinematicPath` → engine_core; `DamageVolume` dissolves into `PlacementRecord` + Tier-0 spec; two-stage lowering registry; `WorldDelta` = ordered ops; placement ids REQUIRED; unknown placement = hard error), `GeoId` §3.6 RULED, and the opus-proofing detail pass pinned CM6 / A3 / N0.1 / BD1 / BD6 / FB4 / E6(d) / E7 / SurfaceRamp / dialog-context / falling-sand-spout. Zero `QUESTION FOR FABLE` markers remain.
- **opus** — the decomposition-unblock run: W-queue step 1 (`entity_catalog::placements` + `engine_core::kinematic_path`, no shims), all 7 E2 in-place back-edge verdicts (byte-parity, one commit each), and the `GeoId` substrate (`Block.id`, Anon default, inert). **SweepSample PARKED** with a decision brief — genuine ECS-seam ambiguity on the hottest engine struct.
- **Codex** — E1a: `ambition_persistence` owns saved shapes (save I/O, `UserSettings`, quest specs/registry).
- **Codex** — E1b: `ambition_audio` owns the reusable SFX-bank runtime; the dead encounter-music fallback deleted.
- **opus** — E1c: `ambition_dialog` owns the dialogue runtime. Two seams make it content-free: GameMode decoupling, and installer-only Yarn vocabulary (`YarnContentBindings`).

## 2026-07-07
- **fable** — SweepSample RULED + LANDED; the parked slice closed. The ruling beat all three options: **the sample is the simulation phase's OWN integration segment, both endpoints captured inside the kernel.** So the ~20-site reset surface DOES NOT EXIST (teleports happen outside the sim window and can never become path), and **blink is a teleport, never path** — enforced for free by the control/sim phase split. The hazard reader migrated to `sample.delta()`. CC6 fully unblocked.
- **fable** — W2 EXECUTED (W2.1–W2.4): serde across the engine IR spine with REAL GeoIds (IntGrid → `TileLayer` + row-major merge ordinal; entity blocks → `Placement(iid)`); render's `"ldtk "` name-sniff DEAD; `RoomEmission` rename; the `PlacementRecord` channel minted; `world::ron_room` round-trips the sanic area as a string fixed point with no LDtk in the second path. **Ruling amendment: `GeoSource` IS the provenance model — no `SpatialSource` was minted.**
- **fable** — E2 EXECUTED: the combat kit IS `ambition_combat`. Eleven compiling commits; the `authored_volumes` INSTALL SEAM minted so combat asks for artist-authored hit polygons through an installed resolver; `Option<&ActorConfig>` is GONE from combat. Combat's upward surface hit ZERO. (`727bafe6`)
- **opus** — E2 tail: `ambition_projectiles` owns the projectile MODEL. The real split is model-vs-stepper, not the raw ref count; the model deps do NOT include combat. Victim/world/anim steppers stay actor-side (boss types = the E6 blocker).
- **opus** — E1d: `ambition_dev_tools` owns the dev-tool STATE. Card deviation recorded: the state is consumed below app so it must be foundational; the egui overlays stay app-level.
- **opus** — E1e: `ambition_settings_menu` (the god-dep dissolution — pure logic, no bevy, no renderer) + `game/ambition_menu_kaleidoscope`, **the first extension crate**. Two independent renderers now drive one page model. C3 explicitly closed. **E1 COMPLETE.**
- **Codex** — E-assets: `ambition_asset_manager::sandbox_assets` owns the catalog/source layer; upward reads inverted into plain `SandboxCatalogInputs` rows.
- **Codex** — W3/W4: `ambition_world` (room IR, placements + lowering registry, platform math) and `ambition_ldtk_map` (the backend) split; ADR 0021 + boundary test. The backend ships no hidden game content.
- **Codex** — E3: `ambition_sprite_sheet::character` owns `CharacterAnim`, sheet specs/geometry, animator, baked RON tables. Also fixed a portal feature-forwarding mismatch under Cargo unification.
- **Codex** — E-enc: `ambition_encounter` owns wave specs/state/events/registry/music/reward math; the ECS/LDtk adapters stay actor-side by design.
- **Codex** — E6 tail: (a) boss anim frame → sim-owned state, closing the E4 slice-8 boundary violation; (c) `BrainSnapshot.target_pos` retired from production; (b/d) the two deep folds closed by permanent code-site policy comments (`BossAnim` rows are authored attack-geometry verbs — folding them through `CharacterAnim` would mislabel them).
- **Codex** — E8: `ambition_items`. E7: workspace re-home (`ambition_app` + `ambition_content` → `game/`), then five combat/vocab facade cleanups across runtime/app/content/sim-view/render, and three E4 render import cleanups.
- **Codex** — F1.5 first cut: render reads rooms/camera/sheet vocabulary from the lower crates; an exact-count ratchet added.
- **fable, FINAL** — the whole-repo audit → [engine/fable-final-audit-2026-07-07.md](engine/fable-final-audit-2026-07-07.md). DAG sound, eleven arrows prescribed (F1); `ambition_actors` classified (F2); rulings verified (F3); **TWO rename-fallout regressions found and FIXED** — the desktop asset root silently degraded (the game ran with no assets) and the music-tool repo probe was dead (F4); the `ambition` umbrella + demo homes proposed (F5); gate green (F6); the lowering seam had three real defects, fixed (F7).
- **Codex** — F1.1/F3 world-purity first cut (`ron_room` re-sided into `ambition_world`; room-transition SFX became a plain cue id; the world dependency allow-list test landed). F1.2 the `actors::portal` facade DELETED. F1.3 `ambition_vfx` owns `HitSide`, dropping its characters dep. F1.4 `GameMode` moved down to `platformer_primitives::schedule`.

## 2026-07-08 (Codex)
- F1.5 complete: **`ambition_render` no longer depends on `ambition_actors`.** `GameAssets`/boss render types → `ambition_sprite_sheet`; physics settings + feature overlay + shrine pulse → `platformer_primitives`; `SandboxDevState` → `dev_tools`; render's feature visuals read `FeatureView`, not live ECS.
- F1.6 `ambition_inventory_ui` split out of items. F1.7 `ControlFrame` → `engine_core`, so reusable character brains no longer depend on the input adapter. F1.8 the unused asset-manager↔sfx adapter deleted.
- F1.9 + F1.11 closed as explicit **no-move rulings** (runtime IS the engine composition tier; `ambition_touch_input` owns the visible touch HUD, so its render dep is correct — it is a presentation/input adapter with a legacy name). F1.10 `ambition_host → ambition_actors` removed. F1.1 closed.
- **F2 CLOSED:** the `ambition_actors` compatibility-facade burn-down — GameMode, camera layers/ease, `SandboxDevState`, `ControlledSubject`, character-sprites, assets, projectile scheduling, dev-tools, audio, schedule labels, menu backend, settings/menu IR, encounter vocabulary, dialog/dev-persistence, `MapMenuState`. Every deletion ratcheted. Deeper actor decomposition moves to later cards.

## 2026-07-09
- **Codex** — F4.3 `ClockResetRequest` routes reset intent through the one time-control owner. F4.4 deterministic lowest-`PlayerSlot` fallbacks replace raw Bevy query order, tagged `AMBITION_REVIEW(determinism)`. F3.2 swept-mover closeout: ECS actors/bosses REQUIRE `SweepSample`; `PortalSweepAnchor` retired; portal CCD feeds from the kernel sample with a live-endpoint guard against reading a teleport as a crossing.
- **E9 umbrella:** `crates/ambition` re-exports runtime/host/render/world/model/vocabulary + a curated prelude; `game/ambition_demo_sanic` + `game/ambition_demo_smb1` registered, depping ONLY the umbrella, oracle-ratcheted. The app manifest collapsed to three `ambition*` deps (facade + content + kaleidoscope).
- The `unified_melee` feel-RED was **diagnosed as a stale read-model assumption in the TEST**, not a sim regression: it now follows the hostile by `FeatureId + BodyMelee` and accepts both swing authorities (flat `BodyMelee` and the moveset-backed `MovePlayback`). **Gate: 44/44 suites green, zero failures.**
- Projectile residual-glue slices: the substrate-only enemy/boss `Effect::Projectiles` spawn executor → `ambition_projectiles::enemy`; kind-specific expiry VFX → `ProjectileVisualKind::expiry_vfx`; pure primitive tests travelled to the model crate.
- **fable** — F9 verification pass (independent, against manifests/source): all F1 arrows closed, world purity real, F3.2/F4.3/F4.4 closed, E9 exit met. RULED: the IR-native-family route for F1.1 is ACCEPTED, but the resulting two-channel IR is an internal split-brain with a real tax — record-over-schema consolidation continues, one family per session, exiting when the dual-emit guard deletes.
- **fable** — **CC6 MOVING PORTALS LANDED** (§5-P2 in full; amendments in §5-P2a). Host-attached frames via `GeoFaceRef`, the relative swept trigger (the scoop works; co-moving bodies never spuriously transit), Galilean transfer with the exit-REST-frame min-exit floor, host-carried motion exempt from eviction (close-only pushout preserved), per-frame frame re-derivation so a portal closes with its host face. Found and fixed: the default gate was silently skipping the `--features portal` content suite (101 tests).
- **fable** — F5.4: the gate-portal phase tests travelled to `ambition_world::rooms::gate_portal`.
- **fable** — **the F9.2 IR-consolidation arc, families 1–6, CLOSED.** Interactables → pickups → chests → breakables were Tier-0 MOVES into `entity_catalog::placements` (one pure type, no schema/world mirror). Portals were the deliberate `Vec2` exception, done as a Tier-0 `PortalSchema` mirror whose lowering DERIVES the face center from the record's `aabb.center()`. Hazards closed the arc: `convert_damage_volume` now LIFTS a legacy inline `motion: KinematicPath` to a synthesized room-level path (`{iid}__inline_motion`) referenced by `path_id`, behavior-preserving. **`RoomSpec` carries zero typed per-family Vecs, there are zero typed spawn loops, and the dual-emit guard is DELETED — `placements` is the sole authored-entity channel.** A future authored family adds ONE `PlacementSchema` variant + one lowering interpreter.
- **fable** — F9.1: `ambition_demo_sanic` authors a real momentum showcase room (`sanic_speedway` — long solid floor + a rideable loop as an interior-winding `SurfaceChain`) built entirely through the `ambition` umbrella, with a headless test that composes it and runs the engine's own chain validator. **The oracle held — nothing was missing from the re-exports.** RULING: the FEEL half (momentum tuning to a Sanic identity, a playable binary, character art) is a fundamentally interactive build and cannot be responsibly completed headlessly.
- **Jon** — the CC6 content-side host adapter committed. (`c9ef23d8`)
- **opus** — bookkeeping. **Playbook exit 5 rewritten** from a comparison against a
  never-recorded baseline into four measured, absolute, ratchetable rebuild loops;
  content authoring rebuilds the app in 9.4 s. **The ledger re-baseline was
  attempted and RETRACTED**: it argued the "63.5k" alarm was a counting error
  because 27.8k of that is test code — but the 2026-07-06 projection was itself in
  total lines (its 101.7k baseline is the monolith's total src; its residual
  breakdown matches today's totals). The alarm stands, the open question reopens,
  and the ledger now states its units. `MODULES.md` per crate remains open.
- **opus** — **N0.2 INPUT STREAM.** `engine_core::InputStream` is the one per-tick
  input artifact (versioned, serde, `SimTick`-keyed, contiguous, validated), and
  `runtime::InputStreamRecorder` the one capture path — recording the frame the
  SIM consumed, not the device frame, because gestures / portal warp / the
  fixed-tick latch rewrite it in between. `SandboxSim::step_frame` replays raw
  `ControlFrame`s. Exit: record a moving session → validate → JSON round-trip →
  replay into a FRESH sim → zero divergence, tick for tick.
- **opus** — **DIALOG SPEAKER-CONTEXT.** A conversation now knows who is in it.
  `DialogueContext` (speaker id, listener id, `speaker_is_self`) rides the
  pending-start request; the Yarn bridge publishes it into variable storage
  before the node begins — the first `$variable` write in the project (every
  other Yarn read is a library function over the state mirror; identity is fixed
  for a conversation and read at line zero, so a variable is the right shape).
  `DialogueNodeIndex` lets the SIM ask "did content author a `__self` branch?"
  with no Yarn dependency, so a self-conversation with nothing written for it is
  suppressed at dispatch — before banner, flags, quest pump, mode flip — instead
  of opening a dialogue box and closing it. Identity is character-first, so
  wearing a character and inspecting its Hall pedestal IS self-talk;
  `hall_player__self` (the mirror scene) is authored because the default
  character makes that the likeliest interaction in the game.
- **opus** — **N0.1 FIXED-TICK LANDED.** The sim no longer names a schedule: it
  registers into `SimSchedule` (`platformer_primitives::schedule`, default
  `Update`), which `PlatformerEnginePlugins::fixed_tick()` swaps for
  `FixedUpdate` on `Time<Fixed>` at 60 Hz. `SimTick` (`ambition_time`) is the
  canonical timeline; `ControlFrameLatch` (`engine_core`) folds device samples
  into one per-tick frame (axes latest, edges OR) and is owned by the DEVICE
  layer, so headless/RL/replay drivers keep authoring `ControlFrame` directly.
  Bullet-time composes inside the tick for free — `run_fixed_main_schedule`
  swaps the generic `Time` to the fixed clock, so `refresh_world_time` yields
  `TICK_DT × time_scale` with no fixed-tick special case. Exit met: the rl_sim
  phase-split suites pass both ways, and a schedule-graph guard fails if any
  sim system is stranded in `Update` under fixed tick. **The label is sealed on
  first read** — a late mode change panics instead of silently splitting the
  graph. Executor deviation (resource vs. per-plugin field) argued in
  [engine/netcode.md](engine/netcode.md).
