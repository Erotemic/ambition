# HEAD status

Audited 2026-07-16 against the source tree underlying the current recon decision
record. This page states only active architectural work. Detailed historical
measurements and execution narratives are archived or remain in git history.

## Immediate architecture state

| Workstream | State at audit | Source evidence | What closes it |
|---|---|---|---|
| Placement lowering | **DONE.** Initial setup, reset, transition, restore, and LDtk reload all consume the App-installed registry through the registry-aware room-staging path; the local six-interpreter fallback is gone. | `crates/ambition_actors/src/session/setup.rs`; `crates/ambition_actors/src/session/reset/`; `crates/ambition_actors/src/world/rooms/load.rs`; `crates/ambition_runtime/src/snapshot/restore.rs` | — closed. |
| Provider lifecycle | **DONE.** `ambition_platformer_provider` owns one shared preparation/activation/session-build/cleanup lifecycle; `crates/ambition/src/provider.rs` is deleted and `ambition::provider` re-exports the new crate. Providers supply a session-world source and call `PlatformerExperienceAuthoring::install`; the four duplicated prepare/activate pairs and the per-provider marker generic are gone. | `crates/ambition_platformer_provider/`; `crates/ambition/src/lib.rs`; provider modules under `game/ambition_content` and the demo crates | — closed. |
| Session-root authority / N3.2 | **DONE.** `SceneEntities` removed (handles derive from canonical markers); `MovingPlatformSet` is registered snapshot state with session identity; `SessionTeardownPlugin` resets the resource mirrors. Both gates met (`session_isolation.rs`; the desync-canary restore oracle + focused reconstruction tests). | `ambition_actors::session::teardown`; `game/ambition_demo_sanic_app/tests/session_isolation.rs`; runtime snapshot codecs | — closed (residual boss/portal-room restore DIRTY-ness is pre-existing N3.1 debt, tracked separately). |
| Content ownership | **DONE at the campaign bar.** Dialogue voices, pirate-weapon presentation, projectile visuals, input techniques, held-item art, and the puppy-slug deep-dream pass (the last named render module — now `ambition_content::presentation` on the renderer's public `ActorOverlaySet` seam) are all provider-owned; item identities + boss sheets ruled already-met at their seams. Only the deliberately deferred engine-default asset families (`EntitySprite`/asset-universe) remain, with recorded rationale. | `docs/planning/tracks.md` §3 | — closed; reopen the deferred tail only when a provider actually differs. |
| Programmatic simulation | **DONE.** `crates/ambition_sim_harness` owns reset/step/typed-action/observation/`SandboxSim` below the demo gate; `SandboxSim::build(options, compose)` inverts the composition; the exit-gate test links only the `ambition` facade. `ambition_app::rl_sim` is a thin binding (feature-gated, optional dep). | `crates/ambition_sim_harness/tests/composes_below_the_app.rs` | — closed. |
| Boss action convergence | **DONE.** Execution, timing (the cycle windup/active clock deleted; the brain observes its live move), motion locks (`MoveWindow::motion_scale`, body-enforced), and semantic effects (telegraph cue/vfx as `MoveEvent`s) all ride the shared move lifecycle; defensive hurtboxes were already move-derived via the pose chain. What stays boss-owned is decision policy + encounter orchestration, by design. The boss-crate carve reassessment (maintainer decision #6) is now open for Jon. | `docs/planning/tracks.md` §5 | — closed; carve question handed to the maintainer. |
| Runtime/domain ownership | **REPAIRED at the decision-#9 bar.** `DevToolsSimPlugin` (+ public `DevEditApplySet`/`DevInspectorMirrorSet`), `DialogSimStatePlugin`, `EncounterRegistryPlugin`, and `map::MapStatePlugin` own their domains' sim state; the runtime chains order the sets. The full registration audit + kept/deferred classification is in tracks.md §6. | `crates/ambition_runtime/src/lib.rs` plugin group; `docs/planning/tracks.md` §6 | — closed; low-value first-plugin follow-ons recorded, not built. |
| Touch input | **DONE.** The semantic fold (raw touch state → `ControlFrame`) compiles with no Bevy and no render stack; every presentation dependency is optional behind the `mobile_touch` overlay feature. | `crates/ambition_touch_input/Cargo.toml` | — closed. |
| Render/read-model seam | **DONE at this track's bar.** `ambition_render` no longer depends on `ambition_combat` or `ambition_dialog` (feature taxonomy → primitives; `FeatureView` → sim_view; new `DialogView` row; `DialogChoiceSlot` → ui_nav). C4/C5/C6 resolved without code — rationale in tracks.md §8. | `docs/planning/tracks.md` §8 | — closed; the "only mutable sim facts" rule remains in force. |

## Current acceptance customers

- **Sanic:** momentum identity and a provider-owned playable character must work
  through the standard host/input/presentation path. See
  [`demos/sanic.md`](demos/sanic.md).
- **Super Mary-O:** equipment, body-scale consumption, classic platformer
  sequencing, and a complete headless level remain the P3 acceptance target.
  See [`demos/super-mary-o.md`](demos/super-mary-o.md).

## Deliberately deferred

- The final public name for the provider crate.
- Whether provider-owned placement families ever form a second channel beside the closed common Tier-0 schema.
- Menu-host extraction until a second real consumer draws the reusable boundary.
- Any boss crate carve until action convergence.
- A full `features/` rename; no partial rename.

Direct maintainer confidence belongs in
[`maintainer-decisions.md`](maintainer-decisions.md), not inferred from this
status summary.
