# A capability that EXISTS, WORKS, and has nobody using it

**Tags:** `false-absence`, `adoption-gap`, `silent-degradation`,
`agent-verification`, `roadmap-staleness`

## The shape

A row, a review, or an agent says *"the engine cannot do X."* The grep for X
finds it. The row is corrected and the work begins — on **connecting** X, which
is right — and everyone moves on.

⛔ **The step that keeps getting skipped is the second one: COUNT THE ADOPTERS.**

```text
grep for X          → "it exists"          ← where investigations stop
count who calls X   → "…and nothing uses it" ← where the actual work is
```

Those are different findings and they imply different projects. *"The engine
cannot size a body from its authored rect"* is an engine task. *"It can, and one
caller does"* is an adoption task, and the fix lives somewhere else entirely.

## Six instances, one session (2026-08-09)

| capability | what it does | adopters |
|---|---|---|
| `authored_body_pixel_size` | the only function that distinguishes the DRAWING (alpha bbox — hat, arms, sparkles) from the BODY; **refuses** to answer for a measured bbox | **1 caller**, the player-robot lineage |
| `EnemySpawn.character_id` | art identity authored separately from the level's label — added 2026-08-06 *because* renaming a character silently un-arted every level placing it | **0 of 65** authored instances, in every world |
| `BodyMetrics::body_pixel_parts` | the disjoint-piece body union (a boss with a head, torso, two hands) | **0 of 190** shipped sheets emit the field |
| `ActorConfig::sprite_override_npc_name` | lets a body wear art its display name does not name | 1 driver, gated on a **hardcoded** `name != "Kernel Guide NPC"` |
| `SelectCursor::move_to` | source-agnostic cursor placement on the smash select screen | **4 drivers** (initial, mouse, stick-snap, one more) and **no touch** — so "a finger cannot tap a portrait" was an adoption gap, not an engine one |
| `resident_tiers()` | reports which texture tiers are physically in memory | ⛔ **0 production callers** — two test sites. It was *wrong* (it reported the request, not the pixels) and fixing it **changed no shipped behaviour at all** |

Two more the same day were capabilities an agent (me) proposed to BUILD and which
already existed: the contextual touch-button label resolved through `ControlSlot`
(shipped), and `scripts/mirror_assets_for_worktree.py` (I spent a day briefing
workers to work around the problem it solves).

## ⛔ Why it survives: every one degrades SILENTLY, by design

- the display-name join still resolves art, so nobody notices `character_id` is
  unset;
- the alpha bbox still produces a *box*, so nobody notices it is the wrong box;
- the static bbox still sizes a *quad*;
- the hardcoded verb still *labels* the button.

⇒ **a capability with no adopters looks exactly like a capability that is
working.** Nothing is red, nothing warns, and the symptom surfaces months later
as a content complaint — *"Iron Mary shoots fireballs"*, *"the collision box is
larger than the sprite"* — which is then investigated as a content bug.

⭐ **and the unused one is often the CORRECT design**, already written down by
someone who understood the problem. `authored_body_pixel_size`'s doc explains the
whole defect in one sentence: *"it is the extent of the drawing, hat and
outstretched arms included, and using it as a body is how a collision box ends up
1.28× the character inside it."* The engine knew. Nothing called it.

## The check, in two commands

```sh
# 1. does it exist?  (finds absence reliably; presence is weak evidence)
grep -rn "<capability>" --include=*.rs crates/ game/ | grep -v "fn <capability>"

# 2. who uses it?  ← the finding
#    for AUTHORED data, PARSE the files — `grep -r` skips gitignored trees
#    and symlinked assets, and both are where content lives here.
```

⚠ **the count is the finding, and it decides the shape of the work.** *"2 of 190
declare `authored_body`"* reads as a 188-file content project. The real number
was **34**, and the real work turned out to be **13 rig configs plus a regen** —
a wrong count sends the next agent at the wrong deliverable.

⚠ **and adding the first adopter can EXPOSE a second defect.** Authoring
`EnemySpawn.character_id` would have un-arted those bodies, because the renderer
keys its sheet lookup on the display name — the exact difference the field exists
to allow. The zero adopters were hiding a broken consumer, not just an idle one.

## ⛔⛔ The worst case: an unadopted capability that is also WRONG

`resident_tiers()` reported the tier that was *requested* rather than the tier
physically in memory — a real defect, found by review, and fixed with a test
whose red output named two resident tiers where the function claimed one.

**It has zero production callers.** Two test sites and nothing else.

⇒ **the fix changed no shipped behaviour whatsoever**, and it would have been
entirely natural to report it as *"the memory accounting is fixed"*. It is not;
nothing was accounting for anything. What actually landed is that the *builders*
now record the truth, so that a future diagnostic built on this reads something
real.

⭐ **this is the failure mode that makes the adopter count worth running BEFORE
the fix, not after**: a broken function with no callers is indistinguishable from
a broken function with many, right up until you write the release note. The
worker here caught it and said so unprompted — *"worth stating plainly so nobody
reports a memory fix that hasn't happened."*

⚠ **and it inverts the usual priority.** For an unadopted capability the
interesting work is almost never the fix; it is deciding whether anything should
call it at all. Fixing first is cheap and feels like progress, which is exactly
why it gets done instead.

## The tell, for an agent

⭐ **you are about to design a WORKAROUND.** A workaround is a confession that the
obvious thing is missing, which is precisely when to check whether it is.
*"I'll have them avoid needing X"* ⇒ **grep for X first.**

## Related

* [`enumerate-one-way-validate-another-2026-08-08.md`](enumerate-one-way-validate-another-2026-08-08.md)
  — the population half of the same disease: a checker that enumerates one way
  and validates another silently reports on a set it never covered.
* [`one-question-two-checkers-only-the-first-runs-2026-08-08.md`](one-question-two-checkers-only-the-first-runs-2026-08-08.md)
  — a capability with a reachable copy and an unreachable one.
