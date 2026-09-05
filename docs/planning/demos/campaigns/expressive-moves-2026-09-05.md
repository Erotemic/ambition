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
| 1 ✔ | **PK-Thunder parody** — **LANDED 2026-09-05** on the Author's side-B (`author_train_of_thought`) | ⭐⭐ **IT NEEDED NEITHER INPUT DELEGATION NOR FIGHTER-CONTROL SUPPRESSION.** ⛔ And **he is NOT rooted while it flies** — I wrote that he was, in three files, and the code never agreed: the move roots him to 0.46s and the bolt lives 2.2s, a gap the guard REQUIRES so a whiff cannot pin him through his own punish window. ⇒ So **one stick does both**: walking right also steers the bolt right. The cost is not helplessness, it is DIVISION. `steer_axis()` already publishes what the PLAYER is holding as distinct from what the BODY may move by, so the caster stays rooted, keeps his own seat, and the bolt reads his live stick — steering, not possession. ⇒ The rung this row was blocked behind (A2) is not on its path. ⛔ And it did NOT need `TechniqueFlow` either: one technique, one component, one system |
| 2 ✔ | **Counter + Revenge variant** — the stand-ins' `riposte`, and **2026-09-05 the Author's `author_second_draft`** | ⭐⭐ **"COUNTER" TURNED OUT NOT TO BE A MOVE TYPE AT ALL.** `answer_a_parry_with_the_authored_counter` dispatches `SpecialActionSpec::Special(stance.response.clone())` — fully generic — so a counter is **ANY TECHNIQUE, TRIGGERED BY A SUCCESSFUL PARRY**. Counter-into-grab (George), counter-into-teleport (the Author), counter-into-mine or -sleep are all authorable today with zero engine work. ⇒ I built this seam and then under-read it for a day |
| 3 ✔ | **Reflector + absorber** — **LANDED 2026-09-05** as Track B's B3 | projectile interception is a projectile authority and supports more than parry reversal. ⭐ **The reflector cost NOTHING**: `step_projectiles` and the melee seam gate on the same `parrying()` window, so a stance that could counter could already return shots. The absorber cost one enum and one shield field, and is the Officer's riot shield. ⚠ This row sat unticked while its own Track B row said complete — corrected 2026-09-05 |
| 4 ◐ | **Tether recovery + aerial tether grab** — **GROUND HALF LANDED** as Track B's B2 | terrain/body tethering composes with ledge and capture authorities. ✔ **Ground tether**: Projectile Polygon's grab reaches 150px and draws a line on both roads, for one read-model field. ▢ **The AERIAL half is NOT done** — a tether that catches a LEDGE and pulls her back is the recovery half of this row. ⭐ **AND IT IS MOSTLY ASSEMBLED ALREADY**: `TeleportParams::ledge_assist` is a shipped, authored parameter whose doc calls it *"the aim assist… the difference between a recovery and a coin flip"* — within its radius an arrival is placed STANDING on the ledge. ⇒ So "a recovery that catches a ledge" EXISTS; what a tether adds is the visible line, and `TetherVisual` landed in B2. ⚠ Its one gap: `TetherVisual` names a `body: Entity` and draws to *"where its GRAB actually reaches"*, so drawing to a teleport destination is a small extension rather than a reuse.

⛔⛔ **AND I OVER-ESTIMATED HOW ASSEMBLED IT IS — correcting my own estimate from an hour earlier on this same page.** `acquire_captures` catches BODIES and a ledge is not one, so her 150px grab cannot become a ledge tether by reach alone. The probe exists (`probe_ledge_grab_in_frame`, which could be called at a VIRTUAL position — *"could a body standing there catch a ledge"* — a legitimate reuse) ⚠ **but it has NO production caller outside its own tests**, and it wants a `&World`, so wiring it into a move is engine work rather than composition. ⇒ The honest cost of this row is a technique plus a probe adapter plus the visual extension — comparable to the bolt, not to the mine. ⭐ `ledge_assist` on a TELEPORT remains the cheap way to get *"a recovery that catches a ledge"*; what it cannot give you is a tether. ⚠ Marked ◐ rather than ✔ deliberately: B2's own row says "ground tether", and reading that as the whole row is how a half-built capability gets reported as finished |
| 5 ✔ | **Cargo command grab** — **LANDED 2026-09-05** on the goblin's down-throw | ⭐ **MOVEMENT WAS NEVER TOUCHED.** The claim was "capture and movement cooperate", and what actually happened is that capture RESTRICTS LESS: `restrict_captor_control` zeroed `locomotion` for every captor, and a carry is a hold that does not. ⇒ `carrying` rides `SmashHoldState` (the ruleset's half) rather than `CapturedBy` (the generic relation), on that component's own argument that platform-fighter rules do not belong on the relation |
| 6 ✔ | **Remote mine** — **LANDED 2026-09-05** on Projectile Polygon's down-smash | persistent spawned identity, owner lookup, remote triggering, rollback. ⭐ **It cost one component with four fields and no new authority**: the object is a `GroundItem`, the blast is a `DamageBoxEffect`, and "where is it, whoever holds it" is `ItemWorldPos`. The mine contributes an arming clock and a decision |
| 7 ▢ | **Parasol** | temporary movement modifiers/controllers are expressive enough for a long-lived post-move locomotion regime. ⛔ **MEASURED 2026-09-05 AND GENUINELY MISSING** — the one open row where the search found nothing. `ActorSurfaceState::gravity_scale` is per-body and already suspended/restored by capture (`prior_gravity_scale`), but **no TIMED modifier exists**: nothing owns "this scale, for N seconds, then back". ⇒ That is the smallest honest shape of this row, and it IS new state — so it wants the campaign's rule applied deliberately: a move may ask for it and must not own it |
| 8 ◐ | **Homing Attack** | deterministic semantic target queries and target-directed fighter motion. ⚠ **THE ROW SPLITS, AND THE CHEAP HALF DODGES WHAT IT WAS MEANT TO PROVE.** The TARGET QUERY exists — the teleport's ambush arrival does deterministic foe selection through `combat_relation`, *"the same call the damage side makes, so a teammate cannot become a target here after ceasing to be one there"* — but it is PRIVATE to `teleport.rs` (`fn may_ambush`), so it is built and not reusable. ⇒ A "homing attack" is authorable today as an ambush teleport plus a strike window, and **that would tick the row while skipping target-directed MOTION**, which is the half worth having. ⛔ Not doing it that way |
| 9 ✔ | **Sing** — **COMPLETE 2026-09-05** on the Performer's neutral special | `BodyCombat::sleep_timer`, the `smash.sleep` technique and the area adapter, all guarded — and now with a customer. ⭐ **ADDED TO `the_monologue`, NOT SUBSTITUTED FOR IT**: her strike is 58×34 out in front and is unchanged to the number; the sleep is 26×26 centred on HER, wholly inside it. Everyone still gets the speech; only whoever stood next to her goes under |
| 10 ▢ | **Limit** | character-local resource state, threshold transitions, timeout, action variants, stat modifiers. ⓘ **THE NEAREST SHIPPED THING IS `StoredMoveCharge`** — per-body, rollback-registered, with a value probe — but it is a CHARGE BANK keyed by move id, filled by holding a button. A meter filled by combat EVENTS and read by a move SELECTOR is a different object, and the selector half (`move_for_directional_verb` varies only on `grounded`) is the part with no precedent |
| 11 | **Free-standing ally summon** | summon is not synonymous with mount; first owned-secondary-actor contract |
| 12 ▢ | **Reusable launch object** | a fighter can create a persistent world actuator another fighter interacts with. ⓘ **Searched 2026-09-05: no placed launcher exists.** `PogoPolicy` is the nearest and is a different thing — bouncing off a body you HIT, not off an object somebody placed. ⭐ But the mine proves the spawning half is cheap (`GroundItem` + a seat-owned component), so what this row really needs is the ACTUATOR contract: a thing that launches whoever touches it, including its owner |
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

---

✔✔ **AND EVERY WORD ABOVE IS SUPERSEDED — kept because being wrong in a
particular way is the finding, not because the section is still true.** All three
rows were called blocked on "the roster is full", and none of them was:

* **Sing** landed the same day on `the_monologue`. ⛔ The framing was a FALSE
  DICHOTOMY — "upgrade her move OR give Sing its own fighter" — and the third
  option displaced nothing: ADD the pulse at a strictly smaller radius, so her
  strike is untouched to the number and only whoever stood next to her goes under.
* **Further absorber/reflector variants** were not blocked either. Three more
  counters landed (the Author, the Shadow Oni, Emmy), each answering with a
  DIFFERENT technique, because the response was always an arbitrary one.
* **The stand-ins' two presses** is the one row that stands — and it stands for
  the reason given, which was never scarcity: authoring more onto a fighter whose
  identity is an open question narrows that question by default.

⭐ **The lesson, and it is the same one this campaign kept re-learning: "blocked"
was a claim about the ROSTER when it was really a claim about my own framing.**
Twelve roster decisions later, eleven of them found a seam nobody had to lose
something for. ⇒ Before recording a row as blocked on a design call, look for the
version that costs nothing — it existed here three times out of three.

### ⛔ AND TRACK A IS BLOCKED DIFFERENTLY — ordering, not permission

~~`TechniqueFlow` runs with an authored customer. The next rung is NOT slots: a
slot binds a symbol to a spawned OCCURRENCE, and projectiles carry no stable
identity.~~

⛔⛔ **THAT PARAGRAPH WAS WRONG AND IT IS STRUCK RATHER THAN QUIETLY EDITED,
because it was published as a blocker and somebody could have planned around it.**
Projectiles carry a stable, deterministic, ROLLBACK-CANONICAL identity:

    ProjectileSeq(u64)      "Monotonic spawn-sequence id", registered as
                            `projectile.seq` — component-canonical
    ProjectileOwner(Entity) with `MapEntities`, remapped across a restore

⇒ **Slots are NOT blocked on identity work.** A slot can bind to a spawned
projectile today, by `ProjectileSeq`, which the step loop already sorts on so
that iteration order is deterministic.

⚠ **How the claim went wrong is the useful part: it was true about the word I
searched and false about the thing I meant.** `SimId` genuinely is unused in
`ambition_projectiles` — I checked that and reported it accurately — but "no
`SimId`" is not "no identity", and I published the second as though I had
measured it. ⇒ **This is the THIRD wrong absence claim in this campaign** (after
the trap's owner identity and the input lease), and all three were assertions
that something did not exist, made from one spelling.

ⓘ The input lease is control-authority work outside the `ambition_combat`
moveset / `ambition_demo_smash` lane — and, per the A2 note below, is not on
PK-Thunder's path at all.

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

### ⛔⛔ A DEFERRAL IS A MEASUREMENT, AND THIS CAMPAIGN EXPIRED TWO OF ITS OWN

A comment that says *"the engine cannot do X, so this move does Y instead"* is a
measurement of the engine on the day it was written. **This campaign's whole
output is invalidating those**, and nothing goes back to update them:

| where | what it said | why it is now false |
|---|---|---|
| Emmy's `conservation_law` ✔ **and her counter has since landed on `invariant_field`** | *"not the counter the sheet's blueprint imagined — `MoveSpec` has no absorb or reflect, and inventing one for one character would be the wrong shape"* | `smash.counter` carries `absorbs_projectiles` and an arbitrary response, authored on **three** fighters — shared, not bespoke, so both halves of the objection are answered |
| Projectile Polygon's `charge_shot` | *"a per-action MUZZLE offset would fix both ends at once; **it does not exist**, and inventing one here would be a fighter reaching into the shared fire site"* | `Muzzle::Offset` landed as Track B's B4 **with her as its customer**, two files away in `authored/projectile_polygon.rs` |

⚠ **The second is the worse one: her own sibling file already uses the thing her
moveset file says does not exist.** Both are corrected in place rather than
deleted, because the reasoning was RIGHT when written — the muzzle did belong in
the shared fire site, and a bespoke counter would have been the wrong shape.

⇒ **The rule this earns: when a capability lands, grep the content crates for the
deferral that asked for it.** A deferral records who was waiting; shipping without
telling them leaves a comment that reads as checked and is not. ⭐ And Emmy's is
now an OPPORTUNITY rather than a refusal — her blueprint wanted a counter and the
engine offers one, which is a design call for Jon rather than a limit.

### ⭐⭐ FOUR MOVES WHOSE OWN COMMENT OR ART DESCRIBED A MECHANIC THEY DID NOT HAVE

Not documentation drift — **the reader here is the PLAYER**, told the wrong thing
every time the move came out. Found by reading each fighter's design comments and
cues against what the move actually did:

| move | what its art and comment said | what it did | now |
|---|---|---|---|
| the Shadow Oni's `command_seal` | a `counter_ring` effect and a `faction.ninja.parry_flash` sound | a plain damage-10 poke | a counter that answers with smoke |
| the automaton's `generation_collapse` | *"everything inside it arrives at the same cell"*, drawing a `causal_cone_collapse` | ⛔ **INVERTED** — `launch_dir: (0.7, -0.68)` threw victims AWAY | three autolink pulses that gather, then the finisher launches |
| Alice's portal | Jon's *"angled portals with directional input"* | `tilt_degrees: 0.0` everywhere | the player's stick leans it ±32° |
| Bob's `rivet_gun` | *"it is not one hit, it is **the tool running**"* | ⛔ one `strike` with a single contiguous `active_s: 0.14` — which the re-hit rule lands **exactly once** | three separated holding pulses, then the same finisher |

⛔ **NONE OF THE THREE NEEDED ENGINE WORK.** `smash.counter`, `VolumeReaction::Autolink`
+ `multihit`, and `tilt_degrees` were all shipped. ⇒ Converting them was not
redesign — it was each move becoming what it already looked like.

⭐ **The method, and it is cheap enough to run on any fighter:** read the design
comment and the authored cues, then check the mechanics against them. A comment
that names a feeling ("the order is obeyed instantly", "the cone closes", "the
tool running") is a SPECIFICATION somebody wrote and nothing verifies.

⭐⭐ **AND THE HIT RATE IS WORTH RECORDING: four of the nineteen fighters, found by
reading — and the sweep also cleared Carl and Oiler**, whose comments describe
exactly what their moves do. ⚠ Oiler's `convergence` is the model: its comment
does not merely claim a multi-hit, it explains the GAP that makes one work. **A
comment that says WHY is one that was checked.**

⛔ Bob's is the sharpest of the four, because the comment names the precise
property the engine REFUSES. `Pulse`'s own doc says it: *"a multi-hit that
authored one long window, or windows that touch, lands exactly once."* ⇒ Nobody
writing "it is not one hit" had read that, and nothing connected the two.

### ⛔⛔ THE BOLT'S FIRST GUARD RUN FOUND A MOVE THAT COULD NOT BE PLAYED

Not a test artefact — a defect the first run exposed and a player would have hit
on every single press. **The bolt spawns at a body-local offset, which is INSIDE
its caster's contact box, so it came home on the tick it was fired and threw him
instantly.** Every use was a self-launch and the bolt was never seen.

⇒ The fix is a latch (`clear_of_caster`), and it is also the genre's rule arrived
at from the bug rather than from the reference: **a bolt cannot answer its caster
until it has left him**, so flying it back is a manoeuvre instead of an accident
of where it starts. ⭐ Removing that latch again fails THREE of the six guards,
which is the shape of a fact the suite actually depends on rather than one test's
private assumption.

⚠ **The general form, for anything a move spawns AT a fighter:** the spawn point
is inside the spawner. Contact logic that does not say so is not "usually fine",
it is wrong on frame one — and frame one is the only frame that matters, because
it happens before anything else can.

### ⓘ WHERE THE ROSTER STANDS, AND WHY THE OBVIOUS METRIC IS WRONG

**Nine fighters gained an expressive move on 2026-09-05**: Projectile Polygon
(mine), the Performer (Sing), the goblin (cargo carry), the Author (counter AND
the steered bolt), Alice (portal aim), the Officer (gust), the Patent Clerk
(armour), the Shadow Oni (counter, plus a cue repair), and the cellular automaton
(the collapse that finally collapses).

⛔⛔ **DO NOT MEASURE THIS BY COUNTING `smash_*::` CALLS PER FIGHTER. I tried, and
it reports the goblin and the automaton at ZERO** — the goblin's carry is a
CAPTURE-family technique and the automaton's collapse is a `VolumeReaction`, not
a `smash_*` helper at all. Alice's angled portal is a PARAMETER on a technique she
already had. ⇒ Three of the nine are invisible to the obvious grep, so a count
that looks objective would rank the two fighters improved most recently as the
least improved.

⭐ **The real signal is not countable and should not be faked:** whether a
fighter's four specials do four DIFFERENT KINDS of thing. That needs reading the
moves, which is how every one of today's finds was made.

▢ **Four fighters remain as candidates** — `bob`, `carl_stargan`, `emmy_noether`
and `oiler` — none of which author a technique yet. ⚠ `pointed_polygon` and
`pugnacious_polygon` are deliberately excluded: they are the sword and brawler
REFERENCE fighters, and their own comments say a new humanoid should copy them
before it has a reason to differ. Giving them a bespoke technique would corrupt
the template.

⭐ **And Emmy is the best of the four**, because her file already says what she
wants: her blueprint asked for a counter and the deferral recording why she could
not have one has now expired.

### ⚠⚠ TWELVE ROSTER DECISIONS MADE UNDER DELEGATION — EVERY ONE IS JON'S TO OVERRULE

Jon's grant was *"you can pick where to put the proof of concept for the other
moves in the roster… we can tune who the moves belong to later."* ⇒ Here is
everything spent against it on 2026-09-05, with what each one COST, so a review
is a table rather than a git log. ⚠ **This table was written at EIGHT and is now
TWELVE — I updated it rather than letting it read as complete**, which is the same
failure as a stale deferral and would have been worse here, because its whole
purpose is to be the thing Jon reads instead of the log.

**Six cost nothing; five displaced something; one is presentation only.**

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
| the cellular automaton · down-B | strike → **the same strike with three autolink pulses in front** | ⭐ none — the finisher is unchanged; the pulses are 2 chip each. It only does what its own comment always claimed |
| the Author · **side-B** | archetype `vector_lunge` → **PK-Thunder** | ⚠ a borrowed poke that was never his. ⭐ Jon's own assignment, and his third owned slot |
| the Shadow Oni · aerial down-B | `counter_ring` + `parry_flash` cues **removed** | ⚠ presentation only, and it is MY DEBT: converting his grounded seal to a real counter made the dive's borrowed parry cues a trap |
| Emmy Noether · down-B | `invariant_field` → **counter that conserves the blow** | ⚠ her CHEAPEST special (`damage: 6`), and she keeps a second ground-claiming field. ⭐ Her own blueprint asked for a counter; the deferral saying she could not have one expired this morning |

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
| Emmy Noether | `smash.vitality` | ⭐ **the theorem as a move**: a symmetry implies a CONSERVED QUANTITY, so the energy you put in is kept. She is simply better off for having been hit |

⇒ **FOUR counters now, none needing engine work, and no two feel alike.** That is the
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

- ⛔⛔ **AND A2 IS NOT ON PK-THUNDER'S PATH AT ALL — measured 2026-09-05.**
  `ActorControlFrame::steer_axis()` already publishes **"what the PLAYER is
  HOLDING, as opposed to what this body is ALLOWED to move by"**, and its doc
  says why it exists: *"`update.rs` PUBLISHES THE DAMPED FRAME back onto the
  component after integration, so a consumer reading `locomotion` off an actor's
  `ActorControl` sees zero for the whole of a rooted move."* The B-reverse flick
  already reads it. ⇒ **A move-scoped system can read the caster's live,
  undamped stick every tick, while the caster stays rooted and keeps their own
  seat.** No `DrivingParticipant` rewrite, no `TemporaryControl`, no two-writer
  hazard, no lease.

  ⭐⭐ **THE DISTINCTION THE PLAN WAS MISSING: STEERING IS NOT POSSESSION.**
  · *Steering* — I stay put and my stick moves something else. Needs only
    `steer_axis()`, which is shipped. **This is PK-Thunder.**
  · *Possession* — my input drives another body through ITS OWN action set, and
    my avatar goes inert. That is what `TemporaryControl` and a per-seat driver
    are for, and no move in this campaign asks for it.

  ⇒ **So Jon's stated order (lease → steerable projectile → PK-Thunder) can drop
  its first rung.** A2 stays open as real work for a genuine possession move; it
  is simply not a blocker for the Author's side-B.

- ✔ **A3. Steerable projectile control source** + authored self-contact
  eligibility — **LANDED 2026-09-05** as the Author's side-B. ⛔ **AND BOTH
  PREMISES BELOW WERE WRONG**, kept for the record: this row was never blocked on
  identity (`ProjectileSeq` is rollback-canonical — see the struck paragraph
  above), and it did not need the actor route either. `steer_axis()` was enough.
  ⇒ What follows is the reasoning as it stood BEFORE the read, and it is exactly
  the shape of over-pricing this campaign kept producing:

  ~~This row is blocked on projectiles carrying no stable identity — but
  `TemporaryControl` names its controlled body by `SimId`, and **actors already
  have one**. ⇒ If PK-Thunder's bolt is a short-lived owned ACTOR rather than a
  projectile, it arrives with a `SimId`, a body, a control frame and rollback
  participation already attached, and A3 stops being identity work. ⚠ **NOT
  MEASURED**: what spawning an actor mid-match costs, and whether an actor's body
  is too heavy for something that lives about a second. Price it before choosing
  it — this is a candidate, not a decision.~~
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
