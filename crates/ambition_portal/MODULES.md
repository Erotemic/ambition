# `ambition_portal` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_portal** — Reusable, content-free portal mechanic.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`color`](src/color.rs) | Portal pair-linking identity. |
| [`eviction`](src/eviction.rs) | Straddle eviction — the ONE sanctioned pushout. |
| [`gun`](src/gun.rs) | Compatibility state for Ambition's held portal gun. |
| [`gun_lifecycle`](src/gun_lifecycle.rs) | Compatibility lifecycle for gun-owned portals. |
| [`gun_pickup`](src/gun_pickup.rs) | Compatibility pickup for Ambition's portal-gun workflow. |
| [`gun_projectile`](src/gun_projectile.rs) | Compatibility projectile for portal-gun-style placement. |
| [`lifecycle`](src/lifecycle.rs) | Portal lifecycle / persistence policy for placed portals and transit cooldowns. |
| [`link`](src/link.rs) | Explicit portal **linking by id**, plus the min-aperture equalizer. |
| [`messages`](src/messages.rs) | Reusable portal intent / outcome messages. |
| [`pieces`](src/pieces.rs) | Pure portal-piece geometry — the **Core invariant** of the portal system. |
| [`placement`](src/placement.rs) | Portal-aware geometry and the surface-fit / aperture-crossing decision logic. |
| [`plugin`](src/plugin.rs) | Portal mechanic plugin assembly: the public [`PortalPlugin`] hosts install, and the [`PortalSimulationPlugin`] it delegates to (registers the portal messages, resources, and simulation systems against [`PortalSet`](crate::PortalSet)). |
| [`schedule`](src/schedule.rs) | The portal-owned [`PortalSet`] schedule labels (carves, input warp, weapon, transit, room-reset ordering). |
| [`transit`](src/transit.rs) | Portal-specific transit systems: drive opted-in actors and in-flight items through a placed portal pair via the shared [`super::placement::transit_step`] aperture machine, plus the carve / input / ability-suppression guards that make a crossing feel right. |
| [`tuning`](src/tuning.rs) | Runtime-tunable portal feel and convention policy. |
| [`types`](src/types.rs) | Shared portal types, geometry constants, and small helpers used across the portal submodules (placement, transit, presentation, …). |
| [`view`](src/view.rs) | Pure through-portal **view** geometry — what a viewer looking into one portal sees of the world at its partner. |

_17 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
