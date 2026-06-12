# Stage 16 ECS-layer extraction — compact historical summary

**Status:** landed. `ambition_platformer_runtime` now owns the generic ECS runtime layer that this plan targeted.

## Durable decisions

- Grow `ambition_platformer_runtime` rather than creating many thin crates around an unstable vocabulary.
- Keep the runtime Bevy-native and ECS-native; do not recreate a parallel engine object.
- Use neutral runtime resources/components such as `SimDt` and `PrimaryBody` where game-specific state used to leak downward.
- Leave content-heavy feature/world/boss machinery in `ambition_sandbox` until their outward dependencies are inverted.

## Landed shape

The runtime crate owns the reusable body/world-query/gravity/orientation/transit/math vocabulary. The sandbox keeps facades/adapters where compatibility or game-specific glue still matters.

## Current follow-up

The extraction backlog is empty. New runtime extraction work should start from `22_monolith_breaker_survey.md` and the architecture-boundary tests, not from this historical stage plan.
