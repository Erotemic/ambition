# HEAD status

Audited 2026-07-18 against the current source tree; amended 2026-07-19 by the
deep review ([`../archive/reviews/deep-review-2026-07-19.md`](../archive/reviews/deep-review-2026-07-19.md))
and re-audited 2026-07-27 against source (three stale clauses corrected below).
Its completed rows are now checked by `scripts/check_roadmap_evidence.py`, which
verifies that a row claiming DONE cites something a machine can still find — it
cannot check a claim phrased as a SITUATION, which is exactly what the three
stale clauses were.
This page records the live state and current work; completed execution
narratives belong in git history or `docs/archive/`.

## Closed architecture campaign

The July 15–16 architecture campaign is complete at its stated bar:

- activation, reset, transition, restore, and LDtk reload share one App-installed
  placement-lowering authority;
- `ambition_platformer_provider` owns the typed provider preparation/activation
  lifecycle;
- `SceneEntities` is gone and sequential session teardown/activation is covered
  through the real host lifecycle;
- `ambition_sim_harness` owns the reusable reset/step/action/observation surface;
- the named content families selected for eviction now register through open,
  content-owned seams;
- boss attack execution, timing, motion locks, and effects converge on
  `MovePlayback` and moveset data;
- domain plugins own the repaired dev/dialog/encounter/menu state families;
- touch semantics compile without the presentation stack; and
- render consumes the repaired combat/dialog read-model seams.

These are foundations to preserve, not active decomposition tracks.

## Current hard work

| Workstream | Current state | What closes it |
|---|---|---|
| Portal camera continuity | **FIXED and PROVEN (2026-07-21).** The bisect and instrumentation were right that `BodyPoseView` was one frame stale, but the journal's "two clocks, structural" conclusion was wrong and is why this sat open: on the frame-stepped host both `FeatureViewSync` and `PresentedPoseSet` live in `Update`, and the presented-pose plugin never ordered its consumer set after the sim read-model producer. The fix adds the missing `FeatureViewSync -> PresentedPoseSet` edge only when the sim shares `Update` (the chain stays acyclic; nothing orders the producer after the consumer), plus a schedule-graph regression test. | Closed. `cargo test -p ambition_app --test app_it -- portal_translation_camera_continuity` → **3 passed** (was 2 FAILED). Poison-tested: gating the new `configure_sets` off reproduces both failures at the original 235.5px delta, so the edge is load-bearing rather than incidental. |
| GGRS correctness debt + effect quarantine | **QUARANTINE DONE 2026-07-21; residual debt OPEN.** External effects now defer to the confirmed-frame boundary instead of being suppressed on replay: audio, VFX, explosion/fireworks/debris requests, the autosave, and the forensic trace each have an explicit, tested policy (tracks #1). The deep review's claim that `gameplay_trace` was "quarantined correctly" was wrong — its gate meant FIRST PASS, not confirmed, so a mispredicted frame kept its guess permanently; rows are now frame-keyed and corrections replace predictions. ~~Still open from #0: the demo-content state composed into the app shell (`BallDash`, `SanicActState`, `MaryOLevelState`, `FlagSequence`) needs the content-side registration seam, and resources still need review by hand.~~ **BOTH CLOSED, corrected 2026-07-27:** all four register through the content-side seam (`require_rollback` / `rollback_component_clone` in each demo's own plugin) with behavioural restore proofs beside them, and resources are no longer reviewed by hand — `every_mutable_ambition_resource_is_registered_derived_or_waived` computes the population from a booted world every run, with a poison test proving the sweep catches an unregistered one. | tracks #0 (debt). #1's remaining clause is the Matchbox transport plus the two-peer predicted-A/corrected-B oracle, which are one piece of work — DEFERRED to the Super Smash Siblings era (Jon, 2026-07-24). |
| Encounter lifecycle convergence | **DONE (2026-07-16).** One command/lifecycle/objective authority (`EncounterLifecycle` + reducer + `EncounterCommand` ingress); ownership/policy-driven cleanup; `SimId::encounter` + snapshot-registered relations; consumers derive from lifecycle + staging policy; the Noether attunement is the shipped non-boss customer. E8–E13 all closed with exit tests in [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md). | — closed; residual boss-owned pieces (outro-gated persistence, reward anchors, adaptive music) recorded there as actor-local/authored policy. |
| GGRS rollback integration | **DONE for the simulation harness (2026-07-18).** `ggrs`/`bevy_ggrs` now own frame history, save/load requests, rollback entity recreation, entity remapping, resimulation, and sync-test checksum comparison. The custom `ambition_runtime::snapshot` engine, restore transaction, coverage debt ledgers, and compatibility facade are deleted. The real `SandboxSim` can run under `SyncTestSession`, exact prepared-content/schema identity invalidates active sessions, and representative actor/projectile/encounter churn is exercised through real GGRS loads. ADR 0027 records the replacement. | Production online boundary: ~~confirmed-frame quarantine for presentation/external effects~~ **DONE 2026-07-21 (tracks #1)**, then a Matchbox-backed two-peer handshake through the same `install_session` seam — which this work deliberately left untouched. That handshake is DEFERRED to the Super Smash Siblings era (Jon, 2026-07-24); no netplay work before a game needs it. |
| Immutable prepared content and exact session identity | **DONE (2026-07-18).** Provider preparation validates and deterministically assembles one immutable `PreparedContent`; canonical roots own the exact object, fingerprint/schema identity, and App-local epoch. The identity now binds GGRS session startup rather than an Ambition-owned snapshot format. LDtk replacement is rejected while a rollback session is active and requires a coordinated restart. ADRs 0026–0027 record the contract. | Closed. |
| Explicit provenance + planned construction | **DONE (2026-07-22); TRANSACTION SUBSTRATE DONE (2026-07-23).** Milestone B landed the content-free planner for three origin families. The 2026-07-23 checkpoints (A, C, "C step 2") then made the room transaction real at its outer boundary: publication + verification after ALL of construction; the authoritative roster and `RoomConstructionPlanId` derived from the completed plan (derived rows, relation payloads, recipe ids, content epoch); exact rig composition a fatal boundary check; reconstruction by stable `SimId` and relation closure; giant hosts+hands AND authored mount links migrated to plan rows with wired-and-verified `ambition.limb` / `ambition.mount` relations (`PendingMountLinks` deleted); runtime origins refuse giant specs rather than spawn handless. ~~Verification still DETECTS, not PREVENTS — no staging world yet.~~ **WRONG, corrected 2026-07-27 by reading the source.** `RoomConstructionPlan::prepare_from_parts` is fallible and every variant is detected BEFORE any live-room mutation; `apply_to_world` is infallible by construction. Plan → validate → commit is real for room construction, so verification does prevent. This clause would have sent somebody to build a staging world for a problem this path does not have — the expensive direction to be wrong in. The residue was two commands that could outlive their targets, now `try_`. **PHASE 4 LANDED (2026-07-23):** every authored family is a plan row, the outer roster is exactly `planned_ids()`, all five lifecycle paths share one transaction, stale content bindings are refused at the boundary (`ActiveContentBinding` + fatal `ContentBindingMismatch`). | **Phase 5 LANDED (2026-07-23):** coverage forcing functions, demo registration with behavioral restore proofs (which caught both demo mode owners registered-but-unanchored), and the track-0 exit oracle GREEN un-ignored — strike volumes joined the rollback envelope (`require_rollback::<Hitbox>` + clone/map registrations), the Combat chain now consumes moveset messages same-frame, the clock chain runs at the frame tail, and `PendingPlayerHitEvents` is lifecycle-voided + checksummed. **Phase 6 second slice LANDED:** Outlander launches and walks its ridge gate under a real routed shell, gated by `external consumer: outlander` in run_tests.py. NEXT: Phase 6 remainder (visible-shell half, task-7 measurements). |
| Super Mary-O acceptance | **LEVEL-1 GATE CLOSED 2026-07-21** (`d92791435`). The acceptance run plays the level: spawn → ?-block → milk through the real pickup/equipment path → pit A → secret pipe → vault → 8 coins → return pipe → surface → re-power → pits B and C → stair pyramid → pole → tally → a real replay back to spawn, no positional set-up, all three lives intact. Nothing previously proved any pit was crossable. Writing it found three bugs: the vault had no working exit (`cbc6902d2`), a body reset redefined the body (`4e4bd0fd8`, engine-wide), and pit B opens into the secret vault (reported). | Closed. Additional authored levels are now unblocked; see [`demos/super-mary-o.md`](demos/super-mary-o.md). |
| Sanic acceptance | **PARTIAL, movement/host seams proven; corrected 2026-07-19.** Persona, control chain, ball dash, transformation, lifecycle, route/momentum oracles, the ring economy, the badnik enemy loop, the on-screen ring tally through the provider-declared HUD seam, AND (2026-07-21) a restart that actually restarts — the act cycle's `RoomReplayRequested` had no consumer in this binary until the seam moved into `ambition_runtime`. | See the single-source remaining list in [`demos/sanic.md`](demos/sanic.md). |
| Fighter-brain L3 rollouts | **FB6a–d IMPLEMENTED 2026-07-30** per the §12 redesign ([`engine/fighter-brain.md`](engine/fighter-brain.md)): `brain::fighter::rollout` — a pure shadow model whose only world input is `Perceived`, exact `rollout_k × (1 + rollout_depth)` step budget as profile data, deterministic predicted opponent, no RNG. The hit-response kernel was CARVED to `ambition_engine_core::hit_response` (route 1) so `damage_apply` and the shadow model call ONE formula — `the_hit_response_is_the_authoritative_kernel_not_an_imitation` pins it. FB6a's `max_damage`/`max_knockback` + L2's `expected_payoff` closed §9's power gap (`the_smash_outbids_the_jab_on_a_punish_it_fits`). FB6e landed its determinism test, bench pin, and the real-sim fidelity instrument (`fb6_shadow_fidelity` in `app_it` — shadow prediction vs a real versus-stage swing at four gaps). | Ladder rows stay `rollout_depth: 0` until `l3_earns_its_depth` exists, which waits on **FB4b — the decision rig — specced as §13** (StateMachineCfg::Fighter; kit rides `BrainSnapshot.attack_kit`; APM at the one emission point; one snapshot-registered noise u64). §13 is the executable next step, [opus, fable-specced]. |
| Character authority — the preparation barrier | **PHASES A, B AND C DONE (2026-07-29).** `prepare_character` produces `PreparedCharacterOverrides`, a type declared with NO visibility modifier inside `character_runtime::definition` — so the seated projection, seating, and the worn path (all siblings of that module) physically cannot read a partial value, and the phase split is a compile error rather than a review convention. `CharacterPreparationPlugin::finish` folds the catalog in once for the whole cast; both construction paths read the complete `PreparedKit`. This closed H1, where a catalog-playable character with no authored action set worked as the worn player and got an EMPTY kit when seated as player two. Phase B deleted the second kit writer (the projection no longer writes persona kits — seated bodies always matched the single writer; the contrary diagnosis was false in three places), proved the seated `PreparedKit::HostCode` case end to end, landed the structural guard that a seated body matches every column the persona writer requires, and unified the remaining constructions (`build_host_code_moveset`, `GrantedBodyFacts`, `IdentityKit::of`). Phase C is done (H6 — bodies carry `{ character_id, generation }`, two records for two writers). ⚠ two things deliberately NOT closed: ids nothing REGISTERED still fold at wear time (the migration tail — no prepared value exists to disagree with), and `PreparedKit::HostCode` is a resolver that will never leave, because the host kit is built from each BODY's `AbilitySet`. | Remaining, per the plan's "Related, deliberately after": versus-match ACTIVATION (`PreparedMatchPlan`-shaped, queue H5) and the `RangedExecution` single switch. Plan: [`character-preparation-finalization-plan.md`](character-preparation-finalization-plan.md); ledger: [`queue-24h-2026-07-26.md`](queue-24h-2026-07-26.md) §H. |
| Local multiplayer topology | **MECHANISMS DONE, ACTIVATION OPEN (2026-07-29).** The GGRS session sizes itself from `LocalSeatTopology`, frozen once per gameplay session and released when it ends; every reconstruction — startup, hot reload, proof-pulse restore — reads the same frozen value rather than resampling live devices; the handle→device MAPPING comes from it too, not just the count; and the whole input-authority cluster (primary latch, seat latches, pending local and seat inputs) is replaced atomically on session rebase and stop. The versus roster records which topology decided its seat count and is rebuilt against a later freeze while it is still only an intention. | Match ACTIVATION — validate every participant, activate the roster atomically, publish it, start the countdown from that. Until it exists, a roster that has ALREADY seated and then disagrees with the session is reported rather than repaired, which is loud but not impossible. Ledger: [`queue-24h-2026-07-26.md`](queue-24h-2026-07-26.md) §Y′. |
| Public API 1.0 (SDK) | **CAMPAIGN CLOSED at §4's terminal condition (2026-07-30).** Slices A–G; ADRs 0031/0032 Accepted; allowlist ratchet 18 → 0; eight blind runs, the last (Script B run 8) opening zero engine files; six consumer-matrix categories each naming a test. Two standing reservations stay recorded at the top of the campaign doc: every matrix row is proven by a consumer authored in this repo, and the capability footprint (19 of 41 facade crates a movement-only game never asked for) never moved. | Findings carried out of the campaign, per [`engine/api-1.0-campaign.md`](engine/api-1.0-campaign.md) §Slice G: **(g)** seat-keyed input and query — `participants()` is the declaration and the stage's seating is an independent fact nothing reconciles, so a composition can declare four and seat two with no error, and no public seam drives input to a NAMED seat (couch-versus is not yet expressible through the SDK; related to the match-activation gap in the row above); **(a)** an accepted-but-inert rollback registration (`rollback_component_canonical` without `require_rollback`) should be unrepresentable, not documented; **slice H** — make the facade's 41 unconditional dependency edges optional and re-measure the closure. |
| Boss animator residue | **BOUNDED.** The execution/body path is converged; remaining residue is animation vocabulary/projection (`BossAnim`→`CharacterAnim`, obsolete target mirrors where still live). | Complete the bounded animator fold. Do not reopen the already-shared body integration path. |

## Rollback terminology

`ggrs` is the rollback driver; `bevy_ggrs` is the Bevy world snapshot adapter.
Ambition has no independent ephemeral snapshot/restore engine. `SimId` remains
semantic identity, while `RollbackId` is GGRS frame-history identity.

The old atomic room-staging restore campaign remains useful history because it
discovered the authoritative state and construction boundaries, but its runtime
implementation has been removed. Ordinary activation, transition, reset, and
hot reload still use canonical construction. GGRS rewinds the ECS world directly.

## Deferred

- The final public name for the provider crate.
- A provider-owned placement-family channel beside the closed common Tier-0 schema.
- Menu-host extraction until a second real consumer exists.
- The boss-crate carve decision. Convergence permits reassessment, but no
  concrete dependency/build/reuse boundary is currently documented; the
  maintainer ruling remains open.
- A full `features/` rename; no partial rename.

Direct maintainer confidence belongs in
[`maintainer-decisions.md`](maintainer-decisions.md), not inferred from this
status summary.

## Standing red instrument: `check_agent_kb.py` (2026-07-30)

`scripts/check_agent_kb.py` exits 1 at HEAD, and was already red before the API
campaign began (verified by running it at `c737ddfd6~1`) — the campaign's suite
gates did not include it. Two mechanical items were fixed in the 2026-07-30
review pass (the `SCRIPT.md` relative link; `## Current implications for
agents` sections for ADRs 0031/0032). What remains needs review, not typing:

- **35 source files carry a ≥200-line inline `#[cfg(test)]` module.** Reviewed
  2026-07-30 and marked below; see *Inline-test review* for what the review
  actually consisted of and what a maintainer still owes.
- **`AGENTS.md` is 227 lines against its 180 budget** (not the 193 this file
  claimed — that number was itself stale) — route content to docs rather than
  trimming meaning.

## Inline-test review (2026-07-30)

Every module in the marker block below was opened and its test surface read:
the fixtures it builds, the names of the cases, and what they reach for. **That
is the extent of the review — the test bodies were not audited assertion by
assertion**, and saying so is the point of `disposition=maintainer-review-pending`.
An agent may record a finding and a recommendation; only a maintainer grants a
permanent inline exception (`MAINTAINER_APPROVED_INLINE` in the checker).

**Finding: all 35 are `behavioral-local`, and the reason is uniform.** Each one
exercises real behaviour through private constructors, private fixtures, or
`super::` items that are not part of the crate's public surface —
`ActorClusterSeed`-style seed builders, `app_with(..)`/`spawn_seat(..)` harness
fns, private `resolve`/`prepare` helpers. Extracting them to `tests/` would
require widening visibility that the design deliberately keeps closed, which is
a worse trade than a long inline module. None is a policy check that happens to
live inline (the `guardrail` kind), which is the case extraction exists for.

⚠ **37 now.** `game/ambition_demo_smash/src/lib.rs` joined on 2026-07-31 as a
NEW crate rather than by growth — the stocks demo's tests build rosters, author
the stage and drive the rules plugin through the engine's own messages, all of
which reach `super::*` for private consts (`BLAST_MARGIN_PX`) and the plugin's
systems. `behavioral-local`, and a demo whose tests could be extracted to
`tests/` would be a demo asserting only its public surface, which is the half
that never breaks.

⚠ **36 before that.** `features/npcs.rs` crossed the threshold on 2026-07-31
when AD8 added the hit/provoked voice-floor case. Reviewed on the way past: its
module builds catalogs from RON literals and a `PreparedCharacterRegistry` by
hand, then asserts the bark precedence chain (catalog pool, then the definition's
voice floor, then the engine's generic text). `behavioral-local` for the usual
reason — `registry_with` and the private bark fns are not public surface — and a
threshold crossed by ADDING a case that closes a defect is the system working.

⚠ **the marker is a review record, not a green light.** Two modules are large
enough that their SIZE is a finding on its own, independent of placement:
`game/ambition_demo_mary_o/src/lib.rs` (2896 test lines) and
`crates/ambition_actors/src/features/enemies/mod.rs` (1039). Those are the two
worth a maintainer's attention first.

<!-- planning-evidence: inline-test path=crates/ambition_actors/src/action_scheme.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_actors/src/features/ecs/autonomous_reconcile.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_actors/src/features/npcs.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_actors/src/features/enemies/mod.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_actors/src/world/rooms/stage.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_audio/src/catalog.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_audio/src/selection.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_characters/src/action_scheme.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_characters/src/actor/character_catalog/binding.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_characters/src/actor/character_catalog/mod.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_characters/src/actor/control.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_characters/src/equipment.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_encounter/src/lifecycle.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_encounter/src/waves.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_game_shell/src/basic_presentation.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_game_shell/src/pause_menu.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_input/src/local_seats.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_ldtk_map/src/conversion/mod.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_persistence/src/save.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_persistence/src/save_data.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer_provider/src/lifecycle.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_render/src/hud.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_render/src/rendering/label_layout.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_runtime/src/rollback/probes.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_runtime/src/rollback/session.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_sim_view/src/camera_snapshot.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_sim_view/src/control_prompt.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_sim_view/src/presented_pose.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_touch_input/src/bevy_plugin.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_touch_input/src/placement.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_app/src/app/versus.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_content/src/falling_sand_sim/sand_grid.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_content/src/presentation/dialog.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_demo_smash/src/lib.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_demo_mary_o/src/flag.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_demo_mary_o/src/lib.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_demo_mary_o/src/powerups.rs kind=behavioral-local disposition=maintainer-review-pending -->

## Mechanically recomputed evidence

These markers are cross-checked against live computation by
`scripts/check_agent_kb.py`; they exist so a claim in this file cannot quietly
drift from the tree. Regenerate by running that script and correcting the values
it reports. (Restored 2026-07-19: the 07-18 rewrite dropped them, which left the
KB check red.)

<!-- planning-evidence: boss-validator errors=8 warnings=10 -->
<!-- planning-evidence: workspace-members count=53 -->
<!-- planning-evidence: module-size waivers=0 unwaived-violations=0 stale-waivers=0 invalid-waivers=0 -->
<!-- planning-evidence: cc3 status=ignored -->
