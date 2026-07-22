# `ambition_test_support` — sequester harness boilerplate and make behavioral tests cheap

> **State:** TRIAGE — STRONG CANDIDATE, DESIGN DECISIONS PENDING, 2026-07-22.
>
> The need is clear: Ambition's tests repeatedly rebuild Bevy apps, schedules,
> fixed time, session roots, room state, catalogs, and command-flush sequences.
> A dedicated dev-only support crate is likely worthwhile. The exact dependency
> boundary and fixture API must be piloted before promotion.

## Problem

Behavioral tests are the repository's main architectural proof, but writing one
often requires a large amount of setup unrelated to the behavior under test.
A repository survey found hundreds of `App::new()` sites and dozens of repeated
patterns involving `MinimalPlugins`, session-world insertion, fixed-time setup,
room fixtures, catalog construction, and manual schedule stepping.

The consequences are larger than line count:

- important tests are expensive to write, so agents test a nearby emitter or
  bookkeeping value instead of the actual consumer behavior;
- nominally similar tests initialize different plugin sets and schedules;
- command flushing and fixed-tick progression are easy to get subtly wrong;
- every crate invents its own fixture vocabulary;
- large setup blocks hide the assertion and discourage adversarial variants;
- copied fixtures drift as the real engine lifecycle changes;
- tests overfit internal setup details because no supported harness exists.

A support crate should make the **real behavioral path** easier than a fake local
approximation.

## Direction under consideration

Create a dev-only workspace crate tentatively named:

```text
crates/ambition_test_support
```

Its purpose is to provide explicit, composable test harness primitives. It
should sequester repeated setup without hiding which plugins, schedules, worlds,
or policies a test uses.

The crate is not yet approved because dependency cycles are a real risk. A test
support crate used by a low-level domain crate cannot depend back on that same
domain crate. The first design task is therefore to establish a layered boundary
rather than moving every fixture into one place.

## Desired outcomes

A good support layer should make these tests short and trustworthy:

- construct a headless app with a named, visible plugin profile;
- install deterministic fixed time and step exactly N simulation ticks;
- create a session world and obtain its root safely;
- stage a minimal room with explicit geometry and ownership;
- register a minimal character or content fixture without copying a full catalog;
- flush deferred commands at the same boundaries production uses;
- send an input, request, or effect and observe the actual downstream consumer;
- capture authoritative roster, messages, traces, or snapshots;
- assert that a failed transaction left the old state unchanged;
- run the same scenario under visible and headless hosts where appropriate.

The assertion should dominate the test body. Setup should read as a small
scenario, not an ad hoc application bootstrap.

## Proposed layers

The exact modules are pending, but the dependency direction should resemble the
following.

### Layer 1 — generic Bevy harness

Safe for broad dev-dependency use and dependent only on Bevy plus very low-level
Ambition primitives, if any:

- `HeadlessTestAppBuilder` or an equivalent explicit builder;
- named plugin profiles rather than an opaque “install everything” helper;
- deterministic fixed-time configuration;
- schedule stepping and command-flush helpers;
- message/event capture utilities;
- world query and entity-count assertions;
- panic/error capture where the production boundary exposes it.

A test must be able to see which profile it selected. Avoid one magical default
app whose installed systems are discovered only by reading the helper.

### Layer 2 — engine/session fixtures

Fixtures for stable low-level engine concepts that do not create dependency
cycles:

- session root and session-world setup;
- participant/input setup;
- minimal clock and simulation-host policy;
- deterministic IDs or test namespaces;
- room ownership and lifecycle markers once those are stable;
- snapshot/rollback harness helpers owned at the appropriate layer.

### Layer 3 — domain adapters

Actor-, platformer-, combat-, and game-specific fixtures may not all belong in
the central crate. Options include:

- local `tests/support` modules that build on `ambition_test_support`;
- leaf-level companion support crates;
- support modules exposed only under `cfg(test)` or a dedicated testing feature;
- integration-test crates that are allowed to depend on the complete engine.

The design must not create Cargo dependency cycles merely to centralize fixtures.
“Everything in one crate” is not a goal.

## Candidate API shape

The API should describe intent and keep setup visible. For example:

```rust
let mut sim = TestSim::builder()
    .profile(TestProfile::PlatformerHeadless)
    .fixed_hz(60.0)
    .with_session(SessionFixture::single_player())
    .with_room(RoomFixture::flat("test-room"))
    .build();

sim.send(PlayerAction::Jump);
sim.step_ticks(3);
sim.assert_one::<JumpStarted>();
```

This is illustrative, not an approved API. The real design may use ordinary
functions and extension traits rather than one builder. The important properties
are:

- setup choices are visible;
- stepping semantics are exact;
- production schedules and consumers are used;
- fixture defaults are documented and overrideable;
- failure messages explain the scenario;
- helpers compose without requiring a monolithic harness object.

## Fixture laws

Shared fixtures need stricter rules than ordinary test code because hundreds of
tests may trust them.

1. **No hidden production behavior.** A fixture may install production plugins;
   it should not quietly replace the behavior being tested with a fake.
2. **No incidental completeness.** “Minimal” fixtures include only named
   requirements. Tests should not pass because an oversized fixture happened to
   install an unrelated consumer.
3. **Deterministic by default.** Time, IDs, provider order, and random sources are
   controlled or explicitly supplied.
4. **Behavior before source shape.** Helpers assert world behavior, messages,
   traces, or snapshots rather than file names and symbol spelling.
5. **Explicit command boundaries.** `update`, fixed schedules, and deferred
   command application must not be conflated.
6. **Poisonable seams.** It should be easy to omit or replace a consumer so a
   test can prove that its assertion discriminates.
7. **Useful diagnostics.** Roster, schedule, message, and lifecycle assertions
   should report the unexpected state, not only `left != right`.
8. **No global mutable fixture state.** Tests remain parallel-safe unless a test
   explicitly requests serialization.
9. **No game-name branches in generic support.** Game-specific scenarios remain
   with the game or an integration support layer.
10. **Source remains inspectable.** An unfamiliar maintainer or coding agent can
    determine what a fixture installs without executing it.

## High-value initial helpers

The pilot should focus on repeated, low-risk setup rather than trying to model
the entire engine.

### Deterministic app profiles

Named profiles such as:

- bare Bevy ECS;
- minimal headless simulation;
- runtime/snapshot host;
- platformer engine without presentation;
- full headless acceptance host.

Profiles must document their plugin and schedule ownership. They should be
assembled from ordinary public plugin groups rather than duplicating production
registration.

### Time and stepping

One canonical vocabulary for:

- advancing a render/update frame;
- advancing a fixed simulation tick;
- running startup once;
- applying deferred commands;
- stepping until a bounded condition;
- recording exactly how many ticks elapsed.

This is likely the highest correctness payoff because many false-positive tests
come from observing the wrong schedule boundary.

### Session and room setup

Safe helpers for:

- inserting or locating a session world;
- creating a minimal room ownership scope;
- installing active room geometry;
- spawning an explicitly authoritative or presentation-only test entity;
- capturing the pre-transaction baseline.

These should evolve with the transactional-construction campaign rather than
freeze legacy room setup as the test API.

### Observation and assertions

Reusable diagnostics for:

- message/event counts and payloads;
- authoritative `SimId` multiplicity;
- entity generation replacement;
- component presence and exact relation endpoints;
- snapshot round-trip equality;
- no-mutation-on-failure;
- deterministic canonical dumps;
- schedule or consumer invocation counts where behavior requires them.

## What should remain local

Do not centralize fixtures simply because two tests share text. Keep these with
the owner unless a stable cross-domain contract emerges:

- boss-specific attack scripts;
- Mary-O or Sanic level authoring;
- one-off character catalogs;
- exact UI layouts and asset handles;
- private implementation details of a single system;
- large golden worlds that are acceptance content rather than harness machinery.

The central support crate should make local fixtures easy to write, not become a
warehouse of every test object.

## Dependency-cycle constraints

Before creating the crate, draw the workspace dependency layers and answer:

- Which production crates may dev-depend on `ambition_test_support`?
- Which production crates may the support crate depend on without cycles?
- Which actor/platformer helpers must remain local or move to a leaf integration
  crate?
- Can a small Bevy-only core serve low-level crates while a higher-level support
  crate serves apps and integration tests?
- Would optional features hide cycles or merely make the dependency graph harder
  to understand?

Prefer two clear layers over one feature-heavy crate if that is what the graph
requires. The name `ambition_test_support` can still refer to the common base;
more specific support should have equally explicit names.

## Pilot migration

Choose three existing test clusters with different needs:

1. a low-level ECS/unit cluster dominated by `App::new()` and schedule stepping;
2. a session/room cluster using repeated session-world and fixed-time setup;
3. a headless acceptance or reconstruction cluster that needs messages,
   authoritative roster checks, and deferred command boundaries.

For each cluster:

- preserve the behavioral assertion;
- shrink setup substantially;
- keep installed production behavior visible;
- add or retain one discriminating poison variant;
- compare failure diagnostics before and after;
- record any helper that was too domain-specific and move it back local.

Do not perform a workspace-wide mechanical rewrite in the pilot.

## Acceptance criteria for promotion

Promote this into [`../tracks.md`](../tracks.md) only when the pilot can name:

- the dependency-safe base layer;
- three test clusters to migrate;
- the exact app profiles and stepping semantics provided;
- how fixtures expose rather than hide installed behavior;
- a policy for domain-specific fixture adapters;
- tests for the support code itself;
- a deletion target for copied setup in the pilot clusters;
- an explicit statement that production crates do not depend on the test crate.

Success is not merely fewer lines. The pilot should demonstrate that a new
behavioral test is easier to write, harder to make vacuous, and produces better
failure output.

## Relationship to other proposed foundation work

`ambition_test_support` is independent of `ambition_registry_core`. Tests may use
registry-core diagnostics later, but neither crate should become a route for
unrelated helpers.

Stable identifier centralization is also a separate pending decision. Test IDs
may use explicit local wrappers until that design is resolved.

No general `ambition_utils` crate is proposed. Shared code should enter a
foundation crate only when it has one coherent owner and a testable invariant.
