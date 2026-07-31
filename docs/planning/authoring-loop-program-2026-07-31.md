# The authoring loop — content packs, participant actions, causal inspection

**Armed:** 2026-07-31, on Jon's instruction. This file is the SPINE for the
three-program architecture campaign; rows land here and are mirrored as `▢` in
[queue-72h-2026-07-31.md](queue-72h-2026-07-31.md), which the guard reads.

**The objective, in Jon's words:**

> An agent should be able to create a mechanically and aesthetically distinct
> game from the public SDK and content documentation, validate it quickly
> without rebuilding the engine, control it consistently across gameplay and UI
> contexts, and diagnose why any important simulation result occurred without
> reading engine implementation code.

One loop, not three subsystems:

```text
author → validate → control → simulate → explain
```

---

## What already exists — read this before building anything

This program is **not** starting from zero, and the largest risk to it is
rebuilding a substrate that is already here under a different name. Surveyed
2026-07-31 against source, not against docs.

| Program | Already built | Where |
|---|---|---|
| A | authored RON catalog + parser + App-local fragment registry that MERGES across providers | `ambition_characters::actor::character_catalog` |
| A | a 698-line cross-content validator (LDtk links, dialogue ids, quest conditions, encounter/boss ids, music refs) | `game/ambition_content/src/content_validation.rs` |
| A | `UnresolvedRef` + `explain(id, declared_by)` — the resolution-diagnostic shape, twice | `ambition_platformer_primitives/src/binding.rs:451`, `ambition_render/.../item_visuals.rs:139` |
| A | `PreparedCharacterDefinition`, `PhysicalRetraction`, `DisplacedPhysicals` — preparation/lowering already exists for ONE family | `ambition_actors/src/character_runtime/` |
| B | `ParticipantId`, `InputParticipant`, `InputContextId`, `ContextClaim`, `ParticipantContexts`, `resolve_active_input_context` | `crates/ambition_input/src/participant.rs` |
| B | `SeatMenuFrames` — seat-keyed menu input with per-seat repeat state | `crates/ambition_input/src/menu.rs` |
| B | `UiCue`/`ActiveUiCues` — context-keyed prompt projection | `crates/ambition_input/src/cues.rs` |
| B | `KeyboardPreset::input_map()` → `InputMap<SandboxAction>`, `action_label()` | `crates/ambition_input/src/presets.rs` |
| C | a rolling flight recorder: frames + discrete events + markdown/JSON dump | `crates/ambition_gameplay_trace/` |
| C | `AMBITION_FIGHTER_TRACE=1`, `ladder_probe`, the rollback observatory | fighter brain, `app_it` |

**So the work is not "build three systems". It is: find where each substrate
stops being an authority, and move the boundary.** The four places it stops are
below, each with the evidence that says so.

---

## The four load-bearing gaps, with evidence

### ⛔ A-gap 1 — content reaches the runtime through `include_str!`

`game/ambition_content/src/character_catalog.rs:12`:

```rust
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");
```

and the public facade's content seam takes the same shape —
`CharacterContent::Ron(&'static str)` in `crates/ambition/src/app.rs`.

**Editing a character therefore requires a Rust rebuild (~10 min in this repo).**
The program's headline acceptance criterion — *"valid character addition →
validates without rebuilding Rust"* — is false today, and it is false **at the
public API**, which is the stated priority trigger *"the public API is about to
harden around the wrong concept"*.

⚠ do not read this as "delete `include_str!`". A shipped binary embedding its own
content is correct for distribution. What is wrong is that it is the ONLY path,
so the authoring loop inherits the distribution constraint.

### ⛔ A-gap 2 — validation is a fixed-arity function, not a registry

`validate_content_graph(&music, &project, &character_catalog)` is a function of
three hardcoded content families. A capability cannot register a schema; adding
a content family means editing this signature and its body. There is no pack
identity, no fingerprint, no canonical ordering, no capability-requirement
check, no source manifest, no alias/symlink dedup, and **no CLI** — the only
caller is a `#[cfg(test)]` block at line 655.

### ⛔ B-gap — `ActiveInputContext` is a global resource resolved from seat ZERO

`crates/ambition_input/src/participant.rs:167` and `:190`:

```rust
#[derive(Resource, ...)]
pub struct ActiveInputContext { open: Vec<InputContextId> }

// resolve_active_input_context:
participants.iter().find(|(p, _)| p.id == ParticipantId::PRIMARY)
```

The claims are per-participant and correct. **The resolved answer is not.** One
global context stack, computed from player one, governs everyone. Consequences
that are already reachable:

* seat 2 opening a menu is invisible to the router — its actions keep flowing to
  gameplay;
* seat 1 opening a menu silently takes gameplay away from seat 2;
* a participant with no controlled body cannot own a context distinct from the
  primary's.

This is the same defect class the repo already fixed one layer down: `SeatMenuFrames`
exists precisely because `MenuControlFrame` was "ONE global answer". The context
resolution never got the same treatment.

⚠ **two authorities currently disagree** — the second stated priority trigger.
`ParticipantContexts` says input ownership is per-seat; `ActiveInputContext` says
it is global. Both ship.

### ⛔ B-gap 2 / C-gap — the action vocabulary is closed, and causality is TEXT

`SandboxAction` (`crates/ambition_input/src/actions.rs`) is a closed
`#[derive(Actionlike)]` enum behind the `input` feature. A capability cannot add
a semantic action; gameplay and menu actions share the enum, split by
convention and by which system reads them.

And the causal substrate is a **player-shaped** flight recorder:
`GameplayTraceFrame` carries `PlayerTraceState`. That is player-centrism in the
observation layer — the one thing Ambition's core values name first. There is no
`explain_tick`; `git grep 'fn explain'` returns two unrelated ref-resolution
helpers. The fighter's decisions are observable only as **printed lines** under
`AMBITION_FIGHTER_TRACE=1`, which is the exact thing Program C forbids
("do not build the inspector by parsing text logs").

---

## Where this lands architecturally — the hole is already reserved

`crates/ambition/src/app.rs:435`, written before this program existed:

> Slice A holds only what host assembly consumes. **`ContentPackDraft` and
> everything under it is slice B**; a content method here would be a method
> whose input nothing can yet validate.

ADR 0032 already named `ContentPackDraft` and deferred it for the stated reason
that nothing could validate it yet. Program A **is** that validator. This is not
a new layer bolted beside the facade; it is the facade's own declared next slice.

---

## Sequencing — three connected vertical slices

Each slice ends with: the displaced path deleted, one positive test, several
negative tests, and the API leaks recorded. No slice waits on another's polish.

### ▢ P1. Slice 1 — one compiled character, its actions, one explanation

The narrowest end-to-end cut of all three programs at once.

**P1a — `PreparedContentPack` is a value, and a character validates off disk.**
New engine crate `ambition_content_pack`:

```text
ContentPackDraft
  ↓ parse            (source manifest: every file read, canonicalised, deduped)
  ↓ schema resolution (SchemaId → installed handler, or UnknownSchema)
  ↓ reference resolution (UnresolvedContentRef<T> → ResolvedContentRef<T>)
  ↓ capability validation (declared requirement → installed capability)
  ↓ conflict detection   (duplicate canonical identity, ambiguous ownership)
  ↓ canonical ordering
  ↓ fingerprint
PreparedContentPack
```

Carries: pack id + version, module namespace, source manifest, declared
capability requirements, schema versions, content identities, asset identities
+ provenance, canonical ordering, resolved references, diagnostics, fingerprint.

First registered schema: the character catalog, owned by the characters
capability. First migration target: `game/ambition_content`. **Do not redesign
every content family before one pack works end to end.**

**Guard:** `cargo test -p ambition_content_pack` green, and a `content_pack`
module in `app_it`.

**P1b — the validator has ONE implementation and a CLI front door.**
The same `validate` used by the standard test, CI, the CLI, dev reload and
packaging. The CLI is a diagnostic door, not the enforcement.

**Acceptance, and it is a measurement not a claim:** build the validator once,
edit a character RON, re-run — **no cargo invocation** — and get the new verdict.
Record the latency. Then probe each hard error red: unknown schema, missing
preset, missing asset, unresolved character/role/action/world ref, duplicate
identity, uninstalled required capability, an unknown field (which must NOT be
silently ignored), and the same file reached through a symlink.

**Guard:** a `validate_without_rebuilding` test that shells the built binary,
and the eight negative cases named above.

**P1c — the participant's semantic gameplay actions route to that character.**
Depends on P2a below only for the multi-seat case; the single-seat route can
land first.

**P1d — explain one movement and one attack decision.**
`harness.explain_tick(tick, actor_id)` answering *why did this body move* and
*which authored move was scored vs which began playback*. Headless API first;
the panel is later and smaller.

**Guard:** an `explain_tick` module in `app_it`.

### ▢ P2. Slice 2 — the multi-context participant flow

**P2a — `ActiveInputContext` becomes per-participant.** The B-gap above. This is
the single highest-payoff row in Program B because it is a *contradiction between
two shipped authorities*, not a missing feature, and every later context (select,
pause, dialogue, inventory, cutscene, debug) inherits the wrong answer until it
moves.

⚠ **the primary's global resource cannot simply be deleted** — it has readers
that legitimately want "the shell's answer". Give it an owner (the same shape as
`MatchParticipantRoster::published_by`) rather than two meanings.

**P2b — the real contexts exist as first-class ids**, not route-specific
branches: `select`, `pause`, `dialogue`, `inventory`, `cutscene`, `debug` beside
today's `shell.startup_acknowledge` / `shell.launcher` / `gameplay`.

**P2c — bindings are ONE authority** for routing, remapping, glyphs, touch
affordances, prompts and help. `action_label()` and `ActiveUiCues` are the two
halves that exist; they must read the same binding the router reads.

**P2d — the flow:** launcher → character select → match → pause → return, with
**two local participants** joining independently, navigating, selecting, playing,
and pausing **without stealing each other's input**. Smash's select screen is the
consumer and it already exists — this proves the seam on a real screen.

**Guard:** a `participant_contexts` module in `app_it`; the two-seat
no-input-stealing case probed red.

### ▢ P3. Slice 3 — a capability-owned schema, action, and causal fact

One capability contributes the full set: behavior + authored schema + semantic
action + rollback registration + schedule participation + causal facts —
**without editing a central character enum or actor monolith**. That last clause
is the test.

**Guard:** the capability lives outside `crates/ambition_actors`, its schema is
authored from content, and `explain_tick` reports its fact.

---

## Shared contracts these three must not duplicate

* **Prepared identities.** The compiler assigns the stable resolved identity;
  actions, characters, moves, roles, capabilities, causal facts and diagnostics
  all quote it. The inspector DISPLAYS those identities — it never reconstructs a
  name from runtime internals.
* **Determinism.** Anything authoritative registers rollback state, participates
  in deterministic scheduling, appears in the schema fingerprint, resets on
  lifecycle replacement, runs headless. ⚠ this repo has been bitten repeatedly by
  the opposite: a derive's memo, a change tick, an unordered reader. Instrumentation
  is observer-only or it is simulation.
* **Cost.** Retained causal history is bounded and the expensive domains are
  feature-gated. CI must be able to run the inspector without rendering.

## What is deliberately NOT in scope yet

* A capability dependency/conflict solver (deferred on the same evidence as the
  API campaign deferred it — no consumer blocked).
* Full multi-experience asset virtualization.
* Every rebind UI and every device backend.
* Migrating every content family. One pack, end to end, first.

## Sentinel consumers — used progressively, not as a gate

1. `game/ambition_content` (content-heavy pack) — P1's migration target.
2. `fixtures/minimal_game` (minimal external platformer).
3. Smash select → gameplay, two participants — P2's consumer.
4. Smash fighter decision + stock lifecycle — P1d's and P3's consumer.
5. A traversal-focused actor (Sanic or Mary-O) — proves the inspector tolerates
   missing combat domains.
6. One custom movement/mechanic capability — P3.

**The program is not complete until the API has survived several genuinely
different consumers.** Do not declare it on the strength of one.

## Measurements to record as we go

Engine files opened · compile iterations · validation latency · internal imports
attempted · missing documentation · unresolved diagnostics · context consumed ·
whether the result is mechanically distinct. These guide refinement; they are not
a reason to stop product work for universal proofs.
