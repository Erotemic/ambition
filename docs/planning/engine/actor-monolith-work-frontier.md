# Actor-monolith decomposition - executable work frontier

> **Verified against `06b25ee8772a7c5bdf934dce5d49a692ebc2f37b` (2026-09-03).**
> ⭐ **Receipt re-measured 2026-09-04 on the registration commit and the READY
> packet is UNCHANGED:** `features -> construction : 30` / `construction ->
> features : 15`, the same strongest mutual edge, and `grep -rn "crate::features"
> construction/` still returns hits.
> ⭐ **Re-measured a second time 2026-09-04 evening at `b208d9e22`, and the
> packet still stands:** the same `30` / `15` mutual edge, still the strongest
> pair in the graph, with `rollback_registration -> features : 28` second and
> carrying no edge back — a ledger naming many domains, which this page's own
> rules say not to mistake for semantic coupling. The production line total has drifted upward
> by a new leaf module (`body_conditions`, no out-edges), which does not touch
> the 14-module cycle — a receipt, not a score.

**State:** ACTIVE TASK BRIDGE. This page exists to make D33 resumable by an
agent that should not have to reconstruct the whole decomposition history before
choosing one bounded task.

This page is deliberately narrow:

- [`../queue.md`](../queue.md) decides whether D33 is the work to run now.
- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) owns the
  measurements, reasoning, carve history, and architectural evidence.
- [`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md)
  owns the target semantics of the residual actor/body kernel.
- **This page only turns the current measured frontier into executable task
  packets.** It does not replace any of the authorities above.

Do not copy historical investigation into this file. When a packet lands,
replace its current-state description with the new frontier instead of growing
an execution diary. Git history is the diary.

## How to use this page

When D33 is selected by the live queue:

1. Re-measure HEAD before touching code:

   ```bash
   python3 scripts/measure_kernel_module_graph.py --edges 20
   ```

2. Compare the result with the receipt below. If the named seam or dependency
   direction changed, update this page from the code and the focused plan before
   implementing an old packet.
3. Take the first **READY** packet. Do not implement a **DESIGN NEEDED** or
   **RE-MEASURE AFTER ...** candidate by guessing its owner.
4. Make one coherent authority/dependency cut. Do not choose a different task
   because it removes more lines.
5. Run the normal D33 post-carve checks in
   [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) and the
   D33 row of [`../queue.md`](../queue.md).
6. Re-run the module graph after the carve. Update this frontier before starting
   another D33 slice. A carve can change which next step is correct.

## Current receipt

At the verified SHA, `scripts/measure_kernel_module_graph.py` reports 53,489
production lines under the actor crate's `src/`. The largest strongly connected
module component contains 14 modules and 48,238 production lines:

`abilities`, `assets`, `avatar`, `character_runtime`, `character_sprites`,
`construction`, `control`, `features`, `items`, `projectile`, `schedule`,
`session`, `shrine`, `world`.

The strongest mutual edge is still:

```text
features -> construction : 30 references
construction -> features : 15 references
```

The focused plan has already measured the direction. `features -> construction`
is consumption of the construction protocol. `construction -> features` is the
protocol reaching upward to name concrete actor recipes. The next packet removes
that reverse dependency.

The line counts above are a receipt, not a score. The objective is a smaller
semantic cycle and cleaner ownership.

## READY - F1: invert actor construction recipe ownership

### Goal

Make actor construction a lower-level protocol/mechanism consumed by the actor
simulation, rather than a lower-level module that imports the actor simulation's
concrete recipe implementation.

Desired dependency direction:

```text
actor-owned recipe registration
            |
            v
construction protocol / registry
            ^
            |
      actor simulation consumes it
```

The important condition is simple:

> Production construction code must stop naming `features`.

At the verified SHA, the reverse dependency is concentrated in one production
file: `crates/ambition_platformer2d_actor_monolith/src/construction/mod.rs`.
The focused plan measures 15 code references to `features` there, plus the
corresponding test shape in `construction/tests.rs`.

The concrete names currently crossing upward include:

- `spawn_staged_actor_into`
- `spawn_runtime_minion_into`
- `spawn_enemy_with_faction_into`
- `spawn_boss_with_overrides_into`
- `is_limbed_host`
- `giant_hand_plans`
- `SpawnActorKind`
- `SpawnActorRequest`
- `GiantHandPlan`

The focused plan reduces the actual inversion to five recipe registrations.
Use the existing construction-domain/registry patterns and the already-landed
capability construction examples as precedent. Do not replace the dependency
with string dispatch, `Any`, a service locator, or another central switch.

⚠ **READ THIS BEFORE PLANNING THE CUT — measured 2026-09-04, and it narrows what
"registration" can mean here.** `ActorConstruction::dispatch` is a CLOSED match
on the parameter enum (`construction/mod.rs:238-277`), and the file defends that
deliberately: its sibling `dispatch_relation` says the ops come from there
*"rather than from a registry lookup, so nothing outside this crate can supply,
replace, or race to install actor relation wiring."* So a registry that
`features` installs into would spend a property this domain chose on purpose,
and the prohibition above already rules out the usual ways of faking one.
⇒ **The direction that does not fight the existing design is to move the DOMAIN
IMPL to the side that owns the recipes, not to make the protocol call upward.**
`construction/` then keeps the protocol — plan, roster, receipt, transaction —
and the `ActorConstruction` implementation with its nine `construct_*` bodies
goes where the actor recipes live. That is also what makes the sentence below
about a dedicated lower crate follow rather than be hoped for.
⚠ The 15 references split three ways, which is worth knowing before sizing:
two are TYPE imports (`SpawnActorKind`, `SpawnActorRequest`, used by the params
enum and `canonical_summary`), six are the recipe constructors the row names, and
three are the limbed-host/giant-hand shape helpers (`is_limbed_host`,
`giant_hand_plans`, `GiantHandPlan`). The type half may move on its own and is
the cheapest first cut; the helper half is a shape query that may belong to
neither side as it stands.

### Required result

The packet is complete only when all of these are true:

- production `construction` has zero dependency on `crate::features`;
- concrete actor recipe behavior is registered from the actor-owning side;
- `features` may continue to consume construction vocabulary and plans;
- recipe identity, deterministic dispatch/fingerprints, refusal behavior,
  construction receipts, and reconstitution behavior remain stable;
- tests follow the new ownership instead of preserving the old reverse import;
- no new upward dependency is introduced to hide the old one;
- the module graph is re-measured after the change;
- ⭐ **the packet states which of the TWO architectural goals it advanced and
  what it left standing** — see the section directly below.

### ⭐⭐ WHICH GOAL DOES THIS CARVE ADVANCE? A packet must answer, and F1's answer is "the first one only"

**Added 2026-09-04, and it is the newest architectural direction in the
repository rather than a restatement of an old one.** Doctrine now names TWO
architectural success criteria and says the second does not follow from the
first: **authority decomposition** (which crate owns the fact, what may mutate
it, one lifecycle, dependency direction) and **capability composability** (can
this capability be ABSENT, does the rest still form a coherent application,
does it declare only its real prerequisites). The rule and its ordering live in
[`decomposition.md`](decomposition.md) under "Decomposition has two dimensions",
with the durable statement in
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

⛔⛔ **THE GAP THIS SECTION CLOSES IS NOT "NOBODY WROTE IT DOWN" — it is that the
criterion sits in the program's EXIT and in no carve's ACCEPTANCE.**
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) already
requires it, twice, in its own vocabulary: exit criterion **2**, *"optional
domains install through semantic capability/plugin seams rather than
actor-kernel imports"*, and exit criterion **4**, *"minimal consumers do not
inherit unrelated domains through the residual kernel"*. ⭐ **Those two ARE
capability composability** — the doctrine's words and the plan's words name one
criterion, and reading them as two is how a reader concludes the plan does not
cover it.

⇒ **So the defect is a seam between two documents, not a missing idea.** The
program is complete only when criteria 2 and 4 hold; no individual packet has
ever been asked about them. A run of carves can therefore pass every acceptance
it is given and arrive at the exit with criterion 2 unmet, because nothing along
the way was scored against it. ⚠ That is precisely the outcome the doctrine
commit named in advance — *"a carve could satisfy every ownership rule on the
page and leave a capability nobody can install alone, with no document to
notice."*

ⓘ **Measured, not assumed, 2026-09-04:** the string `composab` appears zero times
in this page (before this section), in
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md), in
`roadmap.md` and in `status.md`. ⛔ **And that count is why the finding above is
worded the way it is.** The spelling search says "absent" on a page whose exit
criteria state the concept in full; searching the CONCEPT — *optional*,
*install*, *minimal consumer*, *inherit* — is what found criteria 2 and 4.
`roadmap.md` is the one page where both searches agree on absence.

**F1's answer, stated so it cannot be quietly upgraded:**

| goal | what F1 does |
|---|---|
| authority decomposition | ⭐ **ADVANCES IT** — one reverse dependency removed, the `ActorConstruction` impl moved to the side that owns the recipes |
| capability composability | ▢ **DOES NOT ADVANCE IT, AND IS NOT REQUIRED TO** — after F1, actor construction is still mandatory in every supported composition. What F1 buys is the precondition: the extraction into a lower crate becomes clean, and only then is "can a host install this capability alone" a question with a possible answer |

⭐ **That is the doctrine's own ordering, not a weakening of it.** *"Sequencing
is explicitly permitted: move authority into the right domain now, invert a
remaining dependency later, make the capability independently installable after
that. A carve need not deliver all three at once."* ⇒ The requirement this
section adds is a **declaration**, not a second body of work: say which goal the
carve advanced, so that a run of carves cannot add up to *"the engine is
decomposed"* when every one of them moved only the first dimension.

### ⛔⛔ THE COST THIS PROGRAM PAYS, MEASURED — every carve lengthens the compile critical path

**Measured 2026-09-04 and not previously written down anywhere.** The D33 carves
are working on the metric they were aimed at, and they are paying in a currency
nothing on this page priced. Both halves, from `scripts/compile_ratchet.py`
against the baseline frozen at `11ef33c5b5a5` (2026-08-27):

| what the carves BOUGHT | what they COST |
|---|---|
| `largest_unit_lines` 108,364 → **100,153** (−8,211) | `critical_path_crates` 14 → **16** — LONGER |
| the monolith's `edit_cost_lines` share 50.5% → **47.1%** (−3.4 pts) | `worst_edit_cost_lines` +41,400 and `edit_cost_lines` +40,921 |
| `edit_cost_seconds` 1,264.9s → **1,163.8s** (−101.1s) | |

⭐ **The two crates that lengthened the chain are named, by deriving it rather
than guessing:** `ambition_abilities` and `ambition_held_items`, both D33 carve
outputs, now sit at positions 9 and 10 of the longest first-party chain:

```text
ambition_app → content → platformer2d → platformer2d_host → platformer2d_runtime
  → sim_view → platformer2d_actor_monolith → abilities → held_items → items
  → combat → sprite_sheet → interaction → characters → platformer2d_core → geometry
```

ⓘ **That chain was derived independently from `cargo metadata` — normal and build
deps only, since dev-dependencies may form cycles — and it reproduces the
ratchet's `16` exactly** once restricted to the same population the ratchet
declares (`consumer = ambition_app`). ⚠ The crate COUNT does not reconcile as
cleanly: 68 by that walk against the ratchet's 66, a difference of two I have not
chased. The chain length is the load-bearing figure here and it agrees to the
digit.

⚠ **And the two `REGRESSED` line metrics are NOT a structural regression in
`geometry` or `platformer2d_core`.** The workspace gained **4 first-party crates
and 44,056 lines** since the baseline; +41,400 and +40,921 are that growth
passing *through* those crates' blast radius, not those crates changing.
⇒ Triaged rather than re-frozen blind, which is what the ratchet's own message
asks for.

⭐⭐ **WHY THIS BELONGS ON THIS PAGE AND NOT ONLY IN THE QUEUE: it prices the
second dimension.** Capability composability is bought by making capabilities
independently installable, and on this dependency graph that means **more
crates**. Every crate added to the serial chain lengthens a wall clock that
**parallelism cannot compress** — the ratchet says so in those words. The
capability-footprint row already records the sibling tension for the crate COUNT
(*"a carve that adds a crate RAISES the count — the two lines of work must not be
scored against each other"*); nobody had said it about the critical PATH, which
is the worse of the two because it is serial.

⇒ **So the declaration this page now requires has a third line available to it
when it is honest:** what the carve bought, what goal it advanced, and **what it
cost the compile graph**. ⛔ This is not an argument against carving. It is an
argument against reporting a carve as free, and against discovering the price
only when someone re-runs a gate that `--rust` does not include.

⚠ **The seconds columns for eight crates are a PLACEHOLDER and stay one.**
`ambition_abilities`, `ambition_body_seed`, `ambition_encounter_features`,
`ambition_held_items`, `ambition_match`, `ambition_registry_core`,
`ambition_sprite_fx` and `ambition_world_items` are priced at the population
median 2.9059 ms/line, and the ratchet says size predicts compile cost with
**R² = 0.12** — so those seconds are wrong by an unknown factor. ⓘ Measuring them
needs `compile_collect.py --config release`, which builds into a SEPARATE target
root; this volume had **61.3 GB free against a 40 GB floor** when that was
considered, and a cold release tree plausibly exceeds the 21 GB of headroom. Not
run. ⇒ It would move no verdict either way — every metric currently failing is a
LINES metric or the path length, none of which read the weights.

⚠ **And do not answer it by reaching for the wrong mechanism.** Doctrine
prohibits a service locator, a type-erased registry, dynamic dependency
injection, or global plugin discovery to make a crate look optional — which
matters here specifically, because `ActorConstruction::dispatch` is a CLOSED
match that this domain defends on purpose. Composability is bought with a static
dependency graph and explicit plugin composition, or it is not bought.

Moving the `construction` module into a dedicated lower crate is the expected
consequence once the inversion makes that move clean. Do not invent a package
name or force the extraction in the same commit if the post-inversion graph
reveals another unresolved owner. The authority inversion is the first hard
acceptance condition; the graph decides whether physical extraction is then
mechanical.

### Acceptance

At minimum:

```bash
python3 scripts/measure_kernel_module_graph.py --edges 20
grep -rn "crate::features" \
  crates/ambition_platformer2d_actor_monolith/src/construction \
  --include='*.rs'
```

The production portion of the second command must have no hits. Test references
must either move with the new owner or be justified as black-box test usage,
not as a way to keep production dispatch coupled.

Then run the D33 post-carve checks already owned by the queue/focused plan,
including generated module maps, planning citations, doc links, absence
contracts/capability accounting where affected, and the relevant Rust gates.

### Stop condition

After F1 lands, **stop selecting work from the candidate list below until the
module graph is re-measured and this page is updated.** The purpose of F1 is to
change the graph that chooses F2.

## Candidates - not yet executable packets

These are recorded so the next agent knows which questions are real without
mistaking them for approved moves.

| Candidate | State | What must be resolved before implementation |
|---|---|---|
| control / possession / body custody | **DESIGN NEEDED** | Decide the authority topology and final home of `PossessionState`; do not move the leftover `abilities/{possession,teleport,trapdoor,flyline}` family by directory name. |
| character materialization / presentation | **DESIGN NEEDED** | Decide ownership of `CharacterLoadStates`, then separate load/materialization, presentation, and live match activation along their real dependency directions. |
| world integration | **RE-MEASURE AFTER F1** | Re-count `world <-> features`, `world <-> construction`, and `world <-> session` after construction inversion before choosing an extraction boundary. |
| session / Ambition-game orchestration | **DESIGN NEEDED** | Name the composition owner above reusable actor/body domains before moving session, shrine, music/audio, or related policy glue. |
| remaining items adapters | **RE-MEASURE AFTER F1** | The world-item and held-item authorities already left. Re-measure the residue instead of treating the old `items/` line count as one domain. |
| low-coupling islands | **DEFER** | Do not choose these only because they are easy to move. Break the central semantic cycle first unless the live queue gives another reason. |

## Rules that prevent false progress

- **No LOC target.** Crossing 100k was a useful milestone; it is no longer a
  task-selection rule.
- **No wrapper carve.** A new crate that imports the actor monolith or leaves the
  same mutual authority cycle is not decomposition.
- **Move authority with lifecycle.** State, registration, scheduling, rollback
  declarations, tests, and public construction/SDK seams move with the domain
  when they are part of that authority.
- **Do not mistake ledgers for semantic coupling.** Broad registration files
  such as rollback/snapshot ledgers are expected to name many domains.
- **One graph-changing carve, then re-measure.** Do not pre-commit to F2/F3/F4
  from today's graph.
- **If the code contradicts this page, the code wins.** Re-measure, update the
  receipt, then continue. Do not implement a stale packet because it is marked
  READY here.
- ⭐ **A carve is scored on TWO dimensions and must say which one it moved.**
  Satisfying every ownership and dependency rule above is necessary and is not
  sufficient for *"decomposed"* in the sense the doctrine now uses. See "Which
  goal does this carve advance?" above; the criterion itself lives in
  [`decomposition.md`](decomposition.md) and is not restated here.

## Updating this frontier after a carve

Keep the update small:

1. stamp a new verified SHA/date;
2. replace the current graph receipt;
3. mark the landed packet complete in the live queue/focused plan as their
   contracts require;
4. promote exactly one next packet to **READY** only when its owner, dependency
   direction, production sites, and acceptance are measured;
5. ⭐ **record which of the two architectural goals the landed carve advanced**,
   in one line — authority decomposition, capability composability, or both.
   A frontier that never records the second is how a repository arrives at
   excellent internal boundaries and an externally indivisible engine;
5. leave unresolved candidates blocked rather than filling in an architecture
   from intuition.

A weaker agent should be able to open this page, re-run one measurement, and
know either exactly which bounded D33 task is safe to execute or exactly why no
next carve has been specified yet.
