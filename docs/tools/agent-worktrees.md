# Agent worktrees

Three fixed worktrees. A **coordinator** (Jon, or the coordinating session) assigns
a slot; the agent working there does not choose one for itself.

```
.worktrees/agent-worktree1     fast lane
.worktrees/agent-worktree2     medium
.worktrees/agent-worktree3     slow lane
```

⛔ **Never create a worktree named after a feature.** Slots are numbered because
identity is the *path*, not the work. Feature-named worktrees accumulate, nobody
can tell which are live, and each one costs a cold target directory. They are
checked out **detached**; your first act is `git switch -c <your-branch>`.

⛔ **Do not claim a slot yourself.** Ask the coordinator. `list` shows who is
where, so a collision is visible — but the fix is assignment, not detection.

## Commands

```bash
scripts/agent_worktree.sh list                  # slots, HEAD, size, who is building
scripts/agent_worktree.sh setup 1|2|3|all       # create + submodules + assets + bind mount
scripts/agent_worktree.sh seed 2 [--from PATH]  # warm from an existing target (default: main)
scripts/agent_worktree.sh clear 2 [--incremental|--all]
scripts/agent_worktree.sh dedupe [--apply]      # hardlink identical artifacts across slots
scripts/agent_worktree.sh jobs 2                # the -j number for that slot
```

Every destructive or linking command refuses while that tree's build lock is
held, so a slot cannot be cleared or seeded out from under a running build.

## CPU budget

Whoever is on `main` gets the machine. Each slot gets half of the one above it.

| where | `-j` on a 12-core box | rule |
|---|---|---|
| main | 12 | `nproc` |
| slot 1 | 6 | `nproc / 2` |
| slot 2 | 3 | `nproc / 4` |
| slot 3 | 1 | `nproc / 8` |

**Pass it.** `cargo build -j "$(scripts/agent_worktree.sh jobs 2)"`, or
`./run_tests.sh -j N`, or export `CARGO_BUILD_JOBS`. Three agents that each
believe they own twelve cores turn one build into three slow builds.

Slot 3 is *meant* to be slow — it is for work that can take its time. A
coordinator overrules any of this by naming a different `-j`.

## What a fresh worktree costs

`setup` handles all three, and each one is a real failure if skipped:

- **Submodules** — empty in a fresh worktree. `game/ambition_map_assets` holds
  every `.ldtk` world and the symlinks into it dangle without init, so authoring
  work dies on a `FileNotFoundError` naming a path that visibly exists. An
  unpushed submodule sha can only come from a warm clone.
- **Generated art** — gitignored, so an unmirrored worktree bakes an EMPTY sheet
  registry and ~40 tests fail for reasons unrelated to the change.
  `scripts/mirror_assets_for_worktree.py` symlinks them file by file; a
  regenerated sprite lands as a real file and never touches main's copy.
- **A warm target** — cold means an hour before the first useful result.

## Target directories

Each slot's `target/` is bind-mounted to its own store under
`~/.cache/ambition-targets/`, keyed by worktree path. Slots never share a target,
so **concurrent builds in different slots are fine** — the old "do not build
against the shared target" rule applied to unbound worktrees.

⚠ **A bind mount does not survive a reboot.** Re-run `setup all` after one, or
builds silently go to the shared virtiofs mount and everything gets slower.
`list` prints `LOCAL` instead of `bound` when that has happened.

## Seeding and clearing

`seed` copies a warm target into a slot:

| what | how | why |
|---|---|---|
| `deps/` | **hardlinked** | content-hashed and *replaced* on rebuild (new inode), so links are safe and cost no space |
| `.fingerprint/`, `build/` | copied | `*.json` is rewritten **in place** (same inode) — a hardlink here corrupts every tree sharing it |
| `incremental/` | skipped | per-worktree edit-loop state, and the largest thing on disk |

⛔ Linking happens through the **backing stores** under
`~/.cache/ambition-targets/`, never through the mounted `target/` paths: each
slot's target is its own bind mount, and `link()` returns `EXDEV` across mount
points even when both sit on one filesystem. Seeding slot 3 from a warm main
this way moved 58 GB in 1.4s and consumed no disk.

`clear <n>` drops `incremental/` only — artifacts survive, so the next build
relinks rather than recompiles. `clear <n> --all` goes cold; reseed afterwards.

`dedupe` hardlinks byte-identical artifacts **across** slots, `deps/` only.
Cargo's `<crate>-<16 hex>` naming means an identical name implies identical
inputs, and sampling this repo found 0 of 25 third-party rlibs embedding the
checkout path — so foundation crates deduplicate across worktrees. First-party
`ambition_*` crates, and crates whose build script `include!`s generated code
(serde embeds its `OUT_DIR`), simply fail the content compare. No allowlist is
needed. Dry-run by default.

## Etiquette

- `list` before you touch anything. `BUSY=yes` means a build holds that lock now.
- Work only in the slot you were given. Verify on `main` only when asked to.
- Leave the slot on your branch when you finish; the coordinator reassigns it.
