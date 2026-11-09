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

For a moving obstacle, clipping its final shape and then translating the
fragment is not a continuous query. Keep the spatial cell fixed. A clipped
corner follows the intersection of its moving and fixed boundary lines;
its path can differ from the obstacle's displacement. Test the interval in
which each corner exists. A solid can first enter the cell as a line whose
ends are outside the body. Include that event as well as face and corner
contacts. Keep circles analytic.

Use the attempted motion's world-time interval. A leg that stops on a
portal entry must not include later motion by an obstacle. Read published
obstacle bounds as end-of-tick geometry. Broad phase must include its
complete displacement, even when neither endpoint meets the body.

An analytic face contact already supplies the time and the point on that
face. Test the point against the other constraints. Solving the same face
equation again can give a different rounded time and discard the contact.
The oblique-circle witness sweeps a wide face through many orientations
and checks its time, point and normal.

A moving aperture needs cells over the same interval as its host. Pulling
a host back to the tick's start while keeping the aperture at its end pose
can expose a false strip of host geometry. Keep their motion in one query.

Keep the primitive fixed in its canonical chart during a cast. Motion belongs
in the cell maps and cut rates. Broad phase bounds must include the cell in
source position and time. A cell can hold material between endpoint poses
even when it holds none at either endpoint. Round those bounds outwards;
rounding a bound inwards can remove a valid collision candidate.

At a topology event, the cells that begin there own contact. A cell that ends
must not turn an opening's first touch into a wall hit. Keep contact at the
end of the complete cast observable.
