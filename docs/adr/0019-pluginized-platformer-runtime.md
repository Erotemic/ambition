# ADR 0019: Pluginized platformer runtime via same-crate proto-boundaries

## Status

Accepted.

## Context

`ambition_sandbox` contains reusable platformer behavior, Ambition-specific
content, presentation adapters, authoring adapters, and app assembly. That made
feature work fast early on, but the central app plugin file and room lifecycle
conventions are now carrying too much architectural meaning implicitly.

The plugin refactor plan in `docs/planning/plugin_refactor/` proposes a staged
move toward reusable platformer runtime modules, optional mechanics plugins,
adapter/render/authoring plugins, and Ambition content plugins. The immediate
risk is doing a large crate split before the dependency direction is visible and
testable.

## Decision

Use same-crate proto-boundaries before extracting real crates.

- `crate::platformer_runtime` is the canonical home for reusable runtime
vocabulary that should eventually leave the sandbox crate.
- Lifecycle markers and spawn helpers move there first because room-scoped
lifetime bugs have already happened and because spawn policy is independent of
Ambition content.
- Schedule vocabulary may exist in `platformer_runtime` even while `SandboxSet`
remains the concrete app schedule.
- Subsystems should own their own Bevy plugin registration. The app composes
plugins; it should not accumulate detailed system lists for every mechanic.
- Architecture boundary tests should be simple and checked in early. They may be
grep-style tests while the boundary is still being carved.

## Consequences

Positive:

- Runtime code can be reviewed as if it were already a future crate.
- Spawn call sites declare lifecycle policy with verbs such as
`spawn_room_scoped` instead of remembering marker components manually.
- `app/plugins.rs` can shrink over time as subsystems gain module-owned plugins.
- Future crate extraction becomes mostly a dependency-boundary exercise instead
of a behavioral rewrite.

Tradeoffs:

- Some compatibility re-exports remain temporarily, especially for
`RoomScopedEntity` paths used by presentation and reset code.
- Crude architecture tests can produce false positives and need explicit
allowlist maintenance.
- `SandboxSet` and `PlatformerRuntimeSet` coexist until the concrete app schedule
can be mapped cleanly onto reusable runtime phases.

## Validation

```bash
cargo test -p ambition_app --test architecture_boundaries
cargo test -p ambition_sandbox --lib room_scoped
cargo test -p ambition_app --test portal_lab_usable
cargo test -p ambition_app --test gravity_room_reachability
```

## Current implications for agents

- The crate extraction this ADR set up has LARGELY HAPPENED (Stage 20, 2026-06).
  The workspace is now `ambition_engine_core` / `ambition_platformer_primitives` /
  `ambition_portal` / `ambition_time` / `ambition_input` / `ambition_menu` /
  `ambition_audio` (foundations) ← `ambition_sandbox` (machinery lib) ←
  `ambition_content` (named game content) ← `ambition_app` (assembly + bins +
  tests). See `docs/planning/plugin_refactor/22_monolith_breaker_survey.md`.
- `crate::engine_core`, `crate::kinematic`, `crate::input`, `crate::time`,
  `crate::portal` inside `ambition_sandbox` are FACADE re-exports of those crates —
  edit the crate, not a (nonexistent) lib module.
- Machinery must not import content: the `architecture_boundaries` guards (in
  `ambition_app/tests`) enforce it. Add a guard when you win a new boundary.
- New gameplay subsystems should be self-owning `Plugin`s (components-as-plugins),
  not functions hand-wired in the app assembly.
- Integration tests + binaries live in `ambition_app`; machinery unit tests in
  `ambition_sandbox --lib`; content tests in `ambition_content --all-features`.
