# A moved directory is a citation broken in silence

**Status:** ✔ **CLOSED 2026-09-17.** Filed 2026-09-10 as a diagnosis that
deliberately declined to wire anything. It is wired now, because the objection
that stopped it — *"a checker that reddens on all of them buys a suppression
list"* — was measured and did not survive: the legitimate cases announce
themselves, the annotation cost is fourteen lines in a convention that already
existed, and the gate is green at HEAD. Ten live citations were repaired on the
way (last two sections).

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

## What closed it

`scripts/check_planning_citations.py --comment-paths`, registered in
`scripts/run_tests.py` as *"a path named in a source comment exists (reports,
does not gate)"* and running in the default plan (`slow_python_checker_jobs`, so
`--rust` and `--maintenance` drop it and say so).

⛔⛤ **IT WAS REGISTERED `--strict` AND DEMOTED THE SAME DAY.** A review named
`AGENTS.md`'s *"Avoid bullshit guardrails"*, which rules out permanent
source-text and file-location machinery *"unless it prevents a concrete,
recurring, materially harmful failure that cannot be enforced more naturally
through Rust types, APIs, crate boundaries, or behavioral tests."* The failure
here is concrete and recurring — seven live comments pointing at nothing — and
it is not materially harmful: a stale path misleads a reader and breaks nothing.
⇒ **The census was worth running and the gate was not.** What this row closed is
the question *"is anybody checking?"*, answered once, with the seven fixed; a
permanent gate would have been the repository paying rent on that answer
forever. The escape hatch it needed (`cite-ok`, for a path named BECAUSE it is
gone) is itself the tell: a check that has to argue with honest prose.

⛔ **ONE RESOLVER, NOT A SECOND SCRIPT.** `path_resolves` already knew the
repository's abbreviation habit and `cite-ok` already meant *"wrong on purpose"*;
a sibling checker would have been a second authority on the same question, which
is the thing this tree spends most of its hygiene budget undoing.

⚠ **AND IT IS A SEPARATE FLAG FROM `--comments` BECAUSE THE TWO CAN BE WRONG IN
DIFFERENT WAYS.** A path either exists or it does not; a symbol citation can name
something the checker cannot see, and the lane's non-strict citation job argues
that case in its own words — *"failing the lane on one would train everybody to
pass `--no-verify`."* `--comments` still implies `--comment-paths`; only the
reverse is untrue.

⭐⭐ **THE SYMBOL HALF WAS CLEARED THE SAME DAY, WHICH WAS NOT THE PLAN.** It
reported 21 findings and the first reading of that number was "a backlog that
stops it gating". Read one at a time, FIVE were live and repointable — a
`CharacterDefinition::autonomous_profile_ref` that was merged into <!-- cite-ok: this line RECORDS a citation that does not resolve; that is the finding -->
`autonomous_policy` (cited as a live contract in three places), a
`RoomConstructionPlan::retire_outgoing` that does not exist (the room sweep is <!-- cite-ok: this line RECORDS a citation that does not resolve; that is the finding -->
`room_transition::commit` over the `RoomResident` roster), and an
`integration::apply_gravity` that never existed under that name. The other <!-- cite-ok: this line RECORDS a citation that does not resolve; that is the finding -->
sixteen are comments RECORDING a retired authority on purpose and now say so with
`cite-ok`. ⇒ `--comments --strict` exits 0 at HEAD too. What keeps it advisory is
the CLASS of mistake it can make — `World::iter_entities` is an upstream method <!-- cite-ok: this line RECORDS a citation that does not resolve; that is the finding -->
the checker only judges because this repo also defines a `World` — not a
backlog.

⇒ Poison-verified: repointing one live citation at `src/rl_sim/nowhere.rs` <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
reddens it naming the file, the line and the token. Five arms in
`scripts/tests/test_source_comment_path_citations.py` pin the reporting, the
control, the `cite-ok` escape, the symbol/path split, and that a path inside a
string LITERAL is data rather than a citation.

⭐ **THE SWEEP FOUND THREE MORE THAN THE `.rs`-ONLY CENSUS ABOVE**, because the
checker's suffix list is wider: a guard named
`scripts/tests/test_no_test_module_is_dark.py` that has never been tracked under <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
that name, a `tools/ldtk_intgrid_migration.py` named as *"the source of truth for <!-- cite-ok: this line RECORDS a path that does not resolve; that is the finding -->
which value means what"*, and a Mockingbird sprite generator cited as the thing
you install to regenerate a shipped sheet. All three were live instructions to a
reader, and all three are now either repointed or labelled as gone.

## The adjacent gap, MEASURED AND DELIBERATELY NOT WIRED — 2026-09-19

⛔⛤ **A CITATION THAT RESOLVES IS NOT A CITATION THAT LANDS.** The checker above
verifies that `path:N` names a real file and a real line. It cannot ask whether
line `N` is the thing the surrounding sentence is about. Two live examples found
the same day, in `consolidation/README.md` and `consolidation-plan.md`: they
cited two arms of one test file at `:1404` and `:519` while those arms declare
at `:1420` and `:535`. Both resolved. `:1404` is an `assert!` inside an
unrelated arm and `:519` is a doc comment naming the OTHER arm.

⭐ **THE SIGNATURE IS WHAT MAKES IT DIAGNOSABLE: BOTH WERE OFF BY EXACTLY THE
SAME SIXTEEN.** One file gained sixteen lines above both citations, so every
line citation into it aged together. ⇒ **When one line citation into a file is
found wrong, check the others into the same file and diff the offsets.** An
identical offset is one edit, not two mistakes, and it predicts the rest.

⛔ **AND THE OBVIOUS GUARD WAS BUILT, MEASURED, AND REJECTED — WHICH IS THE
FINDING THIS SECTION EXISTS FOR.** The rule "a citation naming `some_fn` must
land within a few lines of `fn some_fn(`" flags **42** citations across `docs/`.
Reading them, the rule is wrong rather than the corpus: **deep-linking into a
body or a doc comment is deliberate and common here**, and it is usually the
more useful citation.

- `crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs:1078`
  points at the comment explaining why advance is an EDGE — 16 lines inside the
  function the prose names, on purpose. ⚠ That path is spelled in full because
  the first draft of this bullet wrote the bare `input_systems.rs:1078` and the
  checker reddened it: TWO tracked files carry that suffix. <!-- cite-ok: the bare spelling above is QUOTED as the mistake this bullet records --> The gap this
  section describes is real, and the checker still caught the writing of it.
- `held_items/src/lib.rs:996`, `:1016` is a PAIR of citations for a pair of
  functions; a scanner reading only the first call it a miss.
- `architecture-census.md:420` names five `propose_*`/`publish_*` systems in
  one sentence and cites the site of a sixth.

⇒ **Do not wire this rule.** The false positives are the convention working.
What is worth checking is the narrower thing the real defect had: not "does the
citation land on a declaration", but "do several citations into one file share
an offset". That is a much smaller population and it is the shape a file edit
actually produces. Nobody has built it; this section is the record that the
wider version was tried and costs more than it finds.

## A third gap on the same axis, measured and not wired — 2026-09-19

⛔⛤ **A QUOTATION OF SOURCE IS A CITATION NOTHING CHECKS AT ALL.** The sections
above are about a path or a symbol that stops resolving. A guard's WAIVER can
also quote a source comment verbatim to justify itself, and that quotation is
plain prose inside a Python string — no resolver looks at it.

Found live: `scripts/check_rollback_mutators_run_in_sim.py` waived
`restore_inventory_from_save` as *"WAIVED FOR THE ACTIVATION CASE ONLY, AND THE
OTHER CASE IS OPEN"*, and its reason quoted `durable_horizon.rs` saying *"THE
`Update` ADOPTER STAYS. A file can also arrive after activation (a mid-session
load), and adoption is idempotent"*. That comment had been deleted when Q135
landed on 2026-09-16. Three distinct fragments of it return **zero** hits across
every `.rs` file in `crates/` and `game/`. ⇒ The waiver spent three days
justifying itself with text that no longer existed, and the road it described
was gone.

⭐ **THE EXISTING ARM CANNOT SEE IT, AND ITS NAME SAYS OTHERWISE.**
`test_every_waiver_cites_the_code_that_makes_it_true` accepts any reason
containing `⛔` and at least 150 characters. That is a SHAPE check wearing a
citation check's name — it cannot distinguish a waiver that cites live code from
one that cites deleted code at length.

⛔ **AND BOTH OBVIOUS GUARDS WERE MEASURED AND REJECTED.**

| candidate rule | measured | why not |
|---|---|---|
| every backticked identifier in a waiver must exist in the Rust tree | 73 identifiers across 15 waivers, **0** unresolvable | green by construction — and it would NOT have caught this one, whose only backticked token was `Update` |
| every quoted phrase in a waiver must appear in source | **5** matches, of which **3** are apostrophes in possessives (*"`X`'s own doc says"*) and 2 are real quotations | the population is two; the regex cannot tell a quotation from an apostrophe without more machinery than the finding is worth |

⇒ **Do not wire either.** The second real quotation — `teardown.rs:375`'s
*"HYGIENE, NOT CORRECTNESS"*, in the `reset_session_scoped_resources_on_retire`
waiver — resolves, so after the repair the population is clean and a gate would
protect two sentences.

⭐ **WHAT IS WORTH KEEPING IS THE HABIT, NOT THE CHECKER: when a ruling lands,
walk its inbound links.** Q135 was answered on 2026-09-16 and left three stale
owners behind — this waiver, the `DURABLE-HORIZON-CHECKSUM` queue row calling it
*"the half that is still open"*, and its own page describing an `#[ignore]`
reason that had changed. All three were found by re-reading one row against the
tree, and none of them by any guard.
