# The character-template architecture, and deleting the enemy-archetype system

**JON'S DECISION AND JON'S BRIEF, 2026-08-10.** This answers queue **D48** —
*"is an enemy a CHARACTER, or an ARCHETYPE wearing one?"* — and answers it
larger than it was asked. The brief below is **his, reproduced verbatim** after
this header; ⛔ do not edit it, summarise it in place, or let a later reading
replace it. Everything above the rule is orientation added by the agent.

---

## The answer in one line

> **A character is a reusable authored template, not a singleton person.**
> `spawn Goblin` three times and `spawn Fretjaw` twice are the same engine
> operation: one `CharacterDefinition`, many runtime actors.

⇒ **the D48 fork is resolved as (a), and then goes past it.** (a) said *"an
enemy IS a character, and the brain is an override"*. Jon's answer keeps that
and adds the part the fork did not reach: there should be **one** character
authority. Not `CharacterCatalog` for half the facts and
`PreparedCharacterDefinition` for the other half, and certainly not
`ArchetypeSpec` for a third set selected through a field called `brain`.

⭐ **the load-bearing correction to the earlier scoping**: D48 framed this as a
content question about 93 authored spawns. It is that, but the spawns are the
*cheap* half. The expensive half is that `character_archetypes.ron` is a second
actor-definition system reached through `EnemySpawn.brain`, and it decides
health, movement, mass, abilities, death traits, mount class, respawn and
hostility — none of which is AI.

## Why the answer is bigger than the question

Jon's own emphasis, which belongs at the top rather than buried in the brief:

> **`character_archetypes.ron` is not merely an enemy tuning file to be
> renamed.** The fact that `respawn`, `attacks_player`, body capabilities, AI
> strategy, and mount semantics all coexist in one row is precisely the evidence
> that it should be decomposed and then deleted.

And the reason to do it now rather than later:

> The current tree is unusually favorable for doing this now because the generic
> ECS runtime underneath it has already been unified. The expensive part is the
> content/construction migration; the simulation core largely does not need to be
> reinvented.

## Baseline and standing state

```text
baseline commit   2fbda948e7461425b96f7bbf701328458201ea3f
```

⚠ **inspect live HEAD before changing anything.** If newer commits exist,
preserve their semantics and adapt.

Facts measured in this repo that the brief depends on, so a resumed session does
not re-derive them:

* **93 `EnemySpawn`s** across all four worlds (`intro` 16, `sandbox` 49,
  `mary_o` 24, `sanic_speedway` 4). **28** already author a `character` id;
  **65** do not, splitting **41** whose NAME is a catalog character and **24**
  whose name is a role (`Skirmisher`, sandbags, skitters, strikers, brutes,
  goblins, `Target`). Recount with a JSON walk of
  `game/ambition_map_assets/**/*.ldtk`; ⛔ `assets/` is not where the worlds live.
* **Two spawn paths, two behaviour authorities.** `NpcSpawn` →
  `features/npcs.rs` → `resolve_initial_brain` reads the catalog row's
  `default_brain`. `EnemySpawn` → `features/ecs/brain_builders.rs` →
  `enemy_default_brain(&ActorConfig)` reads `ArchetypeSpec`, and **never
  consults the catalog row at all**. That asymmetry is the whole bug behind
  Iron Mary's fireballs.
* **D56 has landed** (`2d327f455`): the renderer resolves art through the
  spawn's character id, then art identity, then display name. So authoring
  character ids no longer un-arts the spawns it is meant to fix — the deadlock
  that made D48 unlandable is gone in both directions.

## Phase status — UPDATE THIS AS PHASES LAND

The brief's own sequence, tracked. ⛔ this list is the resumption point after a
compact; a phase is `✔` only when its deletions have happened, not when its new
path works beside the old one.

| # | Phase | State |
|---|---|---|
| 1 | Establish final domain types (`CharacterId`, definition, prepared, registry, controller-profile identity) | ▢ **in progress** — see "phase 1 progress" below |
| 2 | Migrate authored character data out of `character_archetypes.ron` | ▢ |
| 3 | Unify character construction (`PreparedCharacterDefinition` + `CharacterSpawnPlan`) | ▢ |
| 4 | Migrate the 93 authored placements, encounters, summons | ▢ |
| 5 | Controller/provocation simplification; rollback becomes controller-only | ▢ |
| 6 | Remove legacy runtime projections (`ActorTuning`, `CharacterBrainSpec`, `sprite_character_id`) | ▢ |
| 7 | Remove legacy authored infrastructure (`ArchetypeSpec`, `CharacterRoster`, fragments, schema) | ▢ |
| 8 | Rename and document the final architecture | ▢ |

⚠ **the deletion target is the acceptance signal.** ~2,437 lines are obvious
legacy (`ArchetypeSpec` 319, roster/enemies module 1,198,
`character_archetypes.ron` 845, `enemy_roster.rs` 75), with `ActorTuning` (275)
and much of `autonomous_reconcile` (1,045) on top. A result of *+4000 new /
−2400 old* means the old model was wrapped rather than removed.

## ⇥ Phase 1 progress (agent, keep this current)

* ✔ **the field-ownership ledger** — appendix A, all 49 `ArchetypeSpec` fields
  classified against consumers, with seven judgement calls written up.
* ✔ **the multi-instance invariant is PINNED** —
  `one_character_definition_seats_two_independent_fighters`
  (`character_runtime/prepared_match/tests.rs`). A mirror match seats two bodies
  of one definition: same identity, different entity, seat, position and health
  pool. Falsified two ways — damaging both bodies reds the independence half,
  seating a second character reds the identity half.
* ✔ **the uniqueness audit the brief asks for is CLEAN.** Nothing in the
  workspace maps a character id to exactly one entity: the `String → Entity`
  maps that exist are keyed by sim id, encounter id or block name, and every
  `duplicate` guard in `ambition_characters` is about DEFINITIONS
  (`duplicate_character_ids_fail_with_stable_provider_names`,
  `duplicate_display_names_are_rejected_deterministically`), which is the
  correct place for one. ⇒ **instancing is not blocked by an existing
  assumption**, and `MatchSeat`'s own doc already anticipated the collision:
  *"the worn character id collides in a mirror match."*
* ✔ **death traits are authorable on a character** — `CharacterDefinition
  ::combat_capabilities`, carried through preparation and applied by the persona
  derive (`apply_worn_character_gameplay`), which is the ONE writer both a worn
  player and a seated fighter go through. Absence retracts, on the same rule as
  health, mass and the feel marker — see the retraction trap below, which is
  where the first attempt went wrong.
* ⛔⛔ **RETRACT BY RESETTING, NEVER BY REMOVING — cost sixteen integration
  tests, and it is a rule for every field this campaign moves.**
  `CombatCapabilities` is a REQUIRED member of `ActorClusterQueryData`, so
  `try_remove` took each seated fighter out of the actor cluster query entirely
  and it stopped being simulated as an actor. The symptom named nothing about
  components: *"player one swung twelve times in range and the other fighter is
  still on 52/52 HP."* ⇒ **an absent component is a different statement from a
  default one**, and for anything a body's construction owns, only the second is
  legal. ⚠ the reset is also conditional on the PREVIOUS persona having claimed
  the field, because `ActorClusterSeed::into_components` spawns every clustered
  actor with archetype capabilities — an unconditional reset would strip an
  exploding mite the moment anything wore a character on it.
* ✔ **knockback weight is authorable on a character** — `Vitals
  ::knockback_weight` → `PhysicalBaseline` → the seed's `CombatTuning.weight`
  at construction and the live component at a re-wear. It could be stated only
  on a roster ARCHETYPE before, so all three Smash fighters seat through
  `combatant` and weighed the same; they now spread 0.85 / 1.0 / 1.35 around the
  reference body, which is what makes D75's growth term mean something different
  per fighter.
  ⛔ **the first version of that test could not fail.** Its control asserted the
  unauthored character keeps its archetype's weight — but the fixture's
  archetype authored none, so it defaulted to the reference `1.0`, which is
  exactly what an unconditional `unwrap_or(1.0)` writes. Poisoning it passed.
  The fixture now authors a `1.4` archetype, which separates *"kept"* from
  *"overwritten with the ambient default"*, and the poison reds.
* ⭐ **the field that made this worth doing first**: `CombatCapabilities` had
  exactly ONE producer in the workspace — `ArchetypeSpecExt` — so a mite that
  splits on death could say so as an archetype and no registered character could
  say it at all. That is the incompleteness the brief describes, in its smallest
  reproducible form.
* ⚠ **`a_definition_carries_no_controller_binding` is where the brief's ruling
  lands.** That test destructures `CharacterDefinition` exhaustively and says
  *"if `default_brain` is ever added, this stops compiling and the reviewer has
  to justify it against §4.7."* Jon has now justified it — a definition MAY name
  a default autonomous profile. ⇒ when phase 1 adds that field, rewrite this
  test's prose rather than deleting it; it is the structural guard that keeps
  the CURRENT controller off the definition, which is still the rule.
* ⭐ **the catalog fold is FOUR FIELDS, not a pervasive dependency** — measured
  in `definition.rs`'s finalization. `PreparedCharacterDefinition` consults the
  catalog for exactly `max_health`, `motion_model`, `movement_tuning`, and the
  kit (`action_set` / `moveset`). Everything else already carries through from
  the definition. ⇒ the brief's *"still finalized by consulting the catalog"* is
  a much smaller cut than it reads, and it closes the moment those four are
  authored on definitions — which is phase 2's output, not extra work.
* ⛔ **THE PHASE-2 ORDERING CONSTRAINT, in the code's own words.**
  `PLAYABLE_ROSTER` cannot stop gating buildability until definitions carry the
  archetype's intrinsic facts. `character_catalog.rs` records the measurement
  from when someone tried: registering every catalog row flipped ~100
  exploration NPCs off their archetype-built vitals onto defaults, caught by
  `a_player_death_reset_survives_the_rollback_window`, *"because the catalog row
  has no mass or health to fold back in — those come from the ARCHETYPE — so the
  blanket rule cannot be made behaviour-neutral, only narrower."* ⇒ **removing
  the workaround is the LAST step of phase 2, not the first**, and it becomes
  free once the facts have moved.
* ▢ next: whether `WornCharacter` becomes the `CharacterIdentity` the brief
  names. It is already a component in `ambition_characters` holding a character
  id, already carried by non-player bodies (`sanic/badnik.rs`), and already the
  authority the renderer binds from — so the work is extending it to every spawn
  path and retiring `ActorConfig::sprite_character_id`, not inventing a type.

## What this decision also settles elsewhere

* **Jon's Iron Mary observation** — *"the instance of iron marry doesn't use her
  swordgun, she shoots fireballs"* — is this campaign's phase 4 acceptance test.
* **The goblin-lab heavy casting call** (`awaiting-maintainer-decision.md`) is
  reshaped, not answered: under this model a `large_brute` mob names a real
  reusable character definition, and borrowing another character's art is an
  explicit presentation override rather than a lie about identity.
* **`PLAYABLE_ROSTER` stops deciding which characters the engine can build.** The
  earlier blanket-registration failure (~100 exploration NPCs losing vitals) is
  re-read as *evidence the migration is incomplete*, not as a reason to keep the
  workaround.

---

# ⇥ JON'S BRIEF, VERBATIM (2026-08-10)

> Ok, after much deliberating with another agent, I have an answer to the
> question that identifies what the real - industry standard - elegant answer
> is. You should record this as a durable implementation plan in docs/planning
> in case we need to compact while implementing it. Here is the result of that
> conversation:

I dug through the construction, catalog, match, NPC, enemy, encounter, summon, provocation, rollback, mount, provider, and content-compiler paths. The direction is strongly confirmed.

Two refinements emerged from the code inspection:

* We should **not migrate `ArchetypeSpec` into `CharacterDefinition` wholesale**. It is a god-object containing three different authorities: intrinsic character/body facts, autonomous-controller policy, and placement/session policy. The refactor should split those facts and then delete the archetype system.
* We should go farther than D48. The clean endpoint is **one character-definition authority**, not `CharacterCatalog` for half the facts plus `PreparedCharacterDefinition` for another half. The current `PreparedCharacterDefinition` infrastructure is already close to the right endpoint, but it is still finalized by consulting the catalog and has a `HostCode` escape hatch.

The deletion payoff is credible. Just four obvious legacy artifacts are about **2,437 lines** (`ArchetypeSpec`, `CharacterRoster`, `character_archetypes.ron`, `enemy_roster.rs`). `ActorTuning` is another 275 lines, and the 1,045-line autonomous-reconcile module contains a large amount of provocation/archetype reconstruction that should disappear. Including roster fragments, schemas, compatibility branches, tests, and catalog-vs-definition folding, I would expect **several thousand net lines of legacy machinery to vanish**, even after adding the cleaner definitions.

Here is the implementation prompt I would give a fresh agent.

# Complete the Ambition character-template architecture and delete the legacy enemy-archetype system

You are implementing a substantial architecture refactor in Ambition.

Current repository baseline for this handoff:

```text
2fbda948e7461425b96f7bbf701328458201ea3f
```

Inspect current HEAD before changing anything; if newer commits exist, preserve their final semantics and adapt this plan to the live tree.

This is a **pre-release engine with zero external dependents**. Do not preserve legacy APIs, compatibility shims, duplicate registries, deprecated field names, or old authoring formats merely to reduce migration work.

`AGENTS.md` is explicit:

* unified actors;
* one body, one path;
* player/enemy/NPC/boss are data/controller distinctions, not separate actor ontologies;
* make the richer/general path universal and delete the alternatives;
* optimize for the clean engine architecture rather than preserving accidental current behavior.

The purpose of this task is to finish that migration for character identity and construction.

---

# Product decision

A **character is a reusable authored template**, not a singleton person.

These are the same engine operation:

```text
spawn Goblin
spawn Goblin
spawn Goblin

spawn Fretjaw
spawn Fretjaw
```

Each spawn receives the same authored character definition but creates a distinct runtime actor.

Conceptually:

```text
CharacterId::Fretjaw
    → reusable authored definition

SimId(100)
    → one runtime Fretjaw instance

SimId(205)
    → another runtime Fretjaw instance
```

The fact that Fretjaw is fictionally a particular named person does not make the character definition singleton-valued.

If the narrative needs one particular runtime Fretjaw to be “the canonical Fretjaw for this story role,” that is **instance/placement/narrative identity**, not `CharacterId`.

Likewise:

```text
Goblin
Fretjaw
Iron Mary
Puppy Slug
Exploding Mite
```

are all the same engine concept: reusable `CharacterDefinition`s.

A generic character and a named character differ in content semantics, not construction architecture.

---

# Target mental model

The professional endpoint is approximately:

```text
                  authored once
                       │
               CharacterDefinition
         ┌─────────────┼─────────────┐
         │             │             │
       body          capabilities   presentation
       vitals        actions        voice
       movement      moves          hurtboxes
       traits        equipment      metadata
         │             │             │
         └─────────────┴─────────────┘
                       │
             prepare / validate once
                       │
                       ▼
          PreparedCharacterDefinition
                       │
           ┌───────────┴───────────┐
           │                       │
   ControllerBinding           SpawnContext
 human / autonomous /       faction/disposition
 replay / policy            respawn/encounter/etc.
           │                       │
   autonomous profile?             │
           └───────────┬───────────┘
                       ▼
                 spawned actor
                CharacterId = X
                SimId = instance
```

A character definition may name a **default autonomous-controller profile** for authoring ergonomics.

That does **not** mean the controller is intrinsic identity.

The distinction is:

```text
CharacterDefinition
    may say:
    default_autonomous_profile = goblin_melee_ai

runtime actor
    may instead be driven by:
    Human
    another autonomous profile
    Replay
    RL/external policy
```

Possessing a Goblin changes who drives the Goblin. It does not change what a Goblin is.

This is the same broad model as Prefab/Blueprint/PackedScene-style general-purpose engines.

---

# Current architecture: confirmed problems

I inspected the relevant current source deeply.

There are currently **two competing actor-definition systems**.

## Newer system

`character_runtime::CharacterDefinition` and `PreparedCharacterDefinition` already own or resolve substantial character identity:

```text
presentation
body
hurtboxes
vitals
action set
moveset
motion model
movement tuning
voice
```

`PreparedCharacterDefinition` is explicitly intended to be flat, immutable, and complete.

This is the architecture to keep and finish.

## Older system

`character_archetypes.ron` → `ArchetypeSpec` → `CharacterRoster::spec_for_brain()` currently decides far more than AI.

A field called `brain` selects an archetype containing facts such as:

```text
movement physics
maximum health
run speed
mass
aerialness
surface walking
cling behavior

blink
fly
shield
dash

melee action
ranged action
held item
signature move

death explosion/division/crash traits
training-dummy behavior
knockback weight
death policy

mount class
pilot permissions
mount-death effect

projectile visual identity

brain template
patrol/chase effort
aggro radius
attack range
wall-turn behavior
Smash tactical policy
fighter level

hostility
contact damage

respawn
```

That means the current model effectively says:

```text
EnemySpawn.brain
    → what physical creature this is
    → what it can do
    → how healthy it is
    → how it moves
    → how it dies
    → how it mounts
    → how its AI thinks

EnemySpawn.character_id
    → costume/art
```

That ontology is the thing to delete.

Do not rename `ArchetypeSpec` and keep it.

Do not copy all its fields into another god-object.

Split its facts according to ownership and eliminate the competing actor definition.

---

# A particularly important confirmed defect: provocation currently changes the body

The current peaceful-NPC → hostile transition demonstrates how badly the two authorities are entangled.

A peaceful catalog NPC currently starts from a reduced generic configuration, including roughly:

```text
1 HP
peaceful/reduced tuning
reduced combat configuration
```

When provoked, code uses string heuristics over character id/name/dialogue to choose a hostile archetype.

Examples currently map names/id patterns to things such as:

```text
combatant
pirate_heavy
pirate_raider
cellular_automaton_fighter
...
```

The provocation path then overwrites:

```text
health-related configuration
movement tuning
gravity/aerial state
combat capabilities
brain configuration
action set
live brain
sprite/config read models
```

In other words:

> becoming angry currently reconstructs the actor as another mechanical creature.

Delete that architecture.

A peaceful Fretjaw is already Fretjaw.

A hostile Fretjaw is still Fretjaw.

Provocation should primarily change:

```text
disposition/aggression
+
autonomous controller selection/state
```

It should **not** change Fretjaw's intrinsic body, vitals, abilities, movement identity, or action repertoire.

This should allow substantial deletion from:

```text
features/ecs/actors/conversion.rs
features/ecs/autonomous_reconcile.rs
character_catalog/binding.rs
```

including the hostile-archetype reconstruction machinery.

---

# A second confirmed signal: blanket character registration failed because the migration is incomplete

The repository previously tried registering the whole character catalog into `PreparedCharacterRegistry`.

That caused roughly 100 exploration NPCs to lose their expected vitals/behavior.

The current comments interpret this as a reason not to register the whole cast.

For this refactor, interpret it correctly:

> It proves `PreparedCharacterDefinition` is not yet the complete actor definition because health/body/kit facts still come from the archetype path.

The desired invariant is:

```text
every declared character
    → one complete PreparedCharacterDefinition
    → constructible through every ordinary spawn path
```

The current workaround where `PLAYABLE_ROSTER` effectively determines which catalog rows become buildable definitions should disappear.

`PLAYABLE_ROSTER` may remain a UI/content decision about which characters appear in a selection screen.

It must **not** define which characters the engine is capable of constructing.

---

# One authority for character definitions

The current split between:

```text
CharacterCatalogEntry
CharacterDefinition
ArchetypeSpec
```

must end.

Today `CharacterCatalogEntry` itself already owns substantial gameplay facts:

```text
display/presentation
default brain
default action set
playable-kit source
motion model / momentum
abilities
movement tuning
max health
voice/barks
etc.
```

`CharacterDefinition` owns another overlapping set.

`ArchetypeSpec` owns yet another.

The final runtime must not consult multiple registries to answer what one character is.

## Desired source/preparation model

Use a clear pipeline such as:

```text
authored CharacterDefinition
        │
        ├── may reference reusable named profiles/documents
        │
        ▼
prepare_character(...)
        │
        ▼
PreparedCharacterDefinition
        │
        ▼
one CharacterRegistry / PreparedCharacterRegistry
```

The exact serialized source type may be called something like:

```text
CharacterDefinition
CharacterDefinitionDoc
CharacterSource
```

depending on what best fits the current compiler.

The semantic requirement is more important than the spelling:

> one authored character definition becomes one complete prepared character definition.

Do not retain a parallel gameplay `CharacterCatalog` that independently answers:

```text
health
abilities
movement
default action kit
default autonomous behavior
```

If a catalog-like projection remains useful for tooling/presentation, it must be **derived from the same character source/prepared definition** and must not be a second gameplay authority.

Prefer moving `CharacterDefinition`, `PreparedCharacterDefinition`, their stable ID, and their registry into the lowest natural character-domain crate—likely `ambition_characters` if the live crate graph permits it.

Use:

```text
.agent/index/crates/graph-resolved.json
```

from the live repository before changing dependency direction.

Do not create a new crate merely for aesthetic symmetry if the existing `ambition_characters` ownership is clean.

---

# Character identity needs a real name

The current runtime has concepts such as:

```text
WornCharacter
ActorConfig.sprite_character_id
ActorConfig.sprite_override_npc_name
```

These preserve the history where character identity meant primarily “which art is this body wearing?”

That is no longer the model.

Introduce or adopt a sensible stable template identity, for example:

```text
CharacterId
```

and a runtime component conceptually like:

```text
CharacterIdentity(CharacterId)
```

Use the existing terminology if an equivalent type already exists and genuinely has these semantics.

Requirements:

```text
CharacterId
    = reusable character-definition identity

SimId / FeatureId
    = one runtime instance

display_name
    = presentation only
```

A body should not need separate:

```text
sprite_character_id
```

once the body already knows which character definition it instantiates.

Presentation, voice, body metadata, combat geometry, etc. should derive from `CharacterIdentity`.

Remove display-name matching as an authoritative spawn/identity fallback.

Display-name lookup may remain a tooling/search convenience if useful.

---

# Do not make `CharacterDefinition` a new god-object

Move each `ArchetypeSpec` fact to the authority it actually belongs to.

Use this classification as the starting point, then verify every field's consumers before migrating it.

## Character/body intrinsic facts

These belong in the character definition or in reusable profiles referenced by the character definition and flattened during preparation:

```text
body geometry / sprite-authored body
standing/body physical dimensions
mass
movement model
movement tuning / run capability
aerial body capability
surface/crawler capability
cling behavior where intrinsic

maximum health / intrinsic vitals
knockback weight if genuinely body-owned

intrinsic abilities:
    blink
    fly
    shield
    dash
    etc.

action set
moveset/signature moves
melee/ranged capabilities
default/intrinsic equipment or loadout
contact-damage trait where genuinely body-owned

death traits:
    explode-on-death
    divide-on-death
    charge-crash behavior
    intrinsic immortality/training-dummy trait
    only where these really are properties of the character

mount capability
pilot capability
intrinsic mount-death behavior

presentation traits:
    projectile visual where it cannot instead belong directly to the projectile/action
    dream visual metadata
```

Prefer existing generic components and typed definitions over creating another monolithic “character tuning” bag.

For example, a ranged projectile's visual should ideally be carried by the ranged action/projectile specification rather than redundantly copied onto a top-level character when that is the real owner.

## Autonomous-controller policy

These do **not** define the character body.

Move them into a narrow reusable autonomous-controller / brain profile:

```text
brain template
patrol/chase effort
aggro radius
preferred attack distance
wall-turn behavior
Smash tactical policy
Smash heavy/duelist/dash-to-close preferences
fighter AI level
provocation controller policy
```

There is already substantial reusable machinery:

```text
BrainPreset
BrainPresetId
Brain
BrainBinding
ControllerBinding
```

Prefer evolving that vocabulary to inventing a parallel system.

Choose clean names.

For example, depending on the resulting code:

```text
BrainProfile
BrainProfileId
AutonomousControllerProfile
AutonomousControllerBinding
```

may be clearer than `CharacterBrainSpec`.

Do not retain the word `archetype` for autonomous-controller policy.

## Spawn/session/ruleset facts

These should not move into the character definition:

```text
respawn policy
initial faction/team
initial hostility/disposition
encounter membership
ruleset-owned death behavior
story/placement role
current controller
```

A Goblin can be:

```text
hostile room enemy
friendly NPC
human-controlled body
Smash fighter
summoned minion
training instance
```

without ceasing to be Goblin.

The existing code has already discovered this distinction accidentally:

`ActorTuning::adopting_archetype()` has to preserve `respawn` because blindly applying an archetype's respawn policy to a named NPC caused a real bug.

That is direct evidence that `respawn` belongs to placement/session lifetime, not actor identity.

Similarly, do not preserve:

```text
attacks_player
```

as a character-template fact.

Ambition has no privileged “player” engine identity.

Use factions, disposition, aggression, ruleset, and controller state.

---

# Autonomous default versus current controller

Preserve this important distinction:

```text
character identity
    ≠
current controller
```

But improve the authoring ergonomics.

A character definition may have:

```text
default_autonomous_profile: Option<BrainProfileId>
```

or an equivalent concept.

Then:

```text
spawn Goblin with no controller override
→ Goblin's normal autonomous controller

spawn Goblin with Human(...)
→ human drives the same Goblin body

spawn Goblin with brain_override = guard
→ guard policy drives the same Goblin body
```

This means:

> character owns its normal default behavior

without saying:

> the AI brain is intrinsic body identity.

Update the existing design documentation that currently states too absolutely that a character definition cannot carry a default brain.

The correct invariant is:

```text
current controller is session/runtime authority

character definition may provide
the default autonomous-controller choice
when no stronger context overrides it
```

---

# Remove `PlayableKitSource::HostCode`

The player robot's `HostCode` branch is another protagonist-era exception.

Current architecture already has the correct generic layers:

```text
character's intrinsic/base abilities
+
runtime body-owned progression/equipment/grants
+
session restrictions
=
effective abilities
```

Use those.

Every character definition—including `player_robot_v3`—should contain its intrinsic baseline:

```text
abilities
action set
moveset
movement/body identity
```

Progression, equipment, debug grants, possession, and session rules then modify the runtime body through the generic capability/inventory system.

Delete:

```text
PlayableKitSource
PreparedKit::HostCode
host-code fallback branches
special protagonist-kit finalization logic
```

Do not replace them with a differently named protagonist exception.

The final `PreparedCharacterDefinition` should contain one resolved intrinsic kit representation.

---

# Lower every spawn path through one character construction plan

Different authoring surfaces may remain different because they express different contextual concerns.

Do **not** create one enormous authoring struct containing every field any subsystem might ever need.

But they must all lower into the same actor-construction contract.

Conceptually:

```text
NpcSpawn -------------------\
HostileCharacterSpawn -------\
EncounterMobSpec -------------\
SummonSpec --------------------> CharacterSpawnPlan
MatchParticipant -------------/
Programmatic spawn ----------/
```

A conceptual `CharacterSpawnPlan` contains only real shared construction facts, such as:

```text
character_id
controller selection / autonomous-profile override
spawn transform
disposition/faction
lifecycle/respawn context
other genuinely contextual grants
```

Then one body-construction implementation:

```text
PreparedCharacterDefinition
+
CharacterSpawnPlan
→
generic actor ECS components
```

No alternate peaceful-NPC body builder.

No enemy archetype body builder.

No match-specific physical body reconstruction.

One body, one path.

---

# Refactor `EnemySpawnSpec`

Its current documentation explicitly says:

```text
brain        = what it DOES
character_id = what it LOOKS LIKE
```

Delete those semantics.

The normal authored form should be approximately:

```text
HostileCharacterSpawnSpec {
    character_id: CharacterId,
    brain_override: Option<BrainProfileId>,
    ...
}
```

The exact type name may remain `EnemySpawnSpec` if it clearly means the **placement role** “starts as a hostile enemy” rather than a distinct actor ontology.

But:

```text
character_id
```

must mean:

> which reusable character definition this actor instantiates.

Make it required for normal shipped visible actors after migrating the content.

Do not keep the display-name fallback as an authoritative compatibility path.

Rename:

```text
brain
```

to something that truthfully expresses its new semantics, likely:

```text
brain_override
controller_profile_override
```

depending on final vocabulary.

The ordinary case should need no redundant behavior field:

```text
character_id: "iron_mary"
brain_override: None
```

means:

> instantiate Iron Mary and use her normal autonomous behavior.

An unusual scene can say explicitly:

```text
character_id: "iron_mary"
brain_override: Some("berserk")
```

---

# Refactor NPC spawning

The current NPC path is already closer to the desired controller model:

```text
explicit brain override
→ otherwise character default brain
```

Keep that idea.

But NPC physical construction must use the exact same complete `PreparedCharacterDefinition` as an enemy, match fighter, summon, or possessed body.

Delete the current peaceful generic body reconstruction.

A peaceful NPC is:

```text
CharacterDefinition
+
peaceful disposition/context
+
an autonomous controller that does not initiate combat
```

not a body with arbitrarily reduced health and capability.

NPC and enemy editor schemas may remain separate conveniences.

They must lower into the same character spawn plan.

---

# Refactor provocation

Provocation is a major deletion target.

After the migration:

```text
before:
Fretjaw body + peaceful disposition/controller

provoke

after:
same Fretjaw body
same intrinsic vitals
same movement/body capabilities
same action repertoire
hostile disposition/aggression
combat autonomous controller
```

No archetype reconstruction.

No health rewrite.

No kit rewrite.

No gravity/body rewrite.

No name/dialogue string heuristics.

Delete concepts such as:

```text
HostileArchetypeId
AutonomousSource::Provoked { archetype }
project_provoked_archetype
hostile_spec_for_actor
hostile_brain_id_for_actor
ActorTuning::adopting_archetype
```

or their current equivalents.

If a character has a special controller profile when provoked, author that fact explicitly as controller-policy data.

Do not infer it from display names, dialogue ids, string prefixes, or art identity.

---

# Simplify rollback accordingly

Current rollback state contains legacy reconstruction facts because a restored actor may need to recreate the old archetype-derived body configuration.

That should become unnecessary.

`BrainBinding` currently snapshots variants including:

```text
CatalogDefault
CatalogPreset
Provoked { archetype }
Boss { ... }
```

After the refactor, a normal autonomous body should need only enough rollback-owned state to reconstruct **controller selection**, not physical character identity.

Conceptually:

```text
AutonomousControllerBinding {
    character_default
    or explicit profile override
    or boss-specific external ownership where genuinely necessary
}
```

`CharacterIdentity` itself is rollback-owned if it is runtime-mutable through transformation/re-wear.

Its snapshot value is merely the stable `CharacterId`.

Runtime body state restores through the ordinary rollback components.

Do not restore health/movement/capability state by rerunning a character/archetype constructor during rollback unless that state is explicitly defined as derived.

Remove roster/archetype access from rollback reconciliation.

Keep boss-specific logic separate where boss behavior architecture genuinely requires it; do not broaden this task into an unrelated boss rewrite.

When renaming rollback components/types, update the rollback schema deliberately rather than leaving compatibility aliases.

This is pre-release.

---

# Delete `CharacterBrain`

The placement enum:

```text
CharacterBrain::Passive
CharacterBrain::Patrol { ... }
CharacterBrain::Guard { ... }
CharacterBrain::Custom(String)
```

is another conflation.

`Custom(String)` currently means:

> look up an entire creature archetype.

Patrol/guard also mix controller selection with placement context.

Delete this representation.

Use:

```text
BrainProfileId / AutonomousControllerProfileId
```

for controller policy.

Keep contextual information such as:

```text
spawn anchor
patrol path
patrol radius
guard leash
```

in an explicit controller build context / placement context.

The current `AuthoredBrainContext` is already a useful model.

---

# Refactor encounters

Current `EncounterMobSpec` repeats the old ontology explicitly:

```text
kind      = what it DOES
character = what it LOOKS LIKE
```

Delete it.

Target something like:

```text
EncounterMobSpec {
    character_id: CharacterId,
    brain_override: Option<BrainProfileId>,
    spawn: ...,
    delay: ...,
    size_override: Option<...>, // only if genuinely needed
}
```

The character definition determines normal body geometry.

A wave-specific body-size override should be visibly an override, not a required parallel identity.

The generated/minted encounter id remains the **runtime body identity**.

Do not use it as character identity.

---

# Refactor summons and programmatic spawns

Current constructs such as:

```text
SummonedMinionParams.archetype_id
SpawnActorKind::Enemy { brain }
```

must disappear.

Programmatic character spawning should be character-first.

Prefer a generic request shaped around:

```text
character_id
controller/autonomous override
faction/disposition
spawn context
```

Do not preserve a separate runtime `Enemy` construction family if enemy-ness is only contextual data.

`SpawnActorKind::Boss` may remain temporarily if the boss encounter architecture has genuinely different construction requirements.

Do not force unrelated boss work into this campaign merely for visual uniformity.

---

# Refactor matches

The match architecture already demonstrates the right separation:

```text
MatchParticipant {
    character,
    controller,
    team,
    ...
}
```

Keep that.

But `PreparedMatch` currently constructs a hybrid:

```text
PreparedCharacterDefinition
+
old CharacterRoster/ArchetypeSpec
```

and patches character physical facts afterward.

Delete the hybrid.

A match fighter's physical/intrinsic actor state must come directly from the same `PreparedCharacterDefinition` used everywhere else.

Then attach:

```text
ControllerBinding
ruleset state
team
match-specific action override if intentionally authored
```

afterward.

A human Fretjaw and CPU Fretjaw should have the same underlying Fretjaw body definition.

The controller alone differs.

---

# Remove the old enemy-archetype database

Once every consumer is migrated, delete the system completely.

Expected deletion targets include, as applicable in current HEAD:

```text
crates/ambition_combat/src/archetype_spec.rs

game/ambition_content/assets/data/character_archetypes.ron
game/ambition_content/src/enemy_roster.rs

CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
CharacterRosterAssemblyError
spec_for_brain
movement-inheritance assembly specific to archetypes

ArchetypeSpec
ArchetypeSpecExt

old character_archetypes content schema
ARCHETYPES_SOURCE_PATH
pack.ron character_archetypes entry

provider-specific CharacterRosterFragment installers
```

Current provider fragments include areas such as:

```text
Mary-O AI Slop / Snake
Sanic Badnik
Smash
Ambition content
```

Migrate their real facts into:

```text
character definitions
brain/controller profiles
spawn context
```

and delete the fragment mechanism.

Do not leave an empty legacy registry for tests.

Do not leave a `LegacyCharacterRoster`.

Do not provide compatibility re-exports.

---

# Split or delete `ActorTuning`

`ActorTuning` currently remains another projection of the old god-profile and mixes:

```text
body movement
health
AI distances
contact behavior
hostility
respawn
death policy
aerialness
sandbag role
visual identifiers
```

Do not preserve this shape.

Move consumers to the actual authoritative generic components.

Examples:

```text
movement
→ character movement/body components

health
→ BodyHealth/vitals

body capabilities
→ AbilityBase / BodyAbilities / appropriate body traits

AI distances/efforts
→ Brain/BrainProfile configuration

respawn
→ spawn/lifecycle component

faction/hostility
→ disposition/aggression/faction

death policy
→ ruleset/body-health authority as appropriate

visual identity
→ CharacterIdentity / action/projectile presentation data
```

If a small coherent actor-runtime tuning component remains after the migration, name it for exactly what it owns.

Do not retain `ActorTuning` as a dumping ground merely because many systems already read it.

---

# Split or delete `CharacterBrainSpec`

This type contains actual AI policy mixed with duplicated body capability flags.

Examples such as:

```text
smash_heavy
smash_duelist
smash_dash_to_close
fighter_level
```

may legitimately belong to a brain/controller profile.

But flags such as:

```text
smash_can_blink
smash_can_fly
smash_can_shield
```

duplicate capabilities already represented on the body.

A brain should decide using the capabilities of the body it currently controls.

Do not author capability twice as:

```text
brain may attempt blink
body may enforce blink
```

when the controller can inspect the body's actual capability state.

Keep decision-policy facts in the controller profile.

Use body capability state as the enforce/availability authority.

---

# Mounts reinforce this model

ADR 0020 already says:

```text
mount and rider are ordinary actors
mountability/pilotability are body capabilities
controller can be rider/player/AI
```

Honor that.

Move:

```text
mount_class
pilotable_mount_classes
intrinsic mount-death behavior
```

into character/body capability definition where those facts are intrinsic.

The actual mount/rider pairing is a runtime relation/placement fact.

It must not depend on which AI brain happens to drive either body.

---

# Character source-data migration

There are currently 93 authored `EnemySpawn`s across the world files.

Measured population:

```text
93 total

28 already author character_id

65 currently do not
    41 have names which correspond to catalog characters
    24 have role/generic names which do not
```

Migrate them explicitly.

## The 41 named characters

Give them stable `CharacterId`s.

Do not continue inferring identity from display name.

Review actual behavior rather than mechanically assuming the current catalog defaults are correct.

Important examples:

### Iron Mary

Her character row already declares approximately:

```text
default brain = melee brute
action set = brute lunge
```

The sky placement currently gets shark-rider behavior because the enemy path ignores the character kit.

After migration, spawning Iron Mary should get Iron Mary's definition unless the placement explicitly overrides it.

### Burning Flying Shark

Its current catalog default is not necessarily the combat behavior supplied by its old archetype.

This is exactly the kind of row which must be **semantically migrated**, not blindly trusted.

Decide what the character's canonical normal autonomous profile is.

If a particular placement intentionally behaves differently, author an explicit override.

### Exploding / Dividing Mites

Their intrinsic death behavior currently lives in archetype rows.

Move those traits into their character definitions.

### Giant GNU

Current comments explicitly acknowledge that HP/rideability or related physical facts live in the old archetype file.

Move those facts into the character/body definition.

### Fretjaw and peaceful named characters

Do not encode peacefulness by stripping combat capabilities from the body.

Fretjaw's definition should describe what Fretjaw can do.

An ambient/NPC placement chooses peaceful disposition/autonomous policy.

The same Fretjaw definition can later be human-controlled, provoked, or spawned into combat without reconstructing a different creature.

## The remaining 24 generic/role names

Classify each one.

If it represents a real reusable visible thing, create a proper character definition:

```text
generic goblin
heavy goblin
training target
generic skirmisher
etc.
```

if that is what the content genuinely means.

If it is a fixture/debug/structural actor that does not deserve normal character authoring, give it an explicit low-level fixture/dev construction API.

Do not pollute the shipped character registry with fake definitions solely to satisfy uniformity.

If an unfinished character temporarily borrows another character's art, represent that as an explicit presentation reference/override—not by lying about its character identity.

---

# One definition may be instantiated arbitrarily many times

Add regression coverage proving this explicitly.

Example:

```text
spawn Fretjaw twice
```

Verify:

```text
both:
    CharacterIdentity == Fretjaw

different:
    SimId / FeatureId
    health state
    position
    brain state
    inventory
```

No uniqueness guard should reject it.

Character identity is template identity, not runtime entity identity.

Audit any existing lookup that assumes:

```text
CharacterId → exactly one Entity
```

and replace it with proper runtime/placement/story identity.

---

# Game-specific story metadata must not pollute the generic engine definition

The current character catalog also contains Ambition-specific authoring metadata such as Hall/gallery concerns.

While consolidating character authority, distinguish:

```text
generic reusable character definition
```

from:

```text
Ambition-game-specific editorial/gallery metadata
```

Do not put concepts such as:

```text
MainHall
Basement
```

into the generic engine character model merely because they currently share a RON row.

Game-specific metadata may remain in the game content layer keyed by `CharacterId`, provided it does **not** become a parallel gameplay-construction authority.

Presentation fields that are genuinely generic—sheet, portrait, voice, display name—can remain part of the reusable character definition.

---

# Sensible naming is part of the task

Do not leave terminology which preserves the old ontology.

At the end, there should be no concept where “archetype” means:

> a hidden second actor definition selected through a brain field.

Prefer vocabulary in these semantic families:

```text
CharacterId
CharacterDefinition
PreparedCharacterDefinition
CharacterIdentity
CharacterRegistry / PreparedCharacterRegistry

BrainProfile / AutonomousControllerProfile
BrainProfileId
AutonomousControllerBinding

ControllerBinding

CharacterSpawnPlan
SpawnContext
```

These names are guidance, not mandatory exact spellings.

Choose names after inspecting live usages.

Requirements:

* the name says what the object actually owns;
* do not use `brain` to mean body identity;
* do not use `sprite_*` to mean character identity;
* do not use `player` terminology for generic actor concepts;
* do not leave `catalog` in a name if it has become the authoritative character-definition registry;
* do not leave `archetype` in a type which now means AI policy;
* do not create “V2”, “New”, “Unified”, or “Legacy” names as the final API.

Rename production comments/documentation to describe the final invariants, not the migration history.

---

# Content compiler / authored-source cleanup

The old `character_archetypes` schema must disappear when its data has been migrated.

Update the content pack accordingly.

Delete:

```text
character_archetypes.ron
its schema registration
its pack entry
its loader/fragment plumbing
its validation path
```

Character definitions and autonomous-controller profiles must have proper content schemas and `deny_unknown_fields` where appropriate.

Do not duplicate validation through separate runtime parsers.

Preserve the repository's good rule:

```text
authored content
→ compiler/lowering
→ runtime authority
```

not:

```text
same RON parsed independently by compiler and game
```

---

# Provider architecture

Providers currently publish character-roster fragments.

Delete that interface.

A provider should publish things like:

```text
CharacterDefinition(s)
BrainProfile(s)
game-specific metadata
```

through the same generic registration/preparation seams.

A new provider character should require:

```text
author character
choose/default controller profile
register it
```

and **zero core actor-engine edits**.

That is an acceptance criterion.

---

# Preserve the good runtime engine

Do not rewrite systems which already consume generic actor components correctly.

The current architecture is favorable because most per-tick systems already operate on things such as:

```text
BodyKinematics
BodyHealth
BodyAbilities
AbilityBase
ActionSet
Moveset
Brain
CombatCapabilities
ActorDisposition
ActorAggression
ActorFaction
mount relations
```

The main refactor is:

```text
OLD authored/construction authority
        ↓
generic runtime components
```

to:

```text
CharacterDefinition
+ Controller/Profile
+ SpawnContext
        ↓
generic runtime components
```

Preserve the generic bottom half.

Delete the alternate constructors feeding it.

---

# Recommended implementation sequence

This is an end-to-end task. Use intermediate commits if useful, but do not stop with both architectures still alive.

## Phase 1 — establish final domain types

Create/move the final:

```text
CharacterId
CharacterDefinition
PreparedCharacterDefinition
character registry
autonomous controller profile identity
```

into their proper domain owner.

Extend the character source/prepared definition so it can express every intrinsic character fact currently required from the old archetype path.

Add default autonomous-controller-profile semantics.

Remove the conceptual need for catalog fallback during runtime body construction.

## Phase 2 — migrate authored character data

Move intrinsic facts out of:

```text
character_archetypes.ron
```

into character definitions.

Move autonomous-policy facts into brain/controller profiles.

Move placement/session facts out to their actual contexts.

Do this for Ambition plus provider-local demo roster fragments.

Make every normal declared character produce a complete prepared definition.

Remove the `PLAYABLE_ROSTER`-as-buildable-cast workaround.

## Phase 3 — unify character construction

Create one generic character-body construction path from:

```text
PreparedCharacterDefinition + CharacterSpawnPlan
```

Route NPC, enemy, match, encounter, summon, and programmatic character construction through it.

Delete the separate physical body/kit construction paths.

## Phase 4 — migrate authored placements

Migrate all 93 `EnemySpawn`s to explicit stable character identity.

Migrate encounter wave data.

Migrate summons/programmatic requests.

Remove display-name identity fallbacks.

## Phase 5 — simplify controller/provocation architecture

Replace old `CharacterBrain`/archetype selection with autonomous profile selection.

Make provocation a disposition/controller transition.

Delete hostile-archetype reconstruction.

Update rollback reconciliation to controller-only restoration.

## Phase 6 — remove legacy runtime projections

Delete/split:

```text
ActorTuning
CharacterBrainSpec
ActorConfig fields that mirror character/archetype identity
sprite_character_id
sprite_override_npc_name
```

where their facts now have proper owners.

Do not keep read models merely because deleting them touches many systems.

## Phase 7 — remove legacy authored infrastructure

Delete:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
character_archetypes.ron
enemy_roster.rs
old content schema
provider roster fragments
legacy exports
legacy tests
compatibility constructors
```

The old vocabulary should disappear from production code.

## Phase 8 — rename and document the final architecture

Search production code and authored content for stale concepts.

Update:

```text
AGENTS-adjacent architecture docs
character-definition design docs
ADRs whose ownership claims changed
MODULES.md
public API docs
authoring docs
```

Keep comments about current ownership/invariants.

Remove investigation chronology and obsolete migration narratives.

---

# Explicit deletion goal

There should be a large deletion payoff.

Current obvious legacy artifacts include approximately:

```text
ArchetypeSpec                             319 lines
CharacterRoster/enemies module          1198 lines
character_archetypes.ron                 845 lines
enemy_roster.rs                           75 lines
---------------------------------------------
obvious core legacy                    ~2437 lines
```

Additionally:

```text
ActorTuning                              275 lines
autonomous_reconcile                   1045 lines
BrainBinding/catalog compatibility      substantial
provider roster fragments               additional
content-schema plumbing                  additional
legacy tests                             additional
```

Not every line in those latter files should vanish, but large parts should.

A successful implementation should not result in:

```text
+4000 new abstraction lines
-2400 legacy lines
```

because that likely means the old model was wrapped instead of removed.

Several thousand **net deleted legacy lines** is a realistic target, although correctness and ownership matter more than hitting a numeric quota.

Report the actual before/after LOC and list every deleted legacy type/file.

---

# Required behavioral/architectural tests

Add focused tests proving the new ontology.

## Same definition, multiple instances

```text
spawn Fretjaw twice
```

Prove same character identity, independent runtime identities/state.

## Same character through different contexts

Spawn the same Goblin definition through:

```text
NPC placement
hostile room placement
encounter
programmatic spawn
match CPU
match human
```

Where contexts are applicable, verify the **intrinsic body facts agree**:

```text
body geometry
vitals
movement identity
intrinsic abilities
action/moveset baseline
mount capabilities
character identity
```

Only contextual/controller facts should differ.

Do not require every route in one giant test if smaller focused tests prove the invariant more clearly.

## Controller independence

Use one character with:

```text
Human
CPU
Replay or policy where practical
```

Prove changing controller does not change intrinsic character/body definition.

## Provocation

For a named peaceful character:

```text
before provoke:
    character X
    intrinsic kit K
    vitals V

after provoke:
    character X
    intrinsic kit K
    vitals V
    hostile disposition
    different autonomous controller
```

No body reconstruction.

## Iron Mary

Prove an ordinary Iron Mary enemy obtains Iron Mary's authored kit rather than the old shark-rider/archetype kit.

Also prove an explicit brain/controller override works when intentionally authored.

## Provider character

A provider-defined character that does not exist in Ambition's built-in content must construct correctly through the generic character path without a core code edit.

## Complete buildable cast

Every declared normal character intended for runtime spawning should have a complete prepared definition.

Do not define buildability via `PLAYABLE_ROSTER`.

## No authoritative display-name lookup

Authoritative spawn tests should use stable character IDs.

## Rollback

Prove:

```text
CharacterIdentity
controller selection
provocation/controller transition
```

restore correctly without `CharacterRoster` or archetype reconstruction.

---

# Required absence checks at the end

Use direct source searches—not a new policy framework—to verify the migration is complete.

Production code should no longer contain the old architecture concepts except in historical migration documents where explicitly retained:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
HostileArchetypeId
ProvokedArchetype
character_archetypes.ron
PlayableKitSource
PreparedKit::HostCode
sprite_character_id
```

`CharacterBrain` should also be gone from normal actor placement/construction unless inspection proves a genuinely unrelated meaning remains.

`ActorTuning` and `CharacterBrainSpec` should either be gone or reduced/renamed into narrowly coherent concepts; do not leave them with their current mixed authority.

Do not create an automated grep ratchet solely for these names unless an existing repository mechanism naturally owns that assertion.

---

# Validation

Follow current repository instructions.

At minimum:

```text
cargo check -p ambition_app
```

is the integration compile gate, not merely checking a leaf crate.

Run focused tests for every touched actor/construction/content domain.

Because authored schemas and `.ron` fields change, search and migrate **both Rust and authored RON**, including ignored/generated authoring files where repository instructions require filesystem search rather than Git-only search.

Run the relevant content compiler/validation tests.

Run targeted demo tests for at least:

```text
Ambition
Mary-O
Sanic
Smash
```

where their provider character definitions/rosters were migrated.

Do not spend the task running enormous unrelated suites repeatedly; use focused tests during the migration and the app integration gate at the end.

---

# Definition of done

This task is complete only when all of the following are true:

1. **CharacterDefinition is the one reusable actor-template authority.**

2. **PreparedCharacterDefinition is complete.**
   Runtime character construction does not ask another gameplay registry what the character really is.

3. **A character can be instantiated arbitrarily many times.**
   Character identity and runtime entity identity are explicitly separate.

4. **Every ordinary actor construction path is character-first.**

5. **Current controller is separate from character identity.**
   A character may provide a default autonomous profile, but human/CPU/replay/policy control does not change the body definition.

6. **Intrinsic character abilities/body/vitals/moves come from the character definition.**

7. **Respawn/faction/disposition/encounter lifecycle come from spawn/session/ruleset context.**

8. **Provocation does not morph the body.**
   It changes hostility/controller state.

9. **The old enemy-archetype authority is deleted.**

10. **`character_archetypes.ron` is deleted.**

11. **`CharacterRoster` and provider roster fragments are deleted.**

12. **No `brain` field secretly selects health/body/capabilities.**

13. **No `character_id` field means merely “costume.”**

14. **No display string is authoritative character identity.**

15. **The player robot no longer requires a HostCode character-kit exception.**

16. **Match, NPC, enemy, encounter, summon, and programmatic paths agree on what one character is.**

17. **New names describe current semantics without Legacy/New/V2 compatibility vocabulary.**

18. **The final change deletes substantially more legacy machinery than it adds adapter machinery.**

---

# Guiding question

For every field you migrate, ask:

```text
Is this a fact about
    the reusable character,
    the current controller,
    or this particular spawn/session?
```

Put it with that owner.

Then delete the old place.

The intended final experience for an engine user is extremely simple:

```text
author Goblin once
spawn Goblin anywhere
→ it is a Goblin

author Fretjaw once
spawn Fretjaw twice
→ two independent Fretjaw actors

change who controls either body
→ same character, different controller
```

Adding a new character should ultimately mean:

```text
author the character's body/capabilities/presentation
choose its normal autonomous profile
register the content
```

with **zero actor-engine code edits**.

Do not stop when the new path works beside the old one.

Finish the migration, delete the old authority, and leave the repository with one obvious way to answer:

> What is this actor?

The answer should be:

> It is an instance of this CharacterDefinition.

---

> **Jon's closing emphasis, verbatim:** One code-level point I would emphasize to
> the implementing agent in conversation, if it asks: **`character_archetypes.ron`
> is not merely an enemy tuning file to be renamed.** The fact that `respawn`,
> `attacks_player`, body capabilities, AI strategy, and mount semantics all
> coexist in one row is precisely the evidence that it should be decomposed and
> then deleted.
>
> The current tree is unusually favorable for doing this now because the generic
> ECS runtime underneath it has already been unified. The expensive part is the
> content/construction migration; the simulation core largely does not need to be
> reinvented.

---

# ⇥ APPENDIX A (agent, 2026-08-10) — the field-ownership ledger

⚠ **added by the agent, below Jon's brief and outside it.** His brief says
*"use this classification as the starting point, then verify every field's
consumers before migrating it."* This is that verification, banked so phase 2
does not re-derive it.

⛔ **the reference counts are a NAME-BASED UPPER BOUND**, not consumer counts. A
grep for `\.melee` matches every unrelated `.melee` in the workspace, which is
why `melee` reads 105 refs in 40 files. Use the count to rank the work, never to
claim a field is nearly unused — ⭐ but the SMALL numbers are trustworthy in the
direction that matters: `mount_death_splash` at 1 really is one site.

`ArchetypeSpec` has **49 fields**. Their owners under the new model:

## Pure assembly machinery — deleted, migrates nowhere (2)

| field | note |
|---|---|
| `inherits` | archetype-to-archetype inheritance; character definitions reference reusable profiles instead |
| `movement_resolved` | `#[serde(skip)]`, filled by the roster's inheritance pass — it exists only because the roster exists |

## Character/body intrinsic (26)

`movement` · `max_health` · `run_speed` · `mass` · `surface_walker` ·
`cling_breaks_on_hit` · `is_aerial` · `explodes_on_death` · `divides_on_death` ·
`charge_crash_explodes` · `weight` · `mount_class` ·
`pilotable_mount_classes` · `mount_death_splash` · `default_size` · `melee` ·
`ranged` · `held_item` · `can_blink` · `can_fly` · `can_shield` · `can_dash` ·
`body_contact_damage` · `contact_strength` · `signature_move` · `move_style`

## Autonomous-controller policy (11)

`patrol_effort` · `chase_effort` · `aggro_radius` · `attack_range` ·
`attack_cooldown_mult` · `turns_at_walls` · `brain_template` · `fighter_level` ·
`smash_hit_band` · `smash_heavy` · `smash_dash_to_close` · `smash_duelist`
(and `provoke_forced_brute_min_aggro`, below)

⭐ **`turns_at_walls` is not in Jon's starting list and its own doc already
classifies it**: *"this is control policy consumed by Patrol/Wanderer brains, not
movement/collision policy."* The field had the answer written on it.

## Spawn / session / ruleset (2)

`respawn` · `attacks_player` — both named in the brief. `attacks_player` is
**deleted**, not moved: there is no privileged player identity.

## ⭐ THE SEVEN JUDGEMENT CALLS — read these before phase 2

These are the fields where the three-way split does not answer itself. Each one
is a decision the migration must make deliberately, with what is known:

1. **`is_aerial`** — a live TWO-SOURCE CONFLICT, already documented on the field.
   `new_peaceful_npc_in` reads the catalog's `body_kind: Floating`; the hostile
   `EnemySpawn` path reads this. **The Perfect Cellular Automaton is `Floating`
   in its catalog row and played grounded by the shipped duel.** Unifying the two
   authorities forces that disagreement to resolve, and resolving it changes how
   a shipped fight plays. ⛔ do not fold it silently — this is exactly the class
   of thing the brief means by *"semantically migrated, not blindly trusted"*.
   `Option<bool>` must survive the move: `None` ≠ `Some(false)` is why the
   conflict is expressible at all.
2. **`is_sandbag`** — reads as a character fact and behaves as a placement role.
   It reaches the RENDER read model (`ActorRenderView.is_sandbag`, a
   sprite-upgrade fallback), `save_sync`, and cluster pathing. A sandbag is a
   training instance of some body, which argues placement; but three consumers
   treat it as identity. Decide once, and move all three.
3. **`never_dies`** — same shape, cleaner answer: `damage_apply` uses it to make
   a body take no health damage. That is either an intrinsic trait (an immortal
   creature) or a training-mode ruleset fact. The brief allows the intrinsic
   reading *"only where these really are properties of the character"* — the
   shipped users are sandbags, which suggests it travels with `is_sandbag`.
4. **`death_policy`** — the brief puts ruleset-owned death behaviour in session
   context, and `HpDepleted` vs `Unbounded` is precisely a ruleset fact (Ambition
   has health, a platform fighter has stocks and a blast zone). ⚠ but it is
   authored per-body today and a mixed roster is expressible. Recommend: ruleset
   owns the default, a character may not override it.
5. **`provoke_forced_brute_min_aggro`** — provocation controller policy, which the
   brief wants authored explicitly rather than inferred. It is a *controller
   profile selected on a transition*, not a number on the body; the cleanest form
   is a named provoked-profile reference, and this f32 becomes a field of that
   profile.
6. **`ranged_visual`** — the brief's own hint applies: *"a ranged projectile's
   visual should ideally be carried by the ranged action/projectile specification
   rather than redundantly copied onto a top-level character."* Move it into
   `RangedActionSpec`, do not carry it on the character.
7. **`dream_seed`** — presentation metadata for the psychedelic shader pass.
   Generic-enough to ride the character definition (like sheet/portrait/voice),
   but check whether it is Ambition-specific editorial metadata first; the brief
   forbids Hall/gallery-class concepts entering the generic model.

## The capability-authored-twice set

`can_blink` / `can_fly` / `can_shield` / `can_dash` are body capabilities, and
`CharacterBrainSpec` carries `smash_can_blink` / `smash_can_fly` /
`smash_can_shield` alongside them. ⇒ **the duplication the brief calls out is
real and is exactly these three pairs.** The controller reads the body's
capability state; only the DECISION flags (`smash_heavy`, `smash_duelist`,
`smash_dash_to_close`, `fighter_level`) stay on the profile.
