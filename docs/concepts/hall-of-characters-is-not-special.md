---
id: hall-of-characters-is-not-special
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-30
related_docs:
  - docs/concepts/invariants.md
---

# The Hall of Characters is not a special case

`hall_of_characters` stages ~144 characters in one room. It is the most expensive
room in the game and it will keep getting more expensive, because it is a **dual
purpose stress test and exhibition** (Jon):

> *"I feel like you are treating the hall as special. It is not. It is a dual
> purpose stress test and exhibition. Eventually we are going to give all those
> characters normal brains, or at least have the option for it. The only thing
> special about it is that its generated."*

## ⛔ When it is slow, do not fix the Hall. Fix the engine.

Every one of these has been proposed, and every one is wrong:

* **Render it at a lower texture-quality variant** — explicitly rejected. It
  makes the game look worse to make a number better. The only redeemable form is
  real LOD (cheap asset first, upgraded in place as the full one streams in), and
  Jon does not want that opened yet: *"its very easy to do that wrong and have it
  just look sloppy"*.
* **Cap, special-case, or exclude it from a load path** — it is an ordinary room
  reached by an ordinary loading zone. A budget that happens to exclude it is a
  budget that will not protect the next big room either.
* **Treat its cost as acceptable because it is a debug/exhibition room** — it is
  content, it will gain brains and behaviour, and a player walks into it.

The correct response to "the Hall is slow" is a general engine fix that any room
with many actors benefits from — and the Hall is the room that PROVES it, which
is half of what it is for. If a load genuinely takes a hot second, the answer Jon
asked for is *"just have a loading screen"*, not less content.

## The one thing that IS special about it

It is GENERATED, by
`tools/ambition_ldtk_tools/.../generate_hall_of_characters.py` from the character
catalog, so it grows on its own whenever the cast does. **Never hand-edit the
level.** That is the only handling it needs.

## What the Hall is for (Q85, 2026-10-04)

The Hall is a visual showcase, a population and stress test, an asset and
residency test, an animation and rig test, and an AI/body/profile composition
test. Its actors may be non-interactive and may lack dialogue for now; adding
dialogue where it fits is future content work, not a blocker for the Hall.

Its population policy is one rule for every showcase actor: the generator
writes `brain_override: "stand_still"` on each `NpcSpawn`, and every actor
stays where it was placed. A body that moves in spite of that is an engine
defect in how that body's motion model obeys its driver, not a reason to
special-case the Hall or that character.

**How the crawler obeys (2026-10-08).** `npc_puppy_slug` is the catalog's
one `surface_walker: true` row, so it gets the `AdhesiveCrawler` motion model.
The crawler used to advance at its policy's pace whatever its driver
commanded, so the one slug in the Hall crawled. Now `step_crawler`
(`movement/adhesive_crawler.rs`) advances only on the commanded lateral axis,
as every other motion model does; with no command it stays seated on its
surface, and it falls if that surface goes. Witnesses:
`a_crawler_given_no_command_stays_where_it_clings` (core) and
`game/ambition_app/tests/a_still_brain_keeps_its_body_still.rs`, which steps
the generated Hall and finds every actor where it settled, with a patrolling
slug as the control.

## Where this has already bitten

A launch stutter came from neighbour prefetch: the hub has many exits, so
prefetch decoded the Hall's whole cast. The fix capped the prefetch fan-out, an
engine change that every room with many exits benefits from. It did not exempt
the Hall from prefetch.
