# Portal material contacts and carve margins

A broad portal carve can delete a real obstacle. The projectile path used a
rectangle that extended 16 pixels in front of each portal. A wall two pixels
outside the exit disappeared from that collision view. A test in the live
`portal_lab` failed to stop a shot before its root transited.

Keep the physical room geometry for material queries. Partition the swept
spatial domain into cells. In each cell, cast the actual source primitive
against the obstacle pulled back through that cell's map. Do not use the
swept envelope as the contact shape.

A solid clipped to a cell boundary has no occupied area in that cell. That
boundary must not become a new wall. An intrinsic line needs its own finite
opening interval. Its ends must stay solid; a circle must meet an end on its
curve, not on its bounding box.

Keep the original collider and path through ordering and response. Pull its
surface velocity back through the inverse moving-frame law at the physical
contact point. A fixed world-space nudge changes scale behavior. Stop at the
selected contact and record the velocity that produced the travelled segment.

The real-app witness is
`a_projectile_hits_far_side_geometry_before_its_root_transits`. It checks both
absence of root transit and the physical far-side impact. The core material
cast tests check uncarved block hosts and finite line-host ends. The shared
projectile tests check the relative velocity of a bounce through a scaled,
moving pair.

Station K now owns the real-app witness. The LDtk loader places a portal
on a nearby face that has the same normal. A thin target two pixels beyond
an unhosted exit can put its back face inside that placement range. The
loader then moves the exit onto the target. Keep the target's near face at
the test distance and put its back face outside the placement range. Read
the loaded portal poses when an authored contact test fails.
