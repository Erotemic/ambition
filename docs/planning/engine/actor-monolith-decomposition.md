# Actor residual-kernel decomposition

**State:** OPEN — incremental ownership/dependency work.

Durable decomposition doctrine:
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).
Capability closure:
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

## Goal

Reduce `ambition_platformer2d_actor_monolith` to an honest reusable actor/body
simulation kernel. The objective is **not** a target line count and is **not** a
promised frame-time win.

The current reasons to continue are:

- architectural ownership;
- dependency direction and capability closure;
- compile/change isolation;
- test isolation;
- safer feature work;
- reusable engine packages;
- a public SDK that does not expose historical implementation topology.

Recent runtime measurement found a broad set of individually small systems and
did not establish monolith decomposition as a frame-time optimization. Treat
runtime savings as evidence to measure, not as the campaign rationale.

## Residual kernel target

The final actor package should own the closely coupled simulation that truly
belongs to an actor/body:

- authoritative actor/body state;
- control/intent acceptance and actor-local decision integration;
- movement/contact/body-mode integration;
- actor-local lifecycle semantics;
- narrow action/body integration interfaces;
- construction needed specifically to establish the actor-local simulation
  state above.

It should not retain a concern merely because actors happen to use it.

Strong candidates to live outside the residual kernel include:

- named provider/game content and catalog lookup;
- dialogue/conversation;
- encounters and boss orchestration;
- persistence/session policy;
- menus/UI;
- audio/VFX/presentation;
- developer facilities;
- optional item/projectile/portal/mount capabilities with independent semantics;
- world-authoring/backend integration;
- host/platform composition.

Some of these already have dedicated owners. Prefer completing those ownership
moves over inventing new crates.

## Current measured dependency shape

The last reconciled census on 2026-08-29 distinguished three different numbers:

| measure | value |
|---|---:|
| `ambition_*` lines in the monolith `[dependencies]` table | 28 |
| direct edges in the default resolved graph | 27 |
| full default resolved closure | 34 |

Do not compare these as though they were the same metric. Before a footprint
carve, run `cargo tree -i <dependency>`; another path may already keep the crate
in the product closure.

⛔⛔ **THE "SINGLE PATH" LIST IS STALE — RE-MEASURED 2026-08-31.** The census
named four monolith dependencies as reaching the closure through one path and
therefore able to shrink it:

| dependency | dependent crates today |
|---|---:|
| `ambition_dev_tools` | 6 |
| `ambition_mount` | 6 |
| `ambition_items` | 5 |
| `ambition_damage` | 3 |

None is single-path. And the stronger statement, which retires the rationale
rather than just the number: **`ambition_platformer2d` — the facade every game
depends on — depends on all four directly** (`items` optionally). So removing the
MONOLITH's edge to any of them cannot shrink a product's closure at all, however
the census is counted.


⛔⛔ **AND THE FIRST ROW DID NOT MOVE TONIGHT WHILE ITS MEMBERSHIP CHANGED
COMPLETELY — re-counted 2026-09-02.** The monolith's `[dependencies]` table
still holds **28** `ambition_*` lines, exactly what the 2026-08-29 census
recorded, and a reader would conclude nothing happened. Two things happened and
they cancelled: `ambition_dev_tools` left for `[dev-dependencies]` (the D33
developer-state carve) and `ambition_world_items` arrived (the touched-collectible
carve). ⇒ **A COUNT THAT IS UNCHANGED IS NOT EVIDENCE THAT THE SET IS
UNCHANGED**, and this table is the third metric tonight to say crates rather than
what was done to them — the capability-footprint ratchet (43 → 44, counting
crates while the linked code went down) and the `crate::features` reference
counts (27% of which named other crates' types) were the other two.
⚠ The monolith now carries SIX `ambition_*` dev-dependencies —
`dev_tools`, `platformer2d_ldtk`, `characters`, `boss_encounter`, `dialog`,
`sim_view` — and only the production line count is in the table above. A carve
that moves an edge from production to dev is invisible here by construction, so
say which table you mean when you quote "28".

⭐ **SO FOOTPRINT IS NOT A REASON TO CARVE THESE FOUR.** The remaining reasons are
the ones this doc already leads with — ownership, dependency direction, compile
and test isolation — and a slice that promises closure movement for one of them
is promising something arithmetic forbids. Re-run
`grep -rl "^<dep>" crates/*/Cargo.toml game/*/Cargo.toml` before citing any
single-path claim; it is cheaper than `cargo tree` and it is what went stale.

### ✔ `ambition_dev_tools`: the kernel no longer reads or writes developer state

Closed 2026-09-02, in four slices, and carved for OWNERSHIP rather than
footprint (the table above forbids the footprint claim). ⚠ THE COUNT OF WHAT WAS
LEFT WAS RESTATED THREE TIMES AND WRONG TWICE — read the trail below rather than
any one of its numbers, because each was written as if it were the last. The
kernel's production `[dependencies]` edge to this crate is gone; the
`[dev-dependency]` the `#[cfg(test)]` fixtures use is not, and is not meant to
be. What follows is in the order the references fell:

- **the WRITE.** `cleanup_timers_system` decayed `dev_state.preset_flash`, a
  developer HUD timer, and that one line was the only reason the control module
  held a `ResMut<DeveloperRuntimeState>`. Now
  `ambition_dev_tools::decay_developer_presentation_flash`, in the SAME schedule
  — its old home ran in `PresentationSync` so presentation timers decay while
  gameplay is suspended, and `Update` counts a different clock under rollback.
- **the READ.** `update_time_scale_requests` read `dev_state.slowmo` at rung 4 of
  a five-rung ladder, twice. ⭐ THE INVERSION WAS ALREADY BUILT AND UNUSED:
  `ClockRequester::DevTool` exists, `RegimePolicy` grants it in `Solo` and denies
  it in `RLDeterministic`/`Cinematic`, and `apply_clock_scale_requests` reduces by
  `min`. The dev crate publishes its own `ClockScaleRequest`; the ladder lost both
  rungs. ⚠ `min` IS NOT A LADDER: bullet-time used to outrank slow-motion by
  sitting at rung 2, and now the stronger slowdown wins. Stated, not hidden.
- `debug_slowmo_scale` moved with the rung. It was a developer number in
  `Platformer2dFeelTuningMonolith`, whose own module doc says those values *"are
  gameplay parameters rather than developer-tool state"*, and nothing but that
  rung read it.
- ✔ **The two `profiling::phase_mark` calls in `audio/plugin` are gone
  (2026-09-02).** Instrumentation can live anywhere, and a simulation crate
  naming a profiler to describe itself is the thing this carve is about. The
  plugin publishes `AudioInitSet`; the host brackets it in `app/plugins.rs`
  beside every other `phase_mark`, under the same two names so existing startup
  profiles still compare.
- ⛔⛔ **BUT "WHAT IS LEFT" WAS WRONG, and the carve is not one step from done.**
  Counted 2026-09-02: the monolith still reads `ambition_dev_tools` from THREE
  more production paths, and they are not instrumentation — they are the
  simulation reading developer state, which is exactly what this carve exists to
  remove:
  - ✔ `features/npcs.rs` — CLOSED 2026-09-02. `forced_profile()` / `forced_preset()`
    were chosen while building a live brain; the sim reads a session-owned
    `AuthoredBrainOverride` the dev tool writes, the two `OnceLock`s are deleted,
    and `for_room_construction` takes the authority as a parameter so no road can
    forget it;
  - ✔ `features/ecs/spawn_static.rs` — CLOSED 2026-09-02 evening. The quota is
    `ActorAdmission` spent at plan time in `prepare` (a refused NPC plans no row), the
    value is a published `AuthoredPopulationCap`, `for_room_construction` takes
    it as a parameter, and the dev crate keeps only the env name and parse;
  - ✔ `features/mod.rs:350` — `runtime_census` — CLOSED 2026-09-02, and ⛔ THE
    COUNT IN THIS PARAGRAPH WAS ALSO WRONG. "THREE more production paths … they
    are not instrumentation" undercounted by one and mis-described the
    remainder: after the two above closed, the survivors were **TWO** and BOTH
    were instruments. The uncounted one was `features/ecs/actors/update.rs:606`,
    `perception_census::note_world_view`, called inside the `build_world_view`
    loop — a hot path, which is why it could not simply be observed from
    outside.
  - ✔ `features/ecs/actors/update.rs:606` — CLOSED with it, the same day.

⭐⭐ **AND THE EDGE IS GONE: `ambition_dev_tools` IS A `[dev-dependency]` OF THE
KERNEL NOW.** Both instruments moved by publishing the thing DOWNWARD, which is
the third option beside the two this doc had been treating as exhaustive (delete
the measurement, or move `phase_mark` down) — the same inversion `AudioInitSet`
made one bullet up:

- `ActorDecisionSet` moved from `pub(crate)` in `features/mod.rs` to
  `shared_tangle::schedule`, beside `WorldPrepSet` and `PlayerInputSet` — which
  the census-boundary function was ALREADY importing from there, so the enum was
  the only reason the marks could not live with the instrument.
  `runtime_census::install_sim_phase_boundaries` now installs all seven beside
  its other twenty, under the same `if enabled` gate and the same "an instrument
  must not join the population it measures" rule. ⛔ The kernel still CONFIGURES
  the chain: `configure_actor_decision_phases` and every ordering assertion in
  `actor_decision_phase_tests` stayed, because WHERE the sets sit is the
  simulation's business. What left is the developer registration.
- `perception_census` (91 lines) moved to
  `ambition_characters::perception::census`, the module that defines the
  `WorldView` it counts. The developer crate still calls `enable` where it
  installs its rows and `drain` where it prints them, so the POLICY did not
  move — only the counter.

⭐ **AND THE INSTRUMENTS WERE RE-MEASURED FROM THEIR NEW HOMES, because a census
that COMPILES is not a census that BILLS.** Moving a boundary mark is exactly the
change a type-checker cannot judge: every `.after`/`.before` still names a real
set, and the marks could still fire in the wrong order, into the wrong bucket, or
not at all. `AMBITION_PROFILE_CENSUS=1 capture_scene --route ambition_gameplay`
reports all seven decision buckets alive — `WorldPrep.Decision.Gate`,
`.Targeting`, `.Prepare`, `.Observe`, `.StateMaintenance`, `.Decide`, `.Publish`
— with `unmeasured=Trace` and no never-closed line, and the perception row
(`views=N offered=… kept=… kept_max=…`) reports from `ambition_characters`.
⚠ That run is the HUB, not the hall, so the magnitudes are small and are not
comparable to the 0.958 ms/tick hall figure above; what it certifies is that
every mark fires and lands in its own bucket, which is the property the move put
at risk.

⛔ **THE GUARD IS THE MANIFEST, not a test, so it cannot rot.** A production
`use ambition_dev_tools::…` anywhere in the kernel's `src/` no longer compiles.
Poison-verified by adding one: `error[E0433]: cannot find module or crate
`ambition_dev_tools` in this scope`. The `#[cfg(test)]` live-refresh and reset
tests keep reaching it, which is what a dev-dependency is for.

⚠ **AND IT BUYS NO SMALLER CLOSURE, which the table above already forbade
claiming — re-measured after the carve rather than asserted.** Six manifests
name `ambition_dev_tools`, and after this slice exactly one of them
(`ambition_platformer2d_actor_monolith`) says `[dev-dependencies]`. The other
FIVE are production and include the facade every game depends on:
`ambition_platformer2d`, `ambition_render`, `ambition_sim_view`,
`ambition_platformer2d_provider`, `ambition_platformer2d_runtime`. ⇒ Nothing
downstream drops the crate, and the shipped closure is unchanged. The payoff is
ownership and a boundary the compiler holds.

⛔⛔ **AND IT WAS THE LAST MISPLACED REGISTRATION — SO THE "NEXT SLICE FROM THE
DOMAIN FRONTIERS" PLAN POINTS AT NOTHING.** Measured 2026-09-02 late, after this
carve closed. The frontier sizing that recommended it counted REFERENCES
(`ambition_encounter` 74/24 files, `ambition_mount` 69/17,
`ambition_conversation` 48/13, `ambition_items` 40/17 — non-comment), and reading
what those references ARE gives the same answer this doc already reached for
`sfx`/`vfx`/`audio`: they are components the kernel stores (`MountSlot`,
`RidingOn`, `Mounted`), types it matches (`SwitchActivation`, `EncounterSpec`)
and helpers it calls. **Downward vocabulary consumption, which is correct.** A
reference count cannot tell that from an authority read — which is how it
produced a four-item worklist with nothing on it, against this doc's own rule
that footprint is retired as a rationale.

⭐ **THE SWEEP THAT DISCRIMINATES is "who does the kernel register plugins FOR?"**
— the smell both the dev-tools and audio slices turned out to be. There are FIVE
foreign plugin registrations in the whole kernel and every one is accounted for:
`ConversationPlugin` plus five `NarrativeInputPlugin::<T>` (deliberate, and
`FeatureInteractionSchedulePlugin` states why in place — three of the payloads
are `features` types a carved-out conversation crate could not name);
`ambition_characters::brain::BrainPlugin` (brains are the kernel's subject); and
three `ambition_audio` plugins inside the kernel's own `Platformer2dAudioPlugin`,
which the host adds and the `audio` feature removes wholesale.
⚠ COUNTED TWICE, because one form of the grep is not enough: matching
`add_plugins(ambition_x::…)` misses anything imported by `use`. Sweeping every
`*Plugin` inside an `add_plugins(…)` raised two more candidates that both
dissolved — `CharacterCatalogPlugin` occurs only in `character_roster/tests.rs`,
and `PhysicsPlugin` was a substring of avian's `PhysicsPlugins::default()` inside
`AmbitionPhysicsPlugin`.

⇒ **WHAT IS ACTUALLY LEFT IS INTERNAL** — the kernel's own `items/` module,
~6,000 lines, named `items::` 79 times by the rest of the kernel.

⛔⛔ **AND THE PARAGRAPH THAT USED TO END HERE WAS WRONG WITHIN THE HOUR.** It
said that carve had *"no manifest edge to delete at the end of it, and no
compiler-held guard like the one this slice earned … every previous slice
finished with an edge to point at; that one will not."* ⭐ A first slice landed
the same evening and finished with BOTH: `ambition_world_items` is a new crate
with two poison-verified policy rows. The mistake was reading `items/` as one
mass because the sentence above it lists the UNION of every file's imports —
counted per file, `pickup/mod.rs` holds 27 of the module's 51 references into
the rest of the kernel and `item_motion.rs` holds none. ⇒ **"Internal" was a
property of ONE FILE in that module, not of the module.**

### ✔ `ambition_world_items`: the touched collectible left the kernel (2026-09-02)

`world_item.rs` + `item_motion.rs` and their 14 tests — the physical life of a
collectible: where it is, whether it is moving, and that walking into it
collects it. All 14 pass in the new crate and the monolith went 1221 → 1207,
which is exactly them.

- ⛔ **THE SPLIT IS BY COLLECT TRIGGER, NOT BY SIZE.** `items::pickup` keeps the
  PRESSED pickup — a held weapon taken with `Attack` — and its reach into
  `abilities`, `ability_cooldown`, `construction` and `shrine`. That is the line
  the pickup module's own `AMBITION_REVIEW(discrete_ok)` note had already drawn,
  years of comments before anyone carved along it.
- ⭐ **THE APPARENT BLOCKER WAS THE LEVER.** The collect pass named
  `features::ecs::pickups::TouchCollectorFilter`, a type alias composed of
  nothing but `PlayerEntity` and `TemporaryControl` — both already in
  `shared_tangle`. It moved down beside them, and so did its VALUE twin
  `body_collects_on_touch`; the kernel re-exports both under the short names its
  three passes read. ⛔ ONE definition, not a copy: the filter decides who a
  query RETURNS and the value check decides whether a returned body collects, so
  a second copy is how the two halves come to disagree.
- ⛔ **NOTHING IS RE-EXPORTED FROM `items/mod.rs`.** Keeping the module paths
  alive there would have meant zero consumer churn and was refused: a re-export
  keeps the kernel as the discovery path for code it no longer owns, and the
  boundary stops being greppable. Games reach it through the facade's new
  `ambition_platformer2d::world_items` for the same reason — `actors` IS the
  monolith.
- The runtime composes `WorldItemSimulationPlugin` beside
  `ItemPickupSimulationPlugin`, so no registration for the domain lands back in
  the kernel — the shape `ambition_mount` and `ambition_damage` established.
- ✔ **The rollback ledgers did not churn**, and the check was worth doing rather
  than assuming: both types are registered, but
  `rollback_schema_baseline.txt` keys its rows by the registrar's OWNER STRING
  and short type name (`entity:world_item`, `item.motion`, `item.world_item`),
  not by crate path. Only the `use` paths in `rollback_registration.rs` moved.
- ⛔⛔ **The footprint ratchet fired: 43 → 44**, declared in the baseline in the
  `mount`/`damage` idiom. ⚠ It counts CRATES, not bytes — the same code was
  linked the day before inside the monolith and the linked code went slightly
  DOWN — and that sentence now also sits on
  [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
  beside the number, because a queue row there has *"the count falls"* as its
  acceptance and a carve raises it by construction.
- ⭐⭐ **AND THE SIBLING IS NOT MULTI-DAY EITHER, measured the same night.**
  `items/pickup/mod.rs` holds 27 of the module's 51 kernel references, which is
  why it was called the hard part. Split at its `impl Plugin` block: the plugin
  is **201 lines and holds 21 of the file's 23 cross-module references**; the
  other **1,659 lines hold 2**, both at one seam
  (`construction::authored_occurrence_request` and
  `ActorConstructionParams::GroundItem`, spawning a ground item from an authored
  occurrence). Every reference in the plugin is a SYSTEM NAME being scheduled —
  `abilities::{ranged ×10, traversal ×4, thrown ×3}`, `shrine` ×3,
  `construction` ×2, `ability_cooldown` — not a call the pickup logic makes.
  ⇒ **The entanglement is scheduling, not logic**, and it is the same shape the
  `world_items` slice solved in miniature: the systems it moved were registered
  inside the file it was carving, so the split had to take the registrations too.
  ⛔ **THREE SIZING ERRORS IN THIS ROW'S HISTORY NOW SHARE ONE SHAPE** — a count
  taken at the wrong granularity: a module's UNION of imports read as one file's,
  a re-exported name read as kernel coupling
  (`scripts/measure_facade_reexport_coupling.py`: 27% of `crate::features::X`
  uses name a type defined elsewhere), and a plugin's system list read as its
  domain's dependencies. ⇒ **before sizing a carve, ask what granularity the
  number was taken at and whether the thing being counted is code the domain
  RUNS or code it merely SCHEDULES.**

- ⚠ **AND THE SIZING GREP UNDERCOUNTED ITS OWN SUBJECT.** `world_item.rs` was
  sized at TWO kernel references with `grep -o "crate::[a-z_]*"`, which sees
  neither `super::` paths nor a fully-qualified call; the file also reached
  `super::item_motion` four times and called `pickups::body_collects_on_touch`.
  ⇒ the same one-form-grep error recorded two sections above, repeated by its
  author four hours later. Everything resolved downward so the slice held, but
  the number was produced by an instrument blind to half its subject.

⭐ **THE COST WAS ONE POLICY LINE, and it is the honest kind.**
`engine.ambition_dev_tools-manifest-allow` gained `ambition_time`, with the
reason in its rationale: the dep exists so the SIMULATION stops reading developer
state, and `ambition_time` depends only on `ambition_platformer2d_core`, so it is
foundation rather than sim. ⛔ a carve that needs an allowlist widened should say
what the widening buys, in the allowlist.

⛔ **AND REGISTRATION NEEDED AN APP-LEVEL GUARD BOTH TIMES.** A moved system's
behaviour can be proven in its new crate; that anything RUNS it cannot be —
`DevToolsSimPlugin`'s siblings need resources `ambition_dev_tools` does not
depend on, so its schedule will not run in a bare `App`, and bevy names every
system `<Enable the debug feature to see the name>` without a feature that crate
will not enable for a test. Both guards live in `app_it` and both go red when the
system is unregistered.


### ⛔⛔ REGRESSION: `ambition_world_items` LOST ITS SIMULATION PHASE (2026-09-02)

**Live on `main` as of `69641a83f`. Found by Jon's GPT review of `dc8ea607a`, not
by me, and my own comment claimed the opposite.**

Before the carve, both systems ran inside `ItemPickupSet::CoreHeldItems`, which
`ItemPickupSimulationPlugin` configured as:

```rust
ItemPickupSet::CoreHeldItems
    .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
    .after(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled)
// and each system additionally .in_set(GameplayGated)
```

`WorldItemSimulationPlugin` registers the chain with **`.in_set(GameplayGated)`
and nothing else**. Two facts were lost:

1. **PHASE MEMBERSHIP.** `GameplayGated` does not imply
   `GameplaySimulationRoot`/`PlayerSimulation`, so the systems no longer sit in
   the simulation phase that authorizes them for a session.
2. **THE CUSTODY EDGE.** `.after(BodyCustodySettled)` is gone, so a collect can
   observe a hand mid-settle.

⛔ **AND THE COMMENT I WROTE ON THAT PLUGIN SAYS THE ORDERING WAS PRESERVED.** It
argues at length that step-before-collect is load-bearing — which it is, and
which survived — while silently dropping the two facts that were expressed as SET
MEMBERSHIP rather than as a chain. ⇒ **Preserving the systems' order is not
preserving their schedule.** A carve must move the set's `configure_sets` rules,
not only the `add_systems` line; see
the `ambition_world_items` section above, where the same lesson is stated as
*"move the files AND the registrations that belong to them"* — the registrations
included a `configure_sets` nobody looked at.

#### The seam to build (specified here, implemented elsewhere)

⭐ `ItemPickupSet` is in `ambition_platformer2d_shared_tangle::schedule` now,
which is what makes this expressible without the new crate naming the kernel.

- **`WorldItemSet`** joins it in `shared_tangle::schedule`, three variants in
  chain order: `Motion` → `PreCollect` → `Collect`.
- `WorldItemSimulationPlugin` configures, once:
  `(Motion, PreCollect, Collect).chain()`,
  `.in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)`,
  `.after(lifecycle::BodyCustodySettled)`, and each variant
  `.in_set(GameplayGated)`.
- `step_item_motion` in `Motion`; `collect_world_items` in `Collect`.
- **`PreCollect` exists for a real customer, and it fixes a LATENT bug rather
  than a style problem.** Mary-O's `refuse_a_weaker_form_pickup`
  (`ambition_demo_mary_o/src/lib.rs`) is `.before(collect_world_items)` — naming
  a concrete FUNCTION in another crate, which is the thing `ItemPickupSet`'s own
  doc says not to do. ⚠ AND ITS COMMENT EXPLAINS A CHOICE THAT IS ONLY
  CONDITIONALLY RIGHT: *"Registered on `Update` beside `collect_world_items`
  rather than in the sim set, so the ordering edge is real (a cross-schedule
  `.before` is silently vacuous)."* The mechanism is stated correctly and the
  conclusion holds only for ONE of the three hosts — `collect_world_items` is
  registered into `app.sim_schedule()`, which is `Update` under
  `SimulationHost::RenderFrame` (the `#[default]`, and what Mary-O runs) but
  `FixedUpdate` under `Fixed60Hz` and backend-selected under `Rollback`. ⇒ The
  edge binds today because of the HOST, not because of `collect_world_items`,
  and it goes silently vacuous the day that demo gains a rollback or fixed-tick
  host. Moving the system into `PreCollect` makes it host-independent, which is
  the actual reason to do it.

#### The two regressions each need a guard, and both must be poison-verified

1. **PHASE**: a test that the world-item systems are members of
   `Platformer2dSimulationPhaseMonolith::PlayerSimulation` in a composed App —
   red when the plugin registers only `GameplayGated`. ⚠ Assert MEMBERSHIP, not
   "the systems exist": the bug shipped with both systems present and running.
2. **CUSTODY EDGE**: an ordering assertion that the collect runs after
   `BodyCustodySettled`. ⛔ A behavioural test is better if one is cheap — the
   failure is "a collect observes a hand mid-settle", which an ordering
   assertion pins only structurally.

⚠ **DO NOT "FIX" THIS BY PUTTING THE SYSTEMS BACK IN `ItemPickupSet`.** That set
belongs to the PRESSED pickup; sharing it was an accident of where the code used
to live, and the carve's whole point is that touched and pressed are siblings
rather than one thing.

## What recent carves taught

Several completed slices established rules that should guide future ones:

- A forwarding/re-export edge can be worth deleting even when total closure does
  not move; it makes ownership honest and prevents future callers from learning
  the wrong path.
- Gating one obvious dependency does not help footprint when another carved
  domain re-supplies it. Count all suppliers first.
- A domain should publish a semantic event/fact and let an optional presentation
  or orchestration consumer translate it rather than importing the optional
  domain downward.
- Domain-owned rollback registration is no longer a reason for generic runtime
  or actor packages to know each concrete gameplay component.
- Moving files without moving the authority, plugin registration and dependency
  edge is not a carve.


⭐ **THREE MORE, FROM THE 2026-09-02 CARVES (dev-tools, world-items, and the
`features` re-export sweep):**

- **WHEN A CARVE LOOKS BLOCKED BY A TYPE, MOVE THE TYPE DOWN.** Three separate
  blockers dissolved this way in one day: `ActorDecisionSet` was `pub(crate)` in
  the kernel so only the kernel could install the census marks that bracket it;
  `TouchCollectorFilter` was in `features::ecs::pickups` so only the kernel could
  run the touch-collect pass; `ItemPickupSet` was in `items::pickup` so three
  packages outside the monolith had to name the kernel to schedule their own
  systems. All three are composed of things that already lived lower, and all
  three moved to `shared_tangle` in minutes. ⇒ **ask what the blocking type is
  MADE OF before designing an abstraction to get around it.**
- **A COUNT THAT DID NOT MOVE IS NOT A SET THAT DID NOT CHANGE.** The monolith's
  `[dependencies]` still holds 28 `ambition_*` lines, exactly as on 2026-08-29 —
  because `ambition_dev_tools` left and `ambition_world_items` arrived on the
  same evening.
- **SIZE A CARVE AT THE GRANULARITY YOU WILL WORK AT.** Three sizings in this
  document were wrong the same night in the same way: a module's UNION of imports
  read as any one file's, a re-exported name read as kernel coupling (27% of
  `crate::features::X` uses named a type defined elsewhere), and a plugin's
  system list read as its domain's dependencies. You carve files and you split
  files at their plugin block, so count there.

The detailed sequence of LDtk, conversation, boss, mount and other historical
carves is available in git history and should not be reconstructed here.

## Slice selection

Before starting a carve, answer:

```text
What authority is moving?
Who owns it after the move?
What systems/resources/messages move with it?
What dependency edge or change-fanout path should disappear?
Does another path keep the dependency in product closure?
What small App or consumer can test the new owner?
What old facade/import/registration can be deleted?
```

A good slice normally satisfies more than one of:

1. removes a direct monolith dependency;
2. shrinks a minimal consumer's resolved capability footprint;
3. removes duplicate/shared authority;
4. isolates a meaningful compile/test unit;
5. deletes a compatibility/re-export path;
6. creates a coherent domain plugin with an independent test surface.


⛔⛔ **CRITERION 2 IS IN TENSION WITH CARVING, AND A ROW ELSEWHERE USES IT AS
ACCEPTANCE.** *"Shrinks a minimal consumer's resolved capability footprint"* is
measured by `scripts/baselines/capability-footprint-baseline.json`, which counts
**CRATES the sentinel links, not code**. A carve moves code out of the monolith
into a new crate that the monolith still names, so the count goes UP by
construction: `ambition_mount`, `ambition_damage` and now `ambition_world_items`
each RAISED it, 43 → 44, while the code linked went slightly down.
⇒ **A slice cannot satisfy criterion 2 and be a carve.** The two are different
programs: criterion 2 is satisfied by making a facade edge OPTIONAL or deleting
it (`ldtk_left_the_closure_2026_08_22`, `settings_menu_left_the_closure_2026_08_22`),
which moves no code at all. ⚠ Read them as alternatives on this list, never as a
score to maximise together, and see
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
where the same caveat now sits beside the number.

## Current frontier

Prefer outside-in ownership work before splitting the central actor kernel.
Useful frontiers are:

### Developer and presentation dependencies

Developer tools and presentation facilities are poor reasons for the simulation
kernel to depend upward. Remove them when the consumer can observe semantic
facts through an existing engine seam.

⭐⭐ **RE-MEASURED 2026-08-31, AND THE PRESENTATION HALF OF THIS FRONTIER IS
NARROWER THAN IT READS.** The kernel's production references to presentation-ish
crates are:

| crate | refs | files | what it is |
|---|---:|---:|---|
| `ambition_sfx` | 160 | 35 | a cue MESSAGE vocabulary + writer, 1 ambition dep |
| `ambition_vfx` | 103 | 31 | the same shape, 3 deps |
| `ambition_audio` | 70 | 7 | channel/registry types, 2 deps |
| `ambition_conversation` | 33 | 9 | 8 deps |
| `ambition_cutscene` | 16 | 2 | script + stepper, 1 dep |

⛔ **NONE OF THEM PULLS A RENDERER.** No `bevy_render`, `bevy_audio`, `bevy_ui`,
`bevy_sprite` or `bevy_text` anywhere in that column, so the closure argument
that would justify carving them does not exist — they are FOUNDATION vocabulary
crates the kernel consumes DOWNWARD, and a body emitting its own hit cue is the
semantic fact, not a presentation reach.

⚠ `ambition_dialog` and `ambition_sim_view` are DEV-DEPENDENCIES here, so they
are not production edges at all; a count that read the manifest without its
section headings would report two edges that do not exist.

⛔ **AND `cutscene.rs` ALREADY CARRIES ITS OWN DEFENCE**, which a carve should
answer rather than repeat: *"These systems are gameplay-coupled (rooms, save,
schedule) so they live here rather than in `ambition_cutscene` — which sits below
this crate and must stay content- and gameplay-free."* Moving it needs a THIRD
crate above both, which is a cost with no measured benefit behind it.

⇒ **The developer half of this frontier is done and the presentation half is
mostly a mirage.** Pick the next slice from the domains below, not from here.

### Items, mounts and other optional gameplay domains

Move vertically: the domain owns its state/messages/plugin, the actor kernel
exposes the body/action hooks it needs, and the old actor-side implementation is
deleted. Do not replace one central switch with another.

⭐⭐ **AND THE ITEMS HALF IS DEFENDED TOO — measured 2026-08-31 before opening
it.** `src/items/` is 6580 lines against `ambition_items`' 1975, which reads like
a domain sitting in the wrong crate. It is not:

* 1864 of those lines are `pickup/tests.rs`, plus ~270 more in sibling test
  modules — about **4.4k lines of production code**, not 6.6k;
* `ambition_items` already owns what the doc asks a domain to own — the 24-slot
  catalog, owned-item state, the shop, and the `item_catalog` content schema;
* the kernel's half says what it is in its own first paragraph: *"The
  pickup/throw/projectile steppers stay here because they mutate actor bodies,
  gravity, portals, abilities, and hit events."*

⇒ **a stepper that mutates bodies belongs with bodies.** Moving it is not a move:
this frontier's own instruction — *"the domain owns its state/messages/plugin,
the actor kernel exposes the body/action hooks it needs"* — means DESIGNING those
hooks first, generic enough that an item crate can drive a pickup without naming
gravity, portals, abilities and hit events. That is a design slice with a real
budget, and nothing above measures it.


⛔⛔ **THE DEFENCE ABOVE IS OVERTURNED — measured 2026-09-02, and one half of it
was right for the wrong reason.** It rests on a sentence from `items/mod.rs`'s
own module doc: *"The pickup/throw/projectile steppers stay here because they
mutate actor bodies, gravity, portals, abilities, and hit events."* That
sentence is a COMMENT, and the conclusion drawn from it — that a carve needs a
generic hook design first — does not survive reading the code.

✔ **WHAT THE COMMENT GETS RIGHT:** the pickup code really does touch those
things. Counted in the 1,344-line remainder of `pickup/mod.rs`: portal 36
mentions, gravity 33, ability 10.

⛔ **WHAT IT GETS WRONG IS WHERE THEY LIVE.** Every one of them is reached
through a crate BELOW the kernel, not through the kernel:

```text
gravity   -> ambition_platformer2d_shared_tangle::gravity::{GravityCtx, GravityField, apply_world_forces}
portals   -> ambition_portal2d::PortalGun            (#[cfg(feature = "portal")])
abilities -> ambition_characters::brain::{ActionSet, HeldItemSpec, HeldUseBehavior, MeleeActionSpec}
bodies    -> ambition_platformer2d_core::BodyKinematics
hit events-> (none: zero occurrences of `hit_event` or `HitEvent`)
```

⇒ **The remainder names NOTHING in the actor kernel** — zero `crate::<module>::`
paths, zero `super::`, one `crate::items` to its own parent, checked in all three
forms because this document has already been wrong twice by grepping only one.
A carve does not need hooks designed; it needs the new crate to depend on the
same lower crates the file already depends on.

⚠ **AND "hit events" WAS NEVER TRUE AT ALL** — the phrase appears in the comment
and nowhere in the code.

⭐ **THE EVIDENCE IS NOT ONLY A COUNT: `ambition_world_items` LEFT THE SAME
MODULE THE SAME DAY** and needed no hook design. Its one apparent blocker
(`TouchCollectorFilter`) turned out to be a type alias over two `shared_tangle`
markers. The defence predicted that carve was blocked; it took an evening.

⇒ **SO THE THREE-FRONTIER CONCLUSION ABOVE NEEDS RESTATING**: it is not "only one
of three was work". Developer dependencies were work and are done; presentation
is a mirage; ITEMS IS WORK TOO, and what stands between it and a carve is 516
lines — a plugin that schedules its neighbours' systems, and one checkpoint
function that reads authored construction records — not a missing abstraction.
⚠ The plugin is where *"mutates abilities"* is actually true: it names
`crate::abilities::{ranged ×10, traversal ×4, thrown ×3}` and
`crate::ability_cooldown`. Those are its NEIGHBOURS' systems being placed in a
schedule, which is why the plugin stays behind and the domain does not.

⛔ **So three of this doc's frontiers have now been checked and only one was
work.** Developer dependencies: done. Presentation: no renderer anywhere, a
mirage. Items: defended, and a carve needs a hook design first. A future slice
should start from the SEAM it intends to add, not from a line count — the top of
this document already says a line count is a proxy, and these three are what that
warning looks like in practice.

### Encounter/conversation/world orchestration

The actor kernel should emit/consume small semantic facts. Room, encounter,
conversation and provider orchestration belongs to their owning runtime/domain
packages.

### Character preparation versus actor simulation

Prepared character/content ownership should continue moving toward character
and provider packages. The residual actor kernel consumes prepared body/action
facts rather than becoming the content compiler/catalog.

### Central kernel split

Do this last. Once outer domains are gone, the remaining dependency graph will
show whether body state, movement, decision integration and construction still
need one crate or have another stable seam. Do not pre-split this core because a
source file is large.

## Explicit non-goals

Do not:

- carve by LOC targets;
- add wrapper crates that import the whole monolith;
- scatter feature gates through the kernel merely to move a `cargo tree` number;
- duplicate runtime authority during migration;
- keep historical re-exports for compatibility in this pre-release engine;
- claim runtime/startup improvement without an A/B measurement;
- turn every internal domain into an independently published Bevy crate.

## Exit

This program is complete when:

1. the residual actor package can be described as actor/body simulation without
   listing unrelated product capabilities;
2. optional domains install through semantic capability/plugin seams rather than
   actor-kernel imports;
3. major game/provider orchestration no longer lives in the actor package;
4. minimal consumers do not inherit unrelated domains through the residual
   kernel;
5. the public facade exposes actor semantics without exposing the historical
   monolith name/topology;
6. remaining dependencies are justified by genuine actor-kernel ownership, not
   migration residue.
