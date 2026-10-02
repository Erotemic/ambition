# Capability and runtime composition

**Scope:** installation, scheduling, prerequisites and supported absence of
engine capabilities. **Doctrine:** [engine architecture](architecture.md).
**Profiles and closure work:** A9 in the
[packet catalog](actor-monolith-work-frontier.md). **Priority:**
[the queue](../queue.md). Readiness is assessed per authority; a claim that one
boundary is ready does not certify another.

## Measurements

Re-run these before you quote a number. Do not copy counts into other pages.

```bash
python3 scripts/measure_foreign_system_ordering.py
python3 scripts/measure_carveable_installations.py
python3 scripts/check_facade_dependency_closure.py
python3 scripts/measure_minimum_profile_closure.py --minimum
```

- `measure_foreign_system_ordering.py` is a syntactic inventory of foreign
  installations and orderings. A binary root counts as composition whatever its
  package is called. It does not prove semantic correctness.
- `measure_carveable_installations.py` lists upper-bound candidates. An
  "irreducible" label means two packages share a block without a suitable
  dependency edge; it does not prove semantic independence.
- `check_facade_dependency_closure.py` owns the facade closure count and fails
  any planning page that disagrees. It also fails if `ambition_render` re-enters
  the facade's mandatory graph.

The capability-footprint sentinel (`fixtures/minimal_game`) links 56 other
workspace packages besides the facade — the `ambition_closure` of
`scripts/baselines/capability-footprint-baseline.json`, a different subject
from the facade closure. Re-quote this sentence in the commit that changes the
baseline.

## Four independently testable contracts

- **Authority:** private state and its transitions have one coherent owner.
- **Installation and lifetime:** an installed capability runs and retires with
  only its documented prerequisites.
- **Compile closure:** an absent optional capability is absent from the
  intended dependency and feature closure.
- **SDK:** an external consumer uses semantic entry points, not internal
  topology.

A passing optional-plugin suite does not prove compile closure. A green SDK
allowlist does not prove a good API. A falling foreign-installation count does
not prove the new installer owns the behavior.

## Capability installation contract

A capability declares required resources and services, optional integrations,
private systems, public milestones, state scope, rollback participation and
retirement. It owns its internal ordering. An installer is an ordinary Bevy
function or plugin; no new trait hierarchy.

A missing required prerequisite fails with a concrete installation or
preparation diagnostic. Model supported absence on purpose. Do not require
dummy resources or hide required behavior behind an `Option` parameter. Do not
detect render readiness by a proxy such as `AssetPlugin` when the system needs
render-device resources.

**Q73 (ruled):** opaque installation is prohibited, not the Bevy `Plugin` type.
A capability may install its private systems through its own plugin when the
host requests the capability explicitly, the systems sit in documented public
milestones and the composition root controls whether the capability exists and
the order between published boundaries. A plugin that silently installs
unrelated capabilities or hides scheduling dependencies is not allowed.

## Procedural port installation

The [extension contract](extension-state-and-execution.md) adds executable
consumers of published domain ports. The owning capability installs each port's
schema, read projection or request reducer, phase guarantee and prerequisites
together. A metadata-only declaration cannot authorize a call. The runtime
composition root wires the host to selected domain offers; the host never
depends back on the runtime crate or enumerates game algorithms.

Modules write only their own registered state. Engine state keeps its owner.
Resolve phase dependencies at admission, reject cycles and preserve `Commands`
flush and run-condition contracts. Module invocation is stable and serial;
parallel invocation needs declared access and a merge proof.

[Domain contracts](extension-domain-contracts.md) keep operation schemas at the
domain owner and common wire and state primitives in the SDK. No central
all-requests enum, and the executor never imports every domain. An installed
port includes implementation, scope and grant, observation cut, consume barrier
and result. Bevy messages are not rollback queues.

## Composition owns integration, not every algorithm

A host selects capabilities and orders their public milestones. The lifecycle
coordinator owns admission, loading and commit state machines. Those roles can
share the runtime crate and still differ.

A composition block is justified when it states a real relation between
independent owners (capture after custody settlement, presentation after body
geometry publication). It is suspect when it lists one owner's private steps,
performs that owner's transitions or knows its special cases. Do not move
gameplay into runtime to escape Cargo direction. A wrapper that forwards the
same private calls has not reduced knowledge.

## Scheduling contract

For every public milestone, state what is true on entry and exit, the enclosing
phase, run conditions, whether `Commands` have been applied and the population
the guarantee covers. Order only on a real read/write or semantic prerequisite.

Examples that one `.before` cannot capture: reset admission runs before replay
admission in `PlayerInput`; startup checkpoint restoration can run before
gameplay is enabled; item capture observes settled custody. Empty optional
phases must not stop required phases.

## Separation mechanisms

| Mechanism | Legitimate use | Reject |
| --- | --- | --- |
| Direct Cargo dependency | body execution using geometry; a domain adapter using prepared definitions | assuming an acyclic graph proves correct ownership |
| Bevy plugin or installer | owner installs private systems against documented phases | a wrapper around foreign algorithms to move a metric |
| Published `SystemSet` | ordering and visibility between independent owners | one public set per private function |
| Typed intra-tick message | explicit observation with known delivery | replacing a required synchronous result with next-tick delivery |
| Rollback state and effect journal | speculative state restored on rewind; confirmed effects released once per process | treating every message buffer as rollback history |
| App-local provider registry | independent providers registered and frozen before use | dynamic service discovery during a tick |
| Schema or metadata registry | validate IDs, schemas, revisions and conflicts | metadata-only registration as an executable extension |
| Backend-neutral registrar | a domain declares rewind state without importing GGRS | a backend-owned list of every domain type |
| Shared values or `SystemParam` | small owned values; borrow grouping for one operation | bags that carry siblings' private resources |

`ambition_registry_core` is a canonical-registration helper with explicit
New/Idempotent/Conflict behavior. It is not a service locator.

## Compile-time optionality

Measure a real independent consumer manifest under the intended feature set.
Cargo features unify across paths, so an opt-out on one edge does not remove
another edge's request. Check normal, build, dev and target closures
separately. Do not infer binary bytes from graph counts.

**The graph the suite compiles is not the graph the game ships.** `relativity`
is out of `all_capabilities`, and `cargo tree -e normal -p ambition_app` has no
`ambition_relativity2d`. A `--workspace` invocation unifies features and links
it, because `ambition_demo_twintrack` asks for it. So the suite compiles and
exercises a capability that the shipped game omits, and a defect that appears
only when the capability is absent is invisible to a workspace lane.
`the-featureless-facade-links-none-of-these` walks a per-package
feature-resolved tree and measures the right graph.

Start with a small profile set: headless body and world; windowed body and
world; headless combat; collection without held use; generic encounters without
named bosses. Each profile instantiates a real domain object and advances
behavior.

## Rollback and external effects

A domain declares its authoritative rewind state; the backend implements the
registrar. Register an optional domain only when its profile includes it.
Loading or unloading arbitrary rewind-owning native plugins at runtime is out of
scope. Preserve wire IDs and encoding during an ownership-only move; the
same-build policy stands.

`crates/ambition_platformer2d_runtime/src/external_effects.rs` is a bounded
confirmed-effects journal: replacement on resimulation, empty-frame
replacement, session reset and delivery after confirmation. Exactly-once
in-process release is not durable exactly-once delivery at a disk or network
sink.

## Ruleset and session scope

Games choose policy and capabilities; sessions own active lifetime; hosts
choose platform and backends. Restore the prior process policy when a scoped
ruleset leaves. A demo plugin never becomes the permanent owner of a setting
another experience uses. Re-entry, not only first startup, is a profile test.

## Completion evidence

Every claimed optional capability needs explicit prerequisites, a minimal
positive case, a supported absence case, correct re-entry and retirement,
rollback declaration and a resolved closure statement. Cross-owner composition
has a reason and a phase guarantee. Unsupported configurations report
unsupported; they do not borrow a plausible sibling default.
