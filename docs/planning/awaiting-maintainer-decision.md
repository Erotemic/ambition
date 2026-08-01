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

## 1. What do the world's transitions say? (queue Z′13, and it is not three doors)

**What you would see.** Standing in the central hub, the door nameplates read
`military_tower_door`, `hall_of_bosses_door`, `pirate_cove_door`. These survive a
capture with developer overlays off, so they are not debug text — a nameplate
renders `zone.name`, and the authored LDtk zones set `name` to the id string.

⚠ **and it is not three.** Counted across every authored world: **130 of 151
`LoadingZone`s carry an id-shaped name**, and that includes the game's OPENING
room — a capture of `intro_wake_room` shows `wake_room_arrival` written across
the player. So this was filed as "pick three strings" and it is not; at 130 it is
a rule, not a naming session.

**Three ways to take it, and they are genuinely different products:**

* **Author them.** 130 display names, once, by hand.
  [[feedback-entity-id-matches-label]] applies: rename the IDS to match the
  labels in one commit across LDtk + code + tests + docs.
* **Derive them.** A transition's label is the DESTINATION's display name — the
  door to the Hall of Bosses says "Hall of Bosses" because that is what the room
  is called. One rule, no per-zone authoring, and new content is named for free.
  *This is my weak recommendation*, because 130 hand-written strings drift and a
  derived one cannot.
* **Show nothing** unless a zone explicitly authors a label, and let the door art
  carry the meaning.

**The engine is already ready** for any of them: `DoorNameplateSource` carries
`id` and `label` separately, so a real label needs no code change.

⚠ **and one of the 130 looks like a plain bug regardless of the naming
decision.** `wake_room_arrival` is an ARRIVAL zone — the place you appear, not a
thing you walk into — and its nameplate draws over the player on the first screen
of the game. Whatever the labels end up saying, an arrival point probably should
not be announcing itself. Say the word and I will fix that half without waiting
for the rest.

⚠ **and it is bigger than that zone** (measured 2026-07-29, queue AC12). Labels
drawn across things you need to see is not one bad arrival zone, it is the
placement model. The screen carries at least THREE label families — authored
`DebugLabel` signage, actor nameplates, door nameplates — each placed by its own
system with no knowledge of the others:

* `drain_alley`: the sign `[manhole — Bob route return]` renders straight through
  the `npc_news_board` nameplate; both texts illegible.
* `combat_calibration_lab`: `MAP_OFFICIAL: calibration route` renders over the
  `lab_patrol_dummy` nameplate **and** over the player.

The intra-family spacer (`ambition_ldtk_tools.edit.space_debug_labels`) is
correct and useless here — it compares signage against signage, and reports "no
overlapping DebugLabels found" for both rooms, truthfully.

**The decision I need is which family YIELDS.** The nameplate system already
owns ranking and fade machinery, so that is where a single pass over all three
belongs; what it cannot decide on its own is whether a permanent authored sign
outranks a transient actor plate, or the reverse. My weak preference is that the
PLAYER is never occluded and authored signage yields to actor plates (a sign is
still there when you step away; an actor may not be) — but that is a
readability judgement about your game, and the wrong choice makes the intro's
tutorial signs disappear behind whoever is standing near them.

Jon's Thoughts: 
This doesn't feel that important right now. This is more of a debugging feature, but its 
incredibly helpful to have as something always on, but a real game probably doesn't use it
in this capacity. It's more like there's a door, and the player knows where it goes based on
a map, or something. A game itself might define that it wasn't a nameplate style presentation, 
and having engine sugar to make this easy might not be a bad idea. But again, this is extremely
low priority. This is game polish that I don't anticipate caring about until we have really really
good combat.


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
`ambition_characters`, below both callers (`ambition_platformer2d_actor_monolith::action_scheme` and
`ambition_sim_view::control_prompt`), so no match concept is visible to it. A
per-body marker from `ambition_characters` is the only shape both callers can
read — the route `ScriptedControl` already takes.


Jons Thoughts: 

A smash game (versus) shouldn't have the concept of "interact", in fact neither does Mary-O, or Sanic. Only Ambition has an "interact" --- well I guess maryo sort of has interact, it's like going into a pipe, but its a different mapping. The "interact" is something the ambition game might want, and its general to that game, but maybe not to the engine. There is certainly an architecture issue here, and I don't want to make a rash decision and introduce a worse abstraction. We need to have a discussion to think about what the right way to handle this is. 

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

Jon's TThoughts: Yeah the resolver makes sense. The portraits are not a hub feature, 
but different games might want to do it differently, however a character portrait for a dialog
box is ubiquitous for platformer 2d games, so it makes sense the engine has a mechanism to 
make it easy, although like most things with the engine, it should always be possible to 
ignore some part of it and roll your own. This is the Bevy ECS way. 

---

## 4. Which enemies get art first? (queue AB5)

**What you would see.** `combat_calibration_lab` — the room labelled "P4: Combat
Calibration", where the intro teaches a new player to fight — stages three
enemies that draw as red rectangles with `lab_patrol_dummy` / `lab_spitter` /
`lab_striker` floating above them. None is a catalog character, so none resolves
a sheet.

**This is the design working.** §4.10 is explicit that there is no fallback
sheet, because a body borrowing the goblin's art made missing work invisible:
*"Ambition's own enemies visibly regress until each gets art, which is the
point."* Nothing is broken.

**The call is about WHERE the debt is showing**, not whether the mechanism is
right. It is showing in the tutorial room of the opening sequence — the
highest-traffic square metre in the game for anyone you hand a build to. Three
sprites would change what the first fight in Ambition looks like.

**What I need:** whether those three are worth drawing now, or whether the intro
should stage enemies that already have art (Puppy Slug, AI Slop, Goblin) and
leave the lab dummies for later. The second option is a content edit I can make
in one commit if you want it.

Jon's Thoughts: 

The lab striker was an agent invention. I asked it to just use a goblin as the enemy there
until I decided what I wanted to do about the Nazis, which was the original idea. 
Just replace those enemies with a real enemy that already exist. The entire intro sequence
is unpolished slop anyway.  

---

## 5. What is DI worth on a fighter? (queue F0-J1)

**The state.** `di_adjust` — directional influence, the input that lets a
launched fighter steer their trajectory — is landed and pure. `di_max_angle` is
`0.0` everywhere, so DI is off. The combat-model table says outright that turning
it on is *"a feel number **Jon sets**"*, and suggests ≈0.31 rad / 18°.

**Why it matters now and did not before.** Until the blast zones landed there was
nothing to be launched INTO, so the number could not change anything. Now a
launch can end a stock, and DI is the difference between a knock-off that is a
coin flip and one that is a read.

**What I need:** one number (or "yes, use 18°"). The wiring is done.

Jon's Feedback:

In smash DI is critical! I probably want to use some smash physics in ambition itself too, because I want that game
to feel like a cross between smash subspace emissary and hollow knight (among other things, I'm drawing inspiration from a lot of places). 

---

## 6. Does versus end on HP or on stocks? (queue F0-J2)

**The state.** `DeathPolicy::Unbounded` works and is uncalled. Leaving it uncalled
means versus has TWO win conditions — drain the HP bars, or throw them out — and
that is what ships today.

**The choice.** Selecting `Unbounded` makes damage purely a knockback multiplier
and the blast zone the only way to win, which is the genre this stage is
modelled on. Keeping both is a defensible place to stand; it is just a different
game.

This is a decision about what the mode IS, not a defect, which is why nothing
has been done to it.

Jon's Feedback:

For a generic "versus" fighting proof of concept I don't care. Probably use health, to make it a generic fighter.
For Smash Siblings 3 stock, no items, final destination, fox only (that's fox only part is a joke). What I want for smash siblings is actually character select screen, ability to have 1-4 players, have them toggle between real player or cpu, use a smash like, drag an orb onto your character to select, and then the fight boots into a single 
battlefield like 3 platform level. Its 3 stocks, and then when the game ends it goes back to the character select screen. We don't need items in a first pass.

---

## 7. Keep or discard `wip/versus-reserved-surround`? (queue Z′10)

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

Jon's Feedback:

A smash game would have a character portrait on the bottom for each character with an icon for each stock and their current percentage. There is no score, when you lose your stock you are dead. 
---

## Should DIALOGUE stop the world, or only the talker? (2026-08-01)

**This is a product question, not a repair**, and it took a wrong turn to work
that out — so the mechanism is written down here rather than re-derived.

**What is true today.** `GameMode::{Paused, Dialogue, RoomTransition, Cutscene}`
all suspend gameplay, and the mechanism is not the input gate everyone reaches
for first: `apply_suspended_time_scale_system` runs under the
`gameplay_suspended` run condition and sets `ClockState::time_scale = 0.0`. The
world genuinely freezes — a suspended world still advances its TIMELINE (ticks
still count, so hashes and recorded input frames stay aligned) but moves zero
sim seconds.

⚠ I first concluded the opposite, because `allows_gameplay()`'s nineteen call
sites are almost all input routing and prompts, and the two run conditions built
on it look unused. They are not: `player_schedule.rs` uses `gameplay_suspended`,
and that one line is the whole pause.

**Why it is now a question.** Per-seat input contexts landed
(`SeatInputContexts`), so a surface CAN own one participant's input while
another keeps playing — and the test that proves the seam also pins that
`GameMode` overrides it (`dialogue_still_stops_the_world_as_well_as_claiming_
the_input`). On a couch, one player talking to an NPC currently freezes the
other player mid-jump.

**The decision:**

* **keep it** — dialogue is a world-stopping beat, and couch multiplayer
  tolerates the freeze because conversations are short. Costs nothing; the
  per-seat seam simply waits for a surface that is not world-stopping
  (inventory, a select screen).
* **split it** — `Dialogue` stops claiming to stop the world, and the talker's
  input is claimed by `DIALOGUE_CONTEXT` instead. Then NPCs keep patrolling and
  hazards keep ticking during a conversation, which is a real change to how
  every existing scene plays, not just a multiplayer fix.

⚠ **it is not a small mechanical change either way**, and the branch that looks
harmless is the wrong one: leaving `Dialogue` in the suspend set while adding
per-seat contexts gives a seam that silently does nothing, which is worse than
either answer.

⚠ **`RoomTransition` and `Cutscene` are NOT the same question.** Both are
genuinely global — a room is loading, or a scripted beat owns the screen — and
nothing about per-seat input makes them per-seat.

---

## Which of the 33 engine design documents have become history? (2026-08-01)

`check_agent_kb` warns that `docs/planning` is **30,708 lines against a 10,500
soft budget**. The warning is non-fatal and has been standing for a while; this
is the measurement that says why it cannot be closed by a rule.

```text
  docs/planning/*.md   11,508   the queues and live plans
  docs/planning/engine  15,257   33 design documents   ← the budget
  docs/planning/triage   2,713
  docs/planning/demos      899
  docs/planning/game       231
```

The rule that has worked twice — *"a section with no `▢` rows is closed, move it
verbatim to the archive"* — applies to LEDGERS. Applied to the whole backlog
queue it frees 251 lines; it freed 46 from the 72h queue today. Together that is
about **1% of the overage**.

`engine/` has no `▢` rows because those files were never ledgers. Archiving one
means judging that a design is built and stable enough to stop being read, which
is a judgement about the engine's own record rather than a checkable property —
and losing design rationale is the kind of loss nobody notices until they need
it.

**The decision:** which of the 33 are history. A plausible first pass, for
someone who knows what is still live:

* the largest are `immutable-content-and-transactional-construction.md` (2,406),
  `competitive-2d-platformer-engine-roadmap.md` (1,567), `room-transition-loading.md`
  (930), `fighter-brain.md` (873) and `api-1.0-campaign.md` (868);
* `api-1.0-campaign.md` describes a campaign the ledger records as CLOSED, which
  makes it the most likely candidate and still not one an agent should decide.

⚠ **do not resolve this by trimming live plans.** That is what the budget's own
message warns against, and it is what the pressure produces when the only
reachable lines are the ones still being used.

⊘ **not proposed: raising the budget.** A soft ceiling nobody can meet stops
being read, but so does one that is raised whenever it is inconvenient. It should
move because somebody decided what belongs in `docs/planning`, not to silence a
warning.

---

## Does `apple_rain`'s damageable box follow the head row when no sprite sample exists? (2026-08-01)

One line of content blocks the `BossAnim`→`CharacterAnim` fold's first slice.
Pinned by `apple_rain_claims_no_animation_rows_which_is_why_the_fold_is_blocked`
(`ambition_content`), which fails on either answer so neither happens silently.

`apple_rain` is a `Special`, so its animation rows come from
`ambition_content`'s `special_animation_keys()` map — and it is not in it. Its
profile therefore claims **no rows**, while the sample writer emits `"head_down"`
for it, with a comment saying that is deliberate: *"GNU-ton's apple rain reads the
head row for its damageable hurtbox."*

So the intent is written down and the catalog does not carry it. Today the row is
found anyway, through a circular path: `runtime_animation_keys` pushes the
sample's OWN key into the list whenever the sample's profile matched. `apple_rain`
works *because of* the check the fold wants to remove.

**The decision:**

* **add `("apple_rain", vec!["head_down"])` to `special_animation_keys()`** — the
  catalog then says what the code already believes, every identity agrees as a
  set, and the fold becomes a rename.
  ⚠ **it is not purely cosmetic**: with no sprite sample (headless tests, and the
  frames before sprites upgrade) `apple_rain` currently falls back to the body
  bbox and would instead sample the authored `head_down` hurtbox. That is a live
  boss's damageable shape changing on that path.
* **keep the profile identity** — accept that `BossAnimationFrameSample` carries
  two facts that legitimately differ (which attack drives, which row renders),
  and fold only the row VOCABULARY (slice 2), leaving `BossAttackProfile` in the
  gameplay geometry path.

⊘ **not proposed: adding the row and calling it a no-op.** The fallback path is
where headless tests live, and "it only changes the case with no sprites" is the
same sentence as "it only changes what every test sees".
