# Capability and runtime composition

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**State:** active, with readiness assessed per authority. Prior statements that
all authority prerequisites were crossed were too broad. The spawn correction,
checkpoint ownership and contact geometry show why those conclusions must remain
local to the boundary that was actually tested.

[The queue](../queue.md) selects work. The
[responsibility map](architecture-responsibility-map.md) defines the target and
[A9](actor-monolith-work-frontier.md) defines profile/closure work. This page owns
installation, scheduling, prerequisites and supported absence.

## Fresh source measurements

Executed on the baseline with the existing Python tools:

| Instrument | Result | What it establishes |
| --- | --- | --- |
| `scripts/measure_foreign_system_ordering.py` | 0 capability/ruleset foreign private orderings; 73 composition orderings; 174 foreign installations | Syntactic installation/ordering inventory, not semantic correctness |
| `scripts/measure_carveable_installations.py` | 3 mechanically reducible blocks; 38 mechanically irreducible blocks | Upper-bound candidates using present package edges; package names may conceal mixed authorities |
| Normal nonoptional workspace-manifest traversal from facade | 51 other workspace packages reachable | Lower bound on dependency closure; not full Cargo resolution, binary size or installed-system population |

The body-clock contribution is already expressed through published reset/
contribute vocabulary. Do not reopen that repaired C1 row. Older counts of one
capability ordering or 175 installations are not this receipt.

```bash
python3 scripts/measure_foreign_system_ordering.py
python3 scripts/measure_carveable_installations.py
```

A tool's irreducible label means two current packages occur in the block without
a suitable dependency edge. It does not prove that their responsibilities are
semantically independent. Reassess ownership before adding a composition wrapper.

## Four independently testable contracts

**Authority:** private state and its transition rules have a coherent owner.
**Installation/lifetime:** an installed capability runs and retires with only
its documented prerequisites. **Compile closure:** an absent optional capability
is absent from the intended dependency/feature closure. **SDK:** an external
consumer uses stable semantic entry points instead of internal topology.

A passing optional-plugin suite does not prove compile closure. A green SDK
allowlist does not prove the allowed API is well designed. A decreasing foreign-
installation count does not prove the new installer owns the behavior.

## Capability installation contract

A capability declares required resources/services, optional integrations, private
systems, public milestones, state scope, rollback participation and retirement.
It owns internal ordering. An installer may be an ordinary Bevy-native function
or plugin according to the owner's public API; a new trait hierarchy is not
required.

Missing required prerequisites fail with a concrete installation/preparation
diagnostic. Supported absence is modeled intentionally. Do not require dummy
resources or let required behavior vanish behind an Option parameter. Do not
recognize render readiness by the presence of a proxy such as AssetPlugin when
the actual system requires render-device resources.

The combat no-plugin packaging question remains Q73. This plan recommends
owner-controlled installation against public phases; it does not fabricate a
maintainer ruling or require every crate to expose a Plugin type.

## Composition owns integration, not every algorithm

A full host may select capabilities and order their public milestones. The
lifecycle coordinator legitimately owns admission/loading/commit state machines.
Those are different roles even where they currently share the runtime crate.

A composition block is justified when it states a real relation between
independent owners, such as capture after custody settlement or presentation
after body geometry publication. It is suspect when it enumerates the private
steps of one owner, performs that owner's state transitions, or knows every
special case inside a generic context.

Do not move gameplay into runtime to escape Cargo direction. Do not delete
cross-capability coordination merely to reach zero foreign installation counts.
A wrapper that forwards the same 15 private calls has not reduced knowledge.

## Scheduling contract

For every public milestone, specify what is true on entry/exit, its enclosing
simulation phase, run conditions, whether Commands have been applied, and the
scope/population to which its guarantee applies. Use ordering only where a real
read/write or semantic prerequisite exists; do not chain unrelated systems to
avoid reasoning about them.

The A1 checkpoint move must preserve two different requirements: reset admission
runs before replay admission in PlayerInput; startup restoration can run before
gameplay is enabled. Item capture observes settled item/custody state in the full
composition. Those cannot be captured by copying one `.before` expression.

A milestone named after every private function merely republishes implementation
order. Prefer a guarantee such as body geometry published, accepted control
settled, or checkpoint capture ready. Empty optional phases must not prevent
required phases from running.

## Separation mechanisms: when they reduce knowledge

| Mechanism | Legitimate use in this repository | Failure to reject |
| --- | --- | --- |
| Direct Cargo dependency | Body execution using geometry; a domain adapter using prepared definitions | Assuming an acyclic graph proves that data and writers are at the right owner |
| Bevy plugin/installer | Owner installs private systems and state against documented phases | Wrapper around foreign private algorithms solely to change a metric |
| Published SystemSet | Stable ordering/visibility guarantee between independent owners | One public set per private function, with the same undocumented pairwise graph |
| Typed intra-tick message | Explicit event observation with known delivery/consumption semantics | Replacing a required synchronous contact/admission result with next-tick delivery |
| Rollback state/journal | Speculative state restored on rewind; confirmed external effects released once in the current session/process | Treating all Bevy message buffers as rollback history or using the effect journal for ordinary gameplay communication |
| App-local provider registry | Explicit independent game/content providers registered and frozen before use | Dynamic gameplay service discovery or arbitrary implementation replacement during a deterministic tick |
| Schema/metadata registry | Validate IDs, schemas, revisions and conflicts; fingerprint declared data | Treating metadata-only registration as an executable extension point |
| Backend-neutral registrar | Domain declares its rewind state without importing the GGRS backend | A backend-owned global type list that must understand every optional domain |
| Shared values / SystemParam | Small semantically owned values or borrow grouping of one operation | Dependency-neutral bags that carry every sibling's private resources and policy |

`ambition_registry_core` is a small canonical-registration helper. It does not
supply the authority, lifetime or meaning of a registry's entries. Retain explicit
New/Idempotent/Conflict behavior and deterministic enumeration; do not promote it
to a universal capability service locator.

## Compile-time optionality

At this baseline, the facade's direct render edge is optional, but facade ->
platformer2d_host -> ambition_render is nonoptional. Removing the direct feature
therefore does not remove renderer dependencies. Other capability paths must be
traced individually. See F5 in [findings](architecture-review-findings.md).

Measure a real independent consumer manifest under the intended feature set.
Cargo features unify across dependency paths; a default-features opt-out on one
edge does not erase another edge's request. Check normal, build, dev and target
closures separately. The supported production profile is defined by normal
runtime dependencies and its deployment assets, while tests may need additional
tooling. Do not infer binary bytes from any of those graph counts.

Start with a small supported profile set rather than promising every combination:
headless body/world; windowed body/world; headless combat; collection without held
use; generic encounters without named boss content. Rich default composition
continues to be supported. Each profile must instantiate a real domain object and
advance behavior, not merely build an empty App.

## Rollback and external effects

A domain owns declaration of its authoritative rewind state, with the backend
implementing the registrar. Install/register an optional domain only when its
profile includes it. Runtime loading/unloading of arbitrary rewind-owning plugins
is outside the present compile-time composition contract.

Preserve wire IDs/encoding during an ownership-only move. The engine's same-build
policy remains authoritative; this is not a promise of cross-release wire
compatibility. Definitions, derived projections and external effects have
different restoration semantics and must not share one generic default policy.

`crates/ambition_platformer2d_runtime/src/external_effects.rs` already provides a
bounded confirmed-effects journal. Preserve replacement on resimulation, empty
frame replacement, session reset and delivery after confirmation. Exactly-once
in-process release is not a durable exactly-once guarantee at a disk/network sink;
that sink needs its own identity/idempotence contract when required.

## Ruleset and session scope

Games choose policy and capabilities; sessions own active lifetime; hosts choose
platform/backends. Restore the exact prior process policy when a scoped ruleset
leaves. A demo plugin must not permanently become the global owner of a setting
used by another experience. Re-entry, not only first startup, is a profile test.

## Completion evidence

Keep zero capability/ruleset foreign private ordering, but do not use that as the
only criterion. Every claimed optional capability needs explicit prerequisites,
a minimal positive case, a supported absence case, correct re-entry/retirement,
rollback declaration and a resolved dependency-closure statement. Cross-owner
composition has a reason and a phase guarantee. SDK consumers need no internal
module map. Unsupported configurations report unsupported; they do not use a
plausible sibling default and produce misleading benchmark results.
