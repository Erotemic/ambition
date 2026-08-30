# Project build, test iteration and distribution

**State:** OPEN — developer iteration has current measured pressure; external
project/release packaging remains later/product-driven.

## Goal

Make the supported path from checkout to tested/shippable game explicit and
resource-aware:

```text
clone/create
 -> configure providers/capabilities
 -> prepare/generate/validate content
 -> edit/build
 -> run targeted tests
 -> run pre-push/supported-composition gates
 -> package
 -> distribute/update
```

Build/test iteration is an engine productivity concern independently of runtime
frame performance.

## Current empirical lessons

### Dev profile choices should be measured, not inherited

Recent probes found several `opt-level = 0` development exceptions bought only
about **1–2%** on representative one-file rebuilds while the measured runtime
changed from about **5.12 ms to 2.96 ms** when those dependencies returned to
`opt-level = 1`.

That does not establish one universal profile for every crate. It does establish
that large runtime/debug penalties need a measured rebuild payoff.

### Optimized incremental builds are not currently a default solution

The repository has seen invalid/corrupt link behavior in the affected optimized
incremental workflow. Launch tooling disables that path. Do not re-enable it as a
speed tweak without a reproducible correctness test on the actual build path.

### Test resource shape matters

A large app integration suite can exhaust machine memory at default test
concurrency while passing with a bounded thread count. The correct response is a
resource-aware lane/preset, not treating parallelism as always beneficial.

### Feature combinations need explicit proof

The first broad combination sweeps found real compile/configuration gaps that
single default builds could not expose. Supported product/capability combinations
should have a bounded matrix rather than relying on accidental workspace feature
unification.

### Clean checkout/generated assets are part of the contract

A test/build that succeeds only because an ignored/generated artifact is already
present locally is not a reproducible project path. Generated-art freshness and
source-to-output cache keys are distinct concerns; output digests cannot detect a
correctly cached output whose source dependency was omitted from the key.

## Current program areas

### B1 — development profile policy

Keep a small measured table for dependencies/crates whose dev optimization level
materially affects runtime/tooling. Change an override only with both:

- representative edit/rebuild cost;
- representative runtime/tool cost.

Avoid profile folklore copied from an old bottleneck.

### B2 — test lanes and concurrency

Maintain explicit tiers:

- touched-crate/narrow tests while editing;
- product integration tests for changed cross-crate behavior;
- pre-push workspace/library or policy gates where appropriate;
- resource-bounded presets for large monolithic test binaries.

Do not make every turn run the whole workspace merely because it is comprehensive.

### B3 — supported feature/product matrix

Define the combinations the repository claims to support—headless/rendered,
rollback/nonrollback where applicable, relevant capability subsets, key platform
personas—and compile/test those combinations deliberately.

A broad matrix is useful only if it maps to real products/hosts. Do not enumerate
the power set of Cargo features.

### B4 — generated-content/bootstrap contract

A clean checkout should have an explicit path to produce every required generated
artifact or obtain it from the intended cache/submodule. Cache keys must include
all source dependencies that affect output.

### B5 — platform prerequisites

Desktop remains the primary local path. Android/web/cross targets should use
repository-owned prerequisite/setup scripts and clearly distinguish:

- code failure;
- unsupported target;
- missing external toolchain/prerequisite.

Do not report an absent NDK/GPU/display as proof that the target's code is broken.

### B6 — packaging/distribution

Keep packaging/release work product-driven. The eventual external-project layout,
SDK templates, update mechanism and broad release-target policy should follow the
public SDK and real distribution customers.

## Candidate tool shape

Build orchestration belongs primarily in repository/tooling surfaces rather than
a giant runtime project-manager service. Runtime package/asset manifests should
remain narrow data contracts consumed by engine domains.

Tools should expose noninteractive, inspectable commands suitable for humans and
agents: plan/check/build/test/package with clear artifact/cache ownership.

## Acceptance

- a fresh checkout can follow a documented bootstrap/build/test path without
  relying on accidental local ignored files;
- representative edit/rebuild timing is known for any nondefault dev-profile
  exception retained for speed;
- large test suites have a bounded-memory invocation that is part of normal
  workflow;
- supported capability/platform combinations compile in deliberate gates;
- a platform prerequisite failure is distinguishable from a source/build defect;
- packaging work does not introduce a second runtime composition model.

## Open design questions — deliberately unresolved

- eventual external project layout and template format;
- which generated artifacts should be checked in versus produced/fetched;
- web as first-class release target versus later experiment;
- third-party capability/plugin version locking;
- required asset hot-reload guarantees for agent iteration;
- final split between Cargo features and runtime/provider configuration.
