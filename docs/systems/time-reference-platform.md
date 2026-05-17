# Time Reference Platform

The sandbox includes a simple horizontal moving platform as a visual metronome
for time-scale tuning.

For this pass, the platform is intentionally **not yet part of collision**. It
is sandbox presentation/state only, not an engine moving-solid primitive. Its
purpose is to make bullet-time obvious: when precision blink aims, the platform
should crawl almost to a halt along with the player simulation.

Future engine work should promote this into a real moving platform primitive
once we decide how Ambition handles:

- carrying the player;
- pushing and crushing;
- collision order against kinematic actors;
- blink pathing through moving solids;
- deterministic tests for moving-platform contacts.

Current time-scale tuning:

- blink hold slow scale: `0.35`
- precision blink bullet-time scale: `0.10`

The precision scale is intentionally near-frozen. Because the blink destination
cursor is currently updated inside the engine simulation step, its aim speed is
compensated upward. A cleaner later refactor would separate **world simulation
dt** from **aim/user-interface dt**, so precision aim can remain responsive even
when the world is nearly stopped.

## Rideable collision experiment

The moving platform is now inserted into a temporary collision-world clone each
frame so the player can collide with it and ride it. The platform is still a
sandbox experiment rather than a permanent `ambition_engine` moving-solid
primitive. The next engine-level step is to add tests for carrying, pushing,
crushing, one-way moving platforms, and blink interaction.
