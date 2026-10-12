# Portal pair identity and derived aperture - an engine contract

**Status:** CORE DONE 2026-10-12 (witnesses 1, 2, 3, 4 and 6 below, in
`ambition_portal2d`); witness 5 (two GNU-ton bosses) has its interim repair (2026-10-12: one
`PortalLink` per scholar, a `PortalLook` for the colours; see
[GNU-ton debt 12](../game/gnu-ton-fight-debts.md#12-the-portal-and-trail-names-omit-the-encounter));
the port that deletes the game reconciler is OPEN. Discovered in the October 11
late-source review of `abf79de..ed0d1ccd`.

## What landed (2026-10-12)

- **A pair keeps its channels.** `resolve_portal_links` keeps the channel
  pair a link group already has and each end's end of it. A new group takes
  the lowest free pair **of its own live room**. Position decides an end only
  when the end has none yet. There is no rank over all hashes, so a new or
  removed pair, in this room or another one, does not rename an open pair,
  and two ends that pass or meet keep their roles.
- **The cap is per room and deterministic:** 63 groups in one live room
  (bases 128..252). The 64th group of a room is refused (dead channel 255)
  and the open pairs keep theirs. 64 rooms with one pair each all work.
- **The two namespaces are separate:** `c128` and up are refused by both
  channel parsers (`PortalChannelColorSpec::from_name` gives the LDtk error
  "unknown color"; `PortalChannelColor::from_name`), and a monolith test checks
  that `PortalChannelColorSpec::FIRST_LINK_CHANNEL == LINK_CHANNEL_FIRST`.
- **The aperture is derived.** `PlacedPortal.half_extent` is now
  `authored_half_extent`, which nothing but its owner writes.
  `equalize_pair_apertures` writes only `partner_opening_half` (derived from
  the two authored extents). Every reader of the opening (transit capture
  box, `portal_fits`, straddle and carve via `aperture()`, projectile transit,
  the glow, the compositor pane, the gizmo) reads
  `opening_half_extent()`. The rename made each old reader a compile error.
- **Witnesses:** `link::tests::{a_pair_keeps_its_channels_when_another_pair_opens_and_goes,
  the_ends_of_a_pair_keep_their_roles_when_they_pass_and_meet,
  a_pair_opens_its_smaller_end_and_opens_wide_again_when_it_grows,
  sixty_four_rooms_with_a_link_each_all_work,
  the_sixty_fourth_group_of_one_room_is_refused_and_the_others_keep_their_pairs}`,
  each red under its own poison (claims ignored; ends by position; minimum
  written back; one cap over all rooms). Witness 1 runs the eviction road: a
  body in the plane of the open pair is not pushed when another pair opens.
- **Not done here, and why:** transit, cooldown and eviction still know a
  portal by `(live room, channel)`. That is sound now that a live pair's
  channel does not change, so a separate logical pair key was not added (no
  witness needs it). The rewind and fresh-room-load arms of witness 4 were
  not run as their own tests: `partner_opening_half` is a field of the
  clone-snapshotted `PlacedPortal` and is derived again each frame from the
  authored extents. The dev capture tool uses pair 254/255, which overlaps
  the dead link channel 255; it is a tool-only scene with no link portals. **Owner:** `ambition_portal2d`, with portal presentation
and lifecycle adapters. **First new customer:** two simultaneous GNU-ton
encounters. This page owns the general portal defect; the
[GNU-ton debt plan](../game/gnu-ton-fight-debts.md) owns the game adapter and
module-port migration. The [queue](../queue.md) owns execution order.

## Current authoritative paths and exact failures

`crates/ambition_portal2d/src/link.rs:69-124` resolves an LDtk `PortalLink`
by collecting `(live room, link hash)` groups, then assigns each distinct
hash a *rank* within all active hashes (`:85-89`). Its physical channel is
`2 * (64 + rank) + endpoint-slot`. It then assigns the end slot from sorted
**positions** (`:90-111`) rather than from an authored end role. A portal's
`PlacedPortal.channel` is therefore a derived, frame-dependent number, yet
other systems treat it as an authoritative identity:

- `crates/ambition_portal2d/src/transit.rs:34-41` records a currently
  straddled channel in `PortalTransit`; `:288-295` records an exit channel in
  `PortalTransitCooldown` and the transit. Carving and partner discovery
  also search by channel. A new link hash sorting before an existing pair
  changes that pair's channel while a body can still hold the old one.
- Two linked ends whose positions cross exchange slots. Two ends at the
  *same* position both find the first matching slot by `position()` and
  can get the same channel, so the pair cannot be found by `partner()`.
- The high `Indexed` range is also syntactically authorable via `c128`,
  `c129`, etc. (`color.rs::from_name`). Rank assignment does not reserve
  those names; authored-channel and link-channel namespace separation is
  not enforced by the codec. The link rank also has a fixed 63-hash limit
  **across all simultaneously present live rooms**, not per room, because
  `group_index` is built by collecting keys from every room before sorting.
  A pair added in an unrelated room can renumber an existing pair; 64
  distinct names spread across 64 otherwise simple rooms exceed the cap.
- `link.rs:130-153` reads current `PlacedPortal.half_extent`, shrinks it
  to the smaller paired opening, and writes the smaller value back. An
  end that was 80 units wide beside a 30-unit end becomes 30; expanding
  the latter to 80 later cannot restore the former's authored 80 because
  the original extent was overwritten. The existing test checks the first
  shrink (`link/tests.rs:82-106`), not expansion or replay.

These are distinct from GNU-ton's own fixed channels 24/25 and its
room-and-line reconciliation. Fixing only GNU-ton's ID join leaves the
core's positional and channel-index identities unsound for moving links.

## Target semantics, without a universal entity registry

A portal pair is identified by **live room + stable owner/placement
occurrence + named pair slot**. One end has an explicit stable role A or B.
LDtk authoring may continue to use a readable pair link; preparation
resolves and validates it against the room's placement occurrences and
explicit endpoint roles. Procedural extensions may request the same pair
through a held domain port, with the module invocation/encounter occurrence
qualifying its slot. A pairing fact cannot depend on entity iteration,
world-space position, a display colour or a sorted inventory of unrelated
pairs.

`PlacedPortal` is geometric placement; the pair/end key is its logical
relationship. Transit, straddling, cooldown, carve, and far-side presentation
must refer to the same stable logical pair/end where they need continuity.
`PortalChannel` may remain the authored visual/legacy channel until its
consumers migrate, but a temporary channel assignment must not rename a
live transit. Do not hash the pair key into an 8-bit channel and assume
collisions away. If a legacy channel-only path remains, validate its
namespace and explain what caps it.

An authored half aperture is immutable input. The effective opening of two
ends is a **derived** minimum for this frame. Both transit and rendering
read the same derived aperture, but nothing shrinks the authored geometry
or its saved/restored state. Changing a partner's size then grows/shrinks
the effective opening on the next accepted frame. Use a single derivation
for both ends, not update-order-dependent mutation of either end.

This is not a request for a complete scene graph, generalized link database,
or portal-specific boss actor. Reuse room/occurrence identity, existing
`PortalSet::Frame` boundaries, existing transit logic and presentation
adapters. Make the API independent of GNU-ton, Mary-O and LDtk. A bounded
bridge to old manually paired colour channels is acceptable only with an
explicit incompatibility/refusal rule.

## Witnesses in implementation order

1. Link group `z` works, then a pair `a` appears in the same live room;
   repeat when `a` appears **only in another live room**. The original
   `z` pair and a body already straddling it still refer to the same
   physical entry, exit and cooldown pair. Remove `a` again and verify
   the same identity.
2. Two endpoints of one link pass each other and later coincide; the roles
   remain distinct and transit still goes to the other end. Explicitly
   poison the role mapping back to position sorting.
3. A legacy manually authored `c128/c129` pair coexists with one linked
   pair. They must never be mistaken for each other. Prove either correct
   separation or an explicit validation refusal, not accidental success.
4. A pair of widths 80/30 equalizes to 30, then 80/80 grows to 80. Repeat
   after rewind and a fresh room load. Vary spawn/query order.
5. Two GNU-ton bosses request pairs in one live room, one pair is removed,
   a body crosses the survivor, then rewind. Repeat with the same room
   definition live twice. Use the [GNU-ton owner](../game/gnu-ton-fight-debts.md)
   to delete the fixed-channel game reconciler after the domain port lands.
6. Check the number of simultaneously admitted authored/manual/procedural
   pairs across many different room instances. A cap must have an explicit,
   deterministic refusal policy; 64 separate rooms with one distinct linked
   pair apiece must not silently disable one room's pair, and adding a pair
   may not silently repoint existing pairs.

These are **design-level counterexamples derived from source**, not tests
already run. Build the failing production-host witnesses before claiming
repair. Avoid unrelated portal physics refactors unless a witness exposes
them.
