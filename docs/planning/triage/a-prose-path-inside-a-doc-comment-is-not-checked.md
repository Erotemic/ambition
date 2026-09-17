# A moved directory is a citation broken in silence

**Status:** open row, diagnosis complete and now COUNTED. **Not a proposal to
wire anything.** Filed 2026-09-10; censused and its seven live instances fixed
2026-09-17 (last section).

## The mechanism

`scripts/check_planning_citations.py` reads `docs/`, and inside it resolves both
symbols and paths. ⚠ **THIS SECTION SAID "resolves SYMBOLS" UNTIL 2026-09-17, AND
THAT MISNAMED THE GAP** — writing the census below reddened it on seven quoted
paths, which is the checker doing the job this page said it could not. The
uncovered surface is SOURCE comments, which it never reads at all. A doc comment
that names a **directory** in prose —

    /// `abilities/traversal/{blink,dive,mark_recall}.rs`

— resolves nothing, gates nothing, and reddens nothing when those files move.
The files moved to `crates/ambition_abilities/src/traversal/`. The table in
`ambition_combat::moveset::verdict_belongs_to` kept pointing at the old
location, and **the tree stayed green the whole time**.

⇒ **The citation checker never opens a `.rs` file, so nothing reads the
directory a source sentence names.** A carve or a crate extraction breaks these
silently and in bulk, because the prose that describes *where* a population lives
is exactly the prose a carve invalidates.

## Why it is filed rather than fixed

⛔ **A gate for this is not obviously correct.** Prose paths are also written
about deleted files on purpose, about directories in other repositories, and
about shapes rather than locations (`crates/*/src/rollback_registration.rs`).
A checker that reddens on all of them buys a suppression list, and an amnesty
list is how you hide what it exempts.

⇒ **The row is the deliverable.** If this ever costs someone real time, the
diagnosis is already done and the decision about whether to gate it can be made
with the cost in hand rather than in advance.

## What it cost this time

The table it broke was the `attacker_move_instance: None` population on
`verdict_belongs_to` — the input to sizing A12's remaining work. Re-deriving it
found the count was wrong in **both** directions (13 code sites, not 15: two of
the fifteen were prose) and that its load-bearing column had never been
measured at all.

## The cost is now in hand — censused 2026-09-17

This page said the decision could be made *"with the cost in hand rather than in
advance"*. Here is the cost. Every backticked token ending in `.rs` and
containing a directory, read out of `//`, `///` and `//!` lines across every
tracked Rust file, resolved against `git ls-files` — allowing the repo's own
abbreviation habit, where `actor_monolith/rollback_registration.rs` names
`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs`:

| | count |
|---|---|
| path citations in doc comments | 234 |
| unresolved | 20 |
| of those, deliberate placeholders (`tests/foo.rs`, `src/foo/tests.rs`, `spawn/foo/bar.rs`) | 8 | <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
| of those, SELF-LABELLED historical — the sentence says the path is gone | 5 |
| of those, **stale live claims** | 7 |

⭐⭐ **AND THE GAP IS NARROWER THAN THE TITLE SAYS: PLANNING PROSE IS ALREADY
GATED.** Writing this section reddened `check_planning_citations.py` on its own
seven quoted paths, which is the checker doing exactly the job this page says it
does not do — for `docs/`. The uncovered surface is SOURCE comments, where the
same checker never looks, and that is what the fixes below were found by hand.

⛔ **THE FIRST INSTRUMENT SAID 302 AND WAS MEASURING ITSELF.** Reading every
path-shaped token rather than `.rs` ones counted `MoveLeft/Right/Up/Down`, the
fraction `1/3`, the cargo feature `bevy_utils/debug` and asset paths relative to
an assets root. Narrowing to `.rs` gave 33, and teaching the resolver the
abbreviation habit gave 20. The three numbers differ by what the instrument
thinks a path is, not by what the tree contains.

⭐⭐ **AND THE ARGUMENT AGAINST GATING IS WEAKER THAN IT LOOKED, BECAUSE THE
LEGITIMATE CASES ANNOUNCE THEMSELVES.** This page argued a checker would buy a
suppression list. All five historical citations say so in their own words —
*"that path is GONE"*, *"This was … in"*, *"used to contrast itself with"*, and
one already carries the repo's existing `<!-- cite-ok: … -->` marker. The
placeholders are literally named `foo`. ⇒ A gate would cost FOURTEEN annotations
in a convention that already exists, not an amnesty list — and at HEAD it would be
green the day it landed. That is the number the decision was waiting for; the
decision itself is still nobody's here.

## The seven, and they were fixed rather than counted

Fixed 2026-09-17, because a live sentence pointing at nothing is the defect this
page describes rather than an example of it:

- `ambition_encounter/src/switches.rs` — sent a reader to `encounter/switch_index.rs` <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
  *"in the actor monolith"*; it is `ambition_encounter_features/src/switch_index.rs`,
  a different crate;
- `actor_monolith/src/dev/trace/plugin.rs` and `ldtk/src/bevy_runtime/plugin.rs` —
  both said the simulation phase is *"configured by `app/schedule.rs`"*, a file <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
  that does not exist; the configuration is in
  `actor_monolith/src/schedule/schedule.rs`;
- `ambition_app/src/app/mod.rs` — named `src/rl_sim/runtime.rs`; it is <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
  `src/rl_sim/mod.rs`;
- `content/src/bosses/specials/gradient_sentinel.rs` — said its apple constants
  *"stay co-authored with the legacy path"* in `content/features/bosses.rs`. That <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
  file is gone and this one is now the only definition, so the comment described
  a co-authorship with nothing;
- `ambition_app/src/menu/kaleidoscope_app.rs` — cited
  `crates/ambition_mock_demo/src/app/state.rs`; that crate has zero tracked files; <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
- `actor_monolith/src/avatar/bundles.rs` — `engine_core/body_clusters.rs` is a <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
  module alias, not a directory; the file is
  `ambition_platformer2d_core/src/body_clusters.rs`.

⇒ Five of the seven were produced by a CARVE, which is exactly the mechanism at
the top of this page, now with instances instead of one.
