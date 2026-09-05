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
| 7 ✔ | **Parasol** — **LANDED 2026-09-05** on Pugnacious Polygon's up-B | temporary movement modifiers/controllers are expressive enough for a long-lived post-move locomotion regime. ⛔ **MEASURED 2026-09-05 AND GENUINELY MISSING** — the one open row where the search found nothing. `ActorSurfaceState::gravity_scale` is per-body but **no TIMED modifier exists**: nothing owns "this scale, for N seconds, then back". ⛔⛔ **AND THE REAL BLOCKER IS SHARPER THAN THAT, re-measured after a first pass called it "the smallest new-state row": THREE domains already write `gravity_scale` with the same save-set-restore pattern** — capture (`prior_gravity_scale`), mount (restoring from a spawn baseline) and body-seed at construction. ⇒ A fourth writer cannot simply join them: a fighter who is floating AND then grabbed has two saved priors and the restore order decides which one wins. ⭐ So the honest shape is not "add a timer" — it is **a modifier the MOVEMENT domain owns and multiplies**, which the existing writers would then compose against instead of overwriting. That is engine work in `ambition_platformer2d_core`, and it is the row where this campaign's rule bites hardest: a move may ASK for a locomotion regime and must never become the authority for it. ⛔⛔ **AND THE BLOCKER DISSOLVED ON RE-MEASUREMENT 2026-09-05 — the fourth instance this week of an obstacle that was a sentence nobody re-read.** **THE KERNEL ALREADY COMPOSES GRAVITY SCALES BY MULTIPLICATION, and has two of them:** `integration.rs:773` is `*kin_vel += frame.gravity_acceleration() * (water_gravity_scale * jump_gravity_scale) * dt`. ⇒ The *"modifier the movement domain owns and multiplies"* this row asks for is not a new mechanism — **it is a third factor in a product that already exists**, resolved onto `NormalSpineCtx` (a `Copy` read-only gating struct whose fields the caller resolves, which is exactly how `water` and `crouching` already arrive). ⭐⭐ **AND THAT IS WHY THE RESTORE-ORDER ARGUMENT ABOVE DOES NOT APPLY TO IT.** The three save-set-restore writers all feed `ActorSurfaceState::gravity_scale`, which `gravity/resolve.rs:56` folds into the frame BEFORE the kernel's product. A timed move modifier joins at the OTHER end and **saves no prior at all**, so there is nothing to restore and no order to get wrong. ⇒ The row's stated blocker was a property of the approach it assumed (become a fourth writer of that field), not of the problem. ⚠⚠ **ONE REAL HAZARD FOUND WHILE MEASURING, AND IT IS THE FAMILIAR SHAPE:** `resolve.rs` has two arms, and the PLAYER arm is `let player_response = tuning.gravity;` — **`surface.gravity_scale` is not in it.** So that field does nothing on a `PlayerEntity`. ⇒ Smash fighters are safe (measured: nothing in `ambition_demo_smash` or its app carries `PlayerEntity`, so they resolve through the actor arm), but a Parasol authored against `surface.gravity_scale` would be **inert on an exploration player** — authored, paid for, and unreachable on one of the two paths, which is this campaign's most repeated finding |
| 8 ✔ | **Homing Attack** — **LANDED 2026-09-05** on Carl Stargan's slingshot side-B | deterministic semantic target queries and target-directed fighter motion. ⭐⭐ **AND THE TARGET QUERY WAS ALREADY PUBLIC** — `ambition_combat::targeting::assisted_fire_direction`, cone- and range-bounded, **tie-broken on the stable `SimId` rather than the `Entity`** because bevy_ggrs recreates rollback entities. ⇒ The move ASKS it every tick and owns no targeting; only the MOTION was new. ⚠ My earlier note that the query was "private to `teleport.rs`" was about `may_ambush` specifically and misread the general case. ⚠ **THE ROW SPLITS, AND THE CHEAP HALF DODGES WHAT IT WAS MEANT TO PROVE.** The TARGET QUERY exists — the teleport's ambush arrival does deterministic foe selection through `combat_relation`, *"the same call the damage side makes, so a teammate cannot become a target here after ceasing to be one there"* — but it is PRIVATE to `teleport.rs` (`fn may_ambush`), so it is built and not reusable. ⇒ A "homing attack" is authorable today as an ambush teleport plus a strike window, and **that would tick the row while skipping target-directed MOTION**, which is the half worth having. ⛔ Not doing it that way |
| 9 ✔ | **Sing** — **COMPLETE 2026-09-05** on the Performer's neutral special | `BodyCombat::sleep_timer`, the `smash.sleep` technique and the area adapter, all guarded — and now with a customer. ⭐ **ADDED TO `the_monologue`, NOT SUBSTITUTED FOR IT**: her strike is 58×34 out in front and is unchanged to the number; the sleep is 26×26 centred on HER, wholly inside it. Everyone still gets the speech; only whoever stood next to her goes under |
| 10 ◐ | **Limit** | character-local resource state, threshold transitions, timeout, action variants, stat modifiers. ⭐⭐ **THE METER IS SHIPPED AND I HAD THE WRONG NEAREST NEIGHBOUR.** `BodyMana { meter: ResourceMeter { current, max, regen_rate, decay_rate } }` is per-body, **rollback-canonical as `body.mana`**, published to presentation through `sim_view::facts`, ~~and already regenerating — so a first rung needs NO fill system at all.~~ ⛔⛔ **STRUCK 2026-09-05, AND IT WAS WRONG IN THE DIRECTION THAT FLATTERED THE ROW.** Measured, not read: **every `BodyMana` in the workspace is `ResourceMeter::new(100.0, 0.0, 0.0)` — regen ZERO** — and its only refill is the platformer's shrine, which no smash stage has. ⇒ A cost today buys a fixed number of uses per life and no way to earn another. **The gate was the cheap half; the FILL is a design decision and it is Jon's.** ⇒ **What is missing is the GATE**, and its shape is now located: the cost is authoring data and belongs on `MoveGates`, but the CHECK cannot go in `MoveGates::permits` — that is called from `ambition_entity_catalog`, a data crate which must not learn about body state. The check belongs at ACCEPTANCE in `ambition_combat::moveset`, where the body is in hand. ⛔ **AND THE OBSTACLE I NAMED WAS OVERSTATED — CHECKED BY COUNTING.** The sixteen-limit had already been hit and SOLVED: the body QUERY tuple reached it and nested a gesture triple to fit (now 14), while the SYSTEM itself carries 12 params. There was room, and the elegant answer Jon asked for was already sitting in the file — `guards: Query<&mut BodyShieldState>` and `jumps: Query<&mut BodyJumpState>` are BOTH spend-sites looked up by entity, so the meter became the third of exactly that shape rather than a fifteenth query member. ⇒ The earlier note said the acceptance authority is AT BEVY'S 16-PARAMETER CEILING and that its own comment says so — *"Bevy's `QueryData` tuple runs out at sixteen and this query reached it"* — so a `BodyMana` reader has to JOIN an existing grouped param rather than become a new one. ✔ **LANDED 2026-09-05 — the ENGINE half, deliberately without a priced move.** `MoveGates::meter_cost` (`#[serde(default)]`, so every move authored before it still costs nothing); `afford_meter`, the **third sibling** of `afford_recovery` and `permitted_while_held`, shaped like both on purpose — read-only, asked BEFORE any teardown, returning a plain bool; the spend at `start_move`, the one point both roads meet. ⛔ **REFUSES, NEVER SILENTLY NO-OPS** — `MoveGates`' own doc names that failure from the pirate's shark up-B: *"a rule enforced after acceptance is not a rule, it is a silent failure with a comment."* ⭐ **ROLLBACK: no schema bump.** `body.mana` is already `component-canonical` (codec snapshot + checksum projection); the cost is authored data and the spend mutates a field the codec already carries. ⚠ **NO AUTHORED CUSTOMER, AND THAT IS THE HONEST STATE OF THE ROW** — pricing a move before anything fills the meter would ship a special that works twice and then never again. The field is what any fill rule would spend. ⓘ `StoredMoveCharge` was my earlier guess at the nearest neighbour and it is the wrong one — a per-move CHARGE BANK, not a character meter|
| 11 ◐ | **Free-standing ally summon** | summon is not synonymous with mount; first owned-secondary-actor contract. ⭐⭐ **THE ENGINE ALREADY AGREES — measured 2026-09-05.** `Effect::Summon`'s spec carries `ridden_by_summoner: Option<SummonedRide>`, so **`None` IS a free-standing summon**, with `faction`, `health` and `keeps_contact_damage` already on it. The shark is the `Some(..)` case. ⇒ Summon and mount were never synonymous in the ENGINE; they are synonymous in the AUTHORING, because `author_summon_ride` is the only path and it always rides. ⇒ What is actually owed: a technique that asks for the un-ridden case, plus a LIFETIME (the spec has none — the shark's `seconds` is the RIDE's) which the mine and plate show costs one component with a clock. ⛔ **NOT BUILT, and the reason is the goal rather than the cost:** a summoned ally's value in a 1v1 human match is whatever its own BRAIN does, and CPU AI is explicitly deprioritised. A cheap row whose payoff sits behind a deprioritised one is not the next row |
| 12 ✔ | **Reusable launch object** — **LANDED 2026-09-05** on Bob's down-B, additive to his slam | a fighter can create a persistent world actuator another fighter interacts with. ⭐ **AND THE WORD THAT MATTERS IS *ANOTHER***: the plate throws ANYBODY who steps on it, its dropper and his opponent alike — a plate that served only its owner would be a second recovery wearing an object's clothes. ⛔ THREE clocks and a use count, all rollback state. ⓘ Originally: **no placed launcher exists.** `PogoPolicy` is the nearest and is a different thing — bouncing off a body you HIT, not off an object somebody placed. ⭐ But the mine proves the spawning half is cheap (`GroundItem` + a seat-owned component), so what this row really needs is the ACTUATOR contract: a thing that launches whoever touches it, including its owner |
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

### ⭐⭐ FIVE MOVES WHOSE OWN COMMENT OR ART DESCRIBED A MECHANIC THEY DID NOT HAVE

Not documentation drift — **the reader here is the PLAYER**, told the wrong thing
every time the move came out. Found by reading each fighter's design comments and
cues against what the move actually did:

| move | what its art and comment said | what it did | now |
|---|---|---|---|
| the Shadow Oni's `command_seal` | a `counter_ring` effect and a `faction.ninja.parry_flash` sound | a plain damage-10 poke | a counter that answers with smoke |
| the automaton's `generation_collapse` | *"everything inside it arrives at the same cell"*, drawing a `causal_cone_collapse` | ⛔ **INVERTED** — `launch_dir: (0.7, -0.68)` threw victims AWAY | three autolink pulses that gather, then the finisher launches |
| Alice's portal | Jon's *"angled portals with directional input"* | `tilt_degrees: 0.0` everywhere | the player's stick leans it ±32° |
| Bob's `rivet_gun` | *"it is not one hit, it is **the tool running**"* | ⛔ one `strike` with a single contiguous `active_s: 0.14` — which the re-hit rule lands **exactly once** | three separated holding pulses, then the same finisher |
| Carl's `planetary_orbit` | draws **`orbit_lock`** at 0.36s — a LOCK | ⛔ a straight `impulse(700, 0)` that locks onto nothing | a homing dash inside a 60° cone; an unaimed pass is the same straight dash it always was |

⛔ **THREE OF THE FIVE NEEDED NO ENGINE WORK AT ALL.** `smash.counter`,
`VolumeReaction::Autolink` + `multihit`, and `Pulse`'s separation rule were all
already shipped, so the ninja's, the automaton's and Bob's were not redesign —
each became what it already looked like. ⚠ **The other two were not free, and
saying so is the point:** Alice's tilt was authored-but-inert until the `portal`
FEATURE was added to the demo's build, and Carl's `orbit_lock` needed a NEW
technique (`smash.homing_dash`), because nothing shipped could bend a dash toward
a target. ⇒ **Reading found all five; only three were cheap to answer.**

⭐ **The method, and it is cheap enough to run on any fighter:** read the design
comment and the authored cues, then check the mechanics against them. A comment
that names a feeling ("the order is obeyed instantly", "the cone closes", "the
tool running") is a SPECIFICATION somebody wrote and nothing verifies.

⭐⭐ **THE SWEEP IS NOW COMPLETE ACROSS ALL NINETEEN FIGHTERS: five flagged moves,
all five fixed, and the rest positively CLEARED.** ⛔ Count MOVES, not fighters —
**Bob and Carl each appear in BOTH lists**, so a fighter with one move that lies
can own another that is scrupulous, and stopping at the fighter would have missed
one of each. ⚠ Oiler's `convergence` is the model: its comment
does not merely claim a multi-hit, it explains the GAP that makes one work. **A
comment that says WHY is one that was checked.**

⇒ **The four cleared hardest, because each LOOKED like a hit:**
`slick_dash` (*"he oils the floor under himself"* — the oil is him sliding, and
its second paragraph names the unlocked `motion_scale: 1.0` tail that makes it
so), `pressure_vent` (*"everything in the seal goes at once"* — and the paragraph
below names `start_impulse` ADDING rather than setting), `bulkhead_drop` (*"he
drops a plate"* — an animation, and its `shockwave`/`landing_puff` cues agree),
and Carl's `cosmic_calendar`. ⚠ **A method that only ever flags is a suspicion.**

### ⭐⭐ THE SWEEP FINISHED, AND THE HYPOTHESIS IT SUGGESTED WAS REFUTED

**Six fighters were still unread when this section first claimed to be complete**
— `archetype`, `medic`, `pirate_admiral`, `player_robot`, `pointed_polygon`,
`pugnacious_polygon`. ⇒ The claim was written from the thirteen I had touched.
Finishing them cleared all six, so the sentence is now true; it was not when I
wrote it, and the check that caught it was counting the files.

⛔⛔ **AND THE OBVIOUS PREDICTOR IS WRONG. MEASURED, NOT ASSUMED.** The tempting
rule after five hits — *"the liars are the ones without tests"* — does not
survive contact: **`#[test]` count per file averages 4.0 for the five that lied
and 5.4 for the fourteen clean**, which is noise. **Carl had SEVEN tests and
still drew `orbit_lock` over a straight impulse.**

⭐ **The variable is not how many tests a fighter has; it is whether ANY test
names the mechanic the ART asserts.** Measured on Carl at the commit before the
fix: `orbit_lock` appears **five times in his authoring code and zero times in
his seven tests**. His suite asserted frames, budgets and shapes — every one of
them true — and nothing connected the effect name to the impulse under it.

⇒ **So the instrument is a census, not a reading.** Every effect name authored in
`ambition_content`, checked against whether any test anywhere names it:
**110 distinct names, 100 of them named by no test.** Most of that 100 is
decoration and needs none — `oil_drip`, `landing_puff`, `sand_burst`. The
harvest is the subset whose NAME asserts an engine behaviour, and that list is
short enough to read by hand.

⚠⚠ **AND EVERY ONE OF THE FOUR IT SURFACED SURVIVED THE CHECK — which is the
result that makes the method trustworthy rather than the result I wanted.**

| suspect | why it looked like a hit | what cleared it |
|---|---|---|
| `stamp_at_rest` / `stamp_moving` (clerk) | a PAIR implies the move branches on the target's motion | they track the CLERK's own motion per move — jab and back-air at rest, forward-air and forward-tilt moving. Coherent relativity flavour, no branch claimed |
| `ether_cancel` (Emmy) | "cancel" is a real engine feature and she authors no `cancelable` | Michelson–Morley, not a cancel window — and the file explicitly considered one and rejected it with a reason |
| `still_life_lock` (automaton) | a LOCK, in the file that already held one inversion | a jab, and a still life is a Life pattern that does not change. No prose claims a lock |
| `fixed_point_acquire` (automaton) | its comment says *"and **holds it there**"* over `knockback_growth: 1.65`, which sends a hurt victim FARTHER | **measured against the roster**: 1.65 growth / 84 kb sits mid-range of fourteen up-airs (1.55–1.90), slightly BELOW median. An ordinary juggling up-air is what "holds it there" describes |

⭐ **The last one is the one worth keeping.** Reading flagged it and the
comparison cleared it — and the comparison took one script. ⇒ **A single move's
numbers mean nothing without the column they sit in**, which is the same rule as
the balance guard that caught the plate at 860 against `steam_lift`'s 800.

⛔ **What the sweep does NOT find: boredom.** `pugnacious_polygon` has ten
comment lines, zero claim markers and five specials that are a haymaker, a
shoulder rush, an uppercut, a ground slam and a body drop. Perfectly honest, and
exactly the *"many have boring specials"* the goal names. ⇒ **Honest and dull is
a different defect from dishonest, and this instrument is blind to it.**

⛔ Bob's `rivet_gun` is the sharpest of the five flagged, because its comment names the precise
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

### ⚠⚠ FIFTEEN ROSTER DECISIONS MADE UNDER DELEGATION — EVERY ONE IS JON'S TO OVERRULE

Jon's grant was *"you can pick where to put the proof of concept for the other
moves in the roster… we can tune who the moves belong to later."* ⇒ Here is
everything spent against it on 2026-09-05, with what each one COST, so a review
is a table rather than a git log. ⚠ **This table was written at EIGHT, corrected at TWELVE and FOURTEEN, and is
now FIFTEEN — updated in the SAME commit as the decision this time, which is the
rule I had already broken twice.**
⇒ I have now had to bring it current twice, which is the point rather than an
apology: **the artefact whose whole purpose is to be current is the one that reads
as complete when it is not**, and its reader is by construction somebody who will
not check the log. The rule I keep failing and re-learning: update it in the same
commit as the decision.

**Eight cost nothing; six displaced something; one is presentation only.**

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
| Bob · neutral (`rivet_gun`) | one long window → **three separated windows, then the same finisher** | ⭐ none — the finisher is unchanged and the pulses are 2 chip. It only does what its comment always claimed: *"it is not one hit, it is the tool running"* |
| Bob · down-B (`bulkhead_drop`) | **+ a plate that throws whoever steps on it** | ⭐ none — the slam is untouched. ⚠ It throws HIM too, which is the row's point (a persistent actuator ANOTHER fighter interacts with) and not an oversight |
| Carl Stargan · side-B (`planetary_orbit`) | straight `impulse(700, 0)` → **a homing dash in a 60° cone** | ⚠ replaces the impulse, not the move — the swing is untouched, and an UNAIMED pass is the same straight 700px/s dash it always was. ⭐ Its own `orbit_lock` effect had been drawing a lock the move did not have |

⛔ **If exactly one of these is wrong, it is the Officer's** — it is the only
change that removes a way to finish a stock rather than replacing filler or
adding on top. It is also the one that makes his kit a single idea (gun, shove,
shield), so it is a real design choice rather than an oversight, and it is stated
here to be argued with.

### ⛔⛔ A POISON THAT DID NOT FIRE WAS THE MOST USEFUL RESULT OF THE ROW

The Parasol landed with two guards — one on the integrator's product, one on the
authored beat reaching the owner's movement policy — and both were green. Then a
poison that **zeroed the modifier's countdown every tick left BOTH of them
green.**

⇒ Neither fixture ran the body step that owns the clock. Between them they proved
a modifier can be SET and READ, and never that it ENDS. **A regime that never
expires is the precise failure a duration exists to prevent, and it was the one
thing untested.**

⭐⭐ **THE RULE, and it is a real sharpening of this morning's:** *a poison that
does not fire is a FINDING, not a badly-aimed poison.* The tempting reading was
"wrong poison for this test, the clock isn't in that fixture" — which is true,
and is exactly the sentence that would have left the gap in place. The right
reading is that **no test anywhere ran that code**, and the poison is how I
learned it. ⇒ Third guard added (`a_gravity_modifier_expires_on_the_movement_clock`),
driving the real `update_body_simulation_in_frame`, and it now reddens in BOTH
directions: a clock that clears instantly (*"every duration an author writes is a
lie"*) and a clock that never decrements (*"a body that floats past its timer
floats forever"*).

⚠ Together with the cancel-road finding this morning, that is **two coverage holes
in one day found by poisons rather than by failures**, both in guards I had just
written and believed. The pair share a shape: *the fixture answered a smaller
question than the assertion claimed*, and nothing in a green run distinguishes
those.

### ⛔⛔⛔ THE BOMB, THE MINE AND THE BOLT DO ZERO DAMAGE TO A BODY — AND MY OWN RULE PREDICTED IT

A GPT review Jon commissioned found it; **I re-derived every mechanical claim
against current source before accepting any of it**, and all of them hold.

| claim | re-derived |
|---|---|
| `HitSide::Neutral` damages nobody | ✔ `hitbox/mod.rs:556` — `melee_source` is `None` for `(HitSide::Neutral, _)`, and the terminal dispatch at 853 is literally `HitSide::Neutral => {}` with the comment *"Neutral never spawns a damaging hitbox"* |
| all three author it | ✔ `bomb.rs:199`, `bolt.rs:285`, `mine.rs:166` — every one is `HitSide::Neutral` |
| `DamageTeam` pins the same rule independently | ✔ `(Self::Neutral, _) => false`; **`Environment`** is the arm that damages Player, Enemy and Neutral |
| a despawned owner makes the blast a ghost | ✔ the owner's position is resolved at the TOP of the loop and `continue`s when it fails — **before any world-anchor check**, so a World-anchored box still needs an owner that exists |
| bomb/mine own the exploding item; bolt owns the victim | ✔ `owner: entity` on both items (a `GroundItem` despawned as it emits) and `owner: body` on the bolt, which is the CONTACTED RIVAL — so making it damaging would trip self-exclusion and skip the very body it hit |

⛔⛔ **AND THE REASON NONE OF MY GUARDS CAUGHT IT IS THE RULE I WROTE THIS
MORNING, APPLIED TO ME.** `mine/tests.rs` asserts `blasts(&mut app).len() == 1` —
**that a `DamageBoxEffect` request EXISTS.** Nothing drives it through
`apply_effects` → `apply_hitbox_damage` → a victim's health. ⇒ *The third cause
for a silent poison: no fixture runs that code at all.* I formulated that
sentence hours before this review landed, about a gravity clock, and it was
already true of three moves I had shipped.

⭐ **The tell I should have taken: I asserted on a MESSAGE, not on an OUTCOME.**
A test that stops at "the request was written" is testing my own authoring, not
the engine's answer to it — and the engine's answer here is *no*.

⚠ **THE FIX IS FOUR QUESTIONS AND JON SAID SO EXPLICITLY — do not relabel
everything `Player`, and do not conflate them:**

1. **geometry anchor** — where the box IS. `World` is right for a blast, and the
   resolver's owner-position `continue` is wrong for a World anchor: it does not
   need one.
2. **attribution** — who is credited. Should be the placing FIGHTER, not the
   object that exploded.
3. **damage relationship** — who may be hurt. A smash item blast hurts BOTH
   fighters, which is `DamageTeam::Environment`'s meaning; `HitSide` has no such
   variant, and that gap is the actual design question.
4. **self-exclusion** — `victim.entity == hitbox.owner`. ⛔ **This COLLIDES with
   (2)**: making the placer the owner for credit makes the placer immune, and
   your own bomb hurting you is the genre's rule. That collision is why the four
   cannot be answered with one field.

#### ✔ AND THEN IT WAS ANSWERED — `HitSide::Environment`, FOUR QUESTIONS, FOUR ANSWERS

⭐⭐ **A peer supplied the half that unlocked it, and it is worth quoting because
it dissolves the collision rather than trading it off:** *"If Environment damage
simply does not consult self-exclusion, the collision dissolves without touching
attribution at all."* ⇒ `hitbox.owner` was answering two questions — **who is
credited** and **who is immune** — and the genre wants them to disagree. Your own
bomb hurts you, and you still placed it. That is one field doing two jobs, which
is the shape this whole campaign has been removing.

| question | answer |
|---|---|
| geometry anchor | `World`, and it no longer needs a living owner (fixed above) |
| attribution | the placing **fighter**. ⛔ The bolt was crediting **the body it hit**, so a kill made a fighter their own attacker; it now resolves the caster by seat |
| damage relationship | **`HitSide::Environment`**, the presentation-side name for `DamageTeam::Environment` — a relationship that was **already settled and guarded** (`placements.rs` asserts both `Environment.can_damage(Player)` and `.can_damage(Enemy)`), used by exploration hazards today |
| self-exclusion | **a hazard consults none.** That is what lets attribution stay honest |

⛔ **IT COULD NOT SIMPLY REUSE `DamageTeam`, and the reason is worth recording:**
the exploration hazard road never passes through `Hitbox`, which carries `owner`,
`source: HitSide` and a bare `damage: i32` and has **no `DamageTeam` at all**.
Two mechanisms, one relationship — so the new variant maps onto the existing
meaning rather than inventing a second, which would have left the repo with two
answers to *"what is a hazard to a fighter"* that must agree forever.

⭐ **The blast radius was three compile errors, not the twenty-eight files the
name-count suggested** — the exhaustive matches named every site that had to
decide. ⭐ **And no schema bump:** `combat.hitbox` is `component-clone`, so a
clone snapshot restores the new variant with no codec.

⛔ **The guard asserts the CONTROL as well as the fix**: a `Neutral` box reaches
`(0, 0)` bodies and an `Environment` box reaches both — **including its owner** —
so the test can tell the broken vocabulary from the fixed one. Poisoned three
ways: hazard consulting self-exclusion (*"the blast spared the body that OWNS
it"*), hazard back on the no-damage path, and the mine re-authoring `Neutral`
(*"it would explode and hurt nobody"*).

⚠ **And the demo-side test now asserts the FACTION, not the request.** That is
the actual repair to my method: asking whether a `DamageBoxEffect` exists is a
question about my own authoring, and the engine's answer to it was *no*.

#### ✔ AND THE ONE UNAMBIGUOUS ENGINE BUG IN FINDING 1 IS FIXED

`apply_hitbox_damage` resolved the owner's position at the top of its loop and
`continue`d on failure **before consulting the anchor** — but `world_volume`
reads `HitboxAnchor::World { center }` and ignores that position entirely, so the
requirement was never real for a world box. ⇒ Every thrown-item blast in the
demo was a ghost: the exploding `GroundItem` despawns as it emits the effect, so
the blast it had just spawned was skipped on the very next tick, every time.

⭐ The box's own centre is the substitute rather than a placeholder, because the
value is still read for the launch direction and `source_pos` — a blast radiates
from the blast, and a zero would have thrown every victim at the world origin.
⛔ Poisoned in BOTH directions, and the second is the one worth having:
"resolve everything" reddens the `FollowOwner` arm, which is the plausible wrong
fix a reader reaches for.

⚠⚠ **AND A SECOND THING FELL OUT THAT NOBODY HAS ASKED YET, WHICH IS PART OF
JON'S QUESTION 1.** Inside that same loop the IMPACT POINT is
`midpoint(victim.center, world_volume.center())` while the KNOCKBACK DIRECTION is
measured from `owner_pos`. **For a `FollowOwner` box those nearly coincide; for a
world box they can be anywhere relative to each other** — so a blast currently
throws victims away from the FIGHTER rather than away from the explosion.

⇒ I tried the principled version (a world box always radiates from its own
centre) and **zero tests changed.** ⛔ By this page's own rule that is a COVERAGE
finding, not a green light: **nothing in the suite asserts the knockback
direction of a world-anchored box whose owner is somewhere else.** ⇒ Reverted
rather than banked — changing a shared resolver's launch direction on the
strength of "no test complained" is the gamble this campaign keeps writing down.
**It belongs in the answer to question 1, with a test that pins it either way.**

#### ✔ FINDING 3 IS FIXED — THE PAIR HAS AN OCCURRENCE IDENTITY NOW

`channel_index` is AUTHORING data — the same `8` for every Alice — and it was
stored as `pair_index` as though it named a live pair. ⇒ Two Alices recovering at
once put two entrances on channel 8 and two exits on 9; `find_portal` returns the
FIRST match, so one could leave through the other's aperture, and the expiry
sweep despawns every move portal carrying the index, so **one Alice's clock
running out shut the other's pair mid-recovery.** A 2.5s lifetime makes that
ordinary, not exotic.

⭐ **The authored index is now a BASE and the SEAT makes it an occurrence** —
each seat gets its own two-channel window above it, so two Alices open on 8/9 and
10/11. ⛔ `MatchSeat`, not an `Entity`, exactly as the review asked: the seat is
rollback-registered (`actor.match_seat`) so both peers derive the same channel
for the same fighter, where bevy_ggrs recreates entities.

⚠ **It saturates rather than wraps**, because a base near the top of the `u8`
channel space with many seats could roll over onto somebody else's window — the
precise collision this exists to prevent. An overflowing seat degrades to the old
shared-channel behaviour rather than to a silent swap.

⛔ **Requiring the seat broke FOUR existing portal fixtures, and that was the
right answer rather than a reason to make it optional**: each spawned a caster
with no `MatchSeat`, which is a body that cannot exist in a match. An `Option`
with a default would have kept them green while hiding that they were asking the
system a question about nobody. ⇒ Poisoned by restoring the static index, which
prints the defect literally: `[Indexed(8), Indexed(9), Indexed(8), Indexed(9)]`.

#### ✔ THE REVIEW'S FOURTH FINDING IS FIXED — BOLT AND SPRING PICKED WINNERS BY QUERY ORDER

Both broke on the first overlapping body. **The spring ignored the seat outright
(`_seat`)** and had one use to give; **the bolt's two answers are different
MOVES** — the Thunder Jacket (a recovery that launches the caster) versus an
offensive hit on a rival. ⇒ Bevy's iteration order chose, which is not stable
across a rollback resimulation: two peers can resimulate one tick and launch
different fighters, or turn one peer's attack into the other peer's recovery.

⭐ **The spring takes the lowest `MatchSeat`** — rollback-registered, so both
peers agree. ⚠ Arbitrary as FAIRNESS (seat 0 wins a tie) and accepted as such:
two bodies inside one plate on one tick is rare, and **a rare unfair outcome both
peers agree on beats a rare desync.** ⭐ **The bolt prefers a RIVAL over its
caster**, which is a design statement rather than a stabilised coin-toss: a bolt
that could connect does the offensive thing, and the jacket is what it does when
it finds nobody else.

⛔ **Guarded in the review's own shape — identical geometry, REVERSED SPAWN
ORDER, same outcome** — because "somebody was launched" passes on the broken
code. Poisoned by restoring first-wins: the spring's guard fails with *"launched
seat 0 in one order and seat 1 in the other"*, reproducing the defect
mechanically rather than by argument.

### ⭐⭐ "MAKE IT REFUSE AND COUNT WHAT FALLS" IS A MEASUREMENT, AND IT SAVED ME FROM THE OBVIOUS FIX

Two registries in the moveset lane were on a peer's silent-overwrite inventory.
Applying the obvious ruling — adopt refusal — would have been wrong on one of
them, and the thing that said so was **running the refusal and reading the names
of the tests that broke.**

| registry | verdict | what decided it |
|---|---|---|
| `PreparedCharacterRegistry` (production door) | **already done** | `register_character` refuses with `CharacterRegistrationError::DuplicateId`, whose own doc argues it: *"a stable id is the thing saves, replays, and the network key on."* The inventory row was stale |
| `PreparedCharacterRegistry` (test-support hatch) | ⛔ **REPLACE, and now it SAYS so** | I sealed it. **Four tests fell**, and their names are the argument: `deleting_an_override_in_a_hot_reload…`, `a_new_cast_generation_refreshes_a_seated_fighters_kit`, `a_character_that_stops_authoring_hurtboxes_has_them_retracted`, `replacing_the_cast_reprojects_a_body_wearing_the_same_character`. ⇒ Every one re-registers ONE id deliberately, because **that is what a hot reload IS** |
| `MovePrefabRegistry` | ✔ **refuse** — landed | Measured first: the only three registrations in the workspace are the engine seeds, under three distinct literal keys. **Nothing overrode anything**, so "(or override)" documented a capability no caller used and a hazard every caller inherited |

⛔⛔ **THE ONE I WOULD HAVE GOT WRONG IS THE MIDDLE ROW, and I got it wrong for
about four minutes.** The two roads into that registry answer DIFFERENTLY and
both are right: at the production door a second write means two PROVIDERS claimed
one stable id and somebody has to lose; at the hatch it means the SAME author
published again, which is a republication. **A rule applied to the registry
rather than to the road is wrong on one of them.**

⭐ **And the count was the signal, not the failure.** Four tests falling said the
road is exercised deliberately and for a reason; one falling would have said only
that a fixture touched it. ⇒ This is the positive form of the poison rule two
sections up — **when you poison a shared authority, the NUMBER that reddens tells
you how many roads your fixtures actually reach.** A peer put it in exactly those
words the same afternoon, from the opposite direction: emptying a shared table
reddened three tests, and the three was the point.

⚠ `MovePrefabRegistry` also does NOT adopt the shared `classify` helper, for a
reason worth stating rather than inheriting: `classify` decides its three answers
with `PartialEq`, and the value there is a `fn` POINTER. The compiler may merge
identical functions or not, so "the same entry" is a question that depends on
optimisation settings. ⇒ **A registry that cannot soundly recognise idempotence
has no honest Idempotent arm**, and any second registration is a conflict.

### ⚠ ROSTER DECISION #16 — PUGNACIOUS POLYGON GETS THE PARASOL, AND HE WAS PICKED BY MEASUREMENT

His up-B was `impulse(0, -745, Set)` and nothing else. It now opens a 0.35-gravity
float for 1.1s at 0.20s — after the hit, so the uppercut stays a committal rising
attack that can be beaten, and the reward for landing or surviving it is the way
home.

⭐ **He is the fighter the comment-vs-mechanics sweep scored LAST**: the only
moveset in the crate with **zero claim markers across ten comment lines**, five
specials that are a haymaker, a shoulder rush, an uppercut, a ground slam and a
body drop. ⇒ The sweep is blind to boredom and said so; this is the first row to
act on what it could not see, by using its own ranking in the other direction.
**Honest-and-dull was measurable all along — I just had to read the column I
already had.**

⚠ **It is a BUFF to a recovery and that is a balance decision, not a neutral
one.** Jon's to overrule like the other fifteen; the move it replaces was dull
rather than balanced, which is the argument for doing it here rather than on a
fighter whose recovery is already load-bearing.

### ⛔ JON'S CALL — WHAT FILLS THE LIMIT METER (the gate is built and free)

`MoveGates::meter_cost` ships defaulted to `0.0`, so nothing in the game costs
meter today and no authored move changed. **The engine half is done and the
design half is one question:** *what earns it?*

⛔ **The premise this row was written on turned out to be false, and correcting it
is what surfaced the question.** The row said the meter was *"already
regenerating — so a first rung needs NO fill system at all."* Measured instead of
read: **every `BodyMana` in the workspace is `ResourceMeter::new(100.0, 0.0, 0.0)`
— regen ZERO**, and its only refill anywhere is the platformer's shrine, which no
smash stage has. ⇒ Price a move today and it works a fixed number of times per
life and then never again.

| option | what it feels like | cost |
|---|---|---|
| **damage dealt AND taken both fill it** (the genre's own answer) | the comeback mechanic: the player who is losing gets there first | a fill rule at the hit seam, plus a number |
| damage TAKEN only | purely a comeback tool; a winning player never sees it | same |
| a slow clock | a rhythm, not a comeback — closer to a cooldown than a Limit | cheapest; also the least interesting |
| leave it at zero fill | a once-per-life resource, which is a real design (one Final Smash) | nothing — it already behaves this way |

⭐ **My recommendation is the first**, because it is the one that makes the meter
worth showing the player, and because "the fighter who is behind gets the tool
first" is the reason the mechanic exists in the genre at all. ⚠ I did not build
it: the fill rule decides the whole feel, it is the half a player actually
experiences, and every option above spends the same field.

### ⛔⛔ THREE VELOCITY WRITES I SHIPPED TODAY BROKE ADR 0024, AND A PEER FOUND THEM

`engine.velocity-writes-are-authority-only` went red on `bolt.rs:268`,
`homing.rs:158` and `spring.rs:165` — all three mine, all three from today.
⇒ **Three new moves, three bare `kin.vel =` writes, and I did not run the
workspace policy suite between writing them and pushing.**

⭐ **The fix is one entry, not three, and the policy asked for that shape in its
own words** — on `intercept.rs`: *"Waived at the OPERATION rather than at its
callers, which is the point of having an operation: the next interception adds no
entry here."* ⇒ `ambition_demo_smash/src/motion.rs` now owns
`command_body_velocity`, all three call it, and the next smash move that launches
somebody adds no policy entry. **Poisoned by restoring the bare write in
`bolt.rs`: red, naming file and line** — so the waiver is scoped to the operation
and did NOT blanket-skip the demo.

⚠ **`AccelerationFrame::launch` was tried first and rejected for a stated reason,
which is recorded in the module rather than lost:** it is the frame-aware
authority, but it is scalar and always throws AWAY FROM THE FEET, which would
have deleted the ANGLED plate that `PlaceSpringParams::launch` exists to author.
⇒ The seam gives up frame-awareness and says so in its own header.

### ⛔⛔ THE GUARD I WROTE COVERED ONE OF THE TWO ROADS, AND THE POISON FOUND IT

The meter gate's first test drove `trigger_moveset_moves` and passed. **Then
deleting the cancel road's copy of the check left all 605 tests in
`ambition_combat` GREEN.**

⇒ That is the exact defect the trigger site's own comment warns about — *"a gate
enforced on only one of two entry paths is a gate somebody will walk around
without meaning to"* — **except on the GUARD side, which is the half nobody
notices**, because a one-road guard and a two-road guard both print `ok`.

⭐ **The general form, and it is cheap to apply:** when the code you are guarding
has N entry paths, poison EACH ONE separately. A single poison on the path you
happened to write the fixture for proves only that the fixture works. The test
now spawns a body mid-move whose window cancels into the priced one, and the
re-run of that same poison fails with the message naming the cheaper road.

### ⛔⛔ A GUARD THAT VALIDATES WHAT SURVIVED A FILTER CANNOT SEE WHAT IT ATE

Four instances in one day, three of them in guards somebody else wrote and one in
a guard I wrote the same afternoon:

| guard | what it validated | what it was blind to |
|---|---|---|
| the citation checker | every cited FILE exists | a drifted LINE number, forever |
| the rollback schema ledger | the file's rows match the runtime | a row I had just added and never compared — until I poisoned it |
| the select-grid test | everything ON the grid is named and seatable | a fighter the filter silently DROPPED |
| my technique census | a key is NAMED by a ruleset crate | that the naming was in a **test**, not production |

⇒ **"Everything present is valid" and "everything that should be present is
present" are different assertions, and the first is the one that is easy to
write.** A non-empty check does not close the gap: a grid missing one fighter is
still non-empty.

⭐ **The fix is to assert the POPULATION**: enumerate what should be there from
the source of truth, subtract the documented exceptions, and require the
remainder to be empty. `every_fighter_this_composition_can_build_reaches_the_grid`
does that — and it matters here because **all ten fighters improved today are
only worth anything if a player can pick them.**

### ⛔⛔ A SPAWN POINT IS INSIDE THE SPAWNER — THREE TIMES, THREE DIFFERENT MOVES

The same defect found by the first guard run on three unrelated objects, each
time before a player could:

| move | what happened on frame one | the fix |
|---|---|---|
| the remote mine | placing and detonating on consecutive presses | `arm_s`, an arming delay |
| the steered bolt | it came home on the tick it was fired and threw the caster instantly | `clear_of_caster`, a latch |
| the placed plate | it launched the engineer who dropped it, spending a use before anyone saw it | `arm_s` again |

⇒ **A move that spawns something AT a fighter spawns it INSIDE that fighter.** The
body-local offset is smaller than the contact tolerance every time, so contact
logic that does not say so is not "usually fine" — it is wrong on frame one, and
frame one runs before anything else can.

⭐⭐ **AND THE GENERALISATION, named by the peer reviewing this work, is bigger
than the frame-one case: TWO THINGS AGREE ON A POSITION AND DISAGREE ON A
TOLERANCE.** The emitter's offset is authored per move by somebody thinking in
sprite pixels; the consumer's radius is written by somebody thinking "about a body
wide". ⇒ Applied to my own new code it found a live defect immediately:

    the bolt   `offset.x > bolt.radius + 16.0 || offset.y > bolt.radius + 24.0`
    the plate  `offset.x > half_extents.x + 14.0 || offset.y > half_extents.y + 26.0`

⛔ **Four invented constants approximating a fighter's extent — on a component
that CARRIES that extent.** `BodyKinematics` has `size: Vec2` and both systems
were already reading it for `pos`. Every fighter but the default-sized one got a
contact box built for somebody else, and nothing would ever have said so: no test
fails, because the fixtures spawn default bodies. ⇒ Both now read
`half_extents + kin.size * 0.5`.

⚠ **The bolt had the same tolerance written TWICE** — once for contact and once
for its "have I cleared my caster" latch — which is the second-order version:
two answers to one question inside one system, so a bolt could clear itself under
one and report a hit under the other.

⭐ **All three were found by a guard, none by reading**, and in each case the fix
turned out to be the genre's own rule arrived at from the bug: a mine you cannot
instantly detonate, a bolt that must leave before it can come home, a plate you
step off before it works.

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
| `MovePrefabRegistry` — the whole `key + params -> MoveSpec` seam (A2 / R2.3) | ⛔⛔ **a THIRD and a worse kind, found 2026-09-05 while triaging the registry inventory: EVERY `.expand()` call in the workspace is in `moveset/tests.rs`.** Not "no authored customer" — **no production caller at all.** It is `pub use`d, `Default`s to three engine prefabs, is fully tested, and nothing in the game has ever asked it for a move |

⇒ **This is a job for O4's installed-technique catalog, and it is a bigger one
than the catalog was scoped for**: the useful census is not "what techniques
exist" but **"which techniques, and which PARAMETER MODES, no authored content
reaches"**. A list of what exists cannot find these; a JOIN against authored
content can. ⚠ All three were found by accident, which is the argument — nobody was
looking, and nothing would have complained.

⭐⭐ **AND THE THIRD SHARPENS WHAT THE CENSUS HAS TO JOIN AGAINST.** The first two
are techniques with no authored customer, which a content-side join finds. The
prefab registry is a whole SEAM with no production caller, which that join cannot
see at all — it is not a technique, it appears in no moveset, and its own tests
exercise it thoroughly enough to look healthy. ⇒ **The two questions are
different: "what does authored content never reach" and "what does the SHIPPED
GAME never call".** A capability can pass the first and fail the second, and this
one does.

⚠ **It is not a defect and I am not deleting it** — built-ahead infrastructure is
a legitimate thing to have, and the seam's argument (*"a content roster names a
prefab + params to mint a move with ZERO new code"*) is one this campaign is
actively making. ⇒ The finding is that **nobody knew**, and that a silent-overwrite
hazard in it was being triaged as though it were reachable.

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
