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

## ▶ NEXT UP (priority order, audit-normalized 2026-07-10)

### Audit correction order — binding

The next structural work is split into three series. Detailed investigation and
acceptance criteria live in
[`../archive/reviews/static-audit-response-2026-07-10.md`](../archive/reviews/static-audit-response-2026-07-10.md).

1. **Series 1 — guardrail credibility + baseline normalization.** Hard-fail every
   required room load; land the missing-room poison test with that fix; prove the
   resource/message coverage ledger reacts to injected mutable state; expand N0.3
   to every simulation-bearing source root; reopen D-B with an executable line
   gate and an explicit per-path waiver list; regenerate stale module maps.
2. **Series 2 — N3.2 exact-restore substrate.** Enforce unique identities (with
   the duplicate-`SimId` poison test in the same commit); diagnose and enforce
   active-room ownership; compute stale state after reconciliation; make codec
   failures explicit (with the corrupted-blob poison test in the same commit);
   redefine `lossless()` from positive component/resource/message/codec coverage;
   then address dynamic-spawn reconstruction and bounded rollback.
3. **Series 3 — evidence ledger.** This commit is pass 1: correct status and false
   diagnoses before they steer work. Pass 2 happens only after Series 1/2 are
   green and compresses completion evidence to enforced invariants.

**Poison-test atomicity is binding:** a poison test and the behavior that makes it
pass share a commit unless the behavior already exists. Do not land known-red
future tests or tests that pin the current bug merely to keep the tree green.

**Locked audit framings:** required rooms can disappear from the gate; loaded
rooms do not silently pass incorrect canary/replay results. The N3.1 keystone is
valuable; the prose overstated what it proves, and exactness is N3.2 work.

The historical execution sequence below remains useful context, but the audit
correction order above supersedes it for structural work. Existing completed slices
stay completed unless explicitly reopened or reclassified here.

0. **P0 architecture refactor - unified encounter orchestration (Jon's idea).**
   **NEXT after the currently in-flight test-organization work.** Split encounter
   orchestration from boss/actor construction and converge the generic wave system
   plus boss encounter wrapper onto one first-class, event-driven encounter entity.
   Encounters may compose ordinary enemies, boss-capable actors, hazards, puzzles,
   races, escorts, or no actors at all; actors retain local phase/capability state
   and work outside encounters. This is delete-heavy work: the measured migration
   surface is about 3,681 total source lines including tests, and completion requires
   one authority plus a materially smaller code surface. Design, slices, and exits:
   [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md).

   **Do not interleave its code migration with the active test refactor.** The docs
   overlay is safe; implementation starts from the resulting clean base. The Sanic
   visible/input/character recovery is independent and may land first:
   [`demos/sanic-recovery.md`](demos/sanic-recovery.md).

1. ~~**The demo-shell arc — a runnable `ambition_demo_sanic`.**~~
   ✅ **DONE 2026-07-10 — playbook exit 3 is MET and gate-enforced.**
   `game/ambition_demo_sanic_app` (manifest: `ambition` + `ambition_demo_sanic` +
   `bevy`, nothing else) boots foundation + `PlatformerEnginePlugins::fixed_tick()`
   + `PlatformerHostPlugins` + the demo's content and rules, and steps the real
   sim. `SanicRulesPlugin::{hosted, global}` is the D-C constructor flag with its
   **first real rule** — a `spawn_mode_scoped` act timer on `WorldTime`, which the
   engine tears down when the room's mode changes. Three tests in
   `tests/exit_3.rs` pin it. Fable's "interactive build" ruling held for the FEEL
   and was overruled for the SHELL (vision §7; the case is in the commit message).
   **The shell draws nothing — an ENGINE gap, filed as OV1 below.**
   ✅ **And the same shape landed for `ambition_demo_smb1` the same day**: level
   1-1's grammar + `Smb1RulesPlugin` + `ambition_demo_smb1_app`. Two demos, two
   genres, two modes, ONE engine — and the second one needed no engine change at
   all, which is the E9 oracle's actual claim rather than its aspiration.

2. **The refactor chain — [engine/refactor-chain.md](engine/refactor-chain.md).**
   [opus] Six slices in dependency order, all verified, written 2026-07-10 after
   the ledger ruling: **R1** D-C mode-scope seam ✅ **DONE 2026-07-10** (closed
   the last decomposition artifact) → **R2** E6 teardown ✅ **DONE 2026-07-10**
   (the fused `gnu_ton` profile + the split-layer render are gone; GNU-ton is the
   ADR-0020 linked pair. **Its premise was WRONG and the doc now says so:**
   `boss_encounter/` is not a shell and did not shrink — 5456→5457 total src
   lines — so R2 does NOT unblock R4) → **R3** the overlay split ✅ **DONE
   2026-07-10** (`CollisionWorld` + `MovingPlatformSet` → `ambition_world::collision`;
   the spike found an unlisted `ambition_portal` dep, so `subtract_aabb` moved DOWN
   to `engine_core::geometry` rather than let the space IR name a mechanic) →
   **R4** ✅ **RE-CHECKED + STOPPED 2026-07-10** (fable's ruling held:
   `ProjectileCollisionWorld` came home to `ambition_projectiles`; victim routing
   still needs the boss cluster views R2 never settled, charge input still needs
   `BodyAnimFacts` — both blockers named, both deferred) → **R5** the `ControlFrame` allowlist
   lint ✅ **DONE 2026-07-10** (= unified-actors step 5's Phase C. Bidirectional +
   poison-tested. It found a FIFTH holder the re-count missed —
   `possession_trigger_system` reads the global frame through an import path, so
   possession is local-player-only; now the sole `Slot0Gesture` allowlist entry and
   an N1 checklist item) → **R6** the player fold + the `features/` rename — 🟡 R6a–R6d landed.
   **`player/` NO LONGER EXISTS**: body vocab → `actor`, control seam →
   `control`, affordance table → `affordances`, body mechanics → `features`, and
   what remained (correctly) became `avatar` — the home avatar slot 0 returns to.
   Slot-0 filters folded or justified in place. R5's lint went red on the move, as
   designed. **R6e (the `features/` rename) is PARKED with a DECISION BRIEF** in
   [refactor-chain.md](engine/refactor-chain.md) §R6e: measured, it is ~1560 sites
   across 5 crates (838 `Feature*` identifiers, not just 722 module paths), and
   renaming the module while leaving `FeatureId`/`FeatureSimEntity` behind would
   make the tree worse, not better. Jon picks: rename both (`sim` + `Sim*`), or
   accept the name now that `MODULES.md` explains it.

3. **The visible sprite bugs (E3/E6 tail).** [opus] Three player-facing
   regressions in the bug queue below: all bosses render the generic sheet,
   shrine + glider sprites are broken, morph ball still draws the robot.
   Diagnosis path is already written — do a RUN with `boss_sprites.len()`
   logging, and do NOT apply the disproven `sprite_target` dispatch. Ship the
   visual; never defer to "an interactive pass".

4. **CC3 — diagnostic rig LANDED; completion gate OPEN.** All six invariants
   and the minimum repro payload exist, and the shipped-room sweep produced a
   useful measurement. The comprehensive sweep remains ignored/diagnostic and
   does not assert the completion thresholds. Label it as evidence-producing
   instrumentation, not a completed guarantee, until the enforced gate is
   poison-tested and enabled.

5. **Bookkeeping:** the adapter-floor ruling and playbook exit 5 remain
   valid. **D-B is REOPENED.** `scripts/modules_md.py` provides useful concern-map
   drift checking, but the workspace has 44 members rather than the claimed 42,
   at least one generated map is stale, and several production/simulation modules
   exceed the documented ~1.5k-line standard. Add an executable line gate with a
   reviewed waiver list containing one reason per path; never infer a broad
   “generated/declarative” exemption.

**Deliberately NOT next:** CM6 and N1 (both land with the SSB demo, P4);
projectile steppers (blocked by design until their inputs are plain); the
S5/S6 player fold + `features/` rename (deferred until unified-actor work);
CC4 (profile first); CC7 P3a.

6. ~~**N0.1 — fixed-tick sim mode.**~~ **✅ DONE (opus, 2026-07-09).** The sim
   registers into a `SimSchedule` label (default `Update`, byte-parity);
   `PlatformerEnginePlugins::fixed_tick()` hosts it in `FixedUpdate` on
   `Time<Fixed>` at 60 Hz. `SimTick` is the canonical timeline;
   `ControlFrameLatch` is the device-owned frame→tick input latch. Exit check
   met both ways, plus a split-brain guard. **Deviation from the ruled
   plumbing (resource, not per-plugin field) argued in
   [engine/netcode.md](engine/netcode.md) N0.1**, along with three recorded
   remainders (presentation interpolation, `wall_dt` semantics, one-frame
   device latency). N0.2 and N0.4 are unblocked.

7. **N0.3 — ✅ LANDED (2026-07-10).** The four rules, ADR 0023, escape hatch, and
   poison tests now scan every simulation-bearing root: `crates/*` PLUS
   `game/ambition_content` and the `game/ambition_demo_{sanic,smb1}` rules crates
   (the `_app` shells and kaleidoscope menu stay out — device-frame display is
   their job). The widening immediately caught two real rule-3 violations in
   `falling_sand.rs` (std `HashSet`/`HashMap` iterated into `commands.spawn` order),
   now fixed. A self-check asserts each `game/` root is actually reached, so the
   widened scan cannot pass vacuously.

8. ~~**The dialog speaker-context slice.**~~ **✅ DONE (opus, 2026-07-09).**
   `$speaker_id` / `$listener_id` / `$speaker_is_self` published into Yarn
   variable storage at dispatch (the FIRST Yarn-variable write path in the
   codebase — everything else content reads is a library function over the state
   mirror). `<dialogue_id>__self` is the self branch; without one, self-talk is
   suppressed before the banner, the flags, the quest pump, and the mode flip.
   Identity is CHARACTER-first: the default player wears `player` and the Hall's
   player pedestal IS `player`, so `hall_player__self` — the mirror scene — is
   authored, and a content test guards that it must be.

9. ~~**N0.2 — the input-stream type.**~~ **✅ DONE (opus, 2026-07-09).**
   `engine_core::InputStream` (versioned, serde, per-tick `SlotControls` keyed by
   `SimTick`, contiguous, validated) + `runtime::InputStreamRecorder`, the one
   capture path. `SandboxSim::step_frame` drives raw `ControlFrame`s, so replay
   stops laundering the artifact through `AgentAction`. **It was more than
   promotion:** the old fixture is 60 ticks of NEUTRAL input, which only proves a
   falling body falls the same. The new `input_stream_replay` suite records a
   moving session, round-trips it through JSON, and replays it into a fresh sim
   with zero divergence. N0.4 is now a state-hash away.

## Track index (status → next slice)

| Track | Doc | Status | Next |
|---|---|---|---|
| Decomposition D-A | [engine/decomposition.md](engine/decomposition.md) | **COMPLETE** — E1–E9, W1–W4, F1–F9 executed; demo gate open; umbrella crate real; `placements` is the sole authored-entity channel; playbook exits 1–5 are met | shell-dissolution chain: [refactor-chain.md](engine/refactor-chain.md) |
| Decomposition D-B | same | **🔴 REOPENED (audit 2026-07-10)** — Series 1 landed the executable gate (`engine.module-size` policy in `tests/ambition_workspace_policy`, config `policies/module_size.toml`: code-line limit 1500 + bidirectional reasoned waiver list, poison-tested; migrated 2026-07-10 from the retired `crates/ambition_runtime/tests/module_size.rs`), regenerated the stale `MODULES.md`, and corrected 42→44. Criterion-4 line-size debt EFFECTIVELY CLEARED 2026-07-11 — all three over-limit non-declarative modules split: **`snapshot.rs` (3684)** → `snapshot/{mod,registry,restore,codecs}.rs` (1155/718/471/1379), 46 tests + rl_sim `desync_canary` green; **`moveset.rs` (1536)** → `moveset/{mod,prefabs}.rs` (862/691), 102 combat tests green (never waived — a pre-existing gate RED the "under the code-line limit" note had masked; the gate counts TOTAL lines); **`view_cones.rs` (2206)** → `view_cones.rs` render path (1145) + `view_cones/debug.rs` diagnostics (1078), 45 portal-presentation tests green. Each waiver deleted; **gate GREEN (28 policy checks)**. The ONLY remaining waiver is `menu/kaleidoscope_app.rs` (1814), a declarative Lunex node tree — the legitimate "generated/declarative" permanent-waiver class. Other D-B criteria (module maps, dissolved hub globs) already done. | D-B is effectively CLOSED on line-size: every over-limit module is now a justified declarative waiver, not deferred work. Formal re-close is a status call — either accept the one Lunex-tree waiver as permanent (recommended; it is data, not a god module) or split it too. No other decomposition line-size debt remains. |
| Decomposition D-C | same | **✅ DONE (2026-07-10)** — the mode-scope seam shipped as `refactor-chain.md` R1: `RoomMetadata.mode`, `ModeScopedEntity` + `spawn_mode_scoped`, `in_mode(name)` + `ModeScopePlugin`. Two hosted rulesets coexist (`ambition_runtime/tests/mode_scope.rs`); `sanic_speedway` claims its mode | — |
| Collision doctrine | [engine/collision-and-ccd.md](engine/collision-and-ccd.md) | CC1 + CC2 + CC5 + CC6 LANDED; **CC3 DIAGNOSTIC LANDED, ENFORCEMENT OPEN** — six invariants and replay payload exist, but the comprehensive sweep is diagnostic/ignored and does not enforce the completion thresholds | turn the measured oracle into a poison-tested gate when its policy is ready; CC4/CC7 remain |
| Combat stack | [engine/combat-model.md](engine/combat-model.md) | CM1–CM5 + CM7 LANDED — smash axes complete (growth, DI, charge, cancel tables, launch angles, per-move presentation). **A3 equipment→params LANDED (2026-07-11)** — `ambition_characters::equipment`: read-time modifier fold + behavioral grants + `OnHit::ConsumeAsArmor` in the one `resolve_body_hit`; three v1 deviations named in combat-model §8 | CM6 grab/throw/shield-stun (brings OnBlock) [opus, with SSB — a P4 slice, not a P2 exit] |
| Netcode ladder | [engine/netcode.md](engine/netcode.md) | N0.1/N0.2 LANDED. **N0.3 LANDED** (determinism scan reaches `game/ambition_content` + demo rules; caught 2 real `falling_sand` bugs; manifest scan un-vacuumed). **N0.4 LANDED as the canary mechanism** (required rooms hard-fail — a load failure or silent room-fallback panics, no vacuous skip; coverage ledger reacts to added debt). **N3.2 exact-restore substrate — substantial progress across THREE re-audit passes, still OPEN** (2026-07-10): identity invariant + mutation-free `validate_snapshot` (canonical order / registry-kind agreement / roster membership; order-robust dup detection); AUTHORITATIVE + HASHED identity `roster` (restore reconciles against it, so a zero-component `SimId` entity is preserved not despawned, and two worlds differing only by it no longer hash equal); active-room ownership + `CrossRoomBoundary` refusal comparing the full `Option` (a Some/None presence mismatch is refused) + active room folded into the hash; reconciliation-first stale accounting (H4); `DecodeFailed` transactional for standalone codecs, and every cursor/resolved codec now reports `ApplyOutcome` (unapplied rows counted) AND asserts it consumed the whole blob (`finish()` on the resolved + resource-cursor-absence paths, canonical `Reader::bool`, and `validate_snapshot` requiring exact registry entry order); resource cursors presence-tagged (absence ≠ empty cursor); self-measured reliable `lossless()` that denies unapplied rows + unresolved cursors + a spurious census; coverage ledger pinned by TYPE NAME against reviewed inventory files (not just a count); and an `UnsupportedDynamicReconstruction` refusal (ONE refusal, not a general window bound) — each with a red-before/green-after poison test. **`portal_lab` is a transition-spanning window, NOT a leak** (`central_hub_complex` authors `NpcSpawn-0017`; restore refuses the cross-room window). | REMAINING (N3.2, still OPEN): full atomic room-context restore (→ portal_lab CLEAN); per-spawner reconstruction recipes; then a narrower residual — cursor/resolved codecs still cannot be PREFLIGHTED standalone (a decode failure surfaces mid-reconciliation). **TWO residuals closed 2026-07-11:** (a) boss-hand `SimId::spawned` identity — `spawn_giant_hand_limbs` derives the hands' id from the giant's authored id (not `giant.index()`) and mints `SimId::spawned(giant, ordinal)` (`giant_hand_identity_tests`); (b) `SnapshotResolve::resolve` → `Result<Option, ResolveDecodeError>` — a truncated blob is `DecodeFailed`, distinct from authored absence's `Unapplied` (`a_truncated_resolved_blob_is_a_decode_failure_not_a_content_change`). Then N3.3 bevy_ggrs spike |
| Fighter brain | [engine/fighter-brain.md](engine/fighter-brain.md) | **FB1 DONE 2026-07-10** (§7) — the view now carries move phase, i-frames, damage meters, and stage geometry; `DelayedPerception` is the reaction-latency buffer. **Two bugs found: `half_extent` was filled with the FULL body size (2× everywhere), and peers' `on_ground`/`shield_raised` were hardcoded `false`** **FB3 DONE 2026-07-10** (§8): L1's `classify(&WorldView) -> Situation`, RANKED (Disadvantage outranks EdgeGuard — chasing an offstage foe while in hitstun is not edge-guarding, it is being carried), plus the 8-fixture scenario suite in the LIBRARY so FB4's rig scores the same situations. Three of L1's five states were underivable before FB1's audit **FB2 DONE 2026-07-10** (§9): L2's option generator + scorer. Attacks priced from CM7's `MoveFrameData`, so the brain understands a character nobody wrote a table for. **FOUND: none of §1's four features reads a move's POWER, so at any weights the jab beats the smash on a punish** — CM7 carries no damage either. Recorded, not patched: FB4's ladder is the doctrine's own instrument for forcing it **FB4a DONE 2026-07-10** (§10): the nine-rung ladder is content, and **the no-cheat contract is now a TYPE** — `Perceived` has a private field and only `DelayedPerception::perceive` mints one, so a brain layer cannot name the live world. A test can be forgotten and a grep lint argued with; a type cannot | FB4's remaining half (APM histogram + ladder self-play rig — both need a brain that emits inputs; the rig also calibrates L2's weights and will surface FB2's §9 hole), **FB5 DONE 2026-07-10** (§11): the opponent model — bounded (`Situation × Choice`, a 5×6 table), decayed (three fresh jumps outweigh nine stale shields), and honest (an unseen situation reads as the UNIFORM PRIOR, not zero: ignorance is not knowledge of absence). `BTreeMap`, not the sketch's `HashMap` — a trace and FB6's rollouts both iterate. Then FB6 (needs N3.1's `restore`) |
| Boss pipeline | [engine/boss-design.md](engine/boss-design.md) | BD1/BD4 LANDED; BD3 data + validator half LANDED; **BD5 VALIDATOR LANDED, ENFORCEMENT PENDING** — it reports eight hard errors and one warning, but roster installation does not reject them and the expected-error pin accepts the current debt | BD2, BD6, BD7 calibration; then make zero hard errors an install gate |
| Falling sand | [engine/falling-sand.md](engine/falling-sand.md) | **FS1 DONE 2026-07-10** (§3) — the reported defect was a SECOND representation: `emit_falling_sand_spouts` fed the CA grid *and* spawned parallel sprites that fell on their own gravity through every platform. Deleted. Conservation is now a tested `TallyLedger`; spouts are a table one `const` from the ruled `PlacementSchema::Spout` | FS2 (settle/level rules + fixed-point test — it needs the CA-stepping harness FS1's conservation audit also wants), then FS3 |
| S — Sanic | [demos/sanic.md](demos/sanic.md) | S1–S3 landed; **S5 shell DONE 2026-07-10** — `ambition_demo_sanic_app` boots engine+host+content+rules and steps the sim (playbook exit 3, gate-enforced); `SanicRulesPlugin` is D-C's first real consumer | the FEEL half is still interactive. **OV1 blocks the windowed half.** Ball-dash technique [opus] |
| M — Super Mary-O | [demos/super-mary-o.md](demos/super-mary-o.md) | **level 1-1 + the shell LANDED 2026-07-10.** `level_1_1()` authors the grammar (open teach → widening pit rhythm → stepping stone → stair pyramid → goal), pinned by a geometry test; `Smb1RulesPlugin::{hosted,global}` runs the mode-scoped level clock; `ambition_demo_smb1_app` boots + draws. **The E9 oracle held a SECOND time** — a different genre, zero engine edits | **M2 scroll knob LANDED 2026-07-10** — `CameraZoneSpec.scroll_policy` (`ForwardOnlyX`), applied after the bounds clamp, watermark on `CameraEaseState`, cleared on leaving so the clamp is PER-VISIT. Never eases backward to meet the watermark. Byte-parity for every pre-M2 zone. **The oracle-violation is filed and closed in the same breath: the knob is authored data, so no engine code names a demo.** **M3 flag sequence LANDED 2026-07-10** — `flag.rs`, zero engine code: `step_flag_sequence` is a pure `(state, pole, body, dt) -> Option<Vec2>`; the score is decided at the moment of CONTACT, and `FlagSequence::driven` holds the position so a gravity step between systems cannot move the slide. **Deviation stated out loud: NOT on the cutscene kit** — `CutsceneBeat` cannot move a body, and adding a beat that could would be engine code serving one demo, with a presentation crate's timing deciding a gameplay score. `goal_pole()` is the one source of the flag's geometry; the oracle that proves it agrees with the authored block caught a hardcoded tile size on its first run. **M1 DATA + MECHANISM LANDED 2026-07-11** — A3 shipped and `ambition_demo_smb1::powerups` authors `grow_cap`/`spark_blossom` through the umbrella (E9 oracle held a THIRD time, zero engine edits) | M1 tail (powerup PICKUP path + body-scale read-fold + level spawns/art), M4 the game, M5 hosting wing |
| F — Super Smash Siblings | [demos/super-smash-siblings.md](demos/super-smash-siblings.md) | gated on CM6 / N1 / FB | F1 rules crate |
| H — Hollow Lite | [demos/hollow-lite.md](demos/hollow-lite.md) | gated on BD pipeline | after BD7 pilot |
| Slower light | [engine/slower-light.md](engine/slower-light.md) | Tier-0 seams rode E4; L1–L4 in P5 | — |
| Docs refresh | — | P5; safe for [opus] once this stack is north star | mechanics/concepts/systems brought current |

Jon's open questions (Q1/Q2/Q3/Q5) live in [`roadmap.md`](roadmap.md).

## Drift findings (the plan vs. the measured code)

- ~~**The residual ledger is wrong.**~~ **RULED (Jon, 2026-07-10): the adapter
  floor IS the floor.** The alarm's number was right — `ambition_actors` is
  **64.0k total src lines** (units matter: TOTAL, incl. tests) against a
  projected 31–35k. But the gap is not one missing carve. Three measurements
  ([decomposition.md](engine/decomposition.md) THE LEDGER): (1) the crate has
  SHRUNK 4.2k since the F8 audit closed, so 64.0k is the true post-carve floor,
  not new code; (2) the missing ~30k is **nine adapter shells** between 0.8k and
  5.5k (`boss_encounter/` 5.5k is the biggest; `combat/` left cleanly at 0),
  each gated on a different technical precondition — there is no 25k carve in
  `features/`; (3) a further carve buys **no compile time** — touching a leaf in
  actors rebuilds the app in 104 s, touching `ambition_render` (which sits ABOVE
  actors) rebuilds it in 72 s, so the tower dominates and no carve of actors
  touches it. This confirms fable's own stated reason for a floor. The residual
  now shrinks by dissolving shells, sequenced in
  [engine/refactor-chain.md](engine/refactor-chain.md).
  **The units lesson (recorded, hard-won):** an opus pass tried to re-baseline
  this ledger in production-only lines, compared them against a total-lines
  projection, and concluded the alarm was a counting error — the opposite of the
  truth. Retracted. The durable finding is that `ambition_actors` is 43% test
  code (27.8k of 64.0k), useful when SCOPING a carve, not as the comparison.
  **State the units in every ledger.**
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
| ~~`SurfaceRamp` quarter-circle marker entities (Q27)~~ ✅ **LANDED 2026-07-10** | [spatial-model.md](engine/spatial-model.md) §SurfaceRamp — converter + the 4-case winding oracle, which rides each orientation under C4 gravity conjugation. **The winding is DERIVED, not tabulated** (one code path, four cases). **The oracle found a latent kernel bug: `advance_riding`'s joint nudge was a fixed `1e-4`, under one f32 ULP past s≈800, so a body froze ON a joint, still `Riding`, still carrying its velocity.** LDtk entity def + validator row land with the first level that authors a ramp | [opus] |
| Morph ball still draws the robot; generalize modal body morphs | **NARROWED 2026-07-10.** `sync_morph_ball_visual` is CORRECT and now gate-tested (3 tests in `ambition_render::rendering::morph_ball`): it shows the ball, hides the body, and restores `Inherited` — never a hard `Visible` — on exit. So the bug is NOT there. **Suspect 1 FIXED (blind):** `hit_flash`'s overlay is a separate ROOT entity, permanently `Visibility::Visible` (a deliberate `InheritedVisibility` workaround), textured with the SOURCE sprite's own image — so it never followed a hidden source. Get hit while balled up and the robot's silhouette painted right over the ball. `overlay_intensity` now returns 0 for a `Hidden` source: hiding a body must hide everything that draws it. Remaining suspects: a second entity drawing the body sprite, then a last-write-wins ordering. **The DESIGN defect is separate and still owed** (E3, `mode→sprite-state row`): a modal morph should select an animation row on the body's own sheet rather than hide the body and draw a bespoke sibling sprite — which deletes `morph_ball.rs` entirely | [opus] |
| Shrine + glider sprites broken | E3 (rect drift; sprite pipeline) | [opus] |
| ~~All bosses render the generic sheet~~ ✅ **FIXED 2026-07-10** | **The cause was arbitration, not wiring.** A boss is also an actor, so every boss id is in BOTH `ActorRenderIndex` and `BossRenderIndex`. `upgrade_actor_sprites` runs first: it resolved no character sheet for "Mockingbird", fell back to the **generic enemy sheet**, and inserted a `CharacterAnimator` — which `upgrade_boss_sprites` is filtered `Without<>`. Every boss was locked out of its own sheet forever. System order could not fix it (swapping just moves the overwrite) and `Without<BossAnimator>` could not either (the boss upgrader legitimately skips frames while its image loads). The read-model decides: `actor_sprite_path_owns(id, &boss_render)`. Found by running the REAL sim in the REAL boss rooms and reading both indices — `game/ambition_app/tests/boss_sheet_wiring.rs`, 5 tests. **Also found:** `sandbox.ldtk`'s `basement_boss` carried `PhaseScript:tri_slam_sweep_halo` (a pattern name, not a boss id), so `for_authored_boss` silently minted a generic clone. Placement fixed via `ldtk_tools entity set-field`; the fallback now WARNS. | [opus] |
| ~~Dialogs don't adapt to WHO is talking~~ **DONE 2026-07-09** | `ambition_dialog::{DialogueContext, DialogueNodeIndex}`; `interact_ecs_actors_and_switches` resolves both ids and suppresses trace-free. **Amendment to the pin:** identity is the CHARACTER id where a body has one (`InteractionKind::Npc.character_id`, or the home avatar's worn `StartingCharacter`), falling back to the placement id. The pin's literal "config.id vs target id" would make `$speaker_is_self` fire only when a possessed body interacts with its own placement — never at the Hall, which is the case that motivated the slice | — |
| ~~Sanic ball-dash special~~ ✅ **LANDED 2026-07-10** | [demos/sanic.md](demos/sanic.md) §Ball dash — content-side, zero engine additions; input reuses `locomotion.y`+`jump_pressed`, so it is gravity-relative for free. **Found two engine bugs holding each other up: the momentum kernel's airborne air control was MIRRORED, and a body running off a flat chain's open end hovered at the lip forever** | [opus] |
| Portal gun should be a normal item | portal exposes `spawn portal of pair P on surface`; one gun = one pair | [opus, low priority] |
| Build cache re-balloons | `$CARGO_TARGET_DIR/debug` hit 351G once. Consider `cargo-sweep` or a periodic prune | [Jon] |
| Smells journal (`dev/journals/code_smells.md`) | C4-style sweep rides each related track; the journal stays the intake | — |
| **DECISION BRIEF — exit_3 act-timer 59 vs 60 (Bevy-0.18 sync-point removal)** | `ambition_demo_sanic_app::exit_3::the_demos_own_rules...` expects the act clock to accumulate exactly 60 ticks over 60 updates but gets 59 (`0.98333`). ROOT CAUSE: `SanicRulesPlugin`'s rules chain is `(spawn_sanic_mode_owner, tick_sanic_act, …).chain()`; `spawn_sanic_mode_owner` spawns the mode owner via **deferred** `commands.spawn_mode_scoped`, and Bevy 0.18 removed automatic sync-point insertion — so the owner exists one tick AFTER `tick_sanic_act` first runs, missing tick 0. Not caused by the D-B splits or the InputPlugin fix (the `SimTick==119` test passes; tick counting is intact). OPTIONS: **(A, recommended)** insert an explicit sync point between the spawn and the tick (`(spawn, ApplyDeferred, tick, …)` — restores the test's "runs every tick" intent; note the codebase has no `ApplyDeferred` yet, so this sets the modern-Bevy pattern and other spawn→read-same-schedule chains may share the latent lag); **(B)** accept 59 as correct (the owner is genuinely born on tick 0 and counts from tick 1) and recalibrate the test with a comment. A demo-design call — parked per the decision-brief protocol, not guessed. | [opus / Jon] |

**The BLIND ledger (standing, Jon-only):** sanic area layout (`d620a230`),
sanic sheet/params, G3 limb arcs + G5 verb bindings (`a5d15247`), moveset
slash VFX placement (`05a32378`), swept-transit feel (`31342e6f`),
**hit-flash overlay no longer draws a hidden body** (`dbc6bd0b` — does the
robot still appear when a balled-up player takes a hit?), **Sanic's ball dash**
(`4e9f0ce2` — rev cadence, launch speed, ball size, when he stands back up).

## Oracle-violation log (demos file here; engine work exits through tracks)

~~**OV1 — a demo cannot DRAW its own world**~~ ✅ **CLOSED 2026-07-10, same day it
was filed.** `ambition_render::platformer_presentation::PlatformerPresentationPlugin`
is the engine's generic presentation face: the main `Camera2d` +
`MainCameraEntity`, the active room's static visuals at `Startup`, and the
sprite/animation chain. Room transitions already rebuilt through
`respawn_room_visuals_on_request`, which lived here all along.

**Everything the plugin needs already lived in `ambition_render`. What was missing
was a plugin that CALLED it** — `ambition_app` spawned the camera itself and
called `spawn_room_visuals` itself. That is all OV1 ever was.

`game/ambition_demo_sanic_app` now has two shells that differ by ONE plugin and
one foundation call: `build_demo_app()` (sim only, no renderer) and
`build_windowed_demo_app(RenderMode)` (`--features visible`). Three tests in
`tests/ov1_draws_the_world.rs` run the FULL render graph against no wgpu backend
and no window — the standard Bevy CI recipe — and assert the room visuals spawn,
the camera publishes itself, and **no `bevy_ui` node exists**: the face draws the
WORLD; Ambition's HUD, menus, and dev overlays stay app-side.

`cargo run -p ambition_demo_sanic_app --features visible --bin sanic_demo -- --window`
draws the speedway. What it looks like is Jon's to judge (BLIND).

**One thing the fix taught, worth keeping:** the presentation face genuinely needs
a renderer foundation and cannot run under `add_headless_foundation`. Splitting the
shells is not a workaround — a demo that only steps the sim should pay for no
renderer, and now it does not.

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
  attempted, RETRACTED, then re-argued from evidence and RULED by Jon**: the first
  attempt claimed the "63.5k" alarm was a counting error because 27.8k of it is
  test code — but the 2026-07-06 projection was itself in total lines, so it
  compared unlike numbers and reached the opposite of the truth. Re-measured: the
  crate has shrunk since the audit closed, the gap is nine adapter shells rather
  than one carve, and a further carve buys no compile time (the 104 s play loop is
  ≥72 s tower above actors). Ruling: the adapter floor IS the floor. Shells now
  dissolve one precondition at a time via `refactor-chain.md`.
- **opus** — **step 5's Phase C DEFINED** (it had no referent, and was silently
  gating the player fold): it is the `ControlFrame` allowlist lint. B3's claim
  ("only two `Res<ControlFrame>` holders") has drifted to four, unguarded. Write
  the lint before the fold.
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

## 2026-07-11
- **opus** — **D-B LINE-SIZE DEBT CLEARED — three module splits + a Bevy-0.18 gate fix.**
  All three over-limit non-declarative modules split (pure relocations + the minimum
  visibility widenings the split forces), module-size gate GREEN (28 checks), only the
  `kaleidoscope_app.rs` declarative waiver left:
  (1) `snapshot.rs` (3684) → `snapshot/{mod,registry,restore,codecs}.rs`
  (1155/718/471/1379), `8f96a21e` — 46 lib tests + rl_sim `desync_canary` green. The
  plan flagged the shared-`bc` alias; inspection found a second class it missed —
  cross-module privacy (a child sees a parent's privates, but not vice-versa nor across
  siblings), so `SnapshotRegistry.{entries,messages}`+2 consts went `pub(super)` and
  `MessageChannel` moved up to `mod.rs`.
  (2) `moveset.rs` (1536) → `moveset/{mod,prefabs}.rs` (862/691), `fb4db01e` — builders
  to `prefabs.rs`, runtime playback stays in `mod.rs`; 102 combat tests. Never waived —
  a pre-existing gate RED the "under the code-line limit" note masked (the gate counts
  TOTAL lines).
  (3) `view_cones.rs` (2206) → render path (1145) + `view_cones/debug.rs` (1078),
  `5c6b47e5` — the F1/F3 overlay + text/PNG dump machinery (the "no natural seam" the
  waiver claimed was there after all); 45 portal-presentation tests. `MODULES.md`
  regenerated; docs (decomposition §D-B, this row) updated.
  **The workspace `--features rl_sim` gate then surfaced a PRE-EXISTING Bevy-0.18
  failure** (unrelated to the splits — the sanic app references none of that code):
  `ambition_demo_sanic_app::exit_3` panicked on boot, `leafwing`'s `filter_captured_input`
  wanting a missing `ButtonInput<MouseButton>`. Fixed at the host layer (`9519aba2`):
  `HostInputBindingsPlugin` now adds `bevy::input::InputPlugin` when absent, so the host
  input group is self-sufficient headless (guarded → no-op under `DefaultPlugins`).
  exit_3 2/3 green; the third is a separate Bevy-0.18 residual — see the act-timer
  decision brief in the bug queue.
- **opus** — **A3 EQUIPMENT→PARAMS LANDED + M1 first slice.** The equipment→params
  mechanism the M-track was blocked on, in three bisectable commits:
  (1) the pure core `ambition_characters::equipment` — `EquipmentRow{modifiers,
  grants,on_hit}` on a per-body `WornEquipment`; `resolved_param` folds worn
  numeric modifiers at READ time (all Adds then all Muls, order-independent, never
  baked), scoped `Body` vs `Move`/`Verb`; `consume_armor` spends an armor row
  (remove or downgrade-in-place). (2) the sim wiring — `OnHit::ConsumeAsArmor`
  inside the ONE `resolve_body_hit` (shield beats armor beats damage; new
  `BodyHitResolution::Armored`; the player system threads its worn set, actor/boss
  pass `None`); `apply_equipment_grants` overlays `ActionSet` on equip so
  `build_actor_moveset` derives the granted move; `resolved_ranged` folds worn
  modifiers into a shot at `dispatch_move_events` (trigger-resolve). (3) M1 —
  `ambition_demo_smb1::powerups` authors `grow_cap` (size+armor) and
  `spark_blossom` (ranged grant + fire-time ×1.5 damage) through the umbrella,
  zero engine edits (E9 oracle held a third time). Home is `ambition_characters`
  (above engine_core, below combat/actors) so no engine_core edit was needed. All
  three exit-test assertions pinned headlessly (armor sequence, grant→verb-map,
  Mul-at-trigger-resolve). **Named remaining M1 wiring:** the powerup PICKUP path
  and the live BODY-scale collision/render read-fold (the resolver + fire folds
  landed; the body-size read did not). **Three v1 deviations** stated in
  combat-model §8 (size = numeric modifier not a component grant; grant-free
  downgrade rows; body-scale read-fold deferred).
- **opus** — **N3.2 boss-hand identity RESOLVED.** `spawn_giant_hand_limbs`
  derived each hand's `FeatureId` from `giant.index()` — an allocator slot, so two
  sims fed the same inputs gave the hands different `SimId`s (via `ensure_sim_id`'s
  `placement:` path), breaking snapshot/replay determinism. Now the id derives from
  the giant's AUTHORED id and the hand mints `SimId::spawned(giant_placement,
  ordinal)` at spawn (a deterministic spawned child, `placement:<giant_iid>/<ord>`);
  `ensure_sim_id` (`Without<SimId>`) skips it. Extracted `giant_hand_feature_id` as a
  pure, entity-free fn so the bug is structurally unreachable, and pinned it +
  the spawned-child derivation with `giant_hand_identity_tests`. The desync-canary
  ownership test's now-stale comments (which called this entity-index debt) were
  corrected. netcode.md + static-audit-response + tracks netcode-row updated.
- **opus** — **N3.2 resolved-codec `resolve → Result` RESOLVED.** `SnapshotResolve::`
  `resolve` returned `Option<Self>`, mapping BOTH a truncated/malformed blob and a
  legitimately-vanished authored half to `None` → `Unapplied`, so a corrupt wire input
  was silently laundered as a content change. Now returns `Result<Option<Self>,
  ResolveDecodeError>`: `Err` (a `Reader` primitive returned `None`) → `DecodeFailed`
  (aborts restore); `Ok(None)` (content gone) → `Unapplied` (denies `lossless()`);
  `Ok(Some)` → `Applied` after `finish()`. `resolve` now decodes the whole blob before
  the content lookup, so truncation is detected even when the content is present. Fully
  contained in `ambition_runtime` (only `MovePlayback` impls the trait). Poison test
  `a_truncated_resolved_blob_is_a_decode_failure_not_a_content_change` (red-before: the
  old `Option` reported `Unapplied`). Narrower residual named: cursor/resolved codecs
  still can't be preflighted standalone. netcode.md §4 + audit doc + tracks-row updated.
- **opus** — **D-B status corrected (Series 3 pass 1).** The D-B row claimed "nine
  modules remain waived debt (snapshot.rs 3744, moveset.rs 3022, …)". The actual
  `module_size.toml` waiver list is THREE (`snapshot.rs` 3684, `view_cones.rs` 2206,
  `kaleidoscope_app.rs` 1814). Row rewritten to the measured three. No code change — a
  status correction. **[Re-corrected 2026-07-11, same day:] this pass itself erred** —
  it claimed the gate was GREEN and `moveset.rs` (1536) was "under the code-line limit."
  The gate counts TOTAL lines (`s.lines().count()`) against 1500; there is no separate
  code-line count, so `moveset.rs` at 1536 is an UNWAIVED violation and the gate was
  RED, not green. Surfaced when the `snapshot.rs` split deleted its waiver and re-ran
  the gate.
