# ADR 0013: Compile-time hygiene as a project constraint

## Status

Accepted.

## Context

The app/gameplay clean test build already takes ~10 minutes on the dev VM
(Bevy + bevy_ecs_ldtk + bevy-inspector-egui + avian2d + the wider Bevy
ecosystem). Compile time is invisible in any single PR but lethal
cumulatively. Every iteration cycle (`cargo check`, `cargo test`,
`cargo run`, ADR validation, headless smoke runs) waits on the toolchain.

As Ambition grows toward a full Metroidvania (more rooms, more entity
types, more abilities, more features), choices that look small in
isolation can compound into multi-minute incremental rebuilds. The
project's stated direction explicitly favors reusing professional
ecosystem crates (per the use-existing-packages feedback memory and the
project's existing dependency list). Each adopted crate is also a
compile-time tax. The discipline should be: adopt for the right reasons,
keep the tax bounded.

This ADR codifies the practices the project commits to. It is not a
list of micro-optimizations; it is the structural discipline that lets
the project keep moving as it grows.

## Decision

Compile time is a first-class project constraint. New code, new
dependencies, and new architecture choices weigh compile cost alongside
features. The following practices apply.

### Cargo profile and linker setup

Adopt the standard Bevy compile-time toolkit through workspace profiles and
explicit feature gates. There is no current `dev` Cargo feature; the default
desktop development persona is `desktop_dev`, with focused opt-ins such as
`dev_hot_reload` and `profile` when needed. Keep those feature names distinct
from Cargo's `[profile.dev]`.

```toml
# Repo-root Cargo.toml (workspace)
[profile.dev]
debug = 0
strip = "debuginfo"
opt-level = 0
overflow-checks = true   # leave enabled until profiling justifies disabling

[profile.dev.package."*"]
opt-level = 2            # optimized deps; unoptimized own code

[profile.release]
opt-level = 3
panic = "abort"
debug = 0
strip = "debuginfo"
lto = "thin"

[profile.distribution]
inherits = "release"
strip = true
lto = "thin"
codegen-units = 1
```

Use a faster linker (`lld` or `mold`) for development. Configure via
`.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

`mold` is faster than `lld` on Linux when available and is the
recommendation for VMs/dev workstations. `lld` is the cross-platform
default.

### Bevy dynamic linking and hot reload

Do not document a generic `dev` feature unless one is added and validated.
Use the default desktop development build for normal iteration, and opt into
specific tooling features by name when required:

```bash
cargo run -p ambition_app --bin ambition_game_bin                         # default desktop_dev persona
cargo run -p ambition_app --bin ambition_game_bin --features dev_hot_reload
cargo run -p ambition_app --bin ambition_game_bin --release                # final desktop build
```

Dynamic linking may be revisited as a dedicated, validated feature later, but it
is not part of the current feature graph.

### Workspace structure

The earlier monolith has since been split into focused crates. Reusable engine
semantics live in `crates/ambition_engine_core/`; content-free gameplay systems
and runtime state live in `crates/ambition_gameplay_core/`; presentation lives
in `crates/ambition_render/`; named authored content lives in
`crates/ambition_content/`; and runnable binaries/app assembly live in
`crates/ambition_app/`. Keep new dependencies at the lowest layer that actually
needs them.

### Macro and generics discipline

- **`#[derive(Reflect)]`** generates substantial type-registration code.
  Apply only to types that the inspector (`bevy-inspector-egui`) actually
  reaches. Don't blanket-derive Reflect on hot-path components.
- **Prefer `dyn Trait` over generics for cold-path polymorphism.**
  Generics monomorphize per concrete type; `dyn` doesn't. Hot-loop code
  can stay generic; per-frame event dispatch and plugin registration
  usually don't need it.
- **Be selective about new proc-macro-heavy crates.** Each adopts a
  compile-time tax that's paid even on unrelated edits. Existing ones
  (Bevy, serde, leafwing) are paid for; adding new ones should justify
  their cost.

### Crate-root churn

Editing `lib.rs` invalidates every dependent crate's incremental cache.
Keep frequently-edited code in submodules. The lib root should be
declarations and re-exports, rarely-touched.

### Test runner

Adopt `cargo nextest` for parallel test execution on multi-core
machines. It is significantly faster than `cargo test` and integrates
cleanly with CI.

```bash
cargo nextest run -p ambition_gameplay_core
```

### Periodic audits

Run `cargo build --timings` quarterly (or after any large dependency
addition / refactor) to identify compile-time hotspots. The output is
an HTML report showing per-crate compile durations; regressions are
visible immediately. Keep timing outputs with the optimization-report workflow under `tools/optimization_report/` or attach them to the relevant performance note. Do not name a standalone docs directory for audits unless it is created and maintained.

### Compile-time log level capping

Add to the workspace `Cargo.toml`:

```toml
log = { version = "0.4", features = ["max_level_debug", "release_max_level_warn"] }
```

Prevents expensive trace/debug logging from running in production
builds while keeping development logs available. Distribution builds
can further disable logging via `release_max_level_off`.

### When evaluating a new crate

Compile cost is a first-class evaluation criterion alongside features
and maintenance:

1. Fetch and try the crate in a side branch.
2. Run `cargo build --timings` before and after adding it.
3. If the crate dominates compile time, look for alternatives or
   evaluate whether the feature justifies the cost.
4. Document compile-cost considerations in any adoption ADR.

## Consequences

- Workspace `Cargo.toml` owns the development/release profile blocks. CI uses the
  default profile or `--release` as appropriate; nothing changes in
  ship behavior.
- Existing code stays as-is until the next refactor pass. New code
  follows the discipline; old code migrates opportunistically.
- The "use existing packages" principle (use-existing-packages memory)
  is balanced by compile-time review, not contradicted. Adopt
  `bevy_dev_tools` (ADR 0014) and other ecosystem crates with eyes
  open.
- `cargo --timings` output is captured through the optimization-report workflow or attached to focused performance notes so historical evidence stays discoverable without stale empty directories.

## Initial implementation target

Conservative, sequenced:

1. Add the workspace `[profile.*]` blocks (no behavior change for
   default builds; faster dev/distribution profiles available).
2. Add a `.cargo/config.toml` with `lld` (or `mold` if installed) as
   the linker. Document `mold` install in the contributor README.
3. Keep development-only tooling behind explicit features such as
   `dev_hot_reload`, `dev_tools`, or `profile`; do not add them silently to
   release/distribution commands.
4. Add `cargo nextest` to the recommended test commands.
5. Run a baseline `cargo build --timings` and record the output through `tools/optimization_report/` or a focused performance note.

## Non-goals for the first implementation

- Migrating existing `derive(Reflect)` annotations. Audit and prune as
  inspector usage stabilizes; no urgent changes.
- Replacing Bevy or any current dependency for compile-time reasons.
  Bevy is a load-bearing choice; the goal is to use it efficiently,
  not to swap.
- Cranelift codegen backend. Nightly-only and adds toolchain
  variability; revisit if compile time becomes a blocker.
- Adopting a build-cache service (`sccache`, `bazel-remote`). Single
  developer + small CI; the marginal value isn't there yet.

## Review notes

- Cross-references: ADR 0014 (bevy_dev_tools adoption — itself a
  compile-time addition that follows this discipline). The
  `feedback_compile_time` memory note holds operational guidance.
- Compile-time regressions should be flagged in PRs the way
  test-time regressions are. Reviewers should ask "what does this do
  to incremental rebuild time?" for any large dependency or macro
  addition.

## Sources

- [Setup — Bevy Quick Start](https://bevy.org/learn/quick-start/getting-started/setup/)
- [bevy_best_practices](https://github.com/tbillington/bevy_best_practices) — Tom Billington's opinionated Bevy practices guide (compile-time profile blocks, dev feature pattern, log level capping)
- [Bevy compile time discussion](https://github.com/bevyengine/bevy/discussions/9146)
- [Bevy binary size + compile time data gathering](https://github.com/bevyengine/bevy/discussions/14864)

## Current implications for agents

- Prefer feature gates and crate boundaries that preserve quick checks.
- Avoid adding heavy dependencies to default builds without an explicit reason.
- Use focused validation before broad workspace builds.
