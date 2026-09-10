# Awaiting a maintainer decision

This file contains only **currently unresolved maintainer/product decisions**.
Engineering work that can proceed without a ruling belongs in
[`queue.md`](queue.md). Answered, withdrawn and superseded questions are deleted
from this live ledger; Git history preserves the discussion.

Use one unique `Q<number>` per live question. Do not reuse a retired number.
When a question is answered, record the durable ruling in
[`maintainer-decisions.md`](maintainer-decisions.md), update the owning plan/source,
and delete the question here.

## Gameplay and content

## Q33 — how should a recharging ranged weapon communicate that it is unavailable?

Choose the player-facing unavailable/readiness signal. The mechanism should not
invent one presentation independently for every ranged weapon.

## Q36 — what are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

The engine has a canonical-height contract; these remaining authored characters
need product values rather than inferred sprite dimensions.

## Q37 — should the F9 rollback proof pulse survive a gameplay-session change?

Decide whether the pulse demonstrates one rollback session or is a process-level
debug affordance. Its resource lifetime should follow that answer.

## Q38 — does an actor released in a foreign room stay there?

This is a world/product rule about custody and room membership after release, not
a query-order choice.

## Q40 — should a held gun-sword kick the player the way it kicks the pirate?

Choose whether recoil is an authored weapon property applied to every holder or a
pirate-specific behavior.

## Q41 — where should a hand-fired fireball leave the body?

Choose the authored launch landmark/offset contract for hand-fired projectiles.
Do not derive it independently from sprite bounds at runtime.

## Q42 — should the gauntlet fireball keep bespoke art or use the catalog energy ball?

Product/art choice. The runtime should consume one authored presentation identity
once chosen.

## Q43 — does a body hanging on a ledge inside a hazard volume die?

Choose the game rule for ledge custody versus hazard damage. The engine can then
encode one authority rather than special-case the observed overlap.

## Q44 — should `SmashChargeSpec` keep a Smash-specific name?

The mechanism is now broader than one game mode. Rename only if the intended API
is reusable; do not churn names solely for aesthetics.

## Q45 — is a unique capability item an entitlement or an occurrence?

Choose whether losing/dropping the physical object also loses the capability.
Keep occurrence, custody and entitlement separate in either answer.

## Q46 — does Mary-O 1-1 want a fourth question block over floor?

Content-layout choice needed to make the floor-refusal behavior of the fire form
meaningfully playable.

## Q47 — where does TwinTrack's simultaneity limit live while the exhibit is parked?

Choose whether the parked exhibit still consumes a global/participant/world
simultaneity slot.

## Q49 — is near-identical CPU play on a symmetric stage acceptable?

If yes, no diversity mechanism is owed. If no, specify whether the desired
variation is tactical policy, difficulty behavior or presentation/personality.

## Q51 — does a boss reward survive a death/checkpoint rewind that un-fights the boss?

Choose the reward's durability boundary. Boss defeat and reward occurrence are
already separate facts; this decides which checkpoint/replay retracts the latter.

## Q52 — does a second bark set for one enemy replace the first or conflict?

Choose whether bark sets are single-valued authoring or composable collections.
The content validator should enforce the chosen cardinality.

## Q54 — in co-op, does a body gate open for the party, the acting body, or only the primary body?

Choose gate subject semantics. Do not hard-code primary-player behavior into the
generic gate vocabulary.

## Q55 — should authored worlds grow to use all five route-gate families?

The vocabulary exists. Decide whether broader authored coverage is desired now or
the unused families should remain capability surface without demo customers.

## Q56 — should room replay retract boss defeat for every boss family or only the currently wired one?

Choose the intended replay durability rule, then make the generic boss-progress
road enforce it uniformly.

## Q58 — does the BODY gate family ask what a body *can do* or what it *is doing*?

Capability and current action are different facts. Pick the authored gate
semantics before extending content usage.

## Q64 — what happens to a held gun when its holder is clipped by a far-side portal?

Choose whether the item gets its own clipped body-owned presentation, disappears
with the holder, or follows another explicit rule. Do not let draw order decide.

## Q67 — does the Limit meter survive stock loss?

Choose stock lifetime for the meter. The earlier free-Limit respawn bug is fixed
independently of this decision.

## Q70 — should the title Settings tab visibly highlight on pointer hover?

Small UI/product choice; implementation already has the semantic tab state.

## Q71 — how much Limit should a successful block award?

Generic meter policy now permits a block source. Choose the Smash balance value;
keep it out of generic validation.

## Q79 — how far may the camera zoom out before the fight stops being legible?

Choose the product legibility floor. Camera policy can then clamp against a named
limit rather than an arbitrary tuning value.

## Q80 — what does “the art agrees with the hitbox” mean in pixels?

Choose the tolerance/measurement contract used by sprite/hitbox authoring tools.
This should be a measurable visual-authoring rule, not a subjective test failure.

## Q87 — should the top platform and respawn point continue to overlap?

Stage-layout/product decision. If not, move one authored placement rather than
adding runtime avoidance.

## Q89 — what special should each Robot stand-in have?

The stand-ins currently lack the button vocabulary expected by their match role.
Choose authored moves or explicitly accept the omission.

## Q91 — keep the 10× countdown mode?

This was explicitly requested earlier and remains available. Decide whether it is
still a product/debug affordance worth carrying.

## Q93 — should the demo author a dense melee room?

Product/content call. Do not add engine behavior merely to manufacture a stress
scene unless the room itself is wanted.

## Architecture and engine policy

## Q34 — should external/launch-owned motion become an explicit cross-game fact?

If more than one game needs it, give it a reusable authority. Otherwise leave the
current local mechanism local.

## Q35 — what owns fighter reach during move startup?

Choose the semantic owner of startup reach so AI, collision and presentation do
not each infer it from different move state.

## Q48 — should the boss subsystem become a separately composed crate now?

The reassessment requested earlier is due. Judge against current dependencies and
customers, not the historical monolith shape.

## Q59 — two validation ledgers can be red when no lane ran: hook the lane or accept the state?

Choose whether “not run” is a first-class incomplete receipt or whether those
ledgers should be removed from the required surface.

## Q61 — where should ordering live when two systems write the same durable switch?

Choose the intended winner/merge policy where product meaning is ambiguous.
Engineering already requires one accepted mutation authority and explicit phase
visibility; a public set by itself cannot decide between competing writes. See
[composition](engine/capability-and-runtime-composition.md).

## Q62 — keep or discard the epoch-captured 4,741-line `mary_o.ldtk` delta?

This is the explicit history/content decision. Do not modify the retained LDtk
files until the ruling is made.

## Q63 — five authored fields still have no runtime consumer: wire them or delete them?

For each field, choose intended capability versus obsolete authoring. Do not keep
permanent knobs that decide nothing. The review traced three examples end-to-end:
`requires_facing`, pickup `collected`, and chest `persistent`; see finding F4 in
[the source findings](engine/architecture-review-findings.md). The five-field
label is the earlier inventory, not a new claim that all five were revalidated.
Until policy is chosen, unsupported nondefault values should receive diagnostics,
not an invented runtime meaning.

## Q66 — should the citation checker become a ratcheted gate now that its baseline is zero?

Choose whether citation health is required on every relevant change or remains an
advisory audit.

## Q68 — which settings are the evergreen baseline every game inherits?

Choose the cross-game groups/surface. The full settings menu and shell-level audio
controls already exist; this decides composition, not implementation feasibility.

## Q72 — `EncounterEffect::SetMusic` has no production customer: give it semantic identity or remove/defer it?

If retained, multiple simultaneous scripts must not share one subsystem-wide
claimant. If no authored use is planned, remove/defer the unused capability.

## Q73 — may a capability plugin install private systems into a published set under the `ambition_combat` no-plugin stance?

Clarify whether the stance forbids all plugins or only host-owned opaque
installation. C2 needs one durable interpretation. Owner-local installation
helpers and explicit host ordering remain available without prejudging this
plugin-style decision; no registry or broad context is required.

## Q74 — keep or cut the three declared dependency seams that still have no customer?

A seam with a plausible near-term composition use may stay; otherwise remove the
dependency rather than preserving hypothetical architecture. Recheck actual
production call sites and supported profile closure separately. The A9 render
dependency finding concerns a mandatory reachable path, not this older unused-seam
inventory; one is not evidence for the other.

## Q75 — can inventory, dialogue and map coexist in the same frame?

This is the factual reachability question behind the unordered `MenuControlFrame`
reader pairs. If coexistence is supported, input ownership/ordering needs a real
fix; if forbidden, encode the exclusivity as an invariant/test.

## Q76 — are composite mount-riders actually planned?

`MountedBrainCache` has no production constructor at the reference head. Keep the
capability if future authored composite riders are intended; otherwise simplify
rather than maintaining an unused semantic branch.

## Q77 — which target-reclaim rule wins: the sanctioned cleanup-script exception or the repository's “never delete target” rule?

Resolve the contradictory operational guidance before another cleanup tool acts
on the target directory.

## Q78 — how should the divergent/unpushed sprite-renderer submodule state be reconciled?

Before any blind `git submodule update`, decide which line/commit must be kept and
pushed. Tooling warnings should cite this question until the submodule state is
settled.

## Q86 — should cast framing be bidirectional (target) rather than only a floor/minimum?

Choose whether framing owns both minimum and desired composition or only prevents
excessive zoom-in.

## Q94 — what residency-memory limit should the runtime target?

Needs a maintainer/hardware/product value. The residency mechanism can enforce a
budget once the budget exists. Report source, decoded CPU, prepared simulation
content and device residency separately. A8 instance isolation and A9 dependency
closure do not supply a hardware budget.

## Assets, presentation and content policy

## Q69 — at `potato`, should character sprites fall back to the `0_25x` tier?

The tier-drift defect is measured separately. This asks only for the interim
quality policy while the generator/trim problem is repaired. The proposed
`dev/patches/swing-fighter-render-honours-quality-scale-20260902.patch` is a
separate renderer refusal/validation aid, not an answer to this product choice.

## Q81 — what should happen to the mostly-unreferenced bespoke FX rows for Pirate Admiral and George Booul?

The existing FX-row census found these sheets as the extreme unreferenced-art
case (historically summarized as 34 of 35 rows unnamed by production callers).
Either wire effects that correspond to intended authored moves, deliberately keep
rows as future art, or remove superseded rows. The count is owned by
`scripts/measure_fx_row_reachability.py`; do not copy a stale number into code.

## Q82 — should LDtk editor-preview tilesets remain runtime-packaged when the runtime never draws them?

Confirm whether another tool/runtime consumer needs them before excluding them
from runtime residency/packaging. A concrete proposed retarget is preserved as
`dev/patches/ldtk-player-tileset-retarget-20260902.patch`; it changes the map-assets
submodule and therefore needs an explicit content/pointer decision.

## Q83 — should the 442 MB shared sprite pack remain when one prop is the only current reader?

Choose whether this is intentional shared infrastructure or a packaging mistake
that should be split/deferred.

## Q84 — should portraits have independently authored readable low-resolution tiers?

Current generated tiers preserve existence but not necessarily readability.
Choose the product quality requirement before adding portrait-specific generation.
`dev/patches/portrait-tiers-are-never-baked-20260902.patch` is the existing proposed
implementation for the "full-resolution only" answer.

## Q85 — should Hall characters without authored interaction dialogue remain non-interactive?

Content decision: author dialogue or explicitly accept that those cast members are
visual/background only.

## Q88 — who owns the Smash CPU difficulty ladder?

Choose whether ladder tuning is Smash ruleset content, generic fighter-brain
policy, or a combination with an explicit boundary. The current engine should not
infer this from table location.

## Q90 — `read_weight` is authored on the ladder and inert: wire it or delete it?

Do not retain an authored difficulty field that never affects a decision.

## Q92 — is the BODY-PROFILE developer experiment still wanted?

If no, delete the experiment and its planning residue. If yes, name the
measurement it must produce before more implementation work.

## Q95 — fast-forward the music renderer's main and repin the superproject so fresh clones refuse General-MIDI fallback?

The refusal exists on the renderer line prepared for this purpose, but the
superproject pin at the reference state does not contain it. The safe operation
is a durable fast-forward/push of the renderer's main followed by the parent
pointer bump; do **not** repoint the parent at a deletable agent-only commit. In
the same change, flip `scripts/check_pinned_music_renderer_refuses_gm.py` from
reporting to gating.

## Q97 — a character authors a technique this composition did not install: refuse, or degrade?

**Measured 2026-09-09, by turning A11's admission pass on at the preparation
barrier and reading what it refused.** Wired as a refusal first, it reddened
seven application tests, and EVERY refusal was a `smash.*` key in a composition
that does not install `ambition_demo_smash`:

```
mary_o / mary_o_grab   / WindowSustain{1} : smash.capture_attempt
mary_o / mary_o_pummel / Event{0}         : smash.capture_pummel
mary_o / mary_o_fthrow / Event{0}         : smash.capture_throw   (+ b/u/d throws)
```

⛔ **THE MOVES ALREADY PLAY AND DO NOTHING THERE — the pass found that, it did
not cause it.** `mary_o`'s grab, pummel and four throws name Smash's capture
techniques. The capture REQUESTS are engine types in `ambition_combat`
(`CaptureAttemptRequested` and friends); only the authored-effect TRANSLATION
lives in `ambition_demo_smash::capture`. So any composition that wants grabs
without composing the Smash demo gets a silent no-op on six moves.

⚠ **AND THE TABLE CANNOT TELL THE TWO CASES APART.** `TechniqueSupport` knows
only what THIS composition declared, so a genuine typo (`smash.teleprot`) and a
technique some OTHER composition installs are both `Unknown` to it. Refusing on
`Unknown` therefore cannot be narrowed to typos without a workspace-wide key
registry, which does not exist.

⇒ Three answers, and they are not equivalent:

1. **Degrade loudly (what is implemented today).** The whole refusal list is
   logged at `error!` at STARTUP instead of one `warn!` per move mid-fight,
   which is the failure A11 exists to remove. The shipped composition is
   separately ASSERTED to have zero refusals
   (`authored_effects_are_admitted.rs`), so the guarantee is enforced where it
   can be. Costs: an unsupported move still silently does nothing at runtime.
2. **Refuse the definition.** Meets the acceptance row as written ("invalid or
   uninstalled calls cannot publish definitions") and breaks every composition
   that authors a technique it does not install — today that is `mary_o`
   outside the Smash host.
3. ~~**Fix the layering instead: make capture an ENGINE technique.**~~ **DONE
   2026-09-10 — and decided by MEASUREMENT, not by where the request types sat.**
   The review was right to refuse that inference, so ownership was taken from the
   mechanic's state and behaviour:

   * STATE — `CapturedBy`, the component holding who has whom: `ambition_combat`.
   * BEHAVIOUR — acquire, escape sampling, throw edge, hold ticking, pose,
     carries, captor control restriction, release, interruption, pummels:
     TWELVE systems, all `ambition_combat`.
   * VOCABULARY — the four keys and their param structs:
     `ambition_characters::smash_capture`, also engine-side.
   * TRANSLATION — ONE function, in the game.

   ⚠ The old module argued FOR the split — *"a ruleset knows what its own
   authored strings mean"* — and the argument does not survive its own premises:
   the strings are engine constants, and every arm is a field-for-field
   `hydrate -> write` with no ruleset policy in it. The translation moved to
   `ambition_combat::capture::systems::translate_authored_capture_effects` and is
   installed and declared by the engine composition, so `mary_o`'s six grab and
   throw moves now work wherever combat is composed rather than wherever Smash
   happens to be mounted.

⇒ **WHAT REMAINS OF Q97 IS THE POLICY HALF ONLY.** With capture fixed, no shipped
composition is known to author a technique its host did not install — so the
question is no longer "how do we repair these compositions" but the general one:
**may authored content name a capability its host did not compose, and if it
does, should the definition be refused or degraded?** Today it is REFUSED
per-definition (`ambition_characters::prepared::admit_and_finalize_cast`), which
is the literal reading of the acceptance row. Refusing the whole cast, or
degrading with a report, are the alternatives. That is a content-architecture
ruling and it is still Jon's.

⚠ Not a feel ruling: it decides whether authored content may name a capability
its host did not compose, which is the same question the SDK's minimum-profile
work (A9) asks from the other side. Owner document:
[authored technique admission](engine/authored-technique-admission.md).

## Q98 — may the Smash grid seat a character that authors no body?

**Measured 2026-09-10.** `npc_carl_stargan` is seatable on the assembled Smash
grid and, put on a stage against a copy of himself at the top difficulty rung for
a full minute, **deals 0.00 damage — three move starts, zero hitstun, zero
knockouts, in reach for 17 of 3613 ticks.**

⛔⛔⛔ **WITHDRAWN 2026-09-10 — THE PREMISE IS FALSE AND THE QUESTION DISSOLVES.
`npc_carl_stargan` IS NOT A BARE REGISTRATION.** He authors a **locomotion**, a
**600-line moveset of his own** (`carl_stargan_moveset`, `pale_blue_dot` and
all) and **`max_health = Some(4)`**, so `authors_a_body` is TRUE and
`KNOWN_BARE_REGISTRATIONS` had not been reached for him in a long time. **The
exemption was stale and its text — which carries PLACEMENT EVIDENCE, so a reader
takes it as a statement about what he authors — is what this question quoted.**

⚠ **AND THE ASSERTION MESSAGE IT CAME FROM WAS A SPECIFICATION THE PREDICATE
DOES NOT IMPLEMENT.** It read *"authors NOTHING — not a body, not a policy, NOT
A MOVESET"* while the predicate is `authors_a_body || authors_only_policy ||
exempt` and consults no moveset at all. **The third clause is the one lifted into
this question as evidence.** ⇒ A failure message is read only by people who are
already confused, which is when a false claim in it does the most damage.

✔ **Fixed at `30c15da29`:** the list is empty, the message says what it checks,
and a new arm asserts every entry is LOAD-BEARING so an exemption cannot outlive
its need again — poison-verified by putting him back, which names him.

⇒ **NOTHING IS ASKED OF THE MAINTAINER HERE. Delete this question.** The grid
seats a character who authors a body, a locomotion and a moveset; that is a
fighter, and the row it came from was measuring a rung, not a roster.

⚠ **Kept until Jon reads it because the question was ASKED, and a question that
withdraws itself is worth more than one that quietly disappears** — a maintainer
who saw it in passing should be able to find out it was wrong.

⚠ **The measurement that started this stands and is recorded in
[D-CPU-INERT](queue.md): he fights at lower rungs.** The 0.00 above is a **rung-9** reading. Swept across every
published rung, the same character against a copy of himself:

| rung | starts | damage |
|---|---|---|
| 1 | — | separates |
| 3 | 20 / 20 | **55 / 62** |
| 5 | 7 / 7 | 0 / 0 |
| 6 | 34 / 40 | **44 / 66** |
| 9 | 3 / 3 | 0 / 0 |

⇒ **A character the catalog says authors NOTHING — not a body, not a policy, not
a moveset — starts 34 moves and deals 66 damage at rung 6.** So "he is not a
fighter" was true of the rung it was measured at and false of the roster. **Do
not rule on the 0.00.**

⚠ **AND THAT MAKES A SECOND QUESTION THE FIRST ONE HID: what is he fighting
WITH?** Something furnishes a body, a policy and a moveset to an id whose only
registration is the exemption list for ids that have none. **Until that is
answered, options 2 and 3 below are not costed** — "drop bodiless characters"
does not describe him if he is not bodiless, and "author him a body" may be
duplicating one he already receives from somewhere. ⇒ **This is a measurement
somebody owes before the decision, not part of the decision.**

⭐ It is the same species as the defect fixed at `f77ba3a45`, where a duel seat
silently ran a different AI backend from the one the match assigned: **the
composed thing is not what the declaration says**, and a census over declarations
cannot see it.

⇒ **He is not a broken fighter; he is not a fighter** *at rung 9*.
`character_catalog.rs`
lists him in exactly one place — `KNOWN_BARE_REGISTRATIONS`, the exemption for
ids that author *"NOTHING — not a body, not a policy, not a moveset"* — and the
entry records the reason verbatim: *"one placement: hall_of_characters NpcSpawn,
brain_override stand_still. Never an EnemySpawn, so no archetype vitals exist to
retract. **Registered because Jon put him on the Smash grid (2026-08-11) and the
grid drops what it cannot seat.**"*

⚠ So this is a decision that was already made once, deliberately, by the
maintainer — which is exactly why it is a question here rather than a defect in
`queue.md`. He is not on `PLAYABLE_ROSTER` (13 ids, six of them `npc_` fighters);
he reaches the grid by a different road.

⇒ Three answers, and they are not equivalent:

1. **Keep him seatable.** A player can pick a character who cannot fight. That is
   a real product statement if the grid is meant to be "everyone we can draw",
   and it costs nothing to leave.
2. **Drop bodiless characters from the grid.** The grid's rule becomes "a
   fighter", not "an id we can seat". ⛔ This RETRACTS a placement Jon made on
   purpose, so it needs saying out loud rather than being fixed quietly.
3. **Author him a body.** He becomes a fighter and the question dissolves — the
   most work and the only answer that makes the grid entry mean what a player
   would assume.

⚠ **AND THE ENGINEERING HALF DOES NOT WAIT ON THIS RULING**, so it is not
blocking: the duel gate's own assertion checks that an id can be SEATED, not that
it is an authored fighter, and that is what let a bodiless character into a
fighter measurement. Tightening the SWEEP's population is queue work
(D-CPU-INERT) whichever way this is answered.

⭐ Not a feel ruling: it decides whether "on the grid" means "is a fighter", which
is the property every CPU-quality measurement over that grid will assume.

## Q96 — should a projectile collide with an ECS breakable's published surface?

**Measured 2026-09-09, while closing A2b.** A breakable authored
`BreakableCollision::Solid` publishes a `BlockKind::BlinkWall { Hard }` into
`FeatureEcsWorldOverlay::blocks` at its own AABB (`world/overlay.rs`), and the
player collides with it. A PROJECTILE does not:
`ambition_projectiles::collision_world::ProjectileCollisionWorld::solids()`
composites only `gate_solids`, `portal_carves` and `removed_block_names`, so
`overlay.blocks` — every ECS breakable surface and every pogo orb — is absent
from the world a shot sweeps.

Nothing is broken today: the shot reaches the crate through the FEATURE road
instead, which is swept and now ordered against the world by time of impact. The
question is which of two models is intended, because they differ once a
destructible has both a surface and a hurt volume:

- **A shot sees only the hurt volume** (today). A solid crate stops the player
  and is destroyed by shots; the two facts never interact. Simple, and the
  compound-contact row of the contact protocol's acceptance matrix stays
  unreachable.
- **A shot sees the surface too.** Then a destructible's own wall and its
  damageable volume are ONE contact and need stable collider-contributor
  identity to be told apart from an unrelated blocker — the A5 work the protocol
  already describes. It also changes behaviour: a `Bouncing` shot would bounce
  off a solid crate rather than damage it.

⚠ Not a feel ruling — it decides whether A5's contributor identity is required
for projectiles or only for the player road. Owner document:
[projectile contact protocol](engine/projectile-contact-protocol.md).

⛔⛔ **THE SECOND BULLET CONTRADICTS THE OWNER DOCUMENT, AND THAT IS THE PART
NEEDING A RULING.** Raised in the 2026-09-09 GPT review. "A shot sees the
surface too" is framed above as *a `Bouncing` shot would bounce off a solid
crate rather than damage it*. The protocol says the opposite for that exact
case: a destructible's own surface and its hurt region COALESCE INTO ONE
COMPOUND CONTACT that resolves the target once **and also** honors the surface's
physical response — the whole point being that a generic "wall first" rule must
not make every solid destructible immune to projectiles. So the two models on
offer are not "today" versus "the protocol"; the second bullet is a THIRD model
that no document specifies. Whichever way this is decided, the protocol section
and this row have to end up saying the same thing.

⭐ **MEASURED 2026-09-09: the compound row is not merely unreached, it is
STRUCTURALLY UNREACHABLE, and that is why A2 did not have to wait.** A shot
sweeps `ProjectileCollisionWorld::solids()` — the authored room, plus gate
solids, minus portal carves and named removals. `overlay.blocks`, which is every
ECS breakable surface, never enters it; the module's own contract says a
projectile "passes through breakable/ECS overlay solids". So every block the
projectile sweep can return is by construction an INDEPENDENT blocker, and the
protocol's tie rule (an independent surface at equal time beats an unrelated
hurt target) applies with no contributor identity at all. That is what
`dc2fe7ce7` implemented. Contributor identity becomes REQUIRED for projectiles
the moment this row is decided the second way — not before.

### ⭐ ANSWERED 2026-09-10 — AND STILL UNDERDETERMINED. DO NOT CLOSE THIS ROW.

**Jon, verbatim:** *"Yes, they collide. There might be instances that we mark that
certain projectiles do not collide with certain types of collision surfaces. So do
not exclude that possibility."*

⇒ **The row moves from UNANSWERED to ANSWERED-BUT-UNDERDETERMINED**, which is a
different state and is named as one because the difference is what stops someone
closing it.

⛔⛔ **THE WORDS DO NOT SELECT BETWEEN THE TWO MODELS ABOVE.** "A shot sees the
surface too" is what BOTH the second bullet and the owner document say; they differ
in what happens NEXT, and the ruling does not reach that. ⇒ **The ⛔⛤ above still
stands: the second bullet is a third model no document specifies, and the ruling
must not be read as selecting it.**

⚠ **AND A PARAPHRASE ALREADY SELECTED ONCE.** The ruling was first relayed as
*"a solid breakable stops a bolt."* **"Stops" is not in what Jon said** — it is the
second bullet's behaviour, and the relay picked it because the bullet is the
sentence sitting under this heading. ⇒ **The nearest available sentence beat the
owner document.** The relayer caught and withdrew it. Recorded because a reader who
sees only the paraphrase cannot tell an affirmation from a selection.

⭐ **THE EXEMPTION CLAUSE IS BINDING ON WHICHEVER MODEL WINS.** *"Do not exclude
that possibility"* forbids an unconditional rule. A per-projectile / per-surface
exemption must be EXPRESSIBLE, and that obligation lands on the design before the
first implementation, not after it.

#### A recommendation, marked as a recommendation

⚠ **THIS IS YardratAmbition's, RELAYED, AND NOT CONFIRMED BY JON.** It is recorded
so the reasoning is visible, not so it can be built on. Jon gave a principle for
other open questions — *"what is the most elegant solution — the one that pushes us
towards single authority and compositionality?"* — and applied to Q96 it selects the
owner document's **compound contact**, not the bullet:

* the bullet makes surface response and hurt region COMPETE, so it needs a
  precedence rule: **two authorities plus an arbitration layer**;
* the compound contact is ONE event that resolves the target once and honours the
  physical response: **one authority**.

⛔ **AND ONE HALF OF THAT ARGUMENT DOES NOT HOLD, CHECKED HERE RATHER THAN
FORWARDED.** The recommendation adds that an exemption has *"nowhere to go"* under
the bullet. **It has somewhere.** Under the bullet, exempting a `(projectile,
surface)` pair means the shot ignores that surface and reaches the hurt volume by
the feature road — which is **exactly today's behaviour for that pair**, so it is
both expressible and already implemented.

⇒ **What actually differs is WHERE the exemption lands**, and that is a real
distinction worth the ruling:
* **compound:** the exemption masks the SURFACE half while the hurt half resolves
  in the same contact — the shot passes through physically and still damages;
* **bullet:** the exemption removes the surface from the sweep, and the damage
  arrives on a separate road.

⇒ So the exemption clause is **evidence about the shape**, not a disqualifier. The
single-authority argument stands on its own; the "nowhere to put it" argument does
not, and is struck rather than repeated. ⚠ **A peer's reasoning gets the same check
as a peer's relay** — that is the whole lesson of the paraphrase above.

## Human measurements, not design answers

These are recorded here only when the maintainer must supply the measurement; the
engineering follow-up belongs in `queue.md`.

- **D-RASTER-3:** weak-GPU framebuffer/source-tier comparison.
- **Switch Pro outer range:** radial maxima/dead-zone measurement on both machines.

## Q99 — D-BRAIN-MENU is held by two red tests and neither is now evidence about the fighter brain: drop the hold?

**The change.** `truthful_attack_kit` makes the CPU's attack kit resolve a press
the way the PRESS ROAD does (`move_for_attack(.., running)`) instead of its
stance-blind fallback. Today the brain scores `jab`'s frame data while the body
performs `{base}_dash`: **eighteen of eighteen shipped fighters author a dash
attack no press in the kit reaches**, and every scoring term downstream — startup,
reach, damage, frame advantage — reads a different move than the one the press
produces. See [D-BRAIN-MENU](queue.md) for the full receipt.

**It is held because two acceptance tests redden. Both were re-measured
2026-09-10 and neither now says anything about the fighter brain.**

⛔ **RED 1 is a threshold whose calibration subject moved underneath it.** The
gate is 0.50 damage per pool-minute; the fix reads 0.47 on `npc_pirate_admiral`.
⇒ But that fighter's own HEAD baseline moved **1.26 → 0.84** when `f77ba3a45`
stopped a dismount replacing his opponent's brain — **his row was a fighter
beating a brute** — and a second shipped fighter, `medic`, **fails the same gate
at 0.41 with nothing changed at all.** On `medic`, the clean subject with no
mount and no confound, the fix is an IMPROVEMENT: 0.41 → **0.45** damage, hitstun
118 → **132**.

⛔ **RED 2 is a fixture whose own strike does not land.** Its message reads *"the
percent meter is not reaching the launch"*. Measured across the strike:

| up-tilt fixture | feature OFF | feature ON |
|---|---|---|
| percent = 0 | meter **0 → 10**, rose 3.4 px | meter **0 → 0**, rose 26.6 px |
| percent = 1427 | meter **1427 → 1437**, rose 372.8 px | meter **1427 → 1427**, rose 26.6 px |

The shipped arm scales 110×, so the meter is fine; under the feature the victim's
meter does not move by a point. **Every knockback input in that fixture is a
literal in its own file, so the launch cannot stop scaling except by the victim
not taking the hit.** ⚠ That fixture shares its app with live CPU fighters whose
walking it does not control — **it is a gate on "the CPUs still walk the way they
did in August", and this change is a change to how CPUs walk.** It now asserts its
own strike landed, so it cannot make this accusation a fourth time.

⇒ **THE QUESTION IS NOT WHETHER THE EVIDENCE HOLDS — IT DOES NOT. It is whether
a hold is dropped on that basis by an agent.** Two answers:

1. **Drop the hold and land the fix**, re-deriving the 0.50 gate against the
   roster it is meant to police — it does not hold across fighters at HEAD, which
   is a defect in the gate whatever happens to this row.
2. **Keep the hold** and require a positive result — a measurement showing the
   truthful kit makes CPUs fight BETTER on a stated population — before landing.
   ⚠ The three subjects measured at rung 9 say improvement, no-op, and mixed; the
   row's owner document says re-pricing matchups *"needs the ladder rig, not a
   coordinator's judgement"*, and the ladder rig cannot seat Ambition's authored
   movesets at all.

⚠ **Recorded as a question rather than acted on because dropping a hold is a
maintainer's call even when the evidence for it has evaporated.** Nothing is
blocked meanwhile: the fix is behind a default-off feature and the shipped
behaviour is today's by construction.

## Q100 — should the facade pull `bevy/debug` because it always links `ambition_dev_tools`?

**Measured 2026-09-10 (`86c95490d`), and the remedy is a ruling rather than a
repair.** A9's axis two asks which linked crates install anything in the minimum
profile. All **557 systems report `<Enable the debug feature to see the name>`**,
so the census cannot attribute one.

⛔ **THE DATA DOES NOT EXIST, WHICH IS STRONGER THAN "HARD".**
`bevy_utils::DebugName` has **no field** without `bevy_utils/debug`; the string is
discarded at construction and no runtime accessor can recover it. `TypeId`
survives and carries no crate name.

⚠ **The inconsistency is what makes it a question.** `bevy_dev_tools` requires
`bevy_utils/debug`, and the minimum profile does not link `bevy_dev_tools` — the
workspace has the feature, the featureless facade does not. **But
`ambition_dev_tools` IS in the minimum profile**: facade-direct, not optional, so
a consumer selecting nothing links it, and its census attributes systems by name.

⇒ **A mandatory diagnostics crate sits in a profile that cannot supply the one
fact it reads.** That is vision.md's *"machine-readable diagnostics and
provenance"* failing at the profile the SDK offers.

**The trade is binary size against diagnosability.** ⛔ Adding a feature to the
minimum-profile fixture is not an option: its whole value is that everything it
links arrives implicitly.

⚠ **This ruling blocks A9's axis two and nothing else.** Axis one is closed —
false optionals are 0 of 13, and the non-optional closure is 48 excluding the
facade at `939d6aaa5`.

⭐ **The number was measured against `origin/main` at `58cc6c9a8`: Q99 was the
highest filed, in both that ref and this worktree.** If another agent claims Q100
concurrently, **renumber this one** — it was filed by the coordinator, not by the
agent who did the measurement, and it is the cheaper of the two to move.

## Q101 — may an ability's own contact satisfy the launching move's `Connected`?

`verdict_belongs_to` admits `None`, and `None` currently means *"credit whatever
move this body is playing NOW"*. ⭐ **The predicate's own census names every
production site that writes `attacker_move_instance: None` AND REACHES IT** —
`features/empowerment.rs`, `features/ecs/actors/update.rs:964`,
`features/enemies/integration.rs`, and `abilities/traversal/{blink,dive,mark_recall}.rs`.

⚠ **These are ABILITY AND CONTACT roads. Not hazards, not the blast zone.** The
comment's older defence of `None` named four legitimate producers; measured, only
one of the four reaches this predicate, *"and both of them were used to size a
change as too large to make."*

**The question.** For `blink`, `dive`, `mark_recall` and empowerment contact harm,
the player triggered the ability, possibly as part of a move.

- **If that contact SHOULD satisfy the launching move's `Connected` condition**,
  the occurrence must be propagated through those roads, exactly as `f9baa86e8`
  now does for projectiles.
- **If they are INDEPENDENT effects**, they must not mutate a move-local latch,
  and `None` must stop meaning *"credit the current move"*.

⛔ **It is not a bug fix either way.** *"Not attributable to this move"* may be
FALSE for blink and dive, so refusing `None` globally would silently stop
crediting contacts a player would expect to count.

⚠ **A12's reflection blocker cannot be closed without this answer.** Reflection
re-owns a shot and keeps the shooter's stamp; clearing the stamp yields `None`,
which credits whoever is playing — the same defect with an extra step — and
re-stamping is only correct if the reflection is itself move-authored, which in
general it is not. **Every road out passes through this ruling.**

Measured by ToothbrushAmbition; census verified in the predicate's own comment.
⭐ Q99 was the highest filed when this was written, in `origin/main` and in the
worktree, read in one command. **If another agent claims Q101 concurrently, this
one renumbers.**

## Maintenance rule

Do not add investigation transcripts beneath a question. Record enough source
context to make the decision, link the owner doc when useful, and stop. Once
answered, move the durable ruling to `maintainer-decisions.md` and delete the
question here.
