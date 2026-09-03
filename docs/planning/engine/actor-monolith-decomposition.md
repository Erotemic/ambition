# Actor residual-kernel decomposition

**State:** OPEN — incremental ownership/dependency work.

Durable decomposition doctrine:
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).
Capability closure:
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

> **Guard pointer, added ec6d5150b (2026-09-02).** This carve has an absence
> contract naming it directly: `characters-do-not-depend-on-the-actor-integration-layer`
> (`scripts/check_absence_contracts.py`) forbids `ambition_characters` from
> depending on `ambition_platformer2d_actor_monolith`,
> `ambition_platformer2d_runtime` or `ambition_platformer2d`. Its reason cites
> THIS program: the reverse edge is "a cycle waiting to be discovered by the
> compiler at the worst moment", and "if a coherent actor kernel exists at all,
> `ambition_characters` is below it". So the direction of the carve is already
> pinned by a check, not only by intent — green today, and it would go red the
> moment a slice moved something the wrong way.

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

The census distinguishes three different numbers. Re-measured 2026-09-03; the
2026-08-29 values are kept beside them because the DELTA is the whole point — a
carve that lifts a domain out of the monolith adds a dependency EDGE to the
crate it created, so these rise while the debt they measure falls.

| measure | 2026-08-29 | 2026-09-03 |
|---|---:|---:|
| `ambition_*` lines in the monolith `[dependencies]` table | 28 | 30 |
| direct edges in the default resolved graph | 27 | 29 |
| full default resolved closure | 34 | 35 |

⚠ Count the ROOT out. `cargo tree -p <crate> --prefix none` prints the crate
itself as its first line, so a naive `grep -c` reports 30 and 36 here and
silently erases the manifest-lines-exceed-edges gap the census exists to show.

The +2 on the first two rows is `ambition_world_items` and `ambition_held_items`
(D33 carves, 2026-09-02/03).

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
  ✔ **AND THE REPO WAS SWEPT FOR THE SAME SHAPE — this is the only one.**
  Searching every `add_systems(<explicit schedule>, …)` block for a
  `.before`/`.after` naming a concrete cross-crate FUNCTION (rather than a set,
  which carries no such hazard because both sides land wherever the set does)
  returns exactly two: this, and `sim_core_resources.rs`'s
  `apply_camera_shake_requests.before(camera_ease::tick_camera_shake)`.
  ⭐ The camera one is SOUND and deliberately so — both sides are registered on
  `Update` by name, split across composition groups for the reason its comment
  gives (the applier lives with the resource it writes; the tick is the windowed
  host's, because a headless run has no camera), so the edge binds; and if the
  host plugin is absent there is no system to order against and the vacuity is
  harmless. ⇒ Nobody needs to re-run this sweep.

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

- ⭐ **READ WHAT THE DOMAIN ALREADY EMITS BEFORE DESIGNING A REQUEST VOCABULARY
  FOR IT** (2026-09-03, the encounter cut). The seam design for the encounter
  mob spawn specified a new `ActorConstructionParams` variant. Cutting it found
  the request already existed — `ambition_encounter` emits
  `EncounterEvent::SpawnCommand` on the ordinary bus — and the kernel was not
  missing a protocol, it had a SERVER buried inside the adapter, pulling its own
  requests out of a local vector so the request never had to travel. The cut was
  to SEPARATE driving from serving, not to build a channel. A carve that starts
  by designing vocabulary will design vocabulary; start by reading what already
  crosses the seam.
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

## D33 RULE — a carved domain owns its schedule end to end

Settled 2026-09-02, after one carve shipped a live defect by getting it wrong and
the next one stopped at the same fork.

> **A carved domain's plugin configures its OWN sets end to end. A set that two
> crates order on is `shared_tangle` vocabulary, configured by exactly one owner.
> A carve that moves `add_systems` without `configure_sets` has moved the logic
> and left the schedule.**

⛔ **THE COUNTER-EXAMPLE IS IN THIS REPOSITORY AND IT SHIPPED:** `69641a83f`
carved `ambition_world_items` out, moved the two systems' `add_systems` line and
the ordering *between* them, and left behind the `configure_sets` that said
`ItemPickupSet::CoreHeldItems` was `.in_set(PlayerSimulation)` and
`.after(BodyCustodySettled)`. Both facts were lost — session authorization and
the custody edge — and the new plugin's own comment claimed the scheduling was
preserved, because the author had checked the ordering and not the membership.
⚠ It compiles, it runs, and both systems execute every frame. Nothing fails.

⭐ **THE PRECEDENT FOR DOING IT RIGHT** is the fix for that same defect:
`WorldItemSet` lives in `shared_tangle` as VOCABULARY so crates outside can order
on it, and the OWNING plugin does both the `configure_sets` (nesting in
`PlayerSimulation`, `.after(BodyCustodySettled)`, each variant `GameplayGated`)
and the `add_systems`; the kernel merely composes the plugin.
⇒ **It landed: `d220accee`**, with the guard in `dbec94824`. (The SHA was
deliberately left blank when this rule was written, because the fix was on
another machine and citing a commit nobody could resolve is worse than citing
none.)

⛔⛔ **AND WRITING THE GUARD FOUND A SECOND WAY TO BE VACUOUS**, one this rule
did not anticipate and every carved crate will meet. Bevy 0.19 reports a system
as `"<Enable the debug feature to see the name>"` unless `bevy_ecs`'s `debug`
feature is on, and a carved crate that takes `bevy` with
`default-features = false` does not turn it on. So the name-based membership
lookup in the monolith's own `actor_decision_phase_tests` — the obvious thing to
copy — works there only because something ELSE in that build enables the
feature. Copied into `ambition_world_items` it passed under `--workspace` and
failed under `-p ambition_world_items`: a guard whose verdict depends on who
else is in the build, which is the same defect as a guard whose edges depend on
who else configured them. ⇒ **Identify a system by SHAPE, not by name**: assert
the member COUNT of each set against the number the plugin adds. It is exact
rather than approximate whenever the plugin's systems are the only ones present,
which on a bare `App` they are.

⇒ **WHAT IT MEANS FOR A CARVE IN PROGRESS**, and the pickup cut is the worked
example: split a set family BY VARIANT so one crate owns one variant's rules end
to end. Do not split a single variant's `configure_sets` from its `add_systems`.
The kernel's plugin keeps only the sets whose members it still owns, and the
moved tests build the CARVED plugin rather than the kernel's — which is what
makes them able to leave at all.

⛔⛔ **AND THE GUARD SHAPE, because the obvious guard does not catch this.**
Assert phase MEMBERSHIP in the carved crate's own tests — that its systems are
members of the phase set they must run in — never that the systems EXIST. The
defect above shipped with both systems present and running every frame, so an
existence check is green on it. ⚠ A carved crate whose tests register into sets
that nothing configured will pass ordering assertions VACUOUSLY, which is why
the owning plugin must configure them: the test builds the same plugin the game
does, or it is testing a different schedule.

⇒ **The pickup carve's step list is written out as an executable checklist** in
[`pickup-carve-checklist.md`](pickup-carve-checklist.md), including the schedule-ownership
answer this rule's "split by variant" leaves open: the three `ItemPickupSet`
variants are `.chain()`ed to each other, and that inter-variant edge belongs to
the kernel because it is the side that can name both.

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

⛔ **THE DEFENCE BELOW IS A DATED RECORD AND EVENTS OVERTOOK IT. Re-measured
2026-09-03: `src/items/` is 2153 lines, not 6580** — two carves came out of this
very directory (`ambition_world_items` 1328 lines, 2026-09-02; the much larger
`ambition_held_items` 3454, 2026-09-03), and `ambition_items` is 2011. So the
argument was right about what it measured (most of the 6580 was test code, and
`ambition_items` already owned the catalog) and was NOT a reason the directory
would stay put: what left was the collectible lifecycle, along the collect
TRIGGER seam, which this reading did not consider. Kept rather than rewritten —
the reasoning is sound and the conclusion is superseded, which is the useful
thing for the next carve to see.


* 1864 of those lines are `pickup/tests.rs`, plus ~270 more in sibling test <!-- cite-ok: the pre-cut path (moved to ambition_held_items 2026-09-03), kept as the record -->
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

#### The `encounter/` seam design, measured 2026-09-03 (SPEC — nothing cut)

The domain already lives in `ambition_encounter`; what the kernel keeps is
`crates/ambition_platformer2d_actor_monolith/src/encounter/`, an adapter. Its
kernel edges, counted in **all three path forms** because this doc's sizing has
been wrong before by grepping one:

```text
encounter/mod.rs          2 seams   FeatureWorldOverlaySet, sync_authored_gated_lock_walls
encounter/systems.rs      4 seams   spawn_encounter_mob, EncounterMobSeed,
                                    clear_encounter_reward_ecs, sync_encounter_reward_chests_ecs
encounter/lock_walls.rs   1 seam    rebuild_feature_ecs_world_overlay
encounter/loading.rs      0
encounter/switch_index.rs 0
```

⚠ **Seven distinct seams, not the six the assignment named** —
`rebuild_feature_ecs_world_overlay` is a seventh, in `lock_walls.rs`. And 243
lines (`loading.rs` + `switch_index.rs`) name no kernel seam at all, which is the
same shape the pickup carve found: the entanglement is the plugin and one systems
file, not the domain.

**(a) A mob spawn becomes a construction REQUEST — and the road already exists.**
`spawn_encounter_mob` in `features/ecs/spawn/mod.rs:1028` is a pure pass-through
to `spawn_actors.rs:1947`; it is not a construction wrapper today. But the kernel
already serves runtime actor spawns as requests: `ActorConstructionParams`
(`crates/ambition_platformer2d_actor_monolith/src/construction/mod.rs:135`)
carries `StagedActor(SpawnActorRequest)`, `SummonedMinion`, `AuthoredEnemy` and
`GiantHost`/`GiantHand`, dispatched through the generic
`ConstructionDomain`/`RecipeDispatch` protocol in
`ambition_platformer2d_shared_tangle::construction`.
⭐ **The precedent for a domain crate owning its own construction is
`ambition_portal2d`** (`crates/ambition_portal2d/src/gun_construction.rs:45`):
it declares `PortalGunConstructionParams` (pure data), implements
`ConstructionDomain`, and owns its construct fn. The kernel does not know it.
✔ **`EncounterMobSeed` is ALREADY pure data over crates below the kernel** —
`String`, `Option<&str>`, `ambition_entity_catalog::placements::CharacterBrain`,
`ae::Vec2` (`spawn_actors.rs:1916`). Nothing kernel-only is in it. The single
reason `ambition_encounter` cannot emit one is that the TYPE lives in the kernel.
✔ **And the dependency is available:** `ambition_encounter` already depends on
`ambition_platformer2d_core` and `ambition_platformer2d_shared_tangle`; it needs
`ambition_entity_catalog`, which has **zero `ambition_*` dependencies** — a leaf,
so no cycle.
⇒ **New form:** the seed moves down beside the construction vocabulary,
`ambition_encounter` emits a request, and the kernel serves it through the
`ActorConstruction` domain it already runs.

✔ **AND THE FORK IS ANSWERED, by reading the construct body rather than
weighing the two options.** The choice looked open — a new
`ActorConstructionParams` variant (kernel keeps the recipe) versus
`ambition_encounter` implementing its own `ConstructionDomain` like
`ambition_portal2d`. It is not: `spawn_encounter_mob` builds its body with
`ActorClusterSeed::new_character_in` (`features/ecs/spawn_actors.rs:1975`),
which was the actor kernel's body builder when this was written (the seed is
`ambition_body_seed::ActorClusterSeed` since the same night — a VALUE the
kernel spawns from; `spawn_encounter_mob` itself is still the kernel's). For
`ambition_encounter` to own the construct fn it would have to depend on
`ambition_platformer2d_actor_monolith` — **the exact edge this carve removes**,
and the one the sibling crates' policy rows forbid regaining.
⇒ **A new `ActorConstructionParams` variant; the recipe STAYS in the kernel.**

⛔ **CORRECTION FROM CUTTING IT (2026-09-03): no new variant was needed, because
the REQUEST ALREADY EXISTED.** `ambition_encounter`'s wave director emits
`EncounterEvent::SpawnCommand { id, character, kind, pos, size }` — all
primitives — onto the ordinary event bus (`crates/ambition_encounter/src/waves.rs:141`).
The kernel was not missing a protocol; it had a SERVER buried inside the
adapter: `drive_wave_encounters` pulled its own `SpawnCommand`s out of a LOCAL
vector and built the bodies itself, so driving and serving were one system and
the request never had to travel.
⇒ The cut was therefore to separate them, not to invent a channel:
`features::serve_encounter_spawn_commands` reads the bus and constructs. The
conclusion the design reached — *orchestration leaves, construction stays* —
held; the mechanism it proposed was more than the code needed. **Read what the
domain already emits before designing a request vocabulary for it.**

✔ **(a) IS CUT, 2026-09-03.** `EncounterMobSeed` is
`ambition_encounter::mob_seed::EncounterMobSeed` (not re-exported from the
kernel), and the server is its own system.
⭐ The driver's shrink is the evidence: it no longer takes the character
catalog, the prepared cast or the authored sheets — every one a
body-construction input it needed only because it served its own requests.
✔ Guarded and poison-verified: the server must be ordered `.after(WaveEncounterDriven)`,
or it reads requests a tick late while every existing test still passes (they
assert the EVENT, not that a body was built).
⚠ **Stated rather than implied: there is still no test that a wave request
produces an ECS mob.** The guard pins the wiring, not the behaviour; the gap
predates this cut.
The portal-gun precedent does not transfer because a portal-gun pickup is a
simple authored entity while a mob IS an actor, and actor construction is
definitionally the kernel's. That is the doctrine line exactly: *orchestration*
(which mobs, when, in what wave) leaves with the domain; *construction* (how a
body is assembled) stays with the kernel that owns bodies.

**(b) Rewards invert: the encounter publishes facts, the feature layer reacts.**
The adapter today computes `cleared_specs` by filtering encounters whose
`ambition_encounter::EncounterPhase` is `Completed`, then PUSHES that list into
`crate::features::sync_encounter_reward_chests_ecs`
(`encounter/systems.rs:557`); `clear_encounter_reward_ecs` (`:447`) is the same
shape in reverse. Both facts — *cleared*, and *a prior clear must be dropped* —
are already owned by `ambition_encounter`, which has the phase, the lifecycle
events and `EncounterLifecycleSet`.
⇒ **New form:** the encounter publishes cleared/locked facts; the kernel's
feature layer reads them and syncs its own chests. The two reward functions STAY
in the kernel — they are room-feature systems — and the adapter disappears
because the call direction reverses. No new vocabulary is needed beyond making
the fact readable.

✔ **(b) IS CUT, 2026-09-03.** `ambition_encounter::rewards::ClearedEncounters` is
published by the domain, chained after its lifecycle reducer and inside
`EncounterLifecycleSet`; the kernel's `sync_encounter_reward_chests` reads it,
composed by the RUNTIME as `EncounterRewardSyncPlugin` so no registration lands
back in the adapter.
⭐ **The adapter shrank in a way that proves the direction changed: it no longer
spawns anything.** The chest query left with the sync, and the reward call was
its only use of `commands`. Its session guard was KEPT — it gates the whole
system on a live session, and dropping it would newly run the trace, quest,
banner and music projections in a session-less world, which is a behaviour change
for whoever removes the last caller.
✔ Guarded and poison-verified on MEMBERSHIP: every system
`EncounterRegistryPlugin` schedules must be inside `EncounterLifecycleSet`, or
consumers ordering `.after()` it read last tick's list — a chest one tick late,
or absent on the tick an encounter resets, with nothing red.

**(c) `FeatureWorldOverlaySet` is `shared_tangle` vocabulary, and the evidence
predates this carve.** It is defined at
`crates/ambition_platformer2d_actor_monolith/src/world/overlay.rs:32` and used as
an ordering anchor by both lock-wall systems. ⭐ **TWO crates outside the kernel
already order against it in PROSE because they cannot name it:**
`ambition_combat::hazards` cites *"`crate::…::FeatureWorldOverlaySet`: a general
crate consumed by content owed its consumers a name to order against and did not
have one"*, and `ambition_damage` cites it as the precedent for its own one-member
set. Both invented their own sets rather than name this one.
⇒ **The set that is cited as THE precedent for publishing an orderable name is
itself the one nobody outside can name.** Move it to
`shared_tangle::schedule` — the `ItemPickupSet`/`AudioInitSet` inversion — and the
two prose references become real edges. This is worth doing whether or not the
encounter carve proceeds.

✔ **(c) IS CUT, 2026-09-03.** The set lives in `shared_tangle::schedule`; all
five consumers name it from there (`ambition_content` directly, the demos through
the facade's `platformer::schedule::` path they already use for every other
schedule label) and **zero references reach it through the monolith**. The two
prose citations now name the real path.
✔ Guarded and poison-verified with a MEMBERSHIP assertion, not an existence
check: all five external edges are satisfied by ONE
`.in_set(FeatureWorldOverlaySet)` on `rebuild_feature_ecs_world_overlay`, and
deleting that single call leaves every consumer compiling, still carrying its
`.after(..)`, waiting on an empty set. The guard goes red on that deletion; an
existence check would not.

**(d) ⛔ One of the named seams is NOT a dependency, and must not be treated as
one.** `crate::world::gated_lock_walls::sync_authored_gated_lock_walls` is
scheduled by the encounter plugin but is not an encounter system: it is the
AUTHORED-condition sibling, and the plugin says why it sits there — *"registered
beside it so the two roads into `gate_solids` are visible in one place — this one
arrived from `ambition_content`, where being invisible next to its sibling was
part of how it went unnoticed."* A carve that simply moves the encounter plugin
separates two registrations that were deliberately co-located, re-creating the
condition that hid a defect once.
⇒ **New form:** whatever keeps the two `gate_solids` roads adjacent must be named
in the carve — a kernel-side world-gating plugin holding both, not one road
leaving with the encounter.

✔ **(d) IS CUT, 2026-09-03.** `world::gating::WorldGatingSchedulePlugin` holds
both writers, composed by the runtime; the encounter plugin schedules neither and
leaves a note saying where they went and why. The adjacency is now stated by a
plugin named for the invariant instead of surviving by accident of history.
✔ Guarded and poison-verified: both writers must be scheduled by that plugin,
both `.after(FeatureWorldOverlaySet)` and both members of `WorldPrep`. A writer
that lost the overlay edge would write into a list the rebuild is about to clear
— collision that silently is not there, with nothing red. Dropping either road
from the plugin turns the guard red naming the missing one.
⚠ The pre-existing `gated_lock_walls` tests register that system into their own
app, so they prove the SYSTEM and say nothing about the wiring. This is the
wiring.

#### What is LEFT, measured after the cuts (2026-09-03)

**The adapter's kernel seams went 7 → 1, and the one left is a REGISTRATION:**

| seam | file | kind |
|---|---|---|
| `serve_encounter_spawn_commands` | `mod.rs` | the spawn server's registration; it leaves when the plugin does |

`systems.rs`, `loading.rs` (207 lines), `switch_index.rs` (36) and `lock_walls.rs`
name **nothing** in the kernel.

Retired since this table was first written: `EncounterMobSeed` and
`spawn_encounter_mob` (seam a), `sync_encounter_reward_chests_ecs` (seam b),
`FeatureWorldOverlaySet` (seam c), `sync_authored_gated_lock_walls` (seam d, a
deliberate co-location that moved WITH its sibling rather than away from it),
`rebuild_feature_ecs_world_overlay` (a doc link, repointed at the published
set), and finally `clear_encounter_reward_ecs` — which was a real dependency and
a genuine open question until the switch-loop split made its trigger
observable.

⛔ **AND THE LAST REAL SEAM IS NOT A MECHANICAL INVERSION. It is a policy
question, and it is being left open on purpose.** `clear_encounter_reward_ecs`
fires when the switch arming an encounter turns OFF: it despawns the reward
chest and clears the persisted `reward_dropped` flag so a re-clear pays out
again. Inverting it the way (b) went would mean reacting to a published fact —
but the fact is not `Reset`:

* the reward clear runs on **every** switch-off, while the `Reset` command is
  written only when the phase is NOT in flight, so the two do not coincide;
* reacting to `EncounterEvent::Reset` would be equivalent only if an in-flight
  encounter can never hold a stale chest or a set `reward_dropped` flag. That is
  probably true — chests spawn on `Completed` — but *probably* is how a save-flag
  defect ships;
* and the trigger is a SWITCH, whose index and activation queue are kernel-side,
  so "switch off ⇒ retire this encounter's reward" may be encounter policy or may
  be room-feature policy. Nothing in the code settles which.

⛔⛔ **THE HYPOTHESIS IS FALSE, AND DELIBERATELY SO — resolved 2026-09-03 by
reading the other reset road.** The question was whether *an in-flight encounter
can ever hold a stale reward chest or a set looted flag.* It can:

* `apply_encounter_cleanup` reacts to `Completed | Failed | Reset` and releases
  or despawns the encounter's SPAWNED PARTICIPANTS. It does not touch reward
  chests or the save flag.
* So a **player-death reset** — the road the module doc names — leaves the chest
  standing and `encounter_..._reward_looted` set. `sync_encounter_reward_chests_ecs`
  then reconciles the existing chest's `Opened` marker against that flag rather
  than paying out again.
* Only the explicit SWITCH re-arm despawns the chest and clears the flag, which
  is what its comment says it is for: *"so the next clear pays out fresh"*.

⇒ **The switch-off clear is load-bearing, not redundant**, and the coherent rule
underneath it is: *dying and re-running an encounter does not re-pay its reward;
deliberately re-arming it does.*
⛔ **So the inversion sketched above is disqualified on behaviour, not on
taste.** A feature-layer system reacting to `EncounterEvent::Reset` would clear
the flag on death-resets too and enable repeat payouts. This is exactly what
"probably true" would have shipped.
✔ **AND THAT TEST IS WRITTEN** (2026-09-03):
`a_reset_does_not_retire_the_reward_chest` puts a reward chest in the world,
sends a `Reset` event, runs `apply_encounter_cleanup`, and asserts the chest
survives. Poison-verified by making cleanup retire chests on an end event —
which is exactly the refactor that looks right, frees the encounter adapter's
last kernel seam, and would pay a looted encounter out twice.

✔ **RULED 2026-09-03: the reward clear STAYS WHERE IT SITS, and the trigger must
not be written twice.** The switch is the feature layer's input, the chest is its
entity and `reward_looted` is its save fact, so *"a switch-off retires the
reward"* is room-feature policy today and stays that way — no new published fact,
no change to WHEN the flag clears.
⛔ **And it cannot be moved by registration alone**, which is the part worth
recording: the reward retire is attached to the adapter by POSITION inside a
save-mutating drain — after a toggle, behind three early `continue`s — and FOUR
unrelated policies (a quest flag, `FlipGravity`, the four `SetGravity` faces, the
encounter reset) share that one queue. The clear is stuck to the adapter by that
LOOP, not by encounter logic. Moving only the registration would mean
re-implementing the filter beside the original.
⇒ Its release is downstream of the switch-loop split, specced above as its own
frontier item.

⇒ **The remaining question was an owner's, and it is answered.** The question is: *does
the encounter domain own "my reward is retired", or does the feature layer own
"a chest whose encounter is no longer cleared goes away"?* The first is a new
published fact; the second is a rule the feature layer can evaluate for itself
from `ClearedEncounters`, which already exists. The second is smaller and needs
no new vocabulary — but it changes WHEN the save flag clears, which is player-
visible.

#### The adapter itself can now leave — measured 2026-09-03, SPEC, not started

With the seams down to one registration, the remaining question is whether
`crates/ambition_platformer2d_actor_monolith/src/encounter/` can simply move
INTO `ambition_encounter`. Measured, it can, and the list is short.

**The apparent cycles are not cycles.** The adapter's files mention
`ambition_content`, `ambition_platformer2d` (the facade) and
`ambition_platformer2d_host` — all three ABOVE the encounter domain. Every one of
those mentions is a COMMENT, an `include_str!` path to a content asset
(`encounter/loading.rs:23`), or a log TARGET string literal
(`encounter/systems.rs:119`). None is a crate dependency. ⚠ A `cargo tree`-shaped
reading of the module would have reported three impossible edges.

**What the move actually costs** — four production dependencies
`ambition_encounter` does not yet have, none of which depends on it, so no cycle:

| crate | references in the adapter |
|---|---:|
| `ambition_combat` | 14 |
| `ambition_platformer2d_world` | 8 |
| `ambition_gameplay_trace` | 2 |
| `ambition_time` | 1 |
| `ambition_platformer2d_ldtk` | 3, **tests only** → a dev-dependency |

**The one seam that must move the other way first.** `encounter/mod.rs` schedules
`crate::features::serve_encounter_spawn_commands` — the KERNEL's spawn server.
A plugin living in `ambition_encounter` cannot name it. ⇒ That registration moves
to a feature-layer plugin composed by the runtime, exactly as
`EncounterRewardSyncPlugin` took the reward systems. After that the encounter
plugin names nothing in the kernel and the module is free.

⚠ **Then check the footprint ratchet, and expect it to move for the usual wrong
reason**: `capability-footprint-baseline.json` counts CRATES, so a module
changing crates while the linked code goes slightly DOWN still reads as growth.
Declare it in the idiom the `mount`/`damage`/`world_items` rows use.
▢ Not started; the seam-reversal above is the first step and is small.

**Guards that pin it**, same shape as `ambition_world_items`/`ambition_held_items`:
three policy rows (`engine.<crate>-manifest-allow`,
`engine.<crate>-source-purity`, `engine.runtime-manifest-allow`), both
poison-verified; a MEMBERSHIP assertion in the carved crate per D33, never an
existence check; `rollback_schema_baseline.txt` expected byte-identical because
it keys on owner strings; `capability-footprint-baseline.json` WILL move and the
growth must be declared; `scripts/modules_md.py --write`; and the two
sub-workspace lockfiles — `fixtures/minimal_game/Cargo.lock` is committed and
goes stale silently.
⛔ And run `cargo test -p ambition_workspace_policy`, not `cargo check`: the
allow-lists are `exact = true` and the compiler cannot see them.

### The switch-activation loop — four policies sharing one drained queue (SPEC, 2026-09-03)

Promoted out of the encounter carve, which could not finish its last seam because
of this. **Nothing cut; the spec is the deliverable.**

`drive_wave_encounters` ends with ~90 lines (`encounter/systems.rs:337`–`427`)
that `std::mem::take` the `SwitchActivationQueue` and run FOUR unrelated policies
over every activation:

| policy | what it does |
|---|---|
| quest flags | sets `test_switch_toggled` and `switch_<id>_used`, and pushes a quest event — for EVERY activation, whatever its action |
| `FlipGravity` | inverts `BaseGravity.dir`, persists the switch, `continue` |
| `SetGravity{Down,Up,Left,Right}` | sets `BaseGravity.dir` to a cardinal face, persists the switch ON, `continue` |
| `ResetEncounter` | TOGGLES the persisted switch, resets a terminal encounter, and retires its reward |

**Measured shape.** One producer — `features/ecs/effect_bus.rs:51` is the only
`push`. One consumer — this loop is the only drain (`session/teardown.rs:158`
clears it at teardown). The payload is
`SwitchActivation { id, action: String, target_encounter: String }`
(`crates/ambition_encounter/src/registry.rs:61`), so the action is a STRING
matched with `==` and `strip_prefix`, and an unrecognised action falls through
the `ResetEncounter` guard silently.

⛔ **WHY THIS BLOCKS THE ENCOUNTER CARVE.** The reward retire is not attached to
the adapter by encounter logic — it is attached by POSITION inside this loop,
after a save-mutating toggle and behind three early `continue`s. Nothing can
observe that edge from outside: run before the drain and neither the queue nor
the toggle has happened; run after and both are gone. Moving the registration
alone would mean re-implementing the filter beside the original, which is the
shape `adopt_loaded_save`'s own comment warns about — *"a test that
re-implemented this policy beside it would have agreed with the bug"*.

⇒ **THE SEAM: a drained activation becomes a published fact PER ACTION KIND, each
consumer reacts to its own, and the save toggle is owned by exactly one of
them.** Concretely:

1. **Type the action.** A string matched three ways is why an unknown action is
   silently a no-op. An enum makes an unhandled kind a compile error at each
   consumer, which is the `CapabilityLanes` idiom this repo already uses.
2. **One system drains and publishes**, in the domain that owns the queue
   (`ambition_encounter::switches`), turning the queue into per-kind facts. It
   owns the persisted switch write, so the toggle has exactly one author.
3. **Each policy reacts to its own fact, from the crate that owns it**: gravity
   to `shared_tangle::gravity`, quest flags to the quest/effect layer, encounter
   reset to `ambition_encounter`, reward retire to the feature layer — the last
   of which is what lets the encounter adapter finish leaving.
4. ⚠ **The toggle is the hazard.** `ResetEncounter` reads and writes
   `save.switch(id)` in one step and the reward branch keys off the RESULT. Split
   carelessly and two systems both toggle, or none does. The publisher owns it
   and the fact carries the post-toggle value.

⚠ **Order is part of the value** — `SwitchActivationQueue::checksum` says so, and
the queue is rollback-registered because a rewind that re-pushes predicted
activations double-applies an encounter reset. Any split must keep one ordered
drain, not four readers racing the same queue.

✔ **CUT 2026-09-03.** `ambition_encounter::switches::drain_switch_activations`
is the one ordered drain; it parses each action into a typed `SwitchAction`,
performs the persisted write, and publishes `ResolvedSwitchActivations` carrying
the POST-toggle value so no consumer re-derives it. The kernel's loop reacts to
that. `SwitchAction::Unhandled(String)` carries what the string road dropped
silently.
✔ Both named hazards guarded and poison-verified: leaving the queue unconsumed
(a second author for the toggle) goes red; publishing in reverse order goes red.
✔ **And the reward retire released with it**, as predicted:
`features::retire_rewards_for_rearmed_encounters` reacts to the same published
activation, behaviour unchanged including WHEN the flag clears, guarded on the
OFF edge and on the action kind.
⇒ **The encounter adapter is down to ONE kernel reference and it is a
registration** (`serve_encounter_spawn_commands`); `systems.rs`, `loading.rs`,
`switch_index.rs` and `lock_walls.rs` name nothing in the kernel at all.

### Character preparation versus actor simulation

Prepared character/content ownership should continue moving toward character
and provider packages. The residual actor kernel consumes prepared body/action
facts rather than becoming the content compiler/catalog.

**Measured 2026-09-03, from the seam rather than the line count.** `character_runtime/`
is 13,888 lines with tests (production: `mod.rs` 1,235, `prepared_match.rs` 1,611,
`staging.rs` 833, `audit.rs` 618, `presentation.rs` 598, `live_match_clock.rs` 474,
`physical_baseline.rs` 295, `hurtbox.rs` 277, `seating.rs` 267, `definition.rs` 82).
Six of those files name NOTHING in the kernel (`definition`, `staging`, `seating`,
`physical_baseline`, `hurtbox`, `audit`); they are carve-clean today. The kernel
references that remain are in four files, and they are not one kind of thing:

| File | Kernel names | What it is |
|---|---|---|
| `mod.rs` | `character_sprites::{SpriteMaterialization, character_sprite_tier, materialize_declared_character_sprite, demand_character_fx_sheets}`, `assets::platformer_assets` | the ASSET seam — load demand and materialization; leaves with the sprite domain, not with this frontier |
| `prepared_match.rs` | `features::ecs::actor_clusters::ActorClusterSeed` <!-- cite-ok: the pre-cut path, measured that night --> (the `PreparedSeat.seed` field and `new_character_in`), `avatar::starting_character::InitialBodyPolicy` (×3), and in activation only: `enemy_component_snapshot`, `enemy_default_brain`, `FeatureBaseBundle`, `EnemyActorBundle`, `LocalPlayer`, `participant_seat::player_slot_of` | ONE prepared value holding a kernel type, one policy value, and construction |
| `live_match_clock.rs` | `features::stocks_match::StocksMatchSettled` | a message read |
| `presentation.rs` | `avatar::PersonaBaseline` | a component inserted at staging |

⇒ **The seam is `PreparedSeat.seed`.** Preparation builds an `ActorClusterSeed` so
that activation can spawn without a lookup (this module's own contract: *"activation
performs no authority lookups"*). The seed is the kernel's construction value — and
measured, it is not kernel-typed at all: every field of `ActorClusterSeed` is a
`core`/`characters`/`combat`/`shared_tangle`/`vfx` type, its two constructors read
only the catalog and the authored sheets, and the 1,000-line seed region of
`actor_clusters.rs` (lines 54–1016) contains ZERO `crate::` references. What binds
it to the kernel is where it is DEFINED, plus two methods that hand it to the
simulation step (`as_actor_mut`, `update_for_test`) and `into_components`'s return
type, whose members are all lower-crate components too.

**Landed 2026-09-03 (`944e082c4`): the worn-kit compiler.** What a character id
resolves to on a body — name, action set, moveset, identity baseline, durable
`CombatKit`, how it fires — was compiled in the kernel's `avatar` module and is
`ambition_combat::worn_kit::WornKit::resolve` now; the kernel's three roads (spawn
bundle, runtime re-wear, `prepare_match`) consume the value. The authored-overlay
rule had been written twice (preparation and the kernel); it is
`ambition_characters::prepared::overlay_authored_moves`, once. Zero behaviour change
by construction. The absence contract pinning `build_default_action_set` to one file
now exempts the compiler's new home instead of `starting_character.rs`.

**Landed 2026-09-03, cuts 1–2b (`83460e3f3`, `7ba40886e`, `62bdc8ba3`, `7e625e5a5`):**
the body seed is `ambition_body_seed` (the kernel binds it to the tick through a
`SeedActorMut` trait and keeps `ActorMut`); the physical baseline joined it; the
character load demand is `ambition_characters::load_demand` (the drainer passes a
per-token cost, not a tier); and the versus match — roster, rules, `prepare_match`,
the plan and the receipt — is `ambition_match`, with the kernel keeping
`character_runtime::match_activation` (spawn, control binding, opening hold, view)
and the live match clock. `prepare_match` takes `home_body_spawns_a_body: bool`; the
wrapper system still reads the kernel's `InitialBodyPolicy` and answers it. Two
crates, so the footprint ratchet rose 46 → 48 (crates, not bytes; the monolith shed
~3,400 lines) and the compile-cost ratchet's critical path grew — each carve puts a
crate between combat and the kernel, which is the honest price of a serial chain and
is recorded, not banked. ⚠ Owed: the 3,387-line match test module stayed in the
kernel (it is mostly activation tests and runs against the moved code through the
kernel's dependency); its pure preparation half belongs in `ambition_match`.

**What is left in `character_runtime/` after 2b** (measured): `mod.rs` (demand
materialization — the asset seam), `definition.rs`, `audit.rs`, `hurtbox.rs`,
`presentation.rs`, `live_match_clock.rs`, `match_activation.rs`, and the tests.

**Next cut, in order:**

1. ✔ **The body seed leaves the kernel** (`83460e3f3`, above).
2. ✔ **`InitialBodyPolicy` → a value; `ambition_match` cut** (`7ba40886e`, `7e625e5a5`).
   `prepare_match` takes `home_body_spawns_a_body: bool`
   (`ambition_match/src/prepared.rs:579`); the wrapper `prepare_the_match` — now in
   the kernel's `character_runtime/match_activation.rs` — reads the policy component
   and answers it. That wrapper is the kernel's on purpose (it is the system that
   activates), so the policy dependency is where it belongs and nothing further is
   owed here. The clock stayed in the kernel: it reads `features::stocks_match`.
3. `PersonaBaseline` and `StocksMatchSettled` are each one name; they follow their
   owners when the preparation crate exists, not before.

⚠ The one contradiction this measurement surfaced and did NOT change: a prepared
`Authored` character with `ranged_execution: ChargedProjectile` (the V3 robot: both an
action set with `ranged: Some(bolt)` and the charge execution) gets a preparation
moveset that INCLUDES the ranged verb while the worn kit reports the charge path.
The persona derivation drops the ranged verb for an `Unauthored` charged kit and the
preparation derivation never does; whether the V3 press is owned once is a content
question (`player_robot_moveset.rs` says the slot "says the robot throws something,
the execution says the throw charges"). Recorded, not resolved.

### Central kernel split

Do this last. Once outer domains are gone, the remaining dependency graph will
show whether body state, movement, decision integration and construction still
need one crate or have another stable seam. Do not pre-split this core because a
source file is large.

**First measurement, 2026-09-03 (after cuts 1–2b, before the abilities and encounter
carves land): `python3 scripts/measure_kernel_module_graph.py --edges 10`.** Production
lines per top-level module and the `crate::<module>` references each makes; tests
excluded from the edges. A textual count — a shape, not a bill.

```text
module                   prod    all  out-edges (module:refs)
features                23677  38197  construction:30 character_runtime:14 world:12 avatar:10 causal:8 control:6 abilities:4 items:3
abilities                4962   8585  ability_cooldown:4 features:2 projectile:2 control:2 enemy_projectile:1 avatar:1 character_runtime:1
character_runtime        3843  11468  character_sprites:7 features:5 avatar:2 control:1 participant_seat:1 assets:1
world                    3286   5419  features:8 construction:5 session:3 character_runtime:2 encounter:1
avatar                   2999   7291  body_mode:1 control:1
schedule                 2578   2578  control:2 avatar:1 participant_seat:1
items                    2023   2153  abilities:17 shrine:3 session:2 construction:2 character_runtime:1 ability_cooldown:1
session                  2022   3446  avatar:8 items:5 world:5 features:4 abilities:4 assets:2 construction:1
construction             2019   5710  features:15 world:4 shrine:1
control                  1258   1613  abilities:4 features:1
projectile               1231   2631  avatar:2 features:2
character_sprites        1205   1867  assets:3 character_roster:1
encounter                1105   2056  features:1
causal                    752    752  
audio                     672   1308  music:4
rollback_registration     667    667  features:28 abilities:10 character_runtime:5 session:5 avatar:4 shrine:3 world:3 gravity:2
gravity                   574    574  session:1 schedule:1
    30  features -> construction   (and 15 back)
    28  rollback_registration -> features
    17  items -> abilities
    15  construction -> features   (and 30 back)
    14  features -> character_runtime   (and 5 back)
    12  features -> world   (and 8 back)
    11  snapshot_impls -> features
    10  rollback_registration -> abilities
    10  features -> avatar
     8  world -> features   (and 12 back)
```

What the shape says, without pre-splitting anything: `features` (23.7k production
lines — the actor tick, spawn, damage, brains) is the centre, and its heaviest edge
is MUTUAL with `construction` (30/15) — the seed-to-body road runs both ways. Its
other out-edges are to the two halves this frontier just carved around
(`character_runtime` 14, `avatar` 10) and to `world` (12/8, also mutual). The
modules with NO out-edges and nothing pointing back except registration
(`causal`, `action_scheme`, `body_mode`, `time`, `music`, `cutscene`, `quest`,
`world_facts`) are already islands; that they are still in this crate is inertia,
not coupling, and each is a small cut. `rollback_registration` (28 → features, 10 →
abilities) and `snapshot_impls` (11 → features) are the two files that name
everything, which is what a registration file is for — they are not coupling, they
are the ledger. ⇒ The candidate seam the doc predicted ("body state, movement,
decision integration and construction") shows up as the `features`↔`construction`
loop; whether it is one crate or two is decided by which direction the 30 and the 15
run, which is the next measurement, not this one.

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
