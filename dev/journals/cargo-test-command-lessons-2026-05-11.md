# Lesson learned: validate Cargo command grammar before handoff

## Incident

A refactor overlay handoff recommended this command:

```bash
cargo test -p ambition_gameplay_core inventory pause_menu
```

Cargo rejected it because `cargo test` only accepts one positional test filter:

```text
error: unexpected argument 'pause_menu' found
Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]
```

This repeated an earlier mistake where a validation command attempted to pass multiple filters to `cargo test` in one invocation.

## Correct invariant

When handing off Rust validation commands, use exactly one positional `TESTNAME` filter per `cargo test` invocation. If multiple filters are desired, use multiple commands or run a broader target such as `--lib` for the package.

## Correct examples

```bash
cargo test -p ambition_gameplay_core --lib inventory
cargo test -p ambition_gameplay_core --lib pause_menu
```

or:

```bash
cargo test -p ambition_gameplay_core --lib
```

Keep integration regression checks explicit:

```bash
cargo test -p ambition_gameplay_core --test repro_walls
cargo test -p ambition_gameplay_core --test fuzz_random_walker
```

## Process adjustment

Before packaging a handoff, inspect each shell command for tool grammar, not just semantic intent. Validation commands should be copy-pasteable and should not rely on Cargo accepting multiple positional test filters.
