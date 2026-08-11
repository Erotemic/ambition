# Overnight campaign: finish the character architecture and make Smash feel like a real platform fighter

⛔ **Jon's brief, verbatim, given 2026-08-11T03:0xZ.** Recorded here at his
instruction ("You might want to record this in docs planning so it survives
compaction"). Nothing below this banner is agent prose; the only agent addition
is the status block immediately after it. Where this file and
`character-template-architecture-2026-08-10.md` disagree, **this file wins** —
it is newer and it is his, and its "Overnight execution order" (P0–P5) replaces
that file's 23-item checklist as the ordering authority.

> **Baseline check, done at arming:** the brief's handoff baseline
> `6079f2233d7244e64a8c87123f92aac0da809b22` ("wip") IS an ancestor of HEAD, which
> is **11 commits newer**. Per the brief, that newer work is inspected before
> anything it names is redone. The measured state of it lives in
> `character-template-architecture-2026-08-10.md` ("REMAINING WORK — THE ONE
> CHECKLIST" and "Phase 1 progress"), which is where the per-item measurements
> are; use that file as the survey and this file as the order.

## ⇥ RESUME HERE (agent, keep current)

Seven slices have landed since arming; the table below carries the detail. What
a resuming session needs, shortest form:

* **The architecture frontier is P1.** The three authorities all EXIST as types
  — `CharacterDefinition` (+`abilities`, +`authored_moveset`), `BrainProfile`,
  `SpawnContext` — and the MATCH path is character-first as of P1.11:
  `ActorClusterSeed::new_fighter_in` builds a fighter with no archetype at all.
  ⇒ **the next callers are the authored ENEMY and the NPC** (P1.9/P1.10), and
  they are harder for a measured reason: an enemy's run speed, contact damage,
  move style and melee still have NO authoring surface on a definition, so a
  character-first enemy would be a body that cannot move or hurt anything. That
  is P1.8, and it is the true gate on the deletions.
* ⭐ **what the match slice exposed, and it is the shape to expect again**: the
  versus stage's fighters could only swing because the CPU ARCHETYPE's authored
  `melee` reached the body — the match's own ability mask (`basic()`) never
  granted `attack`. Taking the archetype away made a two-year-old omission
  visible in one test. Expect every character-first caller to expose one.
* **What still blocks P0.1's hard error**, measured: 28 authored enemy
  placements name a `character_id` that is not a registered character —
  `ai_slop`, `solid_snake`, both snakes (Mary-O) and `sanic_badnik` (Sanic).
  Migrating those five into registered characters is what lets the caller refuse
  instead of warn.
* **What still blocks a character authoring its controller policy**: nothing in
  the tree registers a `BrainProfile` yet. The archetype PROJECTS one; a
  character can only name a catalog `brain_presets` key. Group B is exactly this.
* ⇥⇥ **THE NEXT ARCHITECTURAL SLICE, measured 2026-08-11 and precise:** deleting
  an archetype ROW is the only real deletion available, because `ArchetypeSpec`
  requires `max_health`, `run_speed`, `patrol_effort`, `chase_effort`,
  `aggro_radius`, `attack_range`, `contact_strength`, `damage_amount`,
  `brain_template` and `move_style` — none of them `#[serde(default)]` — so a
  half-migrated row cannot shed its migrated fields. A whole row needs three
  things the character can now nearly all state:
  ```text
  body        vitals ✔  death traits ✔  locomotion ✔  contact damage ✔  action_set ✔
  controller  brain_template, efforts, aggro_radius, attack_range, smash_hit_band   ▢
  placement   respawn                                                              ▢
  ```
  ⇒ **the ONE missing piece is a character-authored `BrainProfile`.** Today
  `CharacterDefinition::default_brain_profile` is a REFERENCE into the catalog's
  `brain_presets` (the NPC road); the enemy road takes its profile from the
  archetype projection. Either a definition may hold an inline `BrainProfile`,
  or a registry maps a `BrainProfileRef` to one. Then the mites' rows go, and
  they are the first two lines the ledger has lost.
* ⭐ **THE LEDGER HAS MOVED: `character_archetypes.ron` is 799 lines, down from
  843.** The exploding mite and the dividing mite have no archetype row at all —
  the first rows this campaign has deleted. Both are built character-first: the
  enemy road asks `is_complete_body()` and, when a character can carry a body,
  constructs from it and lets the body WEAR itself so its kit arrives through
  the one persona writer. ⚠ one field still reaches a migrated enemy from the
  file — `respawn`, via the `combatant` fallback — because an `EnemySpawn`
  cannot author one; that is the next placement-authority fix, and the fallback
  happens to carry the same policy the deleted rows did.

## Campaign progress (live — update as slices land)

| P | Item | State |
|---|---|---|
| P0.1 | Explicit CharacterId missing from prepared registry must be an error | ◐ **the TYPE says it** — `CharacterSpawnPlan::definition` returns `Result<Option<..>, &CharacterId>`, with both regressions present (unmigrated ⇒ `Ok(None)`, authored-but-unprepared ⇒ `Err`). ▢ the CALLER still only warns, and the blocker is measured: 28 authored enemy placements carry a `character_id` that names an unregistered character — `ai_slop`, `solid_snake`, the two snakes (mary_o) and `sanic_badnik` (sanic). Hard-erroring today refuses the tree. ⇒ gated on migrating those five into registered characters, which is P2.13's demo half |
| P0.2 | Resolve character-owned autonomous profile refs during preparation | ✔ **DONE.** Preparation resolved the ref already; what was open is the half Jon names — *"should not need a parallel catalog row merely to know its namespace"*. It now qualifies with the DEFINITION's own provider (`qualify_in_provider`) and consults no catalog. The two id spaces were "assumed equal, never checked"; `character_provider_namespace` checks them on the shipped composition, and was probed RED by poisoning one registration site. The npc fixture that had argued against this change was repaired to ASSEMBLE its catalog (production namespaces every preset `provider::name`) rather than parse it raw |
| P0.3 | Complete typed CharacterId through prepared registry/runtime/match seams | ▢ |
| P0.4 | Inspect/narrow SpawnContext before adding more callers | ✔ **ALREADY NARROW** — two members, `feature_id` and `aabb`. The display name, faction and room kinematic paths were on it and were taken OFF for exactly Jon's reason (*"a Match participant should not need dummy room paths"*); the type's doc states the rule and names the three evictions. Re-inspected, nothing to remove |
| P0.5 | Fix current-held-item death ownership | ✔ **ALREADY DONE in the tree** — `CharacterDeathTraits::drops_held_item` is a `bool` policy and the drop path reads the body's LIVE held item (`actor_hit.rs`, `held_at_death`). Its own doc records the bug Jon describes: it used to be `Option<HeldItemSpec>`, so a body that picked up a different weapon dropped the one it was authored with. Not redone |
| P1.6 | Finish CharacterIdentity | ▢ |
| P1.7 | Move/finalize character domain types into the appropriate low crate | ▢ |
| P1.8 | Make PreparedCharacterDefinition complete for intrinsic construction | ◐ **capabilities, LOCOMOTION and CONTACT DAMAGE all authorable now** — `abilities` (verbs), `locomotion` (run speed, gait, surface cling, cling-breaks) and `contact_damage` (strength, amount), each `deny_unknown_fields`. A character can finally state how fast it is and whether touching it hurts, which is what a body needed the enemy archetype for. ▢ remaining before an enemy can be built character-first: melee/ranged action specs, mass, held item, respawn (placement) |
| P1.9 | Route authored enemy through character-first construction | ◐ **the road EXISTS and two characters take it.** A placement naming a COMPLETE character (one that states its locomotion) is built by `new_character_in` with no archetype, and wears itself so the persona derive writes its kit. ▢ every other enemy is still half-migrated and takes the legacy road with `adopt_character_intrinsics` patching over it — which is now that seam's only remaining job |
| P1.10 | Route NPC through the same body constructor | ▢ |
| P1.11 | Route PreparedMatch through it immediately after | ✔ **A SEAT'S BODY IS BUILT FROM ITS CHARACTER.** `ActorClusterSeed::new_fighter_in` takes no roster: size and art from the character's sprite, health and weight from its definition, aerial-ness from its catalog body kind, abilities from the ruleset mask, and the CPU's `BrainProfile` handed in as a VALUE (`CharacterRoster::brain_profile_for`) rather than resolved by building a creature. Every fighter on the grid used to be physically a `combatant` wearing a character |
| P1.12 | Route encounter, summon, programmatic paths | ▢ |
| P2.13 | Migrate clean Group-A character/archetype cases | ◐ **the two mites are FULLY migrated and their rows are gone** — health, run speed, gait, contact damage, the swipe, the death blast and the Smash policy are all on their definitions, split across the three authorities. ▢ the remaining seven Group-A characters (puppy_slug 10 spawns, burning_flying_shark 7, pirate_shark_rider 6, sky_parrot 2, giant_gnu 1, Iron Mary 1, ai_slop 1) |
| P2.14 | Delete each migrated legacy row as it becomes unnecessary | ◐ **44 lines out of `character_archetypes.ron`** (843 → 799) the moment the mites could carry their own bodies. The rule held: the deletion landed in the same change as the migration |
| P2.15 | Extract Group-B shared AI behavior into real BrainProfiles | ◐ **the TYPE exists** — `ambition_characters::brain::BrainProfile`, authorable, `deny_unknown_fields`, replacing `CharacterBrainSpec` outright and taking `aggro_radius`/`attack_range`/`turns_at_walls` off `ActorTuning`. ▢ the archetype still PROJECTS one; no character names a profile yet, which is what Group B needs |
| P2.16 | Classify Group-C generic roles | ▢ |
| P2.17 | Migrate provider roster fragments | ▢ |
| P2.18 | Delete CharacterRoster/ArchetypeSpec infrastructure | ▢ |
| P2.19 | Split/delete ActorTuning | ▢ |
| P2.20 | Remove hostile/provocation body reconstruction | ▢ |
| P2.21 | Remove rollback dependence on archetype identity | ▢ |
| P2.22 | Delete `character_archetypes.ron` | ▢ |
| P3.23 | Move Robot v3 off HostCode to normal character data | ▢ |
| P3.24 | Remove `smash_fighter_kit()` as the universal replacement | ◐ **it no longer replaces a real fighter's repertoire** — it is the action-set grant that lets a borrowed peaceful Hall NPC attack at all, and a character with authored moves now keeps them. ▢ the grant itself goes when those characters author their own |
| P3.25 | Remove universal `fighter_abilities` replacement | ◐ **it is a MASK, not a grant** — `seat_abilities` = character's authored verbs ∩ the mode's declared set; a ruleset may FORBID and may never hand a body a verb it lacks. Regression `a_match_cannot_grant_a_verb_the_character_does_not_have`, probed RED by swapping intersect→union. ▢ the bridge remains for characters that author nothing (almost all of them), and that is what deletes the field |
| P3.26 | Make Smash consume each character's actual body/capabilities/moves | ◐ **capabilities and MOVES both reach the seat now.** The blocker was invisible: a match's borrowed action-set grant regenerated the moveset from itself, so eleven authored move timelines lost to one derived swipe on the only path that seats a fighter. A grant covers the action set (*may this body attack*), never the moves (*what the attack IS*) — `authored_moveset` on the prepared value is what tells the two apart. ▢ the other roster characters still author no moves |
| P3.27 | Add Puppy Slug forced-seat regression | ◐ **the SEAM is pinned**: `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` seats a crawler that authors 36px/s, Slither and surface cling beside a character that authors none, and asserts the crawler keeps its own body while the unmigrated one still gets the stage's fighter default. Probed RED. ▢ the full end-to-end version — actually forcing `npc_puppy_slug` onto the Smash stage and pressing Attack/Jump — waits on puppy_slug's definition authoring its locomotion (it is still an archetype row) |
| P4.28 | 3–2–1–GO opening countdown | ✔ **LANDED.** `MatchRules::opening_countdown_ticks` + `OpeningPhase`, DERIVED from `now - activated_on` so there is no timer in the rollback window (`activated_on` is snapshotted — omitting it would restart the ceremony mid-match after a rewind). The release moved OUT of the Smash stage into match flow and frees every seat in one flush; the stage only says the numbers. Test `a_declared_countdown_holds_every_seat_until_it_ends` asserts BOTH states were observed and was probed RED twice |
| P4.29 | Wire shields/parry for appropriate fighters | ◐ **authored, reaching the seat, and it immediately exposed a CPU defect worth the whole exercise.** Giving the fighters `shield` turned the stage into two statues: `Disadvantage` covers CORNERED as well as hitstun, Shield outscored Retreat, and guarding does not un-corner anybody — an absorbing state reached in the opening second, per fighter, forever. Fixed where the genre says: a shield is a reaction to a SWING (gated on a hostile mid-attack), and a cornered fighter with nothing incoming retreats. ▢ still unverified in play: does the bubble block, does the parry window read |
| P4.30 | Wire grounded dodge | ◐ capability authored on the smash fighters; ▢ unverified in play |
| P4.31 | Implement true air dodge | ▢ |
| P4.32 | Enable and tune existing ledge mechanics in Smash | ◐ `ledge_grab` authored on the smash fighters — Jon's diagnosis was exactly right, *"the generic fighter capability set did not grant ledge_grab"*. ▢ verify grab/hang/climb/roll/getup-attack/jump/drop on the real stage, and fix what the first real adopter exposes |
| P4.33 | Author landing lag/autocancel on real aerials | ◐ **all five aerials author BOTH halves** (n/f/b/u/d-air, lag 0.10–0.28s, autocancel windows inside each move's duration) — `MoveSpec` has carried the pair for a while with no adopter, and the guard asserts both are present because an autocancel with no lag is silently inert. ▢ unverified in play: does landing mid-aerial actually lock control for the authored time |
| P4.34 | Add at least one real strong/Smash attack to Robot v3 | ✔ **F-smash, U-smash and D-smash, authored as MOVES.** No resolver change: the runtime already read a Smash-strength gesture off a directional flick and already resolved `smash_forward → attack_forward → attack`. The F-smash is 18 frames of startup, 15 damage, 150 base launch with 1.3 growth and a 1.7× charge payoff, against the jab's 3 frames / 3 damage / 55 launch — a different move by every measure that makes it one |
| P4.35 | Add tumble/knockdown/tech/getup state and animation slots | ▢ |
| P4.36 | Add stock-respawn protection | ✔ **LANDED.** Two seconds of the engine's generic `Empowered`/`UNTOUCHABLE` grant — the same timed untouchable a star pickup uses, already rollback-registered — inserted by the RULESET on a stock spend, never on an elimination. ⛔ the test found immediately that nothing in Smash ticked empowerments: the grant read `remaining: 2.0` five seconds later, permanent. `run_empowerments` is per-GAME registration (Mary-O and Sanic each schedule it) and Smash had never had an empowerment; registered, and noted as a footgun worth an engine-side fix |
| P4.37 | Tune hit feedback using existing generic hooks | ▢ |
| P5.38 | CPU AI chooses from actual character movesets/capabilities | ▢ |
| P5.39 | Remove obsolete Smash stand-ins | ▢ |
| P5.40 | Rerun PCA as an unconditionally registered character | ▢ |
| P5.41 | Clean architecture docs and stale comments | ▢ |
| P5.42 | Measure deletion payoff | ▢ |

---

Work from the live repository.

The handoff baseline was:

```text
6079f2233d7244e64a8c87123f92aac0da809b22
```

If HEAD is newer, inspect the newer work first and preserve anything that already satisfies this brief. Do not redo completed work simply because this prompt names it.

This is an **overnight execution campaign**, not an investigation-only assignment.

Keep working through independent tasks rather than stopping after the first architectural seam. Make coherent commits as major slices land. If one task becomes blocked by an unrelated defect, record the reproduction and move to the next independent item unless that defect truly blocks the architecture.

The two goals are deliberately coupled:

1. **Finish D73**: move Ambition toward the normal professional game-engine model where a character is a reusable authored template that can be instantiated arbitrarily many times, and delete the old enemy-archetype ontology rather than wrapping it.
2. **Use Smash as the proving ground**: by the next playtest, the Smash demo should feel materially more like a real platform fighter—native character moves, real defensive/recovery options, proper round opening, meaningful aerial/landing states, stocks/percent/knockback rules, and less “generic characters walking around an arena.”

Do not treat these as competing projects.

Smash is one of the strongest tests of whether the character architecture is actually compositional.

---

# Product model

The engine should ultimately have this simple story:

```text
CharacterDefinition("Goblin")
    ↓ instantiate
Goblin actor A
Goblin actor B
Goblin actor C

CharacterDefinition("Fretjaw")
    ↓ instantiate
Fretjaw actor A
Fretjaw actor B
```

Character identity is **template identity**, not singleton runtime identity.

```text
CharacterId
    = what reusable character this actor instantiates

SimId / FeatureId
    = this particular runtime actor

controller
    = who or what currently drives this body

spawn/session context
    = why this instance exists and what contextual rules apply
```

A fictionally unique named character may be spawned twice.

A generic Goblin and a named Fretjaw are the same kind of engine definition.

What differs is authored content, not actor ontology.

This is the model associated with reusable Prefab/Blueprint/PackedScene-style authoring in mainstream general-purpose engines.

---

# The three authorities

Every fact migrated out of the legacy model must end in exactly one of these categories.

## Character definition — what this character IS and CAN DO

Examples:

```text
body geometry
body/movement model
mass / knockback weight
vitals baseline
intrinsic movement capabilities
intrinsic abilities
action repertoire
moveset
attack volumes
hurtboxes
intrinsic equipment/loadout
mount/pilot body capabilities
intrinsic death traits
presentation
voice
```

Reusable low-level profiles are fine if they help author these facts, but the **prepared character** must resolve them into one complete answer.

## Controller / autonomous profile — how an autonomous participant chooses actions

Examples:

```text
brain strategy
patrol/chase policy
aggro distance
spacing preference
attack-selection policy
fighter difficulty
wall-turn behavior
Smash tactical preferences
```

A controller supplies intent.

It does not manufacture body capabilities.

## Spawn/session/ruleset context — what is true about this instance here

Examples:

```text
current controller
team/faction
disposition
encounter membership
respawn policy
stocks
match rules
story/placement identity
patrol route
```

A Goblin can be friendly, hostile, human-controlled, CPU-controlled, summoned, or seated in Smash without becoming a different character.

---

# Hard architectural rule

Do not migrate `ArchetypeSpec` wholesale into another struct.

The legacy archetype is a god-object because it currently combines all three categories.

The successful endpoint is not:

```text
ArchetypeSpec
    renamed to CharacterProfile
```

The successful endpoint is:

```text
CharacterDefinition
+
BrainProfile
+
SpawnContext / ruleset state
```

with the legacy god-object deleted.

---

# Current state at the handoff

Important work has already landed.

Preserve it unless current HEAD supersedes it.

## Already good

* typed `CharacterId` exists;
* `WornCharacter` now stores typed character identity;
* gameplay identity for an authored enemy no longer comes through `sprite_character_id` / display-name presentation lookup;
* `BrainProfileRef` and resolved `BrainPresetId` are separate concepts;
* authored death traits were moved below the runtime combat crate as `CharacterDeathTraits`;
* the optional-`ActorMoveset` query-membership bug was fixed;
* the double moveset-mint bug was subsequently fixed;
* a real upstream `CharacterSpawnPlan` exists rather than merely merging the already-lowered NPC/enemy plans;
* an authored enemy can consult a prepared character before spawning;
* duplicate instances of one character definition are already understood/tested as legal.

## Still transitional

The legacy architecture is still heavily present.

At the handoff, source use remained widespread:

```text
ArchetypeSpec         → many production files
CharacterRoster       → many production files
spec_for_brain        → many production files
character_archetypes.ron still exists
PlayableKitSource / HostCode still exists
PreparedMatch still uses CharacterRoster
Smash still injects smash_fighter_kit()
Smash still replaces fighter abilities with one generic set
```

Do not declare D73 finished while those facts remain.

---

# FIRST: repair the current CharacterSpawnPlan correctness hole

Current `CharacterSpawnPlan::definition()` effectively returns `None` for two different cases:

```text
A. placement has no character id yet
B. placement explicitly names CharacterId X but X is absent from PreparedCharacterRegistry
```

Those must not be equivalent.

During migration:

```text
no character authored
    → temporary legacy fallback is allowed and visible

explicit character authored but not prepared
    → construction ERROR
```

An authored:

```text
character_id = IronMary
```

must never silently produce a shark-rider body because Iron Mary was accidentally omitted from some registration list.

Implement a typed failure/result.

Conceptually:

```text
resolve_character(...)
    -> Result<Option<&PreparedCharacterDefinition>, MissingPreparedCharacter>
```

where `Ok(None)` means specifically “this legacy placement has not yet been migrated.”

Add a poison regression proving that an explicit missing character fails loudly.

At the final D73 endpoint the `Option` disappears entirely for normal character spawns.

---

# Resolve character-owned brain references during preparation

The authored form may carry a provider-relative controller reference:

```text
BrainProfileRef
```

but a `PreparedCharacterDefinition` should be prepared.

It should contain the canonical resolved autonomous-profile identity:

```text
BrainProfileId / BrainPresetId
```

not an unresolved local reference which requires `CharacterCatalog` again at spawn time.

Target:

```text
CharacterDefinition {
    provider = foo
    default_autonomous_profile = "fighter"
}

        ↓ prepare

PreparedCharacterDefinition {
    default_autonomous_profile = foo::fighter
}
```

A placement override may remain provider-relative until the placement itself is lowered.

The character's own prepared default should not need a parallel catalog row merely to know its namespace.

---

# Finish typed CharacterId propagation

`CharacterId` should stay typed through:

```text
authored character source
PreparedCharacterDefinition
PreparedCharacterRegistry key
CharacterSpawnPlan
runtime CharacterIdentity
match participant preparation
encounter/summon requests
```

Do not repeatedly convert authoritative character identity back to `String` and then reparse/recompare it.

Strings are appropriate at:

```text
RON/serialization boundaries
UI/debug rendering
external authoring text
```

not between engine authorities.

---

# Generalize WornCharacter into the universal character identity

Do not introduce a second runtime component saying which character an enemy is.

The current `WornCharacter` already contains the right fundamental fact.

Evolve/rename it into something like:

```text
CharacterIdentity(CharacterId)
```

if that name best describes the final semantics.

A body carrying it means:

> this runtime actor is an instance of this CharacterDefinition.

It does **not** mean:

> this is specifically the controlled protagonist or a temporary costume.

Audit the current persona-derive side effects before widening it.

The final architecture should not rely on:

```text
insert CharacterIdentity
    ↓ later update system notices it
    ↓ reconstructs half the body from CharacterCatalog
```

Ordinary construction should produce the complete body directly from the prepared definition.

A later runtime character transformation/re-template may legitimately use a separate reconciliation operation.

Do not let that special dynamic operation dictate normal spawn construction.

---

# Finish the common character constructor

This is the central D73 milestone.

All ordinary character spawn surfaces must lower to one semantic construction operation.

Keep separate authoring schemas where they make sense:

```text
NpcSpawn
EnemySpawn
EncounterMobSpec
SummonSpec
MatchParticipant
programmatic character request
```

but lower them into a common input roughly like:

```text
CharacterSpawnPlan {
    character: CharacterId,
    controller selection / autonomous override,
    minimal shared spawn facts,
}
```

paired with:

```text
PreparedCharacterDefinition
```

then build generic runtime components once.

Do not make `SpawnContext` a giant collection of everything an `EnemySpawn` currently happens to know.

Current fields such as:

```text
room kinematic paths
feature display name
faction
```

may need to move into narrower contextual/controller/relationship inputs as the second and third callers arrive.

A Match participant should not need dummy room paths.

A summon should not need to invent an LDtk feature name.

Let additional callers expose what is truly shared.

---

# Construction acceptance condition

The final ordinary path must resemble:

```text
PreparedCharacterDefinition
+
CharacterSpawnPlan
        ↓
CharacterIdentity
BodyHealth
BodyKinematics / movement model
BodyAbilities
ActionSet
ActorMoveset
hurtboxes
combat traits
presentation identity
etc.
```

Then independently:

```text
ControllerBinding
BrainProfile
team/faction
respawn/lifecycle
interaction
encounter state
```

attach their contextual facts.

No ordinary constructor should first build:

```text
ArchetypeSpec creature
```

and then patch the character over it.

`ActorClusterSeed::adopt_character_intrinsics` is a temporary probe seam.

Do not grow it until it applies every legacy archetype field.

Replace that pattern with character-first construction.

---

# Move CharacterDefinition and prepared domain types to their proper crate

Use:

```text
.agent/index/crates/graph-resolved.json
```

before changing crate dependencies.

The low character-domain types should live in the lowest natural reusable character crate, probably `ambition_characters` if the live graph still supports that direction.

Use the crate boundary as a design test.

The earlier `CombatCapabilities` problem was handled correctly:

```text
runtime CombatCapabilities
    did not belong on authored CharacterDefinition
```

so a lower `CharacterDeathTraits` fact was extracted and then lowered into the runtime component.

Repeat that reasoning for every dependency obstacle.

Do not solve dependency cycles by leaving the authoritative character model inside the actor monolith.

---

# PreparedCharacterDefinition must become COMPLETE

Source authoring may remain compact and optional.

Prepared data should not mean:

```text
None
→ leave whatever the old archetype happened to say
```

for intrinsic character facts.

Preparation should resolve:

```text
explicit character values
+
reusable profiles/defaults
+
provider defaults where appropriate
```

into a complete immutable character template.

Normal construction then needs no second gameplay registry to answer what the character is.

For facts whose ordinary value is “none”:

```text
death traits = default/no special behavior
```

prefer an explicit resolved default in prepared data rather than optionality whose hidden meaning is “ask another authority.”

Keep `Option` only where absence is itself meaningful.

---

# Fix held-item death ownership before migrating it broadly

`CharacterDeathTraits` currently includes something like:

```text
drops_held_item: Option<HeldItemSpec>
```

but Ambition's architecture says the runtime body owns its currently held item.

If the intended rule is:

> when this kind of character dies, drop whatever they are currently holding

then author:

```text
drop_current_held_item_on_death: bool / policy
```

and inspect the live held item when death happens.

Do not snapshot an authored weapon into character death metadata and then drop that stale item after the actor has changed equipment.

If some character instead always produces a specific loot item, model that explicitly as death loot.

Do not conflate the two.

---

# Route every construction surface through the character path

Once the constructor exists, migrate callers in an order that continuously increases proof.

Recommended order:

## 1. Authored enemy

Already partially there.

Make it fully character-first.

An authored character's:

```text
health
body
movement
abilities
actions
moves
death traits
mountability
```

must not come from `spec_for_brain`.

## 2. NPC

The same character in peaceful NPC form and hostile enemy form must have the same intrinsic body and repertoire.

Only:

```text
controller
disposition
interaction
placement context
```

differ.

## 3. PreparedMatch / Smash

Do this **early**, not at the very end.

It is the strongest architecture test.

Remove `PreparedMatch`'s dependency on:

```text
CharacterRoster
ArchetypeSpec
```

The exact same Fretjaw definition should be seatable under:

```text
Human controller
CPU controller
```

without reconstructing another fighter body underneath.

## 4. Encounter mobs

Replace:

```text
kind = what it does
character = what it looks like
```

with real character identity plus optional controller override.

## 5. Summons / programmatic spawns

Replace `archetype_id` / `SpawnActorKind::Enemy { brain }`-style actor construction with character-first requests.

## 6. Provider characters

A provider must be able to add a complete new character plus autonomous profile with **zero actor-engine source edits**.

---

# Start deleting legacy rows immediately after each migration

Do not postpone all deletion until a final cleanup phase.

For each migrated character/profile group:

```text
move intrinsic facts
move autonomous policy
move contextual facts
update consumers
DELETE migrated legacy fields/rows
```

When a legacy row has no remaining semantic owner, delete it immediately.

The diff should begin trending negative.

---

# Migrate character_archetypes.ron semantically

There is intentionally **no universal archetype → character mapping**.

The ids are different namespaces.

That is correct.

Use the existing one-time migration ledger as evidence, then delete it when migration is complete.

## Group A — clear character/profile pairings

Start with characters where the intended mapping is obvious, such as the mite family and other one-character/one-profile cases.

For each:

```text
intrinsic body facts → CharacterDefinition
AI policy            → BrainProfile
respawn/faction      → placement/context
```

Delete the old row afterward.

## Group B — shared behavior profiles

Cases like `medium_striker` are proof that the shared entity is **AI/controller policy**, not actor identity.

Multiple real characters may reference the same `BrainProfile`.

Their bodies remain distinct definitions.

## Group C — generic role names

Classify them.

If a visible recurring thing genuinely exists in the world:

```text
Goblin
GoblinHeavy
TrainingTarget
...
```

give it a real reusable CharacterDefinition.

If it is a true fixture/debug-only construct, use an explicit fixture API.

Do not create fake character identities solely to satisfy uniformity.

If temporary art is borrowed, use a presentation override rather than lying about CharacterId.

---

# Delete the old roster authority

The final production tree should no longer need:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
character_archetypes.ron
enemy_roster.rs
```

Provider roster fragments should become normal registration of:

```text
CharacterDefinition
BrainProfile
provider/game metadata
```

No compatibility re-export.

No `LegacyCharacterRoster`.

No empty dead registry left because tests used to instantiate it.

---

# Split/delete ActorTuning

`ActorTuning` currently mixes too many authorities.

Move its surviving facts to the components that actually own them:

```text
movement tuning      → character/body movement
health                → BodyHealth/vitals
AI distances          → BrainProfile/controller policy
contact behavior      → body/combat trait where intrinsic
faction/hostility     → relationship/disposition
respawn               → lifecycle context
death policy          → body/ruleset authority
visual character id   → CharacterIdentity/presentation
```

If a small coherent runtime tuning component survives, rename it for exactly that responsibility.

Do not retain `ActorTuning` as a miscellaneous compatibility projection.

---

# Provocation: delete body reconstruction

A peaceful named actor is already the complete character.

Example:

```text
Fretjaw in Hall
    CharacterIdentity = Fretjaw
    Fretjaw vitals
    Fretjaw movement
    Fretjaw actions/moves
    peaceful disposition
    ambient controller
```

Provoking Fretjaw should produce:

```text
same CharacterIdentity
same vitals
same movement
same abilities
same moves

different disposition
different autonomous controller
```

Delete name/dialogue-string heuristics which select a hostile body archetype.

Delete:

```text
HostileArchetypeId
provoked archetype reconstruction
health/body/kit rewrites on provocation
```

once rollback no longer needs them.

---

# Rollback: preserve character identity and controller identity separately

Rollback should snapshot/restore:

```text
CharacterId
runtime mutable body state
controller/profile binding
disposition/context state
```

It should not need:

```text
legacy archetype id
→ rebuild what this character physically is
```

Update schema/checksum/registration coherently whenever rollback-owned state changes.

Do not create a newly named replacement for `HostileArchetypeId`.

---

# Remove protagonist HostCode special treatment

`player_robot_v3` must become a normal complete CharacterDefinition.

Delete the final need for:

```text
PlayableKitSource::HostCode
PreparedKit::HostCode
```

The character definition owns its real move repertoire.

The runtime body owns:

```text
progression unlocks
equipment
temporary grants
current inventory
session restrictions
```

The protagonist should not need a separate “ask application code what moves I have” branch.

This is especially important for Smash.

---

# SMASH: the character refactor is the foundation, not a separate project

The Smash demo currently has several good engine systems underneath it:

* stocks;
* blast-zone deaths;
* unbounded damage percent;
* percentage HUD;
* match winner flow;
* damage-scaled knockback;
* DI;
* hitlag/hitstun;
* body-generic combat resolution;
* body-generic movement;
* jump squat;
* landing-lag/autocancel support;
* shield/parry infrastructure;
* dodge infrastructure;
* substantial ledge-grab/getup machinery;
* real ActorMoveset / directional attack infrastructure.

The reason the demo still feels generic is largely that many of these systems are either not adopted by the fighters or are being overwritten by historical leveling hacks.

Use D73 to expose them properly.

---

# Smash principle: same character, different rules

For Player Robot v3 in particular, the target is:

```text
same CharacterDefinition
same moveset
same attack timings
same attack volumes
same authored damage
same authored base launch

Ambition interpretation:
    HP depletion
    low/flat knockback growth
    little/no DI
    exploration progression/context

Smash interpretation:
    percent accumulation
    stocks/blast zones
    damage-scaled knockback
    DI
    match lifecycle
```

Do not make:

```text
PlayerRobotV3Ambition
PlayerRobotV3Smash
```

or maintain two copies of its attacks.

The ruleset changes the interpretation, not the character identity.

---

# Remove Smash's generic fighter leveling hacks

Current Smash currently does roughly:

```text
every selected character
    .with_action_set(smash_fighter_kit())

roster.fighter_abilities =
    same move/jump/double-jump/dash/attack set for everybody
```

These were compensating for the old broken character model.

Remove them as soon as the characters are complete enough to survive without them.

The curated Smash roster may require a usable fighter kit.

The engine must **not manufacture one for arbitrary characters**.

---

# Puppy Slug is an explicit compositional acceptance test

Force a Puppy Slug into Smash through a test/debug setup even though it is not normally selectable.

Expected behavior:

```text
CharacterIdentity = PuppySlug

movement input
    → uses Puppy Slug's actual authored locomotion

Attack
    → no action if Puppy Slug has no attack

Special
    → no action if none is authored

Jump
    → no jump if its body cannot jump

stocks / damage / knockback / HUD / blast zones
    → still work normally
```

Smash must not silently give it:

```text
generic swipe
generic humanoid jump
generic dash
generic shield
```

This proves:

```text
controller ≠ capability
ruleset ≠ moveset
buildable ≠ Smash-selectable
```

Keep Puppy Slug off the normal roster unless there is a deliberate product decision to make it a fighter.

---

# Player Robot v3: make it the first polished real fighter

Once HostCode is removed, make Robot v3 the strongest end-to-end proof.

Its authored repertoire should be usable both in Ambition and Smash.

Do not create duplicate Smash move definitions.

At minimum make sure the shared moveset meaningfully supports:

```text
grounded basic attack(s)
up/down directional grounded attacks where authored
neutral/forward aerial
back air
up air
down air / pogo where appropriate
ranged/special where genuinely part of the robot
```

Use the existing directional `ActorMoveset` architecture rather than adding Smash-only combat dispatch.

---

# Add real strong / Smash attacks

The engine already has:

```text
SMASH_VERB
AttackVariant::FSmash
AttackVariant::DSmash
AttackVariant::USmash
```

but these are not yet a meaningful part of play.

Land a body-generic strong-attack gesture.

A reasonable implementation may use:

```text
held attack / dedicated strong input / existing Smash verb
```

according to the current input architecture.

Do not hardcode Robot behavior into the resolver.

Author the actual moves on the character's moveset.

At minimum give Robot v3 one satisfying forward Smash attack and, if the existing architecture permits cleanly, Up Smash and Down Smash as well.

Strong moves should differ materially in:

```text
startup/commitment
damage
base launch
hitlag/feedback
recovery
```

rather than merely naming the normal swipe differently.

---

# Turn on landing lag and autocancel for real aerials

The mechanics already exist.

At least the principal Smash fighter's real aerial moves should author meaningful:

```text
landing lag
autocancel windows
```

so landing during an aerial has the expected platform-fighter commitment.

Do not build a second Smash-specific landing-lag subsystem.

Exercise the generic one.

Add focused tests for:

```text
landing during active/non-autocancel aerial → landing lag
landing inside autocancel window            → little/no landing lag
```

---

# Wire shields and parry into Smash-capable characters

The body-generic shield/parry infrastructure already exists.

Once generic `fighter_abilities` is removed, appropriate characters should explicitly author:

```text
shield capability
```

and the control affordance should expose it.

Verify in actual match construction:

```text
shield raises
incoming hit is blocked
parry/rising-edge behavior works if currently designed that way
shield does not appear on a body that lacks the capability
```

Do not create `SmashShield`.

Use the shared combat system.

If the shield currently lacks genre-important feedback such as clear block hitlag/stun, measure and tune using the generic shield path.

---

# Ground dodge and real air dodge

The core already has a grounded dodge/roll path with invulnerability.

Make sure appropriate Smash fighters actually receive/use it through their character capability data.

Then inspect aerial Dodge behavior carefully.

The UI/affordance language has implied an aerial dodge in some contexts, but the existing `apply_dodge` implementation is grounded.

Implement a **real body-generic air dodge** if it is still absent.

Requirements:

```text
directional input
finite invulnerability
clear travel/velocity behavior
recovery/end lag
cannot be spammed infinitely in one airtime
refreshes according to an explicit landing/ledge/lifecycle rule
```

Model its semantic state explicitly enough that animation/debugging can distinguish it from a ground roll.

Do not merely reinterpret aerial dash as “close enough” if the gameplay state is different.

---

# Adopt the existing ledge system in Smash

There is already extensive ledge machinery:

```text
grab
hang
climb
roll
getup attack
ledge jump
drop
regrab cooldown
ledge invulnerability
momentum carry
```

The Smash fighters currently do not appear to be exercising it because the generic fighter capability set did not grant `ledge_grab`.

Give appropriate character definitions the capability.

Then verify in the real Smash stage:

```text
fall past edge
grab ledge
hang
jump from ledge
neutral getup/climb
roll getup
getup attack
drop
regrab cooldown
```

Use the generic implementation.

Fix integration/tuning defects that the first real adopter exposes.

This should have a very large next-play payoff without needing to invent a new mechanic.

---

# Add tumble / knockdown / tech / getup as body-generic combat states

This is one of the largest remaining “doesn't feel like a platform fighter” systems.

Build it generically enough that Ambition can reuse the mechanics where appropriate.

A sufficiently launched/hitstunned actor making a relevant collision should have an explicit reaction state.

Conceptually:

```text
launched / tumble
    ↓ contact during tech window
successful tech
    → tech in place
    → directional tech roll

failed tech
    → knockdown
        → neutral/slow getup
        → getup roll
        → getup attack
```

Consider walls/ceilings only if they fit cleanly after ground tech; do not block ground tech on implementing every surface at once.

Important architectural requirements:

* this is body/combat state, not a Smash-only entity marker;
* expose tuning values rather than scattering magic constants;
* rollback-register new authoritative state coherently;
* do not use `dodge_roll_timer` as the only semantic representation merely because it already provides invulnerability;
* share invulnerability projection where appropriate, but preserve distinct maneuver/reaction identity;
* add character animation slots/fallback mappings even if bespoke art is not available yet.

The user has explicitly wanted:

```text
knocked-down animation
slow/neutral getup
tech
getup attack
```

so create the architecture slots even where existing sprites must temporarily fall back.

---

# Opening countdown: 3 – 2 – 1 – GO

Smash already opens its roster suspended.

Currently the stage removes `ScriptedControl` as soon as `ActiveMatch` exists because there is no ceremony.

Replace that with an actual opening countdown.

Use the existing:

```text
opens_suspended
ScriptedControl
```

contract rather than creating a parallel input-lock mechanism.

Expected:

```text
fighters spawn
camera frames cast
3
2
1
GO
all active seats release atomically
```

Use existing HUD/banner/audio infrastructure where practical.

The countdown must be ruleset/match flow, not participant-specific hacks.

Add a deterministic test proving bodies remain held before GO and release at the transition.

This is a high-priority next-play feature.

---

# Respawn protection

After losing a stock, a returning fighter should not be immediately vulnerable during the first instant of materialization.

Add explicit Smash/ruleset-owned respawn protection.

Prefer a generic temporary invulnerability/intangibility mechanism already present in the body system.

Make the policy configurable.

Reasonable behavior:

```text
respawn
→ temporary protection
→ expires after duration
and/or
→ clears when the fighter commits an attack
```

depending on what fits the engine cleanly.

Do not bake the rule into `CharacterDefinition`.

The same character in Ambition need not receive Smash stock-respawn protection.

A full angel platform is optional; clear lifecycle protection is the important mechanic.

---

# Keep knockback policy where it is

Do not duplicate knockback formulas into character moves.

The architecture currently has a strong seam:

```text
move:
    authored base launch
    optional intrinsic growth if genuinely special

ruleset:
    default knockback growth
    DI policy
```

Smash already declares percent-scaled growth.

Ambition remains flatter by default.

Preserve this.

Use the same Robot v3 attack in both games as a regression:

```text
same attack at low/high accumulated damage

Ambition:
    approximately stable launch

Smash:
    high-damage target launches substantially farther
```

This is exactly the intended Hollow-Knight/Smash blend.

---

# Hitlag, hitstun, strong-hit feedback

Audit the actual play result once native moves land.

Do not rewrite the underlying hitlag/hitstun systems unless the measurement shows a defect.

Tune existing generic policies so strong platform-fighter hits have readable impact.

Use existing hooks for:

```text
hit freeze
camera shake
launch VFX
SFX
trails
```

where available.

Prefer scaling feedback from actual resolved hit/launch severity rather than move-name special cases.

The goal for the next playtest is for a strong hit to **feel** materially different from a weak poke.

---

# CPU AI must use the real character

After D73, Smash CPU policy should not carry a hidden whole-body archetype.

The controller profile chooses actions.

The character body exposes the available actions/capabilities.

The fighter brain already has machinery for inspecting the actual moveset and distinguishing attack/smash/special choices.

Use that.

A CPU controlling Puppy Slug should not invent a melee move.

A CPU controlling Robot v3 should understand the robot's real attacks.

CPU difficulty/strategy remains a controller-profile fact.

---

# Curated roster versus forced character

Keep:

```text
SMASH_ROSTER
```

as an explicit product/content selection if useful.

But its semantics should become simply:

> these are the characters the normal select screen offers.

It should not control whether the engine can build those characters.

Likewise:

```text
BUILDABLE_ONLY_CAST
```

must disappear before D73 closes.

Final invariant:

```text
complete registered CharacterDefinition
    → buildable

SMASH_ROSTER
    → normally selectable in this mode
```

If a test/dev caller explicitly seats an unlisted character, construction should still work.

---

# Remove stand-in/copy characters where the new provider model makes them unnecessary

The standalone Smash demo currently contains some stand-in copies of robot-lineage characters so it can run without the Ambition composition.

Once provider/character registration is clean, inspect whether the standalone demo can consume the real reusable definitions without duplicating character identity.

Prefer:

```text
same real CharacterDefinition provider
```

over:

```text
smash_duelist_a wearing Robot art
```

Do not force deletion if the standalone packaging boundary genuinely cannot import the content provider cleanly, but document the exact dependency reason.

Copies should not survive merely because they predate the new registry architecture.

---

# Character-select validity

Once D73 is complete:

```text
CharacterDefinition registered
→ buildable
```

The select screen may then validate its curated roster against that registry.

Do not allow:

```text
portrait selectable
match cannot construct it
```

but also do not make the UI roster the source of buildability.

This should permanently remove the old PCA registration class of failure.

After character registration becomes unconditional, rerun the PCA reproduction.

If the old movement/load-timing divergence still exists, keep it as a separate D74 bug.

Do not “fix” it by un-registering PCA again.

---

# Player Robot v3 and progression

Move Robot v3's actual combat definitions into normal authored character data.

Keep the distinction:

```text
character repertoire
    = what moves exist for this character

runtime progression/grants
    = which capabilities this body currently has unlocked
```

Do not duplicate move definitions per progression stage.

If a locked action is unavailable, input simply cannot execute it.

For Smash, use the character's same move definitions.

If the mode intentionally grants a canonical fighter loadout, represent that as an explicit runtime grant/policy referencing existing character capabilities.

It must not synthesize an attack that does not exist.

This is the same compositional rule demonstrated by Puppy Slug.

---

# Ambition must benefit from the Smash work

Do not create platform-fighter mechanics in a separate Smash-only physics stack.

Features such as:

```text
better hit reactions
tumble/tech/getup architecture
move-specific landing lag
directional attacks
strong attacks
shield/parry
body-generic ledges
better knockback reactions
```

belong in the reusable engine/body/combat layers where sensible.

Ambition may configure or adopt a subset.

The user's intended Ambition feel is:

> closer to Hollow Knight, but intentionally influenced by Smash.

In practice that means it is reasonable for Ambition to retain:

```text
HP-based combat
tighter/smaller rooms
lower/flat knockback growth
little/no DI
different death/checkpoint rules
exploration progression
```

while sharing the same:

```text
moves
hitboxes
hitlag
hit reactions
body capabilities
combat geometry
movement primitives
```

with Smash.

Do not fork the character for each game.

---

# Lower-priority Smash work if the primary list lands

Do these only after the character constructor, native moves, countdown, defense/recovery adoption, and reaction-state work are in good shape.

## Grab / throw foundation

Platform fighters eventually need grabs/throws.

If time remains, establish a generic contact/state model and one simple throw.

Do not create a giant throw subsystem that prevents higher-impact tasks from completing.

## Additional stages

Not a priority tonight.

One stage with good combat is more useful than three stages with weak fundamentals.

## Extensive roster balancing

Also not a priority.

Make Robot v3 and a small number of existing characters feel distinct and correct first.

---

# Tests that should prove the architecture

Do not rely only on unit tests of helpers.

Add focused end-to-end tests for the actual ontology.

## Duplicate character

```text
spawn Fretjaw twice
```

Both:

```text
CharacterIdentity == Fretjaw
```

but distinct:

```text
SimId
position
health
brain state
inventory
```

## One character through multiple contexts

Use one character through applicable paths:

```text
NPC
hostile room spawn
encounter
match CPU
match human
```

Assert intrinsic character facts agree.

Only controller/context facts differ.

## Missing explicit character

Explicit CharacterId absent from registry must error, never fall back to legacy archetype.

## Iron Mary

Ordinary Iron Mary spawn must receive Iron Mary's authored body/kit.

No shark-rider fireball kit unless an explicit override says so.

## Puppy Slug in Smash

Forced seat:

```text
can be controlled
keeps crawler body
keeps own repertoire
Attack does nothing when no attack exists
stocks/percent/blast zones still work
```

## Robot v3 cross-mode

Ambition and Smash instances share:

```text
CharacterId
moveset definitions
attack timing
attack geometry
authored damage/base launch
```

while rules differ in:

```text
death policy
knockback growth
DI
stocks
```

## Controller independence

Same character under human vs CPU:

```text
same intrinsic character
different controller
```

## Provocation

Before and after:

```text
same CharacterId
same body
same intrinsic kit
different disposition/controller
```

## Smash round flow

```text
spawn suspended
3
2
1
GO
release

stock loss
respawn protection
eventual vulnerability

last side standing
winner
```

## Ledge

Real Smash fighter can:

```text
grab
hang
ledge jump
getup
roll
getup attack
drop
```

## Air dodge

One use in airtime according to policy; no infinite spam; refresh occurs on defined lifecycle boundary.

## Tech / knockdown

Successful tech and failed-tech knockdown produce distinct authoritative state and outcome.

---

# Regression philosophy

Whenever possible, poison the old path.

Tests should distinguish:

```text
new authority actually won
```

from:

```text
both old and new happened to contain the same value
```

Use deliberately different test values.

Do not add architecture grep-policy infrastructure merely to assert one historical name disappeared.

Direct absence searches at the end are sufficient.

---

# Legacy absence checklist

Before D73 can be called complete, inspect production source and remove the old concepts where they represented this architecture:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
character_archetypes.ron
HostileArchetypeId
PlayableKitSource::HostCode
PreparedKit::HostCode
BUILDABLE_ONLY_CAST
adopt_character_intrinsics
sprite_character_id as gameplay identity
display-name character identity fallback
provoked-archetype reconstruction
```

Also inspect:

```text
ActorTuning
CharacterBrainSpec
CharacterBrain
```

and either delete them or reduce/rename them to narrowly coherent responsibilities.

Do not retain the old whole-body model under a new name.

---

# Naming quality

Leave final production APIs with names that say what they own.

Good vocabulary families:

```text
CharacterId
CharacterIdentity
CharacterDefinition
PreparedCharacterDefinition
CharacterRegistry

BrainProfileId
BrainProfile
AutonomousControllerBinding

CharacterSpawnPlan
SpawnContext
LifecyclePolicy
Relationship/Disposition
```

Exact names may differ based on the final code.

Avoid final names such as:

```text
Legacy*
New*
V2*
Unified*
sprite_character_id
brain_as_character_type
```

Do not use `player` for generic body/controller concepts.

---

# Documentation hygiene

The D73 planning file became very large during investigation.

As the architecture lands, shrink it.

Keep:

```text
final architecture
field ownership
current phase
migration census still needed
remaining blockers
acceptance tests
deletion checklist
```

Remove:

```text
conversation transcripts
dated incident narratives
old hypotheses
verbatim course-correction messages
progress claims superseded by later code
```

Production comments should describe current invariants, not the history of discovering them.

---

# Overnight execution order

Use this order unless the live tree has already completed an item.

## P0 — make the new authority safe

1. Explicit CharacterId missing from prepared registry must be an error.
2. Resolve character-owned autonomous profile refs during character preparation.
3. Complete typed CharacterId through prepared registry/runtime/match seams.
4. Inspect/narrow SpawnContext before adding more callers.
5. Fix current-held-item death ownership.

## P1 — finish one common character body constructor

6. Finish CharacterIdentity.
7. Move/finalize character domain types into the appropriate low crate.
8. Make PreparedCharacterDefinition complete for intrinsic construction.
9. Route authored enemy through character-first construction.
10. Route NPC through the same body constructor.
11. Route PreparedMatch through it immediately after.
12. Route encounter, summon, programmatic paths.

At this milestone there should be **one physical character-construction path**.

## P2 — turn migration into deletion

13. Migrate clean Group-A character/archetype cases.
14. Delete each migrated legacy row as it becomes unnecessary.
15. Extract Group-B shared AI behavior into real BrainProfiles.
16. Classify Group-C generic roles.
17. Migrate provider roster fragments.
18. Delete CharacterRoster/ArchetypeSpec infrastructure.
19. Split/delete ActorTuning.
20. Remove hostile/provocation body reconstruction.
21. Remove rollback dependence on archetype identity.
22. Delete `character_archetypes.ron`.

## P3 — remove protagonist and Smash leveling exceptions

23. Move Robot v3 off HostCode to normal character data.
24. Remove `smash_fighter_kit()` as the universal replacement.
25. Remove universal `fighter_abilities` replacement.
26. Make Smash consume each character's actual body/capabilities/moves.
27. Add Puppy Slug forced-seat regression.

## P4 — next-play Smash feel

28. 3–2–1–GO opening countdown.
29. Wire shields/parry for appropriate fighters.
30. Wire grounded dodge.
31. Implement true air dodge.
32. Enable and tune existing ledge mechanics in Smash.
33. Author landing lag/autocancel on real aerials.
34. Add at least one real strong/Smash attack to Robot v3; preferably F/Up/Down if clean.
35. Add tumble/knockdown/tech/getup state and animation slots.
36. Add stock-respawn protection.
37. Tune hit feedback using existing generic hitlag/VFX/SFX/camera hooks.

## P5 — polish/integration

38. CPU AI chooses from actual character movesets/capabilities.
39. Remove obsolete Smash stand-ins where provider architecture now makes them unnecessary.
40. Rerun PCA as an unconditionally registered character; keep D74 separate if the timing bug remains.
41. Clean architecture docs and stale comments.
42. Measure deletion payoff.

---

# Do not get derailed

Do not stop D73 to investigate unrelated issues such as D74 unless they block the current integration.

If a separate defect is found:

```text
write focused reproduction
record it in planning/queue
continue the overnight campaign
```

Likewise, do not spend hours on new art assets.

Use existing animation fallbacks/slots where necessary and make the mechanical architecture correct.

Do not add more broad architecture guard frameworks merely because this is a large refactor.

The code structure and focused regressions should carry the invariant.

---

# Validation discipline

Use targeted checks as each slice lands.

Important affected packages include at least:

```text
ambition_characters
ambition_combat
ambition_platformer2d_actor_monolith
ambition_demo_smash
```

plus whichever encounter/content/provider crates the live migration touches.

Run focused tests for changed mechanics and content compiler validation.

At meaningful integration milestones run:

```text
cargo check -p ambition_app
```

and relevant runnable/demo checks available in the repository.

Do not repeatedly run giant unrelated suites while iterating.

Do not weaken existing regression tests just to make the migration compile.

Do not use `cargo fmt` as part of this handoff.

---

# End-of-night report

At the end, leave a concise report containing:

## Architecture

* which character-construction paths now use the common constructor;
* which legacy paths remain;
* whether `CharacterRoster` / `ArchetypeSpec` are gone;
* where character, controller, and spawn/session facts now live.

## Deletion payoff

Report:

```text
files deleted
legacy types deleted
legacy rows deleted
approximate lines added/deleted
```

The expected direction is substantial net legacy deletion.

If the implementation adds thousands of adapter lines while leaving all the old authority alive, consider the architecture unfinished.

## Smash

List exactly which of these are now playable:

```text
native per-character moves
Robot v3 shared Ambition/Smash moveset
strong attacks
landing lag/autocancel
shield/parry
ground dodge
air dodge
ledge options
tumble/knockdown
tech
getup options
3-2-1-GO
respawn protection
damage-scaled knockback
DI
```

Do not claim mechanics merely because infrastructure exists; distinguish:

```text
implemented and adopted in Smash
```

from:

```text
engine capability still awaiting content/adoption
```

## Tests

List the targeted tests and integration checks run.

## Remaining blockers

Only include genuine remaining work after this campaign.

Do not preserve stale TODOs that the night's work made obsolete.

---

# Definition of success

A successful morning tree should make these statements true or substantially closer to true:

> A character is authored once and can be instantiated anywhere.

> Who controls a body does not decide what body it is.

> A brain profile does not secretly choose health, movement, moves, art, or mount identity.

> A display name or sprite never determines gameplay character identity.

> Smash seats the same real characters used elsewhere rather than constructing hidden fighter archetypes underneath them.

> Forcing Puppy Slug into Smash gives you Puppy Slug, even if Puppy Slug is a terrible fighter.

> Player Robot v3 uses the same authored combat repertoire in Ambition and Smash.

> Ambition and Smash create different combat feel primarily through rules such as HP versus percent/stocks, knockback growth, DI, lifecycle, and progression—not duplicate move definitions.

> Smash begins with 3–2–1–GO and exposes real platform-fighter defensive, aerial, ledge, reaction, and recovery mechanics.

> The old `character_archetypes.ron` / `CharacterRoster` ontology is gone or visibly on its last shrinking remnants rather than hidden behind a new abstraction.

The goal is not merely to make the current tests green.

The goal is to leave Ambition with a character architecture an engine user would find obvious, and a Smash playtest that finally feels like the engine is becoming a real platform fighter.
