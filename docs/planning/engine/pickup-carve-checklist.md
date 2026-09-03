# The pickup carve — an executable checklist

**State:** EXECUTED 2026-09-03 (`ambition_held_items`; queue D33 row carries
the receipt). Written the same day as a spec so the cutter did no design, and
it held — with one addition found by cutting: the `CoreHeldItems` chain
interleaved three KERNEL systems between the domain's steps, which neither §0
nor D33 covered. `HeldItemStep` (`shared_tangle::schedule`) is the answer —
the domain chains its steps as sets, the kernel's systems attach
`.before`/`.after` a step — landed before the move (`4aabf8259`) so the chain
moved intact. Kept as the record of what a carve checklist has to contain.
Every number below was re-derived against HEAD when written; the ones that
move under ordinary work are marked ⚠ RE-DERIVE.

Sources: the pickup row in [`../queue.md`](../queue.md), the D33 rule in
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md), and the
`ambition_world_items` carve that D33 was written from.

## 0. The decision that was blocking this: who owns the schedule

The queue row stopped here deliberately, calling it "a fork with a wrong branch"
and refusing to settle it at the end of a long session. It is settled now, and
the answer is **D33 applied per variant, with one addition D33 does not cover.**

`ItemPickupSet` (`crates/ambition_platformer2d_shared_tangle/src/schedule.rs:416`)
has exactly three variants, and they split cleanly along the carve line:

| Variant | Members today | After the carve |
|---|---|---|
| `CoreHeldItems` | held-item pickup/use/throw, ground-item physics, custody projection | **moves** — new crate configures AND registers it |
| `ThrownItemEffects` | bombs, gravity grenades armed by thrown items | stays in kernel |
| `WieldedAbilities` | wielded abilities, ability-cooldown maintenance | stays in kernel |

So each variant keeps its `configure_sets` and its `add_systems` in ONE crate,
which is exactly what D33 requires.

⛔ **THE ADDITION, AND IT IS THE PART THAT WOULD HAVE BEEN LOST.** The three
variants are not independent — they are `.chain()`ed in a single call at
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:69`: <!-- cite-ok: the pre-cut path, kept as the record -->

```text
(CoreHeldItems, ThrownItemEffects, WieldedAbilities)
    .chain()
    .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
```

A "split the family by variant" reading drops that chain silently, because no
single variant owns it. Per D33's own second sentence — *a set that two crates
order on is `shared_tangle` vocabulary, configured by exactly one owner* — the
inter-variant edge needs exactly one declaring crate, and it must be the
**kernel**: the kernel depends on the new crate (it keeps
`restore_custody_to_checkpoint`, which reads the moved types), so only the kernel
can name both sides. The new crate cannot name the kernel and must not try.

⇒ **The split, stated so it can be executed without re-deciding:**
* **new crate** configures `CoreHeldItems` alone — `.in_set(PlayerSimulation)`,
  `.after(BodyCustodySettled)` (`crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:81`) — and registers its members; <!-- cite-ok: the pre-cut path, kept as the record -->
* **kernel** configures `ThrownItemEffects` and `WieldedAbilities`, registers
  their members, AND declares the three-variant `.chain()`;
* neither crate configures a variant it has no systems in.

⚠ **The consequence to write in a comment, or the next reader will delete it:**
a composition that adds the new crate's plugin WITHOUT the kernel's gets a
correctly-placed `CoreHeldItems` and no chain to the other two. That is right for
a unit test (the other variants are empty there) and wrong for a game. The chain
lives with the plugin that owns two of its three links.

## 1. What the tests actually do — the fork's third branch does not exist

The queue row's third objection was that a carved crate's tests *"register
systems into sets nothing configured, so their ordering assertions pass
vacuously."* Measured against the file that would move,
`crates/ambition_platformer2d_actor_monolith/src/items/pickup/tests.rs`: <!-- cite-ok: the pre-cut path, kept as the record -->

* it holds **93 assertions and not one of them names a set, a phase, or an
  ordering edge** — no `in_set`, no `.before`/`.after`, no `ItemPickupSet`,
  no `PlayerSimulation`, no `BodyCustodySettled` (the single textual hit is prose
  in a doc comment at `crates/ambition_platformer2d_actor_monolith/src/items/pickup/tests.rs:762`); <!-- cite-ok: the pre-cut path, kept as the record -->
* **exactly one** of its dozen-plus `App::new()` tests adds a plugin at all —
  `the_production_plugin_registers_the_custody_release` (`crates/ambition_platformer2d_actor_monolith/src/items/pickup/tests.rs:883`), and it <!-- cite-ok: the pre-cut path, kept as the record -->
  is an ENUMERATION test: it initializes the sim schedule and asserts a system
  name is present.

⇒ There are no vacuous ordering assertions to inherit, because there are no
ordering assertions. The one plugin-building test re-points at the carved
plugin and moves with it. **This is not a reason to relax the schedule split** —
it is a reason the split is cheap, and the guard below is what makes it checked.

⛔ **AND IT IS EXACTLY THE TEST D33 SAYS DOES NOT CATCH THE DEFECT.** An
enumeration test asserts a system EXISTS; the `ambition_world_items` defect
shipped with both systems present and running every frame. So the carve must ADD
what is missing rather than carry this one across and call it covered:

▢ **New guard, in the carved crate, asserting MEMBERSHIP not existence:** build
the carved plugin alone and assert its systems are members of `CoreHeldItems`,
that `CoreHeldItems` is nested in `PlayerSimulation`, and that it carries the
`.after(BodyCustodySettled)` edge. Poison-verify it by deleting the
`.in_set(PlayerSimulation)` line and watching it go red — an assertion that
cannot fail is the failure mode this whole rule exists to prevent.

## 2. The partition

⚠ RE-DERIVE the line numbers; they have already moved twice under ordinary work.
Bound the two stay-behind regions **by name**, never by line:

```text
items/pickup/mod.rs                          1,857 lines (HEAD, 2026-09-03)
  impl Plugin for ItemPickupSimulationPlugin    STAYS   (holds 21 of 23 refs)
  restore_custody_to_checkpoint                 STAYS   (holds the other 2)
  everything else                               MOVES   (names nothing outside itself)
```

Also moving: `pickup/conditions.rs` (zero `crate::` references) and <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
`pickup/tests.rs`. **`pickup/minted_horizon.rs` STAYS** — its one kernel <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
reference is `session::durable_horizon::SaveRestored`, whose own move is a
different job.

## 3. Crate name and manifest

Proposed: **`ambition_held_items`** — the PRESSED half, named to sit beside
`ambition_world_items` (the TOUCHED half) the way the domains already read, and
matching the `CoreHeldItems` variant it owns end to end.

Dependency closure to declare in `engine.<crate>-manifest-allow`, from the
partial cut that was made and deleted: `ambition_entity_catalog`,
`ambition_input`, `ambition_mount`, plus `ambition_portal2d` behind a `portal`
feature. ⚠ RE-DERIVE by compiling — this is the one list a spec cannot fix in
advance, and it was measured on a slightly different partition.

⚠ **The `portal` feature is not optional bookkeeping.** `crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs:184` registers <!-- cite-ok: the pre-cut path, kept as the record -->
`ambition_portal2d::arm_portal_pickups` into `CoreHeldItems` under
`#[cfg(feature = "portal")]`, and a THIRD crate — the content layer's
`AmbitionPortalAdaptersPlugin` — orders its own system `.after(arm_portal_pickups)`
*inside this same set*. The set family already has three registrars. Whoever owns
`CoreHeldItems` inherits that contract.

## 4. Policy rows — three, not two

The queue row says "two policy rows". `ambition_world_items` actually touches
three, and the third is the one a carve forgets because it lives far from the
crate:

1. `engine.<crate>-manifest-allow` — the dependency closure above;
2. `engine.<crate>-source-purity` — roots `crates/<crate>/src`, and it must
   never regain `ambition_platformer2d_actor_monolith`;
3. **`engine.runtime-manifest-allow`** — the runtime composes the carved
   plugin, so the runtime's own allow-list gains the crate, with a rationale
   comment in the same idiom as the `ambition_world_items`, `ambition_mount` and
   `ambition_damage` entries above it.

⛔ Both new rows must be **poison-verified**: adding the monolith to the crate's
source trips purity; adding `ambition_dev_tools` to its manifest trips the
allow-list. A policy row nobody has watched fail is a comment.

⛔ **AND RUN `cargo test -p ambition_workspace_policy`, not `cargo check`.**
These allow-lists are `exact = true`; a correct new dependency turns the suite
red until its row names it, and the compiler cannot see any of it. This cost a
merge-time fix on 2026-09-03 and is the cheapest step on this page.

## 5. The tail

* `scripts/modules_md.py --write` — MODULES.md regen;
* the two sub-workspace lockfiles;
* `scripts/baselines/capability-footprint-baseline.json` — ⚠ the ratchet WILL
  fire, because **it counts crates, not bytes**. The same code was linked before,
  inside the monolith. Declare the growth in the idiom of the existing rows, and
  do not let the row read as a size regression: a carve makes linked code go
  slightly DOWN.
* **Rollback: expect the baseline text file NOT to change.**
  `game/ambition_app/tests/rollback_schema_baseline.txt` keys rows by the
  registrar's owner string and short type name, not by crate path — that is why
  the `ambition_world_items` move left it untouched and only the `use` paths in
  `rollback_registration.rs` moved. ⚠ If it DOES change, something was renamed
  rather than moved; stop and find out what.
* Facade export plus the re-point of the ~20 `actors::items::pickup::` references
  across `ambition_app`'s tests and the smash demo.
* ⛔ **Nothing re-exported from `items/mod.rs`.** No
  `pub use ambition_held_items::…` convenience: a re-export keeps the kernel as
  the discovery path for code it no longer owns, and then the boundary is not
  greppable. Games reach it through the facade.

## 6. The one thing that is not mechanical

`restore_custody_to_checkpoint` stays in the kernel while the types it operates
on leave, so it becomes a kernel system reading a foreign crate's components.
That is legitimate — it is checkpoint policy, not item policy — but it must be
said in its doc comment, or the next reader will "fix" it by dragging it after
the domain.

## 7. Two things the ABILITIES carve added, 2026-09-03

Written after the third carve executed against this checklist. Neither is
pickup-specific; both cost time because the list did not have them.

### ⛔ An exemption keyed by a PATH must move with its file

`engine.velocity-writes-are-authority-only` went red on `grapple.rs:101`
(`kin.vel = pull * GRAPPLE_PULL_SPEED`) the moment the file crossed a crate
line. Its `skip_paths` entry named the old path, and the rationale it carries —
*"a grapple pull is a commanded speed toward the anchor, not a nudge"* — is
exactly as true after the move. ⇒ **A carve that leaves those behind silently
re-arms a rule against code that was deliberately excused**, and the failure
arrives as a policy violation on a line nobody edited, which reads like a new
defect. Grep the policy TOMLs for the moving paths before cutting:

```sh
grep -rn "<crate>/src/<module>" tests/ambition_workspace_policy/policies/*.toml
```

Four entries needed repointing in the abilities carve: a `skip_paths` line, a
`watch_paths`, a `roots`, and a single-`file` rule.

### ⚠ ...and a blanket `sed` over the directory is WRONG when the directory is two things

The obvious repair is to substitute the old directory for the new one. That is
right only if EVERYTHING under it moved. `abilities/` was two families —
`possession`, `teleport`, `trapdoor`, `flyline` and `thrown::puppy_slug_gun`
stayed — so the substitution pointed four rules, including a single-file rule on
`possession.rs`, at paths that do not exist. Caught by reading the rewritten
lines rather than trusting the substitution.

⇒ **The same trap catches PROSE, not just config.**
`authoring-loop-program-2026-07-31.md` cited grapple "alongside blink, dive,
flyline, possession, mark/recall". Repointing the path alone would have left a
sentence that resolves and lies: two of those five did not move. A citation
carries its neighbours with it — re-read the sentence, not only the path.

### The lockfiles are plural

`fixtures/minimal_game/Cargo.lock` is the one the footprint ratchet reads, and
`--locked` makes it fail loudly rather than rewrite it — that is the check
working. But `examples/capability_demo/Cargo.lock` resolves the facade too and
has its own gate (`test_sub_workspace_lockfiles_are_current`). Check all of
them; in this carve two of five moved and three did not.
