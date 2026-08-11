# Focused review at 360a8f0 — keep going, but tighten D85 before stamping it complete

Jon relayed this (GPT 5.6) on 2026-08-11, mid-turn, reviewing `360a8f0f5911`.
**Recorded verbatim below** because it corrects a claim I had already committed:
D85 was reported as "the seat asks for nothing" when a SECOND template observer
still had work to do on it.

⚠ **it reviews 360a8f0, and two of its items landed at 8afdd2988 before it
arrived** — its §6 (publish the `duelist*` ladder, delete `SMASH_ROSTER_RON`) is
DONE, and its §4 unblocks D86. Read the progress table before working an item.

## Progress

| Item | State |
| --- | --- |
| §1 the seat needs `grant_prepared_character_body` + `ProjectedCharacterKit` | ✔ **DONE** — `KitOwnership::CallerResolved` (the gate was asking *who writes the kit* when the question is *is a derive coming*); the seat's hand-written stamp and hand-written motion model are DELETED in favour of the shared grant. Acceptance test + poison: with the grant disabled the OLD test stays green (the derive writes an identical-looking baseline) and only the new one goes red |
| §2 ONE resolved seat kit (action set + identity kit + combat kit + moveset + ranged execution) | ▢ — the overlay runs once but its `ActionSet`/`CombatKit` are DISCARDED and rebuilt in `realize_seat` |
| §3 effective abilities resolved BEFORE the kit is derived | ▢ |
| §4 D86: **KEEP the robot's charge**; rename `RangedExecution::HostCharge` → `ChargedProjectile` | ▢ — ⭐ **no longer blocked on Jon** |
| §5 Robot v3 authors its canonical `ActionSet`; delete `HostCode` | ▢ |
| §6 publish the `duelist*` ladder, delete `SMASH_ROSTER_RON` | ✔ **DONE at 8afdd2988** — and the real blocker was not publication: the registry lookup was provider-blind, so the arm had never fired (D87) |
| §7 `CharacterBodyBlueprint` overpromises — resolve geometry at preparation | ▢ |
| §8 don't let `smash_fighter_kit()` broaden; author Hall repertoires | ◑ — authoring primitives extracted to `moveset_authoring.rs`; adopters still 3 |
| §9 preserve the deletion trajectory | ongoing |

---

## The brief, verbatim

Continue from live HEAD. Reviewed checkpoint: `360a8f0f5911`. The campaign is
healthy. Do not restart it.

Do not touch sprite authoring or sprite-generation work. Jon owns that side.
Engine animation/state slots and fallback mappings are fine when mechanics need
them.

Recent work is strong: D78's duplicate ordinary enemy/protagonist template
application was structurally addressed; initial protagonist construction no
longer asks `RecharacterizeBody` to finish it; character-first construction no
longer fabricates a giant inert `ArchetypeSpec` merely to satisfy
`ActorClusterSeed`; provocation is moving toward same-body/different-controller
semantics; `PreparedMatch` consults published BrainProfiles before the legacy
roster; real Robot v3 owns its eleven canonical platform-fighter move timelines;
character archetypes are down to roughly **355 lines / five rows** (`combatant`,
`cellular_automaton_fighter`, `medium_striker`, `gradient_seeker`,
`sandbag_infinite`). Keep pushing.

But D85 has two additional requirements before the match seat is truly
"complete on construction."

### 1. Removing `RecharacterizeBody` is only HALF of D85

You have correctly identified the remaining persona grants (motion model,
physical baseline, stamp). But there is another template projector watching the
seat: `project_prepared_character_definitions`. A freshly seated body has
`WornCharacter` and no `ProjectedCharacterKit`, so that system will still see the
new character and, after ordinary construction, grant template-owned facts such
as `AuthoredHurtboxes`, `ResolvedHurtboxes`, `DamageableVolumes`,
`SpritePosedBody` / authored body projection, `AuthoredMovementTuning`, the
motion-model switch, and the `ProjectedCharacterKit` stamp.

That means a seat can stop asking the **persona** derive and still remain a
two-phase character body. The D73 invariant is stronger:

> An ordinary match seat must be a complete instance of its CharacterDefinition
> on its construction frame. No later identity-observer system is required to
> finish its body.

**Use the shared prepared-character body materializer.** You already have
`grant_prepared_character_body(...)`, which owns exactly these non-persona
template grants and stamps `ProjectedCharacterKit`. Use that during seat
construction rather than manually copying another subset. Because the seat
resolves its own match-specific kit, use the appropriate ownership mode so the
helper does NOT overwrite that kit, but DOES install hurtboxes, posed body /
authored body projection, movement tuning, motion model, and
`ProjectedCharacterKit`. Then establish the gameplay/persona baseline for the
already-materialized seat and remove `RecharacterizeBody`.

After D85, a normal seat should spawn with BOTH template-application records
current — `PersonaBaseline` and `ProjectedCharacterKit` — and neither template
observer should have work to do on the next simulation pass. Do not solve only
one of the two.

### 2. The latest "overlay runs once" commit still computes the kit in multiple places

Preparation currently keeps `IdentityKit` and `ActorMoveset` and DISCARDS the
resolved `ActionSet`, `CombatKit` and `RangedExecution`. Then `realize_seat`
independently does `match_kit or character action_set or default` → `ActionSet`,
→ `CombatKit`. So although the expensive overlay call occurs once, there are
still multiple semantic answers to: *what kit does this seat actually have?*
That is the exact kind of divergence this campaign is eliminating.

**Carry one resolved seat kit.** Have preparation produce one coherent value
conceptually like `PreparedSeatKit { action_set, identity_kit, combat_kit,
moveset, ranged_execution }` (exact name is yours). All five values should come
from ONE resolution of *character repertoire + match-specific action override +
effective BodyAbilities*. Then `realize_seat` inserts those exact values. Do not
reconstruct `ActionSet` and `CombatKit` a second time. This will also make the
final removal of `RecharacterizeBody` much safer.

### 3. Resolve the match's EFFECTIVE abilities before deriving the kit

Current ordering is: prepare character/seat → run character overlay using
`seed.body` abilities → spawn body → later apply `fighter_abilities` mask. That
is only harmless while the later persona pass comes back and repairs any
ability-dependent kit. Once D85 removes that pass, the ordering becomes
observable. For example Robot v3 is still HostCode today, and HostCode's
effective `ActionSet` depends on `abilities.attack` and `abilities.shield`. A
match could forbid one of those, yet preparation currently derives the identity
kit/moveset from the pre-mask body.

**Correct order.** During `prepare_match`, compute `effective_abilities =
character abilities ∩ match restriction`, with the current migration fallback
only where unavoidable. Use THAT SAME `effective_abilities` for the
`PreparedSeatKit` derivation, `BodyAbilities`, `AbilityBase` and AI capability
inspection, and carry it on the prepared seat/build plan. Do not derive the kit
against one `AbilitySet` and mutate the body to another after spawn.

Add a focused regression: character has Shield; match mask removes Shield;
prepared seat's effective `BodyAbilities` has no Shield and the resolved kit does
not contain the shield-dependent action; first frame the same answer; second
frame no persona correction changes it. This test becomes especially useful when
HostCode disappears.

### 4. D86 product decision: KEEP Robot v3's charged projectile

This is no longer blocked on Jon. The robot should keep the charged
Hadouken/projectile behavior. Removing it merely to delete HostCode would be a
gameplay regression and would violate the larger product rule:

> Player Robot v3 is the same character with the same core repertoire in Ambition
> and Smash; the modes change interpretation and restrictions rather than
> silently replacing its moves.

The charge is therefore a **character/action execution fact**, not "host code."

**Generalize the existing execution vocabulary.** `RangedExecution::HostCharge` /
`MovesetVerb` — the word `Host` exposes the old special case. Move the semantic
fact into the reusable character domain, with vocabulary more like
`RangedExecution::ChargedProjectile` / `MovesetVerb` (or an equivalent clean
name). Default ordinary characters to `MovesetVerb`. Author Robot v3 as
`ChargedProjectile`. Preparation resolves that onto the character/seat kit. The
runtime marker `ChargesProjectiles` and its projectile state remain runtime
projections of that authored execution mode. Do not create `PlayerRobotCharge` or
another protagonist-specific branch.

### 5. Robot v3 should author its complete canonical ActionSet next

The moves are already where they belong. Now move *what action slots the robot
intrinsically has* out of `default_player_action_set(...)`, `PreparedKit::HostCode`
and `PlayableKitSource::HostCode`, and into normal character authoring. Then
runtime progression/body grants filter what is currently available.

Target: the Robot `CharacterDefinition` carries a canonical `ActionSet`, a
canonical `ActorMoveset` and `ranged execution = ChargedProjectile`;
`BodyAbilities` / progression enable or disable actions; Smash rules further
restrict capabilities where desired. The body should not acquire a second moveset
just because progression is incomplete. A general helper may filter the
character's canonical `ActionSet` by effective `BodyAbilities`. Do not keep a
HostCode-specific action-set builder after the robot's repertoire is authored.
Once this works, DELETE `PlayableKitSource::HostCode` and `PreparedKit::HostCode`
rather than leaving them empty.

### 6. There is a very cheap, high-value Smash deletion available immediately afterward

The Smash demo's CPU archetype rows are now almost embarrassingly ready to
disappear. `SMASH_ROSTER_RON`'s six rows mix body facts (100 HP, 200 run speed, 4
contact damage, Walk, `attacks_player`) with actual controller policy (`template
= Fighter`, `aggro_radius = 600`, `attack_range = 48`, `patrol_effort = 1`,
`chase_effort = 1`, `fighter_level = N`). The body half is no longer legitimate
for a match seat. Publish all six as real `BrainProfile`s — you already publish
`duelist`; add the ladder variants beside it. Then `SMASH_DUELIST_BRAIN`,
`duelist_l1`, `duelist_l3`, … resolve purely through `BrainProfileRegistry`.
Delete the Smash `CharacterRosterFragment` registration and `SMASH_ROSTER_RON`.

This is a very strong architectural proof: the Smash CPU controller no longer
touches an enemy-body archetype table at all. After that, remove the
`CharacterRoster` parameter/fallback from `PreparedMatch` as soon as its
remaining non-Smash callers are migrated. Do not keep the fallback because the
demo used to need it.

### 7. `CharacterBodyBlueprint` is better, but its name currently overpromises

It says *everything construction needs to build this character's body*. But the
constructor still needs `CharacterCatalog` and `AuthoredSheets` to discover
collision geometry, and D85 still has to fetch/apply `PhysicalBaseline`,
`motion_model`, `movement_tuning`, hurtboxes, posed-body facts and the kit
elsewhere. That means it is not yet the complete body blueprint. This is not a
reason to delete it. Make it true.

**Resolve physics geometry before runtime construction.** A character body
constructor should not ask a presentation/catalog registry *how large is my
collider?* Resolve the body/collision geometry at character preparation and carry
the resolved answer. The renderer may consume the same authored source, but
simulation construction should not need presentation lookup. Similarly, carry the
resolved character body facts needed by construction: typed `CharacterId`,
resolved body/collision geometry, locomotion, motion model, movement tuning,
vitals/mass/weight, hurtboxes, abilities, contact/death traits, mount capability,
held-item baseline. Then a mode can explicitly modify the contextual pieces.

If a separate value is useful, distinguish `CharacterBodyBlueprint` (intrinsic
prepared character body) from `ResolvedSeatBody` / `CharacterSpawnBuild`
(character body + match/context overrides) rather than making one struct
accumulate unrelated match state. The direction is *prepared character authority
→ resolved construction value → runtime components*, not *partial blueprint +
catalog lookup + sheet lookup + PhysicalBaseline lookup + persona overlay +
projection pass*.

### 8. Do not let the remaining Smash migration bridge become policy

Current select behavior is acceptable migration scaffolding: a character that
authors a repertoire keeps it; one that authors none gets the Smash generic
fighter floor. But Jon's Puppy Slug thought experiment remains the end-state
poison. If somebody forcibly seats Puppy Slug without an explicit fighter
adaptation: Attack → nothing; Jump → nothing if the body lacks it; crawler
locomotion → still crawler locomotion; stocks / percent / blast zones → work.

Therefore `smash_fighter_kit()` must continue losing adopters and eventually
disappear. Do not broaden it. For Hall characters that ought to be legitimate
fighters, author the actual character repertoire rather than treating "Hall NPC
was peaceful" as intrinsic inability forever.

### 9. Current deletion trajectory is good — preserve it

The flagship archetype file is now approximately 355 lines / 5 rows, from roughly
843 lines / 24 rows. That is real architectural payoff. Once D85 + D86 are clean,
resume removing `cellular_automaton_fighter`, `medium_striker`,
`gradient_seeker`, `sandbag_infinite`, `combatant` according to their actual
owners. `combatant` should die last because it is the compatibility fallback. Do
not rush that one by copying its mixed facts somewhere else.

### Immediate order

1. D85: resolve effective abilities during preparation.
2. D85: carry one complete resolved seat kit instead of retaining only two outputs.
3. D85: use the shared prepared-character body grant so a seat also starts with
   hurtboxes/body/tuning/motion projection and `ProjectedCharacterKit`.
4. D85: establish `PersonaBaseline`, remove `RecharacterizeBody`, and prove
   neither template observer changes the body on the next tick.
5. D86: keep the robot charge; move ranged execution into reusable character
   authoring.
6. D86: author Robot v3's canonical ActionSet and delete HostCode.
7. Migrate Smash's `duelist*` CPU policies to `BrainProfileRegistry`; delete the
   Smash roster fragment.
8. Resume the remaining five archetype deletions.
9. Only then take on the broader 125-adopter BrainPreset retirement if time remains.

### D85 acceptance test

Do not call D85 complete until a newly constructed match seat has, on its first
observable simulation frame: `CharacterIdentity` / `WornCharacter`, `ActionSet`,
`IdentityKit`, `CombatKit`, `ActorMoveset`, effective `BodyAbilities`,
`BodyHealth`, mass / knockback weight, hurtboxes, resolved body geometry, motion
model, movement tuning, death traits, `ProjectedCharacterKit`, `PersonaBaseline`
— as applicable to that character. And `RecharacterizeBody` ABSENT. Then run
another simulation update with no hot reload or explicit re-template and verify
the character-template systems do not alter those values. That is the actual
"one body, one construction" milestone.
