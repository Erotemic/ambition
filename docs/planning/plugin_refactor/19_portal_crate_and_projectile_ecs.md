# Stage 19 portal crate + projectile ECS — compact historical summary

**Status:** complete. The portal mechanic is now the standalone `ambition_portal` crate, and projectiles are entity-backed.

## Durable ownership boundary

`ambition_portal` owns the reusable mechanic:

- portal placement and channels;
- portal pairing/link semantics;
- transit math and body movement through portals;
- portal events and core policies.

Ambition-specific layers own:

- input and inventory translation;
- room reset policy;
- collision-world projection and carves;
- rendering/audio/VFX;
- named item/projectile semantics;
- LDtk/content adapters.

## What changed

- The old primary-player-specific portal paths were replaced by generic body/policy transit.
- Projectiles moved from Vec pools to ECS entities carrying reusable body/gameplay components.
- Portal carves and room reset behavior became adapter responsibilities.
- Portal tests now use live authored portal pairs instead of assuming color-pair runtime channels.

## Current authority

Use `../../systems/portals.md`, `../../mechanics/projectiles-and-motion-inputs.md`, and the `ambition_portal` / `ambition_platformer_runtime` crate docs for current behavior. This file is retained only for why the split happened.
