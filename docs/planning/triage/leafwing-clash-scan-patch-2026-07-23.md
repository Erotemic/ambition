# leafwing clash-scan short-circuit — deferred upstream patch

> **State:** TRIAGE, 2026-07-23. Deliberately NOT applied — Jon does not want
> to carry a leafwing fork right now. Everything Ambition-side is already
> landed and inert until the dependency changes.

> ⛔⛔ **RE-CHECKED against `8bb0dd5a7` (2026-09-03): THE DEPENDENCY ALREADY CHANGED, AND THIS PAGE'S
> OWN ESCAPE HATCH WAS TAKEN WITHOUT THE RE-MEASURE IT ASKS FOR.** The last
> line below says *"bumping to a newer leafwing … may obsolete this —
> re-measure before carrying anything."* The bump happened:
> `leafwing-input-manager = "0.21"` in both `game/ambition_app/Cargo.toml` and
> `crates/ambition_platformer2d_host/Cargo.toml`, while the patch on disk is
> still named for 0.20. Here is that re-measure.
>
> **The defect SURVIVES in 0.21, unchanged in shape.** `handle_clashes` still
> calls `get_clashes(...)` and only then hands each pair to `resolve_clash`,
> so `ClashStrategy` is still consulted *after* the scan, and `PressAll` still
> nulls each clash afterwards. The two-line fast path is still applicable.
>
> ⚠ **AND UPSTREAM'S OWN COMMENT WILL TELL A CHECKER OTHERWISE.** `get_clashes`
> in 0.21 says *"We can limit our search to the cached set of possibly clashing
> actions"* — but `possible_clashes()` is a method that builds a fresh `Vec`
> with the full nested `buttonlike_actions()` loop on every call, and there is
> no cache field to read. So "rebuilt from scratch every frame" is still true,
> and anyone who checks by reading that comment will conclude the opposite.
>
> ⭐ **THE COST GREW, AND THE 1–3.1% BELOW IS NOW A FLOOR.** That number was
> measured against a *"single ~20-action"* map. `presets.rs` binds **35 of the
> enum's 36 variants** today, and 4 are declared axis-like
> (`#[actionlike(DualAxis)]` ×3, `#[actionlike(Axis)]` ×1), so roughly **31
> buttonlike actions** feed the O(n²) pair scan — about **2.4× the pairs** the
> percentage was taken at. ⚠ The percentage itself has NOT been re-measured; a
> `timeline-run` capture is what would replace it, and until then treat 1–3.1%
> as the low end rather than the current figure.
>
> Still in place and still correct: `tune_clash_strategy_to_bindings`
> (`crates/ambition_platformer2d_host/src/lib.rs:428`) and the patch file. ⇒ The
> "When picked up" steps below need one edit before use — fork at **0.21**, not
> 0.20 — and the patch should be re-checked against 0.21's `clashing_inputs.rs`
> rather than assumed to apply.

## The cost

`possible_clash` / `handle_clashes` burns **1–3.1% of frame CPU** in every
gameplay chunk of `desktop-lifecycle-5` (single ~20-action `Platformer2dInputActionMonolith`
map, zero chords). Upstream `handle_clashes` runs the full O(actions²)
`possible_clashes()` pair scan — `decompose()` allocations per pair, rebuilt
from scratch every frame, per `InputMap` — **before** consulting
`ClashStrategy`; `PressAll` only nulls each clash afterward inside
`resolve_clash`. So with zero chords the entire pass is a semantic no-op that
still pays full price, and no strategy value can avoid it from outside.

## What is already in place (no action needed)

- `tune_clash_strategy_to_bindings` (`ambition_platformer2d_host`, 07bb6bd8c): derives the
  strategy from the live bindings — chord-free maps relax to `PressAll`; the
  frame any composed game authors a chorded binding it returns to
  `PrioritizeLongest`. Both directions pinned by
  `cargo test -p ambition_platformer2d_host --features input`. Harmless today, becomes the
  payoff switch the moment the patched dep lands.
- The exact patch, with rationale and wiring instructions:
  `dev/patches/leafwing-0.20-pressall-shortcircuit.patch` (5f92c96fa).
  Two lines in `src/clashing_inputs.rs::handle_clashes`: return early when
  `clash_strategy == ClashStrategy::PressAll`.

## Re-measured 2026-09-03 — the dependency moved, the defect did not, and the patch was corrupt

The page's own last line says *"bumping to a newer leafwing (with a Bevy
upgrade) may obsolete this — re-measure before carrying anything."* The bump
happened. Re-measured:

⚠ **leafwing is `0.21.0` at HEAD, not `0.20.0`.** Step 1 below ("fork at
v0.20.0") is stale, and so is the patch's filename.

⛔ **THE DEFECT SURVIVED THE BUMP — verified by reading 0.21's source, not its
changelog.** `handle_clashes` still calls `get_clashes(..)` and only then hands
each result to `resolve_clash(.., clash_strategy, ..)`, so the strategy is still
consulted AFTER the scan; and `get_clashes` still loops over
`self.possible_clashes()`, which still builds a fresh `Vec` of every
action-pair on every call. The cost analysis above therefore still stands at
0.21.

⚠ **AND UPSTREAM NOW CLAIMS A CACHE IT DOES NOT HAVE.** 0.21's `get_clashes`
carries the comment *"We can limit our search to the cached set of possibly
clashing actions"*, and the function it calls allocates a new vector each time.
A reader who trusted that comment would conclude this triage item was fixed
upstream. It is not — reading the callee is what separates the two.

⛔⛔ **THE STORED PATCH COULD NEVER HAVE BEEN APPLIED BY ANYONE.** Its hunk
header read `@@ -173,6 +173,11 @@` while the hunk body carries 8 old and 13 new
lines (7 context, 5 added, 1 trailing context). Both `git apply` ("corrupt patch
at line 32") and `patch(1)` ("malformed patch") refuse it. It was written by
hand, described here as *"the exact patch, with rationale and wiring
instructions"*, and never test-applied — for six weeks it was a ready-to-use
artifact that was not usable.

✔ **FIXED AND VERIFIED, 2026-09-03.** The header is now `@@ -173,8 +173,13 @@`,
and `git apply` applies it cleanly to leafwing `0.21.0`'s
`src/clashing_inputs.rs`, producing exactly the intended early return. Verified
by applying it to a pristine copy of the 0.21 source and reading the result, and
by confirming `git apply` REJECTS the old counts — so this is a real check, not
a lenient one.
⚠ `patch -p1` still refuses the corrected file for a reason I did not run down;
`git apply` is the verified path. Whoever picks this up should use it.

⇒ **The item is still LIVE and still deferred** — the Ambition side
(`tune_clash_strategy_to_bindings`,
`crates/ambition_platformer2d_host/src/lib.rs:428`) is present and still inert
by design. Nothing here argues for taking the fork; it argues that if it is ever
taken, the artifact now works and targets the version actually in the lockfile.

## When picked up

1. Fork `leafwing-input-manager` at **v0.21.0** (the version in the lockfile; the
   patch is verified against it), apply the patch with `git apply`.
2. Add a `[patch.crates-io]` entry in the workspace `Cargo.toml`, same shape
   and RETIRE discipline as the existing `bevy_ggrs` entry (git fork + rev,
   HACK-tagged comment).
3. Verify with a `timeline-run` capture: the `clash` category should drop to
   ~0 in gameplay chunks.
4. Upstream it: the change is a clean PR candidate (pure fast-path, no
   behavior change), which is also the retirement path for the fork.

Alternative if a fork is never wanted: bumping to a newer leafwing (with a
Bevy upgrade) may obsolete this — re-measure before carrying anything.
