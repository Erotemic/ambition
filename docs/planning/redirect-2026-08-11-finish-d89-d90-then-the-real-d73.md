# Handoff from aa95b4a4 — finish D89/D90, make the Smash gate truthful, then the real D73

Jon relayed this (GPT 5.6) on 2026-08-11 alongside two direct instructions of his
own, **both of which outrank the text below**:

1. *"if noether gameplay doesn't exist, make it."* ⭐ this authorizes writing
   `noether_gameplay.py` in the sprite submodule — the one file whose absence
   blocks Noether — and it supersedes both the standing *"do not touch sprite
   authoring"* rule and §7 below (*"if that module is still missing … move on"*).
2. *"I also gave a gpt review after you finish noether"* — so **Noether first**,
   then this list in its stated order.

## Progress

| Item | State |
| --- | --- |
| **Jon: make `noether_gameplay.py`** | ▢ **FIRST** |
| §1 D89: separate geometry / locomotion / capability / mode policy | ▢ |
| §2 D89: PCA no-flight in Smash **with body-size parity** | ▢ |
| §3 delete `cellular_automaton_fighter` once parity is proven | ▢ |
| §4 D90: a REAL launched-KO test; Smash 18/18 | ▢ |
| §5 make `ambition_demo_smash_app` an explicit gate | ▢ |
| §6 D73: shrink the `BUILDABLE_ONLY_CAST` / `authored_intrinsics` scaffold | ▢ |
| §7 Noether — see Jon's override above | ▢ |
| §8 the archetype fallback is scaffolding; do not polish it | standing |

⚠ **what this review already agrees with and should not be re-litigated**: the
clip/row architecture is *landed* (§6 of the brief), and `tech`/`tech_roll`/
`getup_roll` wait for the simulation to publish the distinction.

---

## The brief

### 1. P0 — Finish D89 by separating geometry, locomotion, capability, and mode policy

The PCA issue exposed a real modeling defect. **Do not solve it with another
precedence patch.** Four distinct facts that must not be inferred from one
another: sprite/silhouette/collision geometry · baseline locomotion state ·
intrinsic body capabilities · match-specific capability restrictions.

⛔ **`CharacterBodyKind::Floating` must not imply that gravity is disabled or that
a body starts in free flight.** Floating may remain useful as
presentation/gallery/footprint vocabulary, but it is **not locomotion authority**.

The legacy PCA semantics are informative — `is_aerial: Some(false)` with
`can_fly: true` meant a grounded-base HYBRID: the body normally participates in
grounded locomotion but may have a flight capability in contexts that allow it.
**Those facts are not contradictory.**

Jon's product decision is literal: **in Smash, PCA must not have the fly ability.**
⛔ do not globally delete a potentially useful intrinsic PCA flight capability
merely because Smash forbids it. The intended composition:

```text
PCA geometry            preserve its sprite-authored size / silhouette
PCA baseline locomotion grounded, gravity-participating
PCA intrinsic caps      may retain fly/fly_toggle if that is still its authored
                        non-Smash capability, plus blink / shield / dash / attack
Smash rules             explicitly mask/restrict fly and fly_toggle
```

The existing Smash `fighter_abilities` mask already wants to be the mode
restriction seam. **Make that seam actually sufficient**: a body whose flight
capability is forbidden by the match must not nevertheless begin in free-flight
because its catalog `body_kind` happened to be `Floating`.

If sparse authoring needs an `Option<..>` at the SOURCE layer so a definition can
explicitly override a catalog default, that is fine — but a
`PreparedCharacterDefinition` should contain **one concrete resolved answer**, not
ambiguous `None`/fallback semantics.

⛔⛔ **most importantly: changing grounded-vs-flight behaviour must not change
PCA's collision/body size.** The experimental fix shrinking PCA to ~48px is
evidence that geometry and locomotion are coupled somewhere. **Find and remove
that coupling** rather than compensating for the size afterward.

**D89 acceptance:** PCA retains its intended sprite/body-authored collision
dimensions · its baseline locomotion is grounded even though its body-kind
vocabulary says `Floating` · Smash forbids fly/fly-toggle through match policy ·
the fighter brain no longer spends every frame toggling flight · the measured
neutral-fight behaviour returns (shielding/dashing occur, the neutral
attack/defense regression passes). ⛔ do not get there by changing `body_kind` to
manipulate locomotion, and do not add PCA-specific runtime branches.

Once character-first PCA behaviour is equivalent or superior to the legacy path,
**delete the `cellular_automaton_fighter` row and all remaining production
dependencies.** Do not keep two authorities after parity.

### 2. P0 — Close D90 using a real stock-loss path

⛔ **stop investigating why `kin.pos.y = 100_000.0` fails to decrement a stock.**
A raw mutation of `BodyKinematics::pos` is not the semantic operation *"this
fighter lost a stock"*. That another safety mechanism notices the nonsense
position and relocates/restarts the body does not make that relocation a knockout.

The meaningful end-to-end test already exists:
`a_launched_fighter_is_taken_by_the_world_and_spends_a_stock` — a real fighter,
real velocity, the actual blast boundary, and the production chain *launch →
leaves world → `BodyKnockedOut` → `FighterStockSpent` → N → N−1 → ruleset
respawn*. **Extend that, or replace the stale direct-position test with an
equivalent real-KO test.**

One genuine knockout should prove: only the knocked-out fighter loses exactly one
stock · `FighterStockSpent` occurs · the body respawns at the ruleset placement ·
a `BodyRestarted` event/trigger is observable · restart semantics clear stale
maneuver state.

⚠ **observe the published `BodyRestarted` event/trigger.** Do not sample
`BodyLifetime::restart_pending` across `app.update()` calls — it is an internal
one-sim-tick flag and the fixed-tick host may raise and clear it inside one app
update. ⛔ do not make an out-of-world position write manufacture a stock event to
satisfy the old test. If the real launched-KO path decrements a stock but does
**not** publish `BodyRestarted`, that is a production defect and should be fixed;
if it does both, the old test was testing the wrong operation.

**D90 acceptance:** `ambition_demo_smash_app` reaches **18/18 for truthful
reasons**.

### 3. Make Smash part of the actual validation gate

The campaign gate has been proven insufficient: `cargo check -p ambition_app
--all-targets` + `app_it` did **not** run `ambition_demo_smash_app`'s integration
tests, allowing the proving ground itself to stay red. From here, every relevant
change must explicitly run **`cargo test -p ambition_demo_smash_app`** and
`cargo check -p ambition_app --all-targets`, plus the focused tests for whatever
character/movement path changed. **A green Ambition app check is not sufficient
evidence if the Smash suite was not executed.**

### 4. D73 — do not let migration scaffolding become the next monolith

`BUILDABLE_ONLY_CAST` and the giant `authored_intrinsics(id, definition)` match
were useful scaffolding. They are **not** the final architecture. ⛔ do not keep
migrating characters by adding `"character_a" => { .. }` arms to one central
function — that replaces the `character_archetypes.ron` monolith with a
`character_catalog.rs` Rust-match monolith and still requires editing a central
switch per character.

Modifying the existing PCA authoring for D89 is fine; **do not add NEW arms as
incidental work.** After D89/D90 are green, move toward:

```text
provider-owned complete CharacterDefinition → registered → automatically buildable
```

rather than *catalog row + `BUILDABLE_ONLY_CAST` entry + central match arm*.
Definitions may be Rust modules or declarative prepared data — the invariant is
that **the definition itself is the registration/buildability authority**.

The final model stands: `CharacterDefinition` = reusable spawnable template ·
`SimId` = runtime instance identity · `ControllerBinding` = who drives it ·
placement/session = disposition, respawn, team, encounter role. **Fictional
uniqueness is not an engine singleton**: author Fretjaw once, spawn twice, get two
independent bodies with two `SimId`s. And buildability ≠ controllability ≠ Smash
eligibility — **Puppy Slug remains the acceptance example**: universally
registering/building it must not magically grant jump, dash, punches or roster
membership.

### 5. Keep mode policy and character repertoire separate

The Smash ability floor becoming a mask/restriction was the right direction:
**intrinsic capabilities ∩ explicit match restrictions**. ⛔ do not restore the old
behaviour where entering Smash manufactures every verb on every body. Likewise
`smash_fighter_kit()` is scaffolding — **do not broaden it or make it more
sophisticated**; its adopter count should fall as real characters author real
repertoires. Stargan and the Patent Clerk may stay on the roster (an explicit
content decision), but their generic-kit dependence is **debt to remove later, not
a reason to generalize the generic kit**.

### 6. Sprite consumption: consider the current architecture landed

⛔ do not spend this pass redesigning the clip system. `MoveSpec.clip + fallbacks
→ materialized read model → CharacterSheetSpec resolves a row the sheet HAS →
CharacterAnimator plays it → semantic CharacterAnim fallback` is already present,
and human-controlled and autonomous bodies must keep using the same path. Generic
state rows (`air_dodge`, `tumble`, `knockdown`, `getup`) continue to come from
published movement FACTS, never visual/velocity inference. `tech`, `tech_roll`
and `getup_roll` wait until the simulation publishes the distinction — **do not
invent presentation-only guesses.**

### 7. Noether

⚠ **superseded by Jon's direct instruction — see the top of this file.** The
review's position was: inspect the submodule first; if `noether_gameplay.py` is
still missing, record that the generator target is incomplete and move on. Jon
instead asked for the module to be written. What survives from this section: once
the target imports, consume it with the clip-row architecture already landed, and
**resolve source metadata to the canonical game id `npc_noether`** rather than
introducing a game-side alias for a generator-side mismatch.

### 8. Transitional fallback rule

The explicit-character → archetype fallback is migration scaffolding only. ⛔ do
not strengthen it into a permanent API. The final D73 rule is *explicit
`CharacterId` + missing `CharacterDefinition` → composition/authoring ERROR*, not
*→ silently become whatever archetype brain happened to be nearby*. The "no
prepared cast" warning improved diagnosis; **do not polish log wording further** —
as the archetype path disappears, this fallback disappears with it.

### Required order

1. D89 geometry/locomotion/capability separation.
2. PCA Smash no-flight behaviour **+ body-size parity**.
3. Delete `cellular_automaton_fighter` once character-first parity is proven.
4. D90 real-KO restart/stock test; Smash to 18/18.
5. Make `ambition_demo_smash_app` an explicit validation gate.
6. Only then resume D73 structural migration, **reducing** the centralized
   scaffold rather than enlarging it.
7. Noether — per Jon, FIRST rather than last.

### Explicitly NOT acceptable

⛔ fixing PCA by shrinking/changing its collider to fit a locomotion enum ·
using `CharacterBodyKind::Floating` as a synonym for free-flight locomotion ·
PCA-specific runtime logic · globally removing PCA flight because Smash forbids it
(use the match restriction seam) · making `kin.pos = 100000` artificially spend a
stock to rescue the stale test · asserting a one-tick internal flag between
`app.update()` calls when a semantic event exists · adding new characters to the
giant `authored_intrinsics()` switch as incidental work · broadening
`smash_fighter_kit()` · introducing a permanent archetype→character mapping table ·
keeping old archetype rows "for safety" once their real consumers have migrated ·
**declaring success without running the standalone Smash integration suite.**

### Completion report to produce

Exact HEAD · which legacy archetype rows were deleted · where PCA geometry is
authored/resolved · where PCA baseline locomotion is authored/resolved · where its
intrinsic flight capability lives if retained · where Smash removes flight · PCA
body dimensions before/after · D89 neutral-fight measurements after the fix ·
`ambition_demo_smash_app` count/result · whether the stock-loss test observes the
semantic `BodyRestarted` event · whether `BUILDABLE_ONLY_CAST` /
`authored_intrinsics()` grew, shrank or stayed constant · remaining production
callers of `ArchetypeSpec`, `CharacterRoster` and `spec_for_brain`.

*"Optimize for the final engine model, not for making the ledger look shorter. A
migration step is successful when it makes an old authority deletable."*
