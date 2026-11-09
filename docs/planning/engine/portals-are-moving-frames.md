# Portals are moving frames

The initial frame work was built on 2026-10-15. The active scale, chirality,
material and motion work is in
[portal parts, similarity and chirality](portal-parts-similarity-and-chirality.md).

## Contract

- The host publishes displacement, turn, linear velocity and spin. A reader
  must not derive velocity from a difference of portal positions.
- `PlacedPortal.step` holds the start pose and the host's travel during the
  tick. A full turn keeps its winding. An end normal alone cannot describe it.
- Owner placement starts a new pose. Host travel gives the continuous start
  pose through its inverse rigid map. A wrap is a location change outside
  that travel. Eviction compares the start with the previous accepted end.
- `PortalFrame.velocity_at` gives the surface velocity at the contact point.
  The transfer removes entry-surface velocity, maps the relative velocity,
  and adds exit-surface velocity at the mapped point.
- World gravity stays in the world. Body-carried acceleration follows the
  material map. Controller kind does not select a transfer path.
- A static portal is a frame with zero travel. It uses the same geometry and
  transfer rules.

## Implementation

The host adapter resolves an identified face or line in its own live room.
It writes the aperture pose and published host motion. Passage publication
keeps both endpoint steps in each directed link. `ApertureStep` samples the
host arc and translation. A `PortalLinkPose` is one instant of that record.

Captures retain their camera axis and the pose at capture time. A moving
portal does not bypass the capture budget. CPU cost and GPU capture cost
need separate measurements.

## Guards and remaining work

Core tests check host arcs, full windings and a common frame boost. Portal
contract tests check publication in both directions. Content tests drive an
identified translating platform and a spinning plank through the host adapter.
They also check that box and spinning hosts close the old plane on a wrap,
without a sweep across the room.
The app test `a_moving_portal_is_carved_where_it_is` drives the live room.

Translating apertures and obstacles use the same time interval. The temporal
constructor retains material that exists only between the endpoint poses.
Angular construction remains open. A query that can reach angular geometry
returns `CurvedMotion` until that constructor is complete. Actor contact,
physical righting and capture work stay open in the active plan.
