# Gameplay trace recorder

The gameplay trace recorder captures recent control frames, simulation messages, player snapshots, and diagnostic context so movement/combat bugs can be replayed instead of guessed at.

## Current paths

```text
crates/ambition_sandbox/src/dev/trace.rs
crates/ambition_sandbox/src/dev/trace/
  buffer.rs
  detect.rs
  dump.rs
  model.rs
  systems.rs
  tests.rs
crates/ambition_sandbox/src/bin/trace_replay.rs
```

The crate root re-exports `dev::trace` as `ambition_sandbox::trace` for compatibility. Prefer the `src/dev/trace/` path in new docs.

## When to use it

Use traces for bugs involving:

- unexplained collision correction,
- wall cling / ledge / blink edge cases,
- loading-zone or transition placement,
- out-of-bounds movement,
- attack/projectile timing that depends on a sequence of control frames.

## Rules

- Keep trace records compact enough to attach to bug reports.
- Record enough geometry and player-state context to distinguish a collision bug from an input bug.
- Add a regression test when a trace exposes a durable failure mode.
- Do not fix trace-visible teleports by only widening OOB margins; explain the collision correction.

## Validation anchors

```bash
cargo test -p ambition_sandbox trace
cargo run -p ambition_sandbox --bin trace_replay -- --help
```

Related docs: `docs/planning/tech-debt-log.md`, `dev/journals/lessons_learned.md`.
