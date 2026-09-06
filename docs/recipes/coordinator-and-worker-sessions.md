---
status: current
last_verified: 2026-08-15
related_docs:
  - AGENTS.md
  - docs/recipes/cheapest-sufficient-check.md
---

# Coordinating subagents, and worktrees

⚠ **This is for a session that spawns and integrates subagents.** A solo linear
session needs none of it.

The shape that works: **workers write, the coordinator holds the build lease and
verifies.** Everything below is a measured consequence of that split, not taste.

## ⚠ Workers do not run `cargo` — but the REASON changed on 2026-08-15

⛔⛔ **the old reason is gone; re-measure before repeating it.** This section
said per-worktree target dirs were unavailable because one shared dir was
~143 GB against ~143 GB free. Both halves are now false: the stale dirs are
deleted (**379 GB free**), and `scripts/setup/target_bindmount.sh` gives **each
worktree its own backing store on ext4**, keyed by path, so two agents no longer
share a lock or thrash each other's fingerprints.

⇒ **the surviving reason is CPU and cache, not capacity.** Three cold builds on
8 cores still contend, and a cold target dir is ~40–80 GB of writes before a
single test runs. So the default stays "workers write, the coordinator
compiles" — but it is now a *scheduling* choice you may reverse for one worker,
not a hard limit.

⭐ **and the cost of that default is measured, not theoretical.** Of six lanes on
2026-08-15, two handed back code that did not compile (a `u32`/`usize` pair, a
borrow of a temporary) and one handed back a confident diagnosis that a
five-minute source read overturned. ⇒ **treat every worker test claim as UNRUN**,
and budget coordinator time for repairs rather than being surprised by them.

⇒ if a worker genuinely needs to compile, either **bind-mount its worktree** and
let it build in its own store, or **hand it the lease** and stop verifying while
it holds it. ⛔ what you must not do is let two parties build into one directory,
or into two directories while believing they are one — see the
`CARGO_TARGET_DIR` note in `.cargo/config.toml` for what that cost.

## ⭐ The coordinator IS the workers' compiler

A worker editing the **shared tree** produces diagnostics the coordinator's editor
integration surfaces automatically. **Relay them mid-flight.** This is what makes
no-compile workers cheap, and it is the property a worktree gives up.

⚠ a relayed error is often architectural, not syntactic — *"`BrainSnapshot` has no
`abilities` field"* told a worker its whole route did not exist, hours before its
handback would have.

## ⛔⛔ A fresh worktree needs TWO commands before it is usable

**Run both FROM INSIDE the worktree.** Neither takes a path argument; each finds
what it needs itself.

```sh
# 1. assets + submodules. A fresh `git worktree` has neither.
python3 scripts/mirror_assets_for_worktree.py
python3 scripts/mirror_assets_for_worktree.py --dry-run   # see what it would do

# 2. put THIS worktree's target/ on ext4 instead of the shared virtiofs mount.
#    Idempotent, and a no-op on a machine whose checkout is already local.
scripts/setup/target_bindmount.sh
scripts/setup/target_bindmount.sh --status                # which dir am I building into?
```

⚠ **the bind mount is opt-in and safe to skip** — you just get a slower target
dir on the shared mount. ⛔ **what is NOT safe is exporting `CARGO_TARGET_DIR`
instead**, because an env var set in your shell does not reach cargo runs made
by anything else. On 2026-08-15 the goal guard ran `cargo test --test app_it` as
one of its checks, resolved the target dir from the committed config, and hit a
link failure there — for hours, while the session was green in the directory it
had exported. A bind mount cannot split that way: the path stays cargo's
default, so every caller lands in the same place.

⚠ a bind mount does not survive a reboot; re-run after one. `--status` says
plainly whether this worktree is bound.

Generated art/audio/packs are gitignored, so a fresh `git worktree` has none.
**The sheet registry is baked from those directories at build time, so an
assetless worktree compiles a binary with an EMPTY sheet table** — around forty
tests then fail for reasons unrelated to the change under test. The script
symlinks **file by file** on purpose, so a regenerated sprite lands as a real file
in the worktree instead of writing back into the main checkout.

⚠ **and it now checks out `game/ambition_map_assets` first, which it did not
before.** That submodule holds every `.ldtk` world, and the files under
`game/*/assets/worlds/` are symlinks into it — so an uninitialised submodule makes
them dangling links and any LDtk work dies at minute one on a bare
`FileNotFoundError` naming a path that visibly exists. Nothing in that traceback
says "submodule". A submodule is **checked out, never mirrored**: it is
version-controlled content, and a symlink would route an edit made in the worktree
into the main checkout's index.

⇒ **run it, and authored-content work can go to a worktree too.**

## Which lane belongs where

- **shared tree** — narrow, design-risky slices, where the coordinator wants live
  diagnostics. ⚠ **one at a time**: a single broken core crate blocks verification
  of *every* lane, because only the coordinator can build.
- **worktree** — wide mechanical changes (their errors are mechanical and the
  noise is unactionable) and pure measurement.

## Traps that have actually cost work here

- ⛔⛔ **`git commit` commits the WHOLE INDEX.** `git add <my paths>` then
  `git commit` sweeps whatever a concurrent worker had staged. In a shared tree
  use the pathspec form: `git commit -F - -- path/one path/two`.
- ⛔⛔ **do not prune worktrees by "merged into `main`".** An agent-created worktree
  with no commits yet is indistinguishable from a stale merged one; pruning that
  way destroyed a live worker's uncommitted work.
- ⛔ **a subagent only gets a worktree if the spawn explicitly asks for one.**
  Otherwise it is editing the shared tree, and a brief that says *"work in your
  worktree"* is a lie the worker will act on.
- ⚠ **baselines are the coordinator's job.** A worker that cannot run `cargo`
  cannot regenerate `rollback_schema_baseline.txt` or the ratchet under
  `scripts/baselines/`. It must say loudly that one is owed; the coordinator runs it.

## Accepting a handback

Require *"what you could NOT verify, lowest confidence first"*, and read it — it
has been accurate about its own weakest claims every time.

⛔⛔ **and run the falsifier yourself.** *"Poison reasoned, not executed"* has been
**wrong** here: a fix's test stayed green with its system unregistered, because the
test listed that system in its own chain. Verify a worker's red finding by a
different route too — a census once reported six brain families failing to rewind
when the snapshot clones the whole component and only *detection* was missing.
