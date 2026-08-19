# HEAD orientation

**Snapshot:** `5cefafc05` (2026-08-18 local project date).

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
   persistent populations. Its seven focused plans were all written and reachable
   only from [`tracks.md`](tracks.md) until 2026-08-14 — the design was never the
   gap.

   ⭐ **status 2026-08-15: the substrate EXISTS** under names the plans do not use
   — `WornCharacter` (authored template), **`SimId`** (runtime occurrence),
   `SpawnOrigin` (provenance) and four ENFORCED lifetime scopes. Custody has landed
   its first slice (`ItemCustody`, instance vs quantity vs consumable) and
   reachability its first query (`movement/recovery.rs`, which drives the REAL
   kernel on a scratch body rather than enumerating capabilities). ⛔ **what is
   still undesigned is DURABLE PERSISTENCE** — *"has no runtime cleanup scope"*
   does NOT mean *"correctly saved and restored"* — and the INVENTORY leg, because
   `OwnedItems` is a process-global count table with no row per object. ⛔ treat
   `OwnedItems` / held-item synchronisation as a **migration seam**, never as the
   custody model: physical custody belongs to the body and the item instance, and
   participant entitlement is a separate fact with a different owner and lifetime.

   ⭐ **measured 2026-08-15: the seam is ONE CLASS WIDE.** Of the 24 catalog
   slots, **5 of 6 classes are counts forever** (consumables, currency, key items,
   unwired abilities, reserved) and their readers legitimately want a quantity;
   the whole problem is the **nine held weapons/abilities that are an instance and
   a count at once**. ⛔ so do not give the count table a row per object.

   ✔✔ **AND THOSE NINE ARE NOW DECIDED (2026-08-16, `284ebd00d`): the INSTANCE is
   the authority and the catalog PROJECTS it.** A picked-up object writes nothing
   to the count table; `OwnedItems::count` reports `stored(item).max(equipped ==
   item)`, and `to_persisted` writes only the stored quantity, so a hand never
   reaches disk as a row. The two populations are now **disjoint** — a row is a
   quantity with no object, an object is an occurrence the checkpoint owns — so
   the disagreement has nowhere left to live. Exactly nine items answer
   `held_item_id().is_some()`, which is the class, checkable.

   ⛔⛔ **what that fixed was a DUPLICATION GLITCH, and the measurement is worth
   keeping**: the pickup used to `grant(item, 1)` beside taking custody, so ONE
   acquisition left TWO records and only the object's rewound. Acquire a weapon
   after a checkpoint and die — the object returns to its pedestal, the catalog
   row stays, the menu equips the phantom, and throwing it **mints a second real
   weapon** that the durable save then writes to disk.

   ⚠⚠ **it traded that for a LOSS — a held weapon carried across a SAVE/LOAD was
   gone — and ✔ THAT IS CLOSED** (`28c927505`, falsifier B): the object persists
   as an OCCURRENCE, never as a quantity, so the duplication it replaced does not
   return by the new road either.
   ⚠ the granted-quantity half is still open and its gate is named:
   **`OwnedItems` joining the checkpoint baseline, with the mint spending the row
   in that same change and not before** — spending it earlier turns the phantom
   into an annihilation whenever a death retracts a post-checkpoint mint.

   ⛔ **and the reason none of this was ever caught is structural**: the
   durable-save leg (`InventoryRestored` + both persist systems) is installed by
   `install_menu_setup_and_hotkeys`, inside the **visible-binary-only**
   presentation plugins. **No headless composition schedules it**, so one of the
   two authorities does not exist in the test harness at all.

   ✔ **inventory OWNERSHIP is settled (Jon's reviewer, 2026-08-15): the BODY owns
   its inventory and capabilities.** Participant entitlements and possession-transfer
   policy are separate concerns with different owners and lifetimes. ⇒ `OwnedItems`
   is therefore a **migration/compatibility representation**, not an undecided
   authority — ⛔ and it is no longer an open architecture question anywhere.

   ✔ **PERSISTENT OCCURRENCE CONTINUITY LANDED** (2026-08-15, both legs): a
   `Placed` row suppresses the home room and reinstates the occurrence where it
   lies, as ONE decision, so an object carried between rooms and put down comes
   back where it was left with the same `SimId`. ⛔ it was NOT answered by
   teaching the room loader to inspect inventories — custody owns residency, and
   room transition still does not know items exist.

   ✔ **AND THE RESET HORIZON ON TOP OF IT** (2026-08-15): three horizons are now
   distinct — *current world truth* (the ledger), *checkpoint/reset truth*
   (`lifecycle::horizon`, restored on death), *durable save truth* (still
   undesigned). The checkpoint baseline is a **projection of domains**, each
   capturing from its own live authority; ⛔ it is deliberately NOT one resource
   holding every reset-relevant fact. Seven beats of the maintainer's rule hold
   through production roads, including the one no `KeyItem => survives` rule can
   produce: one death, two objects of the same kind, opposite answers.

   ✔ **AND THE RESTORE CAN NOW REBUILD WHAT IT PUTS BACK** (2026-08-16,
   `13dd4d31b`): bank a reward, carry it to another room, drop it there, leave so
   that room UNLOADS, then die — it returns to the hand that banked it as the
   same occurrence, pedestal still empty, no duplicate. The missing mechanism was
   **materialization**: the restore was pure re-assignment, which cannot ask
   anything about an object whose entity no longer exists, and no room build
   could supply it either because an `InCustody` row makes `outlook_for` answer
   `Suppressed` in every room — a thing in a hand is not a thing in a room. ⭐
   **every other reconstruction road here starts from a ROOM and asks what it
   owes; this one starts from an occurrence resident in no room**, so the authored
   definition has to be reachable BY IDENTITY.

   ✔✔ **AND THE RUNTIME-MINTED CASE CLOSED TOO** (2026-08-16, `88b611caf`),
   which was the residual named directly above. **The minimal durable
   description of an occurrence the simulation itself made is three things and
   no more:**

   ```text
   identity     the occurrence's own SimId
   provenance   SpawnOrigin::Dynamic { parent, sequence }
   definition   the item spec's authored id — a REFERENCE, never a copy
   ```

   ⛔ no position, no velocity, **no component snapshot** — that would be
   rollback wearing save's clothes. *"A hand needs strictly less than a world"*
   held: a held object has no place, the hand supplies one.

   ⭐⭐ **the third field is the one nobody would have predicted, and it is the
   durable-save lesson.** An instance rebuilt without its `SpawnOrigin` cannot
   say which spawner it descends from — the state that component's own doc
   refuses to let anyone spell — so it would survive exactly ONE death and then
   be invisible to the next capture. ⇒ **a description that restores the thing
   is not sufficient; it must also restore the thing's ABILITY TO BE DESCRIBED
   AGAIN.** The mint site was not stating provenance at all, so identity and
   provenance are now minted together and *"dynamic, parent unknown"* stays
   unspellable.

   ⭐ and the snapshot-versus-registry property is MEASURED, not asserted: turned
   into a growing registry of every mint, the banked-item fixture stayed green
   and `a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death`
   went red. The baseline answers HOW to rebuild; the custody baseline still
   decides WHETHER and INTO WHOSE HAND. Schema 32 → 33.

   ✔✔✔ **AND THE THIRD HORIZON LANDED (2026-08-16, `28c927505`) — ALL THREE NOW
   EXIST AND ARE DISTINCT.** A save carried counts, flags and a body position and
   nothing about what became of anything the world authored, so everything above
   survived a death and evaporated on a load.

   ⭐⭐⭐ **THE RESULT WORTH KEEPING: the on-disk form is the CHECKPOINT'S OWN
   DESCRIPTION, SERIALIZED — not a fourth description of the same facts.**
   `AmbitionGameSaveData` gained three `#[serde(default)]` lists that are
   `AuthoredOccurrences`, `CustodyBaseline` and `MintedItemBaseline` field for
   field. **That the file needed no field the checkpoint had not already measured
   is the finding, not a convenience**: identity + provenance +
   definition-REFERENCE was derived from what a checkpoint owes a hand, and a save
   asks the same question — *how would you make this again?* — and gets the same
   answer.

   ⭐⭐ **AND A LOAD IS A CHECKPOINT RESUME.** `restore_durable_horizon` adopts
   the ledger and the three baselines from the file and writes one
   `ResetToCheckpoint`; everything after that is the road a death already takes.
   ⇒ the durable slice is **two systems and no reconstruction logic**, and there
   is still exactly ONE authority on what a room owes the world.

   ⭐ **the defect only the fixture could find, and its fix is the reusable
   part**: a session builds its start room BEFORE any file is read, so the instant
   the loaded ledger arrives the world holds an occurrence the file says is
   elsewhere — and the placement recorder republished the stale position over the
   loaded row, sending the object back to the room the player carried it out of
   and resurrecting terminal rows by the same tick order. The fix is an
   **INVARIANT, not a filter**: an occurrence comes to rest here only if its row
   says `InCustody` or already says `Placed` here, *because an object cannot
   change rooms without being carried*. ⛔ it REFUSES rather than repairs, so it
   does not become a second reconstruction authority.

   ⚠ disk added exactly one constraint the memory horizons did not have: **INTEGER
   pixels**, because a float costs the save's `Eq` derive and a `NaN` makes the
   value-comparing autosave rewrite the file every frame forever.

   ⇒ **still open, and these are the durable frontier now**: a runtime mint **not
   in a hand** at save time is undescribed and lost (the description remembers no
   position — exactly where *"a hand needs less than a world"* stops paying);
   `Consumed` round-trips through the file and still has **no live producer**;
   `load_save_at_startup` is still presentation-only, so a headless composition
   mirrors into `AmbitionGameSave` and never writes a FILE; and ⚠ **the body
   resumes at the shrine while the objects resume at the autosave's instant** —
   two different times in one load.
   ⛔ **do not promote easy actor-monolith leaf carving ahead of this.**
2. **Simulation authority and determinism.** Decompose parameter-ceiling systems
   by phase/authority and invert rollback declaration ownership. See
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).
3. **⭐ NEW 2026-08-15 — deterministic authored gameplay logic and orchestration**
   (D127). Authoring is strong for **nouns** and weak for **verbs and
   relationships over time**; several independent partial condition → effect
   systems already exist in tree, which is what promotes this from an abstraction
   idea to a named capability gap. **Rust extends the engine's vocabulary;
   authored content composes vocabulary that already exists.** See
   [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).
   ✔ **M0 is complete** (14 systems inspected).
   ⛔ not scripting, not a rule VM, not a central effect enum. ⭐ M0's headline:
   **the substrate owns no universal sequencer** — the gap is on the *condition*
   side, and boss patterns are the **template**, not a customer.

   ✔✔ **M1 IS MET FOR CONDITIONS, WITH TWO UNRELATED CONSUMERS (2026-08-15/16).**
   ⛔ this page said "M1 is parked" for a day after its acceptance was satisfied.
   `shared_tangle::authored_logic` owns the contract — `publish` is PRIVATE, the
   only way in is `PublishCondition for App`. Three domains publish
   (`custody.is_held`, `world.flag_set`, `inventory.holds`); two very different
   consumers ask: a **gated lock wall** and **authored `.yarn` dialogue**, the
   latter through one generic verb `condition("domain.question", <arg>)` that
   names no question, no domain and no flag. ⇒ publishing a condition makes it
   askable from dialogue **with no edit to any bridge**, which is the same
   behavioural acceptance the second provider had.

   ⛔⛔ **and it refuted a premise THREE module headers asserted**: that Yarn
   library functions cannot be Bevy systems and so cannot reach `&World` — the
   entire reason `YarnStateMirror` existed. False at HEAD: `bevy_yarnspinner`
   advances the interpreter from an **exclusive** system and threads `&mut World`
   to `YarnFn::call_with_world`, and `SystemId<In<P>, O>` implements `YarnFn`.
   ⭐ **three files agreeing is one observation, not three, when the later two
   were written by reading the first.** The mirror SHRANK to a projection rather
   than gaining a feed; two hand-written functions, a per-frame refill, a
   duplicated id rule and an alias table went with it.

   ⇒ ⭐ **COMMANDS ARE A DIFFERENT SHAPE, and that is now established rather than
   assumed.** A condition is safe to call from inside the interpreter *precisely
   because it cannot change anything*. A command mutates, so calling it the same
   way is a presentation-side write into rollback state on a frame the sim does
   not replay — the exact defect `NarrativeInputLedger` was built to fix, and why
   `<<give_item>>` records a REQUEST rather than granting. A `PublishCommand`
   contract owes **authority, ordering and a ledger-shaped replay story**, and
   generalises from `NarrativeInputPlugin<M>`, not from the condition catalog.
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
6. **Capability/runtime composition — ⭐ PROMOTED TO THE LEDGER 2026-08-16 as
   D136, and its evidence arrived all at once.** Make optional capabilities
   honest in dependency and composition topology. See
   [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

   ⚠ **this sits at 6 by the 2026-08-15 ordering and the ordering has NOT been
   re-litigated — but read this before trusting the rank.** Five independent
   slices on 2026-08-16 turned out to be one failure: the engine cannot ship the
   art it draws (every sprite-registration site is a *game* system); the
   durable-save leg exists only in the visible binary, so one of two persistence
   authorities is absent from every headless harness; the shell composes every
   experience into one process, so a demo's rules reach another demo's fighter;
   a crate edge two policies forbade turned out legitimate only because a facade
   deletion converted a laundered edge into a declared one; and the canonical
   session world carried a format-specific field five games filled with
   `::default()`.

   ⭐ **the through-line: none is a bug in the ordinary sense — each is a place
   where "who is this for?" was answered by whoever installed it first and never
   written down.** ⇒ the composition-shaped slices have been paying more than the
   feature-shaped ones: naming a schedule order let `conversation` leave the
   monolith, giving the engine a home for its own art made 189 shipped effects
   reachable, and making a load a checkpoint resume deleted a whole
   reconstruction road instead of adding one.

   ✔✔ **THE MECHANISM EXISTED ONE NOUN SHORT (2026-08-16, `03d4c8d22`) — that is
   the reusable finding.** `DeathRules` was a bare `Resource` inserted at
   plugin-build time by THREE games; the type was the key, so the third
   `insert_resource` won. The shell composes Mary-O after Sanic, so **every Smash
   match in the shipped host ran under her 3.2s level replay**, in an arena whose
   own rules want `LevelReset::Never`. Inert only because an `Unbounded` fighter
   writes no death message.

   ⛔ **the shell's `ExperienceScopeBuilder` was the WRONG tool** — it releases
   what a session published, on route DEPARTURE, and has no ENTERING half.
   ⭐⭐ **the right one already existed and was one noun short**:
   `runtime::mode_scope` scopes a hosted game's **SYSTEMS** and its **ENTITIES**
   to the rooms tagged with its mode — every Sanic and Mary-O system is already
   gated through it — **and it had no word for a RULE.**

   ⇒ a game now declares into `DeclaredDeathRules` under the rooms it governs,
   using the same answer its `RulesPlugin` already gives when gating systems;
   `governing(mode)` is the one place the question is answered; an unclaimed room
   reads the engine default and a second claim on one scope **panics at build**.

   ⭐ **and the second instance wanted a DIFFERENT, smaller cure** — Sanic's
   wallet shield is not a global but a system whose POPULATION was every
   `PrimaryPlayer` in the process. Its `hosted` fork is deleted outright: the
   standalone binary loads the same mode-tagged speedway, so the flag was
   answering a question the room already answers.

   ⇒ **the lesson to carry: when a scoping concept exists, ask what KINDS of
   thing it scopes, not whether it exists.** Systems and entities were covered;
   rules were not, and nothing said so.

   ⛔ **the standing number, and it has not moved DOWN**:
   `capability-footprint-may-not-grow` reads **44 crates linked, 17 a
   movement-only game never asked for** (measured 2026-08-18). ⚠ it said 42/15
   here until then — the ratchet forbids GROWTH and the closure grew by two
   inside its own rule while this page kept quoting the older pair. ⇒ **re-read
   the contract's own output before quoting it**; a number copied into prose is
   a snapshot, and this one is printed by `check_absence_contracts.py` on every
   run. ⛔⛔ **and D135's answer for WHY was
   incomplete in a way that cost a slice**: it named the monolith, and
   `cargo tree -i ambition_platformer2d_ldtk` in the sentinel's own workspace
   names **four** holders — the facade itself (unconditionally),
   `ambition_platformer2d_actor_monolith`, `ambition_platformer2d_runtime`, and
   `ambition_platformer2d_provider`, which declared the edge and named zero of
   its symbols. D136 (2026-08-16) moved `WorldManifest` down to
   `ambition_platformer2d_world`, deleted the provider's dead edge and made the
   facade's optional again; the number did not move, because the monolith and
   the runtime still named the backend unconditionally.

   ⭐ **RE-MEASURED 2026-08-18 and the monolith is off it.** `cargo tree -i` in
   the sentinel now names **two** direct holders — the monolith and the runtime —
   and the monolith's production code names `ambition_platformer2d_ldtk` ZERO
   times (three runtime readers inverted onto the room IR on 08-17/18; the one
   file that still greps is a comment saying the backend is not there). Its
   remaining `bevy_ecs_ldtk` surface is one module, now behind `ldtk_runtime`,
   and the crate builds with `--no-default-features` for the first time.
   ⚠ **the FOOTPRINT number still did not move** (44 linked / 17 unwanted): the
   dep was already declared optional, so no Cargo edge changed — what changed is
   that the declaration became true. ⇒ the runtime is the remaining holder. ⇒ **a slice claiming this
   front must run `cargo tree -i` for the crate it means to evict BEFORE picking
   what to carve**, and must say what it did to the number or why the number is
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
  2026-08-18, and they are what makes D166 the next architectural move.** A cold
  coordinator needs four lines and no commit list:

  ```text
  landed        acquisition · the hold · pose · pummel · throw · release ·
                a bounded hold with a captive escape channel · a control-hold
                claim registry so one authority cannot free another's body
  proven        a REAL match, driven: 14 holds, 2 pummels, all ended by throw
                (`cargo run -p ambition_demo_smash_app --bin capture_probe -- 60 --force`)
  open          WHEN a CPU presses Grab. It owns one, chooses one and presses
                one — mostly at ~110px against a 42px reach. Fighter capture
                POLICY, not a bug in the mechanic.
  ⛔ do not     price a grab's damage to fix that. Tried: "worth its forward
                throw's damage" made the CPU grab from 110px in every exchange,
                nine attempts, none in range, zero holds. A grab deals NO damage.
  ```

  ⇒ the value of a hold depends on the throw it sets up, the escape risk, the
  percent and the stage — and the generic option scorer, shared by every actor
  in every game this engine runs, has no term for it and should not grow one.
  **That is D166's customer**, and the five other pressures capture put on
  generic structures are listed in that row.

  ⭐ **and D166's FIRST FACET landed the same day** (`5cefafc05`): George's
  capture kit is authored content — `game/ambition_demo_smash/assets/fighters/`
  — read by the content compiler, prepared into the same `MoveSpec`s the Rust
  literal produced. What that moved is the VALUES. ▢ the POLICY half above is
  still open and is still the interesting one. ⛔ the sixteen ordinary move
  slots deliberately did NOT move: they are authored by COMPOSING helpers, and
  George's file states a law about the shape of his table beside them.
- **TwinTrack:** strongest current pressure test for independent views and
  observer/reference-frame presentation; split-screen should exercise the same
  multi-view model Ambition uses.
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
- views are local presentation over one simulation, not duplicate worlds;
- transport, control assignment, world residency and view layout are independent
  axes;
- LDtk is Ambition's preferred spatial authoring surface and should improve when
  real Ambition content outgrows it;
- the actor monolith is drained by coherent ownership, not line-count quotas;
- public APIs should expose game concepts rather than historical crate topology.

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
