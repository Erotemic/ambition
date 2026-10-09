# Project build, test iteration and distribution

**State:** OPEN. Developer iteration has measured pressure now. External
project and release packaging are later and product-driven.

## Goal

Make the supported path from checkout to tested, shippable game explicit and
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

Build and test iteration is an engine productivity concern, separate from
runtime frame performance. An external project needs a reliable noninteractive
path from clean checkout to validated target artifact. A graphical export
dialog is not required. See [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

## Current empirical lessons

- **Measure dev profile choices.** Several `opt-level = 0` development
  exceptions bought about 1-2% on one-file rebuilds while the measured runtime
  went from about 5.12 ms to 2.96 ms when those dependencies returned to
  `opt-level = 1`. A large runtime penalty needs a measured rebuild payoff.
- **Optimized incremental builds are disabled.** The affected workflow has
  produced invalid links. Do not re-enable it as a speed tweak without a
  reproducible correctness test on the actual build path.
- **Test resource shape matters.** A large app integration suite can exhaust
  memory at default concurrency and pass with bounded threads. Use a
  resource-aware lane or preset.
- **Feature combinations need explicit proof.** Combination sweeps found real
  compile/configuration gaps that default builds hide.
- **Clean checkout and generated assets are part of the contract.** A build that
  succeeds only because an ignored artifact exists locally is not reproducible.
  Output digests cannot detect a cached output whose source dependency was left
  out of the cache key.
- **Web.** `scripts/run_tests.py` plans the wasm `cargo check`
  (`--target wasm32-unknown-unknown --features web_served_assets`, about 40 s
  warm) in the default plan, and the release wasm link (about 18 min cold) and
  the native "web persona boots" job only in the exhaustive plan. All three are
  green. The pre-`wasm-bindgen` artifact is not a shipped size; the
  bindgen'd, size-optimized artifact is not measured. The persona job runs
  natively, so the wasm-only path is unexercised
  ([`checks-that-did-not-run.md`](../../recipes/checks-that-did-not-run.md)).
- **Disk.** `scripts/check_disk_headroom.py` refuses to start a suite below
  `MIN_FREE_GB` (40), and `run_tests.py` refuses to start the next job below
  `ABORT_FREE_GB` (6), so a full disk names itself instead of dying as a link
  error. `target/` is bind-mounted; `df` on the working directory can read the
  wrong filesystem, so read `scripts/setup/target_bindmount.sh --status`. Every
  feature job builds its own graph variant, so carves and wide plans cost disk.

## Current program areas

### B1 — development profile policy

Keep a small measured table for crates whose dev optimization level materially
affects runtime or tooling. Change an override only with both representative
edit/rebuild cost and representative runtime/tool cost.

### B2 — test lanes and concurrency

Maintain explicit tiers:

- touched-crate/narrow tests while editing;
- product integration tests for changed cross-crate behavior;
- pre-push workspace/library or policy gates where appropriate;
- resource-bounded presets for large monolithic test binaries.

Do not make every turn run the whole workspace.

### B3 — supported feature/product matrix

Define the combinations the repository supports (headless/rendered,
rollback/nonrollback, capability subsets, platform personas) and compile/test
them deliberately. Do not enumerate the power set of Cargo features.

"The smash tests" are three programs: per-crate default features (no device
layer), the gate's `nextest --workspace` feature union (seats read devices), and
`-p ambition_demo_smash_app --features visible` alone. The third cell is
**unsupported**: `build_demo_app` has no renderer, and `visible` adds
presentation systems whose parameters are render-stack resources. The crate's
feature docs say so and name the supported paths (the workspace union and
`build_windowed_demo_app`).

Rules from that cell:

- `PlatformerAssetsPlugin` registers the `Assets<TextureAtlasLayout>` its own
  `Startup` system consumes, guarded on the resource, because `init_asset` is
  not idempotent.
- A presentation system with a render-stack parameter carries
  `run_if(resource_exists::<..>)` (see
  [`headless-verification.md`](headless-verification.md)).

### B4 — generated-content/bootstrap contract

A clean checkout has an explicit path to produce every required generated
artifact or obtain it from the intended cache or submodule. Cache keys include
every source dependency that affects output.

**A fresh clone on a used machine is not a fresh machine.** Prerequisites split
in two:

| Prerequisite | Scope |
| --- | --- |
| six tool venvs (`~/.cache/ambition-tool-venvs/<basename>`) | machine |
| sampled instrument libraries (`/data/audio-tools`) | machine |
| `scripts/` venv | checkout (keyed by repo-root basename) |
| submodule contents | checkout |
| bundled UI fonts (git-ignored) | checkout |
| generated sprite sheets (git-ignored) | checkout |
| `target/` | checkout (bind store keyed by basename + path hash) |

A clone on a used machine inherits the machine half and lacks the checkout half,
so `python_tools.sh --verify` looks nearly green there and fails everything on a
new laptop. Anyone testing the fresh-clone contract says which half they test.

Current shape:

- `scripts/setup/generated_content.sh` runs `scripts/grab_font_assets.py`. The
  `ambition_render` typography test `include_bytes!`s the bundled faces, as the
  app's `embed_core_assets!` does, so missing fonts fail `cargo check
  --all-targets`. Keep the test mirroring the app's path.
- `python_tools.sh` keeps one module list (`scripts_env_modules`) read by both
  install and `--verify` (`verify_scripts_environment`). When the suite gains a
  module-scope import under `scripts/`, re-run the import sweep recorded beside
  that list.
- Sampled instrument libraries install by default. A preflight gate in
  `ambition_music_renderer` resolves every library reference a cue names and
  refuses on any unresolvable one, ignoring `optional:` on purpose (every
  shipped sfz backend is optional, so honouring it would make the gate dead).
- `./run_headless.sh --ticks 600` builds in release and runs the sim headless.
  All four entry points (`run_developer_setup.sh`, `run_headless.sh`,
  `run_tests.sh`, `scripts/setup/audio_libraries.sh`) run from a bare clone, and
  the orchestrator's `--help` names the status/verify commands.

A per-component health check answers only for the components it enumerates.
When you add a check here, ask what it cannot see.

Open:

| Item | Detail |
| --- | --- |
| Venv key collision | `ambition_tool_venv_dir` keys on `basename` only, so two checkouts with one directory name share a `scripts/` venv. `target_bindmount.sh`'s `store_for` (basename + path hash) is the fix to copy |
| `.ipfs` sidecars | Six git-ignored payload directories restore only by manual `ipfs get`. On a fresh clone, `package_asset_guard.py compose` fails on exactly one family: `data/vanity_card.ron`'s nine frames (`assets/vanity_card.ipfs`). Backlog only: Jon owns asset distribution. Do not fold into feature work. This is not the tracked `vanity_card_made_this_meme` that `scripts/regen/sprites.sh` builds |

### B5 — platform prerequisites

Desktop is the primary local path. Android, web and cross targets use
repository-owned prerequisite scripts and distinguish code failure, unsupported
target, and missing external toolchain or prerequisite. Do not report an absent
NDK, GPU or display as broken code.

- **Web:** with `wasm32-unknown-unknown` absent, the gate plans no web job and
  says the web build is unchecked.
- **GPU/display:** a CPU adapter seeds `Potato` and says so;
  `[census] phases_trust` reports `trustworthy=no_render_backend`.
- **Fonts:** a missing font is still reported as a read error from the
  typography test target, not as a named prerequisite. Fail with the
  prerequisite named and the fetch command quoted, as the web job does. The
  app's embedding is behind `static_core_assets` (not default), so only
  `--all-targets` meets it.
- **Disk:** when a lane fails incoherently, check the target bind's volume
  before believing the failure.

### B6 — packaging/distribution

Keep packaging/release work product-driven, but Engine 1.0 needs at least one
complete external-project path rather than leaving packaging indefinitely
abstract. The eventual external-project layout, SDK templates, update mechanism
and broad release-target policy should follow the public SDK and real distribution
customers.

Minimum competitive proof:

- one clean external/minimal project can select capabilities/providers without
  workspace-private wiring;
- preparation/build/test/package are noninteractive and scriptable;
- at least one desktop release artifact is reproducible from documented inputs;
- web/Android/headless profiles state their prerequisites and failure modes;
- target packaging uses the same logical asset/content identities as development
  rather than a target-specific shadow application;
- CI can distinguish source failure, missing prerequisite, unsupported profile and
  packaging failure.

One-click GUI export is not an acceptance requirement.

**One external desktop artifact (2026-10-09):** `python3
scripts/package_outlander.py` produces the release artifact of Outlander
(`fixtures/external_consumer`, the SDK game) and checks that it reads every
asset from inside itself. The exit code tells a missing prerequisite (2) from a
failed build (3) and a failed artifact (1). Linux only; the asset list is
measured, not authored (`--measure`).

### B7 - agent iteration budget

The [extension architecture](extension-model.md) defines three deliberate cost
classes: data artifact production/load, lightweight procedural module build/load,
and engine rebuild. M0-M3 in [extension evidence](extension-iteration-evidence.md)
define the concrete measurements and reporting schema. I1's pure boundary and
I2/I3's last-good artifact path proceed while baseline collection runs. Backend,
storage, linker/profile tuning and numeric speedup claims wait for their evidence.

Do not count a lightweight crate followed by a heavy host link as completion.
Trace actual host link invocations and correlate edits with admitted/observed
behavior. Retain this plan's established profile and resource policies until a
matched experiment justifies changing them.

Track the wall-clock and resource shape of the common agent loop: inspect/edit,
compile, targeted test, preparation/generation, representative run, and package
when required. Optimize the dominant measured step rather than applying generic
Cargo folklore.

The goal is not a universal fixed time budget across machines. The goal is enough
telemetry that an agent can choose a narrow fast path and know when a change has
accidentally expanded the iteration surface.

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
- packaging work does not introduce a second runtime composition model;
- a clean external/minimal consumer can produce at least one release artifact
  through supported noninteractive tooling;
- common agent edit/build/test/preparation loops have enough measurement to avoid
  optimizing the wrong phase.

## Open design questions — deliberately unresolved

- eventual external project layout and template format;
- which generated artifacts should be checked in versus produced/fetched;
- web as first-class release target versus later experiment;
- third-party capability/plugin version locking;
- target-specific asset transport/readiness requirements beyond the mechanical
  generation/reconstruction guarantee now fixed in the extension model;
- final split between Cargo features and runtime/provider configuration.

## Distinguish dependency closure from executable footprint

[Packet A9](actor-monolith-work-frontier.md) measures the public facade's
supported profiles. `scripts/check_facade_dependency_closure.py` owns the
mandatory internal dependency closure of the facade (normal, non-optional,
`path` dependencies) and the fact that `ambition_render` is outside it. Do not
copy the number here. Count other packages, not the facade itself; a number that
does not say which it counts cannot be quoted. See also
`scripts/measure_minimum_profile_parentage.py`. This is a lower bound, not
Cargo's final feature resolution: `default-features = false` on one edge does
not prove another path cannot enable the feature.

For each shipping profile record target, features, Cargo-resolved graph,
compile timings and cache state, linked artifact size and runtime startup. These
are separate metrics. Removing a manifest edge can improve rebuild fanout
without changing linked bytes.

The minimal external fixture exercises supported APIs and real simulation. Do
not use in-workspace defaults or test-support backdoors to make an external
profile look independent.

## B7 delivery detail: measure the whole changed path

M0 must attribute source generation, pure build/codegen/link, changed-section
preparation, candidate materialization, state restore and first observed behavior.
The no-host-relink claim is a structural assertion as well as a timing row: a fast
unnecessary host link is still the wrong content-edit path. Artifact freshness is
proved by the active generation and its changed behavior, not file mtime.

Do not put the new producer in the host build.rs or feed its result back through
include_str for the migrated path. Keep independent manifests/lockfiles and record
actual feature/target units. Reuse the configured cache safely; a new cold target
for every sample/edit measures repeated bootstrap rather than normal iteration.
Existing target-bind/disk/profile rules still apply. M0-M3 own samples and cost
choices; FI1-FI4/FI10 own [behavioral acceptance](fast-iteration-acceptance.md).
