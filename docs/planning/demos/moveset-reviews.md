# Moveset reviews — the maintainer's feedback, in one place

Status: **OPEN** — a living index, not a receipt.

Jon, 2026-09-05, asking for this page:

> **"We should probably keep track of moves I've explicitly authored, or decided
> I like, because everything else can be polished or modified to an LLM's heart's
> content. […] I think we have some concept of move reviews, but I haven't made
> any. You could seed those reviews from what I've said in chat, so there is a
> single source of truth for my moveset feedback."**

## Why this page exists, and what it is NOT

⛔⛔ **THE FEEDBACK WAS NEVER LOST — IT WAS UNREADABLE AS A WHOLE.** Measured
2026-09-05: eleven of nineteen shipped moveset files carry Jon's words verbatim,
and `performer_moveset.rs` alone carries **41** quoted fragments. Every one of
them sits beside the move it governs, which is the right home for a rule and the
wrong home for a roster-wide question. Nobody could answer *"which moves are
mine?"* without reading nineteen files.

⇒ **The quote stays in the code; the VIEW lives here.** This page does not
duplicate the rules — it answers the three questions the code cannot:

1. **Which moves did Jon author or explicitly approve?** Those may not be
   silently rewritten by a polish pass.
2. **Which fighters has he never spoken about?** Those are an agent's to change
   freely, and saying so is the point — *"everything else can be polished or
   modified to an LLM's heart's content."*
3. **What has he asked for that is not done yet?**

⭐ **AND THE CLAIM THIS SERVES IS JON'S OWN, WHICH IS WHY LIST 2 MATTERS AS MUCH
AS LIST 1**: *"The entire point is that this game is demoing the capabilities of
an LLM to make a game, and every decision or explicit authoring choice I make
takes away from that claim."* ⇒ An accurate record of what he did NOT decide is
part of the demonstration. Attributing an agent's move to him weakens the claim
exactly as much as overwriting one of his does.

## 1. Moves Jon authored or explicitly asked for

⛔ **A POLISH PASS MAY TUNE THESE BUT MAY NOT REPLACE THEM**, and where the
execution is weak he has said so himself — that is an invitation to improve the
execution, not to change the idea.

| fighter | move | what he asked for | state |
|---|---|---|---|
| **Pirate Admiral** | up-B | *"their up-b should summon a burning flying shark that they can mount and ride"*; *"There is no hurtbox on this up-b, it's purely a mobility special"*; *"maybe 5 seconds is too long, but that's where I want it right now"* | shipped; duration is explicitly his provisional number |
| **Pirate Admiral** | side-B | *"should briefly equip the lasergun sword and fire a lasersword projectile in the left/right direction the side b was directed towards"* | shipped |
| **Performer** | down-B | the trapdoor: *"a trapdoor opens and the character descends underground"*, *"they can move for up to the timelimit of the move (3 seconds)"*, *"she should be able to pop up at any time from it in a big firework display that damages whoever is on top or above"*, *"a puff of smoke (like a real play would use to disguise going through a trap door)"* | shipped, and the most heavily reviewed move in the game — he corrected it at least six times |
| **Performer** | up-B | *"She doesn't teleport up, she gets lifted up by the wire"*; *"It is not a teleport and should not get the teleport sound"*; *"her up-b uses the trap door, and I don't think it should"* | shipped |
| **Performer** | neutral-B | *"performer gets sing."* | shipped |
| **Author** | up-B | *"Mewtwo / Palutena / Zelda style teleports"* — his idea, and he has said the execution *"isn't great right now"* and that polishing it is an agent's to do, low priority | shipped, **polish invited** |
| **Author** | side-B | *"I want the author to have side-b be the pk-thunder style 'mind' attack."* | shipped |
| **Officer** | side-B | *"we should also polish the officer and give him a side b that pulls out and shoots a gun."* | shipped |
| **Projectile Polygon** | neutral-B | *"This should have parity with samus / mewtwo 'b', so that means it needs to be able to store a charge and fire at different sizes."* | shipped |
| **Projectile Polygon** | side-B | *"I think the projectile polygon should be able to use her ponytail as a boomarang for her side-b."* | shipped |
| **Projectile Polygon** | down-B / down-smash | *"The projectile polygon should poop a bomb onto the stage… The bomb should detonate in 4 seconds"*; *"probably the remote mine as their down smash."* | shipped |
| **Pointed Polygon** | ~~neutral-B~~ **up-B** | *"Pointed extends her swords approximately horizontally. The attack volume should form a broad disk / horizontal spinning envelope around her"* — plus his own four-case list of what the multihit must catch | shipped; he also authorised the cheap fix: *"it is acceptable to fake the spin by repeatedly flipping the sprite horizontally"*. ⚠ **SLOT CORRECTED 2026-09-06: it is the UP-B, not the neutral-B.** The disk is `polygon_rising_edge`, a `multihit` whose four cases are `the_rising_spin_gathers_from_either_side_and_stops_somewhere` — Jon's list quoted verbatim in the test, with the far points asserted too so it cannot pass on a stage-wide box, and a separate test refusing a sideways offset because *"a spin has no front"*. His neutral-B (`polygon_point`) is an ordinary forward thrust and always was. ⇒ I went looking for a missing disk on the strength of this label and found the feature fully built one slot over: a row that names the wrong SLOT sends the next reader to rebuild something that already exists |
| **Polygons** | up-air, basic attacks | he helped with these; *"they aren't the most polished things in the world"* | **polish invited** |
| **Robot** | up-B | *"The robot has a blink up-b, similar to how it works in ambition in terms of the animation."* | shipped |
| **Alice** | up-B | *"up b opens a portal under him, and a portal at the very top of the stage"*; *"we can even exercise angled portals with directional input on the up b"* | shipped, both halves |
| **Goblin** | Limit | *"Give the limit ability to the goblin maybe? […] And give whoever gets the limit meter some move they can use when it fills."* | shipped — and he called the numbers *"just an example, you can tweak things"* |
| **PCA** | ranged | *"PCA needs to shoot a glider, but beyond that i don't really care."* | satisfied |
| **Swordies** | counter | *"Swordies will get a counter."* | ⛔ **"shipped" WAS FALSE FOR THE SWORDIE, and a census caught it 2026-09-06.** The counter MECHANIC shipped — on the stand-ins' `riposte` and the Author's second draft — and this row was marked from the mechanic rather than from the fighter. `pointed_polygon` was five-for-five BARE specials: not one carried a technique. ⇒ **NOW SHIPPED FOR REAL** as `polygon_riposte`, his grounded down-B, replacing the `polygon_low_arc` swipe. ⭐ It needed a new answer to exist: all six shipped counters respond with a grab, a teleport, a sleep, a heal, an absorb or a slow, and NONE of them hits back — `counter_move` builds a stance with no volumes, so the genre's plainest counter was inexpressible. `smash.riposte_strike` is that missing answer and he is its first customer. ⚠ The lesson for this page: a row that reads "shipped" from the MECHANISM says nothing about the FIGHTER, and `the_census_of_specials_that_carry_no_technique` is now the instrument that tells the difference |
| **Officer** | shield | *"this is a push, not a hit"* | shipped |
| **Patent Clerk** | Witch-Time | *"no take-backs"* | shipped |
| **Oiler** | geyser | *"so the column reads as continuous rather than as one puff"* | shipped |

## 2. Fighters with NO maintainer input — an agent's to change freely

Measured 2026-09-05 by scanning every `*_moveset.rs` for the maintainer's name:

`archetype`, `bob`, `carl_stargan`, `cellular_automaton`, `emmy_noether`,
`medic`, `ninja_shadow_oni_leader`, `pugnacious_polygon`.

⭐ **These are the demonstration.** Bob's rivet gun, Carl's homing slingshot, the
ninja's counter-with-smoke, the pugnacious parasol — every one is an agent's
design decision, and Jon's framing says that is the point rather than a gap.

⚠ **A fighter moving OFF this list is a real event**: it means the maintainer has
spoken about it, and the polish rules change. The guard below enforces the list
against the code so it cannot go quietly stale.

## 3. Standing feedback that is not about one move

| what he said | scope | state |
|---|---|---|
| *"we have a lot of characters with boring specials"* / *"Many have boring specials"* | roster-wide | ⇒ the expressiveness census now stands at **18 expressive, 1 plain**; the one plain entry is `theorem_chain`, the duel arena's minimal two-hit demonstrator, which is plain deliberately. ⚠ **THAT COUNTS FIGHTERS, AND A SECOND CENSUS COUNTS SPECIALS — they disagree usefully.** `the_census_of_specials_that_carry_no_technique` (2026-09-06, `cargo test -p ambition_content --lib the_census_of_specials -- --nocapture`) walks every `special*` verb in `tables()` and asks what each move CARRIES: a technique key (an `Effect` on the timeline or a window's `sustain_effect`), and failing that any other authoring — extra hit windows, a charge, a start impulse, timeline events. ⇒ At the fighter level `pugnacious_polygon` counted as EXPRESSIVE while all five of its specials were single hitboxes, which is precisely the state Jon's sentence is about. ⛔ And the finer number needs its second half or it misleads the other way: **65 of 88 specials carry no technique, and only 4 carry no other authoring either** — the Perfect Cellular Automaton's down-B is a three-pulse multihit with autolink volumes and wants no technique at all. Started at 5 plain on 2026-09-06 and stands at **4** (`author_point`, `officer_disperse`, `polygon_point`, `polygon_brawler_ground_slam`) after the brawler's haymaker gained a charge |
| *"I'm biasing towards making moves too powerful to start"* | roster-wide tuning bias | standing — prefer over-strong over timid on a first pass |
| *"We will balance later"* | roster-wide | standing — balance is explicitly NOT the current job |
| *"we can tune who the moves belong to later"* | roster assignment | standing — every roster slot is provisional |
| *"almost every move isn't great or polished, they all need a lot of work. But I think we need the elegant way to express them, and probably a good way to iterate on them before we put too too much effort into it"* | the whole programme | **the current priority ordering**: expression and iteration BEFORE polish |

## 4. Open — asked for and not satisfied

| what he asked for | why it is still open |
|---|---|
| the **Author's up-B** execution | his idea, shipped, and he says it *"isn't great right now"*. Polishing it is an agent's to do and he marked it low priority |
| the **polygons' up-air and basic attacks** | he helped author them and calls them unpolished |
| three item icons — the **mine**, the **bomb**, the **ponytail** | Jon: *"Worth three icons"*. ✔ **DONE 2026-09-06.** ⛔ Two claims in this row were wrong and both were cheap to check. They did NOT "draw the placeholder quad": all three were registered in `items/held_visuals.rs` and BORROWED real art (`gauntlet_bomb.png`, `mark_beacon.png`, and the javelin for the tress, which the code's own comment called "honestly a placeholder"). And the generator is `targets/icons/**item_icons**.py`, not `hud_icons.py` — a neighbouring module whose own docstring says it is "distinct from `item_icons` next door". ⇒ The submodule blocker was real when written and is gone: the pointer matches HEAD after the epoch reset. Landed: three drawers + `HELD_ITEM_ICON_SPECS` in the renderer, installed by the `write_gauntlet_props` call `scripts/regen/sprites.sh` already makes, `held_visuals.rs` repointed (the ponytail's extent went square — a borrowed `44×6` is the javelin's shape, not hers), and `check_held_item_props_are_rendered.py` holds the two lists together with 5 poisons run red |

## The manifest

⛔⛔ **THE PROSE ABOVE IS FOR PEOPLE; THIS BLOCK IS THE CONTRACT.** A guard that
matched fighter names out of the prose would pass on a table row that mentions a
fighter in passing and fail on one that spells a name differently — so the two
lists are stated exactly once, as slugs, and the guard compares them against the
code as SETS. Every `*_moveset.rs` must appear in exactly one of them, which is
what forces a NEW fighter to be classified rather than silently defaulting to
"free to change".

<!-- reviewed-fighters: alice, author, goblin, officer, oiler, patent_clerk, performer, pirate_admiral, player_robot, pointed_polygon, projectile_polygon -->
<!-- free-fighters: archetype, bob, carl_stargan, cellular_automaton, emmy_noether, medic, ninja_shadow_oni_leader, pugnacious_polygon -->

## The guard

`scripts/tests/test_moveset_reviews_are_the_single_source_of_truth.py` holds two
claims, and the second is the one that protects Jon's actual concern:

1. every moveset file quoting the maintainer has a row on this page;
2. every fighter this page lists as **free to change** genuinely has no
   maintainer input in its file.

⛔ **The second direction is the one that can hurt somebody.** A missing row makes
this page incomplete; a WRONG entry in §2 tells a polish pass that one of Jon's
own moves is nobody's, which is the single failure this page exists to prevent.

---

## Reviewer's receipt — exploration session, 2026-09-05

Per the review protocol (each session re-derives one claim when the other lands
architecture), run against `1258f0939`.

✔ **RE-DERIVED BY RUNNING: "a NEW fighter is classified."** Planted an
unclassified `*_moveset.rs` file (named `zzz_probe`) beside the real ones in
`game/ambition_content/src/` and the guard went red naming it — *"in code, not on the page: ['zzz_probe']"* — then green again
once removed. The partition is real and the failure message says what to do,
which is the half that usually rots.

⚠ **COULD NOT RE-DERIVE: "`performer_moveset.rs` alone carries 41 quoted
fragments."** Not a refutation — a definition gap. Counting quote characters in
that file gives 243, and lines matching a quote-shaped pattern give 133; neither
lands on 41, so the number rests on a narrower rule for what a "quoted fragment"
is that only its author holds. ⇒ Per *MEASUREMENT SCRIPTS ARE COMMITTED*, the
count wants the few lines that produced it beside the guard, or the sentence
wants to say "counted by hand". The 11-of-19 file count has the same shape.
⭐ The claim is plausible and nothing here contradicts it; it is the
REPRODUCIBILITY that is missing, and that only matters because the page's whole
purpose is to be the single source of truth for these numbers.
