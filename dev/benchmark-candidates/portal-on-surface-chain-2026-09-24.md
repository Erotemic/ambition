# Portal on a surface chain

Act 3 first put floor portals on rideable surface chains. The portals loaded,
but a grounded runner crossed no portal. The headless test reached the goal
with zero `PortalBodyTransited` messages.

The initial block carve did not cut the rideable chain. The surface rider
stayed on the chain and did not cross the portal plane. Vertical rifts let
the runner cross while moving right, and the same headless test recorded a
transit.

A block-only hole cannot open a rideable route. The surface solver must end
the route at the aperture. Verify a real body crossing it; a placement and a
render do not prove transit. For material contacts, keep the original room
geometry and derive the finite opening from the spatial cells. See
[carve margins and material contacts](portal-material-collision-and-carve-grace.md).
