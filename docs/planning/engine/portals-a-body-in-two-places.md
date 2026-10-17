# Portals: a body in two places

Jon 2026-10-16: "having general attacks and long hitboxes and collision geometry work
properly when they are carved and partially teleported by a portal is essential. I don't
want any hacks for this. The portal should carry the physical consequences even when
something is in transit." The first user is GNU-ton's phase-1 punch: the arm goes down
into the near portal of a pair and the fist hurts at the far one.

## The rule

A volume that crosses the plane of a portal inside its opening, coming from the
portal's front, is in two places. The part in front of the plane stays. The part past
the plane and inside the opening's span is at the linked portal, mapped through the
pair. A part past the plane and beside the opening stays where it is (in free air it
passes beside the portal; on a host it is in the host).

This is the rule the presentation already draws a straddling body by
(`ambition_portal2d::pieces::compute_body_pieces`). Today nothing else uses it: combat
tests one volume in one place, and a straddling body collides only with its own side
(surveyed 2026-10-16).

What counts as "coming from the front":

- A volume of a body that straddles a pair (`PortalTransit`) is cut by that pair, on the
  side the body is on.
- Any other volume is cut by the first opening it crosses whose front its root (its
  owner's centre) is on. A sword swung into a portal reaches through it; the swordsman
  need not be in transit.

## Pieces

`ambition_platformer2d_core::frame::thread`. A piece is a whole shape placed in its chart
plus the half planes that bound the part of it that is there. A circle cut by a plane
stays a cut circle, so the test is exact for every `CombatVolume`. Two pieces meet when
the shapes and all their half planes share a point.

- `here`: the volume, in front of the entry plane.
- `beside`: the volume, behind the plane and past one end of the opening (0, 1 or 2).
- `through`: the volume mapped through the pair, in front of the exit plane and inside
  its span. It carries the pair, so a point of the owner's chart (a strike's source, for
  its knockback) maps into the piece's chart.

## Stages

1. **Geometry**: pieces and the meet test, pure, in core.
2. **The pairs combat can see**: the portal domain publishes each live room's linked
   apertures and its map convention in a resource of core vocabulary, so combat needs
   no portal feature.
3. **Strikes**: `apply_hitbox_damage` tests every piece of a hitbox. One victim is hit
   once (the hitbox's victim set is per hitbox, not per piece). Knockback and impact
   come from the piece that connected, its source mapped into that piece's chart.
4. **Hurtboxes**: a victim's volumes are cut the same way, so a body half through a
   portal is hit at both ends.
5. **A posed body in a pair**: a body whose pose an owner holds (`PoseOwnedExternally`)
   is posed in its own chart, as if there were no portal. The portal cuts it and does
   not transfer it. Its pieces carry the consequences, and its drawing is cut by the
   same pieces.
6. **Collision on both sides**: a straddling body meets the exit's geometry with its
   part past the entry plane. The exit side's solids in front of the exit and inside its
   span are mapped back behind the entry opening, and the body collides with its own
   side and that, as one space.
7. **GNU-ton phase 1**: the arm is one limb (forearm and fist) posed down through the
   near portal; its damage box is cut by the pair like any strike. The giant's pair is
   bigger: he shows he can size his portals. Phase 2 keeps its look-only portals.

## Open

- Projectiles test a sweep, not a volume; a shot past a portal plane already transits
  itself, so they come last.
- Body-to-body contact (standing on a body) is not portal-aware.
