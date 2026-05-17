# 0006: Require explicit repo-state and patch packaging discipline

## Status

Accepted

## Context

Ambition is being developed through iterative patch zips. Some agent environments may only have partial context from earlier patch files, while the user may have a more complete current working tree. Accidentally generating files from partial context can create duplicate top-level directories, stale documentation, or patches that are harder to apply and review.

## Decision

Before producing a patch, an agent should verify whether it has a full repo checkpoint. If it does not, it must say so in the response and keep changes narrowly scoped to inspected files.

Patch zips may contain only modified files to save bandwidth, but they must preserve repo-relative paths exactly and include `patch` in the zip name. Crate-local files must stay under `crates/ambition_engine/` or `crates/ambition_sandbox/`; agents must not create duplicate top-level crate directories such as `ambition_sandbox/`. Documentation should normally live under `docs/`, except for the root `README.md`.

Because the standard apply command extracts files but does not remove stale paths, cleanup of accidentally created files or directories must be called out with explicit shell commands.

## Consequences

This keeps bandwidth low while making patch scope clear. It also makes future agent work safer: if a patch was generated without a full repo view, reviewers can treat it as partial by design rather than assuming it reflected the whole project.

## Current implications for agents

- Verify whether a patch is based on a full repo checkpoint.
- Preserve repo-relative paths exactly in overlay packages.
- Provide explicit cleanup commands when stale paths must be removed.
