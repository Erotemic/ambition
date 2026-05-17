# Feature basement wave

This patch turns the Patch 2 taxonomy into a playable basement test wing.  It is still a sandbox/proving-ground implementation, not the final production version of each mechanic.

The central hub now has a basement loading zone.  The basement hub fans out into focused rooms for hazards, enemies, boss patterns, breakables, treasure/pickups, and NPC/dialogue hooks.  Loading zones render their destination names as world-space debug labels so transition authoring can be checked without relying only on the HUD.

Sandbox feature objects now spawn as ECS entities/components. The feature systems support moving damage volumes for spike balls and saw sweeps, simple enemy patrol/guard behavior with short-range attacks, a large boss placeholder that cycles slam/sweep/halo hit patterns, breakables that can either respawn or remain broken, pickups, chests, and NPC interactables. Breakable platforms marked `solid: true` contribute temporary solid blocks to the collision world while intact, so movement, blink preview, and collision share the same geometry view.

This is deliberately a conservative feature wave.  The reusable engine vocabulary remains the source of truth; the sandbox behavior is an adapter that makes those objects visible and testable before adopting heavier systems such as `seldom_state` or `bevy_yarnspinner`.
