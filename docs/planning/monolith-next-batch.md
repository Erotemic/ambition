# Monolith breakup — next batch (scoping)

_Scoped 2026-06-14 (Opus 4.8, autonomous). Builds on `monolith-endgame-run.md`
(the de-naming + seam-proof run) and `refactor-candidates.md`. This is planning
material — turn an item into a patch only with the stated validation command and
a clean rollback boundary._

North star (unchanged, per memory): turn `ambition_sandbox` into a reusable
2D-platformer **engine** with named content in `ambition_content`. Oracle: _could
a different platformer be built by ADDING a content crate without editing core?_
Jon's four goals the work must serve: **(1)** incremental compile time, **(2)**
agent-navigability, **(3)** idiomatic Bevy plugins, **(4)** audit-grade reuse.

## Where we are (measured 2026-06-14)

`ambition_sandbox` is **88k LOC** — still the monolith (next crate, `ambition_app`,
is 21k). Subdir size vs **outward coupling** (distinct `crate::<mod>` paths each
subdir imports — the extractability metric, not content-refs):

| subdir | outward deps | LOC | read |
|---|---|---|---|
| features | 28 | 13541 | ECS feature systems — core machinery, highly woven |
| presentation | 36 | 10379 | renders everything; **but 44 lib files import IT** (see below) |
| world | 21 | 9371 | LDtk + rooms |
| mechanics | 26 | 5166 | combat woven into ~10 subsystems |
| boss_encounter | 17 | 5040 | boss machinery (data already de-named → content) |
| player | 10 | 3927 | |
| persistence | 8 | 3831 | save/settings — lowest coupling of the big ones |
| abilities | 17 | 3469 | |
| projectile | 17 | 2754 | |
| encounter | 13 | 2450 | |
| items | 22 | 2449 | |
| dialog | 13 | 1989 | yarn runtime + content |
| enemy_projectile | 12 | 1122 | |
| body_mode | 5 | 820 | depends on player + presentation |
| music | 5 | 596 | content-ish |
| quest | 2 | 563 | content-ish (cleanest outward dep count) |

## Why pure crate-extraction keeps failing (the honest finding)

Three runs have now hit the same wall: **there is no clean leaf to extract DOWN**.
The low-outward-dep modules aren't leaves — they depend on mid-tier modules:

- `quest` (2 deps) → `persistence` + `rooms`
- `body_mode` (5 deps) → `player` + `presentation`
- `music` (5 deps) → `audio` + `encounter` + `persistence` + `rooms` + `runtime`

So extracting any of them below the lib would drag a chain. And the obvious
move-UP candidate — `presentation` (render-only, _should_ sit above the lib like
`ambition_app`) — has **44 lib files importing `crate::presentation`**, because
sim systems reference render/message types directly. It is not a leaf in either
direction.

**Conclusion:** the productive work is not "find a crate to cut." It is
**seam-building** (so a future cut is clean), **internal restructuring** (agent-nav
+ compile-time wins without a crate boundary), and **content promotion** (the one
path proven to work — the boss de-naming). Ordered batch below.

---

## The batch (prioritized)

### 1. Move the VFX/SFX message vocabulary DOWN to a foundation crate ⭐ linchpin
**What.** `VfxMessage` / `SfxMessage` (and the small enums they carry) live in
`crate::presentation`/`crate::audio`. Sim systems that only _emit_ a cue
nonetheless depend on the whole presentation module. Move the **message types**
(not the renderers) into a foundation crate — `ambition_effects` already exists as
the home for the effect vocabulary, or a sibling `ambition_vfx`. The renderers
stay up; the sim emits a neutral message.

**Why.** This is the **prerequisite for decoupling presentation** (goals 2+4). Of
the 44 lib files importing presentation, only 4 are message-only today, but
`VfxMessage` is referenced in 26 files — moving it down immediately breaks the
sim→presentation edge for every fire-and-forget cue, and converts the rest into a
small, countable list of "uses a render type directly" sites.

**Effort.** M (mechanical: move 2 enums + update 26 import sites).
**Risk.** Low–M — VFX/SFX are presentation, replay-neutral; a missed import is a
compile error, not a silent bug.
**Validation.** `cargo test -p ambition_sandbox --lib` + `-p ambition_app --test
replay_fixture_regression` (must stay bit-identical) + the visible build.

### 2. Inventory the remaining presentation render-type couplers
**What.** After (1), enumerate the lib files that still import `crate::presentation`
for a **render type** (`FeatureVisual`, sheet specs, etc.). Classify each: (a) can
the type move down with the message, (b) does it need an event seam, (c) is it
genuinely presentation-only and the _importer_ is the thing that should move up.
Write the list into this doc. No code — this is the map for batch 4.

**Why.** Turns "presentation is woven in" into a finite, attackable list (goal 2).
**Effort.** S (analysis). **Risk.** none. **Validation.** n/a (doc).

### 3. Content promotion: dialog + quest + music → `ambition_content`
**What.** `quest` (563), `music` (596), and the **`.yarn` dialog content** (not the
runtime) are game-specific CONTENT sitting in the lib. The de-naming run proved the
install-holder pattern (content installs data into a lib holder with a
`#[cfg(test)]` fixture). Promote these the same way: the lib keeps a generic
quest/dialog/music _runtime_ + holder; `ambition_content` owns the actual quests,
yarn files, and scores.

**Why.** Directly advances the oracle (goal 4) — another platformer swaps these
without touching core. Proven-safe pattern.
**Effort.** M each (dialog is the biggest — the `.yarn` files + binding layer).
**Risk.** M — `dialog` has the lib↔content half-move hazard noted in the endgame
doc (move the _content_, keep the runtime; don't split the yarn list from its
files). Do dialog last.
**Validation.** `cargo test -p ambition_sandbox --lib dialog quest music` +
`-p ambition_content` + replay.

### 4. Presentation move-UP (the 10k-LOC lib shrink) — multi-session endgame
**What.** Once (1)+(2) leave a finite render-type coupler list, move `presentation`
(and its remaining couplers, via the batch-2 plan) **up** into `ambition_app` or a
new `ambition_render` crate that depends on the lib. This is the single biggest
lib-shrink available (~10k LOC, ~11% of the monolith).
**Why.** Goals 1+2+4 all at once: a 10k-LOC module out of the hot incremental-build
unit, and the sim crate becomes presentation-agnostic (truly reusable).
**Effort.** L (multi-session). **Risk.** M — gated entirely on (1)+(2) being done;
do NOT start before the coupler list is empty-or-finite.
**Validation.** full workspace test + replay + visible build + the architecture
guard (add a "sim crate does not import render crate" boundary test).

### 5. Internal god-module concern-splits (ongoing, low-risk agent-nav)
**What.** Continue Refactor 6 but for **production** concerns (not test-mod
extraction, which is now low-value). The biggest production files live in
`features` (13.5k), `presentation` (10.4k), `world` (9.4k). Split by concern behind
`mod` dirs with re-exports (public paths preserved), one file per commit, using the
child-module `pub(super)` + `super::*` pattern from the endgame run.
**Why.** Goals 1+2 (smaller rebuild unit, navigable files) without a crate
boundary. Safe filler between the bigger items.
**Risk.** Low. **Watch-out (learned):** verify the test mod is at EOF before
sweeping `cfg(test)..EOF` — production code can live after it (the `boss_pattern`
trap). **Validation.** `cargo test -p ambition_sandbox --lib` + replay per commit.

### 6. (Deferred / blocked, with reasons)
- **`mechanics/combat` extraction** — 26 outward deps, woven into
  player/interaction/rooms/quest/presentation (~15 inversions). Not a clean
  increment; revisit only after the presentation seam (1–4) thins the graph.
- **Boss / item sprite data de-naming** — render-blind; blocked on the
  presentation render-type seam (batch 2). Pair it with batch 3 once (2) lands.
- **`ambition_sandbox` → engine + content rename (Phase 3)** — the final act;
  needs 1–4 done so the rename reflects a real boundary, not a label.

## Suggested order for the next run
1 → 2 → 3 (quest+music first, dialog last) → 5 as filler → then 4 as its own
multi-session push once 2's coupler list is finite. Each item is independently
shippable and replay-checked; none requires the GUI except the eventual feel-tune
of any gameplay touched (none of 1–5 changes gameplay).
