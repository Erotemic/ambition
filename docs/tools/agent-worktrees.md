# Agent worktrees

Work in the checkout and directory assigned by the maintainer or coordinator.
Multiple agents may deliberately share one working tree; that is an accepted
tradeoff, not a reason to move a task elsewhere. **Do not create, switch to,
or claim a worktree without explicit authorization.** Building in an
independently selected worktree is not a default workaround for concurrent
edits or a long-running build.

When authorized, use the existing fixed slots. A coordinator assigns the
slot and its work; the worker stays in that slot.

```text
.worktrees/agent-worktree1
.worktrees/agent-worktree2
.worktrees/agent-worktree3
```

Avoid feature-named worktrees. Numbered slots make ownership and cleanup
explicit.

## Commands for an authorized slot

Run from the primary repository checkout:

```bash
scripts/agent_worktree.sh list
scripts/agent_worktree.sh setup 1
scripts/agent_worktree.sh seed 1
scripts/agent_worktree.sh jobs 1
scripts/agent_worktree.sh clear 1 --incremental
```

`setup` initializes submodules, mirrors git-ignored assets, and configures
target storage when needed. Fresh linked worktrees do not automatically
contain generated art or initialized submodules. For an already created
authorized worktree, `python3 scripts/mirror_assets_for_worktree.py` prepares
the same assets and map submodule.

A slot is checked out detached; create a task branch there if the
coordinator requested one. Check `list` before making structural changes.
It reports the current branch, target state, and active build lock.

## Builds and target storage

A worktree is not permission to run Cargo. The maintainer or coordinator
decides which agent builds and when. Prefer one coordinated build over
several simultaneous cold builds; workers can usually hand code back for
verification in the assigned checkout.

When a build is authorized, inspect the target before building:

```bash
scripts/setup/target_bindmount.sh --status
```

`target_bindmount.sh` uses a guest-local store for worktrees on `virtiofs`.
On an already local filesystem, a bind mount is optional and the script
reports that no mount is needed. On `virtiofs`, set up the mount before
a substantial build to avoid shared-filesystem overhead. A mount does not
survive a reboot.

Distinct worktrees must not write to one shared Cargo target directory.
The prescribed storage setup gives each slot its own target. Do not use
`CARGO_TARGET_DIR` as an improvised replacement for a required target-store
setup; scripts that do not inherit your environment would use another path.

The worktree helper caps its job budget at six for the primary checkout and
divides that limit across the three slots (half, quarter, eighth, with a
minimum of one). Ask the helper instead of hardcoding counts:

```bash
cargo check -j "$(scripts/agent_worktree.sh jobs 1)" -p ambition_app
```

Do not clear or reseed a slot while its build lock is held. Before cleanup
under a target directory, confirm the correct mount or local ownership;
the backing store may be shared with other tasks. The helper provides
`clear` and `dedupe` for explicitly authorized maintenance.

## Generated assets and submodules

Generated binary assets are git-ignored, so an unprepared worktree can
build an empty sprite registry and fail unrelated checks. The mirror script
links assets file by file. A publisher must not write through a symlink
back into the primary checkout; it should replace the symlink before writing
a generated output. This behavior has a focused repository check.

When a submodule has diverged, follow
[the submodule procedure](../recipes/submodules.md); do not reset or
overwrite another agent's work.

## Integrating a slot

The coordinator handles merging and reassignment. Typically, merge the
current primary branch into the assigned slot, resolve and verify there,
then merge the slot back. Check shared schema numbers or counters after
merging; a conflict-free textual merge can still combine different schema
changes under one version number.

Leave the slot on its task branch for reassignment unless the coordinator
asks for cleanup. For shared-tree work, coordinate file ownership and
commits directly instead of silently changing checkout topology.

See [coordinating agents](../recipes/coordinator-and-worker-sessions.md) and
[cheapest sufficient check](../recipes/cheapest-sufficient-check.md).
