# Expressive moves — engine acceptance campaign (opened 2026-09-05)

**State:** OPEN. **This file is execution order, not feature status.** The
mechanics authority stays [`../smash-parity-inventory.md`](../smash-parity-inventory.md);
the capability/authority map is
[`../../engine/expressive-move-capabilities.md`](../../engine/expressive-move-capabilities.md).
⚠ The previous campaign in this directory closed with the warning that replaying
a campaign's chronology is actively harmful once the inventory has moved on.
Same rule here: when a row lands, the fact goes to the inventory and this row
becomes a receipt.

Jon, 2026-09-05: **get the moveset so a 1v1 human vs human match is excellent.**
⛔ **Explicitly deprioritised: great CPU AI.** The ladder/fighter-brain
measurement campaign is not the work; a human opponent is.

## Why these twelve

Jon's list is chosen so that together they force most of the expressive surface,
each one as an **engine acceptance fixture** rather than a character feature:

| # | Proof move | What it proves |
|---|---|---|
| 1 | **PK-Thunder parody** | `TechniqueFlow`, projectile input delegation, steerable projectile state, self-contact, correlated events, temporary fighter-control suppression |
| 2 ✔ | **Counter + Revenge variant** — the stand-ins' `riposte`, and **2026-09-05 the Author's `author_second_draft`** | ⭐⭐ **"COUNTER" TURNED OUT NOT TO BE A MOVE TYPE AT ALL.** `answer_a_parry_with_the_authored_counter` dispatches `SpecialActionSpec::Special(stance.response.clone())` — fully generic — so a counter is **ANY TECHNIQUE, TRIGGERED BY A SUCCESSFUL PARRY**. Counter-into-grab (George), counter-into-teleport (the Author), counter-into-mine or -sleep are all authorable today with zero engine work. ⇒ I built this seam and then under-read it for a day |
| 3 | **Reflector + absorber** | projectile interception is a projectile authority and supports more than parry reversal |
| 4 | **Tether recovery + aerial tether grab** | terrain/body tethering composes with ledge and capture authorities |
| 5 ✔ | **Cargo command grab** — **LANDED 2026-09-05** on the goblin's down-throw | ⭐ **MOVEMENT WAS NEVER TOUCHED.** The claim was "capture and movement cooperate", and what actually happened is that capture RESTRICTS LESS: `restrict_captor_control` zeroed `locomotion` for every captor, and a carry is a hold that does not. ⇒ `carrying` rides `SmashHoldState` (the ruleset's half) rather than `CapturedBy` (the generic relation), on that component's own argument that platform-fighter rules do not belong on the relation |
| 6 ✔ | **Remote mine** — **LANDED 2026-09-05** on Projectile Polygon's down-smash | persistent spawned identity, owner lookup, remote triggering, rollback. ⭐ **It cost one component with four fields and no new authority**: the object is a `GroundItem`, the blast is a `DamageBoxEffect`, and "where is it, whoever holds it" is `ItemWorldPos`. The mine contributes an arming clock and a decision |
| 7 | **Parasol** | temporary movement modifiers/controllers are expressive enough for a long-lived post-move locomotion regime |
| 8 | **Homing Attack** | deterministic semantic target queries and target-directed fighter motion |
| 9 ✔ | **Sing** — **COMPLETE 2026-09-05** on the Performer's neutral special | `BodyCombat::sleep_timer`, the `smash.sleep` technique and the area adapter, all guarded — and now with a customer. ⭐ **ADDED TO `the_monologue`, NOT SUBSTITUTED FOR IT**: her strike is 58×34 out in front and is unchanged to the number; the sleep is 26×26 centred on HER, wholly inside it. Everyone still gets the speech; only whoever stood next to her goes under |
| 10 | **Limit** | character-local resource state, threshold transitions, timeout, action variants, stat modifiers |
| 11 | **Free-standing ally summon** | summon is not synonymous with mount; first owned-secondary-actor contract |
| 12 | **Reusable launch object** | a fighter can create a persistent world actuator another fighter interacts with |
| 13 ✔✔ | **Portal recovery** (Jon, added same day) — **LANDED 2026-09-05** on Alice's up-B, **and the ANGLED half completed the same day** | an authored customer places linked world portals and traverses them. ⭐⭐ **Jon's second sentence is now built too**: *"we can even exercise angled portals with directional input on the up b as a flavor that isn't actually in smash and is ours."* `tilt_degrees` was `0.0` in every literal in the tree — the parameter existed, the code applied it, nobody ever set it — and the player's own stick now leans the shaft ±32°. ⇒ **It reuses `MovePlayback::aimed_stick`, the LATCHED UNDAMPED aim the teleport already reads**, so no new authority: an aimed special is rooted, and a live stick read would be neutral for the whole move |

⭐ **Sanic's spring analogue is a SPEED BUMP** (Jon): he slams down a ridiculous
yellow-and-black speed bump that catapults anyone who touches it. On the ground
it stays as the reusable launcher; in the air he drops it and it falls as an
object/hazard. Same mechanical role, its own joke.

## Roster assignments — who each proof move is FOR

Jon, 2026-09-05, in the same breath as *"we have a lot of characters with boring
specials, and when we build the code for these we should exercise them in the
characters."* ⇒ **Every rung lands on a real fighter.** That is acceptance
clause 2 for each row, pre-answered.

| Fighter | Move | Proof rung |
|---|---|---|
| **Swordies** | **counter** | 2 — successful-defense consequence (B1) |
| **Projectile Polygon** | **tether**, and probably the **remote mine** as down-smash | 4 and 6 |
| **Performer** | **Sing** | 9 |
| **Author** | **side-B: the "mind" attack**, PK-Thunder style | 1 — the keystone |
| **Alice** (provisional) | **up-B: a portal recovery** | ✔ LANDED 2026-09-05 (`9444800b7`) |

⚠ **PLACEMENT IS PROVISIONAL AND SAID SO** — Jon: *"We can tune who the moves
belong to later. The important thing is that they can be expressed and authored
easily and creatively."* ⇒ A rung is finished when the move is EASY TO AUTHOR,
not when it is on the right fighter. Do not spend a decision on the roster slot;
do spend it on whether a second fighter could take the same move by authoring
alone.

### Sing is one field, because the control lock is already generic

⭐⭐ **MEASURED 2026-09-05, and it confirms Jon's hypothesis exactly.** He wrote
that a Disable-style *"you cannot act for this duration"* may already be
expressible through the existing combat/control lock machinery. It is, and the
seam has a name: `attack_support.rs`'s `hard_lock_timer` is a `max()` over four
NAMED causes — the knockback/landing locks a body owns, the dizzy a broken guard
owes, the shieldstun a blocked hit charges, and shield-drop lag. Its comment
calls them *"three facts that remove control outright"* and then adds a fourth,
which is a seam that has already been extended once.

⇒ **So a sleep is a fifth timer joined to that max.** `BodyCombat::hard_lock_timer()`
is itself `recoil_lock_timer.max(landing_lag_timer)`, and
`decay_reaction_timers` already ticks every reaction timer on that component —
so a `sleep_timer` field costs one line in each and inherits the decay.

⭐ **Wake-on-damage falls out too**: the hit path already touches `BodyCombat`,
so clearing the timer there is where a wake belongs, next to the reaction timers
it sits beside.

⚠ **WHAT THIS DOES NOT BUY, and Jon named both:** the specific POSE and the MASH
escape. Those are what make a sleep richer than a Disable, and neither is
expressible as a timer in a max. ⇒ A first version without them is honest and
playable; it is a Disable wearing Sing's name, and the row should say so rather
than claim the mechanic is finished.

✔ **BUILT 2026-09-05 exactly as costed** — `BodyCombat::sleep_timer` folded into
`hard_lock_timer()`, ticked by the shared decay, cleared by `reset()`, and woken
by a real hit inside the windbox guard (a flinchless gust declines to charge
stun, so it must not discharge a sleep either). `smash.sleep` is the authored
key; `apply_authored_sleep` finds the bodies in range. Three poisons fire: the
singer caught by their own song, songs that STACK instead of taking the longer,
and an ignored area.

⛔⛔ **AND IT HAS NO AUTHORED CUSTOMER, WHICH MEANS THIS ROW IS NOT DONE BY THIS
PAGE'S OWN BAR.** Jon assigned Sing to the Performer, and she already has all
five specials — including `the_monologue`, which is a proto-Sing that holds the
room with FIXED KNOCKBACK and holds her too. ⇒ Turning that into a real status is
a BALANCE change to shipped, documented content: adding the sleep on top makes
the move much stronger, and replacing the knockback removes the hit. Neither is
mine to decide, so the seam is registered and tested and the move is not
authored. ⚠ Do not read the ✔ as "Sing ships".

⛔ **Do NOT reach for `break_timer`.** It is the shield-break dizzy, presentation
draws it as one, and overloading it would make "why is this fighter helpless"
have two answers — the one-authority failure this campaign keeps finding. The
lock seam is generic; the CAUSE must stay its own fact.

### The portal recovery — Jon's, and not a Smash move at all

> *"up b opens a portal under him, and a portal at the very top of the stage, and
> when he falls into it he comes out the higher portal (or vice versa, it's a
> portal so just use the portal crate rules, we can even exercise angled portals
> with directional input on the up b as a flavor that isn't actually in smash and
> is ours)."*

⭐ **This is the "author a customer for a shipped primitive" pattern in its
purest form**, which the D72 row measured as almost always the real next slice.
`ambition_portal2d` exists with link groups and portal rules; a recovery that
places a linked pair and falls through it should be authoring plus a technique
key, not portal physics. ⇒ **Use the portal crate's rules rather than
re-deriving recovery behaviour** — if the fighter's traversal through a portal
does not already work, that is a finding about the portal seam, and it is worth
more than the move.

✔ **COSTED 2026-09-05 — it really is authoring plus a technique key, checked
against the crate rather than hoped.** `PlacedPortal` is a **Component**, so
placing one is a `spawn`; `PlacedPortal::fixed(channel, pos, normal, half_extent)`
is the static (unhosted) constructor that fixtures and placement sites already
use; and `PortalChannelColor::Indexed(n)` documents its own pairing rule —
*"even = slot A, odd = slot B; the partner is `Indexed(n ^ 1)`"*, with indices
`8..` reserved for exactly this kind of extra pair. ⇒ **A move-placed pair is two
spawns on `Indexed(n)` and `Indexed(n ^ 1)`**, and the portal crate's own transit
rules carry the fighter through — which is the point of using it rather than
writing a recovery.

⇒ **The slice, in the order it should be built:** a `smash.portal_pair` technique
carrying rise, half-extent and (later) an angle; a smash-side adapter that spawns
the two apertures — one under the fighter with an upward normal, one at the stage
top facing down; and a LIFETIME, because a recovery that leaves a permanent hole
in the stage is a different move. ⚠ The lifetime is the part with no obvious
answer yet: close on move end, on first transit, or on a timer are three
different mechanics, and that is a design call rather than a wiring one.

ⓘ The angled variant (directional input on the up-B tilting the pair) is
explicitly OURS rather than genre parity. It is also the cheapest possible test
of whether the portal placement seam takes an authored orientation, so author
the straight version first and let the angle be the second commit.

## Sequencing — two tracks, because the keystone is not the fun

The twelve are not in build order. **1 is the architectural keystone and 2–5 are
the fun**, and they do not depend on each other, so they run as two tracks.

### Track B — playable depth that needs NO new engine rung

Taken first, because each lands as something two humans can feel in a match and
none of them waits on `TechniqueFlow`.

- ▢ **B1. Successful-defense consequence** (proof move 2's engine half). Parry
  already denies a qualifying contact; what is missing is emitting an authored
  semantic effect when it succeeds. One seam, and it unlocks ordinary counters,
  Revenge-style stacking and Witch-Time-style application as pure composition.
  ⭐ Highest fun-per-line on the page: it adds a READ to every exchange, which is
  what 1v1 depth is made of.
- ▢ **B2. Ground tether grab** — authoring plus line presentation only.
  `CaptureAttemptRequested` already takes an authored reach volume.
  ✔ **THE REACH LANDED 2026-09-05** on Projectile Polygon — her grab, not a
  special, because in the genre a tether IS the grab and she is the roster's
  ranged identity. 150px along the facing, with the vertical extent cut from
  16px to 10px so the reward is distance and the price is precision.
  ⛔ **The guard that was supposed to catch it could not see her**: the demo's
  ceiling walks two movesets and `ambition_demo_smash` does not depend on
  `ambition_content`. A sibling now walks `authored_movesets::tables()` and holds
  a named TETHER allowlist rather than a raised ceiling — so one fighter having
  a tether is a reviewed fact, not a loosened rule for everybody.
  ✔ **THE LINE LANDED 2026-09-05**, and it is the fourth customer of
  `flyline.rs`'s procedural-visual shape exactly as this row predicted — same
  sprite, same `place_wire`. ⭐ Published as a POINT
  (`BodyPoseView::grab_reach` / `FeatureView::grab_reach`) resolved from
  `MovePlayback::live_capture_reach()`, so presentation never re-derives the
  reach from authored params it should not read. ⛔ Drawn on BOTH roads, with
  the actor arm poison-verified against the trapdoor's own defect.
- ◐ **B3. Projectile interception as a projectile-domain operation** (proof move
  3). ✔ **THE OPERATION LANDED 2026-09-05** (`projectile/intercept.rs`):
  `ProjectileInterception::{Reflect, Consume}` and `intercept_projectile`, with
  the parry as its first caller. It touches exactly three of the six axes —
  combat owner, allegiance, trajectory — and emits NO cues, because a parry
  clangs where a reflector hums and an absorber swallows. Returns whether the
  shot SURVIVED, which is the distinction an absorber needs on its first line.
  ⛔ `Redirect` deliberately absent: no customer, no test, and a third variant
  nothing calls is the speculative peer authority this campaign exists to avoid.
  ⭐⭐ **AND THE REFLECTOR TURNS OUT TO BE ALREADY SHIPPED, BY COMPOSITION —
  found 2026-09-05, no work.** `step_projectiles` gates its parry on
  `shield.parrying()`, and its comment says so in place: *"The SAME catch the
  melee strike seam resolves, from the other route a strike arrives on: one
  fact, both roads."* The counter stance (`smash.counter`) opens exactly that
  window. ⇒ **A fighter standing in `riposte` reflects an incoming shot**, with
  the projectile re-owned and its velocity reversed, because both roads read one
  predicate. Proof move 3's reflector half needed no move and no engine change;
  it fell out of proof move 2.
  ⚠ **BY CONSTRUCTION, NOT BY TEST, and the distinction is load-bearing.** The
  existing parry tests call `reflect_parried_shot` DIRECTLY — they never exercise
  the `shield.parrying()` gate — and the stance test asserts the window is open
  without firing a shot at it. So the two halves are each covered and the join
  is not. ⇒ If someone gates the projectile road on `active` as well (the exact
  mistake `parrying()`'s own history records), the reflector silently stops and
  nothing goes red.
  ▢ **What remains is the AUTHORED half**: an absorber move, and an end-to-end
  guard for the join above,
  which is where `Consume` gets the customer it is
  currently tested without.
- ◐ **B4. Per-action muzzle transform + sustained charge presentation.**
  ✔ **THE MUZZLE LANDED 2026-09-05.** `Muzzle::Offset { x, y }` joins `BodyOrigin`
  and `Hand`, as FRACTIONS of body height (the decision `HAND_OFFSET_NORM` already
  made), facing-flipped and resolved through the acceleration frame. Projectile
  Polygon's charge shot now leaves the head-mounted cannon that this repository
  described in three separate files while the code fired from her midriff.
  ⭐ **The resolution is a function (`muzzle_world_pos`) so it can be ASKED** —
  inline in the fire system, the only way to check that an authored muzzle moved
  anything was to stand up an app, a body, a weapon and a shot, which is why
  nothing did.
  ▢ **AND THE SECOND HALF TURNS OUT TO BE THE SAME DEFICIENCY, not a separate
  one — measured, and it revises the row's premise.** The charge presentation is
  ALREADY sustained: `draw_player_projectile_charge` despawns and respawns the orb
  every frame from `BodyPoseView::charge_tier`, tracking the presented body and
  scaling by tier. It is not a point VFX event. ⇒ What is wrong with it is the
  same thing that was wrong with the shot: it draws at a HARDCODED body offset
  (`size.x * 0.5 + 6.0`, `size.y * 0.20`), so the orb forms at the hip while the
  shot now leaves the cannon.
  ⛔⛔ **ANSWERED 2026-09-05 — AS TWO DEFECTS, NOT ONE, AND MY FIRST ANSWER
  CONFLATED THEM.** This repository has THREE charge concepts and I read one as
  another:
  * the **exploration fireball** — `PlayerProjectileState::charging`, published as
    `BodyPoseView::charge_tier`, drawn as the orb by
    `sync_projectile_charge_visuals`;
  * the **smash-attack** charge — `MovePlayback`'s, published as
    `BodyPoseView::smash_charge`, which drives audio cues in `body_cues.rs`;
  * the **ranged-action** charge — `RangedCharge` on a `RangedActionSpec`, which
    is Projectile Polygon's.
  ⇒ **The orb was never hers.** `RangedCharge::visuals` is documented as "one
  `ProjectileVisualId` per tier": it changes how the FIRED SHOT looks, not how the
  fighter looks while holding it.

  ▢ **DEFECT ONE — the orb never renders in a versus match.** It is gated
  `With<PlayerEntity>`, and every PRODUCTION insertion of that marker is
  `PlayerIdentityBundle` inside `PlayerSimulationBundle`, which hardcodes
  `PlayerSlot::PRIMARY` and `PrimaryPlayer`; the four others are all inside
  `#[cfg(test)]` modules, checked one at a time. There is no `FeatureVisual`
  counterpart. ⭐ This is exactly the defect `flyline.rs` names in bold — *"what
  happened to the trapdoor, and every test it had spawned a `PlayerVisual`, so
  none of them could fail."* Real, and about the fireball rather than about smash.

  ⛔⛔ **"DEFECT TWO" WAS WRONG AND IS RETRACTED — the fact IS published.**
  I claimed a held ranged charge publishes nothing. It does.
  `charge_shot()` authors `smash_charge: Some(SmashChargeSpec { … })` with
  `ChargeGesture::Special`, so `MovePlayback.charge` is populated while she holds;
  `smash_charge_fraction()` reads that same `MoveCharge`; and
  `BodyPoseView::smash_charge` publishes the fraction every tick she is charging.
  ⇒ The name is smash-flavoured and the FACT is general — which is the second time
  in this same investigation I read a name as a scope.

  ▢ **WHAT IS ESTABLISHED, and it is structural rather than a claim about what a
  player sees:** `FeatureView` carries no charge field at all (zero matches for
  `charge` in `view_index.rs`), while `BodyPoseView` carries two. So whatever the
  player road can present, the FEATURE road — the one the `flyline` doc says every
  match fighter takes — cannot present at all, because the fact never reaches it.
  Consumers of `smash_charge` today: `emit_smash_charge_cues` (audio) and a
  `hit_flash` overlay fed from anim frames.

  ⛔⛔ **AND THE HONEST LIMIT: I DID NOT ESTABLISH WHAT A PLAYER ACTUALLY SEES.**
  Three times in this one thread I inferred a user-visible outcome from code
  structure and was wrong twice — the orb was the fireball's, and the "unpublished"
  fact was published under another name. ⇒ **Whether a charging fighter reads as
  charging in a versus match is a question for RUNNING THE DEMO**, not for another
  grep, and the next person on this row should start there. The structural finding
  above stands on its own and does not need that answer.

### ✔ TRACK B IS COMPLETE (2026-09-05), and what it cost

| rung | landed as | engine cost |
|---|---|---|
| B1 successful-defense consequence | `ParriedBodyHit` + `smash.counter`; stand-ins' `riposte` | one message |
| B2 ground tether | Projectile Polygon's grab at 150px + a line on both roads | one read-model field |
| B3 interception | `intercept_projectile`; reflector FREE by composition; Officer's riot shield absorbs | one enum, one shield field |
| B4 per-action muzzle | `Muzzle::Offset`; Projectile Polygon fires from her cannon | one enum variant |

⭐ **Three of the four cost one field or less, and one cost nothing at all.** The
reflector was already shipped the moment the counter existed, because both roads
gate on `parrying()`. That is the campaign's thesis surviving contact: the
authorities were there and what was missing was coordination.

### ✔ THE ONE BLOCKING DECISION IS MADE (and here is who made it)

⭐⭐ **THIS SECTION SAID THE SINGLE MOST UNBLOCKING ANSWER WAS WHAT THE
PERFORMER'S NEUTRAL SHOULD BE. It is answered, and I answered it**, on Jon's
standing grant in the same brief that assigned Sing to her: *"you can pick where
to put the proof of concept for the other moves in the roster"*, and *"we can
tune who the moves belong to later."*

⇒ **The answer that made it a non-decision: Sing did not have to displace
anything.** I had framed it as *upgrade `the_monologue` or give Sing its own
fighter*, and both are design calls. The third option is neither — ADD the pulse
to the move she already has, at a strictly smaller radius, so the existing
balance is untouched and the new thing costs position to land. ⚠ Same shape as
the mine on the down-smash, one hour earlier, and I did not notice it was the
same shape until after I had shipped both.

⚠ **Still Jon's to overrule** — it is his fighter and his numbers (1.4s asleep,
26×26). The guard names the design so a change to it is loud.

### ⛔ WHAT WAS BLOCKED, AND IT IS ONE DECISION RATHER THAN FOUR

Three separate rows are stuck on the same thing — **the roster is full**, and
filling a slot is a design call rather than engineering:

* **Sing** — engine landed and guarded; the Performer has all five specials, and
  her neutral (`the_monologue`) is ALREADY a proto-Sing that holds the room with
  fixed knockback. Upgrading it is a balance change to shipped, documented
  content: adding the sleep makes it much stronger, replacing the knockback
  removes the hit.
* **The stand-ins' last two presses** — `special_neutral_air` and
  `special_back_air`. Four specials have been authored onto them this campaign
  and each one narrows the open question of what they ARE. ⇒ Stopping there
  deliberately.
* **Any further absorber/reflector variants** — same reason; the Officer took the
  one slot that was a borrowed generic rather than an authored move.

⇒ **The single most unblocking answer is what the Performer's neutral should
be**, because it decides whether Sing upgrades an existing move or wants a
fighter of its own.

### ⛔ AND TRACK A IS BLOCKED DIFFERENTLY — ordering, not permission

`TechniqueFlow` runs with an authored customer. The next rung is NOT slots:
a slot binds a symbol to a spawned OCCURRENCE, and projectiles carry no stable
identity (`SimId` exists in `shared_tangle`; nothing in `ambition_projectiles`
uses it). ⇒ **Slots come WITH the steerable projectile, not before it**, which
makes Jon's stated order — input lease, steerable projectile, then PK-Thunder —
the right one. ⓘ The input lease is control-authority work, outside the
`ambition_combat` moveset / `ambition_demo_smash` lane this campaign has held.

⛔⛔ **AND ONE CORRECTION TO THIS SECTION, FOUND BY BUILDING THE THING.** When
this was first written it grouped the **remote mine** with slots, on the reading
that both wanted "a spawned occurrence with a stable identity its owner can look
up". ⇒ **That grouping was wrong, and the mine shipped the same day without any
of it.** `MatchSeat` is already rollback-registered and is already how this
codebase names a fighter durably — so "my mine" is a `usize` comparison.

⭐ **The distinction that was missing: a TRAP needs to name its OWNER, and a SLOT
needs to name the SPAWNED THING.** Those are different questions, and only the
second one is open. A fighter has had a durable name in this codebase for a long
time; a projectile has not.

⚠ The general shape, and it is the same one the reflector produced: **"this row
needs a new capability" is a claim, and the cheapest way to test it is to build
the row.** Two of the three capability gaps this campaign predicted turned out to
be already-shipped authorities under a name I had not searched for.

### ⚠⚠ EIGHT ROSTER DECISIONS MADE UNDER DELEGATION — EVERY ONE IS JON'S TO OVERRULE

Jon's grant was *"you can pick where to put the proof of concept for the other
moves in the roster… we can tune who the moves belong to later."* ⇒ Here is
everything spent against it on 2026-09-05, with what each one COST, so a review
is a table rather than a git log. **Four cost nothing; four displaced something.**

| fighter · slot | change | cost |
|---|---|---|
| Projectile Polygon · down-smash | **+ remote mine** | ⭐ none. Every number on the swing is unchanged; `smash_charge_mult` 1.75 intact |
| the Performer · neutral (`the_monologue`) | **+ Sing pulse**, 26×26 centred on her | ⭐ none. Her 58×34 strike is untouched and the pulse sits wholly inside it — only whoever stood next to her goes under |
| the Patent Clerk · side-B (`reference_frame`) | **+ super armour** over the crossing | ⭐ none. Armour covers `0.20..0.31` only; his startup is still punishable and his locked tail still a free hit |
| Alice · up-B | **+ ±32° aim** on the portal | ⭐ none. Base tilt stays `0.0`, so a neutral stick recovers exactly as before |
| the goblin · down-throw | throw → **cargo carry** | ⚠ the roster's weakest throw (damage 4, fallback clip). No empty slot existed — every fighter authors all four |
| the Author · down-B | archetype low arc → **counter into ambush teleport** | ⚠ a borrowed poke that was never his. Gains a second slot of his own; his `owned_slots` went 1 → 2 |
| the Shadow Oni · down-B | damage-10 poke → **counter into sleep** | ⚠ 10 damage. ⭐ But the move already carried a `counter_ring` and a `parry_flash` sound: the art always said counter |
| **the Officer · neutral** | haymaker → **damageless gust** | ⛔⛔ **THE BIGGEST ONE, and the only KO move lost.** `damage: 13, knockback: 142` becomes a move that does no damage at all. He keeps every smash and `the_draw`; what a player trades is a button that kills for a button that creates space |

⛔ **If exactly one of these is wrong, it is the Officer's** — it is the only
change that removes a way to finish a stock rather than replacing filler or
adding on top. It is also the one that makes his kit a single idea (gun, shove,
shield), so it is a real design choice rather than an oversight, and it is stated
here to be argued with.

### ⭐⭐ A COUNTER IS NOT A MOVE TYPE — THREE ON ONE ROSTER, THREE DIFFERENT MOVES

`answer_a_parry_with_the_authored_counter` dispatches
`SpecialActionSpec::Special(stance.response.clone())`. The response is an
ARBITRARY TECHNIQUE, so "counter" is a trigger, not a reaction:

| fighter | answers a parry with | reads as |
|---|---|---|
| George (stand-in) | `smash.capture_attempt` | a riposte into a grab; reflects shots |
| the Author | `smash.teleport`, **ambush mode** | you commit, and he is behind you; absorbs shots |
| the Shadow Oni | `smash.sleep` | a smoke seal — you wake up on the floor; absorbs shots |

⇒ **None of these needed engine work, and no two of them feel alike.** That is the
campaign's thesis at its cheapest: the composition seam was already there, and
what the roster lacked was somebody spending it.

⛔⛔ **AND THE NINJA'S WAS ALREADY DRESSED AS A COUNTER.** `command_seal` carried
a `counter_ring` effect and a `faction.ninja.parry_flash` sound on top of a plain
damage-10 poke. ⇒ **The presentation announced a parry the mechanics did not
have** — the same class as prose describing code that is not there, except the
reader is the PLAYER, who was told the wrong thing every time the move came out.
Converting it was not a redesign; it was the move becoming what it already looked
like. ⚠ The guard holds both halves: it is a counter now, AND it still wears the
cues that always claimed it was one.

### ⭐⭐ THE CENSUS PAID FOR ITSELF TWICE — TWO MOVES, ZERO ENGINE WORK

Running the "what is built and unused" query as a JOIN rather than a list turned
up **two complete, tested engine capabilities with no authored customer**, and
both became fighters' moves the same day:

| capability | state found in | now |
|---|---|---|
| `VolumeReaction::Windbox` | shipped down to a `WindboxWithDamage` validation error; `hit_reaction` already sets `flinchless` from it — *"this is a push, not a hit"*. **Zero authored windboxes on the roster** | the Officer's neutral: a sustained wall of air that hurts nobody |
| `WindowTag::Armor` | consumed end to end — `MovePlayback` republishes `BodyCombat::armored` every tick, `hit_reaction` gates the launch on `!combat.armored`, tests either side. **No authored move had ever opened one** | the Patent Clerk's pass, armoured for exactly its crossing |

⛔ **IN BOTH CASES WHAT WAS MISSING WAS A WAY TO *SAY* IT.** The engine was
finished; `moveset_authoring` had no verb. `invuln()` existed and `armor()` did
not, one enum variant apart — and the module's own comment on `invuln` already
told the story: *"`WindowTag::Invuln` HAS BEEN AUTHORING VOCABULARY WITH NO WAY
TO SAY IT."* Nobody wrote the sequel.

⇒ **That is this campaign's thesis in its cheapest possible form.** Jon's
acceptance test is *"a move must be expressible and AUTHORABLE easily and
creatively"* — and here the whole gap between an unused engine and two new
fighters' moves was two helper functions.

⚠ **A field census could not have found the second one.** It counts leaves:
`WindboxVolume.repeating` was visible, but a whole reaction with no customer is
not a bool and so is invisible. The BRANCH axis — authored enum variants never
named in content, 149 of 543 — is what surfaced `VolumeReaction::Windbox` and
`WindowTag::Armor`. ⇒ **Ask about variants, not just fields.**

### ⛔⛔ AND ITS WORSE SIBLING: COMPILED IN, NEVER INSTALLED

**The portal recovery was inert in one of the two apps that ship it, and every
test passed.** Found 2026-09-05 by chasing `close_on_transit` — a field that was
not merely dormant but **DEAD**: authored, stored, snapshotted into rollback
state, and read by nothing.

| app | `ambition_platformer2d_runtime` `portal` feature |
|---|---|
| `ambition_app` | ✔ on |
| `ambition_demo_smash_app` | ⛔ **off** |

⇒ **`all_capabilities` turns on the optional `ambition_portal2d` DEPENDENCY; a
separate `portal` FEATURE installs `PortalSchedulePlugin`. Two switches, one
name-shaped concept** — and without the second everything still COMPILES, because
`PlacedPortal` is just a type and the spawn system spawns two of them happily. A
fighter's recovery opened two apertures they fell straight through.

⛔ **Every guard passed because every fixture registers its own systems by hand.**
A hand-built `App` cannot detect a missing PLUGIN: the fixture IS the plugin. The
guards proved the systems work; nothing proved they RUN in a shipped composition.

⭐ **The check, and it belongs in this campaign's acceptance list:** a row is not
landed until `cargo tree -p <each shipping app> -e features -i <providing crate>`
shows the feature on in EVERY app. And the requirement goes in the Cargo.toml of
the crate that REGISTERS the systems, not in each app that composes it.

⛔⛔ **NAME THE FEATURE SET OR THE NUMBER IS ABOUT NOTHING.** `ambition_demo_smash_app`
has `default = []`, so a `cargo tree` with no `--features` measures a composition
nobody runs and reports nearly every capability off. ⇒ My original `0` was
**right by luck**; the peer re-measurement that caught it also showed `audio`
absent from three demos that ship it. Re-measured after the fix, under both:

    smash_app [default]            portal=1  portal_render=0
    smash_app [--features visible] portal=1  portal_render=3

⭐ **And that shape is the fix, not an accident of it.** Simulation is
unconditional — `portal` rides `ambition_demo_smash`'s own dependency, because
that crate registers the systems in every composition including headless tests.
Presentation is not — `portal_render` rides the app's `visible` feature, because
a headless build has no business linking a renderer. ⚠ Stopping at `portal=1`
would have shipped a demo that simulates the recovery correctly and draws no
apertures at all: the same symptom, a different bug.

⚠ **What made it findable was adding the first HARD dependency** — a
`MessageReader` for a message the plugin registers, where the old code only
spawned a component. The breakage had been shipping quietly until something
needed the plugin to actually be there.

### ⭐⭐ A CAPABILITY WITH NO CUSTOMER IS ONE NOBODY CAN TELL IS BROKEN

Twice in one day a technique turned out to be **fully built, guarded, and used by
nobody**:

| capability | state found in |
|---|---|
| `smash.sleep` (Sing) | engine landed the same morning, no authored customer until the Performer got it |
| `TeleportParams::behind_nearest_foe` — the ambush arrival, its foe selection and its facing rule | **no authored customer anywhere in the tree**, until the Author's counter |

⇒ **This is a job for O4's installed-technique catalog, and it is a bigger one
than the catalog was scoped for**: the useful census is not "what techniques
exist" but **"which techniques, and which PARAMETER MODES, no authored content
reaches"**. A list of what exists cannot find these; a JOIN against authored
content can. ⚠ Both were found by accident, which is the argument — nobody was
looking, and nothing would have complained.

### Track A — the keystone

- ▢ **A1. `TechniqueFlow` minimum**: `emit` / `wait` / `branch` / `finish`,
  slots, and the `MovePlayback` extension (current node, latches, issued-event
  bookkeeping, timeouts, symbolic slots). ⛔ No variables, arithmetic,
  expressions, ECS queries, or blackboard.
- ◐ **A2. Input lease** — ⭐⭐ **THE SUBSTRATE IS ALREADY BUILT AND ALREADY
  ROLLBACK-SAFE; WHAT IS MISSING IS A SECOND DRIVER FOR IT.** Read 2026-09-05,
  and this row was written before that read:
  - `DrivingParticipant(PlayerSlot)` — a per-body component, *"the authoritative
    driver identity"*, rollback state *"because no upstream component can
    reconstruct the seat assignment after rewind"*.
  - `ActorControl` — the per-tick control frame, *"a separate component rather
    than a field on `Brain` precisely so a brain swap cannot disturb the frame"*.
    ⇒ That sentence is the input lease's hardest requirement, already met.
  - `TemporaryControl` — which controller masks a body's autonomous brain,
    named by **stable `SimId`, never a raw `Entity`**, with `Player` and
    `Mounted` variants already shipped.
  - ⭐ And **combat already reads `DrivingParticipant` for attribution**:
    `causal::seat_of` resolves a body to a seat, so a body driven by seat N
    already has its hits attributed to seat N. Nothing to build for that.

  ⛔ **THE ONE DRIVER OF ALL THAT IS SINGLE-SEAT AND EXPLORATION-ONLY.**
  `project_driving_participant` is the documented SOLE writer, and it reads one
  resource (`PossessionState`, one `possessed` and one `home`) and hard-wires
  `PlayerSlot::PRIMARY`. Its own comment closes the case: *"A session that never
  possesses — a versus match whose seat-0 fighter legitimately holds PRIMARY —
  never reaches here at all."*

  ⇒ **So A2 is not "build an input lease" and not "it already exists". It is:
  give the existing substrate a per-seat driver a MOVE can operate.** ⚠ And the
  hard constraint that shapes it: a move must FEED that projection, never write
  `DrivingParticipant` itself — the system's own comment describes the failure
  when two holders answer one seat's press, which is *"the exact two-writer state
  this whole component exists to make impossible"*.

  ⚠ **NOT MEASURED**: that a versus match never sets `PossessionState.home`. That
  is the system's claim about itself, quoted, not something this read verified.

- ▢ **A3. Steerable projectile control source** + authored self-contact
  eligibility. ⭐ **AND A2's READ OPENS A CHEAPER ROUTE WORTH PRICING FIRST.**
  This row is blocked on projectiles carrying no stable identity — but
  `TemporaryControl` names its controlled body by `SimId`, and **actors already
  have one**. ⇒ If PK-Thunder's bolt is a short-lived owned ACTOR rather than a
  projectile, it arrives with a `SimId`, a body, a control frame and rollback
  participation already attached, and A3 stops being identity work. ⚠ **NOT
  MEASURED**: what spawning an actor mid-match costs, and whether an actor's body
  is too heavy for something that lives about a second. Price it before choosing
  it — this is a candidate, not a decision.
- ▢ **A4. PK-Thunder parody** as the acceptance fixture that consumes A1–A3.

### Then

Proof moves 4–12 in inventory order, each re-costed when reached. ⛔ Do not
schedule them now — the capability page's rule is that a move earns an engine
change only by introducing a new semantic primitive, and half of these will turn
out to be authoring once the rungs above exist.

## Acceptance, per row

Every row lands with all four or it is not done:

1. the semantic operation is owned by the domain the capability map names, and
   the move does not become the authority for that state;
2. an authored customer exists — a real move in the smash demo, not a fixture;
3. a guard that has been **run red** before it was trusted, with the poison
   named in the commit;
4. rollback participation stated: what per-occurrence memory exists, who owns
   it, and what a rewind does to it.

## The move → composition table

Jon's target expression for each named move, kept here so a builder does not
re-derive it:

| Move | Composition to aim for |
|---|---|
| Peach parasol | recovery move + scoped glide movement modifier + open/close input transitions + ordinary hitboxes |
| Limit meter | character-local resource + full-state timeout + movement/stat modifier + conditional move resolution |
| Ground tether grab | long-reach capture attempt + procedural tether visual |
| Air tether grab | same + generalized aerial capture eligibility |
| Ledge tether | spatial/ledge acquisition + movement-owned tether/reel + recovery route |
| Puff Sing | area/query or hit volume → explicit sleep/control status |
| Disable | targeted hit/query → existing or generalized stun/control lock |
| Reflector | active projectile-interaction volume → projectile-owned reflect/transfer |
| Absorber | same volume → projectile consumes itself → semantic resource/heal effect |
| Thunder from above | transient hazard/projectile at an authored or query-derived spatial relation; it exists whether or not it connects |
| PK Thunder | controlled projectile + input lease + fighter control restriction + correlated self-hit/expire branch + movement launch |
| Nikita | controlled projectile + input lease + control-release transition to ballistic/drop |
| Summon ally | generic spawn + `SummonedBy` + lifecycle + autonomous/commanded control policy |
| Transform | flow requests form change; character-form authority changes the resolved form |
| Sanic Homing Attack | deterministic target acquisition + temporary guided fighter-motion controller + hit/whiff branch |
| Spring Jump analogue | spawn reusable stage actuator + self launch; the aerial version is a falling actuator/hazard |
| Remote mine ✔ | ⭐ **THE TAG TURNED OUT TO BE A SEAT.** `MatchSeat` is already rollback-registered and is already how this codebase names a fighter durably — so "my mine" is a `usize` comparison, not an occurrence-identity scheme, and **this row did NOT need the `SimId` work Track A is blocked on.** ⚠ The scope that bought: ONE mine per seat, which is the rule that makes the arming delay a brake rather than a decoration |
| Revenge-style counter | defensive contact interception → persistent character resource/modifier |
| Cargo carry ✔ | ⭐ **NO "CONSTRAINT CONTRACT" WAS NEEDED.** One bool on the ruleset's half of the hold, one branch in one system, one authored event on a throw-shaped beat. ⚠ **AND THE AUTHORING SEAM, NOT THE MECHANISM, IS WHAT PRICED IT**: the flag could not go on `CaptureAttemptParams` because that struct is constructed literally at 29 sites, so the carry had to be entered from a THROW instead of the grab — which is the genre's own shape anyway. ⇒ Worth recording as the general lesson: in this codebase a shared authored-params struct is the expensive place to add a field, and a new authored EVENT is the cheap one |
| Pocket | projectile interception → immutable stored payload → later projectile respawn |
| Corrin-like pin | directed movement/hit → terrain query → spatial attachment → input branch |
| Sonic-Blade chaining | flow waits for repeat direction/input, reacquires target, runs another scoped motion segment |
| Stone / Withdraw | body/form mode + modified collision/movement + explicit exit conditions |
| Shadow-Flare delayed mark | persistent gameplay occurrence attached to victim + timer/remote trigger |
| Pikmin-like latch | owned secondary entity + body attachment + periodic effect |
| Wind / vacuum ✔ | **LANDED 2026-09-05** on the Officer's neutral special (`officer_disperse`), plus a `moveset_authoring::gust` helper. ⛔ **THE ENGINE HALF WAS ALREADY COMPLETE AND NO FIGHTER USED IT**: `VolumeReaction::Windbox` ships down to a validation error for `WindboxWithDamage`, the push is an ordinary `knockback` + `launch_dir` with `flinchless` set, and `WindboxVolume::repeating` opts a gust out of the hit-once set so it pushes every frame you stand in it. ⇒ Not the flag, the whole reaction — **zero authored windboxes on the entire roster**, so this row's *"generalize to a sustained field"* was pricing work nobody needs. ⭐ What was actually missing was a way to SAY it: three silent invariants (damage must be zero or the catalog rejects it; growth must be fixed or wind obeys a hit's rule; the slash arc must go or a damageless blade swings), now held by the helper and guarded by three poisons |
