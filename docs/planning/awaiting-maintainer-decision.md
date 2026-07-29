# Awaiting a maintainer decision

Questions that reached the point where the next step is **Jon's judgement, not
more engineering**. Each one was scoped far enough that the choice is real and
the work after it is small; none is blocked on unknowns.

This is the counterpart to [`maintainer-decisions.md`](maintainer-decisions.md),
which records what Jon HAS decided. Nothing here is a decision. When one is made,
it moves there and this row is deleted.

⚠ **These are not bugs, and they are not agent preferences waiting for a rubber
stamp.** Every one is a case where two defensible answers exist and picking
between them is authoring or product design. Where I have a recommendation it is
marked as such and it is weak — the reason the row is here is that I should not
be the one choosing.

---

## 1. What do the three doors in the hub say? (queue Z′13)

**What you would see.** Standing in the central hub, the door nameplates read
`military_tower_door`, `hall_of_bosses_door`, `pirate_cove_door`. These survive a
capture with developer overlays off, so they are not debug text — a nameplate
renders `zone.name`, and the authored LDtk zones set `name` to the id string.

**Why it is yours.** [[feedback-entity-id-matches-label]] says the fix for
label/id drift is to rename the IDS to match the label, in one commit across LDtk
+ code + tests + docs, and to *surface it rather than silently introduce a third
name*. Picking what three doors say to a player is authoring.

**The engine is already ready.** `DoorNameplateSource` carries `id` and `label`
separately; a real label needs no code change.

**What I need:** three strings. ("Military Tower", "Hall of Bosses", "Pirate
Cove" are the obvious readings, but "obvious" is exactly the kind of guess this
file exists to stop.)

---

## 2. Should a prompt advertise a verb that resolves to nothing? (queue Z′4)

**What you would see.** The versus fighting stage offers `Interact (F)`. There is
nothing on that stage to interact with.

**Why the obvious fix is wrong.** `derive_action_scheme` grants Interact
unconditionally and its comment says why: the actual prompt (Talk / Open / …)
resolves against nearby interactables at press time. Under that design this is
not a versus bug — it is prompt noise on EVERY stage, and gating it for fighters
would fix the symptom where it happened to be noticed and leave it everywhere
else.

**The actual question.** How much should a control prompt promise? Two coherent
answers:

* **The prompt is a KEYMAP.** It says which buttons exist; whether a press does
  anything is the world's business. Today's behaviour, and cheap.
* **The prompt is an OFFER.** It shows only verbs that would do something, which
  means it must know about nearby interactables — a real change, and it makes the
  legend flicker as you walk past things.

**Layering note for whoever takes it:** `derive_action_scheme` lives in
`ambition_characters`, below both callers (`ambition_actors::action_scheme` and
`ambition_sim_view::control_prompt`), so no match concept is visible to it. A
per-body marker from `ambition_characters` is the only shape both callers can
read — the route `ScriptedControl` already takes.

---

## 3. Portraits: build the resolver, or delete the field? (queue Y″6)

**The state.** `CharacterDefinition.portrait` is carried faithfully through
preparation and read by nothing. The CONCEPT has a real consumer —
`ambition_content::presentation::dialog` resolves a speaker portrait through a
`portrait_catalog` override, then `CharacterCatalog::portrait_ref`, then a clip
from the portrait registry.

**Why it cannot just be wired.** `CharacterDefinition.portrait` declares a
portrait TARGET — a name resolved at preparation against the composition's
vocabulary. `portrait_ref` yields concrete paths (`image`, `manifest`,
`default_clip`). **There is no target → portrait-art resolver anywhere**, so the
authored target has nothing to resolve through.

**Two honest options:**

* **Build the resolver.** The sheet path already has exactly this shape:
  `SheetTarget` resolves a name to a manifest. This makes a registered character
  able to bring its own portrait, which is what a character-definition seam is
  for.
* **Delete `CharacterDefinition.portrait`** and let the catalog own portraits
  outright.

*Weak recommendation: build the resolver*, because a character another game
registers should be able to speak with its own face without editing our catalog —
but that is an engine-ambition argument, and if portraits are a hub-dialogue
feature rather than a character-authoring one, deleting is cleaner.

⛔ **Not an option:** "wiring it" by copying the catalog's concrete paths onto the
definition. That makes two places declaring the same art, which is the split this
whole campaign has been removing.

---

## 4. Keep or discard `wip/versus-reserved-surround`? (queue Z′10)

**The branch** (`c90cff205`) reserves a top band on the versus stage and draws
the scoreboard into it, instead of over the arena.

**It was parked as unverifiable** — a screenshot could not show a viewport
policy. **That changed:** the Mary-O route now captures with visible 4:3
letterboxing and its SCORE/COINS/TIME/LIVES HUD drawn in the reserved surround,
which is the same mechanism. So someone can apply the branch, capture
`versus_gameplay`, and SEE it.

**The question is about the stage, not the code:** does a reserved top band read
better than a HUD over the arena, or does it waste fighting width?

⚠ **Note what it no longer fixes.** It was written for a HUD/legend overlap that
turned out to be two other bugs (an unresolved layout and the touch overlay
drawing unplaced surfaces). Both are fixed on main, so this branch is now purely
a look-and-feel proposal. If it is not wanted, discarding costs nothing.
