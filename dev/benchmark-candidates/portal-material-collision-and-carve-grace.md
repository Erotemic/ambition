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

A world contact and its response need separate typed results. The contact can
order a leg even when the rebound refuses. An earlier terminal target outcome
retires that later response. A reached rebound must fit every occupied material
chart before any pose, bounce budget or combat write. Root finiteness alone
does not prove carried velocity is representable. The numeric check and the
next-tick separation witness serve different invariants; keep both.


For motion after a root crossing, advance age and apply forces once before
the leg loop. Each leg needs its own departure material and interval in the
original tick clock. A contact query's local time is not the tick time.
Prepare the crossing only after earlier terminal contacts have been selected.
Commit through the common portal operation, then spend the unused time from
the admitted exit pose and velocity. Do not reset a projectile's contact
budget at each root. A refused later leg must retain the last admitted root
and retire its sweep.


A guard is canonical body material. Gravity alone does not give its accepted
turn or parity. Keep the authored region, including the canonical feet
offset, and project it with the body map. Partition that region directly.
Test front and top coverage after mapping the strike source back through
the contact path and root material. An AABB corner can be outside the guard;
use that corner to check exact shape contact. Keep numerical refusals through
coverage and ranked sibling selection.


A facing request and accepted material facing have different axes. Project
ordinary facing through the physical side, and use material parity for an
oriented route. Apply the request inside the admitted movement candidate.
A write after the kernel can undo its projection and its refusal. Neutral
input must not reissue an old pose. Concrete actions use accepted body facing.

A whole-point motion envelope can exhaust work on a nearly tangent contact.
For root material, bound signed plane distance with its directional derivative
and relative angular acceleration. This retains fast tangential travel
without widening the normal envelope by that full speed. Keep the original
conservative envelope when the tighter finite bound is unavailable. Do not
turn an unproved query into a miss or raise the work budget to hide the cause.


A guard response must retain both meeting pieces' root maps. Root positions
from different charts do not give the contact direction. Unit-bearing guard
speeds use the guard material map; duration does not scale. Pull velocity
through the recorded portal frames at the response position. Use the guarded
body's mapped velocity as the response baseline so a common boost cancels.
Prepare both launches before changing a body, move or guard state. A contact
chart does not make an approximate contact position or time exact.


A stored item's last world material is not its holder's coordinate frame.
Record the holder material at pickup and derive the later material change
from that basis. Select a released root from the native partition; a hand can
cross before its holder. The object's own budget applies at release. Prepare
all releases before hand, custody, stock, identity or membership changes.
A refused object release must not enter a quantity-mint path.

A spec id does not identify an occurrence. A nearby authored prop can take a
pickup press before an injected copy does. Use the authored occurrence and
its stable identity in an acceptance witness.


A rig point has a canonical feet offset. Map that offset and point through
physical material together. Gravity and the root AABB cannot reconstruct the
point after scale or reflection. Keep a missing landmark separate from a
failed map. Resolve a required hand before firing state changes; a muzzle
that does not use a hand must not query it.


A firing hand belongs to the shooter; an independent shot has its own root
extents. Map the hand through body material, then project the shot's extents
on the physical feet normal for clearance. Do not scale the shot's extents
as a side effect of placing the hand. A fixture that expects a rotated body
must state its physical material as well as its acceleration frame.


A child can need a new root chart before its emitter crosses. Select that
chart from the native material partition, then admit its mapped velocity and
carried vectors. Use the child's policy, not the emitter's. Stage resource
changes until the complete birth passes admission. A projectile cue uses
the admitted birth position.


A cut boundary can supply a tangent contact normal. An instantaneous closing
rate alone does not prove that a curved path stays clear. Full linear shapes
can prove continued separation with their native boundary planes; their
clipped subsets stay clear too. Retain the continuous result for curves.
A refused support query keeps the accepted body and its settled state.


A free body's root crossing changes the link that seeds the next native
cast. A later righting proposal must use its recorded leg's root link and
world duration. A zero-duration end pose has no time for a source turn.
A contact or root crossing at tick end can leave no free leg. Keep the
accepted translation when a solid blocks the turn. Query hurt material
from the accepted path before impact facts commit. Retire transport that
has already produced its root transfers so a later transit phase cannot
process it twice.

A source cell's linear travel covers its cast interval. A world's published
collider displacement covers the full tick. Scale the latter by the cast's
world duration before the separation proof compares these two quantities.

Build source cells once for each recorded motion leg. Reuse them across
recipient queries. Keep exact contact and refusal with each recipient.
The source chart, clock and admission snapshot bound the cache lifetime.
Geometry validation can be shared within that lifetime; each sample must
still check its clock. A broad phase must include all physical cells. An
unknown motion envelope retains its candidate for exact admission.

A segment can be disjoint from a carried piece by a very small, constant
gap at a portal plane. A curved envelope can retain that segment throughout
many time subdivisions even when every exact sample is empty. Apply the
cell's fixed world-plane separation proof before curved contact for segments
and points as well as polygons. Keep a nonempty clipped segment or point;
requiring positive area would remove valid boundary contacts. A moving cut
can be used only when its rate matches the collider's motion in that chart.
Keep finite-input validation before candidate exclusion. Do not raise the
interval budget or convert a numerical refusal into a miss to hide this cost.

A collision query can reject a contact component that persists while the body
turns. Prove its continuity in a common physical chart when
possible. Bounding a collider point and a portal cut separately in rotating
material coordinates loses their shared motion and can require very small
time subdivisions. A complete set of cuts with fixed normals and affine
offsets permits an affine relative-distance proof for a collider point with
no angular motion. A partial set is sufficient for exclusion, but is not
sufficient to prove containment. Keep the conservative proof when any cut
or its eligibility is unrepresented, and still prove containment in the body.
