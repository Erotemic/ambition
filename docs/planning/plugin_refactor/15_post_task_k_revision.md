# Post-Task-K revision (2026-06-06)

Tasks A–K were executed in order (see git `d48f0505..f732153d`). Reaching Task K
required extracting the **cleanest minimal seam** — `ambition_platformer_runtime`
currently holds only `lifecycle` + `schedule`. That was the correct call under the
plan's rule "do not extract runtime until the proto-runtime is import-clean," but it
means the extracted crate is a **seed crate**, not yet a substantial runtime. Several
stages are therefore only partially satisfied.

This document amends `14_action_plan.md`: **before Stage 14 adapter-crate extraction,
do a runtime-substance pass.** Extracting adapter crates around a thin center would
create crates orbiting an unstable core.

## Honest status of the original tasks

| Task | Status | Notes |
| --- | --- | --- |
| A Documentation baseline | done | |
| B Architecture guardrails | done | needs the post-K backlog ratchet (below) |
| C Lifecycle proto-runtime | done | `6c7aa098` foundation |
| D Portal plugin shell | done | |
| E Generic helper extraction | **partial** | E1 (into sandbox proto-runtime) done; E2 (dependency-clean + into the crate) NOT done |
| F Mechanical portal split | done | `a010cf20` |
| G Semantic portal cleanup | done | `f9cf1e57` + PortalColor split `e9e0eb09` |
| H Ambition portal adapters | **partial** | H1 (item/inventory glue) done `8026209a`; H2 (ControlFrame + GroundItem) NOT done |
| I Portal feature gate | done | `ece185db` |
| J Content boundary | done | `43b5f3b0` |
| K First crate extraction | **seed only** | `35c99c8b` extracted lifecycle+schedule; runtime substance remains |

## What "complete A–K" requires (revised sequence)

```text
L. Post-K revision docs + extraction backlog        (this file + runtime_extraction_backlog.md)
M. Runtime substance pass — make the crate deserve its name:
     M1 pure transit math out of portal_pieces -> runtime; move transit.rs into the crate
     M2 generic world-query (SolidBlock / SolidWorldQuery / SurfaceHit / SurfaceFlags) +
        adapt engine_core::World; move collision raycast into the crate
     M3 generic body vocabulary (Body2d/BodyVelocity/BodyHalfExtents/BodyFacing/
        BodyTransitCooldown) + BodyRoll + roll-easing; decouple orientation.rs from
        PlayerKinematics/ActorKinematics/BossKinematics; move into the crate
N. Gravity completion — richer crate::mechanics::gravity (components/field/zones/switch),
     move GravityCtx, split GravityPresentationPlugin   (seed landed in f732153d)
O. PortalColor split — DONE (e9e0eb09): PortalGunColor + PortalChannelColor + PortalChannel
P. Portal render separation — portal/presentation.rs -> a portal_render module so portal
     core compiles without render-facing systems where possible
Q. Portal remaining adapter cleanup (H2) — remove ControlFrame + GroundItem from portal
     core via portal input-intent messages + a generic transitable body/item component
R. (DEFERRED) real mechanic/adapter crate extraction — only after M–Q
S. (DEFERRED) cleanup + ECS inventory comparison
```

Do **not** start Stage 14 / R until: the runtime owns body/world-query/transit vocabulary,
gravity is not owned by portal, portal presentation is separable from portal simulation,
and portal LDtk conversion sits behind a clear adapter boundary.
