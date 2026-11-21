# Capability linking and prepared facets

**State:** PLANNED, 2026-11-21. These are source-based design proposals, not
implemented capability or compile-time claims.
**Baseline:** `6895e516863872eb6d38093a27225ed8e2973d70`.
**Execution:** [the queue](../queue.md) selects bounded work from this owner.
**Related owners:** [composition](capability-and-runtime-composition.md),
[content generations](content-generation-and-reload.md),
[actor effects](actor-effects-and-modifier-composition.md),
[actor resources](composable-actor-resources.md), and
[public SDK](public-sdk-1.0.md).

## The architectural change

Make a selected game composition a linked set of domain contributions.
Each owner supplies its authored vocabulary, prepared values and native
interpretation. Preparation resolves their relations before a body or session
uses them. Native installation selects those owners explicitly. Runtime policy
then decides where an installed service may act.

This joins three current problems at their common cause:

- Content validation resolves names, but some domain values are lowered before
  those references resolve and are reconstructed later from App registries.
- The character definition and construction context collect fields from many
  domains. Each new feature makes more consumers understand that union.
- Runtime profiles subtract plugins from a full engine, while schemas, ports,
  rollback declarations and construction inputs are selected separately.

The common operation is **linking a selected set of owners against exact
inputs**. It is not a new general component registry. Keep the existing content
compiler, native plugins, typed construction and publication transaction.
Change the values and dependencies that connect them.

## What the source establishes

| Source | Current fact | Structural consequence |
| --- | --- | --- |
| [FacetOutcome](../../../crates/ambition_content_pack/src/schema.rs), [compile](../../../crates/ambition_content_pack/src/lib.rs) | A handler can emit `lowered` with declarations and references before global reference resolution. There is no final domain bind callback that consumes resolved targets. | A successful reference pass does not establish that the lowered value contains those bindings. |
| [PreparedCharacterDefinition::moveset_in](../../../crates/ambition_characters/src/prepared.rs), [WornKit::of](../../../crates/ambition_combat/src/worn_kit.rs) | Both prepare move overlays for damage, verbs, timing and knockback. Their fallback behavior differs. | Two readers can interpret the same selected character facts through different rules. |
| [Capability](../../../crates/ambition_platformer2d_shared_tangle/src/capability.rs) | One enum supplies room system gates, a removable-plugin subset and a partial mapping to content capabilities. | Installation, room permission and schema interpretation are not one-to-one. |
| [CapabilityComposition](../../../crates/ambition_platformer2d_runtime/src/profile.rs) | Several omission arms do nothing; some installation probes name a larger schedule plugin. | A profile is an implemented promise, but the enum does not define independent native service units. |
| [Rollback composition](../../../crates/ambition_platformer2d_runtime/src/rollback/mod.rs), [ports](../../../crates/ambition_platformer2d_runtime/src/extension_composition.rs), [schemas](../../../crates/ambition_engine_schemas/src/lib.rs) | These separately compose rewind declarations, ports and content schemas. | Selecting an owner requires several related decisions. |
| [ActorConstructionServices](../../../crates/ambition_platformer2d_construction/src/lib.rs), [SessionMechanics](../../../crates/ambition_platformer2d_construction/src/mechanics.rs) | Generic body construction carries boss catalogs and other domain inputs. | An absent boss plugin does not remove its construction dependency. |
| [PendingGenerationInputs](../../../crates/ambition_platformer2d_runtime/src/content_identity.rs) | Each reload family adds fields and accessors to a common candidate input structure. | A new domain extends the coordinator's knowledge of concrete catalogs. |
| [Engine-schema manifest](../../../crates/ambition_engine_schemas/Cargo.toml) | Schema tooling imports complete ECS capability crates. | A parser or validator can inherit native execution dependencies. |

These are static observations. Existing gameplay, profile and reload witnesses
remain useful. No new runtime failure or build-time speedup is established by
this review. Recheck the named paths before implementing a slice.

## Keep four distinct questions

| Question | Owner | Example |
| --- | --- | --- |
| What can this executable provide? | Cargo features and linked native code | This build has the portal implementation. |
| What did this App install? | Explicit native composition | This host installed body motion and combat, without cutscene playback. |
| What does this experience require or this room permit? | Game and room rules | One room permits captures; another room does not. |
| What can interpret this authored value or operation? | Selected domain offers | Item definitions may serve both collection and held use. |

Do not replace the current enum with a larger enum that claims to answer all
four. Their relation is many-to-many. A tool can understand a schema without
installing its native service. One App can install the union needed by several
experiences while each room retains its own permission rules.

State the distinction in diagnostics: unknown vocabulary, known but not built,
built but not installed, installed but not permitted here, and invalid authored
input can require different remedies. Do not invent a dormant service to make
an unsupported composition appear valid.

## Link one selected generation

Use these conceptual stages inside the existing compiler and admission path.
The names below describe responsibilities, not required public Rust types.

| Stage | Result | Failure leaves |
| --- | --- | --- |
| Declare | Source-aware definitions, typed imports, and unbound owner fragments | The active generation unchanged; diagnostics identify the source and owner. |
| Aggregate | Each owner's combined fragments, with explicit conflict policy | No partly combined owner value is runnable. |
| Link | Immutable owner values bound to the selected imports, rules and mechanical asset facts | No executable value with an unresolved required reference. |
| Admit | Proof that the selected native services can interpret the linked values with the required scope and lifetime | The previous session and selected generation unchanged. |
| Activate | One publication through the current hidden-candidate lifecycle | The old or the complete new selection; never a mixture. |

### Inputs and identity

Content linking reads one explicit immutable view: selected definitions,
rules, installed offers and exact generation. It includes the selected
mechanical sheet metadata when body geometry or attack volumes depend on it.
Retain the actual immutable metadata/product versions that supply those facts.
A digest beside a logical path that later follows a moving library head is
not a pinned input.

It does not read whichever catalog happens to be in `World` during execution.
An App can prepare a new generation while the active session still uses the
old one. The link input must name which one it means.

There are several binding times. Keep their lifetimes separate:

| Binding time | Result | What does not happen |
| --- | --- | --- |
| Content preparation | Linked domain definitions and available facet programs in one admitted generation | No current body custody or cooldown is captured as content |
| Body construction or equipment/form/ruleset transition | A specialization of those admitted definitions for the body's selected immutable terms | No new content epoch or whole-session reload for an ordinary equipment change |
| Simulation read cut | Current eligibility and results from live permissions, amounts, holds and cooldowns | No source parsing or static-reference resolution |

Reuse the owning pure binder where the semantic operation is the same.
At a body transition, pass selected definition IDs and stable policy values
through the existing accepted transition. Hand occupancy, custody, durability
and resource amounts remain live authority. A specialization's semantic key
names the immutable inputs that affect its result. A separate selection
receipt binds it to the admitted generation; an unrelated audio edit need
not duplicate the combat value. Neither identity denotes a new content
generation. On content reload, the existing candidate protocol validates
or rebuilds those specializations together.

Keep source location and diagnostic provenance separate from mechanical
values and their digests. A moved source file can change diagnostics without
changing a move's mechanics. A changed hurtbox must change the relevant
mechanical identity. The generation owner composes domain identities; it does
not repeat domain interpretation to calculate its own version of the value.

Resolve declarations before dependent values. A legal symbolic cycle does not
require a cyclic computation: two definitions may name each other after both
identities exist. A cycle in values that must be computed from one another
needs an explicit domain rule or refusal. Do not obtain an order by silently
using a default for whichever value is visited first.

A standalone compiler can report incomplete or unavailable host inputs. Only
a fully linked and admitted generation is runnable. This preserves the
difference between source validity and actual installed support.

### Reuse the current compiler

Change a selected handler from producing its final executable value in
`check` to producing an owner fragment with typed unresolved imports.
After declaration and aggregation, its bind step receives resolved imports
and produces the final owner value. The current schema registry can dispatch
these stages. Do not add a parallel registry of compiler participants.

The compiler may keep erased storage at its existing in-process boundary.
That storage is not the artifact format and must not become a public
`TypeId` lookup service. Domain code consumes typed values. Diagnostics name
the authored reference, expected domain, selected provider and source.

Replace each later re-resolution only after its owner bind step covers the
same inputs and refusal policy. Do not delete a late runtime check that
protects a genuinely mutable fact merely because static references now link.

## Prepared facets follow semantic owners

A prepared character is a composition of facts about one character, not the
owner of every fact that can affect a character. Keep the common identity and
body contract. Move an independently owned capability's prepared result to
that owner when a real consumer or dependency cut needs it.

For example, combat prepares an action repertoire from selected character,
equipment and ruleset inputs. A resource owner prepares pool identity,
capacity and compatibility. A mount owner prepares mount terms. These may
share the same character and generation key without sharing one ever-growing
struct or one generic property map.

Join the typed prepared facets in the composition that needs them. The body
constructor receives a coherent selected view. It does not query unrelated
registries to find missing pieces. Keep domain-specific invariants together:
a facet boundary must not split a body geometry fact from the motion rules
that give it meaning solely to reduce imports.

Prepare the stable repertoire and action-slot bindings. Current eligibility
still reads body permissions, cooldowns, control holds and room rules at the
declared simulation cut. Do not narrow the character's identity kit to what
the body is allowed to do this tick, or cache a transient permission as an
immutable facet.

### Identity and mutable state stay distinct

- The character identity answers which authored character this is.
- A facet identity answers which immutable domain meaning was linked.
- The selected composition identity binds the facets and their inputs.
- A runtime entity and occurrence identify a live body built from them.
- Mutable health, resource amounts, cooldowns and control remain live state.

`ResourceLayoutId`, `PreparedActorResourcePlanId` and whole composition identity
remain different facts. Do not reuse one digest as proof of all three.
Changing a capability does not imply resetting every live-state field.
The owner states retain, convert, rebind or refuse policy for that transition.

Cache only combinations that are actually admitted. Do not precompute the
Cartesian product of characters, rulesets and equipment. Keep a prepared value
while live state or rollback history can still refer to it; generic cache
eviction must not decide its semantic lifetime.

### Candidate inputs without a universal field bag

An owner contributes its prepared candidate value to the selected generation
and exposes a typed view to consumers that need it. The coordinator sequences
validation and publication; it must not gain a new concrete catalog field for
every future capability.

Start with explicit typed fields or tuples in the selected native composition.
Keep the shared generation receipt small: identity, selected owner identities,
dependencies and publication status. Do not replace `PendingGenerationInputs`
with a global map of arbitrary `Any` services. Do not create one generic
typestate for every possible engine feature combination.

The hidden-candidate protocol already owns all-or-nothing publication. Use it.
Domain contributions cannot publish themselves before that decision. A linked
candidate may borrow or share immutable domain values from its base when their
dependencies are unchanged; that does not make active mutable state shared.

## Native services contribute a complete contract

Selection of a native service must bring its required initialization, private
systems, public milestones, typed state declarations, preparation support,
procedural ports and retirement behavior together. The same owner supplies
these projections. Metadata describes that contribution; metadata alone does
not install it.

Use ordinary Bevy plugins and owner functions. Start with explicit selection
and direct typed calls. A helper is justified when it removes a repeated
semantic choice. A new trait hierarchy or dynamic plugin discovery is not the
goal. The facade's full/default profile becomes a preset over the same selected
services used by smaller games.

`RollbackRegistrar` has generic methods. A design that assumes it can be passed
as `dyn RollbackRegistrar` is invalid. Keep its existing typed backend-neutral
composition. Schema collection and a concrete rollback backend may make
separate passes over the same explicit service selection; they prove different
things and need not share one erased executable callback.

### Preserve order while changing selection

The current extension host chains request reducers in installation order.
`extension_host/src/lib.rs::install_request` records the previous port for a
phase and stage. `runtime/src/extension_composition.rs::install_ports` relies
on that order. Reordering contributions can therefore change gameplay.

Preserve each operation's observation cut, consume point, command flush and
retirement barrier during the first move. Then replace a relevant hidden
installation-order dependency with its actual domain prerequisite. Do not sort
installers by name or run all requests at the end of the tick.

One operation can have several phase bindings. Its domain rules and value
schema can be shared while its observation and consume contracts differ.
A late contact request cannot change integration that already happened.
A held effect can need retirement work on a tick with no submitted request.
These cases rule out one generic event queue as the replacement.

## Dependency direction and build boundary

The intended direction is:

`authoring/link tools -> domain value contract <- native execution adapter`.

A domain contract holds authored and linked values, validation and canonical
identity when these are meaningful without ECS. The adapter binds those values
to Bevy, native resources and schedule semantics. Both use the same validator;
the tool does not carry a second copy of the rules.

Start within the current owner module. Extract or feature-gate a compiled unit
only for an independent consumer, supported absence or measured rebuild
benefit. A schema type that still imports combat or sprite execution types has
not crossed the boundary merely because its filename says `spec`.

Do not move all schemas into one new foundation crate. That would give a new
name to the same dependency union. An all-schema CLI may select all pure
contracts, while a minimum game selects only what it needs. The native adapter
must depend on its value contract; the contract must not import the adapter to
obtain defaults or validation.

## Bounded implementation slices

These slices are options for [the queue](../queue.md), not a second execution
ledger. The first semantic slice does not wait for package extraction.

### L1 — One owner compiles the equipped action repertoire

**Customer:** an ordinary body wears a character, receives equipment and hand
overrides, and changes the selected ruleset through existing supported paths.

1. Trace `PreparedCharacterDefinition::moveset_in`, `WornKit::of`, effective
   repertoire assembly and action-scheme selection against the same inputs.
2. Make the combat owner compile the overlapping move decisions once. State
   precedence and refusal for missing or incompatible selected facts. Do not
   preserve two different fallback policies without a semantic reason.
3. Bind its required character, move, equipment and mechanical asset inputs in
   the selected link context. Native materialization consumes that result.
4. Remove the replaced overlay and re-resolution paths. Extend to action-scheme
   projection only after the first shared result covers its real inputs.

**Acceptance:** the same admitted inputs select the same actions for human,
AI and scripted control; changing one selected input changes the intended
result; a missing required input refuses before materialization; health,
motion and other mutable state follow the existing owner's transition policy.
An equipment change selects the new repertoire without changing ContentEpoch.
Use the current body and action witnesses at the narrowest useful crossing.

This is not a requirement to extract every actor facet. Resource-plan work must
consume this composition context rather than build a second actor compiler.
`BOSS-CHARACTER` continues to give bosses the common body plus an optional role;
its current common-seed slice does not depend on L1 or on package extraction.

### L2 — Link one candidate across two mechanical owners

Use the L1 repertoire and a character form or mechanical sheet fact it consumes.
Change both in one candidate. Link their imports before creating the candidate
body; make one referenced value invalid and keep the old generation active.
Run the valid candidate through startup and the current development reload
road. A later room rebuild must use those same admitted values.

Delete the migrated provider's duplicate catalog lookup or overlay. Retain
rechecks for mutable progress or ownership that can change before publication.
The [generation owner](content-generation-and-reload.md#link-one-selected-generation)
owns the rest of this transaction and cross-submodule candidate selection.

### L3 — Make one installed service an honest optional contribution

Use cutscene playback as the bounded candidate: it is already omittable, has
authored scripts, a native stepper, rollback state and session retirement.
Collect its current owner operations under explicit selection and let a
cutscene-free composition refuse authored requirements at admission.
Keep two experiences with different room permissions in the same App.

`SchemaRegistry::register` currently adds the schema owner to its capability
set. That can describe compiler vocabulary and authored requirements; it
cannot prove native installation. A tools-only composition may register and
link the cutscene schema yet refuse playback at host admission. Remove any
use of that compiler set as executable-support proof. The admission reads
the actual selected native offers, without another compiler registry.

The pure format/stepper/schema can be a separate build pilot. Its source says
these parts do not need ECS, but `ambition_cutscene/Cargo.toml` currently makes
Bevy and platformer core mandatory for the whole crate. Use an optional runtime
feature or a separate package only when the isolated consumer proves it useful.
For that consumer, establish that ECS and rollback dependencies are absent from
the resolved normal graph. This does not by itself remove them from the
all-schema CLI through other paths.

After the pilot, select the next service from actual consumer pressure.
Ordinary character construction should stop requiring boss orchestration inputs
as the existing common-body work permits. Do not create a second boss body
contract to keep a provisional construction branch alive.

### L4 — Prove the selected compile closures

For each actual dependency cut, use independent manifests outside workspace
feature unification. State the selected features and inspect normal, build,
dev and target graphs separately where they matter.

| Consumer | Required useful behavior | Dependency question |
| --- | --- | --- |
| Minimum body/world | Construct, accept intent, move and collide | Did the cut remove the omitted native service and its path back through construction? |
| Combat | Prepare and execute an equipped action | Can it use the combat contract without named boss orchestration or unrelated authoring tools? |
| Boss-capable game | Common body plus authored boss role | Did the cut preserve the optional role and all required preparation? |
| Schema/link CLI | Parse, resolve and diagnose the migrated domain | Does it avoid the domain's ECS execution adapter? Name surviving paths. |
| Portable procedural module | Use the required pure ports and values | Does it remain free of the host and Bevy feature closure? |

Record wall-clock and rebuilt-unit changes with the existing telemetry for the
edit class that motivated the cut. A smaller graph is evidence of absence,
not a speedup measurement. Do not add a repository-wide schema or descriptor
completeness checker.

## What completion removes

The useful result is less knowledge in consumers:

- One owner compiles each migrated facet instead of several readers repeating
  its precedence and fallback rules.
- Linked values carry resolved required imports; providers do not resolve the
  same static references again against a different App selection.
- A new migrated capability does not add fields to every construction and
  candidate context.
- The selected service supplies its interpretation and lifetime operations;
  omission does not leave a dummy catalog as a construction prerequisite.
- Pure authoring consumers use the domain contract without its native adapter.

Keep authority, installation/lifetime, resolved compile closure and SDK
usability as independent acceptance axes. None can be inferred from a new
type name, fewer crates, fewer fields or fewer lines.

## Do not build these substitutes

- A universal property bag or central all-domain schema enum.
- A second compiler registry beside `SchemaRegistry`.
- A service locator that finds concrete native owners during a simulation tick.
- A generated type for every combination of capabilities.
- A single merged actor state that owns health, equipment, movement and effects.
- A new lifecycle transaction beside hidden candidate publication.
- A blanket crate-splitting campaign before the first useful consumer improves.
