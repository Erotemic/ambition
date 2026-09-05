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
| 2 | **Counter + Revenge variant** | reactive defensive contact can emit either an immediate attack OR persistent character state, with no Counter subsystem |
| 3 | **Reflector + absorber** | projectile interception is a projectile authority and supports more than parry reversal |
| 4 | **Tether recovery + aerial tether grab** | terrain/body tethering composes with ledge and capture authorities |
| 5 | **Cargo command grab** | capture and movement cooperate without either duplicating pose authority |
| 6 | **Remote mine** | persistent spawned identity, owner/tag lookup, cross-move triggering, attachment, rollback |
| 7 | **Parasol** | temporary movement modifiers/controllers are expressive enough for a long-lived post-move locomotion regime |
| 8 | **Homing Attack** | deterministic semantic target queries and target-directed fighter motion |
| 9 | **Sing** | a real non-hitstun control status with explicit lifetime/wake policy |
| 10 | **Limit** | character-local resource state, threshold transitions, timeout, action variants, stat modifiers |
| 11 | **Free-standing ally summon** | summon is not synonymous with mount; first owned-secondary-actor contract |
| 12 | **Reusable launch object** | a fighter can create a persistent world actuator another fighter interacts with |
| 13 | **Portal recovery** (Jon, added same day) | an authored customer can place linked world portals and traverse them, exercising `ambition_portal2d` from a move — including an angled pair, which genre parity does not have |

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
| **Alice** (provisional) | **up-B: a portal recovery** | NEW, below |

⚠ **PLACEMENT IS PROVISIONAL AND SAID SO** — Jon: *"We can tune who the moves
belong to later. The important thing is that they can be expressed and authored
easily and creatively."* ⇒ A rung is finished when the move is EASY TO AUTHOR,
not when it is on the right fighter. Do not spend a decision on the roster slot;
do spend it on whether a second fighter could take the same move by authoring
alone.

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
  ▢ **Still open: the line presentation**, the fourth customer of `flyline.rs`'s
  procedural-visual shape. A tether that reaches 150px and draws nothing is the
  mechanic without the read.
- ◐ **B3. Projectile interception as a projectile-domain operation** (proof move
  3). ✔ **THE OPERATION LANDED 2026-09-05** (`projectile/intercept.rs`):
  `ProjectileInterception::{Reflect, Consume}` and `intercept_projectile`, with
  the parry as its first caller. It touches exactly three of the six axes —
  combat owner, allegiance, trajectory — and emits NO cues, because a parry
  clangs where a reflector hums and an absorber swallows. Returns whether the
  shot SURVIVED, which is the distinction an absorber needs on its first line.
  ⛔ `Redirect` deliberately absent: no customer, no test, and a third variant
  nothing calls is the speculative peer authority this campaign exists to avoid.
  ▢ **What remains is the AUTHORED half**: a reflector move and an absorber move,
  each an active volume that names an interception. `Consume` is tested but has
  no authored customer until the absorber exists.
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

### Track A — the keystone

- ▢ **A1. `TechniqueFlow` minimum**: `emit` / `wait` / `branch` / `finish`,
  slots, and the `MovePlayback` extension (current node, latches, issued-event
  bookkeeping, timeouts, symbolic slots). ⛔ No variables, arithmetic,
  expressions, ECS queries, or blackboard.
- ▢ **A2. Input lease** — routing selected input channels to another controlled
  entity, and restoring them.
- ▢ **A3. Steerable projectile control source** + authored self-contact
  eligibility.
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
| Remote mine | persistent tagged occurrence + later move queries the owned tag and requests detonation |
| Revenge-style counter | defensive contact interception → persistent character resource/modifier |
| Cargo carry | capture relation with captor locomotion enabled + movement/capture constraint contract |
| Pocket | projectile interception → immutable stored payload → later projectile respawn |
| Corrin-like pin | directed movement/hit → terrain query → spatial attachment → input branch |
| Sonic-Blade chaining | flow waits for repeat direction/input, reacquires target, runs another scoped motion segment |
| Stone / Withdraw | body/form mode + modified collision/movement + explicit exit conditions |
| Shadow-Flare delayed mark | persistent gameplay occurrence attached to victim + timer/remote trigger |
| Pikmin-like latch | owned secondary entity + body attachment + periodic effect |
| Wind / vacuum | existing force/wind semantics, generalized to a sustained field only for a persistent customer |
