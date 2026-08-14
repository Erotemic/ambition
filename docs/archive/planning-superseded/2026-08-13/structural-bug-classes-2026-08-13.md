# The three bug CLASSES found on 2026-08-13, and what actually stopped them

**Written as a seed for the architecture campaign, not as its plan.** Everything
here is a class with instances, a root cause, and a remedy that has already been
applied at least once in this repository — so the campaign can start from
"generalise these" rather than from a survey.

⚠ **the instances are ledger rows D107, D108, D113, D114.** This file is about
what they have in common.

---

## CLASS 1 — a hand-kept list of a type's fields

**Shape.** A comment, a document or a function keeps a subset of some struct's
fields. A field is added to the struct later. The list is not updated, and
nothing says so.

**Instances (all `BodyCombat`, all found in one afternoon):**

```text
  sync_actor_components_from_cluster   carries 5 timers across the rebuild
  boss_component_snapshot              carries the same 5 — copied from that one
  decay_reaction_timers                decays 5
  BodyCombat::reset()                  clears 6
```

`landing_lag_timer` is missing from **all four**, which is D108. The two lists
that DO carry it (`lifecycle_commit`, `room_transition::commit`) are the two that
were edited on the day it was added.

⛔⛔ **the sharpest fact: `decay_reaction_timers` exists *"retiring the two
hand-copied five-line decay blocks"*.** Somebody already noticed the duplication
and consolidated it — and the consolidated version still missed the field.
**Consolidation solves DUPLICATION; only the compiler solves ROT.**

⭐ **THE REMEDY, and the repository already proves it works.** `BodyCombat`'s
ROLLBACK codec lists all twelve fields via `snapshot_pod!`, whose `decode` builds
`Self { $($field: …),+ }` — a struct literal, so a missing field is `E0063`.
**That list has never rotted.** The carry list four files away lost a field.

An exhaustive destructure, never called, gives any hand-kept list the same
property:

```rust
#[cfg(test)]
#[allow(dead_code)]
fn every_field_declares_whether_x(v: &Thing) {
    let Thing { /* ── GROUP A ── */ a: _, /* ── GROUP B ── */ b: _ } = v;
}
```

**Applied today at seven sites**: `ArchetypeSpec` (49 fields, the three
authorities), `ActorTuning` (19), `CharacterDefinition` (27), and the four
`BodyCombat` lists above. Adding a field to any of them is now a compile error
until somebody files it.

⚠ **a group may hold a known defect — label it.** The guard's job is to force a
DECISION, not to assert the decision was right. `// ⛔ DROPPED (1) — not a
design, see D108` is an honest column.

---

## CLASS 2 — a borrow split that became a semantic boundary

**Shape.** Bevy cannot hand two `&mut` queries the same entity, so player and
actor are split into parallel systems. The comments say so outright: *"here to
keep the two queries provably non-aliasing"*. **Each such pair is a place where
one side can implement a body-generic rule and the other can forget to.**

**Instances** — a fact armed body-generically, consumed player-specifically:

| | armed for | consumed by |
|---|---|---|
| D108 `landing_lag_timer` | any body landing mid-move | carried / decayed / gated for the player only |
| D114 `hitstop_timer` | victim AND attacker, "one hitlag law" | player road + a `With<PrimaryPlayer>` clock request |
| D107 `attacking` | — | `With<PlayerEntity>`, so false for every non-player body |

⇒ **in exploration nobody notices**: the player is the only body anyone watches.
**A platform-fighter stage is where it surfaces**, because every fighter is a
body and at most one is the primary player. That is the campaign's own claim
about Smash, arriving as bugs.

⭐ **THE REMEDY, applied twice today**: name the rule as a method on the shared
type and have roads CALL it rather than spell it — `BodyCombat::hard_lock_timer()`
(D108) and `BodyCombat::is_in_hitlag()` (D114). Neither fixes its defect. What
they do is make the gap **greppable instead of inferable**: one road asks the
body, the other never asks.

⭐ **the sweep that bounds the class** (it closed at three):

```
grep -rn 'With<PrimaryPlayer>\|With<PlayerEntity>' -B 6 --include=*.rs \
  | grep -E 'BodyCombat|BodyHealth|MovePlayback'
```

45 player-filtered systems; 3 read body-generic combat state. The rest read
`BodyHealth` for heals/mana, which IS player-specific.

⚠ **the MOVEMENT layer is clean, which bounds where this can live.** Ledge grab
runs inside `movement/mod.rs`, the one kernel both roads step, so it cannot
diverge. The class exists only for state written by the damage path and consumed
by per-road systems.

---

## CLASS 3 — a rule implemented, and guarded by nothing

**Shape.** The behaviour is correct today and one edit restores the reported bug.

**Instance (D113).** Mary-O's two-on-screen fireball rule: `MAX_LIVE_SPARKS = 2`
appeared in exactly two places — its declaration and the gate — with no test, no
ledger row and no campaign row. Three more of Jon's observations were the same:
implemented, unrecorded, and in two cases unguarded.

⛔⛔ **and the guard has a trap this campaign should know about.** My first
version derived every expectation FROM the constant, so it held for any cap ≥ 1 —
and a cap of 1 is what Jon reported. **When an observation is about a NUMBER, a
test parameterised by that number is vacuous against it.** The guard needs both:
the mechanism (it counts live shots) and the value (`>= 2`).

---

## What this campaign should probably NOT do

⛔ **do not fix Class 2 by extracting more shared helpers.** Class 1's sharpest
instance is a shared helper that was extracted for exactly that reason and still
rotted. Extraction moves the list; it does not make the compiler read it.

⛔ **do not trust a pattern that keeps paying out.** I over-fitted Class 2 on its
fifth application and filed a false defect: I claimed `BodyCombat::attacking` was
blind to the moveset road, because that *sounded like* the pattern I had
confirmed four times. It is not — the moveset projects onto `BodyMelee`. The
patch built on it reported a fighter as attacking while it fired a bolt, and its
regression pinned that. Caught in review, reverted the same day.

⇒ **a pattern with a good hit rate is exactly when to keep checking the
premise.**
