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
cargo test -p ambition_sandbox architecture_boundaries
cargo test -p ambition_sandbox room_scoped
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox gravity_room_reachability
```
