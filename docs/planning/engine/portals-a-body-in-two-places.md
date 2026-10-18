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

## State (2026-10-17)

Stages 1-7 landed (`6e9045068`, `15d0cf99c`, `bc9c56af6`); the audit fixes below
in `049a9081e`. The general model (below) landed as G1-G6 (`5f3a40dc1`,
`f9410f2cc`, `e16fca525`, `d301399ba`, `da519ec6d`, `e93fa533c`), with the
open items after it (`029a03a02`, `ef0dfd7e4`, `4cd9bece5`, `22f25a85d`,
`cb0a6abe2`, `5ff8fdd00`).

- Stage 5: a held body (`PoseOwnedExternally`) is in the pair it reaches into from a
  portal's front while it reaches into it; the transit, the snag and the eviction do
  not move it.
- Stage 6: `PortalCarves::glues` → the overlay's `portal_glues` → `as_met_by` glues
  each pair a body is in (`glue_portal_pair`), CARVE_DEPTH deep, as the hole is.
- Stage 7: the giant draws the whole arm; the fist is posed under the near line
  (`arms::fist_depth`); the giant's visual is cut by the pair its fist is in
  (`PortalHostReach`); the fist carries its damage box (`ambition.boss.limb_strike`).

## Audit of the review of `15d0cf99c` (2026-10-16)

| | Finding | Status | Evidence | Gap left | Superseded by the general model? |
|---|---|---|---|---|---|
| A | Glue maps two corners of a box and boxes them; selects by the box of a turned region | **Fixed** (049a9081e) | `collision.rs` `glue_portal_pair`/`carried_box`: exact convex cut by the turned region (`frame::thread::face_rect_planes`, `clip_convex`), every corner mapped. Test `a_square_through_a_pair_that_turns_it_by_45_degrees_is_solid_where_it_is` (passes; that it fails on the old code is REASONED: the two corners of a square turned 45° share an x) | The world's solids are boxes: a turned image is put as its bounding box — conservative, more solid than the truth. A one-way block turned on its side is left out (no kind says it). The entry side is cut by the hole's box (the old hole approximation). Water and climbables are not carried. | Yes: piece queries (below) test the true clipped convex shape; no world is built. |
| B | A shot's leg ends at a crossing the transit may refuse | **Fixed** (049a9081e) | `placement::swept_crossing` is the one swept crossing: `swept_crossing_step` and `first_portal_crossing` both use it (paired, fits, a moving portal's scoop, backside pair excluded via `ProjectileCollisionWorld::backside_of`). It also exposed that the transit asked "fits whole" where the step ENDS (a fast body past the plane fits trivially) — now where the centre crosses; and the rescue took a body beside the opening — now only one in line (`snag::in_line_with`). Test `a_shots_leg_ends_only_where_the_transit_takes_it`. | (1) After the crossing the rest of the leg is swept in neither room this tick; next tick the shot is put at the mapped end — a wall just in front of the exit inside that remainder can be tunnelled (REASONED, not reproduced). (2) Cooldown: neither the swept crossing nor the leg cut asks it (they agree). (3) Contacts before the crossing are ordered by the leg; the crossing is a cut at its fraction, so a wall before the plane is met first. | Partly: the leg cut stays; the remainder becomes a second sweep in the exit room (stage G2). |
| C | `PortalParallaxCarry` is one global | **Fixed** (049a9081e) | Per view: `carry(view, by, now)`/`at(view, now)`; the continuity carries the views whose `ResolvedViewSubject` is the body that crossed, and maps only their eased targets; `parallax.rs` reads the layer's own view. Test `a_crossing_in_one_view_carries_no_other_views_foreground`. | `PortalCameraContinuityState` is still one focus (`focus_bodies.iter().next()`): two views crossing at once are not both continued. | No (presentation ownership, separate). |
| D | Glue rebuilds the room per straddler | **Fixed** (`f9410f2cc`, `da519ec6d`): a straddler's world is the room's shared lists and a layer; 1.4 µs at 1 000 or 5 000 blocks | `glue_cost` (ignored test, release, this machine): 20 µs/body/tick at 100 blocks, 245 µs at 1 000, 1.7 ms at 5 000; unglued 5 ns (borrowed). The per-body hole (`as_met_by` with holes) already cloned the room the same way. | Linear in the room per straddler per tick. Not a correctness bug; on Android it is several times this (REASONED). | Yes: stage G2 queries the room's broad phase through the piece; no copy. |
| E | `thread_volume` stops at the first pair | **Fixed** by G1 (`5f3a40dc1`): the partitioner cuts by every pair, chains under `Composed` | `frame/thread.rs` `thread_volume` returns at the first opening; `cut_through` cuts one pair; a through piece is not cut again; `PortalPassages::straddled_by` and `PortalTransit` hold one pair per body. | One volume across two pairs is cut by the first only; nothing chains (A→B then C→D). | Yes: the worklist partitioner (G1). |

The repairs the review found already made (held items, transfer straddle state,
transit outliving its pair, item eviction, free-body sweeps, eviction order,
moving-host poses) keep their tests; nothing above touched them.

## GNU-ton's punch: what it does and does not do

Does:
- ONE pair, one hop. The fist's body is posed under the near line; it is in the pair
  (`PortalTransit`, held rule) while it reaches into it.
- Damage: the fist's box (`LimbStrike`, `FollowOwner` of the fist) is cut by the pair
  (`apply_hitbox_damage` over `PortalPassages::pieces`); it hurts under the far line
  and not in the giant (test `the_punch_hurts_under_the_far_line_through_the_pair`).
- Drawing: the giant's whole picture is cut by the pair (here: in front, deeper than
  the fist, beside the opening; through: under the opening to the fist's depth).
- A body under the far line half through the pair is hit at both ends (combat stage 4).

Does not:
- Physical contact: the fist is posed, so nothing pushes it and it pushes nothing; it
  lands where its keys say, not on what is under the far line (G6). The giant does
  not collide through the pair.
- The near line is fixed to the giant each tick; a giant that moved during a strike
  would re-place it (held bodies are not evicted, so the fist is not pushed out).

Since G1-G5 (2026-10-17): a second pair in the arm's way cuts it too (side by
side always; one after another where the game's `PortalGeometry` composes
them), the drawing has a piece through each, and a player's strike through a
pair reaches the giant (feature hits take the pieces).

## The general model (implemented, G1-G6)

Invariant: a portal transforms where the parts of an object are; it never makes a
second object. One logical body (health, brain, inventory, authority) owns any finite
number of portal-connected PIECES.

### Data

A piece, as built (`frame::thread::VolumePiece`):

```rust
struct VolumePiece {
    volume: CombatVolume,       // the primitive, placed in this piece's chart
    bounds: Vec<HalfPlane>,     // in the piece's chart: the part of it that is here
    to_piece: Iso2,             // root chart → this piece's chart: orthogonal 2x2 (det ±1) + translation
    path: Vec<u32>,             // the links crossed, in order (`PortalLink::key`)
}
```

A pair is two portals of one live room (`ambition_portal2d::rooms`), so a
piece is always in its root's room and needs no room of its own.

- `Iso2` composes as matrices; a reflection stays a reflection (`MapConvention::
  Reflection` has det −1). `frame::map_point/map_vec` are its two-frame case today.
- Pieces are values, rebuilt per tick from rollback state (portals, poses, transits);
  they are derived state like `PortalPassages`, never rollback state themselves.
- Nonconvex shapes are lists of convex primitives (`CombatVolume::{Aabb, Obb, Circle,
  Convex}`, a body box, a sprite quad); each primitive is cut on its own. Bounds stay
  half planes, so a cut circle stays a circle (today's rule).

### The cut (one hop)

For a convex primitive C coming from the front of an aperture (frame o, n, t, half h):

- here: C ∩ {front}
- beside: C ∩ {behind} ∩ {past an end} (two pieces at most) — in free air it passed
  beside the slit; on a host it is in the host
- through: Iso(C) ∩ {front of exit} ∩ {exit's span}, carried to the exit's room

"Crosses" means the shape has area behind the plane inside the span (the slab rule,
today's `reaches_through`), not that it touches the infinite line. Grazing: area below
`EPS` is no crossing. Endpoints: the span is closed; a primitive across an end is cut at
the end lines (the beside pieces). Bodies never straddle an end (the snag holds them);
volumes may, and the slab rule says where each part is. "From the front" is the root
rule (today: the owner's centre, or the straddle state); a primitive whose root is
behind is not cut by that aperture.

### Many hops (the worklist)

```
work = [(primitive, Iso2::IDENTITY, bounds = [], path = [])]
while let Some(piece) = work.pop_front():           // FIFO: stable order
    candidates = apertures in piece.room whose bounds meet piece (broad phase), sorted by (room, channel)
    first = the candidate the piece reaches through from its front, nearest the root along the path  // deterministic
    if none: emit piece; continue
    parts = cut(piece, first)                        // here, beside, deeper, through: up to five
    if path.len() == max_hops or emitted+queued+len(parts) > max_pieces:
        refuse(piece, first); parts = cut without the through part   // up to four
        if emitted+queued+len(parts) > max_pieces: emit piece whole; continue
    push here/beside parts (bounds += cut planes)
    push through part: to_piece = Iso(first) ∘ to_piece, bounds mapped + exit planes, path + hop
```

The budget is checked on the parts a cut makes, so `max_pieces` is a hard bound on
the partition (one piece for a budget of none): a link with a depth makes five parts
(`a_partition_is_never_larger_than_its_budget`).

- Four pairs at once (scenario A): the first cut splits off one through piece; the
  here part is pushed back and cut by the next aperture, and so on — one through piece
  per pair, order by channel.
- A chain (scenario B): the through piece of A→B is in B's room and is cut by C→D;
  `to_piece = T_CD ∘ T_AB`.
- Invariants (pure tests): at each cut, the sibling parts tile the cut piece's source
  region (union = it, interiors disjoint, areas sum); `to_piece` of a chain equals the
  composed maps; area is preserved by every `Iso2`; the result is independent of the
  order apertures were inserted (sorted candidates); termination under the budget.
- Cycles: occurrences, not one partition. Re-entering an aperture with a different
  composed transform is a new occurrence; the source-space tiling holds per cut, not
  across the whole tree. Termination is by `max_hops` and `max_pieces`, never by
  detecting "the same" piece.

### Failure contract (budget exceeded)

Never drop collision-relevant geometry and play on. The aperture the over-budget piece
would cross is SOLID for that piece: the piece is clipped at the plane (its through part
is not made; when even the clipped parts would pass the budget, the piece stays whole
where it is, as at the step limit), and the body's motion is held there as the snag holds a body past an end
(collision); a combat volume simply does not reach further (it is not a body). A dynamic
portal is refused at placement when the room's declared policy cannot admit the network
it would make (e.g. a pair whose exit faces its own entry within `max_hops`); authored
networks are checked when the room is built. Each refusal logs once with the body and
the path.

### Consumers (one geometry, separate algorithms)

As proposed, then as built where it differs.

- Combat: `apply_hitbox_damage` tests strike pieces × hurt pieces over the
  worklist output; dedupe stays per hitbox × victim; source and knockback are
  carried by the composed `Iso2`. Feature hits take the pieces too
  (`HitEvent::pieces`). Built so (G2).
- Collision. Proposed: no glued world; a body's step asks the room's broad
  phase per piece (Parry casts of the clipped shape), and maps the contact
  back by the inverse `Iso2`. Built (G3): the step keeps its one world, and in
  a pair that world is the room's shared lists with a layer of its own (the
  hole, and the far side mapped back by the inverse `Iso2`), found through the
  room's grid. It answers the same question (what does each part of the body
  meet, where) with no second collision path in the kernel and no copy of the
  room; a turned solid is its exact outline. Moving portals: the layer is
  made again each tick from the pair's pose, so what is carried is where the
  pair puts it that tick; a carried block's own motion (a per-tick
  displacement) and the pair's own motion over the tick are both carried
  (`PortalGlue::dt`, the sim tick, with `frame::map_velocity`): a body
  standing on a far-side floor through a moving pair rides it as the pair
  shows it.
- Articulated bodies. Proposed: a contact is routed to the limb that owns the
  piece. Built (G6): a held limb is swept from its last pose to its new one
  through the world it meets and stops at the first contact; its owner reads
  where (`ambition.boss.mount`'s `contact`). One limb is one body, so the
  piece's owner is the limb.
- Transit: unchanged authority — a body is moved by the transfer rule; pieces
  describe where its parts are, they do not move it.
- Drawing (G4): every piece of a visual is drawn by a clipped quad with the
  piece's `Iso2` and the planes that cut it (up to six). Pooled quads
  (`ClipQuadPool`), reused frame to frame; room and view isolation as before
  (`InRoomInstance`, `PresentedForView`).

### As built (G1-G6)

- **G1** `frame::Iso2` (in `frame.rs`), `frame::thread::{partition, PortalLink,
  PieceBudget, Partition, Refusal}`. The worklist as above; a piece carried
  through a link has its root on that link's exit (where the root's line
  meets it); `first` forces the straddled link. `PortalLink::depth` limits how
  deep an opening carries (a host's picture, below). `VolumePiece::
  active_bounds` drops the planes that do not cut it. `recursive_links` names
  rings. Tests: tiling, 45°, four pairs, a chain, mirror vs turn, order
  independence, grazing, a ring at its budget, a straddler's first cut, a
  depth-limited link, rings.
- **G2** `PortalPassages` holds links and a budget per room and runs the
  partition; combat uses it unchanged in shape (`carry`/`uncarry` are the
  `Iso2`'s). `HitEvent::pieces` and `meets`/`meets_aabb`/`impact_on`: bosses,
  breakables, the rope and a broadcast's actors are hit by the pieces
  (`hit_reaches_victim`). Tests: a feature under the far line; a strike
  through two chained pairs hurts only where the room composes them. The
  GNU-ton punch tests pass unchanged.
- **G3** A body steps against one world, and in a pair that world is the
  room's with a layer of its own. `World::blocks` and `World::chains` are
  `layered::Layered` lists: the room's base shared (`Arc`), and per world
  entries hidden and added; readers read them as a `Vec`. The hole, the gates,
  the glue, moving platforms and contributed blocks are layers, found by a
  grid over the shared base (`Layered::near`, made once per base), which the
  body sweep also asks for the blocks near its swept box. So no world is
  copied (a clone is 40 ns), and a straddler's world costs what is near the
  pair. A solid block a pair turns is carried as its exact cut, a
  `SurfaceKind::Solid` outline: the walker stands on its upward edges and is
  put out of it the shortest way (`solid_lines`). A one-way block turned off
  upright is carried as its top, a one-way line. Water, and ladders the pair
  keeps upright, are carried. This is the proposal's "query the room's broad
  phase through the piece" without a second collision path: the kernel keeps
  its one step, and the world it steps against is the query's answer.
- **G4** The straddler drawing cuts the sprite's drawn quad by the
  partitioner (every pair of the room, the straddled one first, the body's
  budget) and draws each piece with its composed map, clipped by up to six
  planes. A host's picture is cut by its limb's pair as deep as the limb
  reaches (`PortalLink::depth`). `ClipQuadPool`: quads and materials are made
  once and reused (body and far-side pieces). Fixed on the way: a rolled body
  through a mirroring pair was drawn rolled the wrong way
  (`PortalCopyTransform::rolled`).
- **G5** `PortalTuning::geometry` (`PortalGeometry::{TransferOnly, SingleHop,
  Composed}`) is the game's rule; `PortalPieceBudget` on a body is its own.
  TransferOnly: no pieces, no glue, no cut picture. A refused cut is logged
  (`ambition_portal::refusal`) and its opening is solid to the piece, or the
  piece stays whole where the budget cannot pay for its clipped parts
  (`Refusal::whole` says which); a
  composed room says its rings once (`ambition_portal::rings`). Placement is
  not refused: the budget bounds a ring, and refusing a player's portal shot
  is a game decision.

- **G6** `posed_limbs::stop_posed_limbs_at_what_they_meet` (after
  `boss_conduct` lowers): a held limb goes from where it was toward its pose
  and stops at the first solid of the world it meets (its hole, and the far
  side of its pair glued behind the near end). A pose into something it was
  already in, or a move of more than four times its size in a tick (a
  reposition), is left as posed. `ambition.boss.mount` v2 tells the owner
  where each limb touches (`contact`, at the far end when through the pair);
  GNU-ton's landing dust is there. The host does not recoil. Taken on the
  design's recommendations below, without Jon's answers.

### Policy (portal geometry modes)

Owned by the game's rules for a room (`DeclaredRules`/room rules), overridable per
body or per ability by a component; never by a graphics quality setting.

- No portals: the portal plugins are not installed; no system runs, no resource exists.
- `TransferOnly`: the transit and its snag; no pieces, no glue, no cut drawing — a body
  is in one place, moved at the crossing. Cheapest.
- `SingleHop`: the worklist with `max_hops = 1` (today's behaviour).
- `Composed { max_hops, max_pieces_per_body }`: the full worklist.

Mixing: a body's own policy wins for its pieces; a room's policy is the default for
bodies in it; a pair across two rooms with different policies uses the lower of the
two for crossings through it (a `TransferOnly` room does not grow pieces from a
`Composed` body's visit — the body is transferred whole at that pair). Everything is
a pure function of rollback state and the policy, so it is deterministic and saves
nothing new.

Cost control: a body is partitioned only if its bounds meet an aperture's slab (broad
phase over the room's apertures, a handful); the worklist runs for those bodies only;
scratch buffers are reused (built: per thread, so each system's calls share them);
candidates are sorted vectors, not hash maps.

### Benchmarks to take (before and after each stage)

No portals near; one stationary pair (no straddler); one body straddling one pair;
a giant straddling four pairs; an arm through two pairs in sequence; a cycle (a pair
facing itself). Time per tick and allocations per tick, release, on this machine and on
the phone (`profile_android.sh`).

Measured (release, this machine, 2026-10-17; `partition_cost` and
`glue_cost`, ignored tests, and `partition_allocations`. Not on the phone: this
machine has no adb and no phone attached, so `profile_android.sh` waits for one):

| case | per call |
|---|---|
| partition: no portals | 0.05 µs, 2 allocations |
| partition: one pair, not reached | 0.05 µs, 1 |
| partition: straddling one pair | 0.40 µs, 7 (was 0.85 µs, 36) |
| partition: a giant across four pairs (10 pieces) | 5.3 µs, 30 (was 12.9 µs, 572) |
| partition: an arm through two pairs | 0.81 µs, 11 (was 2.0 µs, 89) |
| partition: a pair facing itself (budget 4) | 3.3 µs, 33 (was 7.9 µs, 371) |
| glue, 100 / 1 000 / 5 000 blocks, per straddler | 0.9 / 1.5 / 1.4 µs (was 18.6 / 205 / 1 287; the pair moved to the first row so every size has the same neighbourhood) |
| a world's clone (any size) | 40 ns |

Allocations per call are counted by `partition_allocations` (its own test
binary, with a counting allocator). The "was" figures are before the scratch
buffers: every emptiness test cloned its piece and allocated a polygon per
plane. Now a piece is tested in the thread's kept buffers and only the pieces
of the result are built, so what is left is the result (each piece's planes
and path) and a convex hull's outline.

## Roadmap (each stage reviewed before the next)

Status 2026-10-17: G0-G6 done (see "As built").

- **G0 (done)**: audit fixes A-C; D measured.
- **G1 — pure partitioner** (core, no ECS): `Iso2`, the worklist over convex
  primitives, the budget. Tests: the tiling invariants above, the 45° glue case, four
  simultaneous pairs, the two-pair chain (`T_CD ∘ T_AB`), a reflection pair, insertion
  order independence, grazing and endpoint cases, a cycle that stops at its budget.
  Benchmarks: the list above, pure.
- **G2 — combat on the partitioner**: `thread_volume`/`cut_through` become the
  worklist with `SingleHop` (behaviour unchanged, tests unchanged), then `Composed`
  where the policy says. `HitEvent` carries pieces for feature hits. GNU-ton's punch is
  the acceptance case, plus a strike through two chained pairs.
- **G3 — collision by queries**: replace `glue_portal_pair` with per-piece broad-phase
  queries; delete the per-body world copy (and the per-body hole copy with it: a hole is
  a piece query that ignores the host inside the opening). Acceptance: the stage-6 tests,
  the 45° case exact (not conservative), the cost benchmarks flat in room size. Risk:
  the kernel's step is built on a `World`; this stage needs a query trait the kernel
  steps against (`SolidWorldQuery` exists in `cast`).
- **G4 — drawing by pieces**: pooled clip quads per visual, N pieces; the host-picture
  reach becomes the general rule (a limb's pieces cut its host's picture).
- **G5 — policy**: the modes as room rules and body overrides; admission checks for
  dynamic portals; the failure contract's snag at an over-budget aperture.
- **G6 — articulation**: contacts routed to limbs with joints (separate design).

### G6 — articulation (the design it was built from)

Today a held limb (`PoseOwnedExternally`, the GNU-ton fist) hurts and is drawn
through a pair, and takes no contact: it goes where its owner poses it,
through whatever is there, on either side.

Proposal:

1. A limb is attached to its host (`Limb::of`); the owner's pose is the
   limb's target. (The design had a `LimbJoint` with a reach; the owner's
   pose already keeps a limb within reach, so none was added.)
2. Each tick the limb moves from its last pose toward the target by a sweep
   of its box through the world it meets: its own side, and the far side of
   the pair it is in, glued behind the near end. It stops at the first
   contact.
3. Where it touches is an observation for the owner's module (the limb's
   `contact` in `ambition.boss.mount`), so a choreography can react: the
   fist lands where it lands, its dust there.
4. The joint carries nothing back to the host (a giant does not recoil).

Decisions taken (2026-10-17, on the design's recommendations; Jon may
revisit any of them, and each is one place in the code):

- A posed fist stops at what it meets (physics), and the owner is told,
  rather than keeping its keys (`posed_limbs`).
- A limb meets its own side too (a fist into the floor beside the giant):
  one rule for both sides.
- No recoil into the host.

Cost: one box sweep per limb piece per tick.

Risks: the kernel's world-shaped collision (G3) is the largest change; rollback cost of
derived pieces must stay off the snapshot (derived, rebuilt); drawing more than one
through piece raises render cost — pooled quads and the broad phase keep it to bodies
that cut a portal.

## Rules

- A volume placed in the world (`HitboxAnchor::World`) is cut by what it
  reaches through from its own centre; a limb's box says what it is attached
  to by riding the limb (`LimbStrike`).
- A shot that crosses a pair ends its step just past the plane, and the rest
  of that step is its next one's (a delay of less than a tick, not a skip).

## Open

None. Closed since 2026-10-16: a shot's step ends at the crossing and its
next step sweeps the far side whole, walls and bodies (`5ff8fdd00`); feature
hits take the pieces (G2); the glue carries water, upright ladders, turned
solids exactly and turned one-way platforms as lines (`029a03a02`,
`da519ec6d`, `cb0a6abe2`); each view keeps its own camera anchor
(`ef0dfd7e4`); body contact is portal-aware both ways (`4cd9bece5`,
`22f25a85d`); a posed limb takes contact (G6, `e93fa533c`).
