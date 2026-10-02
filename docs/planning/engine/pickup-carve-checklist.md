# Crate carve checklist

**State:** standing checklist. Use it when you move a family of systems and types
out of the kernel (`ambition_platformer2d_actor_monolith`) into its own crate.
The pickup, abilities and world-items carves were cut against it. Section 1 is
the D33 schedule-ownership rule. The decomposition program is
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md).

## 1. Schedule ownership

- Each system set is configured (`configure_sets`) and registered
  (`add_systems`) in one crate.
- A set that two crates order on is `shared_tangle` vocabulary, configured by
  exactly one owner.
- Look for edges between variants of one set family (a `.chain()` across
  variants). No single variant owns that edge. Declare it in the crate that
  depends on the others. Write a comment at the edge, so a composition that adds
  only the new crate's plugin is understood to lose the chain.
- A kernel system that stays behind and reads the moved types (for example,
  `restore_custody_to_checkpoint`) says in its doc comment why it stays.

## 2. Guards: count the shape, never the name

Bevy strips `system.name()` to a placeholder unless `bevy_ecs`'s `debug` feature
is on, and it is on nowhere in this workspace. A guard that looks a system up by
name matches nothing and reports a scheduled system as missing. The result can
also differ between `-p` and `--workspace` builds, because feature unification
differs per build graph.

Use the schedule graph:

```rust
graph.hierarchy().graph().contains_edge(set_node, NodeId::System(key))  // membership
graph.dependency().graph().contains_edge(a, b)                          // order
graph.system_sets.get_key(Set.intern())                                 // existence
```

- Count direct members and assert the number. Worked examples:
  `crates/ambition_held_items/src/schedule_tests.rs`,
  `crates/ambition_abilities/src/schedule_tests.rs`.
- Assert membership, not existence. An enumeration test passes while a system
  runs in the wrong set.
- Assert absence: a carved plugin does not configure a set it does not own.
  Check `get_key(..).is_none()`; a set nobody named is not in the graph.
- Poison-verify each guard (delete the `.in_set(..)` line and watch it fail).

## 3. Policy rows

A carve touches three rows in `tests/ambition_workspace_policy/policies/*.toml`:

1. `engine.<crate>-manifest-allow`: the new crate's dependency closure.
   Derive it by compiling.
2. `engine.<crate>-source-purity`: roots `crates/<crate>/src`; it must never
   name `ambition_platformer2d_actor_monolith`.
3. `engine.runtime-manifest-allow`: the runtime composes the plugin, so its
   allow-list gains the crate, with a rationale comment.

Some crates use a different legitimate shape (a dependency-free row, or source
purity with no allow-list and a stated reason). Audit by path, not by row id:

```sh
grep -n "<crate>/Cargo.toml" tests/ambition_workspace_policy/policies/*.toml
```

Poison-verify the new rows. Run `cargo test -p ambition_workspace_policy`, not
`cargo check`: the allow-lists are `exact = true`, and the compiler cannot see
them.

**Path-keyed exemptions move with their files.** A `skip_paths`, `watch_paths`,
`roots` or single-`file` entry that names a moving path re-arms its rule against
excused code. Before cutting:

```sh
grep -rn "<crate>/src/<module>" tests/ambition_workspace_policy/policies/*.toml
```

Do not substitute the whole directory with `sed` when only part of it moves.
Read each rewritten line.

## 4. The tail

- `scripts/modules_md.py --write` regenerates MODULES.md.
- Every sub-workspace lockfile that resolves the facade (`fixtures/*/Cargo.lock`,
  `examples/*/Cargo.lock`). `fixtures/minimal_game/Cargo.lock` feeds the
  footprint ratchet; `test_sub_workspace_lockfiles_are_current` checks the rest.
- `scripts/baselines/capability-footprint-baseline.json`: the ratchet counts
  crates, not bytes, so it fires. Declare the growth in the idiom of the
  existing rows.
- `game/ambition_app/tests/rollback_schema_baseline.txt` keys rows by owner
  string and short type name, so a pure move does not change it. If it
  changes, something was renamed. Stop and find out what.
- Export through the facade. Do not re-export the moved types from the kernel.
  A kernel re-export keeps the kernel as the discovery path for code it no
  longer owns.
- Compile `--all-targets` on every crate the carve touched (maintainer ruling
  2026-08-22). Derive the list:

  ```sh
  git diff --name-only <base>..HEAD -- crates game | cut -d/ -f2 | sort -u
  ```

## 5. After the cut

- **Sweep planning citations into the residue.** A carve rarely empties a
  directory. Citations to the kernel residue still resolve while the code they
  mean has moved. For each hit, ask where the name lives now:

  ```sh
  grep -rnoE '`[^`]*(<old/module/path>)[^`]*`' docs/planning --include=*.md
  ```

  Re-read the sentence around a repointed path. A list of neighbours in the same
  sentence may not all have moved.
- **Run the orphan census before and after.** `scripts/orphaned_symbols.py
  --json` finds public functions whose only callers are tests. The total is
  noise; the delta is the signal. A delta of domain functions means callers were
  left behind.
- **Disk.** A carve multiplies feature-matrix variants under `target/`. If
  `target/` is huge, run `scripts/setup/target_bindmount.sh --status` first. An
  absent bind is the usual cause. Follow the `target/` rule in `AGENTS.md`; do
  not prune a shared volume by mtime.
