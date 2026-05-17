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
  - docs/AGENT_HANDOFF.md
related_memory:
  - dev/journals/lessons_learned.md
  - dev/benchmark-candidates/overlay-stale-feature-events-api-question-2026-05-12.md
last_verified: 2026-05-17
---

# Patch overlays and repo state

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
cargo test -p ambition_sandbox --lib
```

Adjust the code validation to the files touched by the overlay.
