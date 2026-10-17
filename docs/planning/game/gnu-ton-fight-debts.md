# GNU-ton's fight with arms and portals: what is not elegant, and what it should be

Written 2026-10-11, when phase 1 (arms on the giant, a punch through a real
pair of portals) and the clamp went into the game. Jon: "if there is
anything inelegant that you've added, log it in docs/planning for refactor
later." The fight's design is in `bosses.md`; the debts of the giant's
PLACES are in `../engine/the-art-says-where-a-body-stands.md`.

Status of each: OPEN. The content comes first.

## What was elegant

- The pair of the punch is two `PlacedPortal` entities on partner channels.
  The engine did the rest: the lines, the window that opens when a body is
  near, the transit, the push of a body that is on the plane when the pair
  closes. No engine code was changed for it.
- The phase-2 fight is the fight of before, tick for tick (the parity test
  holds it), so the new phase did not cost the old one.

## The debts

### 1. The fist does not go through the portal. Two drawings pretend it does

**PAID 2026-10-16** (Jon: "use real portals to handle phase1 of gnuton's
giant's punches"; `docs/planning/engine/portals-a-body-in-two-places.md`).
The giant's rows draw the whole arm, uncut, and the fist's body is posed
under the NEAR line by the same keys (`arms::fist_depth`). The fist is held
(`PoseOwnedExternally`), so the pair takes it in and does not move it; the
giant's visual is cut by the pair its fist is in (`PortalHostReach`: under
the opening and as deep as the fist), and its damage box rides the fist
(`ambition.boss.limb_strike`), so it hurts at the far line. The pair is wider
(84 a side) and starts past the arm's root, so nothing else of the giant is
under the opening. The fist's rows `through_*` are no longer drawn in the
fight; a fist on its arm has no trail (`LimbBond` version 2, `start: None`).
Phase 2 keeps its look-only lines.

### 2. A module can not open a portal

**THE PAIR AND THE LOOK LINES LANDED 2026-10-12.** The conductor asks
for the pair of the punch each tick it is open through
`ambition.portal.held_pair` (`HeldPortalPair { slot, ends, looks }`), and
the portal domain's adapter (`ambition_portal2d::extension`) owns the two
ends, their link, their identity and their end. `open_gnu_ton_portals` is
deleted.

The look lines are `ambition.portal.light_line` (`LightLine { slot, at,
normal, length, look }`), debt 3's road (b). The adapter keeps this tick's
lines of each body as a view on the body (`HeldLightLines`, the derived
row `derived.portal.held_light_lines`), and
`ambition_portal2d_presentation::dress_held_light_lines` keeps a
`PortalLightLine` for each. The simulation spawns nothing for a line.
`light_gnu_ton_lines`, `GnuTonLine` and `GnuTonLookLine` are deleted; the
colours and sides of the four lines are the conductor's. Witnesses:
`ambition_portal2d extension::tests::each_body_has_the_lines_its_entry_asks_for_this_tick`
(poisons: no removal, red at "a body that asks for no line has no view";
the first word for a slot kept, red at "the second word for slot 0
counts"); `ambition_portal2d_presentation glow::tests::a_light_line_is_drawn_while_its_body_has_it_in_its_view`
(poison: no despawn, red at "a line the view lost is taken away");
`app_it gnu_ton_fight::his_look_lines_are_the_lines_his_conductor_asks_for`
(the view is the record's lines, and a defeated scholar has none; poison:
the conductor asks for none, red at "the lines drawn are the lines his
record has"). Not done: the line does not dissolve (debt 3). The trail starts are a
port too (debt 6). The text below is the case as found.

- The conductor is an extension module. It has no port for a portal, so it
  writes where each line is into its RECORD, and a native system of the
  game (`ambition_content::bosses::gnu_ton::portals`) reads the record and
  keeps the entities. The record is used as a channel.
- **Should be:** a port. `PortalPair { slot, near, far, half }` and
  `LightLine { slot, at, normal, half }` as held requests (as
  `HeldDamageBox` is): the host owns the entities, their identity, their
  end on defeat and on a reset. Then the Mockingbird or any module can use
  portals, and `portals.rs` goes.

### 3. A line that is a look needed a new thing

**ROAD (b), 2026-10-12.** `PortalLightLine` stays, and the presentation
spawns it from a view (`HeldLightLines`), not the simulation (debt 2). It
does not dissolve yet. The text below is the case as found.

- A `PlacedPortal` with no partner takes nothing through, but when it MOVES
  the engine pushes each body on its plane to one side
  (`evict_straddlers_on_portal_change`). The lines at the shoulders are on
  the giant and the lines at the wrists move with the fists. So the look
  lines are `PortalLightLine` (new, `ambition_portal2d_presentation`): the
  material of a portal's line, on an entity that the simulation spawns and
  the presentation dresses.
- It opens as a portal does. It does not dissolve: it is gone in one frame.
- **Should be:** one of two things, to be decided with the portal lane.
  (a) A portal has a flag "occupied": it is a real aperture with its glow,
  its dissolve and its window, and it takes nothing and pushes nothing.
  That is the lore (the arm is in it) and it is one concept, not two.
  (b) `PortalLightLine` stays, and gets the dissolve, and is spawned by the
  presentation from a view, not by the simulation.

### 4. The place, the length and the angle of each line are typed by hand

- `choreography::arms` has: the near line of the punch, the two shoulder
  lines, the wrist line, how far the far line is over the fist, and the
  width of the arm at each line (MEASURED from a rendered frame by a script
  that is not in the tree). When the drawing changes, they do not.
- Each line is upright or flat. The arm at a shoulder's line goes down by
  about 8 degrees; the line does not turn with it.
- When the giant bucks or rears, the lines stay where they are for the
  giant at rest.
- **Should be:** design A of `the-art-says-where-a-body-stands.md`, with one
  more kind of place: a `line` (a point, a direction, a width), per row and
  per frame. The renderer knows all three: it draws the arm and cuts it
  there. The conductor reads them through `MountView`. With angled portals
  the line turns with the arm.

### 5. The fists are bodies when they are on the arms

- In phase 1 a fist is in the giant's picture. Its body is still there: the
  conductor holds it in the middle of the giant, pins the row `none` (a
  frame of nothing in its sheet), and says it is no opening.
- A fist is built at its limb's home, and the conductor first runs on the
  second tick of the room. For those ticks the two fists are at their homes
  (the arena test `entering_the_arena_the_fists_are_on_the_giants_arms`
  bounds it at 3). Not seen in a window.
- A name tag ("Giant GNU Hand") is over the giant's chest for each.
- **Should be:** a limb has a state "on its host": no body in the world, no
  view, nothing to hit. Loosing it makes the body. (Or: the conductor can
  say a body is not presented, which is less.)

### 6. The trail of a fist joins two lines, by a resource

**THE START IS ON THE FIST, AND A PORT, 2026-10-12; the view index is
OPEN.** Each fist carries `LimbBondStart` (`ambition_characters::actor`, a
derived row). The conductor asks for each fist's start each tick through
`ambition.boss.limb_bond` (`LimbBond { slot, start }`), and the boss
domain's `bond_limbs` is the one writer of the start: a limb not asked for
in a tick has none, so its bond is from its host's body. `join_gnu_ton_lines`
and `portals.rs` are deleted, so no system of the game reads the record as a
channel. The fist's view carries the start as `FeatureView::limb_host`, and
the trail reads only the view. The renderer's `LimbTrailStarts` is deleted,
so no simulation system writes a presentation resource, and the start is
not keyed by `FeatureId`.

- Witness of the port: `ambition_boss_encounter conduct::limb_bond_tests::a_limb_has_the_bond_start_asked_for_it_this_tick`
  (poison: no removal, red at "a fist not asked for has its bond from the
  giant again"). With the conductor asking for no bond, the two app tests
  below fail at their own assertions.

- Witness: `app_it gnu_ton_fight::the_fist_through_the_pair_has_its_trail_from_the_near_line`.
  It compares against the near end of the held pair; the control is the
  other fist, whose bond is on the giant.
- `a_slam_lands_where_you_stood_and_the_stuck_fist_carries_your_blow_to_him`
  now asserts that a loosed fist's bond is its shoulder's line.
- A writer that writes no start fails both tests at their own assertions.

**Still open:** `FeatureViewIndex` is keyed by `FeatureId` alone, so two
live instances of one room share a limb's view. The text below is the case
as found.

- Jon (2026-10-11): the trail is part of the scholar's spell, and joins the
  two portals an arm is in. So the wisps (`limb_trail.rs`) start at the
  shoulder's line (or at the near line of the punch), not in the giant.
- How: a presentation resource, `LimbTrailStarts` (a start by the limb's
  feature id), which a system of the game writes each tick from the
  conductor's record (`join_gnu_ton_lines`). A simulation system writes a
  presentation resource.
- The trail ends at the middle of the fist, not at the line behind its
  wrist (the fist's picture covers the rest).
- **Should be:** with the port of debt 2, a line that a module holds can be
  "the start of this limb's bond" and "its end", and the presentation reads
  both from the view of the limb.

### 7. A pinned row does not loop

- On the actor road a pinned row plays once and holds, and its `elapsed`
  and `looping` are not read. So the giant's rest with its arms on it is
  two rows that are the same (`armed_rest`, `armed_rest_b`), and the
  conductor goes from one to the other.
- **Should be:** the actor road reads `looping` (and `elapsed`), as the boss
  road does. Then `armed_rest_b` and the turn go.

### 8. Two sets of rows for one body

- Each row of the giant is drawn with its arms in the lines; `armed_*` is
  the same row with whole arms. Only four rows have an `armed_*` (rest,
  its copy, hit, buck): a stomp in phase 1 would draw the arms in lines.
- **Should be:** with the giant as an SVG rig (debt 6 of the other
  document), the arms are parts, and "arms on" or "arms in lines" is a
  state of the rig, not a second set of rows.

### 9. Whether the arms are on is found from signs

- The trigger of a conductor does not say the encounter's phase. The
  conductor has `loosed` in its record and sets it: when the clamp is
  through, or when it sees a move of the free fists or an enrage with no
  clamp before it (a fight that starts late; the parity test starts so).
- A punch that is in progress when the clamp begins is cut: the arm goes
  from the punch to the clamp in one frame.
- **Should be:** `BossConduct` says the phase. And the pattern can end a
  move well before a phase ends (a property of the pattern's steps).

### 10. The hall of characters

- The giant in the hall has no conductor: its arms are cut at two lines
  that are not drawn, and its right fist rests on its right arm.
- **Should be:** the hall shows a character's states (arms on; arms in
  lines with the lines), from the same data the fight uses.

### 11. The way up is a narrow thing

- A test plays it (two jumps up into the far line, out of the near line,
  to the side onto the back). No person has played it.
- It works by numbers that are near their limits: two jumps rise about 148
  and the far line needs 117; out of the near line the player has about
  0.45 s to go 60 to the side.
- **Should be:** a way up with room in it. With the arm through the pair by
  the engine (debt 1) the near line can be where the design wants it, and
  the pair can give a body that comes out a push along its normal.

### 12. The portal and trail names omit the encounter

**THE PAIR AND THE LOOK LINES ARE PORTS, 2026-10-12; the view index is
OPEN.** The punch's pair is a held request of the conductor
(`ambition.portal.held_pair`, debt 2). The portal domain's adapter keeps
the pair's record on the scholar (`HeldPortalPairs`, rollback row
`portal.held_pairs`, with its entity map), and the pair is ONE `PortalLink`
of its owner's simulation identity, entry and slot, so the portal core gives
it a stable channel pair of its room (the
[portal identity](../engine/portal-pair-identity-and-aperture.md) core). The
blue and orange are a presentation-only `PortalLook` (channels 24 and 25,
asked for by the module). Each end is `SimId::singleton("held_portal",
"{owner}/{entry}/{slot}/{end}")`. `open_gnu_ton_portals` and the punch half
of `GnuTonPortal` are deleted; the tag is now `GnuTonLookLine`.

- Witness: `ambition_portal2d extension::tests::each_holder_has_his_own_pair_while_he_asks_for_it`.
  Two holders share a room, and a second live instance holds a body with
  the first one's placement id. Then one stops asking, one moves its pair,
  and one is despawned. Five poisons each fail at their own assertion: a
  link without the owner (at "his two ends are a pair"); no release ("only
  B's pair"); no orphan sweep ("a holder that is gone"); no held check ("no
  pair spawned twice"); no move ("B's ends follow").
- A body with no simulation identity gets no pair, and the control is the
  same body with one (`a_body_with_no_identity_gets_no_pair`).
- Under a GGRS sync-test session the real fight opens and closes the pair as
  one pair (`app_it gnu_ton_fight::the_punchs_pair_is_one_pair_across_rewinds`).
  With the `portal.held_pairs` rows removed, the sync test fails with a
  checksum mismatch at frame 233.
- The way up still works through the held pair
  (`the_punchs_pair_is_the_way_up_to_the_giants_back`).

**Still open:** the look lines are a port now (debt 2); the trail
starts are on each fist now, but the view index is keyed by `FeatureId`
alone (debt 6); two live instances of one
room give one placement's end one `SimId` (as they give its body one); two
sessions in one host were not run.

**Evidence, `ed0d1ccd` (2026-10-11):**

- `game/ambition_content/src/bosses/gnu_ton/portals.rs:125-174`
  collects wanted portals by `(LiveRoomInstance, GnuTonLine)` and matches
  existing ones by the same two fields. It does not keep the scholar, mount,
  encounter or module invocation that requested the line.
- Both punch ends use authored channels 24 and 25 (`GnuTonLine::channel`).
  `PortalChannel::partner` pairs these channels; the channels have no encounter
  field. Two same-room encounters cannot have independent pairs.
- A spawned end uses `SimId::singleton("gnu_ton_portal", line.name())`.
  That name omits both room and encounter. A singleton is the wrong identity
  for an entity that can have many live occurrences.
- `light_gnu_ton_lines` repeats the room/line join. `join_gnu_ton_lines`
  stores a trail origin in `LimbTrailStarts: HashMap<String, Vec2>`, keyed by
  `FeatureId`. The renderer then looks it up by `FeatureVisual.id`. The
  presentation request has no explicit encounter or room key.

**Failure:** a second GNU-ton with the same line in a live room shares the
first one's portal record. A second occurrence can also reuse a singleton
name. The trail can select the last origin written under a reused visual id.
The production code proves the collision in the join keys; a two-encounter
runtime witness still needs to be written.

**Should be:** the portal domain owns a *held pair* identified by
`(session occurrence, owning boss/module occurrence, slot)`. Its two ends
carry this pair identity and an end role, not merely a display colour. An
owner may change their locations each tick; a missing request, defeat,
room unload, session replacement or rollback removes or reconstructs the
pair under that owner. `LightLine` has a similarly owned presentation-only
slot. A stable `SimId` comes from the encounter identity and its slot, with
an explicit collision policy. Do not derive authority from an ECS `Entity`,
from a display string, or from an authored colour index.
Colours remain an authored look; they must not be a global pairing namespace.

A module's conductor record remains its own clock, not a hidden portal
registry. The module requests the held pair and its light lines through
ports owned by the portal and presentation domains. The host reconciles the
requests. The game-side `portals.rs` and the global `LimbTrailStarts` writer
then go away. The bond's origin follows the identified limb and its
current line through an owner-scoped presentation view.

**Order:** repair occurrence identity *before* adding another portal-using
boss. The later visual refactor in debts 1-6 can then use the same contract.
An interim repair is acceptable only if it keeps the full owner key and does
not install a second permanent portal manager.

**Required witnesses:** two living GNU-ton encounters in the same live room;
two live instances of one room; two sessions in one host; one encounter
defeated while the other remains; both pairs displaced in a rewind. Assert
pair endpoints, transit destination, stable IDs, trail origin and teardown.
Poison: remove the owner from the join key or use the old singleton id;
the two-encounter test must fail.

### Core correction before the pair port

The October 11 follow-up found a **separate engine problem** in
`ambition_portal2d::link::resolve_portal_links`: it computes channels
from the rank of all live link hashes and end roles from endpoint
position. Adding/moving an unrelated pair can change the channel that
`PortalTransit` or its cooldown holds. The aperture equalizer also
writes a derived minimum into the authored `PlacedPortal.half_extent`,
so a temporarily small end can permanently shrink a moving pair.

This problem is **not** fixed by giving GNU-ton one `PortalLink` per
boss. The [portal-pair design](../engine/portal-pair-identity-and-aperture.md)
owns explicit end roles, source/placement occurrence, derived openings,
legacy authored colour compatibility and the link/transit migration.
GNU-ton debt 12 provides the first two-boss consumer. The smaller
GNU-ton-key repair may land before the full portal API only if it is
explicitly temporary, collision-free and covered by the two-boss test.

### 13. The exit gate reads a corpse, not an encounter outcome

**✅ DONE 2026-10-12.** Rule chosen: the arena is ONE encounter. Its ladder
and `ladder_floor_gate` open when every GNU-ton of the arena is `Cleared` in
the save (`placement_is_cleared`, the same answer as `boss.cleared`). The
members are the GNU-ton placements the arena's spec authors plus any GNU-ton
body live in the room. A corpse is not the evidence. A record retracted by a
replay closes the arena again, and a rewind restores the record (the save is
checksummed rollback state). Two live instances of the arena share their
placements, so a recorded defeat is the outcome of both (a fact of the world).
**Found on the way:** the gate compared the geometry's DISPLAY name
("Ambition: gnu ton arena") with the room id `gnu_ton_arena`. It never ran in
the shipped game: the ladder was never hidden in the fight, and the floor
gate was never removed after a win. It now reads the live room's spec id.
Witnesses: `bosses::gnu_ton::tests::{two_gnu_tons_in_one_arena_open_it_only_when_both_are_beaten_and_it_stays_open,
the_arena_is_known_by_its_room_id_not_by_its_display_name, leaving_arena_resets_state_for_next_visit}`
and, in the real arena, `gnu_ton_fight::{defeating_him_opens_the_arena_and_his_body_going_does_not_close_it,
a_won_fight_leaves_the_arena_open_when_you_come_back}` (both red on the old
gate). Not run: a rollback test across the defeat tick, and a rendered
playthrough.

**Source-confirmed, `ed0d1ccd`:**
`game/ambition_content/src/bosses/gnu_ton/mod.rs:79-106`
computes `boss_defeated = bosses.iter().any(...)` over dead GNU-ton
`BodyHealth` entities in the live room. One defeated boss opens the ladder
while a second one still lives. If the defeated boss entity is later
removed, that evidence disappears; the gate closes on the next overlay
rebuild. The code does not store that the encounter has completed. A
single-boss happy-path fight does not witness either case.

**Choose one explicit game rule:** if the arena is a single encounter with
several participants/bosses, open only after *all required members* complete;
if each boss has its own exit, name each encounter's gate owner and attach
that gate to its encounter. Do **not** replace `any(dead)` mechanically with
`all(dead)`; an empty or despawned query would still be ambiguous.

**Engine contract:** use the existing encounter/session-scoped terminal
state, or extend that domain with a typed terminal fact if it lacks one.
Room geometry projection consumes the fact; boss ECS lifetime is not the
history of arena completion. Decide explicitly whether a completed arena
persists through room residency changes, checkpoint restore, and New Game.
Keep this separate from portal-pair identity (debt 12) while testing both
against the same two-encounter fixture.

**Writers' acceptance tests:** two live GNU-ton bosses, defeat only A (gate
closed if they share an arena), defeat B (gate open), despawn both (gate
still follows the recorded outcome), unload/re-enter, and restore a
pre-defeat snapshot (gate closed). If the intended design is *one exit per
encounter*, substitute two individually named exits and assert isolation.
Poison: restore `any(!health.alive())`, which must fail the two-boss test.

### 14. An effect's frame is stretched, and each effect fades from its start

Found 2026-10-11 with the fist of the fighter's smash attacks
(`gnu_ton_vfx`).

- `draw_effect_clip` (`crates/ambition_render/src/fx.rs`) gives the sprite
  one `custom_size`, a square, for its life. Each effect sheet is published
  with trimmed frames, and a trimmed frame is not the square's shape: it is
  stretched to it. A character's sprite does not have this fault
  (`CharacterAnimator::current_render` gives the size and the anchor of
  each trimmed frame); the effect road does not call it.
- `update_effects` sets `alpha = 1 - t` for each effect: an effect is at
  half strength at the middle of its clip, whatever its frames say. The
  effect sheets fade in their own frames already.
- What was done: the fist's sheet is published with whole frames
  (`whole_frames` in `_character_vfx_common.publish_catalog`), so it is not
  stretched. And a sheet now declares who fades its effects
  (`FxSheet::fade`): the fist's says `InItsFrames`, each other sheet says
  `OverItsLife`, which is the rule of today.
- Why it was not fixed: the size of each effect in the game today is the
  stretched size, and its look is the faded look. The right rule (the
  trimmed frame in its own shape; the frames own the fade) makes each
  effect smaller and stronger. That needs a look at each effect, with the
  hit sparks first: they are combat feel.

## An order

1. Determine the arena-terminal policy and record its source of truth (13).
   The gate should not depend on a dead body's retained ECS entity.
2. Correct the pair/line/trail occurrence identity (12). This blocks a
   second instance of the encounter and is independent of later art work.
2. The actor road reads `looping` (7). Small; one copy of a row goes.
3. A person plays the way up (11), and its numbers get room.
4. A limb "on its host" (5), and the trail (6).
5. The port for a pair and a line (2), with the decision of 3 and the
   occurrence contract of 12. Delete the interim reconciler.
6. The lines from the sheet (4), with step 1 of the other document.
7. The arm through the pair by the engine (1), with angled portals.
