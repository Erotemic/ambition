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
- `ambition_platformer2d_provider` owns the typed provider preparation/activation
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
| GGRS correctness debt + effect quarantine | **QUARANTINE DONE 2026-07-21; residual debt OPEN.** External effects now defer to the confirmed-frame boundary instead of being suppressed on replay: audio, VFX, explosion/fireworks/debris requests, the autosave, and the forensic trace each have an explicit, tested policy (tracks #1). The deep review's claim that `gameplay_trace` was "quarantined correctly" was wrong — its gate meant FIRST PASS, not confirmed, so a mispredicted frame kept its guess permanently; rows are now frame-keyed and corrections replace predictions. ~~Still open from #0: the demo-content state composed into the app shell (`BallDash`, `SanicActState`, `MaryOLevelState`, `FlagSequence`) needs the content-side registration seam, and resources still need review by hand.~~ **BOTH CLOSED, corrected 2026-07-27:** all four register through the content-side seam (`require_rollback` / `rollback_component_clone` in each demo's own plugin) with behavioural restore proofs beside them, and resources are no longer reviewed by hand — `every_mutable_ambition_resource_is_registered_derived_or_waived` computes the population from a booted world every run, with a poison test proving the sweep catches an unregistered one. | tracks #0 (debt). #1's remaining clause is the Matchbox transport plus the two-peer predicted-A/corrected-B oracle, which are one piece of work — DEFERRED to the Super Smash Siblings era (Jon, 2026-07-24). |
| Sanic acceptance | **PARTIAL, movement/host seams proven; corrected 2026-07-19.** Persona, control chain, ball dash, transformation, lifecycle, route/momentum oracles, the ring economy, the badnik enemy loop, the on-screen ring tally through the provider-declared HUD seam, AND (2026-07-21) a restart that actually restarts — the act cycle's `RoomReplayRequested` had no consumer in this binary until the seam moved into `ambition_platformer2d_runtime`. | See the single-source remaining list in [`demos/sanic.md`](demos/sanic.md). |
| Fighter-brain L3 rollouts | **FB6a–d IMPLEMENTED 2026-07-30** per the §12 redesign ([`engine/fighter-brain.md`](engine/fighter-brain.md)): `brain::fighter::rollout` — a pure shadow model whose only world input is `Perceived`, exact `rollout_k × (1 + rollout_depth)` step budget as profile data, deterministic predicted opponent, no RNG. The hit-response kernel was CARVED to `ambition_platformer2d_core::hit_response` (route 1) so `damage_apply` and the shadow model call ONE formula — `the_hit_response_is_the_authoritative_kernel_not_an_imitation` pins it. FB6a's `max_damage`/`max_knockback` + L2's `expected_payoff` closed §9's power gap (`the_smash_outbids_the_jab_on_a_punish_it_fits`). FB6e landed its determinism test, bench pin, and the real-sim fidelity instrument (`fb6_shadow_fidelity` in `app_it` — shadow prediction vs a real versus-stage swing at four gaps). | Ladder rows stay `rollout_depth: 0` until `l3_earns_its_depth` exists, which waits on **FB4b — the decision rig — specced as §13** (StateMachineCfg::Fighter; kit rides `BrainSnapshot.attack_kit`; APM at the one emission point; one snapshot-registered noise u64). §13 is the executable next step, [opus, fable-specced]. |
| Local multiplayer topology | **MECHANISMS DONE, ACTIVATION OPEN (2026-07-29).** The GGRS session sizes itself from `LocalSeatTopology`, frozen once per gameplay session and released when it ends; every reconstruction — startup, hot reload, proof-pulse restore — reads the same frozen value rather than resampling live devices; the handle→device MAPPING comes from it too, not just the count; and the whole input-authority cluster (primary latch, seat latches, pending local and seat inputs) is replaced atomically on session rebase and stop. The versus roster records which topology decided its seat count and is rebuilt against a later freeze while it is still only an intention. | Match ACTIVATION — validate every participant, activate the roster atomically, publish it, start the countdown from that. Until it exists, a roster that has ALREADY seated and then disagrees with the session is reported rather than repaired, which is loud but not impossible. Ledger: [`queue-24h-2026-07-26.md`](queue-24h-2026-07-26.md) §Y′. |
| Boss animator residue | **BOUNDED.** The execution/body path is converged; remaining residue is animation vocabulary/projection (`BossAnim`→`CharacterAnim`, obsolete target mirrors where still live). | Complete the bounded animator fold. Do not reopen the already-shared body integration path. |

## Closed, kept as one line each

These were multi-paragraph DONE rows in the table above until 2026-08-03. A
HEAD status should show what is OPEN; a finished campaign needs one sentence and
a citation, and its narrative lives in git history and `docs/archive/`.

- **Portal camera continuity** — the missing `FeatureViewSync → PresentedPoseSet` edge when the sim shares `Update`, with a schedule-graph regression test; poison-tested. (2026-07-21)
- **Encounter lifecycle convergence** — one command/lifecycle/objective authority (`EncounterLifecycle` + reducer + `EncounterCommand` ingress), exit tests in [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md). Residual boss pieces recorded there as actor-local policy. (2026-07-16)
- **GGRS rollback integration** — `bevy_ggrs` owns frame history, save/load and rollback entity identity for the simulation harness. (2026-07-18)
- **Immutable prepared content + exact session identity** — provider preparation deterministically assembles one immutable `PreparedContent`. (2026-07-18)
- **Explicit provenance + planned construction** — the content-free planner (`RoomConstructionPlanId`) plus the transaction substrate. (2026-07-22 / 07-23)
- **Super Mary-O level-1 acceptance gate** — the run plays spawn → ?-block → milk through the real pipeline (`d92791435`). (2026-07-21)
- **Character authority — the preparation barrier** — `prepare_character` produces `PreparedCharacterOverrides` at one barrier; phases A–C. (2026-07-29)
- **Public API 1.0 (SDK)** — closed at §4's terminal condition; ADRs 0031/0032 Accepted, allowlist ratchet 18 → 0. See [`engine/api-1.0-campaign.md`](engine/api-1.0-campaign.md). (2026-07-30)

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
`crates/ambition_platformer2d_actor_monolith/src/features/enemies/mod.rs` (1039). Those are the two
worth a maintainer's attention first.

⚠ **Three joined the list on 2026-08-04 and TWO of them are the agent's own
doing**, recorded rather than quietly marked: `player_robot_lineage.rs` went
141 → 277 test lines when D14's sprite-body assertions landed, and
`character_sprites/assets.rs` sits at exactly 200 because one fixture field was
added to it. `features/transform_beat.rs` (211) crossed on its own. All three are
`behavioral-local` and none is extractable without widening a private API, which
`test-placement.md` forbids — so they are review-pending, which is the disposition
an agent is allowed to record.

<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/features/transform_beat.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=game/ambition_content/src/player_robot_lineage.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d/src/app.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/action_scheme.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/features/ecs/autonomous_reconcile.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/features/npcs.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/features/enemies/mod.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_actor_monolith/src/world/rooms/stage.rs kind=behavioral-local disposition=maintainer-review-pending -->
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
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_ldtk/src/conversion/mod.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_persistence/src/save.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_persistence/src/save_data.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_provider/src/lifecycle.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_render/src/hud.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_render/src/rendering/label_layout.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_runtime/src/rollback/probes.rs kind=behavioral-local disposition=maintainer-review-pending -->
<!-- planning-evidence: inline-test path=crates/ambition_platformer2d_runtime/src/rollback/session.rs kind=behavioral-local disposition=maintainer-review-pending -->
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
<!-- planning-evidence: workspace-members count=59 -->
<!-- planning-evidence: module-size waivers=0 unwaived-violations=0 stale-waivers=0 invalid-waivers=0 -->
<!-- planning-evidence: cc3 status=ignored -->
