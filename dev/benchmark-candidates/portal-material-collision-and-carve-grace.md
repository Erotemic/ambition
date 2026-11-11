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

An opening can first meet a primitive at the complete cast end, before any
material area crosses. Retain that closed contact stratum. Cut the source
host at the same boundary, and retain contacts with finite opening ends and
far obstacles. A contact stratum uses the incoming motion derivative; a
static pose query loses this contact. Count material interiors for the piece
budget. Do not count a boundary twice or revisit a column already cut from a
stay region.

A periodic collider can approach the same body more than once in one tick.
Reject only the current continuous overlap component. Search later approaches
against the same collider. One-way normals, edge identity and surface velocity
must use the physical contact time. Read the moving pivot at that time too.
Use the pose and interval evaluation errors when tracking a rejected component;
otherwise a change of world origin can spend all work on its rounded boundary.

Curved partitions need bounds for both material and the carried reference.
Prove that a column can contain material before splitting its complement.
Independent bounds for link order can otherwise retain empty branches at a
rank change. Material carried through a link is in front of its exit plane.
An entry with the same normal curve cannot cut that material from a front
reference. Use this geometric fact before interval subdivision.

Line openings must use the admitted partition. For an affine cell, derive
active opening intervals from its child cells. A second rank test at the
contact instant can disagree with the interval that already owns that contact.
For a turning or sliding opening, keep the cut motion through the query.
Clipping the final line once gives the wrong finite endpoint path.

At adjacent times, a midpoint can round to an interval end. Use the larger
distance to the two ends. Include angular argument rounding in curve bounds.
A large winding can amplify that error even within one representable time
step. An endpoint's actual material must remain inside the interval bound.


A rounded contact box can hide penetration by canonical material. Use the
canonical projected extent to place a block contact. Round the accepted
coordinate toward free space and commit that coordinate directly. Do not
loosen aperture fitting to accept a body that contact placement put in a wall.

Classify a cell from its source, path and active constraints. A distant turning
link does not make a static cell curved. Use exact translating constraints
when they apply. A constant rounding bound can keep an empty cell possible
through every time subdivision. More subdivision or a larger work budget
does not prove that cell empty. Retain closed boundary strata for contact.

End a source leg at its root crossing. Rebase the material before the remaining
travel continues. A departure-chart cast cannot retain its far chart by moving
the old root behind the entry. A return chart step restores existing material;
reduce that inverse path before checking its traversal budget.

A finite cross-section needs no artificial thickness. Native point and segment
domains must keep their closed contact set. Build their planes from the convex
boundary; repeated box vertices do not give a segment a polygon winding.


An admitted material map can have a small orthogonality error. A coincident
host face can gain a thin solid region when its vertices and the portal cut
are projected separately into canonical axes. Keep physical cut constraints
with the cell. A cut fixed relative to a translating obstacle can prove that
the obstacle has no solid area in that cell before inverse projection. Do
not apply this area rule to intrinsic points or line segments.

A source that fits an opening stays in its front spatial cell before first
touch. Keep that boundary through the topology event, even when the temporal
program can represent the earlier material as a whole primitive. Otherwise
separate rounded contact and topology times can give the old cell a false
host contact. An event-time epsilon does not fix this ownership error.

Repair gravity penetration before transverse motion with canonical material.
Keep velocity and contact state during that correction. A rounded root pose
can put canonical feet slightly inside a floor. A transverse sweep from that
pose can otherwise lose speed to an unrelated floor overlap.

A support probe can start with a line inside the body. A contact at probe
time zero does not prove support. Check that the admitted material is in
front of the physical support plane. Keep the plane point and normal in
wide precision until this check ends. Apply boarding rules to the resulting
support candidates, not to a root bounding box.

Read a moving support at tick start when finding its previous rider. Map
its physical contact and displacement back through the body's occupied
part. Use the admitted support for carry and slope corrections. A later
root-box query can select another surface or put a body back on an opening.


## Projectile recipients need the admitted source

A full root box includes material that the portal has cut away. It also omits
carried material. Sweep the admitted source cells against the sampled target,
with both sets of cuts. Keep target circles analytic. Source-map curves need
an interval proof; matching endpoint poses do not prove a miss.

Keep separate bounds for separate cells. Their combined box can cover empty
space between portal ends and admit an unrelated invalid recipient. Reuse the
bounds that material admission already validates. Do not rebuild them for
each recipient. Body eligibility must precede its geometric validation.

Prepare unit-bearing payloads with the contact before commit. A length map
converts `LaunchSpeed`; it does not convert dimensionless `FeelScale`.
`a91231348` and `4825f1f1c` have the source, controls and real actor witness.

A carried contact response needs the full velocity map at its physical point.
Use the existing material payload law. Portal translation and spin contribute
an affine offset. Apply the response in that physical chart, then invert the
whole velocity map. Admit the returned payload over every occupied source cell
before guard, reward or ownership writes. Keep the source root prefix and the
physical cue position as separate facts.
