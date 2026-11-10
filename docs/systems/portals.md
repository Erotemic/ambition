---
status: current
last_verified: 2026-11-10
---

# Portals

Portals connect room-space apertures through similarity transforms. The
simulation owns crossing, collision, and persistent physical state. Rendering
observes that state; it cannot change admission or traversal results.

## Ownership and composition

- `ambition_portal2d` owns paired openings, transit, cooldowns, the portal
  shot simulation, and the portal-specific schedule.
- `PortalSimulationPlugin` installs simulation without a portal gun.
  `PortalGunPlugin` adds gun-specific interaction and construction vocabulary.
  The combined `PortalPlugin` installs both for callers that need them.
- `ambition_platformer2d_runtime` supplies host schedule and input adapters.
  A game that does not install portals still uses the ordinary body movement
  authority.
- `ambition_platformer2d_shared_tangle::body::pose::BodyPoseAdmission` is the
  general external-pose admission interface. `body::geometry::BodyGeometry`
  publishes material and portal geometry for supported queries. Gameplay code
  must not import a portal-specific pose API just to move a body.
- `ambition_platformer2d_core::frame` owns `Sim2`, `BodyMaterial`, canonical
  material partitions, and typed numerical refusal behavior.
- `ambition_portal2d_presentation` owns aperture glow, view windows, and
  carried-body capture/presentation. It does not own physics.

`PortalRooms` resolves installed portal capability by live room. A
multi-game host can include the portal crates for one game while another
room uses no portal systems. Keep queries and state scoped to the room and
session that own them.

## Crossing and material geometry

A `PlacedPortal` retains its authored opening length. Linked openings can
have different lengths; `opening_half_length()` no longer reduces both ends
to the smaller opening. `Sim2` composes translation, rotation, scale, and
reflection between room charts.

`BodyMaterial` records the body's persistent material scale, orientation,
and parity. A crossing carries that material into the destination chart.
The material model can describe parts of one body in multiple spatial
regions before and during a center crossing. Root motion and outside pose
requests use the movement authority and its refusal results.

Portal-aware casting is also used by projectile, hurt, strike, and
articulated-body contact paths. Check which path a consumer uses before
introducing another collision representation. Failed numerical or
partition queries must not be silently treated as valid misses.

An authored portal is anchored to its wall face by the LDtk adapter when a
matching face exists. A shot portal is placed on the surface it hit. Portal
IDs are authored/content identities; Bevy entities are runtime handles.

## Rendering

The portal presentation crate draws the opening, the mapped view, and
portal-carried body material. It publishes visual occupancy before a body's
root crosses, and it uses physical material transforms for scale and parity.
A capture is a derived image of a view, not an alternate motion authority.

Independent bodies must keep their own partition and depth semantics even
when presentation shares capture resources. Window resolution, recursion,
and carry capacity are renderer budgets rather than gameplay rules.

## Invariants

- A rejected movement or transit does not partially commit body motion,
  movement history, or dependent gameplay state.
- Passage geometry comes from canonical material, not an unrelated sprite
  envelope or player-specific collision path.
- Crossing composes scale, rotation, and reflection deterministically.
- Reset, room replacement, and restore do not retain foreign-room links,
  transit latches, or cooldown state.
- The active portal capability is optional. Portal-free providers must not
  import portal-specific gameplay interfaces.
- Headless and visible apps compute the same authoritative physics.

## Finding the current implementation

```bash
python scripts/agent_query.py "portal simulation transit material"
python scripts/agent_query.py crate ambition_portal2d
python scripts/agent_query.py crate ambition_portal2d_presentation
```

For behavior, use the real portal lab with an unequal-width pair, a body
whose pose shows reflection, and physical contact on both sides. For
regressions, use focused simulation/headless acceptance, then inspect a
rendered capture for alignment. Do not add a second proxy geometry model or
expand unit tests solely to grow coverage.

Active incomplete work and acceptance criteria live in
[`../planning/engine/portal-parts-similarity-and-chirality.md`](../planning/engine/portal-parts-similarity-and-chirality.md),
not in this evergreen system description.
