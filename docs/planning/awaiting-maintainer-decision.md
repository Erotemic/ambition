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

## Is a crate name part of the rollback wire format? (queue S30, 2026-08-01)

**Newly blocking.** This sat as a queue row for days without costing anything.
It now blocks a specific next step, which is why it is here.

### The fact

`descriptor()` stores `std::any::type_name::<T>()`, `schema_dump()` writes it,
and `schema_fingerprint()` hashes the dump. So **moving a type between crates
changes `SnapshotSchemaFingerprint`**, and two peers with byte-identical wire
formats refuse to agree. Measured during the rename: **94 of the 352 lines** in
`rollback_schema_baseline.txt` name a crate.

⚠ this is the same defect v5 fixed one level down. `registry.rs:33` says it in
its own words — hashing the registration OWNER "made 'which module registered
this' a wire-format fact", so v5 stopped hashing it. The crate path inside
`type_name` is the same category of organisational label, and it is still hashed.

### What it blocks now

The `ambition_geometry` carve (done today) took shapes, boxes, reference frames
and swing/combat volumes out of the platformer core, and `ambition_vfx` stopped
declaring a platformer dependency it never had. The census says the **snapshot
codec** — `snapshot::{Reader, SnapshotState, SnapshotCursor, put_*}`, 347 lines,
wanted by `ambition_time` and `ambition_sprite_sheet` — is the next equally
general thing stuck in there.

⛔ **but moving it rewrites the fingerprint**, which is why the geometry carve
went first and this one has not started. Deciding this unblocks it.

### ⭐ Two things measured since, both of which enlarge this decision

**1. The codec blocks carve-outs by a SECOND mechanism nobody had named: the
orphan rule.** Found 2026-08-02 while carving `ambition_projectile_spec`.
`ambition_platformer2d_shared_tangle` contains

```rust
snapshot_unit_enum!(crate::projectile::WorldHitPolicy { Bouncing = 0, … });
```

— an impl of **core's** `SnapshotState` for its own type. Move that type into a
new crate and the impl becomes a foreign trait on a foreign type and stops
compiling; move the impl with it and the new crate must depend on core for
`SnapshotState`, reintroducing exactly the edge the carve existed to remove.
`ProjectileSpec` is dragged along by its `world_hit` field.

So wherever the codec lives, it anchors every type anyone implements its traits
on. That is independent of the fingerprint, applies to **66 encoded types across
9 crates**, and (b) does not fix it — only moving the codec out of core does.

**2. `ambition_input` is the single highest-value cut in the workspace, and it is
this decision.** `scripts/core_import_census.py --cuts` simulates dropping one
direct edge and reports who leaves the closure:

```text
 3 crate(s)  ambition_input        input, inventory_ui, ui_nav
 3 crate(s)  ambition_characters   characters, content_cli, interaction
 1 crate(s)  ambition_time         time
 1 crate(s)  ambition_platformer2d_shared_tangle
 0 crate(s)  …everything else
```

`ambition_input` imports **exactly one item** from core — `ControlFrame` — and
`ControlFrame` is line 121 of `rollback_schema_baseline.txt`
(`derived.control_frame`). `ambition_characters`, the only cut of equal value,
imports 77. So the cheapest three-crate cut available anywhere in the workspace
is gated on this fork.

⚠ **and be careful reading that table the way I first did.** The `--paths`
column made it look like SEVEN crates reached core only through `ambition_input`;
simulating the cut says three. A shortest path hides every alternative path by
construction. Rank by `--cuts`, never by `--paths`.

**3. Checked the other three cuts, and THREE OF THE FOUR are this decision.**
`ambition_time` takes the snapshot codec and nothing else. `shared_tangle` is
already platformer-named. And `ambition_characters` — the only cut of equal value
— turns out not to be a carve at all: of its 78 imports, 11 already live in
`ambition_geometry`, 3 are the codec, and **20 are genuinely platformer**
(`AxisJumpLaw`, `MomentumParams`, `AIR_ACCEL`, `DASH_SPEED`, `SLASH_RECOIL`,
`AbilitySet`, `MovementTuning`). A crate whose brains reason about jump laws and
air acceleration is a platformer crate with an unqualified name; that is a
renaming question for you, not a carve-out.

So there is no remaining carve-out work that this fork does not gate.

### The fork

**(a) Accept it.** Regenerate the baseline and bump
`GGRS_ROLLBACK_SCHEMA_VERSION` whenever a type moves. Honest, one commit, and
concedes that reorganising crates is forever a compatibility break.

**(b) Hash below the crate.** Fingerprint the type's final segment plus its
module path *below* the crate, so the fingerprint stops caring where a type is
packaged — what v5 already decided for owners, applied consistently.
⚠ two same-named types in different crates then collide, so `RollbackRegistry`'s
duplicate-name check has to stay the thing that catches it. It already rejects
duplicate `name`s, so this is a dependency on existing behaviour, not new work.

The baseline regenerates either way. **Only (b) makes the next move free.**

### It already happened, today

The `ambition_geometry` carve broke the schema baseline, and it took four
commits to notice because `cargo check --all-targets` compiles tests without
running them. The entire diff:

```text
- actor.centered_aabb  ambition_platformer2d_core::geometry::CenteredAabb
+ actor.centered_aabb  ambition_geometry::geometry::CenteredAabb
```

Same type, same projection, same registration, same bytes on the wire. Only the
crate path inside `type_name` moved. Under (a) that is a compatibility break
every peer must be told about; under (b) it is invisible, which is what it
deserves to be, because nothing a peer can observe changed.

⚠ note also how it was caught: not by a guard, but by going looking for
something else. Under (a) the cost of a carve is not just the version bump — it
is that every carve needs somebody to remember to run one specific test.

### Recommendation

**(b)**, for one reason beyond consistency: the engine is at the start of a
carve campaign, not the end. The census still shows 24 crates on the platformer
core and names three more general things inside it. Under (a) every one of those
is a wire-format break, which prices the reorganisation in exactly the currency
that stops it happening.

⚠ against (b): it is a real behaviour change to a determinism-critical hash, and
it wants its own probe — two types with the same final segment in different
crates must be REJECTED loudly, not silently merged. That check is the work item
attached to choosing (b), and it should land in the same commit.

## The top four ladder rungs ship a knob that measurably makes them worse (2026-08-02)

**Newly surfaced**, and it is a balance call rather than a bug — but it has sat
inside a probe's header comment where nobody was going to weigh it.

### The fact

`profile.rs` ships, since `ed6c55d0e` (2026-07-31 02:23):

```rust
rollout_depth: if level >= 6 { 12 } else { 0 },
rollout_k:     if level >= 6 { 4  } else { 0 },
```

So levels 6–9 — the rungs meant to be HARDEST — run L3 lookahead, and levels 1–5
do not. Every measurement of that knob since it shipped says it makes the fighter
kill itself sooner:

```text
2026-07-31 morning  9/d0  5.0s to first self-KO   9/d12  9.8s    ← d12 better
2026-07-31 evening  9/d0  5.2s                    9/d12  2.7s    ← d12 WORSE
  (three seeds, identical on every seed)
2026-08-02          9/d0  no self-KO in 60s       9/d12  4.8s    ← d12 WORSE
                          0 stocks lost                  1 stock lost
```

The morning run is the outlier and the probe's own header says why it should not
be compared: *"the morning's numbers are not comparable to the evening's."* Two
independent runs, four seeds, agree that the knob costs survival.

### What the measurement does NOT cover

⚠ **the probe's opponent cannot attack.** Its header says so — *"a human seat
with no controller… every stock lost is a self-KO"*. So this measures
self-preservation and nothing else. Rollout could plausibly buy offence and pay
for it in walking off ledges, and **nothing has measured the offence half**.
`fighter-brain.md` §12.6 already records exactly this: the first measurement
*"unblocks authoring a nonzero `rollout_depth` on a survival claim but not on an
attack one"*. The nonzero depth was then authored anyway, and the survival claim
it rests on has since inverted.

### The fork

* **Revert to `rollout_depth: 0` everywhere** until `l3_earns_its_depth` (still
  owed, FB6e) measures an attacking opponent. Restores the state the docs
  describe, at the cost of the top rungs losing their only distinguishing
  mechanic.
* **Keep it and treat self-KO as acceptable** for high rungs — a fighter that
  commits harder and sometimes overshoots may be the more interesting opponent,
  and the probe cannot see that.
* **Keep it and fix the recovery** — and this option changed the same day it was
  written, because the recovery failure turned out to be MEASURED and GEOMETRIC
  rather than a momentum problem.

  ⭐ **the fighter loses its stocks in a 2.5-second limit cycle against the
  platform's side face.** From `ladder_probe`'s unclaimed-velocity detector, 91
  events at one frozen `x`:

  ```text
  fall off the left edge  →  jump (vel_y = -520 = JUMP_SPEED)
                          →  dash RIGHT at 760 to recover
                          →  dash killed instantly, x pinned at 101.84
                          →  fall back, repeat every 150 ticks
  ```

  The platform is `x 110..530`, `y 300..332`. The body sits 8.16 px left of its
  left edge at the platform's own height — **beside the wall, not under the
  lip** — and a horizontal dash cannot climb 32 px of platform. `chose=Some(Recover)`
  appears in the seam on those ticks, so the brain IS trying; it is aiming its
  recovery at a wall.

  ⚠ that makes this option concrete rather than speculative: the fix is a
  recovery that gains HEIGHT (a jump-then-drift, or a dash whose direction rises
  toward the lip), not a tuning change. And it means the survival numbers above
  may be measuring stage geometry as much as lookahead depth — a fighter that
  cannot recover at all will self-KO regardless of what its rollout decides.
  ▢ unconfirmed detail: the 8.16 offset implies a body half-width of 8.16 and no
  such number is authored; the box may be sprite-derived (`SpriteAuthored`'s
  per-pose projection) or the stop may be a different contact.

⚠ whichever way: the three prose claims that said `rollout_depth` was zero
everywhere are corrected as of 2026-08-02, so the code and its description now
agree. This decision is about the value, not about the drift.

---

## 8. Which block did you stand on? (queue 08-03 D10 / D11)

**One answer closes two rows or reopens a collision question**, which is why it is
worth asking rather than guessing.

Your report was *"I can stand on a broken brick"*. Everything downstream was
traced and probed, and the brick path is CORRECT: break marks the bit, the bit
becomes `removed_block_names`, `world_with_sandbox_solids` applies the
subtraction before anything reads `solids()`, and a written probe
(`a_broken_brick_leaves_the_collision_world_the_body_reads`) shows brick 0 gone
from the composed world against the real level while its neighbour is untouched.

⭐ **So the leading explanation is that it was a SPENT `?`-BLOCK**, and a spent
`?`-block staying solid is correct — it is what the genre does.
`SpentPowerBlocks` contributes nothing to `removed_block_names`, deliberately.

- **(a) It shattered into pieces and I stood where it had been.** The probe is
  wrong or incomplete and the collision path reopens.
- **(b) It popped an item and stayed put.** Then nothing is broken and the real
  defect is that it still LOOKS live — which is your own D11 row (*"used blocks
  need their own texture and a nudge animation"*), and that becomes the fix.

⚠ **I cannot distinguish these from the report**, and the two lead to opposite
work: one is a physics investigation, the other is an art and animation task.

## 9. Can a flying fighter shield? (queue 08-03 F7-duel)

Two `app_it` duel tests have been failing since before this run. The cause is
measured, not suspected — the same fight, both fighters, identical capabilities:

```
PCA:   shield_frames=535  dash=0   fly=194  fly_toggles=4   blinks=3
robot: shield_frames=0    dash=30  fly=427  fly_toggles=12  blinks=1
```

In `brain/smash/mod.rs`, ARMING the block requires `cfg.can_shield &&
obs.self_on_ground`, and HOLDING it requires `shield_hold_timer > 0.0 &&
obs.self_on_ground`. The robot is airborne for 427 of ~600 frames, so the second
layer of the layered defence (blink → shield) is **unreachable** for it — not
chosen against, unreachable. Nothing says so at the decision site.

- **(a) Air-shielding is legal.** Drop `self_on_ground` from both conditions. The
  layered defence becomes available to a flying body, and shield stops being a
  ground-only tool.
- **(b) It is deliberately ground-only.** Then the DECISION SITE should say so,
  the brain should not keep choosing a defence it cannot enact, and the tests'
  expectations become air-aware rather than symmetric.
- **(c) A flying fighter should land to shield.** The most work, and the most
  like a real fighting game.

⚠ **this is a design call and I should not pick it**: it changes what a fighter
IS, and two of the three answers change the test rather than the game.
⚠ likely fallout from the 08-02 frame-typing fixes (`5ae87b2e8`, `3e7780623`) —
the fighter used to write raw world x into the body-local `locomotion`, so its
movement, closing speeds, and flight decision all changed when that was
corrected. The tests were calibrated against the buggy movement.

## 10. Does `character-authority-is-app-local` bind PRESENTATION? (queue 08-03 G1 pick 2)

The workspace-policy suite was found red with 10 violations from 08-02 work
(invisible because the failing job already had a known failure in it). Eight are
repaired or narrowed; **two are a real break with no obviously right fix.**

`slash_visuals.rs` and `unauthored_volumes.rs` take
`Option<Res<CharacterCatalog>>`, which the policy forbids by name and by
rationale: *"may not … silently substitute an empty catalog, make those resources
optional"*. The concrete harm is real — an absent catalog makes `attack_vfx`
return `None`, so `authored` is false, so a placeholder volume is drawn over
**every** attack including characters that author their own art. Silent and
backwards.

⚠ **but making the resource required panics the headless and test compositions
the `Option` was added for**, so the obvious repair is not available.

- **(a) The read-model carries it.** *Does this character author its own attack
  VFX* becomes a published fact beside the sprite quad the read-model already
  carries for the same reason. Presentation stops consulting the catalog at all,
  which is what E4 says anyway. A publisher change plus two consumers.
- **(b) The policy does not bind presentation.** Its rationale is about
  gameplay AUTHORITY — who decides what a character IS — and a renderer asking
  "did you author art for this" is not that. Narrow the policy's scope with the
  reason, as was done for the dialog policy this run.

⚠ **(a) is more work and probably right; (b) is defensible and I am wary of
choosing it**, because "narrow the rule" is the answer that makes my own red go
away, and that is exactly when it should not be my call.

## 11. Is `bevy_material_ui` being adopted, or is it 31% of the frame for nothing?

**`MaterialUiPlugin` contributes 182 systems to `Update` — 31% of the entire
schedule, and half of every system in it that belongs to no set.** They run every
frame, title screen included, which is exactly where `code_smells.md` measured the
executor spending 10–18% of CPU self-time.

`git grep bevy_material_ui` across every `.rs` in the workspace returns **one
file**: the line that installs the plugin, and a doc comment above it. No type,
component, system or helper from the crate is named anywhere.

⚠ **but it is load-bearing.** Removing it fails
`the_title_screen_says_choose_game_and_is_readable` — the title renders at 20.0px,
Bevy's default font size, so menu typography stops resolving. A plugin contributes
systems and resources, not names; the grep could not have told me that.

⭐ **and `MaterialUiPlugin` is a BUNDLE**: `MaterialUiCorePlugin` plus 29 component
plugins, including a date picker, a time picker, snackbars, chips, an app bar and
a toolbar. The crate's own documentation offers the core separately so a consumer
installs only what it draws.

✔ **TRIMMED, 2026-08-03** — `MaterialUiCorePlugin` + `DialogPlugin` only, found by
a six-step bisect against the one failing test. **584 → 428 systems**, 156 off
every frame. `app_it` 284/2 (the duel pair alone, exactly the baseline).

⚠ `DialogPlugin` is load-bearing for MENU TYPOGRAPHY, which no one would have
guessed — two reasoned guesses (the core alone; the core plus the *adaptive
layout* plugin) were wrong before the mechanical bisect found it.

**So the question left for you is narrower than it was, and it is about intent:**

- **(a) Leave it trimmed.** Ambition draws no Material date picker, time picker,
  snackbar or toolbar, and adding one later means adding its plugin beside the
  two — one line.
- **(b) Put the bundle back.** If Material UI is a direction you are part-way
  into and you want the whole widget set available while you build with it, say
  so and it reverts to `MaterialUiPlugin` in one line. The 31% is then a cost you
  are choosing, which is a completely different thing from one nobody noticed.

⚠ **I am not treating "unused by grep" as "unused"** — that inference was already
wrong once here. This is a question about your INTENT for the dependency, which no
measurement can answer.
