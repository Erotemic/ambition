# Actor monolith decomposition

**Status: OPEN — active incremental engine work.**

This plan replaces the July-era conclusion that no further
`ambition_platformer2d_actor_monolith` carve was currently justified. That
conclusion was useful while the remaining crate still behaved like one tightly
woven actor authority. It is no longer true of the current tree.

At HEAD `43373f72ddc2` on 2026-08-07:

- `crates/ambition_platformer2d_actor_monolith/src/**/*.rs` is **110,911 lines**;
- the generated `.agent` packet reports **42 root modules**, **176 registered
  systems**, **47 resources**, and **28 normal internal `ambition_*`
  dependencies**;
- [`../../recipes/cheapest-sufficient-check.md`](../../recipes/cheapest-sufficient-check.md)
  records the durable focused-validation rule; the archived 2026-08-02 campaign
  measured the actor monolith as a costly recompilation unit with repository
  incremental compilation disabled;
- the public-API campaign's movement-only consumer still inherited 15 capability
  crates it did not request because the actor monolith brought them into the
  resolved graph: audio, cutscene, dialog, encounter, items, LDtk, menu,
  persistence, portal, projectiles, settings menu, SFX, SFX bank, UI nav, and
  VFX. See [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
  and [`public-sdk-1.0.md`](public-sdk-1.0.md).

The decomposition trigger has therefore fired for **two independent reasons**:
consumer dependency leakage and compile-unit cost. The work is no longer
conditional on discovering whether a carve is justified. The remaining design
question is where each incremental boundary belongs.

## Goal

Drain the current monolith until the remaining crate is honestly the reusable
actor/body/control simulation domain, then rename that residue to a durable
actor crate.

The desired property is not merely fewer lines. Editing movement or body control
should not require recompiling dialogue, menus, audio, persistence, LDtk
integration, encounter orchestration, character presentation, and other
independent capabilities. Likewise, changing dialogue or presentation should not
recompile the movement implementation merely because both once lived under the
same package.

The public `ambition_platformer2d` facade remains the compatibility boundary.
Internal crate ownership may change without forcing consumer-facing module paths
to mirror the historical implementation layout.

The immediate kernel priority is
[`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md).
Do not widen the open-world/multiplayer architecture around protagonist-special
simulation if the controlled body can first be made ordinary.

Every new carve should also be evaluated with
[`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md):
registration should move with the domain plugin, and a generic crate should be
usable in a small Bevy `App` without importing Ambition content or this monolith.

## Operating rule: subtract one concept at a time

This is a **ratcheted subtraction campaign**, not a flag day.

A good slice makes the actor monolith stop knowing about one coherent concept.
The strongest closure evidence is usually one or more of:

1. a direct dependency disappears from the actor monolith's `Cargo.toml`;
2. the movement-only consumer's resolved capability footprint shrinks;
3. an internal cycle or shared-authority knot disappears;
4. a substantial independent compile unit can rebuild separately;
5. a historical compatibility facade or duplicate path is deleted.

Moving files without improving any of those measures is not a successful carve.
Do not create a new forwarding crate that simply imports every extracted domain
back into the monolith; that preserves both dependency leakage and compile
fanout.

Prefer an existing owning crate when one already exists. Create a new crate only
when the extracted code names a durable reusable engine concept with a clear
dependency direction and a plausible independent consumer/test surface.

## Evidence used to choose each slice

Use the repository's generated navigation data before inventing a boundary:

- `.agent/index/crates/graph-declared.json` — declared crate edges;
- `.agent/index/crates/graph-resolved.json` — resolved build graph;
- `.agent/index/crates/ambition_platformer2d_actor_monolith.json` — files,
  symbols, declared dependencies, tests, and module map;
- `.agent/ecs_inventory/crates/ambition_platformer2d_actor_monolith.{md,json}` —
  module ECS ownership, schedule registrations, message producers/consumers, and
  resource readers/writers;
- `crates/ambition_platformer2d_actor_monolith/MODULES.md` — intended concern of
  each root module;
- [`../../../dev/journals/code_smells.md`](../../../dev/journals/code_smells.md)
  — known duplicate mechanisms, compatibility debris, and second-order
  monoliths.

Generated data is navigation evidence, not source authority. For an extraction
candidate, confirm the important edges in Rust before changing ownership.

For each candidate, record at least:

```text
module/domain
    approximate size
    outgoing actor-module dependencies
    incoming actor-module consumers
    external crate dependencies
    resources it mutates / messages it owns
    expected Cargo edge(s) removed
    expected compile-isolation payoff
    destination owner
```

Incoming edges and shared mutable authorities matter as much as imports. A small
module with twenty consumers can be harder to peel than a larger leaf. The ECS
inventory already exposes several high-coupling authorities — for example
`AmbitionGameSave`, `OwnedItems`, `SlotInteractionState`, `ActiveConversation`,
and `ControlledSubject` — that must not accidentally acquire two owners during a
carve.

## Measured carve candidates (2026-08-14)

Ranked the monolith's 28 `ambition_*` dependencies by use sites in its own
source. Two ends of that ranking are worth recording.

**`ambition_sim_view` — refused, and the reason was already in place.** One use
site, and it is inside a doc comment. But it is declared under
`[dev-dependencies]` with a comment saying so: the headless camera example
resolves its snapshot through it, dev-only, and cyclic dev-deps are fine. Not a
production edge, nothing to remove.

**`ambition_platformer2d_ldtk` — a DECLARED, UNFINISHED migration, now mostly
repointed.** The monolith's `world/ldtk_world/mod.rs` was a blanket re-export
that ~60 call sites relied on instead of depending on `ambition_platformer2d_ldtk`
directly, and the provider/runtime/content declared no direct edge to it either.

### ✔ LANDED 2026-08-15 — the compat module is DELETED

**Deleted:** `world/ldtk_world/mod.rs`, `world/mod.rs`'s `pub mod ldtk_world;`,
and `ldtk_world` from `lib.rs`'s `pub use world::{…}`. All 89 sites across the
provider, runtime, content, app and monolith now name `ambition_platformer2d_ldtk`
directly (repointing them first, as closure evidence #5). Two dead policy
entries naming the old paths (`engine.portal-core-no-content-roster`) were
replaced by the one live name. `fixtures/minimal_game` was already in the
measured closure via the monolith's unconditional edge, so `closure_size`
stayed 42 / `never_asked_for` stayed 15 — the slice buys honesty, not footprint.

⇒ ✔ **RESOLVED 2026-08-16 (D136).** `WorldManifest` — the one consumer that had
blocked deleting the compat module, because `game_assets` used it
unconditionally while the facade's LDtk dep was optional — turned out not to be
LDtk-specific at all: every field is an `AssetId`, a path, a `&'static str` or a
bool. It moved to `ambition_platformer2d_world::world_manifest`, no re-export
left behind, and the facade's LDtk edge is `optional` again. ⚠ the resolved
graph did not move: `ambition_platformer2d_actor_monolith` and
`ambition_platformer2d_runtime` still name the backend unconditionally, so
closure stayed at 42/15 — the facade edge was one of four that hold it.

**Can the monolith's LDtk edge itself become optional? Measured: not by
gating.** ~20 production references across eight modules, several in public
signatures (`Platformer2dAssetCatalog::for_profile` took `&WorldManifest`), so a
feature gate would scatter `#[cfg]` through eight modules. ⇒ the answer is
relocation, not gating: what the monolith uses LDtk for is content LOADING —
world manifest, asset catalog, encounter loading — and that concern dissolving
by owner takes the dependency with it.

⇒ the asset-catalog six are gone as of 2026-08-16 (D136) —
`Platformer2dAssetCatalog::for_profile`'s public signature no longer names the
backend. ▢ **What is left is six production files**: `features/mod` and the
settings model (hot-reload state — not LDtk, a debounced mtime watcher wearing
the name), `menu/map/systems`, `world/gated_lock_walls`, and
`encounter/{loading,systems}` — the last three walk `LdtkProject`/`LdtkLevel`
and want the room IR instead (`encounter/systems`'s own comment already names
the plan: *"W4 will route encounter loading through RoomEmission instead of the
project"*). ⛔ Cutting all six still would not move the counter alone:
`ambition_platformer2d_runtime` installs `LdtkRuntimeSpinePlugin` and
rollback-registers `LdtkRuntimeIndex` unconditionally, so the runtime holds the
edge too.

## Slice procedure

For every incremental extraction:

1. **Recompute the slice-local graph.** Start from `.agent` data and source.
2. **Name the authority and lifetime.** State what is authoritative, what is
   derived, who mutates it, and whether it belongs to process, experience,
   session, participant, body, room, simulation tick, or presentation frame.
3. **Repoint compatibility imports first.** A dependency through
   `actor_monolith::foo` may only be a stale re-export of a lower canonical
   owner. Remove that false coupling before designing a new interface.
4. **Choose the destination by semantics, not directory shape.** Existing domain
   owners beat a new crate whose only rationale is file location.
5. **Put integration above the domains it joins.** If code genuinely needs both
   actor state and a leaf capability, keep the leaf model independent and place
   the adapter at a composition layer rather than introducing a wrong-way edge.
6. **Migrate all production consumers.** Avoid dual authorities and parallel
   implementations.
7. **Delete the displaced path.** Do not retain internal compatibility shims in
   a pre-release repository.
8. **Remove the old Cargo edge when the slice makes it unnecessary.**
9. **Run focused checks and remeasure.** The useful scorecard is dependency
   closure, actor-crate compile cost, and affected package tests — not a mandatory
   full-workspace suite.

## Current shape: peel the outside before splitting the actor kernel

The largest root modules at the 2026-08-07 baseline are:

| Root area | Rust lines under `src/` | Planning interpretation |
|---|---:|---|
| `features` | 37,884 | deeply coupled actor/NPC/boss ECS core; late carve |
| `character_runtime` | 12,543 | several lifetimes/owners hidden under one name |
| `boss_encounter` | 6,778 | encounter + actor + content orchestration |
| `avatar` | 6,716 | historical mixture that should dissolve by owner |
| `construction` | 5,124 | planner plus domain-specific recipes; split by owner |
| `abilities` | 4,826 | optional gameplay capabilities around body state |
| `character_sprites` | 3,989 → 1,760 | ⭐ **half of it left on 2026-08-09** — `{anim, posed_body, attack_hitbox}` are `ambition_character_sprites`, a SIBLING crate this one does not depend on. What remains is `assets.rs`, the actor/content join, which stays for the same reason `character_runtime` does |
| `world` | 3,324 | world/runtime/LDtk integration candidate |
| `items` | 2,388 | item/body integration; vertical carve |
| `schedule` | 2,375 | global ordering vocabulary; requires deliberate ownership |

⛔⛔ **HOW TO MEASURE A MODULE'S OUTWARD EDGES — the two obvious greps are each
wrong in one direction, and both have now cost a slice.** Measured 2026-08-17
while refusing the `encounter` carve:

- `grep "crate::"` **counts PROSE.** This repo's `//!` and `///` blocks cite
  module paths constantly. That reading nearly filed `conversation` as coupled
  when its five apparent edges were all doc comments.
- `grep "use crate::"` **counts a WRITING STYLE, not a dependency.** A module
  whose edges are inline fully-qualified paths in system signatures and plugin
  bodies reports near-zero. `encounter` reported **1** and has **9**;
  `items` reports 5 and has 14; `character_runtime` reports 3 and has 13.
  The undercount is **not uniform** — `boss_encounter` reports 3 and has 3 — so
  the column cannot even rank candidates relatively.
- ⭐ **the honest instrument is `crate::` paths on NON-COMMENT lines**, then
  **each surviving name chased to its `pub struct` / `pub fn`.** That second step
  is not optional: most of `encounter`'s nine sibling modules turned out to be
  pure re-exports of crates already BELOW the monolith (`ambition_combat`,
  `shared_tangle`, `platformer2d_core`, `ambition_characters`,
  `platformer2d_world`, `ambition_gameplay_trace`). ⚠ note `crate::combat` is a
  crate-level alias for `ambition_combat` (`lib.rs:99`), so a path that reads
  local is already cross-crate; and several re-exports are GLOBS, which name
  nothing textually at the re-export site.
- ⛔⛔ **Say which GRANULARITY you mean — a module count and a site count rank
  candidates differently.** A module count says how many seams a carve must
  cut; a site count says how much editing it costs, and the two can differ by
  3-4x for the same module. Neither answers the question that actually kills a
  candidate: how many sites point INWARD — relocating a domain's vocabulary
  into it RAISES the inward count, because laundered edges become declared ones
  (see Wave E).

⇒ **an edge count is a screening tool. The verdict is the definition sites**, and
a module is carvable only when every name it reaches resolves at or below where
it is going.

Do **not** begin by slicing `features` into arbitrary crates. The code-smell log
already found genuinely tight private coupling in the actor-update machinery.
Peel independent capabilities and misplaced outer-layer work first; the central
actor SCC should become smaller and more legible as its spokes disappear.

Two historical aggregate directories should probably **dissolve** rather than
become crates:

- `avatar`: body mechanics move to actor/body owners; participant/home-body and
  starting-character policy move to session/control/provider owners;
  presentation reactions move downstream;
- `construction`: retain a small neutral planning/construction vocabulary where
  useful, while actor, item, encounter, shrine, boss, and other domain-specific
  construction recipes move with their domains.

Likewise, `features` is not a candidate crate name. Its current module map says
it is the enemy/NPC/boss actor simulation. As capabilities peel away, the residue
should be renamed according to the actor concept it actually implements.

## Architecture specimen: Blink must be body capability, not session identity

The current default-Blink behavior is a concrete example of the ownership error
this campaign should eliminate.

The movement kernel correctly gates Blink on the body's capability. The bad
policy is upstream in `session/setup.rs`: when a character catalog row does not
author an ability set, body construction falls back to the session-wide
`EditableAbilitySet`. That developer/session mask defaults to the unrestricted
implemented ability set, including Blink. Consequently an omitted character
capability declaration can manufacture an intrinsic body ability.

The intended authority is:

```text
prepared character/body definition
    -> intrinsic BodyAbilities
    -> effective abilities after explicit session/dev restrictions
    -> movement/action systems
```

The protagonist may explicitly own Blink. Other controlled bodies do not acquire
Blink because control authority moved to them. A development restriction mask
may remove an intrinsic ability; it must not serve as the source of character
identity.

This is useful as an early seam correction because the newer prepared-character
path should eventually carry intrinsic body capabilities alongside motion model,
vitals, moveset, and other body identity. It also provides a concrete test of the
decomposition: answering “can this body Blink?” should not require session setup,
host policy, or protagonist-specific state.

Avoid using the historical word “sandbox” for this concept. Retain that word
only for literal artifacts or environments whose actual role is an experimental
sandbox. Prefer game, experience, session, runtime, host, world, room, dev policy,
or body capability according to the owner being described.

## Measured 2026-08-08 — the baseline holds, and the leak is not what it looks like

Re-measured before working the plan, per its own rule that generated data is
navigation evidence rather than source authority.

**The 2026-08-07 baseline is CURRENT, not stale.** 110,911 → **112,020** lines
(the narrative ledger and the item/shop appliers landed since), internal
`ambition_*` dependencies 28 → **29**. The largest root areas are unchanged in
rank and within ~200 lines each. Both decomposition triggers still hold.

### ⭐ The dependency leak is not 15 independent edges

The most useful thing measured: **how many monolith root modules name each
declared dependency.** A dep named by one or two modules is a carve that removes
a Cargo edge; a dep named by thirty is the actor kernel itself.

| declared dep | root modules naming it | reading |
|---|---:|---|
| `ambition_ui_nav` | **0** | ⛔ a conduit edge, nothing else — REMOVED, see below |
| `ambition_menu`, `ambition_gameplay_trace` | 1 | one module each (`menu`, `dev`) |
| `ambition_causal`, `ambition_cutscene`, `ambition_items`, `ambition_settings_menu`, `ambition_sim_view` | 2 | cheapest real candidates |
| `ambition_dialog`, `ambition_interaction`, `ambition_platformer2d_ldtk` | 3 | `features` is the third in each |
| `ambition_characters`, `ambition_platformer2d_core`, `ambition_platformer2d_shared_tangle` | 32–35 | the kernel; not carve candidates |

⚠ **`features` appears in almost every low-count row**, which is the same finding
the plan already states from the other direction: the outer capabilities are
nearly peeled, and what pins them is the actor-update machinery. A two-module dep
where the second module is `features` is a carve blocked on one file, not on a
domain.

### Slice landed: the `ambition_ui_nav` conduit edge

The monolith declared `ambition_ui_nav` and **named no ui_nav path anywhere** —
its own manifest comment said so. The dependency existed only so the `input`
feature could forward `ambition_ui_nav/input`, doubly redundant with two other
forwards that already enabled it. Removed: one declared Cargo edge, zero lines of
code moved — the plan's step 3, *repoint compatibility imports first*, in its
cheapest possible form.

⛔ **and the movement-only footprint did NOT shrink, which is the finding.** It
is still *40 crates linked, 15 a movement-only game never asked for*. `ui_nav`
still arrives via
`minimal_game → ambition_platformer2d → …_actor_monolith → ambition_dialog → ambition_ui_nav`.
So **the 15 leaked capability crates are not leaked by 15 removable edges.** Some
ride on another leaked crate, and cutting the direct edge changes nothing a
consumer can observe. `ui_nav` leaves when DIALOGUE leaves, and not before.

⭐ **Criterion 1 (a Cargo edge disappears) and criterion 2 (the consumer
footprint shrinks) are not the same measurement**, and a slice can honestly
satisfy the first while the second stays flat. Before claiming a footprint win,
run `cargo tree --offline --edges normal -i <crate>` from `fixtures/minimal_game`
and look at who else pulls it.

### The dialogue "domain" is two things with different owners

Measured 2026-08-08. The `ambition_dialog` namers inside the monolith are
`dialog/` and `conversation/`, and they do not belong in the same place:

- **`dialog/` is ONE file** — `yarn_bindings.rs`, 609 lines. ⭐ its own module
  docstring already says what it is: *"This module keeps only what is genuinely
  Ambition-side."* It is NAMED GAME VOCABULARY (`<<give_item>>`, `<<buy_item>>`,
  `<<challenge>>`), and `ambition_dialog` already exposes the
  `YarnContentBindings` installer seam precisely so a host pushes that from
  outside — `game/ambition_content` already pushes two installers through it
  (the duel, the cut-rope commands). **Its owner is the content crate, not a
  dialogue crate.**
- **`conversation/` is the reusable half** — the authority, the ledger, the
  hold, the break rule, the opening port — and it keeps the `ambition_dialog`
  edge wherever it lives.

⛔ So "lift the dialogue domain as one crate" is the wrong shape: there is no
single dialogue domain in the monolith, only a reusable continuity authority and
a pile of this game's Yarn verbs. Moving `yarn_bindings.rs` to `ambition_content`
by itself removes no Cargo edge and shrinks no footprint — it is an OWNERSHIP
correction, not a decomposition win. The measured win requires `conversation` to
leave, which is what removes the `ambition_dialog` edge.

### `ambition_conversation` is a COMPILE-ISOLATION win worth ~1%, and NOT a footprint win

**The payoff, as a number** (`scripts/compile_ratchet.py`, 2026-08-08):

```text
largest recompilation unit    111,579 → 109,412    −1.94%
edit cost, rest of monolith   248,672 → 246,505    −0.87%
edit cost, conversation       248,672 → 248,672    ±0.00%
```

The carved crate lands BELOW the monolith, so editing `conversation` still
rebuilds everything above it, which is the whole monolith. The carve makes every
OTHER edit marginally cheaper and dialogue work no cheaper at all.

⚠ **Read `critical_path_crates` in HOPS, not seconds.** A carve that inserts a
layer *lengthens* the serial chain even while every size metric improves — but a
naive chain-of-durations overstates the wall-clock cost: rustc releases a
dependent as soon as the predecessor's `rmeta` lands, so only the FRONTEND is
serial across a chain edge and codegen overlaps everything downstream (measured
377.9s naive vs. 210.5s real, a 2.2x gap). Hops is the honest unit for the
regression; do not read it as seconds.

⭐ **Compile-time economics decide which lever a carve should pull — a cold build
and a rebuild are in different regimes** (`dev/ambition_dev_measurements/compile_units.jsonl`):

```text
                         work/8 cores   dependency floor   binding
  dev cold, 583 units        767.6s          418.9s        CORES
  dev rebuild, 57 units      123.9s          168.4s        THE CHAIN
```

Halving codegen saves 282.7s cold and 11.8s on a rebuild; halving the frontend
saves 101.1s and 61.6s. Codegen wins 2.8x on a cold build; the frontend wins
~4-5x on a rebuild (reproduced 3.6x/4.0x/4.1x/5.2x across four runs) — and the
rebuild is what an agent pays before one test runs. ⚠ the "work/8" row is
measured directly by summing the ledger; the dependency floor and every
halve-a-phase delta are produced by the collector's DAG simulation, not a
stopwatch — well-motivated by the `rmeta`-pipelining evidence above, but not
independently re-derived. The cheap confirmation, if this is ever load-bearing:
run a rebuild after a change that touches one leaf crate and check the model's
predicted wall against the measured one. This is the main argument for carving
at all, ahead of raw line count.

**Who names `ambition_dialog` inside the monolith, production only, 2026-08-08:**
`conversation/` (2,164 lines, two inward edges, both the bark), `dialog.rs`
(110 lines, zero `crate::` edges), and test code under `[dev-dependencies]`
which does not propagate.

⛔ **Asking "who names `ambition_dialog`?" is the wrong question — the right one
is "would the new crate itself still be pulled in?"** It would:
`minimal_game → ambition_platformer2d → …_actor_monolith → ambition_conversation
→ ambition_dialog → ambition_ui_nav`. Five production files in the monolith
consume `crate::conversation` — `features/{mod,npcs}.rs`,
`features/ecs/{mod,interact}.rs`, `schedule/input_systems.rs` — so
`ambition_conversation` is a non-optional dependency of the monolith and every
one of its own dependencies stays in a movement-only game's resolved graph. The
footprint stays at 15.

⭐ **The carve is a COMPILE-ISOLATION win; capability shedding is a separate
architecture obligation.** The maintainer has answered the product question
decisively: a game may compose the engine without a capability
(`maintainer-decisions.md`, 2026-08-08). That does not make a `dialogue` feature
sprinkled across the monolith/runtime a good design — optionality must follow a
coherent capability boundary with no hidden runtime/rollback edge. Measure the
consumer graph after each candidate boundary, every time, including your own.

**The work, in order:**

1. **The BARK PORT.** `rules.rs` is the only thing keeping `conversation` inside:
   it names `crate::features::npcs::npc_ambient_bark_line` and
   `crate::character_runtime::PreparedCharacterRegistry`, both answering *"what
   line does this character say"* — a CAST question, not a continuity one.
   Install a small port (a resource holding a fn, or a trait object) that the
   monolith fills; leave the cast lookup behind.
1b. ✔ **DONE 2026-08-16 — THE PLUGIN, missed until 2026-08-15.** `conversation`
   owned no registration: `features::FeatureInteractionSchedulePlugin` did all of
   it and interleaved three of its systems into an anonymous `.chain()` with the
   switch/chest systems. `conversation::ConversationPlugin` now owns
   `ActiveConversation`, `ConversationCutBark`, the `ConversationEnded` ledger
   install, the presentation pair and its three sim systems; the anonymous chain
   is replaced by `FeatureInteractionSet`, a seven-variant vocabulary in
   `ambition_platformer2d_shared_tangle` (deliberately BELOW the monolith, so a
   carved crate can still name it). Four schedule-graph tests assert the edges as
   the plugin composes them. See the 2026-08-15/16 section below.
2. ✔ **DONE 2026-08-17 — `conversation/` MOVED to `crates/ambition_conversation`.**
   2,734 lines, ten files, `mod.rs` → `lib.rs`, and **not one line inside them
   changed shape**: `use super::…` resolves to the crate root exactly as it
   resolved to the parent module, so every internal path survived the move
   untouched. The only edit inside the carved code was a log `target:` string
   that still spelled the monolith. ⚠ **`dialog.rs` did NOT go with it** — see
   the note under step 3.
3. ▢ **`ambition_dialog` is NOT yet a `[dev-dependency]`** of the monolith, and
   step 2 landing does not change that: `dialog.rs` (135 lines, `ui`-gated) is
   still a production namer of it. ⭐ that file is the obvious next slice and it
   is clean — **zero `crate::` edges**, naming only `ambition_dialog`,
   `ambition_input` and `ambition_platformer2d_shared_tangle`. What it costs is a
   `ui` feature on the carved crate forwarding to `ambition_dialog/ui`, which is
   why it was left out of a move that was otherwise a manifest.
4. ✔ **DONE — the rollback registration is repointed and the wire format did not
   move.** `rollback/domains/actors.rs` and `room_transition/commit.rs` now name
   `ambition_conversation::`, and `ambition_platformer2d_runtime` declares the
   edge. ⭐ **CONFIRMED rather than assumed**, as this step asked:
   `rollback-wire-format-is-frozen` reports the same 357 stable names and 85
   encoded types across 11 crates, and `rollback_schema_baseline.txt` needed no
   edit — it records SHORT type names, so the crate move is invisible to it. The
   one full path in the tree is `rollback_coverage.rs`'s `NarrativeInputLedger`
   entry, which is a `type_name` string and follows the crate.
5. ✔ **REMEASURED — and both halves of the prediction were WRONG, in opposite
   directions.**
   - `cargo tree -i ambition_dialog` from `fixtures/minimal_game` does NOT come
     back empty and never could: the monolith depends on `ambition_conversation`
     unconditionally, so `ambition_dialog` and `ambition_ui_nav` still arrive.
     This is the C4e correction above arriving in practice; the step's own
     expectation was written before it.
   - `capability-footprint-may-not-grow` went **15 → 16**, not 15 → 13. The
     ratchet stopped the change and was right to — a new crate name entered the
     sentinel's closure — and the baseline moved in the same commit with the
     reasoning written into it. ⭐ **the carve did not CAUSE the 16; it NAMED
     it.** The same code was already linked, inside the monolith, under a name
     the counter could not see.
   - ⛔⛔ **`critical_path_crates` went 12 → 13, and this plan predicted it would
     stay at 12.** Measured, not inferred: recomputing the first-party height
     with `ambition_conversation` folded back into the monolith gives 12, and
     with it carved gives 13. The chain is
     `conversation → ambition_dialog → ambition_ui_nav → ambition_input →
     ambition_platformer2d_core → ambition_geometry`, and inserting a layer
     under `ambition_dialog` pushed that whole tail down one hop. ⭐ **this is
     exactly the regression `critical_path_crates` is guarded for** — every size
     metric can improve while the serial chain, and so the wall clock, gets
     worse. ⚠ read it in HOPS, not seconds: rustc releases a dependent at the
     predecessor's `rmeta`, so a chain edge serialises only the frontend.
   - ⚠ **the ratchet baseline was NOT re-frozen.** It is frozen at
     `208cf8acf937` (2026-08-09) and reports **nine** findings, of which the
     critical path is the only one this carve caused — the other eight are eight
     days of unrelated growth (`ambition_platformer2d_actor_monolith` +10,390
     lines, `ambition_platformer2d_core` +5,391, `ambition_content` +11,883).
     Re-freezing here would launder all of them under a carve commit. Whoever
     re-freezes should say what they are blessing.
   - ⚠ **the seconds column for this crate is a PLACEHOLDER.** `ambition_binding`
     and `ambition_conversation` are unpriced, so the ratchet estimates them at
     the population median 2.9059 ms/line — and size predicts compile cost with
     R² = 0.12. `python3 scripts/compile_collect.py` is what makes those numbers
     real.

⚠ **the `ParticipantId` ↔ `PlayerSlot` correspondence is the carve hazard to
watch.** `conversation/opening.rs` briefly acquired an edge to
`crate::participant_seat` by deriving `ConversationInputOwner` itself, which
would have forced the carve to take that correspondence along or duplicate it.
It takes the owner as a parameter now. Anything else leaving this crate meets the
same wall, because `participant_seat` exists precisely because `ambition_input`
and `ambition_characters` are siblings that cannot see each other.

## 2026-08-08: `rollback/domains/*.rs` measured, then resolved 2026-08-18

Twelve files, 2,130 lines, one per capability domain, each registering that
domain's types for rollback by name. Three findings converged on it: it was the
whole reason `ambition_cutscene`/`ambition_items` were runtime dependencies at
all; the runtime cost more frontend compile time than the monolith despite being
7.6x smaller, with 74 of its 79 generic functions living here; and it was the
concrete blocker to splitting rollback declare/install (`T` not recoverable from
a `type_name`). ⛔ **The frontend-cost link was REFUTED same day by a subtraction
test** (`cargo clean -p ambition_platformer2d_runtime` with/without the 11
domain modules): removing 2,130 lines and 74 generic functions changed check
time by nothing measurable — `cargo check` (2.5s) does not exercise the same
work as a build's frontend phase (24.8s), so the causal claim needed
`-Z self-profile` it never got. The dependency-isolation finding stood.

✔ **RESOLVED 2026-08-18 by the domain-owned-rollback refactor below**, which
made the whole question moot rather than answering it: `runtime/rollback/domains/*`
is deleted outright, not merely decoupled.

## Measured 2026-08-08: the dependency leak is mostly NOT the monolith's

This plan opens by naming two independent triggers, the second being *"the
public-API campaign's movement-only consumer still inherited 15 capability
crates it did not request **because the actor monolith brought them into the
resolved graph**"*. Measured against the tree, **that attribution is wrong for
14 of the 15.**

`cargo tree --offline --edges normal -i <crate> --depth 1` from
`fixtures/minimal_game`, for every crate on the never-asked-for list:

| crate | direct dependents in the consumer's graph |
|---|---|
| **`ambition_platformer2d_ldtk`** | **the monolith, and nothing else** ⭐ |
| `ambition_items`, `ambition_cutscene` | monolith **+ the RUNTIME** |
| `ambition_dialog`, `ambition_encounter` | monolith + runtime + `sim_view` |
| `ambition_portal2d`, `ambition_projectiles`, `ambition_vfx`, `ambition_menu` | monolith + runtime + render/host/others |
| `ambition_audio`, `ambition_settings_menu` | monolith + `game_shell` |
| `ambition_sfx`, `ambition_persistence` | nine and eleven dependents respectively |
| `ambition_sfx_bank` | **`ambition_sfx` only — not a monolith dependency at all** |
| `ambition_ui_nav` | `ambition_dialog`, `ambition_menu`, `ambition_render` |

⭐ **`ambition_platformer2d_runtime` declares TEN of the fifteen**, and it is a
direct dependency of the facade. So carving a capability out of the monolith
removes one of several paths and the closure does not move — which is exactly
what the `ambition_ui_nav` removal measured, and what the aborted
`ambition_conversation` footprint claim got wrong.

### Rollback ownership is no longer the blocker; capability composition remains

✔ **RESOLVED 2026-08-18:** the concrete rollback declarations no longer live in
`ambition_platformer2d_runtime`. `RollbackRegistrar` is a backend-neutral floor
trait, each gameplay crate owns its own `register_rollback_state` function, and
the GGRS host implements the trait through `GgrsRollbackRegistrar`. The former
`runtime/rollback/domains/*` adapter census is deleted. This falsifies the old
argument below that generic `bevy_ggrs` registration forced the type list to stay
in the runtime: genericity constrains where monomorphisation happens, not who
owns the list.

The measured dependency pressure that motivated the investigation is **not
magically erased by that ownership repair**. `ambition_cutscene` and
`ambition_items` are still direct runtime dependencies today because the engine
host composes their public rollback offers. That is now a much narrower question:

```text
old coupling:
    runtime names Cutscene/Item concrete types + projections + backend install

current coupling:
    domain owns concrete types + projections
    runtime host calls domain::register_rollback_state(&mut registrar)
```

So the rollback **authority inversion is closed**. Making an optional capability
remove even that one composition edge belongs to the capability-installation
campaign: the composition site that elects to install a capability should also
elect to install its rollback offer. Do not move concrete registrations back up
to chase the footprint number, and do not introduce a type-erased second snapshot
layer merely to make the call dynamic.

### What this changes about the campaign

- **The compile-unit trigger stands.** The monolith remains a large recompilation
  unit; domain-owned rollback declarations neither worsen nor solve that.
- **The dependency-leak trigger is now narrower.** The runtime no longer owns
  gameplay rollback semantics. Remaining optional-capability edges are
  composition decisions, and should disappear when capability installation is
  made truly optional end-to-end.
- **The maintainer answer remains settled: capabilities are optional.** Use the
  domain-owned rollback seam as part of that composition rather than re-opening
  rollback architecture. See [`../maintainer-decisions.md`](../maintainer-decisions.md).

⚠ **the baseline's own split is stale**:
`scripts/baselines/capability-footprint-baseline.json` historically grouped many
of these dependencies under the monolith even when the runtime was another
route. The ratchet's live invariant (the SET may not grow) remains useful; do not
infer ownership from that annotation.

## ⭐⭐ Measured 2026-08-15: the internal module graph, and the ONE thing an import count cannot see

Re-derived the monolith's 44-root-module graph from source (comments and block
comments STRIPPED — without that, log-target string literals and doc citations
score `ambition_platformer2d`, a crate ABOVE the monolith, as a production edge
from ten modules). Counts are production `crate::…` references, with the crate
root's `pub use` aliases resolved to their owning module — `save`→`persistence`,
`shop`→`items`, `trace`→`dev`, `rooms`→`world` — because a sibling reaching a
module through a re-export is otherwise invisible.

**Every module with ZERO inward production references**, i.e. the leaf set:

| module | prod lines | outward mods | external crates it alone names | reading |
|---|---:|---:|---|---|
| `conversation` | 1,836 | **0** | `ambition_dialog` (with `dialog`) | ⭐ the only large module with zero edges in BOTH directions |
| `affordances` | 1,200 | 4 | — | a BRIDGE (input × body × world); its consumers are outside this crate |
| `menu` | 748 | 1 | `ambition_menu` | sole namer of `ambition_menu`; also names `ambition_platformer2d_ldtk` for the Map tab |
| `gravity` | 548 | 5 | — | 36 outward refs into `features`/`world`; a spoke, not a leaf |
| `snapshot_impls` | 431 | 6 | — | trait impls FOR other modules' types; dissolves, never carves |
| `action_scheme` | 351 | 4 | — | small; removes no edge |
| `cutscene` | 214 | 3 | `ambition_cutscene` (with `schedule`) | 214 lines; the runtime names the crate too |
| `config`, `dialog`, `participant_seat`, `host` | ≤114 | 0 | — | too small to be a slice |

⛔ **The leaf set is a trap unless you also count REGISTRATIONS.** The strongest
candidate by every import measure — `conversation`, zero edges out, zero in —
was pinned by the schedule: `features::FeatureInteractionSchedulePlugin` did all
of its registration and interleaved three of its systems into an anonymous
`.chain()` with the switch/chest systems, load-bearing but recorded only in
prose. `conversation` had no plugin of its own; its composition root was a
plugin named after switches and chests. Fixed by step 1b above
(`ConversationPlugin` + `FeatureInteractionSet`) — only ONE of the seven
`NarrativeInputPlugin` installs was actually conversation's, since a ledger
payload belongs to whoever consumes it.

⭐ **The durable lesson: a module with zero inward imports can still be pinned by
the schedule.** Count the registrations, not just the paths.

⛔ **The other leaves do not justify a carve on this evidence.** `menu` is the
sole namer of `ambition_menu`, but `ambition_menu` also arrives through
`ambition_render` and the host, so criterion 2 stays flat — the same lesson
`ambition_ui_nav` already paid for. `affordances`, `gravity`, `snapshot_impls`
and `action_scheme` remove no Cargo edge at all. The answer for every leaf
except `conversation` is NOT YET.

## Candidate extraction waves

These are **priority hypotheses**, not a promise to create one crate per row.
Recheck the current graph before each slice and change order when a cheaper,
higher-payoff boundary becomes available.

### Wave A — easy outer dependencies and stale facades

Start with leaves and false edges that can make the rest of the graph truthful:

- menu/settings integration;
- host/persistence compatibility adapters;
- cutscene integration where schedule ownership can be made neutral;
- touch/input dependencies that only exist through actor compatibility re-exports.

The code-smell backlog specifically identifies actor compatibility-facade debris
and the second-order `spawn_actors.rs` dispatcher. Delete compatibility paths as
their last consumer moves. Do not extract `spawn_actors.rs` as a crate; shrink it
by moving each domain-specific branch with the domain it constructs.

### Wave B — character preparation and presentation

✔ **PARTLY DONE, 2026-08-09 — and the directory was NOT the seam, exactly as this
section says.** `character_sprites` split in half: the DERIVATIONS from a sheet
(`anim`, `posed_body`, `attack_hitbox` — 2,144 lines) are
`ambition_character_sprites`, a sibling crate the actor crate does not depend on;
the actor/content JOIN (`assets.rs`) stays, because it is coupled to
`assets::platformer_assets`, `persistence::settings` and the character-runtime
materializer in both directions. ⛔ the load-bearing part was moving the one
`WorldPrepSet::BeforeIntegrate` registration into the new crate as a plugin —
see "A carve is decided by its DIRECTION" in
[`decomposition.md`](decomposition.md). `character_runtime` is untouched.

`character_runtime` plus `character_sprites` is a large compile-isolation target.
Split by lifecycle rather than by current directory:

```text
authored character
    -> provider/preparation
    -> prepared body definition
    -> actor construction

prepared presentation
    -> sprite/art loading
    -> presentation
```

Hurtbox/combat adaptation belongs with combat integration; physical body
baseline belongs with actor/body construction; match seating belongs to session
or match topology. “Character” is the subject of these operations, not one shared
runtime authority.

### Wave C — world authoring/backend integration

The actor kernel should consume lowered world/session semantics, not depend on
LDtk as an authoring backend. Use the existing `ambition_platformer2d_world`,
`ambition_platformer2d_ldtk`, provider, and runtime owners to move backend
loading/hot-reload and room orchestration outward until the actor crate can drop
its LDtk edge.

### Wave D — items/equipment and optional gameplay capabilities

Treat items, equipment, abilities, projectiles, portals, and similar mechanics as
body/gameplay capabilities composed around the actor kernel. Use vertical slices:
model/vocabulary stays in the leaf domain, actor-facing execution uses a narrow
body interface, and integration lives above both when necessary.

Do not simply move the current `items/` or `abilities/` directory wholesale if it
still orchestrates unrelated actor/session concerns.

### Wave E — dialogue, conversation, encounter, and boss orchestration

These are attractive dependency removals but higher-risk authority seams.
Conversation changes simulation lifetime and currently crosses the rollback
boundary; preserve the rule that simulation-affecting narrative transitions are
replayable at the same simulation point. Encounter owns orchestration, not actor
identity. Boss-specific policy remains content/game policy while reusable
encounter state stays in `ambition_encounter`.

✔ **Encounter's half of this wave is DONE; the residue is not a second carve**
(measured 2026-08-17, carve refused). `crates/ambition_encounter` holds the
lifecycle, commands, objectives, participants, timeline, waves, registry, music,
rewards, spec and staging. The lines left in the monolith are adapters that
still touch LDtk, ECS spawning, player/body queries, feature overlays, banners
and save/quest plumbing. ⛔ The load-bearing blocker is `drive_wave_encounters`
calling `features::spawn_encounter_mob` — a wave arena spawns actors through the
monolith's actor construction path, which is Wave G and leaves LAST — plus
schedule-pinning by `FeatureWorldOverlaySet` and a foreign module's system
registration. The useful slice was de-laundering: six
`pub use ambition_encounter::…;` compat shims deleted, `encounter/mod.rs`
re-exports nothing it does not define (39 exported names down to 13), inward
references 29 → 6.

✔ **The boss data model came home 2026-08-17 (D33), ending the `features` ↔
`boss_encounter` bidirectional edge that was the boss carve's single blocker.**
`features/ecs/boss_clusters.rs` → `boss_encounter/clusters.rs`; `BossOverrides`
and `sync_boss_reward_chests_ecs` relocated with no re-export left behind. This
raised `siblings → boss_encounter` sites from 155 to 201 — not a regression:
those are the SAME callers, previously reading `crate::features::BossConfig`,
now naming their real dependency. **A relocation does not delete an edge a
caller genuinely has; it makes the edge say whose it is.**

✔ **The carve landed the same day** — `crates/ambition_boss_encounter`, 7,635
lines out of the monolith, `largest_unit_lines` 121,822 → 114,139.

⛔⛔ **The 201 inward sites were never the blocker — reading them as one was a
direction error.** An inward edge is a caller naming the domain; after the carve
it names `ambition_boss_encounter::` instead of `crate::boss_encounter::` and
compiles unchanged, a rename not a dependency. **Only OUTWARD edges block a
carve** — the ones that would make the departing crate depend on the monolith it
left, which cargo refuses outright. Count both directions, but adjudicate on the
outward one.

Of the thirteen distinct outward sibling paths, eleven resolved to crates
already below the monolith — the hub was re-exporting every one of them. The two
real ones moved down rather than across: `CutsceneTriggerQueue` →
`ambition_cutscene`, beside the script format it triggers; and `MountDied` →
`ambition_platformer2d_shared_tangle::body`, below both domains that share it.
`impl SnapshotCursor for BossEncounter` moved with the type per the orphan rule;
the rollback wire format is unchanged (357 names / 85 types).

```text
  largest_unit_lines   121,822 → 114,139   (−7,683)
  critical_path_crates      13 → 13        no new hop, unlike the conversation carve
  capability closure         43 → 44 crates, 16 → 17 a movement-only game never asked for
  ambition_geometry worst_edit_cost  48 → 49 crates (+17.5s) — the honest cost
```

⚠ `critical_path_crates` did not rise because the boss domain sits over crates
the monolith already sat over, slotting in beside them rather than under
anything — a carve lengthens the chain only when it inserts a layer BELOW a
crate that was already deep, which is what happened when `conversation` went
under `ambition_dialog`.

### Wave H — the mount pair, and what an OUTWARD-EDGE census costs to get right

⭐ **`ecs/mount` (1,871 lines) is the smallest honest carve left inside
`features`**, and Wave E's rule adjudicates it: only OUTWARD edges block. The
census, 2026-08-26, after three fixes landed the same day:

```text
BEFORE                                          AFTER
brain_builders::dismounted_rider_…    import    ✔ mount ANNOUNCES `MountDied`; the
                                                  builder answers it. The message
                                                  already existed for the boss bridge
crate::physics::ResolvedMotionFrame   re-export ✔ it was in shared_tangle all along
crate::features::TemporaryControl     real      ✔ moved to shared_tangle beside
                                                  `body::Mass` — two domains share it
actor_clusters::ActorClusterQueryData real      ✔ NOT NEEDED: 26 columns, mount
                                                  touches 5, and 4 are already in
                                                  `_core` / `ambition_characters`
actor_clusters::ActorConfig           real      ⛔ THE ONE LEFT, and it is TWO FIELDS
```

⛔⛔ **THE LESSON IS THAT THE FIRST CENSUS COUNTED `use` LINES.** It reported ONE
outward edge; there were four, because three were spelled inline
(`crate::physics::…`, `super::actor_clusters::…`). ⇒ **count the PATHS a module
names, not the imports it writes.**

⭐ **AND HALF OF WHAT A CENSUS FINDS IS NOT AN EDGE AT ALL.** Two of the four
resolved to crates already below the monolith — the hub was re-exporting them,
exactly as eleven of the boss carve's thirteen did. That ratio is now the
expectation, not a surprise: **look up the real owner before designing a way
around a dependency.**

⭐ **AND THE LAST EDGE PAID FOR ITSELF.** The dismount restores `spawn.size` and
used to re-derive gravity from `tuning.is_aerial`; the live components
(`BodyBaseSize`, `BodyFlightState::fly_enabled`,
`ActorSurfaceState::gravity_scale`) are all written at runtime and could not
stand in — mount ZEROES the last one itself. Naming the baseline also collapsed
**three** hand-written `if is_aerial { 0.0 } else { 1.0 }` sites into one
recorded value. Cost: one stable schema name and a declared bump to
`GGRS_ROLLBACK_SCHEMA_VERSION` 113. See the D33 row in `queue.md`.

### Wave I — the monolith stops republishing its peers, and the census stops lying

⛔⛔ **THE HUB WAS RE-EXPORTING THREE ALREADY-CARVED CRATES UNDER `crate::`
PATHS, AND EVERY COUPLING NUMBER ON THIS PAGE WAS INFLATED BY IT.** Wave H's
census already recorded the symptom — *"half of what a census finds is not an
edge at all... eleven of the boss carve's thirteen"* — but treated it as a
lookup discipline. It was a source defect, and it is deleted, 2026-08-26:

```text
DELETED                                       WAS                       SITES
crate::physics (physics.rs, 15 lines)         pure re-export of          50 in
                                              shared_tangle::{gravity,      + 17
                                              frame_env}                 outside
pub use ambition_combat as combat             a whole-crate alias        436 in
  (it carried its own TODO(compat-remove))    on the PUBLIC surface        + 47
                                                                        outside
pub use ambition_projectiles::*               a glob forward; the module 79 model
  (in projectile/mod.rs)                      also owns real code, so    names,
                                              the split is by NAME       locals kept
```

⭐⭐ **THE PAYOFF IS THE NEXT CARVE, AND IT IS BIGGER THAN THE MOUNT PAIR.**
Re-measuring `features/ecs` with the facades gone:

```text
                lines   outward path refs   BEFORE (facade-inflated)
damage_apply     3674          17             61 crate::combat:: alone
perception       1580          13
aggression        860          13
bosses           1852          36
actors           3601          43
damage           4399         120
spawn            3352         154
```

⇒ **`damage_apply` is now the strongest candidate left** — 3,674 lines against
SEVENTEEN outward references, where mount carved at 1,871 against four.

⛔⛔ **THAT TABLE IS STALE AND THE RANKING IS WRONG — RE-MEASURED 2026-08-28.**
`damage_apply` does not exist any more; the work is in `features/ecs/damage/`.
And the old counts mix TEST lines into the size and count every `crate::`
occurrence rather than the distinct concepts behind them, which is what put the
biggest file on top. Non-test lines, total outward `crate::` refs, and how many
DISTINCT monolith modules those name:

```text
                non-test   refs   modules named
bosses              1181      2   1   ← and both were a facade hop; now ZERO
damage              2144      9   2   (banter registry, character_runtime)
damage_drops         333      5   3
interact             255      3   2
aggression           214      3   2
actors              2892     25   5
spawn               2025     52   7
```

⭐⭐ **`bosses` IS THE CARVE, and it was already almost free.** Its two outward
references were `crate::features::ecs_boss_anim_state_and_entity` and
`ecs_boss_animation_frame_sample` — **both of which already live in
`ambition_boss_encounter::anim`** and were reached through two hops of the
monolith's own facade republishing a peer domain. That is precisely the shape
this section already names for the SDK case. Both call sites name the crate
directly now, so **`features/ecs/bosses/` production code names ZERO monolith
paths** — the milestone the mount pair reached before it carved. ⭐ its test-side
`crate::` refs are all SELF-references (`crate::features::bosses::…`), which
become `super::` inside the new crate rather than being coupling at all.
⇒ the remaining price is the MOVE, not an inversion: `ambition_boss_encounter`
already exists and already owns the profiles, the catalog and the anim helpers.
It is missing exactly ONE dependency the module needs —
`ambition_platformer2d_world`, for `CollisionWorld` — and that is a clean
downward edge: `_world` depends only on `asset_manager`, `_core`,
`entity_catalog` and `shared_tangle`, so there is no cycle. ⚠ the
`ambition_platformer2d::` strings in `tick.rs` are LOG TARGETS, not a facade
dependency; a grep for the crate name reports them and they cost nothing.

⛔⛔ **AND AN IMPORT COUNT NEARLY MISSED THE ONE CHANNEL THAT MATTERED.** Both
non-test files opened with `use super::super::*` — a glob over the whole
`features/ecs` module, which no `crate::` grep can see and which could have been
hiding any amount of monolith vocabulary. Measured by DELETING it and reading
what the compiler asked for: bevy's prelude, `WorldTime`, and
`ambition_platformer2d_core as ae`. **No monolith types at all**, so the estimate
survived the harder test — but it survived by measurement, not by the grep.
⚠ read the WHOLE error set when you do this: the first run reported only six
missing names, and adding those revealed `ae` and the `Component` derive behind
them. rustc stops resolving early, so a truncated list reads as a short one.
✔ the globs are gone; both files name what they use, and the test modules that
were living off the same glob name theirs.

### ✔✔ CARVED 2026-08-28 — `ambition_boss_encounter::ecs`

The boss ECS module lives with the boss profiles, catalog and anim helpers it was
already calling. `features/bosses.rs` (the attack-moveset authoring, 218 lines and
**zero** `crate::`/`super::` references) went with it as
`attack_moveset`. `ambition_boss_encounter` gained one dependency,
`ambition_platformer2d_world`. The monolith keeps facade rows so its schedule
registration and callers did not move.

⛔⛔ **AND A THIRD INVISIBLE CHANNEL SHOWED UP AT THE MOMENT OF THE MOVE.**
`crate::` grep missed the glob; deleting the glob still missed
**`super::super::`**, which is neither. The compiler found three the instant the
files changed crates:

```text
super::super::actors::ActorSteering
super::super::actor_clusters::ActorClusterQueryData
super::super::actors::integrate_actor_body
```

⇒ all three belong to ONE function, `integrate_boss_bodies`, **whose own doc had
already recorded the verdict**: *"The shared seam is `integrate_actor_body`; keep
this as the boss orchestrator around that seam."* So it stayed, as
`features/ecs/boss_bodies.rs`, and the rest left. Moving it would have meant
moving the generic actor integrator — a different carve at a different price.
⚠ one fixture came back for the same reason: `scripted_pattern_tests` builds an
`ActorClusterSeed`, so it sits beside what it builds and names the boss crate.

⛔ **ONE UNRELATED-LOOKING RED, AND IT WAS THE CARVE.**
`the_reaction_timer_clock_forks_on_purpose` walks `CARGO_MANIFEST_DIR/src`
counting `.decay_reaction_timers(sim_dt)` sites and wants TWO — actor and boss.
The boss one left the crate, so it found one and printed its own escape hatch:
*"the scan is broken, not the code."* It was. Widened to both roots rather than
lowered to one: the invariant did not move, the code did. ⇒ **a source-scanning
guard is a guard on WHERE CODE LIVES, so every carve has to look for one.**

⚠ **`damage`'s nine refs are FOUR concepts**, which is what a carve would have to
subtract: `CombatBanterRegistry` (×3), `PreparedCharacterRegistry` (×2),
`ActiveMatch` (×2), and `RespawnPolicy` + `ENEMY_DEAD_UNTIL_REST_SUFFIX`. Count
the concepts, not the occurrences — the occurrence count is what made this table
rank by file size.

✔ **ONE OF THE FOUR IS SUBTRACTED, 2026-08-28: `CombatBanterRegistry`.** 63 lines
depending on nothing but `std` and bevy, and no combat semantics at all — a
name → lines table. It reads as actor machinery only because the hit path is what
looks at it. It lives in `ambition_conversation` now, because **a bark is the
shortest conversation there is**, and that keeps `ambition_combat` free of
dialogue vocabulary. ⇒ `damage` is down to THREE concepts.

⛔⛔ **AND THE MOVE HIT A TYPE PATH HELD AS A STRING.**
`rollback_coverage.rs` waives it by the literal
`"::features::banter::CombatBanterRegistry"`, which no compiler checks. Poison-verified
BOTH ways: with the old string the coverage arms go red naming the new path, and with
the new one they pass. ⇒ **every carve must grep its moved type names IN QUOTES**,
not only as paths.

✔ **AND TWO OF THE REMAINING THREE WERE FACADE HOPS, not coupling — the boss
lesson again.** `PreparedCharacterRegistry` is `ambition_characters::prepared`'s
and `RespawnPolicy` is `ambition_entity_catalog::placements`'; `crate::` only
republished them. Named through their owners, `damage`'s `crate::` count is now
**three, and `boss_hit.rs` names ZERO**.

⛔⛔ **AND THAT NUMBER IS STILL NOT THE PRICE, for exactly the reason the boss
carve found.** `damage/` reaches the monolith through `super::super::` and a
`use super::*`, and THOSE are the real cost — thirteen distinct names across six
concepts:

```text
actor_clusters::{ActorMut, ActorClusterQueryData, ACTOR_DAMAGE_IFRAME_S}
damage_drops::{drop_currency_coin, drop_held_weapon, EXPLODER_BLAST_DAMAGE}
npcs::{npc_flag_id, npc_hit_bark_line, npc_hostile_bark_line}
damage_predicates::target_is_ignored
NPC_HOSTILE_STRIKE_THRESHOLD
ActiveMatch + ENEMY_DEAD_UNTIL_REST_SUFFIX   (the `crate::` three)
```

⭐⭐ **THE CANDIDATE TABLE, RE-MEASURED ACROSS ALL THREE SHAPES (2026-08-28).**
Non-test lines; DISTINCT inward names reached by `crate::` **or** `super::…`; and
whether a PRODUCTION file globs its parent (a `use super::*` inside
`#[cfg(test)] mod tests` is the ordinary self-glob and is not counted):

```text
                    lines   inward names   prod globs
attack                604        0             0
brain_effects         343        0             0
damage_predicates     237        0             0
ledge_trump           152        0             0
encounter_rewards     101        0             1 → 0   (prelude + vfx + RoomVisual)
chests                130        4             1 → 0
interact              255        4             1 → 0
aggression            214        5             0
damage_drops          333        6             0
actor_clusters       1409        4             0
damage               2146       15             2
actors               2892       29             4
spawn                2025       34             2
```

✔ **`ledge_trump` CARVED 2026-08-28 → `ambition_combat::ledge_trump`**, beside the
`DeclaredCombatRules::ledge_trump_pop` it enforces, which this crate already
owned. Zero new dependency edges; the eight tests went with it.
⛔ **AND CHECK THE DESTINATION FOR A CYCLE BEFORE PROMISING THE OTHER THREE.**
`damage_predicates` needs `ambition_boss_encounter`, which depends on
`ambition_combat` — so combat is not its home. `brain_effects` names
`ambition_app`, `ambition_mount` and `ambition_projectiles`, so it cannot go down
at all yet. ✔ **`attack` CARVED THE SAME DAY → `ambition_combat::attack_support`** (604 lines),
for that one clean edge. ⭐ its own module doc had already argued the move: the
melee LIFECYCLE left for `combat::moveset` some time ago and *"there is ONE melee
path"* — what stayed behind in the monolith was support for a path that lives in
combat. Renamed on the way in, because `combat::attack` beside `combat::moveset`
would have read as a second melee road.

⭐ **FOUR MODULES — `attack`, `brain_effects`, `damage_predicates`, `ledge_trump`,
1,336 lines together — REACH THE MONOLITH IN NO SHAPE AT ALL.** They are the
carve-ready set, and none of them appeared in the old ranking because it counted
lines. ⇒ the destinations to check are `ambition_combat` for the first four and
`ambition_encounter` for `encounter_rewards`.
⚠ the three production globs are gone: measured by deleting them, all three
supplied bevy's prelude, `ambition_vfx`'s two message types, `RoomVisual`
(`shared_tangle`), `ambition_platformer2d_core as ae` and `AabbExt` — no monolith
vocabulary, on the third module set in a row.
⛔ and the old table's small-module rows were counting the wrong FILES: `aggression`,
`interact`, `damage_drops` and `chests` each have a sibling `.rs` holding the code
and a same-named directory holding only tests.

⇒ **`damage` is NOT the next carve.** `actor_clusters` is the same dependency the
boss integrator kept it behind, and `npcs`/`damage_drops` are two more modules
that would have to move or invert first. ⭐ **grep `super::` beside `crate::` on
every candidate in the table above before ranking it** — the table's numbers count
only one of the three shapes.

⚠ **AND `crate::actor` IS NOT THE SAME DEFECT — MEASURED, NOT ASSUMED.** It is
138 lines that OWN `AncillaryMovementBundle` (110 of them) and re-export 27
vocabulary names from FOUR crates, so it composes something a single alias does
not. It stays.
✔✔ **IT DOES NOT. DELETED 2026-08-28, and the ruling was sound while the premise
held.** The 27 re-exports went on 2026-08-27, leaving the module as the bundle
plus a doc. And the bundle did not have to stay either: nineteen of its twenty
fields are `_core` body clusters and the twentieth is `shared_tangle`'s
`ResolvedMotionFrame`, so **`shared_tangle::body` is the lowest crate that can
name them all** — `AncillaryMovementBundle` lives there now and `actor.rs` is
gone. ⇒ *"it composes something a single alias does not"* was true, and the thing
it composed belonged one crate down. **A ruling that rests on a measurement dies
with the measurement**: re-read the premise, not the verdict.

✔✔ **AND `character_runtime` HAD THE SAME DEFECT, 2026-08-28 — nine names, ~250
sites.** It republished `CharacterBodyBlueprint`, `CharacterCatalogGeneration`,
`CharacterPreparationPlugin`, `MissingCharacterFacts`,
`PreparedCharacterDefinition`, `PreparedCharacterRegistry`, `PreparedKit`,
`CharacterBindings` and `CharacterRegistrationError` — all
`ambition_characters::prepared`'s — and **57 of the sites naming them were OUTSIDE
this crate**, which is precisely how a census reads the monolith as their owner.
Deleted; 66 files repointed. ⭐ the module is not a facade otherwise: `MatchSeat`,
`ActiveMatch`, `PreparedMatch`, `MatchRules` and the load states really are its
own, so this is a republication removed rather than a module carved.
⭐ **DO IT COMPILER-DRIVEN.** Delete the `pub use` block FIRST and let the 87
errors enumerate the callers; a sed sweep over qualified paths misses the grouped
`use super::{A, B}` forms and the bare names inside the module's own children. But a census cannot tell composition from republication, so the
resolution is written down ONCE here instead of re-derived per carve:

```text
crate::actor::BodyKinematics                    crate::platformer_runtime::body
crate::actor::{PlayerEntity, PrimaryPlayer,     shared_tangle::markers
               PrimaryPlayerOnly}
crate::actor::BodyAnimFacts                     ambition_characters::actor
crate::actor::BodyMelee                         ambition_combat
crate::actor::Body*  (19 movement clusters)     ambition_platformer2d_core
```

⇒ of `damage_apply`'s seventeen, the four `crate::actor::` refs resolve to
`_core` / `shared_tangle` / `ambition_characters` — already below the monolith.
**Its real outward edge count is thirteen.**

⛔ **WHAT IS NOT A DEFECT: the SDK facade.** `ambition_platformer2d` re-exports
`ambition_combat as combat` on its own, one hop, and `architecture.md` sanctions
exactly that — *"curated re-exports and convenient composition."* The rule is
about WHO republishes: a curated SDK may; a domain crate republishing a peer
domain is how `ambition_platformer2d::actors::combat::X` became a three-hop path
that made the runtime look coupled to the monolith when it already depended on
`ambition_combat` directly. Those 47 external sites now name one hop or zero.

### Wave F — presentation effects and audio

Simulation publishes deterministic/confirmed effect intent; audio, SFX, VFX,
character visuals, and other presentation consumers live downstream. Reuse the
confirmed-frame external-effect seam rather than making actor simulation depend
on playback/rendering implementations.

Be precise about types currently named like presentation effects that actually
spawn simulation entities or change combat state: those belong to simulation
vocabulary even if their historical name says VFX.

### Wave G — central actor residue

Only after the outer domains have left should we reassess `features`, actor
updates, body-mode integration, control, scheduling, and the remaining
construction path. At this point the residual dependency graph should tell us
whether one actor crate is honest or whether there are still multiple durable
simulation domains.

When the residue really is actor/body/control simulation, rename
`ambition_platformer2d_actor_monolith` to the final actor-domain name and update
the public facade to re-export semantic API modules from their actual owners.

## ✔✔✔ 2026-08-17: the compile ratchet is GREEN, and the slice that did it was NOT a carve

`largest_unit_lines` **114,139 → 110,929**, below the 111,429 frozen on
2026-08-09 for the first time. `critical_path_crates` held at **13** — no new
crate was made, so no new hop.

⛔⛔ **Both named candidates were refused, both for Wave G.** Chasing every
outward site to its definition AND splitting production from test — a split no
earlier measurement on this campaign made — gives `items` 30 real production
sites and `world` 11. But `items/pickup` rebuilds a carried object through
`construction::authored_occurrence_request`, and `world/rooms/{stage,transaction}`
stages actors through `ActorConstructionPlan`; `construction/mod.rs` imports
`crate::world::placements::ActorPlacementContext` back. **Actor construction is
the blocker for both, exactly as it was for `encounter`.**

⭐⭐ **The population nobody had counted is modules with ZERO real outward edges
whose owning crate ALREADY EXISTS.** Four of them left in this slice:
`persistence` (1,336, deleted outright — all eight public names of its
pause-menu `settings/model` had zero code references in the workspace),
`menu` (809 → `ambition_menu`, which already held the `MapMenuState` the
renderer imported), `dialog` (672 → `ambition_conversation`), `equipment`
(388 → `ambition_items`).

⭐⭐⭐ **The transferable rule is about the DESTINATION.** Three other "obvious"
homes were refused by the crates themselves: `ambition_dialog` declares itself
content-free and the moved glue is host coupling; `ambition_settings_menu` is
renderer-agnostic and carries no bevy; `ambition_menu`'s manifest says its
trimmed bevy features are *"load-bearing for the WHOLE workspace"*. ⇒ **read a
destination's stated contract before moving code into it — a crate that refuses
your dependency is telling you the code does not belong there.** The Map tab
passed because `ambition_menu` already sat downstream of
`ambition_platformer2d_core` via `ambition_ui_nav → ambition_input`, so its
three new edges added **no crate** to `ambition_geometry`'s or
`ambition_platformer2d_core`'s rebuild sets.

⚠ **a relocation launders the per-crate ledgers better than a carve does**,
because there is no new `Cargo.toml` to prompt the edit: `ambition_items` and
`ambition_menu` joined `check_doc_link_ratchet.py`'s `CRATES` in the same
commit, and seven workspace policies were re-pointed — two of which had to
change SIDES, from *requiring* a path in the monolith to *forbidding* it.

## Scoreboard

Record these measurements after meaningful waves, not after every tiny edit:

| Measure | 2026-08-07 baseline | Direction |
|---|---:|---|
| Actor `src/**/*.rs` lines | 110,911 | down substantially |
| Normal internal `ambition_*` dependencies | 28 | down |
| Unwanted movement-only capability crates inherited through actors | 15 | toward 0 |
| Actor-monolith builds in the measured full-suite workflow | 16 | suite target remains <=2; carves also reduce cost per build |
| Root modules | 42 | descriptive only; do not optimize this count directly |

⭐ **Measured 2026-08-28, after the boss carve: `src/**/*.rs` is 107,354 lines.**
⛔ **AND THAT NUMBER IS NEARLY USELESS ON ITS OWN, which is worth saying once
rather than re-deriving each time.** 110,911 → 107,354 is 3.2% over three weeks
in which mount, boss ECS, boss attack-moveset, `SpawnBaseline`, `TemporaryControl`
and the rollback declarations all left — because carves REMOVE lines while the
game keeps ADDING them, and this row cannot tell those apart. The measures that
moved and mean something are the OUTWARD EDGE COUNTS per module, which is what the
re-measured candidate table above tracks, and the count of modules naming ZERO
monolith paths. Read this row as a trend, never as a verdict on a slice.

Add a focused compile-time measurement once the first substantial carve lands so
later waves can compare clean-build and representative incremental-edit cost.
The compile-time goal is both **less work per actor edit** and **more independent
work available for Cargo/rustc parallelism**.

## Definition of done

The campaign is done when:

- the residual actor crate owns body/actor simulation, control integration, and
  genuinely actor-local state rather than miscellaneous platformer gameplay;
- optional capabilities do not enter a minimal consumer solely through the actor
  crate;
- presentation, authoring backend, menu, audio, persistence, dialogue, and other
  independent domains compile outside the actor implementation unit;
- intrinsic body capabilities come from prepared body/character identity rather
  than session/dev defaults;
- historical `avatar`, `features`, and broad construction aggregates have either
  dissolved or been renamed to honest durable concepts;
- the public SDK remains semantic and stable even though implementation ownership
  moved underneath it;
- the compile-time and resolved-dependency measurements demonstrate a material
  improvement over the 2026-08-07 baseline.
