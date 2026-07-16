# HEAD status

Audited 2026-07-16 against the source tree underlying the current recon decision
record. This page states only active architectural work. Detailed historical
measurements and execution narratives are archived or remain in git history.

## Immediate architecture state

| Workstream | State at audit | Source evidence | What closes it |
|---|---|---|---|
| Placement lowering | **OPEN — correctness fork.** Initial session setup and reset construct a local six-interpreter registry, while room transition and restore consume the App-installed registry. | `crates/ambition_actors/src/session/setup.rs`; `crates/ambition_actors/src/session/reset/`; `crates/ambition_actors/src/world/rooms/load.rs`; `crates/ambition_runtime/src/snapshot/restore.rs` | Activation, reset, transition, and restore all call one registry-aware lowering path; the production local-registry helper is deleted. |
| Provider lifecycle | **DONE.** `ambition_platformer_provider` owns one shared preparation/activation/session-build/cleanup lifecycle; `crates/ambition/src/provider.rs` is deleted and `ambition::provider` re-exports the new crate. Providers supply a session-world source and call `PlatformerExperienceAuthoring::install`; the four duplicated prepare/activate pairs and the per-provider marker generic are gone. | `crates/ambition_platformer_provider/`; `crates/ambition/src/lib.rs`; provider modules under `game/ambition_content` and the demo crates | — closed. |
| Session-root authority / N3.2 | **PARTIAL.** Exact session-root state exists, but ambient process resources and reconstruction asymmetries remain. | `PlatformerSessionWorld`; `SceneEntities`; `MovingPlatformSet`; runtime snapshot restore | Both gates pass: a leak-free sequential second session/provider switch, and exact reset/restore reconstruction parity through the same authorities. |
| Content ownership | **OPEN.** Named provider content remains compiled into reusable crates in item, asset/art, projectile, input-technique, audio-cast, and render paths. | Examples recorded in [`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md) and the archived recon | Each eviction ends in provider-owned registration/catalog/presentation structure; no relocated closed table. |
| Programmatic simulation | **OPEN — extraction candidate.** The useful reset/step/action/observation surface remains under `ambition_app`. | `game/ambition_app/src/rl_sim/` | `ambition_sim_harness` composes arbitrary provider/runtime plugins without depending on the flagship app. |
| Boss action convergence | **PARTIAL.** Ordinary melee has converged; boss policy and remaining special paths must use the canonical moveset/action authority before any crate decision. | `ambition_combat::moveset`; actor boss-pattern and boss-encounter modules | Boss selection may remain specialized, but execution uses the shared action lifecycle and obsolete boss-specific paths are deleted. |
| Runtime/domain ownership | **PARTIAL — drift repair.** Runtime correctly owns global order but still initializes or names some domain-local resources and leaf systems. | `crates/ambition_runtime/src/*_schedule.rs`; known dev-tools registration example in the decision record | Domain plugins own local resources/systems/sets; runtime orders the public sets and retains only true cross-domain orchestration. |
| Touch input | **OPEN — real split.** Pure touch folding and visual touch controls share one crate/dependency envelope. | `crates/ambition_touch_input` | A headless semantic-input layer is separable from the presentation overlay. |
| Render/read-model seam | **PARTIAL.** The main one-way observation boundary exists, with a small set of direct live-component and unnecessary dependency residue. | `ambition_sim_view`; `ambition_render` | Remove dead deps and migrate only high-value mutable sim facts; immutable authored world IR may remain a direct presentation input. |

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
