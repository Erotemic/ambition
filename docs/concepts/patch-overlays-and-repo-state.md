---
id: patch-overlays-and-repo-state
aliases:
  - overlay package
  - stale base
  - repo state
  - platform entrypoint
  - patch packaging
related_adrs:
  - docs/adr/0006-repo-state-and-patch-packaging.md
related_docs:
  - AGENTS.md
related_memory:
  - dev/journals/lessons_learned.md
  - dev/benchmark-candidates/overlay-stale-feature-events-api-question-2026-05-12.md
last_verified: 2026-09-03
---

# Patch overlays and repo state

> **RE-MEASURED 2026-09-03, three and a half months on. ⚠ THIS DOCTRINE IS
> NEARLY DORMANT, AND A READER SHOULD KNOW THAT BEFORE FOLLOWING IT.** The
> workflow it governs — a zip of replacement files unpacked over a checkout —
> belongs to an era when changes arrived as a download. Agents now work directly
> in the repository through git, and the invariants below (*"do not ask the user
> to delete their repo"*, *"preserve platform entrypoints when replacing shared
> files"*) are about a delivery mechanism that mostly no longer happens.
>
> Every reference to `overlay.zip` / `unzip -o` in the tree is historical — a
> May benchmark note, `dev/journals/lessons_learned.md`, and a dated patch
> record — **with one exception that is live and misleading**:
> `tools/ambition_sfx_renderer/README.md:221` heads a section *"Current overlay <!-- cite-ok: a path inside a SUBMODULE; the superproject does not track it -->
> notes"* and tells the reader to run
> `unzip -o ~/Downloads/ambition_sfx_renderer_overlay_duration_policy.zip`. ⛔ No
> such archive exists anywhere in this repository, and the path is a person's
> home directory.
>
> ⇒ **Kept, not retired.** The invariants are still correct for what they
> describe, and this page is the only place they are written down; if an overlay
> ever arrives again the rules apply unchanged. But it should be read as a
> contingency, not as current practice, and the one README claiming to be
> "current" is the thing to fix rather than this page.
>
> ⛔ **AND AN ACCEPTED ADR GOVERNS THIS DORMANT WORKFLOW, CARRYING A SNAPSHOT
> THAT HAS MOVED BY AN ORDER OF MAGNITUDE.**
> [`../adr/0006-repo-state-and-patch-packaging.md`](../adr/0006-repo-state-and-patch-packaging.md)
> is status **Accepted**, opens *"Ambition is being developed through iterative
> patch zips"*, and requires crate-local files to stay under
> `crates/<workspace_member>/` — *"(today: `ambition_platformer2d_actor_monolith`,
> `ambition_asset_manager`, `ambition_sfx`, `ambition_sfx_bank`)"*. **There are 63
> crates now.**
>
> ⇒ **Not edited, deliberately.** `AGENTS.md` treats ADRs as things a NEW ADR
> supersedes (*"do not resurrect backend-neutral constraints unless a new ADR
> says so"*), and no amendment convention exists in `docs/adr/`. Rewriting an
> accepted decision record to keep its parenthetical current would destroy the
> record while looking like maintenance. ⇒ The finding is filed here instead: the
> DECISION may well still be right; the four-crate enumeration inside it is a
> 2026-era snapshot and should not be read as a list of where crate-local files
> may live. Superseding 0006 — or marking it dormant beside its overlay workflow
> — is a maintainer's call.
>
> ⚠ **That README was NOT fixed here, and the reason is worth recording:**
> `tools/ambition_sfx_renderer` is a **submodule**, a separate repository with
> its own history and its own credentials. Editing it from this checkout would
> commit to another project and move the submodule pointer in the same breath —
> so the finding is filed here for whoever owns that repository, and the file
> itself was restored untouched.

## Definition

Overlay packages are complete replacement-file patches that unpack over a user's checkout. They are convenient, but dangerous when based on stale source snapshots or broad files with platform-critical entrypoints.

## Core invariants

- Do not ask the user to delete their repo before applying an overlay.
- Prefer complete replacement files that reflect the desired end state.
- Preserve Android/web/platform entrypoints when replacing shared files.
- Do not clobber current typed event/message APIs with stale copies.
- Include validation notes and a clear commit message.

## Edit protocol

1. Inspect the current uploaded source, not a remembered old repo shape.
2. Replace only files needed for the chunk.
3. If replacing broad files, verify platform entrypoints and feature flags survived.
4. Package the overlay so it can be applied with `unzip -o` over the checkout.
5. Include follow-up commands: test/build/check and `git add` / `git commit`.

## Validation

```bash
unzip -l overlay.zip
cargo fmt --check
cargo test -p ambition_platformer2d_actor_monolith --lib
```

Adjust the code validation to the files touched by the overlay.
