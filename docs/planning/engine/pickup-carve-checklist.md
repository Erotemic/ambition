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

### ⛔⛔ COUNT THE SHAPE, NEVER THE NAME — the trap every carve's guard walks into

A carve ends with a guard proving the moved systems still sit where they used to.
The obvious guard looks a system up by name. **It cannot work.** Bevy 0.19
strips `system.name()` to a placeholder unless `bevy_ecs`'s `debug` feature is
on, and that feature is enabled NOWHERE in this workspace — so a lookup doing
`format!("{}", system.name())` and `rsplit("::")` matches nothing, and reports it
as *"X must be scheduled by Y"* when X is scheduled perfectly.

⚠ **It has caught three carves in two days**, which is why it is here rather than
in a comment: this session's first attempt, `ambition_encounter_features`'s
`world_gating` and `encounter_spawn_service` tests (both red on a tree where the
systems ARE registered), and the abilities carve's own first draft. Worse, the
failure is composition-dependent — the same test can pass under one `-p` and
fail under `--workspace`, because feature unification differs per build graph —
so "it passed for me" is not evidence.

⇒ **The shape that works**, and both carved crates already use it:

```rust
graph.hierarchy().graph().contains_edge(set_node, NodeId::System(key))  // membership
graph.dependency().graph().contains_edge(a, b)                          // order
graph.system_sets.get_key(Set.intern())                                 // existence
```

Count direct members and assert the number. `ambition_held_items/src/schedule_tests.rs`
and `ambition_abilities/src/schedule_tests.rs` are the two worked examples.

⭐ And assert the ABSENCE too: a carved plugin must not configure a set it does
not own. ⚠ Assert it as absence FROM THE GRAPH, not as an empty set — a set no
plugin has named is not in the graph at all, so `get_key(...).is_none()` is the
check and a member count panics in the lookup instead.

### ⛔ `--all-targets` ON EVERY CRATE THE CARVE TOUCHED — Jon's ruling, not a preference

`maintainer-decisions.md` (2026-08-22): *"the gate stays `cargo check -p
ambition_app --all-targets`, and a carve additionally compiles `--all-targets`
on each crate it touched, because a carve moving a type is exactly the change
that breaks a sibling crate's TEST build — the app gate sweeps production only."*

⚠ **THE OPERATIVE FLAG IS `--all-targets`, NOT `--workspace` — corrected
2026-09-03 after I got this backwards once.** A bare `cargo check --workspace`
builds libs, not test targets, so a carve that moves a type out from under a
sibling's `#[cfg(test)]` import passes it and fails that sibling's own
`--all-targets`. But `cargo check --workspace --all-targets` DOES build them and
would satisfy the ruling — Jon's choice was the per-crate sweep over widening
the gate, for gate cost, not because `--workspace` is incapable. ⇒ Read the
ruling as "every touched crate, `--all-targets`", and do not conclude from it
that a workspace-wide check is useless.

⇒ Derive the list rather than recall it:

```sh
git diff --name-only <base>..HEAD -- crates game | cut -d/ -f2 | sort -u
```

then `cargo check -p <crate> --all-targets` for each. The abilities carve
touched six (`ambition_abilities`, `ambition_app`, `ambition_platformer2d`,
`ambition_platformer2d_actor_monolith`, `ambition_platformer2d_runtime`,
`ambition_sim_view`) plus `ambition_match` from the test move, and running them
individually is also what let the carve be verified at all when the box could
not fit `cargo test --workspace`.

### ⛔⛔⛔ SUPERSEDED 2026-09-03 — the carve campaign's disk bill, and why you may NOT reclaim it

**AGENTS.md's standing rule forbids everything this section used to recommend:**
*"NEVER `rm -rf` anything under a `target/`. NOT `incremental`, NOT `deps`, NOT
"superseded" artifacts, NOT AS A FAVOUR WHEN THE DISK IS FULL … If the disk is
genuinely short after that, SAY SO AND STOP — the reclaim is Jon's call, on
Jon's machine, and `cargo clean` is his to run."*

⚠ **This section was a recipe, and it was followed.** An agent pruning
`target/debug/{deps,examples,incremental}` by mtime on 2026-09-03, with the bind
mount present, was doing what this page told it to. That is the cost of leaving
a contradiction in a document someone reaches for under pressure.

⇒ **What to do instead:** run `scripts/setup/target_bindmount.sh --status`. An
enormous `target/` is almost always an ABSENT BIND, and repairing it returns the
space without deleting anything. If it is bound and genuinely full, report the
numbers and stop.

⭐ **The MEASUREMENTS below are kept, because they are the evidence for how a
carve campaign spends disk** — which is a real planning input, and the reason
"resource-aware lanes" has to mean disk as well as concurrency. Read them as a
description of the bill, not as instructions for paying it.

A carve multiplies the feature-matrix variants a feature job resolves, and cargo
never prunes the last one. Measured 2026-09-03 after five carves in a day:
`target/debug/deps` alone reached **141 GB**, and the box could no longer run its
own suite — the runner's own floor is 40 GB and one `cargo test --workspace`
spends 14 GB in under three minutes.

⛔ **Deleting whole profiles is the expensive way and I did it first.**
`target/{profiling,release,wasm32-unknown-unknown,outlander}` plus incremental,
four separate reclaims, bought ~26 GB and cost a rebuild of each.

⇒ **What was done (⛔ NOT a recipe to repeat — see the rule above):** an mtime
prune of `target/debug/{deps,examples}` past a four-hour window plus the
incremental directories freed 104 GB in one pass, 26 GB → 130 GB. The commands
are in git history for this file; they are deliberately not reprinted here,
because a command block in a checklist is read as a step.

⚠ **CORRECTION, MEASURED THE SAME HOUR: "no full rebuild" was wrong, and the
window is the whole story.** The prune keeps what a build touched INSIDE the
window and drops everything else, so its cost depends entirely on how recently
you built. The next `./run_tests.sh --rust` after this one rebuilt `bevy_ecs`,
`bevy_reflect`, `egui` and the rest from scratch — because the last full build
was older than four hours, `-mmin +240` swept the base along with the variants.
⇒ **Tune the window to your cadence, not to a number copied from here**: a tree
built ten minutes ago keeps its base at `+240`; a tree built yesterday does not,
and there the prune costs about what `cargo clean` costs while freeing less.
⇒ **And the honest reading of that correction is what settles the policy**: the
prune's saving depends on build cadence, it can cost about what a full clean
costs while freeing less, and on a shared box the rebuild is everyone's price
and not just yours. A lever whose cost you cannot predict, on a volume you share,
is exactly the one that should not be pulled without the owner — which is what
the standing rule says.

### ⚠ AUDITING A CARVE'S GUARDS: two shapes are legitimate, so grep by PATH

Audited all eight crates carved or extracted out of the kernel (2026-09-03).
**Every one is guarded** — but two use a different shape, and grepping for
`id = "engine.<crate>-manifest-allow"` reports them as gaps:

* `ambition_registry_core` has `engine.ambition_registry_core-dependency-free`,
  which is stronger — its `[dependencies]` is empty and the row keeps it so.
* `ambition_damage` has source-purity and NO allowlist **on purpose**, and its
  rationale says why: *"its closure is eleven crates … pinning that list is a
  real ownership decision rather than a copy of the `world_items` row — it
  belongs to whoever finishes the damage carve."*

⇒ **Audit by asking which policy names the crate's `Cargo.toml`, not by row id**:

```sh
grep -n "<crate>/Cargo.toml" tests/ambition_workspace_policy/policies/*.toml
```

⚠ I filed both as gaps before reading them. A deliberate absence with a stated
reason is not a missing guard, and an id-shaped grep cannot tell the difference.

### ⛔ RESIDUE IS MORE DANGEROUS TO CITATIONS THAN DELETION

A carve rarely empties a directory. `items/pickup/` survived the pickup carve as
the kernel's schedule residue — the three-variant chain plus a few attachments —
and `abilities/` survived the abilities carve holding possession, teleport,
trapdoor, flyline and the puppy-slug gun. ⇒ **Every planning citation to those
paths still RESOLVES, while the code it meant has moved.** Deletion fails loudly
and is caught; residue passes every checker silently.

Two live examples found the day after (both now repointed):

* `demos/sanic.md` cited the `aabb_path_contacts` swept-route callout as "called
  out in `pickup/mod.rs`" — it is in `ambition_held_items/src/lib.rs`, and the
  `collect_ecs_pickups` beside it is in `features/ecs/pickups.rs`, a third file.
* `awaiting-maintainer-decision.md`'s decision 40 told Jon the zeroing is
  `fire_held_ranged_system` in `items/pickup/mod.rs`; it is in
  `ambition_held_items/src/lib.rs`. ⛔ **That one costs a RULING, not a read.**

⇒ **After a carve, sweep planning for citations INTO the residue**, not only for
citations to files you deleted:

```sh
grep -rnoE '`[^`]*(items/pickup|abilities/|character_runtime)[^`]*`' docs/planning --include=*.md
```

and for each hit ask where the NAME lives now, not whether the path exists. ⭐ A
previous session did this correctly for `physical_baseline.rs` — it names the new
`ambition_body_seed/` location and keeps the old path as `cite-ok` history with
the commit that moved it. That is the shape to copy.

### The lockfiles are plural

`fixtures/minimal_game/Cargo.lock` is the one the footprint ratchet reads, and
`--locked` makes it fail loudly rather than rewrite it — that is the check
working. But `examples/capability_demo/Cargo.lock` resolves the facade too and
has its own gate (`test_sub_workspace_lockfiles_are_current`). Check all of
them; in this carve two of five moved and three did not.
