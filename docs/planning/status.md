# HEAD orientation

**Snapshot:** `a50c7ea12` (2026-08-21 local project date).

⚠ **this SHA goes stale within hours during an active run** — it names the tree
these paragraphs were measured against, not the tree you have. ⭐ **if it
disagrees with `git log -1`, trust HEAD and the ledger, and update this line
rather than reasoning from it.**

This page is a cold-start map, not an execution queue and not a completion
diary. [`queue.md`](queue.md) is the continuing
execution authority. [`tracks.md`](tracks.md) is the standing reservoir used to
replenish it. Focused plans own technical design.

If this page disagrees with current source or a focused open plan, update this
page rather than appending an archaeological correction.

## 2026-08-21, latest: four competing-authority defects, from a review of `f8ad04f9a`

⭐ **all four were CONFIRMED at source before any code moved**, and the two the
review got right about severity were the two nobody had a test for.

**Landed.**

```text
5902930a7  an input SEAT is not a match SLOT. The smash select screen keyed
           cursors by seat (right — a hand belongs to a person) and then used
           that index as the ROSTER CARD. With a CPU between two people,
           first_free_device gives card 2 device 1, so the second person drove
           the machine's card and their own was unreachable. Also: any human
           could grab any CPU token and nothing arbitrated, so two cursors
           carried one piece. SmashSelect::slot_driven_by + SelectCursors::
           try_grab; SelectCursor::grab is now private so it cannot be bypassed
79f465e62  two bodies may not both SPEND one gap. Every body_contact test was
           one mover against blockers standing still, so nothing saw two movers
           each granted the whole gap: 5 apart, 4 asked each, closed 8. And
           resistance could not save it — the free-gap part of a step is
           granted at full speed by construction, so it happened at 1.0. The
           gap is now DIVIDED in proportion to closing speed
3b804b947  a demo does not own where its cameras land on the glass. TwinTrack
           cleared MainCamera.viewport every frame against the generic owner
a50c7ea12  a claim given back by VALUE EQUALITY is not owned. DeclaredInputSeats
           and the InputAssignmentPolicy resource became one owned
           LocalSeatOffer; two Local<bool> flags and two "if it still equals
           what I wrote" tests are deleted with them
```

⛔ **the shape worth carrying forward, because it appeared three times in one
day:** a claim released by comparing the resource to the value you wrote is not
ownership, and no test whose successor claims DIFFERENT values can see it. Ask
*is this mine*. `SessionSeatingSource::release` had the right shape the whole
time, two lines below the broken code.

**Opened by the same review** — D177 (the viewport fix has no fixture; it needs
a windowed host and the TwinTrack route in ONE app), D178 (a pane follows a
BODY, not the participant driving it), D179 (contact eligibility is still
inferred from displacement MAGNITUDE, and the propose/commit residual the gap
split could not reach).

**Deliberately not done:** D175 remains the largest named architecture item —
seat 0 rides the global shaped `ControlFrame` bus while seats 1+ publish
straight to `SlotControls`, so any new semantic shaper is primary-only unless
someone remembers to write it twice.

## Also 2026-08-21, later: content, guards, and the carve's PRICE

⚠ a second session ran beside the carve above. What a cold start needs from it:

**Landed.** Every fighter authors all four throws (`f3611b93d`), and the roster
ratchet grew 16 → 22 to cover grabs. Seat declaration moved out of the GGRS
backend into `ambition_input` (`d8994ce92`) — TwinTrack's second seat drives, and
the bug it fixed was *host-only*, invisible to every demo-binary test. The LDtk
`EdgeExit` reachability rule now reads the Collision IntGrid; it had been scanning
`Solid` ENTITIES and **could never fire** (15 levels vs 4, intersection empty).
A misspelt loading-zone `activation` is refused instead of silently becoming a
`Door`.

**Open and worth knowing before you pick a row:**

```text
Jon's interact-door bug   NOT reproducible in the sim harness, the demo
                          binaries, the shell host, under rollback, or with the
                          touch overlay. Five suspects eliminated by
                          measurement. Next probe is machine-local:
                          AMBITION_DATA_DIR=$(mktemp -d) ./run_game.sh
D174                      a 16px floor lip inside FIVE EdgeExit zones — you
                          must jump into the hub's contact exits. Content fix
                          or soften the contract; Jon's call
D175                      nine participant-input items, promoted from a doc no
                          ledger row could reach. Its first item is the SEAT-0
                          SHAPING BUS, which is why couch bugs keep recurring
TwinTrack                 PARKED, with a known limit: two panes render ONE
                          coordinate-time slice, so they can disagree about
                          optics and never about simultaneity
```

⛔ **and the carve's price, measured — read this before proposing one.** More
than half the monolith's modules hold rollback-registered types, so they cannot
move without rewriting the wire format. Worse, the cheap-by-coupling modules are
cheap *because* they are assembly or observers — which is exactly what the
monolith is finished AS. Five candidates were taken to a verdict; four were
refused by their destination's own stated contract. D33 carries the table, the
pre-flight command, and the reasoning.

## What moved on 2026-08-21: the monolith carve, by DOMAIN

Jon, that day: *"loc is the proxy. the real win is conceptual domain
separation."* D33 is where this lives, and the row now leads with that.

**Landed** — each with gate + 31/31 absence contracts + smash + `app_it` green:

```text
ActorSurfaceState            -> ambition_platformer2d_core   6c4592021
feel tuning                  -> ambition_combat              d6db434f4
hit reaction + stance        -> ambition_combat              403a32155
capture systems (1,922 ln)   -> ambition_combat              8669740f5
footstool (859 ln)           -> ambition_combat              00030e603
hit camera shake             -> ambition_combat              23755e201
boss animation               -> ambition_boss_encounter      9ea8ea2fa
PickupKindSpec               DELETED (was PickupKind)        d3bd6e95a
```

⭐ **the reusable part is the METHOD, not the moves.** A carve's coupling is not
what a file imports: strip comments, grep BOTH `crate::…` and `ambition_…::…`,
resolve each symbol to its defining crate. Three of these needed a retry because
that was done partially — and a whole 1,922-line domain turned out to be pinned
in the wrong crate by one `pub(crate)` function, not by any real coupling.

⚠ **and two scans produced confident-looking rankings of NOTHING** ("quest"
matching inside "request"; `super::super::` counts that never cross a crate).
Both are corrected in place with the negative recorded, because a list left
standing gets worked. ⛔ check the top hit by eye before believing any of them.

⇒ next: `attack.rs` and `limbs.rs` are both measured and BLOCKED — one on a
dependency decision costing five lockfiles, one on a three-way split across a
crate ordering constraint. Read D33 before picking either up.

## Major closure: D73 is finished

The authority-convergence campaign closed on 2026-08-13. The live architecture
no longer has an enemy `ArchetypeSpec` / `CharacterRoster` body authority or a
build-legacy-body-then-patch character road. Intrinsic body/capability facts come
from authored/prepared `CharacterDefinition`; placement, disposition,
controller, participant and ruleset facts remain contextual.

The migration working memory is archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/).
Do not reconstruct deleted D73 representations because an archived review names
them.

## Current architectural direction

The successor umbrella is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
The goal is a credible Godot/Unity-class 2D engine on Bevy while **Ambition
remains the flagship game and primary product driver**.

The highest-value successor fronts are, **in priority order** — ⚠ this list is ORDERED, and it was reordered on 2026-08-15 because the systemic-world
substrate had overtaken the two fronts printed above it:

1. **⭐ THE SYSTEMIC WORLD SUBSTRATE — the next major frontier, and PRIMARY
   CAPACITY GOES HERE** (D125). What a thing IS, which runtime occurrence it is,
   why it exists and how long it lasts; then item custody as the first demanding
   consumer, then capability-driven gating and reachability, then residency and
   persistent populations. Its seven focused plans are reachable from
   [`tracks.md`](tracks.md).

   ⭐ **status 2026-08-20: the substrate EXISTS** under names the plans do not use
   — `WornCharacter` (authored template), `SimId` (runtime occurrence),
   `SpawnOrigin` (provenance) and four ENFORCED lifetime scopes. Custody, item
   ownership, and all three persistence horizons (current world truth, the
   checkpoint/reset ledger, and durable save) are landed and distinct:

   * ✔ inventory ownership is settled (Jon's reviewer, 2026-08-15): the **body**
     owns its inventory and capabilities; `OwnedItems` is a migration/
     compatibility projection, not an undecided authority.
   * ✔ a held object's identity is the authority; the catalog only projects a
     count (`284ebd00d`). Held objects and pure-quantity items are disjoint
     populations, so a pickup can no longer mint a duplicate.
   * ✔ persistent occurrence continuity (`Placed` rows), the checkpoint/reset
     horizon, and durable save (`AmbitionGameSaveData` carrying
     `AuthoredOccurrences`, `CustodyBaseline`, `MintedItemBaseline`) are all
     landed. A durable description of a runtime-minted occurrence is exactly
     identity + `SpawnOrigin` + a definition reference — no position, no
     component snapshot (`88b611caf`). Headless compositions now install
     `DurableSaveHorizonPlugin` themselves, so an RL episode persists too.
   * ⛔⛔ a relation may not cross the durable horizon without its own authority
     (2026-08-20): `InCustodyOf` has two owners (item custody is durable,
     `PossessionState` is not), so the mirror now writes an `InCustody` claim
     only for occurrences the durable road can restore.
   * ▢ open: `Consumed` round-trips through the file with no live producer yet
     (load-bearing for `AuthoredOccurrences::rewind_argument` — a real open
     design item). The body resumes at the shrine while objects resume at the
     autosave's instant — two different times in one load, a deliberate
     first-slice trade, not an oversight.

   ⛔ **do not promote easy actor-monolith leaf carving ahead of this.**
2. **Simulation authority and determinism.** Decompose parameter-ceiling systems
   by phase/authority and invert rollback declaration ownership. See
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).
3. **⭐ NEW 2026-08-15 — deterministic authored gameplay logic and orchestration**
   (D127). Authoring is strong for **nouns** and weak for **verbs and
   relationships over time**; several independent partial condition → effect
   systems already exist in tree. **Rust extends the engine's vocabulary;
   authored content composes vocabulary that already exists.** See
   [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).
   ⛔ not scripting, not a rule VM, not a central effect enum — the substrate
   owns no universal sequencer, and boss patterns are the **template**, not a
   customer.

   ✔✔ **M1 IS MET FOR CONDITIONS, with two unrelated consumers.**
   `shared_tangle::authored_logic` owns the contract — `publish` is PRIVATE, the
   only way in is `PublishCondition for App`. Three domains publish
   (`custody.is_held`, `world.flag_set`, `inventory.holds`); a gated lock wall
   and authored `.yarn` dialogue both consume through one generic verb
   `condition("domain.question", <arg>)`, so publishing a condition makes it
   askable from dialogue with no edit to any bridge.

   ⛔⛔ **this also refuted the premise behind `YarnStateMirror`**: Yarn library
   functions CAN be Bevy systems and reach `&World` (`bevy_yarnspinner` advances
   the interpreter from an exclusive system; `SystemId<In<P>, O>` implements
   `YarnFn`). The mirror shrank to a projection rather than a feed.

   ⇒ **commands are a different shape than conditions, established rather than
   assumed**: a condition is safe to call from inside the interpreter precisely
   because it cannot change anything; a command mutates, so `<<give_item>>`
   records a REQUEST rather than granting. A `PublishCommand` contract owes
   authority, ordering and a ledger-shaped replay story, and generalises from
   `NarrativeInputPlugin<M>`, not from the condition catalog.
4. ⏸ **Ambition authoring + kinematic world objects — RESTING (D115, K2–K6 all
   closed).** Treat authoring/tooling as
   an engine product, improve LDtk as a first-class spatial compiler surface,
   and use moving platforms as the first vertical slice. See
   [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md) and
   [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
   and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).
5. ⏸ **Ambition multiplayer + multi-view presentation — RESTING (D116).** Support local, online and
   mixed participants independently of shared/fixed/adaptive split-screen; grow
   toward multiple resident rooms when participants separate. See
   [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
   and [`game/multiplayer.md`](game/multiplayer.md).

   ⏸ **D116 RESTS (2026-08-15), and M2 is only HALF done** — say it in two parts.
   ✔ **closed:** the presentation/projection sub-slice — per-view association and
   viewport application are proven by an assembled-host fixture, and both
   `PresentsView` writers that guessed are fixed. ▢ **deferred:** production
   two-view composition and layout — production spawns one camera and publishes
   one screen rectangle to every view **by construction**, and M2's own plan also
   names HUD ownership and input routing, which this slice did not touch.
   ⛔ do not expand into networking; the deferred half needs a real product need
   for a second view.
6. **Capability/runtime composition** (D136). Make optional capabilities honest
   in dependency and composition topology. See
   [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

   ⭐ **the through-line: each gap is a place where "who is this for?" was
   answered by whoever installed it first and never written down.**

   ✔✔ **`DeathRules` fixed (2026-08-16, `03d4c8d22`).** It was a bare `Resource`
   inserted at plugin-build time by three games, so the shell's Mary-O-after-
   Sanic composition order made every Smash match run under her 3.2s level
   replay. Fixed by declaring into `DeclaredDeathRules` under the rooms a game
   governs, using `runtime::mode_scope` (which already scopes a hosted game's
   systems and entities) — a second claim on one scope panics at build.
   ⇒ **the lesson: when a scoping concept exists, ask what KINDS of thing it
   scopes, not whether it exists.**

   ⛔ **the standing number, re-measured 2026-08-18: 44 crates linked, 17 a
   movement-only game never asked for** (`capability-footprint-may-not-grow`,
   printed by `check_absence_contracts.py` on every run — read the contract's
   own output rather than quoting a stale copy). The monolith is now off the
   `ambition_platformer2d_ldtk` holder list (production code names it zero
   times; the crate builds `--no-default-features`); the runtime is the
   remaining holder, and the footprint number itself has not moved because the
   dependency was already declared optional. ⇒ a slice claiming this front must
   run `cargo tree -i` for the crate it means to evict before picking what to
   carve, and must say what it did to the number or why the number is
   dominated by something it did not touch.
7. **Public SDK, authoring ergonomics, performance and iteration.** See
   [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md) and
   [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

⚠ **the browser is a TEST FIXTURE, not a front** (Jon, 2026-08-14). It is a
powerful architecture probe while the engine is decomposed — it found a shipped
composition that differed from desktop's and a developer instrument that was
load-bearing for gameplay input — but it does not decide which subsystem gets
built next. ⭐ **the test for any tempting performance task: would we want this
abstraction if the web target disappeared tomorrow?** Semantic asset readiness,
cross-platform phase telemetry, canonical asset publication, host-owned input and
an explainable load barrier all pass it. Brotli, wasm audio scheduling, Hall
streaming, a generic residency scheduler and byte shaving do not.

## Product and engine customers

- **Ambition:** flagship game. Its real content, authoring, multiplayer,
  persistence and presentation needs have first claim on product value.
  ⭐ its structural hub is [`game/ambition.md`](game/ambition.md) — the game and
  engine co-evolve, and it is **not** a thin demo waiting for a finished engine.
  From there: [`game/vision.md`](game/vision.md),
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md),
  [`game/systemic-progression.md`](game/systemic-progression.md),
  [`game/multiplayer.md`](game/multiplayer.md). ⚠ nothing linked that hub until
  2026-08-15, which is how the flagship customer's own map went unreachable.
- **Super Smash Siblings:** serious platform-fighter customer and possible future
  first-class game, but not the project focus. Its remaining body-generic work is
  in [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md).

  ⭐⭐ **GRABS — the third leg of the rock-paper-scissors core — are LIVE as of
  2026-08-18.** Landed: acquisition, the hold, pose, pummel, throw, release, a
  bounded hold with a captive escape channel, a control-hold claim registry so
  one authority cannot free another's body. Proven in a real driven match: 14
  holds, 2 pummels, all ended by throw
  (`cargo run -p ambition_demo_smash_app --bin capture_probe -- 60 --force`).
  A human could not grab at all until `988807b99` — `brain/player.rs`, the seam
  a human's input frame crosses to reach a body, never copied `grab_pressed`;
  no capture test could have caught it because every one writes `grab_pressed`
  directly onto the body. The input-reachability chain
  (`ControlSlot → action → ControlFrame → body`) is now closed at all four
  links, three of them by the compiler and poison-verified.

  ⭐⭐ **AND THE SUITE IS GREEN — `smash_it` 26/7 → 34/0 on 2026-08-20.** All
  seven long-red guards, both repertoire ones and all five `the_stage_kills`,
  pass with **body contact** in the movement sweep and nothing in the brain
  moved. That MEASURES the reading of the CPU limit cycle as a missing physical
  spacing primitive: two fighters closing on each other now STALL where they
  meet, which is where attacks land. `ambition_platformer2d_core::movement::body_contact`
  constrains proposed motion before the world sweep and writes no position, so
  nothing is teleported apart; `BodyContact { resistance }` is presence-as-opt-in
  and the smash stage grants it to its cast. ▢ the resistance number (0.85) is an
  unmeasured feel choice and is Jon's.

  ▢ **open: WHEN a CPU presses Grab.** It owns one, chooses one and presses one
  — mostly at ~110px against a 42px reach. Fighter capture POLICY, not a bug in
  the mechanic — the value of a hold depends on the throw it sets up, the
  escape risk, the percent and the stage, and the generic option scorer has no
  term for it and should not grow one. **That is D166's customer.**
  ⛔ do not price a grab's damage to fix this — tried, made the CPU grab from
  110px in every exchange, nine attempts, none in range, zero holds. A grab
  deals NO damage.

  ⭐ **D166's first facet landed the same day** (`5cefafc05`): George's capture
  kit is authored content, read by the content compiler into the same
  `MoveSpec`s the Rust literal produced. The sixteen ordinary move slots
  deliberately did NOT move — they are authored by composing helpers.
- **TwinTrack:** ⭐ **a TWO-PLAYER game with a real split screen as of
  2026-08-20**, and the pressure test that paid for three engine seams. The
  laboratory twin is Emmy No-Ether, a constructed character body driven by seat
  one; the screen is split by construction, one gameplay `LocalView` per
  participant. What it bought the engine: `ambition_sim_view::ViewPlacement`
  (where a view sits, as a fraction of the gameplay rect),
  `ambition_sim_view::ViewSubject` (which body a view frames — the resolve
  answered that ONCE above the per-view loop), and `spawn_main_camera` declining
  to spawn a rig it cannot honestly bind. Both relativity read-models publish one
  row per observer and TwinTrack is their first adopter, so the two panes'
  numbers disagree because the physics does.

  ⛔ **two rules it cost to learn.** How many views there are belongs to the LIVE
  SESSION, not to what the binary links — composing a second view at plugin build
  time split every route in the game, and the symptom was `bevy_egui` panicking
  about schedules in 95 unrelated tests. And two seats is TWO statements:
  `DeclaredInputSeats(n)` makes seat entities, `InputAssignmentPolicy::JoinToClaim`
  gives them devices, and a surface that says only the first gets a dead second
  seat — measured on Jon's hardware.

  ▢ which pane shows which observer's aberrated sky is a presentation choice
  nobody has made; the seam is there.
- **Sanic / Super Mary-O / Hollow Lite:** retained acceptance customers for
  movement, classic platforming/content, and encounters/boss authoring.

An acceptance customer may eventually become a first-class game. That changes
its product investment, not the engine ownership rules.

## Durable architecture to remember

- one body, one path;
- character definitions own intrinsic reusable body composition;
- controllers provide intent rather than defining a body species;
- construction/preparation fails before partial mutation;
- deterministic simulation authority is explicit and snapshotable;
- views are local presentation over one simulation, not duplicate worlds — and
  **how many there are is a property of the live session, never of what the
  binary links**;
- **a capability is not landed until something ADOPTS it**, and a `Deref`-to-the-
  first-row fallback is how an unadopted split hides: every old reader compiles,
  reads identically while there is one row, and silently switches to whatever
  the publisher sorts first the day there are two;
- transport, control assignment, world residency and view layout are independent
  axes;
- LDtk is Ambition's preferred spatial authoring surface and should improve when
  real Ambition content outgrows it;
- the actor monolith is drained by coherent ownership, not line-count quotas;
- public APIs should expose game concepts rather than historical crate topology;
- **a relationship may not cross the durable horizon without its authority** —
  the save may only claim what the load can reconstruct, and a generic component
  gaining a second population enrols that population in every generic sweep,
  persistence included;
- **a set of lanes is a composed value, not a repeated one** — when a second
  customer of a federation arrives, the enrollment cost it MEASURES is the
  evidence for a composition owner; make it a plain struct whose every operation
  destructures exhaustively, so the carry list is one the compiler keeps, and
  keep the dynamic machinery out (`Any`, `TypeId`, registries, service locators
  trade a compile error for a runtime lookup).

## Explicitly deferred, not abandoned

- production online transport/Matchbox work should grow from an actual
  multiplayer slice rather than be built speculatively;
- Slower Light remains a future 3D relativity game;
- water/oil extensions to falling-sand remain desired deferred product ideas;
- the Leafwing clash-scan optimization remains trigger-based maintenance.

## Where to look next

1. [`queue.md`](queue.md) for execution order.
2. The focused plan named by the selected row.
3. [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct maintainer observations.
4. [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) only when
   an actual product/feel decision is required.
5. [`tracks.md`](tracks.md) when replenishing the queue.
6. `docs/concepts/`, `docs/systems/`, `docs/architecture/` and `docs/adr/` for
   settled truth; `docs/archive/` for history.
