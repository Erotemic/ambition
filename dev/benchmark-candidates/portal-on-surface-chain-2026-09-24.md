# Portal on a surface chain

Act 3 first put floor portals on rideable surface chains. The portals loaded,
but a grounded runner crossed no portal. The headless test reached the goal
with zero `PortalBodyTransited` messages.

`ambition_platformer2d_world::collision` applies portal carves to blocks. It
does not cut a surface chain. The surface rider stays on the chain and does
not cross the portal plane. Act 3 now uses vertical rifts that the runner
crosses while moving right. The same headless test records a transit.

If a future level needs a portal in a chain floor, add a shared carve rule to
the surface solver and test a real body crossing it. A portal placement and a
portal render alone do not prove transit.
