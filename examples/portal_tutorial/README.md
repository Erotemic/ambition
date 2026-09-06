# Portal tutorial

This example is a deliberately small Bevy host for Ambition's reusable portal crates.
It uses the actual `ambition_portal2d` simulation and `ambition_portal2d_presentation`
renderer without loading LDtk, the portal gun, Ambition content, or the full game app.

Run it from the repository root:

```bash
cargo run --manifest-path examples/portal_tutorial/Cargo.toml
```

## What the host supplies

The reusable crates do not own a game loop. This example supplies the four host seams:

1. `SimDt`: mirrors Bevy frame time into the portal simulation clock.
2. `BodyKinematics`: a tiny motion system advances one square.
3. `PlacedPortal`: two authored channels create a linked static pair.
4. `PortalBodyView` and `PortalWorldFrame`: copy simulation facts into presentation.

The tutorial is its own nested Cargo workspace so its desktop render features do not leak into Ambition's main workspace. The important setup is visible in `src/main.rs` and intentionally kept in one file.

## Suggested reading order

1. Find `PortalPlugin` and `PortalPresentationPlugin` in `main`.
2. Read `setup` to see the minimum components on a transiting body.
3. Read the chained schedule sets: host motion runs before `PortalSet::Transit`, and
   observation runs afterward.
4. Read `observe_body`; this is the explicit simulation-to-presentation seam.

## Exercises

- Change one portal normal to place it on a floor or ceiling.
- Add a second body with a different `PortalPolicy`.
- Enable `view_cones` in `PortalPresentationPlugin` and publish a `PortalViewer`.
- Replace the automatic motion system with keyboard or gamepad input.
