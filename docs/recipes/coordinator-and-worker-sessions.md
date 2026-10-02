---
status: current
last_verified: 2026-10-02
related_docs:
  - AGENTS.md
  - docs/recipes/cheapest-sufficient-check.md
---

# Coordinating subagents, and worktrees

This page is for a session that spawns and integrates subagents. A solo linear
session needs none of it.

The shape that works: **workers write; the coordinator holds the build lease and
verifies.**

## Workers do not run `cargo` by default

`scripts/setup/target_bindmount.sh` gives each worktree its own target store, so
two agents do not share a lock or fingerprints. The remaining reason for the
default is CPU and cache: several cold builds contend for the cores, and a cold
target is tens of GB of writes before a test runs. The default is a scheduling
choice. You may reverse it for one worker.

- Treat every worker test claim as UNRUN. Budget coordinator time for repairs.
- If a worker must compile, bind-mount its worktree and let it build in its own
  store, or hand it the lease and stop verifying while it holds it.
- ⛔ Do not let two parties build into one directory, or into two directories
  while they believe it is one.

## The coordinator is the workers' compiler

A worker that edits the **shared tree** produces diagnostics that the
coordinator's editor integration shows. Relay them while the worker runs. A
relayed error is often architectural (for example, a field the worker's whole
route depends on does not exist). A worktree gives up this property.

## A fresh worktree needs two commands

Run both from inside the worktree. Neither takes a path argument.

```sh
# 1. assets and submodules. A fresh `git worktree` has neither.
python3 scripts/mirror_assets_for_worktree.py
python3 scripts/mirror_assets_for_worktree.py --dry-run   # see what it would do

# 2. put THIS worktree's target/ on its own store. Idempotent.
scripts/setup/target_bindmount.sh
scripts/setup/target_bindmount.sh --status                # which dir am I building into?
```

- Generated art, audio and packs are gitignored. The sheet registry is baked from
  those directories at build time, so an assetless worktree compiles an EMPTY
  sheet table and unrelated tests fail. The script symlinks file by file, so a
  regenerated sprite lands as a real file in the worktree.
- The script checks out the `game/ambition_map_assets` submodule. It holds every
  `.ldtk` world, and the files under `game/*/assets/worlds/` are symlinks into
  it. Without it, LDtk work fails with `FileNotFoundError` on a path that looks
  present. A submodule is checked out, never mirrored.
- ⛔ Do not export `CARGO_TARGET_DIR` instead of the bind mount. Other callers
  (for example the goal guard) do not see your shell's variable and build into
  the default directory. A bind mount keeps cargo's default path for every
  caller.
- A bind mount does not survive a reboot. Run the script again after one.

## Which lane belongs where

- **Shared tree:** narrow, design-risky slices where the coordinator wants live
  diagnostics. One at a time: a broken core crate blocks verification of every
  lane.
- **Worktree:** wide mechanical changes and pure measurement.

## Traps

- ⛔ **`git commit` commits the whole index.** In a shared tree, use the pathspec
  form: `git commit -F - -- path/one path/two`.
- ⛔ **Do not prune worktrees by "merged into `main`".** A new worktree with no
  commits looks the same as a stale merged one.
- **A subagent gets a worktree only if the spawn asks for one.** Otherwise it
  edits the shared tree. Do not tell it to "work in your worktree" when it has
  none.
- **Baselines are the coordinator's job.** A worker that cannot run `cargo`
  cannot regenerate `game/ambition_app/tests/rollback_schema_baseline.txt` or the
  ratchets under `scripts/baselines/`. It must say that one is owed.

## Accepting a handback

- Require "what you could NOT verify, lowest confidence first", and read it.
- Run the falsifier yourself. A poison that was reasoned and not executed has
  been wrong: a test can stay green with its system unregistered when the test
  lists that system in its own chain.
- Verify a worker's red finding by a different route too.
- Check a peer's argument as well as a peer's quote. When a handback carries two
  arguments, check the one that closes the question first. A disqualifier added
  to make an argument decisive is what the maintainer weighs hardest and what its
  author checked least.
