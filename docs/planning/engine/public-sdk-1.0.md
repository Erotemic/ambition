# Public SDK 1.0

**State:** open; baseline `300004d601af1e633cfaee969f079cf9bb368ca8`.
The supported product is an ergonomic programmatic engine API, with validated
agent-native authoring and an explicit build/package path. Internal crate names
are not the public capability model.

[The queue](../queue.md) chooses execution. The concrete profile packet is A9 in
[the frontier](actor-monolith-work-frontier.md); current owners are in the
[responsibility map](architecture-responsibility-map.md).

## Current implementation and remaining gap

The external consumer can use `PlatformerApp::windowed`, mounting and build
operations rather than hand-ordering the internal plugin sequence. Preserve that
improvement. The facade has curated semantic namespaces but also retains
crate-shaped mirrors, including access to unfinished implementation boundaries.
Removing mirrors is an API task, not proof of dependency-footprint reduction.

Current SDK source contracts cover the external consumer, minimal game, sim
harness and capability-demo consumers. Their allowlists protect against known
internal imports; they do not prove that every allowed namespace is coherent,
that every selected profile works, or that an omitted capability is absent from
Cargo's transitive closure. Do not add a blind-agent ritual to every API change.

The facade's compile closure and the content author's compile closure are
separate contracts. Current source and the explicitly limited manifest traversal
are recorded in [extension evidence](extension-iteration-evidence.md). Use A9's
feature-resolved external fixtures for composition claims; do not copy an older
mandatory-render path or package count as current evidence.

## Public surface families

Expose composition/profile selection, provider/prepared content, body/character
definitions, participant/actions, spatial worlds, lifecycle/transition,
simulation stepping/queries, views/presentation, asset readiness, diagnostics and
build/package integration. Expose Bevy-native extension where appropriate;
wrapping every Bevy API adds little value.

These are discoverable concept families, not a mandate to add a public module
for every current crate. Keep one facade for common host/game composition. That
facade is not the dependency of the independent content builder or portable
procedural SDK. Those use the narrow value/port surfaces in the
[extension model](extension-model.md). A separate static Bevy adapter preserves
native ECS ergonomics without feature-unifying Bevy into portable authors.
Advanced engine plugins retain normal Bevy APIs. Accidental broad reexports do
not become a supported lightweight boundary.

## Supported-profile ladder

The table defines target acceptance, not current passing status. Record each
profile's actual fixture/result and limitations when implemented.

| Profile | Required useful behavior | Negative requirement |
| --- | --- | --- |
| Data authoring | Independent builder emits an admitted portable move/content artifact | No Bevy, rendering, audio, host or named-game dependency |
| Procedural game module | Independent code and schema run through admitted ports and real rollback | No host relink for ordinary module edits; no raw World in the portable API |
| Headless body/world | Construct a prepared body, accept intent, collide with world geometry and expose state after ticks | No renderer/audio, named game content, inventory or encounter prerequisites |
| Windowed body/world | Same simulation plus a view, input and prepared visual assets | Rendering observes simulation; changing quality/view does not change replay |
| Combat | Authored action, target geometry, accepted reaction and deterministic result | No boss content, inventory, dialogue or shrine needed for ordinary combat |
| World collection | Spawn and acquire a physical collectible with correct occurrence accounting | No hold/use/throw capability required |
| Generic encounter | An external provider orchestrates reusable encounter semantics | No Ambition boss catalog or cutscene authority as a hidden prerequisite |
| Full/default game | Current flagship and acceptance games with supported services | Convenience profile does not define the minimal foundation |

Do not promise the full Cartesian product. Document known unsupported combinations
and their diagnostics. Distinguish installation, compile closure, asset/package
requirements and platform support for each profile.

## Migration method

Attempt a useful task through the facade from an independent consumer. Record
exactly which private fact or internal path it needs. Decide whether the missing
surface is engine capability, provider policy or game-specific code. Add the
smallest semantic API, migrate the consumer and delete the old internal path.

For character authoring, use the existing definition/preparation/placement path;
do not teach the removed archetype model through a compatibility alias. For
world authoring, keep spatial definitions separate from the LDtk adapter and
actor-aware construction lowering. For participants, keep driver/home-body/view
identity distinct. For inspection, expose read-only prepared/simulation facts
without requiring an internal debug crate.

Internal decomposition and SDK cleanup may proceed in bounded parallel slices.
Do not wait for an SCC score before fixing one proven API leak; do not stabilize
a mixed implementation container as the public API merely to hide the leak.

## Agent-facing discovery

Domain owners provide machine-readable list/describe/schema/reference/diagnostic
projections of their actual installed/prepared vocabulary. The projection must
carry profile, revision and provenance. It is read-only discovery, not a global
service locator or reflection engine that mutates arbitrary components.

A11 makes technique existence/parameter validation part of the same offer that
installs its handler. A key known to the repository but absent from a selected
profile must be distinguishable from both unknown and installed. Schema-only
support cannot be advertised as working runtime behavior.

Examples should be complete enough to compile, prepare content, advance behavior
and explain a deliberate failure. A one-line API sample with hidden global
initialization is insufficient for an external author.

## Failure and compatibility contracts

Preparation errors report provider, definition, field/reference, expected kind
and source location where available. Unsupported nondefault authored semantics
cannot be accepted as if active. Runtime defensive errors still exist for trusted
Rust callers that bypass preparation; they are not a substitute for admission.

Current pre-release internal paths may change without compatibility aliases.
Before SDK 1.0, decide which public type/schema contracts are stable and how
versioned authored data is migrated. Keep save format, authored schema, prepared
content identity, public Rust API and rollback wire policy separate. Same-build
rollback does not imply persistent-save compatibility, nor vice versa.

## Verification procedure

Use a consumer outside inherited workspace feature unification. Inspect its
resolved Cargo metadata and feature tree, not just the facade's manifest.
Exercise headless/windowed behavior and teardown/re-entry independently. Package
one declared desktop target from a clean build with explicit asset inputs;
missing generated packs/toolchains are incomplete prerequisites, not a pass.

A profile receipt states the exact manifest/features, engine revision, target,
selected capabilities, installed behavior, dependency closure, prepared content
identity, commands/results and unsupported services. Measure build time and
binary/asset bytes separately; neither is inferred from crate count.

## Exit

A Rust/Bevy developer or LLM author can build a small game with a world, body,
input, an authored action/object, optional presentation and a lifecycle transition
through public docs/discovery, without reading migration plans. The same project
has headless tests and a noninteractive release-artifact route. Each advertised
optional capability has a tested positive case and a supported absence case.
