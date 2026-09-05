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

⭐ **Sanic's spring analogue is a SPEED BUMP** (Jon): he slams down a ridiculous
yellow-and-black speed bump that catapults anyone who touches it. On the ground
it stays as the reusable launcher; in the air he drops it and it falls as an
object/hazard. Same mechanical role, its own joke.

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
  `CaptureAttemptRequested` already takes an authored reach volume. ⓘ Raise
  `MAX_REACH_PX` in `no_authored_grab_reaches_further_than_the_stage_allows` in
  the same commit, which is what that guard's doc already instructs.
- ▢ **B3. Projectile interception as a projectile-domain operation** (proof move
  3). Generalise parry's existing re-own — `ProjectileOwner`,
  `ProjectileAllegiance`, velocity — into reflect / redirect / consume, keeping
  the six ownership axes independent.
- ▢ **B4. Per-action muzzle transform + sustained charge presentation.** The two
  concrete deficiencies the charge-ball source names about itself. Not a new
  mechanic; the authored spatial/presentation contract being too weak. Fixes how
  Projectile Polygon READS without touching how she works.

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
