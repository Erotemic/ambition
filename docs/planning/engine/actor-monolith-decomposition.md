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
  VFX. See [`api-1.0-campaign.md`](api-1.0-campaign.md).

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
feature could forward `ambition_ui_nav/input`, and that forward was **doubly
redundant**: `ambition_dialog/input` (forwarded on the next line) already enables
it, and `ambition_ui_nav/input` is itself only `ambition_input/input`, which the
line above already enables. Removed: one declared Cargo edge, zero lines of code
moved. This is the plan's step 3 — *repoint compatibility imports first* — in its
cheapest possible form.

⛔ **and the movement-only footprint did NOT shrink, which is the finding.** It
is still *40 crates linked, 15 a movement-only game never asked for*. `ui_nav`
still arrives, by this path:

```text
minimal_game → ambition_platformer2d → …_actor_monolith → ambition_dialog → ambition_ui_nav
```

So **the 15 leaked capability crates are not leaked by 15 removable edges.** Some
of them ride on another leaked crate, and cutting the direct edge changes nothing
a consumer can observe. `ui_nav` leaves when DIALOGUE leaves, and not before.

⭐ **The scorecard needs reading in that light**: criterion 1 (a Cargo edge
disappears) and criterion 2 (the consumer footprint shrinks) are not the same
measurement, and a slice can honestly satisfy the first while the second stays
flat. Before claiming a footprint win, run
`cargo tree --offline --edges normal -i <crate>` from `fixtures/minimal_game` and
look at who else pulls it.

⚠ **a manifest edit fails the contracts job until the fixture lock is
regenerated**, with an opaque `cargo tree --locked` traceback that names nothing.
`cd fixtures/minimal_game && cargo tree --offline …` regenerates it; commit the
lock WITH the manifest.

### Re-derived: the `conversation` module's carve accounting

`conversation/mod.rs` carries an accounting written before the narrative ledger
was added to it. Re-derived from source rather than from that prose:

- **inward edges: still exactly TWO**, both in `rules.rs`, both the BARK
  (`features::npc_ambient_bark_line`, `character_runtime::PreparedCharacterRegistry`).
  The ledger added none, so the module's own claim survives its own growth.
- **outward edges GAINED**: `ambition_time` (`SimTick`),
  `ambition_platformer2d_core` (`ConfirmedFrameBoundary`), and
  `ambition_platformer2d_shared_tangle` (`SimId`, the schedule sets). All three
  are below the monolith, so none would cycle.

⛔ **but it is a BAD first slice, by this plan's own scorecard.** 1,907 lines, and
it removes no Cargo edge: every crate it names is named by something else in the
monolith. *"Moving files without improving any of those measures is not a
successful carve."* The right unit is the DIALOGUE domain — `conversation` +
`dialog` + the Yarn bindings together — because that is what takes
`ambition_dialog` (and with it `ambition_ui_nav`) out of the graph. Its blocker
is the third `features` reference, which is the same blocker every other
low-count row has.

### C4b measured — the dialogue "domain" is two things with different owners

Measured 2026-08-08 after `features` was unpinned (`69b53c42d`). The remaining
`ambition_dialog` namers inside the monolith are `dialog/` and `conversation/`,
and they do not belong in the same place:

- **`dialog/` is ONE file** — `yarn_bindings.rs`, 609 lines — and its inward
  edges are `items` (13), `features` (9), `shop` (8), `conversation` (5),
  `actor` (4), `save` (2). ⭐ **its own module docstring already says what it
  is**: *"This module keeps only what is genuinely Ambition-side."* It is NAMED
  GAME VOCABULARY (`<<give_item>>`, `<<buy_item>>`, `<<challenge>>`), and
  `ambition_dialog` already exposes the `YarnContentBindings` installer seam
  precisely so a host pushes that from outside. `game/ambition_content` already
  pushes two installers through it (the duel, the cut-rope commands). **Its
  owner is the content crate, not a dialogue crate.**
- **`conversation/` is the reusable half** — the authority, the ledger, the
  hold, the break rule, the opening port — and it keeps the `ambition_dialog`
  edge wherever it lives, because that is the runtime it projects onto.

⛔ **so "lift the dialogue domain as one crate" was the wrong shape**, and
measuring is what showed it. There is no single dialogue domain in the monolith:
there is a reusable continuity authority and a pile of this game's Yarn verbs.

⚠ **and neither move satisfies the scorecard above on its own.** Moving
`yarn_bindings.rs` to `ambition_content` removes no Cargo edge (conversation
still names `ambition_dialog`) and shrinks no consumer footprint. It is an
OWNERSHIP correction — named content leaving an engine crate, which this
repository cares about independently — and it should be recorded as that rather
than banked as a decomposition win. The measured win still requires both halves
to leave, and `conversation` leaving is what removes the edge.

### C4e answered — `ambition_conversation` is a COMPILE-ISOLATION win worth ~1%, and NOT a footprint win

⛔ **this heading said the opposite until 2026-08-08** — *"IS a footprint win, if
`dialog.rs` goes too"* — for a day after the body below it was corrected. The
correction landed in the paragraphs and the heading kept asserting the refuted
claim, which is the only part a reader scanning the outline sees. **Correct the
heading in the same edit as the body**; a heading is a claim, not a label.

⭐ **AND THE PAYOFF IS NOW A NUMBER** (`scripts/compile_ratchet.py`, 2026-08-08):

```text
largest recompilation unit    111,579 → 109,412    −1.94%
edit cost, rest of monolith   248,672 → 246,505    −0.87%
edit cost, conversation       248,672 → 248,672    ±0.00%
critical path (crates)             12 →      12         0
```

The third row is the one that matters and it follows from the five files below:
the carved crate lands BELOW the monolith, so **editing `conversation` still
rebuilds everything above it, which is the whole monolith.** The carve makes
every OTHER edit marginally cheaper and dialogue work no cheaper at all.

⚠ `critical_path_crates` is guarded because a carve that inserts a layer
*lengthens* the serial chain — every size metric can improve while the wall clock
gets worse. This carve leaves it at 12, measured rather than assumed.

⛔ **CORRECTED the same day by a real timed build: that number is right in HOPS
and wrong by 2.2x in SECONDS.** The guard's own comment claimed parallelism
cannot compress a serial chain. Pipelined compilation does: **rustc releases a
dependent as soon as the predecessor's `rmeta` lands**, so only the FRONTEND is
serial across a chain edge and codegen overlaps everything downstream. A naive
chain-of-durations reads **377.9s for a build that finished in 210.5s**.
Keep the guard — a lengthened chain is still a real regression, and hops is the
honest unit for it — but do not read it as seconds.

⭐ **and the corollary decides which lever a carve should pull.** A cold build and
a rebuild are in different regimes:

```text
                         work/8 cores   dependency floor   binding
  dev cold, 583 units        767.6s          418.9s        CORES
  dev rebuild, 57 units      123.9s          168.4s        THE CHAIN
```

Halving codegen saves **282.7s** cold and **11.8s** on a rebuild; halving the
frontend saves **101.1s** and **61.6s**. So codegen wins 2.8x on a cold build and
the frontend wins **5.2x** on a rebuild — reproduced 3.6x/4.0x/4.1x/5.2x across
four rebuild-shaped runs. **The rebuild is what an agent pays before one test
runs, and this repo has been prioritising it with the cold build's number.**

⚠ **AUDITED 2026-08-08 — which half of this is data and which is a MODEL.**

*Verifiable by summing per-unit fields in `dev/ambition_dev_measurements/compile_units.jsonl`:*

```text
                 units/run   frontend   codegen     work   work/8
  dev cold             688     1120.4    4597.6   6345.8    793.2
  dev rebuild (x2)      57      241.7     700.2    961.6    120.2
```

Those are measured, and the codegen share follows from them directly.

*NOT in the ledger — produced by the collector's DAG simulation:* the
**dependency floor** (418.9s cold / 168.4s rebuild) and **every halve-a-phase
delta** (−282.7 / −11.8, −101.1 / −61.6). ⛔ **so the headline "frontend wins
5.2x on a rebuild" rests on a model, not on a stopwatch.** The model is
well-motivated — rustc releases a dependent at the predecessor's `rmeta`, so only
the frontend is serial on a chain edge, and the naive chain-of-durations
overshoots a real build by 2.2x, which is itself evidence the pipelining is real —
but nobody has re-derived the floor independently.
⭐ **the cheap confirmation, if this is ever load-bearing**: halve nothing and
predict. Run a rebuild after a change that touches ONE leaf crate and check the
DAG model's predicted wall against the measured one. A model that predicts an
unmodified build is worth trusting about a counterfactual one.
⚠ recorded because this figure is now the main argument for carving at all, and
the 1% line figure it displaced was measured rather than modelled.
A carve that shortens the chain therefore helps the loop that is actually paid,
which is a better argument for decomposition than the line count ever was.

The row was told to answer its own payoff before starting. Measured 2026-08-08:

**Who names `ambition_dialog` inside the monolith, production only:**

| where | lines | can it leave? |
|---|---:|---|
| `conversation/` | 2,164 | yes — two inward edges, both the bark |
| `dialog.rs` | 110 | **yes** — `ambition_dialog`, `ambition_input`, `ambition_platformer2d_shared_tangle`, and **zero `crate::` edges** |
| `features/ecs/{tests,interact/tests}.rs` | — | test code → `[dev-dependencies]`, which does not propagate |

⛔ **CORRECTED 2026-08-08, and the correction is the finding.** The paragraph
that stood here said the edge leaves and the carve is a footprint win. **It is
not**, and the error is the SAME CLASS as the `ui_nav` one recorded above — made
immediately after writing the warning against it.

Asking *"who names `ambition_dialog`?"* is the wrong question. The right one is
*"would the new crate itself still be pulled in?"* — and it would:

```text
minimal_game → ambition_platformer2d → …_actor_monolith
             → ambition_conversation → ambition_dialog → ambition_ui_nav
```

**Five production files in the monolith consume `crate::conversation`** —
`features/{mod,npcs}.rs`, `features/ecs/{mod,interact}.rs`,
`schedule/input_systems.rs` — so `ambition_conversation` would be a
non-optional dependency of the monolith and every one of its own dependencies
stays in a movement-only game's resolved graph. The footprint stays at 15.

⭐ **The carve is a COMPILE-ISOLATION win; capability shedding is a separate
architecture obligation.** The maintainer has since answered the product
question decisively: **a game may compose the engine without a capability**
(`maintainer-decisions.md`, 2026-08-08). That does not make a `dialogue` feature
sprinkled across the monolith/runtime a good design. Optionality must follow a
coherent capability boundary with no hidden runtime/rollback edge and no
conceptual overhead. Measure the consumer graph after each candidate boundary.

⚠ **the generalisable rule, now paid for twice**: a carve's payoff is measured on
the CONSUMER's resolved graph, never on which files name a crate. Run
`cargo tree --offline --edges normal -i <crate>` from `fixtures/minimal_game`
BEFORE writing down what a slice will buy — including when the slice is your
own.

**The work, in order:**

1. **The BARK PORT.** `rules.rs` is the only thing keeping `conversation` inside:
   it names `crate::features::npcs::npc_ambient_bark_line` and
   `crate::character_runtime::PreparedCharacterRegistry`, both answering *"what
   line does this character say"* — a CAST question, not a continuity one.
   Install a small port (a resource holding a fn, or a trait object) that the
   monolith fills; leave the cast lookup behind.
2. **Move `conversation/` + `dialog.rs`** into `crates/ambition_conversation`.
3. **`ambition_dialog` becomes a `[dev-dependency]`** of the monolith — the two
   test files still name `DialogState`, and a dev-dependency does not reach a
   consumer's resolved graph.
4. **Repoint the rollback registration.** `rollback/domains/actors.rs` names
   `ambition_platformer2d_actor_monolith::conversation::{ActiveConversation,
   ConversationEnded, ConversationInstanceId, ConversationInputOwner}`; the
   runtime already sits above both crates, so this is a path rewrite. ⚠ the
   schema NAMES do not change, so the wire format and both baselines stay put —
   confirm that rather than assuming it.
5. **Remeasure**: `cargo tree --offline --edges normal -i ambition_dialog` from
   `fixtures/minimal_game` must come back empty, and
   `capability-footprint-may-not-grow` should report **13**, not 15.

⚠ **the `ParticipantId` ↔ `PlayerSlot` correspondence is the carve hazard to
watch.** `conversation/opening.rs` briefly acquired an edge to
`crate::participant_seat` by deriving `ConversationInputOwner` itself, which
would have forced the carve to take that correspondence along or duplicate it.
It takes the owner as a parameter now. Anything else leaving this crate meets the
same wall, because `participant_seat` exists precisely because `ambition_input`
and `ambition_characters` are siblings that cannot see each other.

## ⭐⭐⭐ 2026-08-08: THREE INDEPENDENT FINDINGS CONVERGE ON `rollback/domains/`

Twelve files, **2,130 lines**, one per capability domain — `actors`, `characters`,
`combat`, `cutscene`, `encounter`, `items`, `lifecycle`, `portal`, `primitives`,
`projectiles`, `vfx`. Each file's whole job is *"this domain's types are
rollback-registered."* Three separate investigations, run for three unrelated
reasons, all landed here:

1. **It is the DEPENDENCY LEAK.** C5 found that for `ambition_cutscene` and
   `ambition_items` the runtime's only reason is `rollback/domains/*.rs`.
   Re-verified: `ambition_cutscene` appears in **exactly one file**
   (`domains/cutscene.rs`, 5 references) and `ambition_items` in exactly one
   (`domains/items.rs`, 4). Two of the fifteen crates a movement-only game never
   asked for reach it through 9 lines of registration.
2. **It is the FRONTEND COST CENTRE.** The runtime costs 24.8s of frontend
   against the monolith's 23.9s from 7.6x fewer lines, and **74 of its 79 generic
   functions are in `rollback/`**. **211 of the ~222 call sites** of the
   `rollback_component_*<T>` family are inside the runtime itself, so the
   instantiations land here rather than in callers.
3. **It is why C7's refactor was blocked.** That row concluded the declare/install
   halves *"cannot be split by data alone — installation is generic per type and
   `T` is not recoverable from a `type_name`."* The same property that blocks the
   split is what generates the instantiations.

⭐ **So the carve target this plan has been looking for is probably not in the
monolith at all.** It is 2,130 lines in the runtime, where a capability crate's
presence in a movement-only game's graph, the workspace's densest frontend cost,
and a known-hard refactor are all the same fact: **per-type rollback registration
performed by name, in one file per capability.**

### ⛔⛔ REFUTED, same day, by the subtraction test — finding 2 does not hold

`cargo clean -p ambition_platformer2d_runtime` then `cargo check`, on a quiet
machine, with and without the 11 domain modules and their 11 `register(app)`
calls commented out:

```text
  baseline (domains present)   2.52  2.58  2.52 s
  subtracted (domains gone)    2.59  2.42  2.44 s
```

**Removing 2,130 lines and 74 of the crate's 79 generic functions changes the
runtime's check time by nothing measurable.** The frontend-cost half of the
convergence is dead. Findings 1 (the dependency isolation) and 3 (C7's blocker)
are unaffected — they were grep-measured and stand.

⚠ **the refutation is BOUNDED, and the bound is worth stating**: `cargo check`
of this crate is **2.5s**, while the timed build attributed it **24.8s** of
frontend. A 10x gap means `check` is not doing the same work as a build's
frontend phase (it emits no codegen-ready MIR, and the build figure was also
taken under load 14–18). So what is refuted is the strong claim — *the generic
surface in `domains/` dominates the runtime's type-checking cost*. Whether it
dominates the build's frontend phase is still unmeasured, and still wants
`-Z self-profile`.

⭐ **the process point, which is the durable part**: three independent findings
converged, the convergence was written down as a hypothesis with the inference
link named, a falsifier was queued — and the falsifier killed it within the hour.
That is the system working. The queue row said *"three findings agreeing is a
reason to measure next, not a substitute for measuring, and this campaign has
already had three premises die on contact with measurement, always in the
direction of the tidy answer being wrong."* It was four.

⚠ **what is MEASURED and what is INFERRED**, because this convergence is
seductive: measured — the file-level dependency isolation (grep), the 211 call
sites (grep), the crate-level 8x frontend cost per line (timed build). Inferred —
that the generics *cause* the frontend cost. Cargo cannot attribute time below a
crate, so confirming the join needs nightly `-Z self-profile`.
⛔ **do not start a carve on this without that measurement.** Three findings
agreeing is a strong reason to measure next, not a substitute for measuring.

## ⭐⭐ Measured 2026-08-08: the FRONTEND cost points at the runtime, not the monolith

Two independent measurements now name the same crate, and it is not the one this
plan is about.

**Frontend seconds, first-party, dev cold build** (`dev/ambition_dev_measurements/compile_units.jsonl`):

| crate | frontend | lines | ms/line |
|---|---:|---:|---:|
| `ambition_platformer2d_runtime` | **24.8s** | 14,747 | **1.68** |
| `ambition_platformer2d_actor_monolith` | 23.9s | 111,579 | 0.21 |
| `ambition_demo_mary_o` | 13.2s | 15,110 | 0.87 |
| `ambition_characters` | 10.6s | 35,475 | 0.30 |
| `ambition_content` | 9.7s | 21,496 | 0.45 |

⛔ **`ambition_platformer2d_runtime` costs MORE frontend time than the monolith
while being 7.6x smaller — 8x the cost per line.** And the frontend is the phase
that matters: a rebuild is dependency-bound, and there halving the frontend is
worth **5.2x** halving codegen. The monolith is the biggest unit in the workspace
and among the cheapest per line either way.

⭐ **and the dependency measurement already said the same thing.** C5 found that
of the fifteen capability crates a movement-only game inherits,
`ambition_platformer2d_runtime` declares **ten** and is a direct facade
dependency, while exactly one belongs to the monolith. So the runtime is both
the dependency problem and the frontend-cost problem, measured two entirely
different ways.

### ⭐ And it localises: 94% of the runtime's generic surface is in `rollback/`

Structural scan (`grep`, not timing — see the caveat below):

| crate / area | lines | generic fns / kloc | `where` / kloc |
|---|---:|---:|---:|
| `..._runtime` **whole** | 14,747 | **5.35** | **8.13** |
| `..._runtime::rollback` | 8,649 | **8.55** | **11.09** |
| `..._runtime::room_transition` | 1,723 | **0.00** | 0.58 |
| `..._runtime` everything else | 4,375 | 1.14 | — |
| `..._actor_monolith` | 111,579 | 0.35 | 2.57 |
| `..._platformer2d_core` | 29,062 | 0.65 | 2.85 |
| `ambition_characters` | 35,475 | 0.28 | 3.80 |

**The runtime carries 15x the monolith's generic-function density, and 74 of its
79 generic functions live in `rollback/`.** Generic functions and `where` clauses
are precisely what the FRONTEND pays for — trait resolution and instantiation —
which is the phase a rebuild is bound by.

⭐ **the mechanism is per-type registration.** `rollback/registry.rs` exposes a
family of `fn rollback_component_clone<T>`, `..._cursor<T>`, `..._canonical<T>`,
`..._resolved<T>`, `..._clone_entity_ref<T>`, `..._clone_entity_set<T>` — each
instantiated once per registered type, and the schema currently holds **83
encoded types across 11 crates** (`rollback-wire-format-is-frozen`).

⚠ **and C7 already met this from the other side and did not recognise it.** That
row concluded the declare/install halves *"cannot be split by data alone —
installation is generic per type and `T` is not recoverable from a
`type_name`."* It recorded that as a blocker to a refactor. It is also the
description of the workspace's densest frontend cost.

⛔ **stated as a HYPOTHESIS, because grep counts are not seconds.** Confirming it
needs per-module attribution, which cargo cannot give — its unit is a crate — so
it wants nightly `-Z self-profile`. What is measured is the crate-level 8x, and
what is counted is where the generics are; the join between them is inference.

⚠ **this does not retarget the plan by itself.** **But every future "which crate
should we carve" argument should start here rather than at line count**, which is
the measure this plan opened with and which ranks the monolith first for a cost
it does not have.

## ⛔ Measured 2026-08-08: the dependency leak is mostly NOT the monolith's

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

### And for two of them the runtime's ONLY reason is the rollback schema

`ambition_cutscene` is named in `rollback/domains/cutscene.rs` and nowhere else
in the runtime. `ambition_items` is named in `rollback/domains/items.rs` and
nowhere else. The registrations are the whole edge.

⚠ **`central-rollback-does-not-enumerate-domains` is GREEN over this**, and
correctly so by its own terms — it reads exactly one file,
`rollback/src/rollback/mod.rs`, and Campaign 2 deliberately moved the
registrations to `rollback/domains/*.rs`. The contract guards against the
central FUNCTION regrowing. Nothing guards the consumer-visible consequence,
which is that the runtime still names every gameplay domain one directory down.

### What this changes about the campaign

- **The compile-unit trigger stands.** 112k lines in one recompilation unit with
  incremental compilation off is reason enough, and every carve here still pays
  that.
- **The dependency-leak trigger points at the wrong crate.** A movement-only
  game's fifteen unwanted crates are mostly the RUNTIME's, and the largest single
  mechanism is the central rollback schema enumerating every domain.
- **`ambition_platformer2d_ldtk` is the only capability whose ONLY dependent is
  the monolith** — but it is not therefore sheddable. ⛔ measured: the single
  production reference is `world/ldtk_world/mod.rs`'s blanket
  `pub use ambition_platformer2d_ldtk::*`, which looks like a stale facade and is
  not one. **Seven production files in SEVEN different root modules** consume
  LDtk types through it — `assets/platformer_assets`, `encounter/{loading,systems}`,
  `features`, `menu/map`, `persistence/settings`, `session/setup`. The monolith
  genuinely uses LDtk; there is no slice here, and **no footprint win is
  available from this plan as written.**
- **The maintainer answer is settled: capabilities are optional.** The open
  work is architectural: identify coherent capability boundaries and make them
  optional through the runtime as well as the monolith, rather than scattering
  conditional compilation through unrelated systems. See
  [`../maintainer-decisions.md`](../maintainer-decisions.md).

⚠ **the baseline's own split is stale**:
`scripts/baselines/capability-footprint-baseline.json` lists all fifteen under
`reachable_via_ambition_platformer2d_actor_monolith_alone`. Only one is. Left
unchanged here because the ratchet's live invariant (the SET may not grow) is
still enforced correctly and rewriting the annotation is a separate, careful
edit — but do not reason from that field.

### Could a domain register its OWN rollback schema? Measured: not without a cost

The finding above — that `rollback/domains/{cutscene,items}.rs` is the runtime's
ONLY reason to name those crates — suggests an obvious inversion: let each domain
declare its own schema, the way `ambition_content` already declares
`content.cut_rope_heavy_object_cycle`. Ten runtime edges would invert at once.

**It does not work below the runtime, and the reason is structural.**

- `ambition_content` can do it because it sits **above** the runtime. Every
  crate in the never-asked-for list except `ambition_persistence` sits **below**
  it, so a domain naming `AmbitionRollbackApp` is a cycle.
- The vocabulary cannot simply move down. `registry.rs` is *"a thin Ambition
  registration layer over `bevy_ggrs`"* and imports it directly; moving the trait
  to `ambition_platformer2d_core` would drag `bevy_ggrs` to the FLOOR, so a
  movement-only game would link a rollback backend to compile a jump.
- ⭐ **the declaration/installation split already exists — in one direction
  only.** `register_app_descriptor` records the descriptor in every composition
  and installs `bevy_ggrs` machinery only under a GGRS host, which is why a
  fixed-tick game already carries exact schema identity without paying for
  snapshots. But both halves live inside the same generic trait method, so a
  caller must name the runtime to reach either.
- ⛔ **and the halves cannot be separated by data alone.** Installation is
  generic per type (`ComponentSnapshotPlugin::<T>`), and `T` cannot be recovered
  from a descriptor's `type_name` string. Splitting them needs the domain to
  supply a monomorphised `fn(&mut App)` — whose body names `bevy_ggrs`, which
  puts the dependency back on the domain.

**So rollback inversion is not a free file move.** The maintainer has already
settled that capabilities should be optional; the remaining design problem is to
place declaration/installation seams so domain crates do not acquire the
rollback backend merely to declare participation, while the runtime does not
regain knowledge of every optional domain. Treat that as its own architecture
problem rather than re-opening the product decision.

⚠ recorded so the naive inversion is not attempted as a slice. It looks like a
free dependency win from the table and it is not one.

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

## Scoreboard

Record these measurements after meaningful waves, not after every tiny edit:

| Measure | 2026-08-07 baseline | Direction |
|---|---:|---|
| Actor `src/**/*.rs` lines | 110,911 | down substantially |
| Normal internal `ambition_*` dependencies | 28 | down |
| Unwanted movement-only capability crates inherited through actors | 15 | toward 0 |
| Actor-monolith builds in the measured full-suite workflow | 16 | suite target remains <=2; carves also reduce cost per build |
| Root modules | 42 | descriptive only; do not optimize this count directly |

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
