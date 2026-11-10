---
status: current
last_verified: 2026-11-10
related_docs:
  - AGENTS.md
  - docs/tools/agent-worktrees.md
  - docs/recipes/cheapest-sufficient-check.md
---

# Coordinating agents and builds

This page applies when a session delegates code changes to other agents.
The user can intentionally put several agents in one checkout. Respect the
assigned directory and coordinate edits there; do not introduce worktrees
merely to avoid possible conflicts.

## Determine what is shared

Similar hostnames do not prove that cloned VMs share a filesystem. Use
`git rev-parse --show-toplevel`, `git worktree list`, and filesystem or
build-lock evidence before assuming that another agent can see local
uncommitted edits or is using the same `target/`. Remote agents coordinate
through commits and pushes.

## Editing and verification

In a shared checkout, assign files or tasks explicitly where useful.
Source edits can be visible immediately to a coordinator's diagnostics,
but a running build may read a changing tree. Coordinate a stable point
for meaningful verification. Do not force an agent into another checkout
if the maintainer deliberately assigned shared work.

Workers do not launch expensive Cargo builds by default. The coordinator
can run the focused check after a coherent change, or explicitly delegate
a build when parallel compilation is justified. Never run concurrent Cargo
builds against the same writable target. A worker that did not run a check
should report it as unrun, not failed or passed.

When an isolated worktree is explicitly authorized, use one of the fixed
slots described in [agent worktrees](../tools/agent-worktrees.md). The
helper prepares the map submodule, generated assets, and target storage.
A worktree on local storage does not need a bind mount merely because it
is a worktree; on `virtiofs`, use the provided mount setup.

## Commits and handoff

In a shared tree, inspect the staged index before committing. Stage only
the files for your task and use explicit pathspecs when necessary; a plain
`git commit` includes everything already staged.

Before integrating independent work, account for shared constants, schema
versions, and generated files. A clean Git merge can still produce an
incorrect combined meaning.

A useful handoff says what was changed, which checks actually ran, what
remains, and the highest-risk unverified behavior. The coordinator need not
rerun broad suites merely because multiple agents contributed. Choose the
cheapest check that can expose each consequential integration defect.

See [verification choices](cheapest-sufficient-check.md) and
[submodule policy](submodules.md).
