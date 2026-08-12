# Review at 849b628 — architecture is ready to consume the new fighter sprites

Jon relayed this (GPT 5.6) on 2026-08-11 with an explicit steer of his own:
*"I have a GPT 5.6 review and redirect, in this I do want to prioritize getting
the new emmy sprite hooked up."* **Noether (P3) is the priority Jon named.**

⛔ **Jon owns sprite authoring.** Do not edit or redesign the Python sprite
targets, SVGs, rigs, poses, or sprite art. Regenerating/publishing through the
normal tooling is engine work; editing the source is not.

## Progress

| Item | State |
| --- | --- |
| P0 clip-identity resolution (`ClipBinding` → sheet row) without growing `CharacterAnim` | ▢ |
| P1 Robot v3 moves request their exact new rows | ▢ |
| P2 generic fighter-state rows (air_dodge / tumble / knockdown / getup / tech) | ▢ |
| P3 **Noether's new art** | ⛔⛔ **BLOCKED ON JON, and Jon has now asked for it twice.** `noether_gameplay.py` DOES NOT EXIST. Re-checked at submodule `15845f8` (2026-08-11, after the bump): still missing. See below for the four symbols it must export |
| P4 Noether's game-side repertoire | ▢ (after P3) |
| P5 PCA off `cellular_automaton_fighter` + its new rows | ◑ body/policy/moveset authored; ▢ the arm, the row, the clip bindings |
| P6 Patent Clerk / Carl Stargan as non-roster validation | ▢ |
| P7 `PreparedKit::HostCode`, `CharacterRoster` fallback, five rows, `BUILDABLE_ONLY_CAST` | ▢ (ongoing) |
| P8 do not make the rendered frame a simulation authority | standing constraint |

### ⛔ P3 is blocked, and the reason is concrete

`tools/.../targets/characters/noether.py` **fails to import**:

```text
$ .venv/bin/python main.py sheet noether
error: target 'noether' is not registered.
  reason: import failed (ModuleNotFoundError:
          No module named 'ambition_sprite2d_renderer.targets.characters.noether_gameplay')
```

`noether.py`, `noether_effects.py` and `noether_motion.py` are present;
`noether_gameplay.py` is not. Re-checked at `15845f8` — still absent.
**Nothing game-side can consume the new sheet until that import resolves**, and
`npc_noether` is ALREADY on `SMASH_ROSTER`, so the only thing standing between
Jon and seeing her in Smash is this file.

⛔ **it is sprite-authoring content and Jon owns it.** `noether.py` imports four
symbols from it:

```python
from .noether_gameplay import (
    ATTACK_HITBOXES,
    NOETHER_MOVE_BLUEPRINT,
    body_metrics as authored_body_metrics,
    hurtbox_parts_for_rows,
)
```

⭐ **the PCA's `pca_gameplay.py` is the working sibling** — 332 lines exporting
`hurtbox_parts_for_rows`, `body_metrics`, `ATTACK_HITBOXES` and
`PCA_MOVE_BLUEPRINT`, the same four shapes under its own name. It is the template
for what Noether's still needs.

⇒ **the moment that file lands**, `.venv/bin/python main.py sheet noether` is the
whole game-side step, and the identity waiver comes off once the source's
`character_id` says `npc_noether`.

⚠ **and the review's claim about the waiver is half right.** It says the waiver
comment is stale because the target source says `"noether"`. The source does say
that — and the target's own GENERATED sidecar
(`.parity-baseline/noether/noether_actor.ron`) says `character_id: "npc_noether"`,
exactly as the waiver claims. Both are true: **the scanner reads the `.py`
SOURCE, and the source is what disagrees with the catalog.** The waiver stays
until Jon's source says `npc_noether`; its comment should say *source* rather
than implying the sidecar is wrong.

⚠ the shipped `noether_spritesheet.png` is dated 2026-08-08 — the OLD art. So
the new sheet is not merely unbound, it is not built.

---

## The brief

### P0 — consume animation CLIP identity rather than growing `CharacterAnim`

The renderer has moved beyond the runtime's typed pose vocabulary. Full-fighter
sheets carry named rows: `jab`, `attack_up`, `attack_down`, `smash_charge`,
`smash_forward/up/down`, `air_neutral/forward/back/up/down`, `air_dodge`,
`tumble`, `knockdown`, `getup`, `getup_attack`, `getup_roll`, `tech`,
`tech_roll`, `shield_raise`, `block`, `shield_release`, `parry`, `spot_dodge`,
`ledge_attack`, `ledge_roll`, … Robot v3 has 132 rows, PCA 136, Patent Clerk 123,
Carl Stargan 133; Noether reuses the standardized surface plus signature clips.

The runtime collapses many: `air_dodge → Roll`, `tumble → Hit`,
`knockdown → LandHard`, `getup → LandRecovery`, forward smash → `AttackSide`.
That wastes the sheets.

⛔ **do not expand `CharacterAnim` toward the 271-entry catalog.** There is a
better authored seam: `MoveSpec { clip: ClipBinding { clip, fallbacks } }`, and
`MoveSpec` already says its timeline is authoritative for gameplay AND
presentation. When a `MovePlayback` is active, presentation should prefer
`MovePlayback.spec.clip` over the coarse `AttackIntent → CharacterAnim` mapping,
resolved against the character's actual sheet rows, then its authored fallback
chain, and only then the semantic body-pose ladder.

```text
clip = smash_forward, fallbacks = [attack_side, slash]
Robot sheet has smash_forward        → smash_forward
simple character has attack_side     → attack_side
minimal character has only slash     → slash
```

⛔ no hardcoded character ids in the resolver. Build on the existing binding
vocabulary — `AnimRow`, `AnimRowRef`, `BoundAnimRow`, `SheetRecord::anim_rows()`
— not another raw row lookup, and ⛔ do not reintroduce
`row_index_of(&str) … unwrap_or(0)` silent fallbacks. An unresolvable key moves
through an explicit chain and stays diagnosable. `CharacterAnim` remains the
generic semantic vocabulary and compatibility fallback; it stops being the
requirement that every expressive row acquire an engine enum variant first.

### P1 — Robot v3 is the first adopter

`jab → jab`, `tilt_up → attack_up`, `tilt_down → attack_down`,
`smash_forward/up/down → smash_forward/up/down`,
`air_neutral/forward/back/up/down → the matching rows`. Structural fallbacks e.g.
`smash_forward → attack_side → slash`, `smash_up → attack_up → attack_side →
slash`, `jab → attack_side → slash`. ⛔ do not duplicate move timings or combat
geometry — the same canonical move runs in Ambition and Smash; only its visual
clip becomes as expressive as the sheet permits. Tests: the forward smash's
active playback clip is `smash_forward` and the Robot sheet resolves it; a sheet
lacking it but carrying `attack_side` resolves that; missing expressive rows
never change gameplay execution.

### P2 — consume the new generic fighter-state rows

Use exact rows where available: `air_dodge`, `tumble`, `knockdown`, `getup`,
`tech`, `tech_roll`/`getup_roll`, `spot_dodge`/ground roll, shield
transitions/parry, ledge attack/roll/getup. ⛔ **do not infer semantic state from
velocity or sprite appearance.** If the sim only publishes
`getup_invulnerable = true` but needs to distinguish tech / tech_roll / normal
getup / getup roll, publish the actual semantic fact from the subsystem that
already knows which `MovementOp` occurred — presentation reads facts, it does not
reverse-engineer them. Keep fallback chains for lean sheets
(`air_dodge → roll → fall → idle`, `tumble → hit → fall → idle`,
`knockdown → prone → land_hard → hit → idle`, `getup → land_recovery → idle`).
Expressiveness stays opt-in.

### P3 — Noether's new art replaces the old sheet

The catalog already uses `npc_noether` with `sprites/noether_spritesheet.{png,ron}`,
so there is no asset-path or row migration. Once the sprite-side target publishes
corrected canonical metadata: regenerate/publish ONLY the Noether target through
the normal tooling; do not touch the authoring source; remove the identity
waiver; prove the rendered target declares/resolves `npc_noether`; verify the
catalog still points at the generated sheet; the new sheet replaces the old
without a second character identity.

### P4 — Noether's expressive rows and Noether's GAMEPLAY are separate

Her target names `generator_strike`, `conservation_law`, `symmetry_shift`,
`ethereal_lift`, `invariant_field`, `symmetry_break`, `noether_theorem`,
`invariant_parry`. ⛔ **the Python target is not the engine's runtime gameplay
authority.** Sprite submodule owns animation vocabulary, rig/art, pose-specific
visual/body metadata, authored visual hit geometry where the sheet pipeline owns
it. The character definition owns attack timings, damage, launch, movement
capability, `ActionSet`, `MovesetContract`, autonomous profile.
`NOETHER_MOVE_BLUEPRINT` is design input and naming vocabulary, not a second live
combat database. She currently has a peaceful/stand-still catalog-era definition;
if she is to be a Smash character now, author a real repertoire in Ambition
content referencing the new clip names — a small coherent first kit, not every
signature move at once — and she ceases to be a `smash_fighter_kit()` adopter.

### P5 — PCA is the ideal second character after Robot

136 rows, and still tied to `cellular_automaton_fighter`. One combined
D73 + sprite slice: intrinsic body → `CharacterDefinition(perfect_cellular_automaton)`,
decision policy → `BrainProfile`, placement/disposition → context. Then DELETE
the row when its last production adopters are migrated. Give PCA canonical moves
whose `ClipBinding`s select the new rows. ⛔ do not import `PCA_MOVE_BLUEPRINT` as
a runtime combat authority. Rerun the old PCA registration/timing reproduction
once PCA is unconditionally prepared; ⛔ do not preserve the old workaround.
Three wins at once: one fewer `ArchetypeSpec` row, one more real fighter, one more
consumer of the standardized vocabulary.

### P6 — Patent Clerk and Carl Stargan prove the layer is generic

⛔ they do NOT join `SMASH_ROSTER` merely because art exists. Use as non-roster
validation: manifests parse the standardized rows, the generic resolver resolves
them, zero engine source branches. Roster membership stays a content decision.

### P7 — continue D73 cleanup alongside sprite consumption

`PlayableKitSource::HostCode` really is deleted, but `PreparedKit::HostCode {
authored_moveset }` remains the compatibility answer for an incomplete/unknown
character. ⛔ the name is stale and the concept must not be professionalized under
a new compatibility name — migrate repertoires until it has no legitimate
production adopters, then delete it. Correct final state: *a prepared
`CharacterDefinition` has a complete resolved intrinsic kit*, not *complete
characters OR ask host code*. `PreparedMatch` still accepts `CharacterRoster`;
final preparation should use `PreparedCharacterRegistry` + `BrainProfileRegistry`
+ `MatchParticipantRoster`/rules and no enemy body-archetype table. Five rows
remain (`combatant`, `cellular_automaton_fighter`, `medium_striker`,
`gradient_seeker`, `sandbag_infinite`); keep deletion coupled to semantic
migration. `BUILDABLE_ONLY_CAST` is scaffolding — do not extend it into the final
definition of engine constructibility.

### P8 — be careful with sprite-authored hurtboxes

The new sheets carry rich pose-specific hurtbox metadata. ⛔ **do not make the
renderer's current animation frame a simulation authority.** The engine
deliberately separates the gameplay move timeline from the presentation animation
timeline, and rollback/headless simulation must stay deterministic without
rendered frame cadence. Where pose geometry is meant to affect simulation, derive
it from the same semantic gameplay state/move phase that selected the pose,
through the existing authored-body/posed-body architecture. ⛔ never
*whatever PNG frame is showing → authoritative collision box*. For attack
hitboxes the `MoveSpec`/`HitVolume` timeline stays authority unless a deliberate
preparation step imports authored sheet geometry into that contract.

### Immediate execution order

1. Exact animation-row/`ClipBinding` resolution without growing `CharacterAnim`.
2. Canonical Robot v3 moves request their new exact rows.
3. air dodge / tumble / knockdown / getup / tech use exact rows where available.
4. Fallbacks keep working for all old/simple sheets.
5. Once Jon's Noether metadata says `npc_noether`, regenerate/publish and remove the waiver.
6. Give Noether a real canonical repertoire if she is to exercise the new rows now.
7. Migrate PCA off `cellular_automaton_fighter` and bind its moves to its sheet.
8. Continue deleting `smash_fighter_kit()` adopters, `PreparedKit::HostCode`,
   `CharacterRoster` fallbacks, and the remaining `ArchetypeSpec` rows.

### Acceptance demos

**Robot v3** — in Smash, visibly distinguish jab, tilts, F/U/D smash, N/F/B/U/D
air, air dodge, tumble, knockdown/getup using its new rows; the same moves remain
valid in Ambition. **Sparse old character** — one with only `idle`, `walk`,
`slash`, `hit` still plays correctly through fallback, needing no new art.
**Puppy Slug** — forcing it into Smash still does NOT synthesize a repertoire
merely because the resolver knows fighter row names; no attack remains no attack.
**Noether** — `npc_noether` → the new sheet, no second identity, no waiver.
**PCA** — one canonical character with real body, moves, `BrainProfile` and
expressive rows, with `cellular_automaton_fighter` no longer its hidden body.

*The purpose is not prettier animation. It is to prove the new character
architecture lets richer authored content flow into every game without more
per-character engine branches.*
