# Redirect, 2026-08-11 (second) — close the duplicate template-application architecture

**This file OUTRANKS the first redirect (`redirect-2026-08-11-finish-the-architecture.md`)
and the campaign brief wherever they disagree.** Jon's second redirect, relayed
from a GPT 5.6 review of checkpoint `7ce7397ec474`, recorded VERBATIM.

⛔ **AND IT CAUGHT A CLAIM OF MINE THAT WAS WRONG.** I reported that a
character-first body is built complete and "the projection never touches it
again", and I verified that by instrumenting
`project_prepared_character_definitions` — which does skip. But that is not the
system the contract depends on. `apply_worn_character_gameplay` is a SECOND
template applier, gated on `stale_cast = PersonaBaseline.is_none_or(..)`, and
`grant_prepared_character_body` never writes `PersonaBaseline`. So a freshly
constructed character-first body has no baseline, `stale_cast` is true, and the
character IS applied again on the next persona pass. I instrumented one of two
writers and reported the answer as if it covered both.

⛔ **STANDING, from this message:** do not touch sprite authoring or
sprite-generation work — Jon owns it. Engine-side animation/state slots and
fallback selection are fine when mechanics require them.

---

## VERBATIM

# Redirect from 7ce7397 — the campaign is healthy, but close the remaining template-application split now

Continue from live HEAD. Reviewed checkpoint:

```text
7ce7397ec474
```

Do not restart the campaign. The recent direction is good.

Do not touch sprite authoring or sprite-generation work. Jon is handling sprites. Engine-side animation/state slots and fallback selection are fine when required by mechanics, but do not spend this campaign drawing or redesigning art.

## What is working

The architecture has materially improved.

`character_archetypes.ron` is now only 414 lines / 7 rows:

```text
combatant
cellular_automaton_fighter
medium_striker
gradient_seeker
sandbag_infinite
pirate_raider
pirate_heavy
```

The new shared-controller model is substantially healthier:

```text
BrainProfile
BrainProfileRef
BrainProfileId
BrainProfileRegistry
```

`BrainProfile` no longer owns hostility or copies body capabilities.

The canonical Robot v3 moveset now lives in reusable Ambition content instead of belonging to Smash shadow characters.

The Smash work—air dodge, tumble, knockdown, tech, getup, strong attacks, landing behavior—is going into generic body/combat systems.

Keep all of that.

But pause broad migration long enough to close the following seams.

# P0 — D78: there are STILL two character-template writers

The live implementation contradicts the intended `WornCharacter` / `RecharacterizeBody` contract.

`WornCharacter` correctly documents:

> carrying this component populates nothing on its own.

And `RecharacterizeBody` correctly documents:

> ordinary construction does not go through this; it is for genuine runtime re-template.

But `apply_worn_character_gameplay` still does:

```text
stale_cast =
    PersonaBaseline is absent
    OR id changed
    OR prepared-generation changed

if recharacterize || stale_cast:
    apply character overlay
```

A freshly constructed character-first enemy currently:

```text
spawn body
insert WornCharacter
grant_prepared_character_body(...)
```

but `grant_prepared_character_body` does NOT establish the `PersonaBaseline` that `apply_worn_character_gameplay` tests.

Therefore on the subsequent persona pass:

```text
PersonaBaseline == None
→ stale_cast == true
→ character is applied again
```

So the statement "construction is now complete and the character projection never touches it again" is not true of the live code.

This also explains why merely moving `ActionSet` to construction has not fully removed the architectural shape implicated by D78.

## Fix the architecture rather than doing more checksum probes

There should be ONE implementation of:

```text
PreparedCharacterDefinition
→ template-derived gameplay state
```

with two explicit boundaries:

```text
Construction
Replacement
```

Construction means:

```text
fresh body
initialize template-derived state
initialize the applied-template generation/stamp
no displaced old persona exists
```

Replacement means:

```text
existing live body
preserve the runtime state that should survive re-characterization
capture/retract displaced template-owned facts
apply new template
update applied-template generation/stamp
```

Both should invoke the same underlying template materializer rather than duplicating:

```text
grant_prepared_character_body
```

and:

```text
apply_worn_character_gameplay
```

as independent answers to what a character grants.

The exact decomposition is yours to design.

A reasonable endpoint might have something like:

```text
apply_character_template(
    prepared,
    boundary: Construction | Replacement,
    ...
)
```

with an authoritative gameplay-template application record.

Do not mechanically merge visual presentation tracking into gameplay state if presentation needs its own memo.

But gameplay should not have:

```text
ProjectedCharacterKit
+
PersonaBaseline
+
two independent template appliers
```

certifying overlapping work forever.

## Ordinary initial construction must not carry RecharacterizeBody

Current initial match seating explicitly inserts `RecharacterizeBody`.

Current protagonist creation explicitly inserts `RecharacterizeBody`.

Those are still normal construction paths.

Remove that need.

A match seat should arrive fully built from:

```text
PreparedCharacterDefinition
+
match contextual policy
```

on its construction frame.

The protagonist should arrive fully built from:

```text
PreparedCharacterDefinition
+
progression/runtime grants
+
session context
```

on its construction frame.

`RecharacterizeBody` should survive for actual operations such as:

```text
Mary-O transformation
intentional runtime character swap
hot reload / refreshed definition
other genuine re-template
```

not as "finish initializing this actor."

## Add a direct invariant test

Do not infer this from rollback only.

Construct a character actor and verify:

```text
construction frame:
    complete character kit/body present
    applied-template stamp current

later update with no hot reload/recharacterization:
    no template reapplication occurs
```

Also prove independently:

```text
changing CharacterIdentity alone
    does NOT rewrite the body

CharacterIdentity change + RecharacterizeBody
    DOES perform replacement
```

Then rerun D78.

If D78 is still red after there is literally one construction writer and no normal post-spawn reapplication, resume the per-frame deterministic-state investigation.

But close this structural contradiction first.

# P1 — provocation has reintroduced presentation → gameplay identity

The new pirate provocation direction is correct:

```text
same character
same body
different disposition
different autonomous policy
```

But `provoke_actor_in_place` currently discovers the prepared character through:

```text
em.config.sprite_character_id
```

That is the exact identity inversion D73 already removed from authored enemy spawning.

Provocation must use the body's actual:

```text
CharacterIdentity / WornCharacter CharacterId
```

Never use sprite/presentation identity to decide which character's provoked policy applies.

Thread typed CharacterId into the mutation/query seam as necessary.

## Make character-first provocation rollback-authoritative

The new character path currently changes:

```text
em.config.brain_profile
ActorDisposition
live Brain
```

and returns.

The old rollback authority still models provocation as:

```text
AutonomousSource::Provoked {
    archetype: HostileArchetypeId
}
```

That cannot be the final representation for the new path.

A provoked character should have a reconstructible autonomous-policy source such as a resolved BrainProfile identity/value, without reconstructing another body archetype.

Finish this coherently with rollback before declaring the pirate archetypes deletable.

Target:

```text
CharacterIdentity
    unchanged

autonomous binding
    peaceful/default profile
        ↓ provoke
    resolved combat BrainProfile

disposition
    Peaceful → Hostile
```

Rollback restores the controller-policy selection separately from the character body.

That should make `HostileArchetypeId` removable as the legacy provocation path disappears.

# P2 — remove the fake ArchetypeSpec from character-first construction

`ActorClusterSeed::new_character_in` is vastly better than the old fourteen-argument constructor.

But it still manufactures an enormous **inert `ArchetypeSpec` literal** purely because `ActorClusterSeed` requires a `spec` field.

That is now a very useful deletion signal.

A character-first actor should not contain a fake old-style body definition saying:

```text
max health ...
run speed ...
melee = None
ranged = None
mount = None
can_blink = false
...
```

solely to satisfy a legacy struct shape.

That fake value is one of the things physically preventing `ArchetypeSpec` from disappearing.

Inspect every remaining consumer of:

```text
ActorClusterSeed.spec
ActorConfig/spec projections
```

Move each live responsibility onto its actual runtime component/configuration owner.

Then remove `spec` from the generic character-first seed.

If legacy construction temporarily needs it, isolate it to the legacy constructor rather than forcing every new character actor to carry a fabricated legacy archetype.

This should be one of the next major deletion milestones.

# P3 — there are currently TWO different types called BrainProfileRef

The new type:

```text
ambition_entity_catalog::BrainProfileRef
```

correctly means:

> provider-relative reference to a real shared BrainProfile.

But:

```text
character_catalog::binding::BrainProfileRef
```

still exists and actually means:

> reference to the old BrainPreset vocabulary.

Do not leave two identically named public concepts with different referents.

Rename the old one immediately to:

```text
BrainPresetRef
```

if it must survive temporarily.

Better, continue D81 and migrate its remaining users.

The final architecture should converge on one controller-policy vocabulary, not keep BrainPreset + BrainProfile indefinitely.

Do not mechanically convert a preset to a profile: the agent was correct that absolute speed cannot become normalized effort without body context.

Semantically migrate each preset:

```text
absolute/body facts
    → CharacterDefinition

decision policy
    → BrainProfile

relationship/lifecycle facts
    → context
```

then delete it.

# P4 — PreparedMatch still has a CharacterRoster dependency

The body side of match construction has improved substantially.

But `PreparedMatch` still receives `CharacterRoster` and resolves CPU policy through:

```text
archetypes.brain_profile_for(...)
```

So Smash is not yet fully proving the new controller architecture.

Move match CPU controllers to:

```text
BrainProfileId
+
BrainProfileRegistry
```

as the shared-profile migration permits.

The final match preparation contract should need:

```text
PreparedCharacterRegistry
BrainProfileRegistry
match participants/rules
```

not `CharacterRoster`.

This is an especially useful deletion because Smash already states the intended model at its public API:

```text
character
+
controller
+
team
```

Make the implementation agree with it.

# P5 — Robot v3 progress is excellent, but HostCode remains the next big exception

The move data migration is exactly right.

The real:

```text
player_robot_v3
```

now owns the eleven canonical attack timelines in reusable Ambition content.

Keep that.

The same down-air already receiving Ambition-vs-Smash interpretation through rules rather than duplicate attacks is exactly the desired architecture.

But Robot v3 still has:

```text
playable_kit: HostCode
```

and `PreparedKit::HostCode` still exists.

So the robot now owns:

```text
what its attacks ARE
```

but host code still determines:

```text
what actions it HAS
```

Finish that separation.

The character should author its canonical action repertoire.

Runtime body state/progression should determine which authored actions are currently available.

Conceptually:

```text
CharacterDefinition
    canonical repertoire:
        attack
        directional attacks
        ranged/special/etc.

runtime progression / equipment / temporary grants
    enable or disable parts of that repertoire
```

Do not represent early-game Robot as a different CharacterDefinition.

Do not have Smash synthesize missing robot actions.

Once this works:

```text
PlayableKitSource::HostCode
PreparedKit::HostCode
```

should die.

That is also the clean path toward making Hall peacefulness a controller/placement fact instead of `default_action_set: peaceful` determining what somebody intrinsically knows how to do.

# P6 — keep shrinking the Smash bridges, but do not spend time on sprites

The current Smash direction is good.

Keep the new:

```text
air dodge
tumble
knockdown
tech
getup
strong attacks
landing lag/autocancel
damage-scaled knockback
shared Robot moves
```

No redirect needed there.

Do not spend this agent's time authoring sprites for those states. Mechanical animation-state slots/fallbacks are enough; Jon owns the sprite work.

Continue shrinking:

```text
smash_fighter_kit()
fighter_abilities fallback
smash_duelist_a/b
```

as real characters acquire canonical repertoires and capability definitions.

The current semantics where `fighter_abilities` is a MASK for authored characters are much healthier.

The remaining branch:

```text
character authored no abilities
→ mode grants the whole fighter floor
```

is migration scaffolding and must eventually disappear.

Likewise `smash_fighter_kit()` should reach zero adopters rather than become a permanent "make this character Smash-compatible" function.

The Puppy Slug forced-seat test remains the poison for both.

# P7 — explicit CharacterId must eventually stop falling back to a legacy body

The current room-spawn path still allows an explicit character id to fall back to an archetype in some partial compositions when that character definition is unavailable.

That is understandable migration scaffolding for the Mary-O plane packaging case.

It is not final semantics.

An explicit:

```text
character_id = X
```

must eventually mean:

```text
instantiate CharacterDefinition X
```

or fail composition/preparation.

If a placement merely borrows X's presentation, represent that as presentation borrowing.

If a provider needs X's real character, declare/register the provider dependency.

Do not preserve:

```text
character X if available,
otherwise unrelated archetype wearing X's art
```

as a production engine behavior.

# Current priority order

Do not broaden Group-B/Group-C migration until P0 is structurally closed.

Then proceed:

1. unify initial character-template materialization and recharacterization;
2. rerun D78;
3. fix provocation identity + rollback source;
4. remove fake `ArchetypeSpec` from character-first seeds;
5. move PreparedMatch controller policy to BrainProfileRegistry;
6. finish Robot v3 off HostCode;
7. resume deletion of the seven remaining archetype rows and old BrainPreset/CharacterRoster roads;
8. continue shrinking Smash generic-kit/capability bridges.

The next checkpoint should have **fewer authorities**, not merely more migrated content.

The branch is on the right trajectory. The important redirect is to close the remaining duplicate template-application architecture before treating D78 as an unrelated determinism mystery.

---

## PROGRESS

| # | Item | State |
|---|------|-------|
| P0 | ONE template materializer, Construction vs Replacement boundaries | ▢ |
| P0 | No `RecharacterizeBody` on ordinary construction (seat, protagonist) | ▢ |
| P0 | Direct invariant test (no reapplication; identity-alone does nothing) | ▢ |
| P0 | Rerun D78 | ▢ |
| P1 | Provocation reads `WornCharacter`, not `sprite_character_id` | ▢ |
| P1 | Provoked policy is a rollback-authoritative BrainProfile source | ▢ |
| P2 | Delete the fake `ArchetypeSpec` from character-first seeds | ▢ |
| P3 | Rename the old `BrainProfileRef` → `BrainPresetRef` | ▢ |
| P4 | `PreparedMatch` off `CharacterRoster` | ▢ |
| P5 | Robot v3 off `HostCode`; repertoire authored, progression gates | ▢ |
| P6 | Shrink `smash_fighter_kit()` / `fighter_abilities` fallback to zero | ▢ |
| P7 | Explicit `character_id` stops falling back to an archetype | ▢ |
