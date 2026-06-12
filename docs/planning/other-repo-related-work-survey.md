# Related-work shortlist for Ambition

Reviewed: 2026-06-06. Condensed during docs cleanup so this file stays a shortlist, not a catalog.

Use this page to choose comparison/prototype work. Do not adopt crates just because they appear here; check Bevy version, maintenance, feature gates, runtime/adapter split, compile-time cost, and migration risk first.

## Highest-value investigations

| Candidate | Why it matters | Suggested next action |
|---|---|---|
| `bevy_asset_loader` | Already in the graph; may replace more compiled asset registries with dynamic asset collections. | Prototype one content pack manifest for boss or portal assets and compare against current `GameAssets` wiring. |
| `bevy_proto` | Good model for data-authored entity prototypes, even if the crate version is not adopted. | Prototype a low-risk entity class such as pickup/chest/NPC. |
| `bevy_save` | Reflection/migration ideas may help checkpoint/save evolution. | Borrow concepts before replacing current game-specific save code. |
| `bevy-tnua` | Useful prior art for floating platformer controllers and Avian integration. | Compare architecture, not tuning. Keep Ambition controller deterministic/readable. |
| `vleue_navigator` | Possible macro-navigation for NPCs or room-level route planning. | Prototype only if a specific NPC/pathing task needs it. |
| `dogoap` / `big-brain` | Prior art for GOAP/utility AI and componentized action/scorer/planner shapes. | Compare against the current brain/action pipeline before adding bespoke AI layers. |
| `Lightyear`, `bevy_replicon`, `bevy_quinnet` | Networking/rollback choices affect runtime assumptions even if multiplayer is deferred. | Use as architectural constraints; do not adopt until multiplayer goals are explicit. |
| `scrcpy-mask`, `virtual_joystick` | Mobile input mask/profile and Android test harness ideas. | Borrow UX/test patterns, not core dependencies. |
| `bevy_tween`, `bevy_easings`, `smooth-bevy-cameras` | Could replace or inspire custom camera/easing/transition code. | Only behind presentation feature gates. |
| Bones | Strong reference architecture: renderer-agnostic core, deterministic ECS, snapshot/restore, modding/scripting, Bevy renderer adapter. | Study as architecture, not as an immediate dependency. |

## Keep but clarify ownership

- `bevy_ecs_ldtk`: keep as LDtk adapter; keep Ambition-specific conversion in content/adapter layers.
- `avian2d` / `parry2d`: keep evaluating as low-level geometry/physics tools, not a full player-controller replacement.
- `leafwing-input-manager`: keep for physical input mapping; Ambition semantic input profiles sit above it.
- `bevy_kira_audio`, Yarn Spinner / `bevy_yarnspinner`: keep as adapters; do not leak them into headless runtime or content validation builds.

## Watch only

- `dip`, `bevy_retrograde`, `bevy_mod_scripting`, 2D lighting crates, and networking crates are useful references but not current adoption targets.

## Current dependency snapshot

| Area | Current dependency | Review point |
|---|---|---|
| Engine | `bevy` | Keep feature-gated dependency sets explicit. |
| Physics/collision | `avian2d`, `parry2d` | Compare with custom platformer solver and `bevy-tnua`. |
| LDtk | `bevy_ecs_ldtk` | Keep adapter-specific code out of reusable runtime. |
| Assets/data | `bevy_asset_loader`, `bevy_common_assets`, custom asset manager | Prefer manifest/data keys over compiled rosters where possible. |
| Audio/dialogue | `bevy_kira_audio`, Yarn Spinner | Adapter layers only. |
| Input | `leafwing-input-manager`, `virtual_joystick` | Map physical devices to semantic intents. |
| UI/dev/debug | vendored UI crates, inspector crates | Keep optional and app/presentation-scoped. |
| Testing | `insta`, `proptest` | Keep; add boundary/inventory tests when architecture changes. |

## Deprecated detail

The original version of this document included long per-crate notes. Those details age quickly. Re-run a focused web/code review before making an adoption decision.
