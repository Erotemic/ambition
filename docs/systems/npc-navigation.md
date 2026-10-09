---
status: current
last_verified: 2026-10-09
related_docs:
  - docs/planning/engine/platformer-navigation-and-reachability.md
  - docs/systems/room-looks.md
---

# NPC navigation

A body can go from one place in a room to another: it walks, jumps and drops
between the surfaces it can stand on. The first body that does this is the
companion dog in the basement of the central hub. Nothing in the system is
about the dog.

The design, its rules and its limits are in
[`platformer-navigation-and-reachability.md`](../planning/engine/platformer-navigation-and-reachability.md)
("The first slice"). This page is for an author.

## How a character gets it

Give the character a `Roam` brain preset in
`game/ambition_content/assets/data/character_catalog.ron`.

```ron
"companion_dog_roam": Roam(
    speed: 54.0,          // walk, px/s
    trot_speed: 105.0,    // to a point more than 160 px away
    rest_min_s: 1.2,      // the rest at each place
    rest_max_s: 4.0,
    company: 0.25,        // how often the next place is beside the player
    stay_within: 520.0,   // farther than this from the player: go to the player
    playful: 0.2,         // how often it runs four places with no rest
    notice_radius: 120.0, // at rest, it faces a player this near
),
```

The character needs the `Walk` and `Jump` abilities. A run-up and a jump are
at the body's top speed (`run_speed` in its `locomotion`).

## How to make a room that a body can navigate

A body goes only where its own jump reaches. The numbers are the body's, not
the player's. The dog (64 px tall, run speed 120) rises 89 px in a jump. The
default player body rises about 130 px.

- Put a step no more than about 70 % of the jump above the last one. For the
  dog that is 64 px (four tiles).
- A one-way platform above a floor is a step: the body jumps up through it.
- A solid block is a step when the body can stand next to its end.
- A body walks off an end to drop. It does not drop through a one-way platform.
- A hazard block takes its part of a floor out. The body does not stand there.
- A platform that moves is not a place to go.

The basement of the central hub has twelve one-way stones in 64 px steps for
this (`central_hub_basement` in `sandbox.ldtk`).

To paint a step:

```bash
cd tools/ambition_ldtk_tools
python -m ambition_ldtk_tools.edit.intgrid paint --level central_hub_basement \
    --px 560,864 --size 64,16 --value 2
```

## How to see what a body can reach

Photograph the room with the navigation overlay:

```bash
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex 950,1500 out.png 1500x1584 \
    --fit-room --warmup 900 --nav-overlay
```

The overlay draws the graph each navigating body is advised from: a green line
for each standing surface, a yellow arrow for each hop (take-off to landing),
an orange arrow for each drop, a magenta cross at the place a body is going
to, and a cyan cross at the place beside its target. A room with no navigating
body has no graph, so the overlay draws nothing there. The overlay is
`NavigationOverlay` in `game/ambition_app/src/dev/navigation_overlay.rs`; no
menu turns it on yet.

A test can read the graph: `RoomNavigation::graphs()` (a resource of the
session) gives each `NavGraph`, with its `surfaces`, its `links`, the body's
`apex_rise`, and `reachable_from(surface)`.

## Validation

```bash
cargo test -p ambition_platformer2d_world --lib navigation
cargo test -p ambition_characters --lib roam
cargo test -p ambition_app --features rl_sim --test app_it companion_dog
```
