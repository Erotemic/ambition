# Moveset reviews — the maintainer's feedback, in one place

Jon asked for this page: *"We should probably keep track of moves I've explicitly
authored, or decided I like, because everything else can be polished or modified
to an LLM's heart's content."*

Jon's own words stay in the moveset files, beside the move each one governs. This
page is the roster-wide view. It answers three questions:

1. Which moves did Jon author or explicitly approve? A polish pass must not
   silently rewrite them.
2. Which fighters has he never spoken about? Those are an agent's to change
   freely.
3. What has he asked for that is not done yet?

List 2 matters as much as list 1. Jon: *"The entire point is that this game is
demoing the capabilities of an LLM to make a game, and every decision or explicit
authoring choice I make takes away from that claim."* Attributing an agent's move
to him weakens that claim as much as overwriting one of his.

## 1. Moves Jon authored or explicitly asked for

⛔ A polish pass may tune these. It must not replace them. Where he called the
execution weak, improve the execution, not the idea.

| fighter | move | what he asked for | note |
|---|---|---|---|
| Pirate Admiral | up-B | *"their up-b should summon a burning flying shark that they can mount and ride"*; *"There is no hurtbox on this up-b, it's purely a mobility special"*; *"maybe 5 seconds is too long, but that's where I want it right now"* | the duration is his provisional number |
| Pirate Admiral | side-B | *"should briefly equip the lasergun sword and fire a lasersword projectile in the left/right direction the side b was directed towards"* | |
| Performer | down-B | the trapdoor: *"a trapdoor opens and the character descends underground"*, *"they can move for up to the timelimit of the move (3 seconds)"*, *"she should be able to pop up at any time from it in a big firework display that damages whoever is on top or above"*, *"a puff of smoke (like a real play would use to disguise going through a trap door)"* | the most heavily reviewed move in the game |
| Performer | up-B | *"She doesn't teleport up, she gets lifted up by the wire"*; *"It is not a teleport and should not get the teleport sound"*; *"her up-b uses the trap door, and I don't think it should"* | |
| Performer | neutral-B | *"performer gets sing."* | |
| Performer | tilts, smashes, aerials | *"attacks rarely ever feel like they connect"*; larger, longer-lived hitboxes; posing and sequencing closer to Smash Ultimate | first pass shipped; keep evaluating feel in the Smash demo (`normal_contact_windows_match_the_authored_light_and_pose_clock`) |
| Director | up-B | *"Mewtwo / Palutena / Zelda style teleports"* | his idea; execution *"isn't great right now"*; polish invited, low priority |
| Director | side-B | *"I want the director to have side-b be the pk-thunder style 'mind' attack."* | |
| Officer | side-B | *"we should also polish the officer and give him a side b that pulls out and shoots a gun."* | |
| Officer | shield | *"this is a push, not a hit"* | |
| Projectile Polygon | neutral-B | *"This should have parity with samus / mewtwo 'b', so that means it needs to be able to store a charge and fire at different sizes."* | |
| Projectile Polygon | side-B | *"I think the projectile polygon should be able to use her ponytail as a boomarang for her side-b."* | |
| Projectile Polygon | down-B / down-smash | *"The projectile polygon should poop a bomb onto the stage… The bomb should detonate in 4 seconds"*; *"probably the remote mine as their down smash."* | |
| Pointed Polygon | up-B (`polygon_rising_edge`) | *"Pointed extends her swords approximately horizontally. The attack volume should form a broad disk / horizontal spinning envelope around her"*, plus his four-case list of what the multihit must catch | his cases are quoted in `the_rising_spin_gathers_from_either_side_and_stops_somewhere`; he allowed *"it is acceptable to fake the spin by repeatedly flipping the sprite horizontally"* |
| Pointed Polygon | down-B counter (`polygon_riposte`) | *"Swordies will get a counter."* | a counter that strikes back (`smash.riposte_strike`) |
| Polygons | up-air, basic attacks | he helped with these; *"they aren't the most polished things in the world"* | polish invited |
| Robot | up-B | *"The robot has a blink up-b, similar to how it works in ambition in terms of the animation."* | |
| Alice | up-B | *"up b opens a portal under him, and a portal at the very top of the stage"*; *"we can even exercise angled portals with directional input on the up b"* | |
| Goblin | Limit | *"Give the limit ability to the goblin maybe? […] And give whoever gets the limit meter some move they can use when it fills."* | he called the numbers *"just an example, you can tweak things"* |
| PCA | ranged | *"PCA needs to shoot a glider, but beyond that i don't really care."* | |
| Patent Clerk | Witch-Time | *"no take-backs"* | |
| Oiler | geyser | *"so the column reads as continuous rather than as one puff"* | |

## 2. Fighters with no maintainer input — an agent's to change freely

The guard decides this list by scanning each fighter's moveset files (`*_moveset.rs`
and `data/movesets/*.ron`) for the maintainer's name:

`bob`, `carl_stargan`, `cellular_automaton`, `emmy_noether`, `medic`,
`ninja_shadow_oni_leader`, `pugnacious_polygon`, and
`imperfect_cellular_automaton` (which wears `cellular_automaton`'s table).

These are the demonstration: each design is an agent's decision. A fighter that
moves off this list means Jon has spoken about it, and the polish rules change.

## 3. Standing feedback that is not about one move

| what he said | scope | rule |
|---|---|---|
| *"we have a lot of characters with boring specials"* | roster-wide | Measure with `cargo test -p ambition_content --lib the_census_of_specials -- --nocapture` (shared rule: `expressive_reasons`). Zero plain specials is not a target; a plain strike is a legitimate design. |
| *"I'm biasing towards making moves too powerful to start"* | tuning bias | Prefer over-strong over timid on a first pass. |
| *"We will balance later"* | roster-wide | Balance is not the current job. |
| *"we can tune who the moves belong to later"* | roster assignment | Every roster slot is provisional. |
| *"almost every move isn't great or polished, they all need a lot of work. But I think we need the elegant way to express them, and probably a good way to iterate on them before we put too too much effort into it"* | the whole programme | Expression and iteration come before polish. |

## 4. Open — asked for and not satisfied

| what he asked for | why it is open |
|---|---|
| Director's up-B execution | shipped; he says it *"isn't great right now"*; polish is an agent's, low priority |
| Polygons' up-air and basic attacks | he calls them unpolished |

## The manifest

This block is the contract; the prose above is for people. The guard compares
these slugs with the code as sets. Every moveset file must appear in exactly one
list, so a new fighter must be classified.

<!-- reviewed-fighters: alice, director, goblin, officer, oiler, patent_clerk, performer, pirate_admiral, player_robot, pointed_polygon, projectile_polygon -->
<!-- free-fighters: bob, carl_stargan, cellular_automaton, emmy_noether, imperfect_cellular_automaton, medic, ninja_shadow_oni_leader, pugnacious_polygon -->

## The guard

`scripts/tests/test_moveset_reviews_are_the_single_source_of_truth.py` checks
both directions:

1. every moveset file that quotes the maintainer is in the reviewed list;
2. every fighter in the free list has no maintainer input in its files.

⛔ The second direction is the one that can hurt somebody. A wrong entry in §2
tells a polish pass that one of Jon's own moves is nobody's.
