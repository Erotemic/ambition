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
purpose stress test and exhibition** — Jon, 2026-07-30:

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

## Where this has already bitten

The 2026-07-30 launch-stutter investigation: the hub has 21 exits, so neighbour
prefetch was covering 162 characters / 357 MP of sprite decode. The fix was to
cap the prefetch fan-out (162 → 10) — an engine change that every room with many
exits benefits from — not to exempt the Hall from prefetch.
