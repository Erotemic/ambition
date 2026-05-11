# Lesson learned: moving tests out of a Rust module can strand attributes at EOF

## What happened

During the `rl` -> `rl_sim` rename/split, tests were moved out of the runtime
implementation file into a dedicated `rl_sim/tests.rs` child module. The split
left a bare `#[cfg(test)]` at the end of `rl_sim/runtime.rs`, so both `cargo fmt`
and `cargo test` failed to parse the module.

The facade file also retained several Bevy imports that belonged to the runtime
implementation. Those imports were harmless but noisy, and they were a useful
signal that the split had not fully assigned ownership of dependencies to child
modules.

## Invariant

When extracting Rust child modules, validate that every moved item takes its
attributes with it, and that every old parent/facade import still has a local
consumer after the move.

## Practical checklist

- Search for dangling attributes at the ends of edited files:
  `tail -n 20 <file>` is often enough.
- Search for dangling doc comments and attributes after automated extraction:
  `grep -RIn '^#\[cfg(test)\]$\|^///' <changed-dir>` and inspect suspicious
  end-of-file matches.
- Keep facade modules boring: docs, `mod` declarations, and re-exports only.
- Treat unused imports in a facade after a split as a smell that implementation
  code moved but dependency ownership did not.

## Better generation behavior

Before packaging an overlay that moves tests to a child module, the generator
should inspect the old extraction boundary and remove any orphaned `#[cfg(test)]`,
`#[test]`, `#[derive(...)]`, `#[allow(...)]`, or doc-comment fragments that no
longer document an item.
