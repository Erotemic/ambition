# Redirect, 2026-08-11 — finish the architecture before broadening the migration

**This file OUTRANKS `overnight-campaign-2026-08-11.md` wherever the two disagree.**
It is Jon's redirect (relayed from a GPT review of checkpoint `853d9a66b5ed`),
recorded VERBATIM at his standing instruction so it survives a compact. The
campaign brief's P0.1–P5.42 order is superseded by section 20's P0–P4 order
below; everything the campaign said about the two coupled goals, the three
authorities, deleting legacy rows as each migration lands, and the prohibitions
(no `cargo fmt`, no git-diff commands, do not commit
`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`, do not weaken a regression) still stands.

Also standing, from the same message and NOT to be asked about again:

> **Do not invent characters solely for the 0140–0146 archetype demonstration.
> Delete the obsolete row, or convert it into a gallery composed from existing
> real characters plus explicit BrainProfile overrides if the mechanical
> demonstration is still useful.** Proceed without asking again.

---

## VERBATIM

# Redirect from 853d9a66b5ed — finish the architecture before broadening the migration

Continue from live HEAD. The reviewed checkpoint is:

```text
853d9a66b5ed
```

The recent work is valuable. Preserve it.

In particular:

* converting shared `medium_striker` behavior into reusable autonomous policy is the correct Group-B direction;
* the Goblin first adopter is useful;
* shrinking `character_archetypes.ron` from roughly 843 to 600 lines and removing eleven creature bodies from it is real progress;
* the new body-generic air dodge is valuable;
* the tumble → knockdown → tech → getup floor game is valuable;
* the richer directional/Smash moveset machinery is valuable.

But **pause broad character migration for a moment**.

D78 has exposed the remaining architectural fault line, and fixing that root now will make the rest of D73 dramatically simpler.

---

# 1. D78 IS NOT PRIMARILY A ROLLBACK BUG

You have already measured enough.

The important observations are:

```text
character-first enemy without WornCharacter
    → rollback green

same body + WornCharacter/persona projection
    → desync

same ActionSet present at construction
    → green

ActionSet introduced later by persona projection
    → desync
```

Stop spending another sequence of commits probing checksums around this.

The architecture has answered.

The problem is:

> **ordinary character construction is still two-phase.**

Today the rough path is:

```text
construct partial body
    ↓
attach WornCharacter
    ↓
Changed/Ref<WornCharacter> projection notices it
    ↓
derive ActionSet / moves / physical facts later
```

That is the wrong normal-spawn model.

A normal character actor should exist **complete on its construction frame**.

---

# 2. SEPARATE CHARACTER IDENTITY FROM RE-TEMPLATING

`WornCharacter` currently means two things at once:

```text
A. this body is an instance of Character X

B. please apply/reapply Character X's template to this body
```

Those are different concepts.

Split them.

The universal stable runtime fact should be conceptually:

```text
CharacterIdentity(CharacterId)
```

Every normal character-backed body carries it.

Merely carrying or changing observation state on `CharacterIdentity` must not be the mechanism ordinary construction depends on to populate its body.

Then make dynamic character replacement explicit.

Conceptually:

```text
apply_character_template(...)
RecharacterizeBody(...)
CharacterTemplateChange(...)
```

Exact API is yours to design.

That explicit operation is for things such as:

```text
possession transformations if character identity truly changes
character-select adoption
hot reload
intentional runtime re-template
```

It is **not** the normal initial-spawn constructor.

The important invariant is:

```text
CharacterIdentity
    = stable statement of template identity

Recharacterization operation
    = explicit request to replace/reconcile template-derived body state
```

Do not keep:

```text
Changed<CharacterIdentity>
    → reconstruct body
```

as the fundamental architecture after the rename.

That only renames D78.

---

# 3. NORMAL SPAWN MUST RECEIVE THE COMPLETE CHARACTER BODY AT CONSTRUCTION

Fix D78 by advancing D73.

When a character actor is constructed, create its:

```text
CharacterIdentity
BodyHealth
BodyAbilities
ActionSet
ActorMoveset
IdentityKit
movement model
movement tuning
body geometry
hurtboxes
contact/death traits
mount capability
held item
presentation identity
```

from the prepared character **before the entity begins simulation**.

There should be no next-tick persona grant required for correctness.

Then rollback sees a deterministic complete initial state.

The existing persona/re-wear projection can be reduced into a dynamic replacement/hot-reload facility.

That is the root fix I want.

Do not solve D78 by trying to make a delayed initial projection happen to be rollback-order-safe.

---

# 4. THE CURRENT `new_character_in` IS STILL A TRANSITIONAL CONSTRUCTOR

Despite the good direction, it still takes too much pre-unpacked parallel state:

```text
catalog
character_id
display name
max health
BrainProfile
CharacterBrain
locomotion
contact damage
dream seed
practice target
paths
...
```

and then asks the catalog again for some physical facts.

That is not yet:

```text
PreparedCharacterDefinition
+
construction context
→ body
```

It is still a hand-assembled projection.

Move toward a constructor consuming either:

```text
&PreparedCharacterDefinition
```

directly, or preferably an explicit complete lower value such as:

```text
CharacterBodyBlueprint
```

if `PreparedCharacterDefinition` still contains authoring/tool metadata irrelevant to construction.

For example:

```text
CharacterBodyBlueprint {
    character_id
    display/presentation
    body geometry
    locomotion
    movement tuning
    vitals
    weight
    abilities
    action set
    moveset
    hurtboxes
    contact traits
    death traits
    held item
    mount capability
    ...
}
```

Then:

```text
CharacterBodyBlueprint
+
SpawnContext
+
ControllerBinding
→ runtime actor
```

The constructor should not need to rediscover what the character is from:

```text
CharacterCatalog
CharacterRoster
ArchetypeSpec
```

---

# 5. `PreparedCharacterDefinition::is_complete_body()` IS TOO WEAK

Current completeness is effectively inferred from:

```text
locomotion.is_some()
```

That is a migration heuristic, not an engine contract.

Do not let "has locomotion" become synonymous with:

> this character can now bypass the archetype path.

Make completeness explicit.

Either make `PreparedCharacterDefinition` genuinely complete, with resolved defaults, or have preparation produce:

```text
Result<CharacterBodyBlueprint, MissingCharacterFacts>
```

where missing intrinsic data is enumerated explicitly.

Then the path decision is not:

```text
if locomotion exists:
    character-first
else:
    archetype
```

It is:

```text
complete character body available
    → character constructor

unmigrated character
    → temporary legacy migration path
```

and eventually the second case disappears.

---

# 6. THE SHARED `BrainProfile` IDEA IS RIGHT, BUT CLEAN ITS AUTHORITY NOW

`medium_striker` becoming a shared controller policy is exactly right.

But the current `BrainProfile` contains:

```text
attacks_player
```

That does **not** belong there.

It is also player-centric vocabulary, which the generic actor architecture should not use.

A controller policy answers things like:

```text
how aggressively do I close distance?
how far away do I notice eligible targets?
which attack do I prefer?
how do I patrol?
how difficult/tactical am I?
```

It should not decide:

```text
which social relationship is an enemy?
does this body attack "the player"?
```

Those belong to disposition/relationships/context.

You already have nearby concepts:

```text
SpawnDisposition
ActorDisposition
CombatStanding
team/faction relationships
```

Use them.

The brain should receive/observe **eligible hostile targets** and decide what to do about them.

So remove:

```text
BrainProfile.attacks_player
```

rather than carrying that legacy archetype field into the new policy model.

For the Giant GNU case:

```text
body = GNU
controller = stationary/passive controller
disposition/relationship = whatever the placement requires
```

Do not encode "doesn't attack the player" as an AI-profile body/social hybrid.

---

# 7. DELETE THE `smash_can_*` MIRRORS AS PART OF THIS PASS IF PRACTICAL

The profile itself already documents these as wrong:

```text
smash_can_blink
smash_can_fly
smash_can_shield
```

They are copies of body capability.

Do not let these survive while BrainProfile becomes the permanent shared-policy vocabulary.

The autonomous controller should inspect the controlled body's actual capabilities/action repertoire.

Then:

```text
same Smash AI profile
+
PCA body
→ AI can consider PCA abilities

same profile
+
Puppy Slug
→ AI cannot invent them
```

That is exactly the compositional behavior we want.

---

# 8. THE NAMED `BrainProfile` AUTHORING API NEEDS REAL IDENTITY SEMANTICS

The new concept is correct:

```text
several characters
→ name the same reusable BrainProfile
```

The current representation is not final.

Today:

```text
CharacterDefinition.autonomous_profile_ref: Option<String>
```

and production content manually writes something equivalent to:

```text
"ambition::medium_striker"
```

while raw tests may write:

```text
"striker"
```

Do not make content authors know whether the surrounding catalog has already namespaced them.

Introduce proper types conceptually like:

```text
BrainProfileRef
    provider-relative authored reference

BrainProfileId
    canonical resolved identity
```

A character definition in provider `ambition` should be able to author:

```text
medium_striker
```

and preparation resolves that to the canonical id.

An explicitly named profile which does not exist should be a **preparation error**.

It must not silently become:

```text
None
→ archetype remains in charge
```

That would reproduce the explicit-CharacterId fallback mistake at a different layer.

---

# 9. DO NOT CALL THE CURRENT INLINE PROFILE BEHAVIOR "SPECIALIZATION"

Current semantics are effectively:

```text
if inline autonomous_profile exists:
    use it

else if named profile exists:
    use named profile
```

That is replacement/precedence.

It is not:

```text
named shared profile
+
inline specialization patch
```

because nothing merges.

Do one of two things.

Simplest and preferable now:

```text
inline profile XOR named profile
```

and reject/clearly define simultaneous authoring.

Or, if actual specialization is already demonstrably useful, introduce a real:

```text
BrainProfilePatch
```

with explicit patch semantics.

Do not document whole-value replacement as specialization.

That becomes misleading API immediately.

---

# 10. DO NOT INSTITUTIONALIZE TWO PERMANENT AI VOCABULARIES

Your statement that a `BrainPreset` cannot be mechanically converted into `BrainProfile` without body context is correct.

A preset may contain absolute movement speeds while the new profile expresses normalized effort against the body.

Therefore:

> **do not build a generic converter.**

But the conclusion should **not** be:

> both vocabularies remain forever.

The migration should instead be semantic.

For every live `BrainPreset`:

```text
body-owned absolute locomotion facts
    → CharacterDefinition

decision policy
    → BrainProfile

relationship/context
    → SpawnContext / disposition
```

Then update its consumers and delete the old preset.

The end state should have one autonomous-controller-profile vocabulary.

Do not reproduce:

```text
old CharacterRoster
+
new CharacterRoster
```

as:

```text
BrainPreset
+
BrainProfile forever
```

---

# 11. `autonomous_profiles` SHOULD NOT MAKE CharacterCatalog THE NEW CONTROLLER AUTHORITY

It is reasonable to deserialize provider content from one package/file while migrating.

But conceptually:

```text
CharacterCatalog
```

should not own the permanent runtime registry for autonomous-controller profiles.

Move toward:

```text
CharacterRegistry / PreparedCharacterRegistry
BrainProfileRegistry
```

as separate authorities.

A provider package may register both.

That is normal:

```text
provider
    ├── CharacterDefinitions
    └── BrainProfiles
```

The fact they came from one RON document during migration does not mean the character catalog conceptually owns controller policy.

Avoid building another permanent fragment hierarchy mirroring `CharacterRosterFragment`.

---

# 12. THE GOBLIN MIGRATION IS A GOOD FIRST ADOPTER, BUT IT IS NOT YET THE FINAL PROOF

Keep it.

It correctly demonstrates:

```text
Goblin body
+
shared medium_striker policy
```

instead of:

```text
generic medium_striker body wearing Goblin art
```

But check that the Goblin's full intrinsic repertoire is resolved from the character definition/prepared body and not still indirectly coming from catalog `default_action_set`.

The acceptance test should be:

```text
delete medium_striker whole-body row

Goblin still has:
    Goblin health
    Goblin locomotion
    Goblin contact traits
    Goblin actual actions
    Goblin moves

shared BrainProfile still controls how it chooses among them
```

The policy should be reusable even if the old archetype row is physically gone.

---

# 13. PRODUCT DECISION: DELETE/RECAST THE 0140–0146 ARCHETYPE DEMO ROW

Do **not** invent six fictional characters solely to preserve:

```text
patrol cutter
small skitter
guard striker
medium striker
gradient seeker
large brute
```

as a one-of-each archetype museum.

Those names exist because the old architecture made body+AI into one thing.

Preserving them as fake characters would encode the old ontology into new content.

Two acceptable outcomes:

### Preferred if the row has little ongoing value

Delete it.

### If it remains a useful developer demonstration

Re-author it as a **composition/controller gallery** using existing real characters.

For example, deliberately demonstrate:

```text
Goblin + patrol profile
Goblin + striker profile

Puppy Slug + wander profile

Lab Raider + striker profile

some other real body + seeker profile
```

The exact cast is up to the existing content.

The point is to demonstrate:

> the same controller policy can drive distinct bodies, and the same body can use distinct policies.

That is much more valuable than preserving "one of every removed archetype."

Do not add lore characters just to keep demo coordinates occupied.

---

# 14. NOW THE IMPORTANT SMASH REDIRECT

The recent Smash mechanics are good.

Keep:

```text
true body-generic air dodge
tumble
knockdown
tech
getup
landing lag/autocancel machinery
strong attack variants
3–2–1–GO if already landed
ledge/shield/dodge integration already landed
```

These belong in the reusable engine and are exactly the kind of work Smash should contribute back to Ambition.

But **stop adding more move depth to the Smash-only shadow duelists for now.**

Current rich fighter moves are still centered on identities such as:

```text
smash_duelist_a
smash_duelist_b
```

with Robot art.

That does not satisfy our central product requirement.

---

# 15. MOVE THE REAL PlayerRobotV3 OFF HostCode NEXT

This should now be one of the highest-priority D73/Smash tasks.

The canonical:

```text
player_robot_v3
```

must have a normal authored:

```text
abilities
ActionSet
ActorMoveset
body tuning
```

like any other character.

Delete the need for:

```text
PlayableKitSource::HostCode
PreparedKit::HostCode
```

The repertoire that represents the real robot should live on the real reusable character definition.

Then both games consume it:

```text
Ambition
    PlayerRobotV3 CharacterDefinition

Smash
    SAME PlayerRobotV3 CharacterDefinition
```

Do **not** copy the current rich Smash moves from `smash_duelist_a` into a second independent Robot definition.

Move/refactor the canonical move data into the reusable Robot character provider and have both compositions reference it.

---

# 16. SAME ROBOT MOVESET, DIFFERENT GAME RULES

This remains the design target.

The robot's authored attack should own things such as:

```text
startup
active frames
recovery
hitbox geometry
damage
base launch
move identity
landing-lag/autocancel properties where they are truly move properties
```

Both Ambition and Smash use those same moves.

Then mode/ruleset supplies the different interpretation.

## Ambition

Closer to Hollow Knight:

```text
HP depletion
exploration lifecycle
lower / bounded damage-dependent knockback
little or no DI
progression gates
```

## Smash

Platform fighter:

```text
unbounded percent
stocks
blast zones
stronger percent-dependent knockback
DI
match lifecycle
```

Important correction:

**Do not permanently keep Ambition knockback flat.**

Jon's product goal is specifically a Hollow-Knight/Smash blend, closer to Hollow Knight but with damage-dependent knockback contributing to the fun.

At minimum the controlled robot should eventually have:

```text
damage increases launch
```

in Ambition too, but lower/capped relative to Smash because HP bounds the damage state.

Do this through rules/combat policy, not duplicate moves.

---

# 17. REMOVE `smash_fighter_kit()` AS THE CHARACTER MODEL BECOMES COMPLETE

The normal Smash path should be:

```text
seat Character X
→ use Character X's real ActionSet/Moveset
```

not:

```text
seat Character X
→ replace it with generic Smash swipe
```

The current generic kit can remain only as narrowly visible migration scaffolding while particular roster characters still lack real authored repertoires.

Shrink its adopter count continuously.

Do not add new adopters.

The goal is deletion.

---

# 18. `fighter_abilities` MUST NOT MANUFACTURE A BODY'S CAPABILITY

The recent masking behavior is moving in the right direction.

Final semantics should be something like:

```text
character/body capabilities
    + explicit mode grants
    - explicit mode restrictions
```

not:

```text
if character has no ability definition:
    Smash's generic set becomes its body
```

That latter fallback must disappear once characters are complete.

Keep the Puppy Slug regression as the poison:

```text
force Puppy Slug into Smash

movement:
    its actual crawler movement

attack:
    nothing if it has none

jump:
    nothing if it has none

air dodge:
    nothing unless its body/mode explicitly grants it

stocks/percent/blast zones:
    still work
```

That is the best compositional test we have.

---

# 19. USE THE NEW FLOOR GAME ON THE REAL ROBOT

Once canonical PlayerRobotV3 is seated with its real moves, verify the recently landed generic mechanics with that body:

```text
launch → tumble
failed landing → knockdown
tech input → tech
failed tech → knockdown
neutral getup
roll/getup options
getup attack if supported
air dodge
ledge behavior
shield/parry
landing lag
strong attacks
```

This is much more valuable than proving these systems on `smash_duelist_a`.

A successful playtest should demonstrate:

> the actual Ambition protagonist is a coherent platform-fighter character without becoming a separate Smash character.

---

# 20. PRIORITY ORDER FROM HERE

Do these in this order unless current HEAD has already completed one.

## P0 — resolve the root seam

1. Stop D78 rollback probing.
2. Separate stable `CharacterIdentity` from explicit template replacement/re-wear.
3. Make ordinary character spawn fully populate the template-derived body at construction.
4. Re-run D78.
5. Delete delayed initial persona projection from normal character spawning.

## P1 — harden the new controller-policy model

6. Remove `attacks_player` from BrainProfile.
7. Remove `smash_can_*` capability mirrors when practical.
8. Introduce typed provider-relative `BrainProfileRef` + canonical `BrainProfileId`.
9. Missing referenced profile is a preparation failure.
10. Clarify inline-vs-named semantics; preferably XOR until a real patch type is needed.
11. Publish profiles through a dedicated `BrainProfileRegistry`, even if provider parsing remains colocated temporarily.
12. Plan semantic migration and eventual deletion of `BrainPreset`; do not build a generic conversion layer.

## P2 — make the character constructor real

13. Replace `new_character_in`'s parallel unpacked parameters with PreparedCharacterDefinition / CharacterBodyBlueprint.
14. Eliminate catalog queries for already-prepared intrinsic body facts.
15. Replace `is_complete_body = locomotion.is_some()` with explicit completeness.
16. Route enemy/NPC/match through the same complete-body constructor.
17. Then resume broad Group-A/B/C migrations and delete rows as they become dead.

## P3 — canonical Robot + Smash

18. Move real `player_robot_v3` off HostCode.
19. Move/refactor its actual rich moveset into reusable canonical character authoring.
20. Seat the real Robot v3 in Smash using that same definition.
21. Remove the relevant `smash_duelist_*` shadow definition if packaging now permits.
22. Shrink/delete universal `smash_fighter_kit()`.
23. Shrink/delete capability-manufacturing `fighter_abilities` fallback.
24. Prove Puppy Slug forced seating remains capability-faithful.

## P4 — gameplay integration

25. Exercise air dodge/floor game/strong attacks/landing lag/ledge/shield with canonical Robot v3.
26. Give Ambition the lower/bounded damage-dependent knockback policy intended by the product design.
27. Keep Smash's stronger growth + DI + stocks interpretation.
28. Tune the shared mechanics instead of forking them.

---

# DELETION EXPECTATION

Recent work has been productive, but the campaign is still highly additive.

From the reviewed range, large legacy surfaces remain.

The next architectural milestones should visibly delete things.

Do not consider:

```text
new BrainProfile path exists
```

success while:

```text
BrainPreset
CharacterRoster
ArchetypeSpec
persona initial-construction path
HostCode protagonist path
generic Smash fighter replacement
```

all remain permanent alternatives.

Every new authority should allow an old one to shrink.

---

# DO NOT STOP ON THE DEMO-ROW QUESTION

The maintainer decision is:

> **Do not invent characters solely for the 0140–0146 archetype demonstration. Delete the obsolete row, or convert it into a gallery composed from existing real characters plus explicit BrainProfile overrides if the mechanical demonstration is still useful.**

Proceed without asking again.

---

# DEFINITION OF THE NEXT MAJOR MILESTONE

I want the next strong checkpoint to demonstrate all of this simultaneously:

```text
PlayerRobotV3
    is one canonical CharacterDefinition

Ambition:
    constructs that definition completely at spawn
    uses its canonical moves
    HP-based combat
    lower/bounded damage-dependent knockback

Smash:
    constructs the SAME definition completely at spawn
    uses the SAME canonical moves
    percent/stocks
    stronger knockback growth
    DI
    air dodge
    ledges
    floor-game reactions
```

and:

```text
Goblin
    owns Goblin body facts

medium_striker BrainProfile
    owns only reusable decision policy

hostility/relationships
    are contextual

no delayed persona projection
    is required to make either body correct
```

If that works, D73 has crossed from migration scaffolding into the professional architecture we actually want.

---

## PROGRESS AGAINST THE 28 ITEMS

Flip a box only when the DELETION or the contract it names has actually landed.
Section 20's numbering is preserved.

| # | Item | State |
|---|------|-------|
| 1 | Stop D78 rollback probing | ✔ — no further checksum probes; D78 is now a construction-shape task |
| 2 | Separate `CharacterIdentity` from re-templating | ✔ — `RecharacterizeBody` is the request; `Changed<WornCharacter> → populate` is deleted |
| 3 | Complete body at construction | ✔ — `grant_prepared_character_body`, one batch, memo included |
| 4 | Re-run D78 | ✔ — **still RED, at the same frame.** Two-phase construction was not the mechanism; see the ledger row |
| 5 | Delete delayed initial persona projection | ✔ for the character-first enemy road — the projection reads a constructed body as current and skips it |
| 6 | Remove `BrainProfile.attacks_player` | ✔ — deleted; the giant's placement says `Peaceful` |
| 7 | Remove `smash_can_*` mirrors | ✔ — deleted; `smash_cfg_from_spec` takes the body's `AbilitySet` |
| 8 | `BrainProfileRef` + `BrainProfileId` | ✔ — provider-relative authoring; content writes `medium_striker` |
| 9 | Missing named profile = preparation error | ✔ — refused at the finalize barrier |
| 10 | Inline XOR named | ✔ — authoring both is refused; the old precedence assertion is inverted |
| 11 | `BrainProfileRegistry` | ✔ — published beside the catalog; preparation asks the POLICY authority |
| 12 | Semantic `BrainPreset` migration plan | ✔ — ledger D81, census attached; `sniper_default` (0 adopters) already deleted |
| 13 | Constructor takes a blueprint | ✔ — `CharacterBodyBlueprint`; 14 args → 7 |
| 14 | No catalog re-queries for prepared facts | ✔ — gravity-freedom folds at preparation; only the SHEET-derived silhouette still reads art at construction |
| 15 | Explicit completeness, not `locomotion.is_some()` | ✔ — `body_blueprint() -> Result<_, MissingCharacterFacts>` |
| 16 | One complete-body constructor for enemy/NPC/match | ✔ — all three roads take the blueprint; the seat's three overrides are named |
| 17 | Resume Group-A/B/C migrations | ▢ (BLOCKED until 2–5 land) |
| 18 | `player_robot_v3` off HostCode | ▢ |
| 19 | Canonical robot moveset in the reusable provider | ✔ — `ambition_content::player_robot_moveset`, attached to v3 |
| 20 | Seat the real Robot v3 in Smash | ▢ |
| 21 | Remove `smash_duelist_*` shadow definitions | ▢ |
| 22 | Delete `smash_fighter_kit()` | ▢ |
| 23 | Delete the `fighter_abilities` manufacturing fallback | ▢ |
| 24 | Puppy Slug forced seating stays capability-faithful | ▢ |
| 25 | Floor game / air dodge / ledge on Robot v3 | ▢ |
| 26 | Ambition's bounded damage-dependent knockback | ▢ — but the RULESET seam it needs now carries `downward_hit` too |
| 27 | Smash keeps the stronger growth + DI + stocks | ✔ — `SMASH_KNOCKBACK_GROWTH` 0.01 → 0.02, 2026-08-11 |
| 28 | Tune shared mechanics rather than forking | ✔ for the down-air: one move, `Pogo` vs `Spike` declared per stage |
