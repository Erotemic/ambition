# Benchmark candidate: cargo test accepts one positional test filter

## Context

During a Rust refactor handoff, the assistant recommended:

```bash
cargo test -p ambition_sandbox inventory pause_menu
```

Cargo rejected this with:

```text
error: unexpected argument 'pause_menu' found

Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]
```

The assistant had made the same mistake earlier with an engine command that tried to pass multiple test filters in one invocation.

## Benchmark question

You are preparing validation commands for a Rust workspace after splitting two modules, `inventory` and `pause_menu`, inside the package `ambition_sandbox`.

A proposed command is:

```bash
cargo test -p ambition_sandbox inventory pause_menu
```

Is this valid? If not, provide a corrected validation sequence that runs both targeted filters and preserves the broader integration regression checks.

## Expected answer

No. `cargo test` accepts at most one positional `TESTNAME` filter before `--`. Run separate invocations for separate filters, or run the whole relevant package/library test suite.

A corrected sequence is:

```bash
cargo fmt --all
cargo test -p ambition_sandbox --lib inventory
cargo test -p ambition_sandbox --lib pause_menu
cargo test -p ambition_sandbox --test repro_walls
cargo test -p ambition_sandbox --test fuzz_random_walker
```

A broader but simpler alternative is:

```bash
cargo fmt --all
cargo test -p ambition_sandbox --lib
cargo test -p ambition_sandbox --test repro_walls
cargo test -p ambition_sandbox --test fuzz_random_walker
```

## What this tests

- Whether the assistant knows Cargo's command-line grammar.
- Whether it avoids inventing multi-filter command forms.
- Whether it provides validation commands that actually run the intended checks.
