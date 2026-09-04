# Decomposition doctrine — durable authority moved

> ⛔⛔ **DO NOT RETIRE THIS PAGE WITHOUT REPOINTING EIGHT POLICY ROWS FIRST — I
> deleted it on 2026-09-03 and my own guard caught it.** It has zero inbound
> links from `docs/planning` and its standing rule IS absorbed by
> [`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md),
> so both halves of the usual retirement test passed. ⚠ **The test misses a
> third reference class: `source_doc` fields in the workspace-policy TOMLs.**
> Eight rows cite this path — five in `engine.toml`, two in `module_size.toml`,
> one each in `game.toml` and `repository.toml` — and
> `every_source_doc_names_a_real_file_and_heading` went red in the feature-union
> job within the hour. ⇒ Before deleting any planning page, run
> `grep -rn "<path>" tests/ambition_workspace_policy/policies/*.toml` as well as
> the prose sweep. This is the same reason `engine/architecture.md` earns its
> keep, met a second time by a different road.


**State:** doctrine distilled; no independent execution queue lives here.

The durable package/dependency rules formerly maintained in this file now live
in
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

Use the focused plans for current work:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) — the
  measured residual actor-kernel frontier, with
  [`actor-monolith-work-frontier.md`](actor-monolith-work-frontier.md) as its
  bounded executable packet for the carve that is READY now;
- [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
  — product/capability dependency closure;
- [`public-sdk-1.0.md`](public-sdk-1.0.md) — consumer-facing semantic API pressure.

Standing rule: **decompose by ownership and dependency value, not by line count.**
A carve should move one coherent authority with its registration and remove a
real dependency/change-fanout path, or provide another concrete isolation/SDK
benefit. Runtime performance is not assumed; measure it separately.

## Decomposition has two dimensions

⭐⭐ **AUTHORITY DECOMPOSITION AND CAPABILITY COMPOSABILITY ARE SEPARATE
ARCHITECTURAL SUCCESS CRITERIA. This page owns that rule.** Everything else in
planning applies it; nothing else restates it.

**Authority decomposition** — correctness and maintainability inside the engine,
and the work Ambition has been doing extensively. Which crate owns this fact?
What may mutate it? Is there one authoritative lifecycle? Does the dependency
direction point toward lower-level vocabulary? Did the schedule move with the
state machine? Is presentation downstream? Is there a competing implementation?

**Capability composability** — the engine as presented to users and integrators.
Can this capability be ABSENT? Does the rest still form a coherent application
without it? Does it declare only its real prerequisites? Does an unrelated
sibling have to exist merely because the full Ambition game normally installs
both? Can a user choose another implementation at an appropriate seam? Is the
full composition a convenience rather than the only supported architecture?

⛔ **The second does not follow from the first.** A repository can have excellent
internal boundaries and still expose an effectively indivisible engine: combat,
items, encounters, portals, dialogue, persistence, projectiles and rendering each
in their own crate, one plugin group that always installs all of them, and
several of those crates assuming resources their unrelated siblings register.
That is internally modular and externally monolithic.

⇒ Authority decomposition is generally a PREREQUISITE for composability and is
not sufficient for it. A domain in a clean standalone crate can still be
mandatory in every supported composition.

### The ordering is not negotiable

1. establish correct semantic authority;
2. establish correct dependency direction;
3. expose intentional extension seams;
4. let higher layers compose capabilities.

⛔ Do not sacrifice 1 and 2 to reach 4. Optionality does not outrank correctness
or deterministic simulation, and singular authority is not weakened by any of
this. **Capability composability is not runtime polymorphism**: a static Rust
dependency graph with explicit plugin composition provides it well. Do not reach
for a service locator, a type-erased registry, dynamic dependency injection or
global plugin discovery to make a crate look optional.
⭐ And sequencing is explicitly permitted: move authority into the right domain
now, invert a remaining dependency later, make the capability independently
installable after that. A carve need not deliver all three at once.

### The user model

A user should be able to think *"I need the Ambition platformer foundation and
these capabilities"* rather than *"I must instantiate the whole engine and ignore
what I do not use."* Both of these are supported architecture:

- a **default composition** — convenient, opinionated, installs the normal stack,
  and is what ordinary users and the Ambition game itself take;
- a **capability composition** — a deliberate subset or an alternate
  implementation at a stable seam.

⛔ Not a slogan. "Everything is a plugin" is too strong and probably wrong, and
"every crate must be optional" is wrong outright. Many crates are foundational
vocabulary, adapters, implementation layers or product-specific modules, and
several small ones exist precisely to fix dependency direction and should be
consumed transitively. **The requirement applies to MAJOR REUSABLE
CAPABILITIES** — combat, projectiles, encounters, held items, world items,
portals, dialogue, cutscenes, persistence, rendering, possibly mounts — where
absence is a meaningful thing for a consumer to choose.

### What the property looks like

Compositions of roughly this shape should be architecturally possible, given the
capability's true prerequisites. ⚠ **These are the target TEST of whether
capability decomposition is succeeding, not a claim that they work today:**

```text
movement and world simulation without combat
combat without inventory or held-item systems
characters without encounters
generic encounters without boss encounters
world collectibles without the held-item state machine
a platformer without portals / dialogue / cutscenes / persistence
simulation without the default renderer, or without presentation at all
a custom presentation layer reading stable simulation views
projectiles without unrelated item or encounter systems
mounts, or relativistic simulation, only when explicitly selected
```

### A stronger meaning of "decomposed"

A major reusable domain is not necessarily fully decomposed because its
authoritative implementation sits in a dedicated crate. For a domain intended to
be independently selectable, complete decomposition also requires that:

- its prerequisites are explicit;
- unrelated sibling capabilities are not assumed;
- installation and lifecycle compose through stable extension seams;
- its ABSENCE is supported by the rest of the engine;
- product-specific composition lives above it rather than inside lower layers.

This complements the authority-transfer rules; it does not replace them.

### Absence is a first-class case

Current architecture spends real effort proving one authoritative path EXISTS.
Composability adds the opposite question: what happens when the whole subsystem
is absent? For a capability meant to be optional, absence should be an ordinary
supported configuration rather than an exceptional one — in resources, messages,
schedules, startup assumptions, queries, registries, presentation, persistence
and tests. A capability should not need a placeholder resource from another
optional capability to keep its systems from being skipped or panicking.
⛔ This is not a prescription to wrap everything in `Option<Res<..>>`; that is an
implementation detail and often the wrong one. The expectation is that an
optional boundary is represented INTENTIONALLY rather than by assuming the full
stack is always present.

⛔⛔ **AND WHEN A CAPABILITY DEGRADES TO A DEFAULT ON ABSENCE, THE DEFAULT MUST
NOT BE A VALID-LOOKING VALUE FROM THE SAME DOMAIN.** Measured 2026-09-04
(yardrat): the fighter ladder's authored per-level rows arrive only with
`AuthoredFighterLadder`, which `ambition_content` inserts and which neither smash
crate depends on — so the engine floor's `UtilityWeights::default()` was used
instead, and that default IS the ladder's level-9 row. Every rung therefore ran
level-9 priorities, and no output could tell that from the authored ladder. ⇒ The
absence was not merely unsupported; it was INVISIBLE, and every number the
project had recorded from that rig described the fallback.
⚠ The proof was a null result and could not have been anything else: removing a
redundant override changed nothing in ten of ten cells, which is only possible if
the authored rows were never arriving.
⇒ So an optional capability owes its consumers a way to tell absent from
defaulted. A default that is a plausible member of the value space it replaces
turns a composition error into a silent measurement error.

⛔⛔ **AND THE OTHER FAILURE MODE IS THE OPPOSITE ONE: A CAPABILITY THAT CANNOT
DECLINE AT ALL.** In Bevy 0.19 a missing system parameter is a HARD FAILURE that
takes the whole `App` down, so a plugin whose systems demand resources a minimum
host lacks is not degraded — it is a capability that cannot be composed. Measured
twice, five days apart: `sync_portal_view_cones` was 37 of the feature union's
original 48 failures, and `ambition_sprite_fx::draw_sprite_effects` was **every
one of 40** against 7,072 passes on 2026-09-04.

⭐ **Both times the plugin ALREADY had a guard that read like the right one**, and
that is the part worth carrying: `SpriteFxPlugin` returned early when there was no
`EmbeddedAssetRegistry` — which answers *"is there an `AssetPlugin`"*, and a
headless demo HAS one. What it lacks is a render stack. ⇒ **A prerequisite check
must name the RESOURCE the system demands, never a proxy for the subsystem it
belongs to**, because the proxy is true in exactly the composition that fails.

⇒ The mechanics, the test shape and the complete-set question are in
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
under Principles, so they live with the program that measures them rather than
being restated here.

### Intended layering

Conceptual, and deliberately without crate names where the architecture has not
settled them:

```text
foundation and extension seams
  geometry, time, deterministic body simulation, world query vocabulary,
  semantic intent, schedule/lifecycle extension points, construction
  protocols, observation seams, registry mechanisms
        ↓
reusable platformer capabilities
  substantial domains that register themselves against those seams
        ↓
Ambition product-specific modules
  particular bosses, shrines, authored characters, story cutscenes,
  game-specific match/session policy
        ↓
the Ambition composition
  chooses and installs what the game actually uses
```

Foundations may be common prerequisites; they must not know about every
higher-level optional capability. Product modules may depend on reusable
capabilities. ⛔ Reusable lower layers must not depend upward on product-specific
modules. ⚠ This page does not classify the existing crates into these tiers, and
a future audit should not treat the lists above as an inventory.

### Runtime and shared scheduling are the named risks

A runtime crate legitimately owns common schedule infrastructure, lifecycle
phases, plugin ordering seams, shared execution contracts and the resources the
platformer simulation itself needs. ⛔ It must not become the semantic owner of
every optional capability — a runtime that hard-codes *"install combat, then
items, then portals, then encounters, then bosses, then cutscenes"* with each
capability relying on that central knowledge is the failure mode. The healthier
direction: runtime exposes stable seams, capabilities install themselves against
them, and a higher-level default composition selects which to include. That is
the narrow property worth taking from Bevy — **major capabilities can be
independently installed and composed** — and not its API or its implementation.

The same risk applies to shared schedule vocabulary. Sets that name stable engine
PHASES — intent collection, actor decision, simulation, contact resolution,
authoritative finalization, observation, presentation — are healthy extension
seams. ⛔ What must not appear in a foundational scheduler is pairwise knowledge
of optional capabilities: an ordering equivalent to *"held-item reconciliation
runs after shrine maintenance and before portal-gun bookkeeping"* belongs
between those capabilities if the dependency is real, and should not exist at all
if it was manufactured by the full game installing both.

### Public API and internal topology are different concerns

Composability does not mean consumers depend on dozens of implementation crates.
`ambition_platformer2d` can remain the public API, and it may be desirable for
the facade to expose stable capability GROUPS while hiding internal topology —
something like a minimal foundation, a default capability set, individual major
capability groups, and the full Ambition composition. The API design is out of
scope here; the architectural statement is that internal decomposition and public
ergonomics are separate concerns.

### Future validation: minimum-host tests

Not built yet, and named so this criterion is testable rather than aspirational.
A minimum-host test installs one capability plus only its declared prerequisites
and proves the result runs; its purpose is to expose hidden sibling dependencies.
Candidates:

- **minimal foundation** — the smallest headless platformer app, with body/world
  simulation working and no rendering, combat, encounters, items, portals or
  dialogue;
- **combat composition** — that foundation plus combat, initialising and running
  without inventory, encounters, portals or dialogue;
- **encounter composition** — generic encounter support with its declared
  prerequisites and NO boss encounter, with the generic lifecycle still working;
- **presentation independence** — simulation with no `ambition_render`, proving
  the stable read models exist; then the default renderer installed separately and
  shown to consume those facts without becoming simulation authority;
- **item separation** — world collectibles without the held-item capability, so
  touch acquisition does not require the hold/use/throw state machine.

⇒ These are architectural probes. They answer one question: *did this crate
become an actual capability boundary, or did we only relocate source code?*

### Vocabulary

Use: authority, capability, prerequisite, composition, extension seam,
foundation, reusable platformer capability, product-specific module, default
composition, minimum host, absence, lifecycle ownership.

The previous execution ledger is available in git history. Do not rebuild that
ledger in live planning.
