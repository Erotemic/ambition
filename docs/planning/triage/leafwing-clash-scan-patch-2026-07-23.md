# leafwing clash-scan short-circuit — deferred upstream patch

> **State:** TRIAGE, 2026-07-23. Deliberately NOT applied — Jon does not want
> to carry a leafwing fork right now. Everything Ambition-side is already
> landed and inert until the dependency changes.

## The cost

`possible_clash` / `handle_clashes` burns **1–3.1% of frame CPU** in every
gameplay chunk of `desktop-lifecycle-5` (single ~20-action `SandboxAction`
map, zero chords). Upstream `handle_clashes` runs the full O(actions²)
`possible_clashes()` pair scan — `decompose()` allocations per pair, rebuilt
from scratch every frame, per `InputMap` — **before** consulting
`ClashStrategy`; `PressAll` only nulls each clash afterward inside
`resolve_clash`. So with zero chords the entire pass is a semantic no-op that
still pays full price, and no strategy value can avoid it from outside.

## What is already in place (no action needed)

- `tune_clash_strategy_to_bindings` (`ambition_host`, 07bb6bd8c): derives the
  strategy from the live bindings — chord-free maps relax to `PressAll`; the
  frame any composed game authors a chorded binding it returns to
  `PrioritizeLongest`. Both directions pinned by
  `cargo test -p ambition_host --features input`. Harmless today, becomes the
  payoff switch the moment the patched dep lands.
- The exact patch, with rationale and wiring instructions:
  `dev/patches/leafwing-0.20-pressall-shortcircuit.patch` (5f92c96fa).
  Two lines in `src/clashing_inputs.rs::handle_clashes`: return early when
  `clash_strategy == ClashStrategy::PressAll`.

## When picked up

1. Fork `leafwing-input-manager` at v0.20.0, apply the patch.
2. Add a `[patch.crates-io]` entry in the workspace `Cargo.toml`, same shape
   and RETIRE discipline as the existing `bevy_ggrs` entry (git fork + rev,
   HACK-tagged comment).
3. Verify with a `timeline-run` capture: the `clash` category should drop to
   ~0 in gameplay chunks.
4. Upstream it: the change is a clean PR candidate (pure fast-path, no
   behavior change), which is also the retirement path for the fork.

Alternative if a fork is never wanted: bumping to a newer leafwing (with a
Bevy upgrade) may obsolete this — re-measure before carrying anything.
