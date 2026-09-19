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

### The same sweep over `docs/planning`, run 2026-09-19

Having found it in a guard's waiver, the obvious next question is how often
planning prose quotes source that has moved. Measured over every `*"…"*` span
of 30+ characters in `docs/planning`, normalised for backticks, comment
markers, blockquote and list prefixes and markdown links, against every `.rs`
file in `crates/` and `game/`:

| population | count |
|---|---|
| quotations of 30+ characters | 783 |
| of those, ATTRIBUTED to a `.rs` file within 90 characters before the quote | 32 |
| of those, not found in the tree on the first pass | 16 |
| after fixing the matcher (`…` elisions, blockquote and list markers) | 12 |
| of those, the triage record above quoting deleted text ON PURPOSE | 3 |
| of those, a document quoting ITSELF, not a file | 1 |
| **genuine defects, each read individually** | **4** |

The four, all repaired 2026-09-19:

- `queue.md` printed *"capture resolves the semantic DIRECTION and simulation
  never sees a mode at all, at which point this waiver and the row above both
  shrink"* as a quotation of `rollback_coverage.rs`. The file states the claim
  in INDIRECT speech, and the trailing clause is in no file in the repository.
- `world-facts-observations-and-memory.md` quoted `authored_conditions.rs` in
  two sentences. The second is verbatim; the first was a paraphrase, and it
  read as the stronger claim — *"must be queried there"* where the module says
  a second copy is not NEEDED. The paragraph used it as evidence that the
  module *"says so at the site"*.
- `actor-monolith-work-frontier.md` substituted *"the outgoing sweep"* into an
  otherwise verbatim quotation of `room_transition/commit.rs`.
- `commit.rs` itself, plus four more source comments, named a
  `retire_outgoing` that A10 deleted on 2026-09-14 — see below. <!-- cite-ok: names the method A10 DELETED on 2026-09-14; a resolvable citation here would mean the deletion did not happen -->

⛔⛤ **THE WIDER POPULATION IS NOT A BACKLOG, AND SAYING SO TOOK A SAMPLE
RATHER THAN AN ASSUMPTION.** Dropping the attribution filter, 262 of 650
quotations resolve to no original prose anywhere. Reading eight at random:
five are the corpus's own *"this sentence used to say X"* convention, where
the quoted text is absent from every original BECAUSE it was deleted — that is
the convention working; one quotes the maintainer speaking; one quotes a
planning page by a former name; and one was a false negative. ⇒ **The
attribution is what makes a miss meaningful.** Without it the rule measures
the repository's habit of quoting its own history.

⚠ **TWO FALSE-NEGATIVE CLASSES, RECORDED BECAUSE THEY COST TIME BOTH WAYS.** A
Rust string continuation (`\` at end of line, then indentation) joins into
text no naive normaliser reproduces — that is why the
`NarrativeInputLedger` waiver first read as unresolved and is in fact verbatim.
And `…` is not `...`: the first matcher treated only the ASCII form as an
elision and reported four misses that were quotations with a gap in them.

⇒ **Still not worth a gate**, for the reason the section above gives: the
population that a checker can judge is 32, the convention it would argue with
is 650 wide, and the repair here was four sentences.

### The deletion that left five survivors — `retire_outgoing` <!-- cite-ok: names the method A10 DELETED on 2026-09-14; a resolvable citation here would mean the deletion did not happen -->

⛔⛤ **A10 DELETED `RoomConstructionPlan::retire_outgoing` ON 2026-09-14, ONE <!-- cite-ok: names the method A10 DELETED on 2026-09-14; a resolvable citation here would mean the deletion did not happen -->
COMMENT WAS CORRECTED ON 2026-09-17, AND FIVE WERE NOT.** The name has no
definition anywhere in the workspace; every hit is prose. `spawn_ext.rs`
carries the 2026-09-17 correction — *"This named a
`RoomConstructionPlan::retire_outgoing` until 2026-09-17; no such method <!-- cite-ok: names the method A10 DELETED on 2026-09-14; a resolvable citation here would mean the deletion did not happen -->
exists"* — and the other five went on naming it as a live mechanism in
`room_transition/commit.rs`, `construction/tests.rs`, `session/reset/mod.rs`,
`session/reset/tests.rs` and `carried_item_crosses_rooms.rs`. ⇒ **A repair
applied to the owner a search happened to surface is not a repair of the
fact.** The three planning pages that name it were already `cite-ok`-marked as
recording a dead name on purpose, so the docs were clean and the source was
not — the opposite of the direction this page was written to watch.

⛔⛤ **AND TWO OF THE FIVE REPAIRS WERE WRONG ON THE FIRST DRAFT, IN THE WAY
THIS CORPUS KEEPS FINDING.** A dead name has to be replaced by the RIGHT live
owner, and there are two different room sweeps here:

| | roster | so |
|---|---|---|
| the room TRANSITION sweep (`replace_live_world`) | `RoomResident` = `(With<RoomScopedEntity>, Without<InCustodyOf>)` | a carried object rides across |
| the NEW GAME reset sweep (`process_new_game_reset_request`) | `With<RoomScopedEntity>`, deliberately NOT `RoomResident` | the hand is emptied, so nothing is exempted |

Both `session/reset` sites were first repointed at the transition sweep because
the dead name looked like a transition. Reading the paragraph instead of the
name showed each is about the RESET, whose own parameter list says *"the two
sweeps ask different questions; unifying them is not the cleanup it looks"*.
⇒ One of them claimed the sweep is unconditional, which is TRUE of the reset
and FALSE of the transition: the first draft would have replaced a correct
sentence with an incorrect one while removing a dead name.

### What the extended differential reports, and what that number is

Run at the lane's own baseline — `PLANNING_VANISHED_BASELINE` in
`scripts/run_tests.py`, dated 2026-08-13, which predates the 2026-09-14
deletion, so the instrument could always have seen this:

⛔ **EVERY NUMBER IN THIS SECTION IS AT THAT 2026-08-13 REFERENCE POINT, AND THE
LANE NO LONGER USES IT** — the constant was advanced to the epoch root on
2026-09-19 (see the closing section). They are kept as measured rather than
restated, because re-running them against a different baseline would produce
different numbers about a different question.

⛔⛤ **AND WRITING THAT SHA HERE FAILED A GUARD, WHICH TURNED OUT TO BE ABOUT
THE LANE AND NOT ABOUT THE SENTENCE.** `test_no_unresolvable_citation_that_the_
epoch_did_not_grandfather` reddened on the abbreviation. The object resolves in
this checkout, so `git log` answers and the periodic vanished job runs green —
but the commit is dated 2026-08-13 and the git epoch starts at `b924f419c`
(2026-09-06), so it is PRE-EPOCH: unreachable from any ref here and readable
only in the `ambition-history` store. ⇒ **The lane's own baseline is a commit a
fresh clone cannot read**, and the job depends on a loose object that survived
the truncation on this machine. That is the shape the citation checker's own
comment warns about — *"it exists here, so this checker said RESOLVED … on the
fighter lane's machine the object does not exist"* — one layer up, in the
BASELINE rather than in a citation. Filed below rather than fixed, because
choosing a new baseline is a judgement about what window the sweep should
cover.

| | count |
|---|---|
| bare citations in source comments naming a name that vanished since the baseline | 246 |
| distinct vanished names they cite | 109 |
| of those citations, self-labelled HISTORICAL on their own line | 59 |
| reading as a LIVE claim | **187** |

⛔⛤ **THAT 187 WAS AN UPPER BOUND AND THE UNIT WAS WRONG — RE-MEASURED THE
SAME DAY, AND THE REAL RESIDUE IS 65.** The first classifier read the
citation's OWN line. A paragraph routinely self-labels a line or two above the
name: `transaction.rs`'s *"THIS COMMENT USED TO SAY … ALL FOUR ARE CLOSED"*
sits three lines above the `commit_deferred` it lists. <!-- cite-ok: names a symbol DELETED since the baseline; this row is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen --> Re-run over the COMMENT
BLOCK the citation lives in — the run of `//` lines it belongs to:

| | line-local | comment block |
|---|--:|--:|
| self-labels as historical | 59 | **178** |
| residue reading as a LIVE claim | 187 | **65** |

⇒ **Three times fewer, from reading the unit this corpus actually writes in** —
the same mistake that made the blocking set wrong twice in one day, and here it
is the difference between an unusable backlog and a slice somebody can finish.

⭐ The 178 are not repointing work. Sampled, they are explicit deletion
records: `yarn_vocabulary.rs` alone carries eight, every one saying *"USED TO
BE HERE"*, *"lived here and are gone"* or *"died too"*. They want the `cite-ok`
the convention already has.

⇒ **EIGHTEEN OF THE 65 WERE REPAIRED THE SAME DAY**, each verified against
what the cited system actually calls: `CANCEL_CLASS_NAMES` became the derived <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
function `cancel_class_names`; `damage_apply` moved to its own crate as <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`ambition_damage::resolve_body_hit`; `attach_mount_role` gained a `_from`; <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`load_encounter_specs_from_ldtk` became `…_from_rooms` AND changed its input to <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`&[RoomSpec]`; and `body_is_corpse` split — four roads call the free <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`body_is_untouchable` while `apply_hitbox_damage` asks the METHOD `is_corpse`,
a distinction one find-and-replace would have erased.

⚠ **AND THE RESIDUE HAS A FALSE-POSITIVE CLASS OF ITS OWN: A DATA KEY THAT
ONCE SHARED A NAME WITH AN ITEM.** `dash_pressed` is a RECORDED key in the <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
gameplay-trace dump, kept stable ON PURPOSE — *"the RECORDED key stays
`dash_pressed` so a dump written after the rename still replays"* — and it <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
flags only because a Rust item of that spelling existed at the baseline. A
wire or trace key is not a citation, and a checker cannot tell them apart from
the token alone. ⇒ Another reason this pass reports rather than gates.

⛔ **WHAT IS LEFT IS ONE MIGRATION, NOT A LIST.** `ArchetypeSpec`, <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`spec_for_brain`, `CharacterRoster`, `CharacterRosterFragment` and <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`PlayableKitSource` are the same dead CHARACTER-ARCHETYPE family and want <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
reading as a whole. ⚠ And that family is where a rename hides a semantic
change: `CharacterRoster::spec_for_brain` *"has no failure mode — an unknown
key falls back to the combatant row"*, while its successor
`CharacterCatalog::get` returns an `Option`. Those comments are substantively
WRONG, not misnamed, which is the same trap the two `session/reset` sites set
above — one layer deeper.

⭐ **AND TWO OF THAT FAMILY ARE RUSTDOC INTRA-DOC LINKS, WHICH MEANS A THIRD
INSTRUMENT ALREADY WARNS AND NOBODY READS IT.**
`state_machine/mod.rs:66` links `[`tick_state_machine_with_actions`]` and <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`sim_core_resources.rs:12` links `[`CharacterRoster`]`; neither target exists, <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
so `cargo doc` emits a broken-intra-doc-link warning for each. ⚠ A wider
census of that shape is NOT a finding — 3858 links, 104 targets this scan
cannot resolve, and the residue is dominated by upstream types (`App`,
`Default`, `SystemParam`), crate names and struct FIELDS, none of which a
token-level scan can tell from a defect. The narrow statement is the useful
one: rustdoc owns this check already, and these two are its live output.

⚠ **246 AND THE POST-REPAIR COUNT ARE ONE INSTRUMENT AT TWO REFERENCE POINTS,
NOT A DISAGREEMENT.** 246 is BEFORE any of this page's repairs;
after them the same run reports **223 across 104**. ⭐ And three of those five STILL
reported until they were marked, because each now RECORDS the dead name in the
very sentence that corrects it — **a repair that explains itself re-enters the
population it just left.** That is what `cite-ok` is for, and it is the
clearest demonstration that this pass now reads source at all.

⭐ **AND THE RUN CONFIRMS THE HAND CENSUS INDEPENDENTLY**: it names
`retire_outgoing` <!-- cite-ok: names the method A10 DELETED on 2026-09-14; a resolvable citation here would mean the deletion did not happen -->
exactly five times, which is the five this page found by reading.

⇒ **The five biggest clusters are the next slice, not this one.**
`ArchetypeSpec` (13), `damage_apply` (13), `RoomTransitionRequested` (13), <!-- cite-ok: names a symbol DELETED since the baseline; this row is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`spec_for_brain` (8) and `CharacterRoster` (6) have no definition in the tree <!-- cite-ok: names a symbol DELETED since the baseline; this row is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
and 53 live mentions between them. Sampled, they split the same way the path
census above did: `RoomTransitionRequested` reads as deliberate history <!-- cite-ok: names a symbol DELETED since the baseline; this row is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
(*"this was a … message"*), while the other two read as present-tense
descriptions of live mechanisms.

## One of those clusters, worked — 2026-09-19

`PlayableKitSource` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
was an enum with exactly ONE variant, `HostCode`, and `prepared.rs` deleted it
once that made it a membership test wearing a type's clothes. **Twenty-one
comment sites kept describing the world it named**, all on one road
(`avatar/starting_character*`, `avatar/bundles.rs`, and the catalog row itself).
All twenty-one are repaired; the five mentions that remain each narrate the
deletion in the past tense, which is the correct end state rather than a
residue.

⭐ **THE SHAPE IS NOT "STALE PROSE". IT IS A FIX THAT LANDED AS A NEW PARAGRAPH
BESIDE THE OLD ONE INSTEAD OF REPLACING IT** — three times, and in all three the
correction and the claim it refutes were close enough to read in one screen:

| the uncorrected claim | the correction that was already sitting beside it |
|---|---|
| `character_catalog.ron:383` — *"the protagonist's PLAYABLE kit is host-code-owned … not this catalog row"* | six lines later, same comment block: *"v3 states its own repertoire … on its definition"* |
| `starting_character/tests.rs:533` — *"the code kit (Swipe + Bolt + bubble_shield from sandbox_all abilities) is rebuilt"* | 25 lines later, same function: `assert_eq!(*set, ActionSet::peaceful())` |
| `starting_character.rs:244` and `:336` — the rule stated twice in the dead vocabulary | `:583`, same file: *"`HostCode` was the other half of this condition and no longer exists"* <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen --> |

⇒ Both halves were written by somebody who knew, and neither reader could tell
which half was current. A census that looks only for names with no definition
finds the first column and never notices that the answer is already in the file.

⛔ **AND THE WORST SITE WAS AN INSTRUCTION, NOT A DESCRIPTION.**
`starting_character.rs:133` told an author that a protagonist opts its ROW into
`PlayableKitSource::HostCode` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen --> —
a route that cannot be taken, reading as a supported one. The live rule has no
row-side switch at all: a body is rebuilt from its persisted `AbilitySet`
exactly when the catalog does not know its id, and `resolve_playable_action_set`
owns that sentence now.

⭐ **A THIRD BROKEN RUSTDOC LINK IN THIS FAMILY, AND THIS ONE IS MEASURED ON BOTH
SIDES.** `bundles.rs:256` linked
`ambition_characters::actor::character_catalog::PlayableKitSource::HostCode` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
from the doc of a `pub fn` on the spawn path. At HEAD `504a6c152`, `cargo doc`
emits *"unresolved link … no item named `PlayableKitSource` in module <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`character_catalog`"*; after the repair it emits nothing for that file. ⇒ With
`state_machine/mod.rs:66` and `sim_core_resources.rs:12`, **rustdoc has been
reporting this class the whole time and the warning is buried in 68 others** in
these two crates alone — which is why a third instrument existing is not the
same as the class being watched.

⚠ **ONE REPAIR WAS A TEST NAME, AND THE NAME WAS THE CLAIM.**
`host_code_kit_refreshes_when_body_abilities_change` asserted a row type that <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
cannot exist; the body wears `"player"`, which no catalog row defines, so it was
covering the UNKNOWN-ID branch throughout. Renamed to
`an_unknown_ids_kit_refreshes_when_body_abilities_change`, and the load-bearing
fixture property — that the id is absent from the SHIPPED catalog
`install_test_catalog` installs — is now stated, because nothing else in the
file shows it. Poisoned by wearing `"goblin"` instead: the arm fails on
`initial.ranged`, so the branch really is selected by the id's absence.

⚠ The repair also had to answer a question the old prose hid: the ability-only
refresh gates on `!catalog.knows(id)` rather than on the prepared kit, so the
two could in principle disagree. They cannot — the only other route to
`PreparedKit::Unauthored` is preparation with no catalog, and
`apply_worn_character_gameplay` takes `Res<CharacterCatalog>` unconditionally
and so does not run in a composition that has none. That is now written at the
gate instead of being re-derived by whoever next reads it.

## The baseline that made all of it weaker — repaired 2026-09-19

The section above deferred this: *"picking a new baseline decides what window
the sweep covers."* Two measurements made the decision instead.

**It was reachable from nothing.** The old constant — the 2026-08-13 SHA the
section above deliberately does not spell, because writing it reddens the
commit-citation guard and that redness IS this finding —
is an ancestor of no ref in this repository — `git merge-base --is-ancestor`
says so against HEAD and against every `for-each-ref` entry. It survived here
only as a dangling object in the checkout that wrote it.
`check_planning_citations.py` exits 1 when it cannot resolve the ref, so
maintenance job 47 was **red on every clone but this one** and nobody could
see it from here. The guard on the constant asserted it was a full 40-hex SHA
and never that git could reach it.

⚠ That is the hazard
`test_no_unresolvable_citation_that_the_epoch_did_not_grandfather` already
documents at length — *"a commit can exist locally and be reachable from
nothing"*, which is why it was **green on one machine and red on another from
the same source**. The repository knew this failure mode, wrote it down inside
the guard for it, and the instance in the LANE'S OWN CONFIGURATION was outside
every population anything checked.

**And it was reporting nothing.** Over the five doc trees job 47 actually
passes: **0 findings at the old ref, 2 at the epoch root.** A baseline can only
see a name that was DEFINED at it, so a baseline that predates a deletion is
blind to that deletion — which is exactly why the 21-site
`PlayableKitSource` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
family above survived five weeks: the variant was gone before 2026-08-13, and
the baseline's own enum body at that commit already held only `Authored`.

⇒ **Moving the baseline FORWARD widens what the sweep can see.** That is the
opposite of the intuition the constant's comment is built on, and it is the
whole reason the deferral was wrong.

### What the sweep became

| | old ref (2026-08-13) | epoch root (2026-09-06) |
|---|---|---|
| doc trees, `--strict` | 0 findings, job green | 2 findings, both real, both fixed |
| source comments, `--comments` | 222 across 103 names | **54 across 28 names** |
| can a fresh clone run it | no — exits 1 | yes |
| can `git log` attribute a finding | no, the deletion predates the graph | yes |

The last row is the one that changes the work. Sampled four of the new
findings and every one names the commit that made it stale, with a subject
that says what that commit was doing: `spawn_world_for` → `2a1fc35fc` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
(2026-09-16, *"One mint for the session root: the unreachable primitive is
gone"*), `activate_prepared_platformer_sessions` → `c89c68747` (2026-09-15), <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`setup_simulation_system` → `d3135def0` (2026-09-06, *"A false ordering claim <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
was hiding a dead system and two dead SystemParams"*),
`CheckpointResumeProgress` → `7102674a7` (2026-09-08). <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->

⛔⛤ **AND THE FIRST TWO OF THOSE FOUR WERE ATTRIBUTED TO THE WRONG COMMIT,
BY THIS PAGE, IN THE SAME CLASS IT IS ABOUT.** `git log -S<name>` searches
every tracked line, COMMENTS INCLUDED, so it answers "who last edited the
count of this string" — which for a deleted symbol is usually whoever last
wrote prose about the deletion. It named `172816069` and `62ecde029`, both of
which only touched comments mentioning the name. ⇒ **Search the DEFINITION
form**: `git log -S"fn <name>"` or `-S"struct <name>"` reaches the removal
itself. A bare-name query cannot tell a definition from a sentence, which is
the same confusion the `--comments` pass exists to surface.

⇒ The 222 was a haystack dominated by prose deliberately recording old names,
because a three-month-old window catches every rename the corpus ever narrated.
54 recent carves, each with a dated commit and a subject line to read, is a
list somebody can work. ⭐ `setup_simulation_system` is in it — one of the two <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
names the previous slice could not map at all, now attributable in one command.

⚠ **The reachability guard is what makes this stick.**
`test_the_vanished_baseline_is_a_commit_git_can_actually_reach` asserts
ancestry rather than shape, because existence is precisely the test that passed
on the machine holding the object. Poisoned with the old SHA: it fails naming
that SHA, and the three sibling arms stay green.

## The first sweep at the new baseline, worked the same day

54 findings, 28 names. Classified by the COMMENT BLOCK rather than the line —
the unit that carries the meaning, as the line-vs-block section above measured:
**48 deliberate history, 4 live.** The largest cluster is entirely clean:
`spawn_world_for` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
has 6 sites and all 6 narrate its deletion, including a doc comment on
`adopt_world` that exists to explain why the primitive went. ⇒ **A cluster
being big is not evidence it is rotten**, and a count of findings is not a
count of work.

All three remaining `setup_simulation_system` sites were live, and it traces to <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
`d3135def0` — *"A false ordering claim was hiding a dead system and two dead
SystemParams."* The system had never been registered, so the `.after()` edge
that appeared to order the app's `Startup` chain against it was a claim rather
than a constraint; removing it let rustc name ~100 more lines of dead code.

⛔⛤ **AND THE MARK OUTLIVED THE STAGE.** The app's Startup chain still emitted
`phase_mark("after_setup_simulation")` between two adjacent chained systems
with nothing between them, so every boot printed a stage that does not exist —
and [`../../recipes/profiling.md`](../../recipes/profiling.md) reproduces it as
the example output at **+312.7ms of a 412.5ms startup**, the dominant cost. A
reader optimising startup went looking for a function the tree does not
contain. Renamed to the slot that IS there (`SimulationSetupSet`, which the
demo fixtures fill and this composition does not), and the recipe now says the
figure predates the deletion and must be re-measured before anyone acts on it.

### The repair that would have made a false sentence read true

`sim_core_resources.rs:12` described *"the content-free `CharacterRoster` <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
default below"* as *"an explicit authority resource for Apps with no hostile
provider"* that *"provider registration replaces transactionally"*. The obvious
repair is the rename the rest of this page is made of —
`CharacterRoster` → `CharacterCatalog`. **It would have been wrong.** <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->

This module initialises no cast resource of any spelling; the paragraph two
lines above already lists the character catalog among what the engine group
does NOT provide, so the comment contradicted itself; and the policy the
sentence states is the one the architecture REFUSED. `prepare_declared_cast`
records it: *"Not `insert_resource(CharacterCatalog)`: that would be a second
authority on what the cast is, and fragments MERGE, which is what lets several
experiences coexist in one composition at all."*

⇒ **A dead name is a symptom and the sentence around it is the subject.**
Renaming would have left a fluent, checkable, wrong paragraph that no future
sweep could ever flag again — strictly worse than the stale name, which at
least announced itself. This is the third time on this page that reading the
paragraph instead of the token changed the repair, after the two
`session/reset` sites and the `player_robot_v3` re-wear.

### What the instrument cannot see, measured

`BARE` requires backticks on both sides, so an unbackticked name is invisible.
Censused across all 28 vanished names: **6 unbackticked comment mentions, of
which 4 are the English verb "unbind" and 2 are real.** One was a live defect
(`resources.rs` justified an `insert_resource` by naming the deleted system as
its reader); the other was a citation WRAPPED ACROSS A LINE BREAK, so each
backtick sat on a different line and neither the opening nor the closing pair
matched. Both are repaired and the second is rewrapped so the pass can see it.

⇒ A recorded negative: the missing-backticks blind spot is real but tiny, and
not worth loosening the pattern for — a looser one would match every ordinary
word. The line-wrap case is the one worth knowing, because it looks correct in
the source and is invisible to the check.

### The comment block is too coarse, and today is why — 2026-09-19

The line-vs-block section above replaced the LINE with the BLOCK as the unit,
and that was right: 187 line-local "live" claims were 65 once the block was
read. Running the same classifier over the new baseline's 54 findings said
**0 live** after the four repairs. Splitting the block into SENTENCES and
asking only about the sentence that actually names the symbol said **19**.

⛔⛤ **THE BLOCK CANNOT WORK, AND THE REASON IS THIS PAGE'S OWN HEADLINE.** A
block is classified as history the moment it contains one history word — and
the word arrives exactly when somebody adds the CORRECTING sentence. So the
block-level filter goes quiet at the precise moment the file starts holding
both answers, which is the state all three of today's findings were in:

- `character_catalog.ron` — *"AUTHORED since 2026-08-11"* six lines under
  *"the protagonist's PLAYABLE kit is host-code-owned"*.
- `authored_movesets.rs` — *"THE NAMESPACE IS WIDER THAN THE OLD
  `CANCEL_CLASS_NAMES` CONST"* <!-- cite-ok: names a symbol DELETED since the baseline; this section is the census OF those deletions, so a resolvable citation here would mean the deletion did not happen -->
  25 lines under a doc comment introducing the guard with that const as the
  definition.
- `starting_character/tests.rs` — the assertion that refutes the comment, in
  the same function.

⇒ **A history marker anywhere in the block is evidence that somebody noticed,
not evidence that they finished.** Of the 19, six were real and are repaired:
`an_edit_reaches_the_shipped_game.rs` (the same false sentence `reload.rs`
carried — one fact, two owners, so the correction had to be made twice),
`authored_movesets.rs`, `entry.rs` (a pointer into `actor_monolith` that the
F1 cut moved to another crate entirely), `rollback_lifecycle_reset.rs` (*"the
memory is X now"* naming what A1c/5 deleted), and both copies of a recorded
measurement in `one_body_two_tickers.rs` whose phase labels the instrument no
longer emits — a reader re-running it could not have matched the record to its
own output.

⚠ The other thirteen are genuine history that the sentence split reads wrong,
mostly long sentences whose tense sits before the clause naming the symbol. ⇒
Neither unit is a classifier. The block is the right unit for READING and the
sentence is the right unit for TRIAGING, and the residue is small enough to
read by hand — which is the actual answer, not a better regex.

## A THIRD way a comment stops describing its subject: it loses the subject — 2026-09-19

Everything above is about a comment whose CONTENT went stale. This one is about
a comment whose ATTACHMENT did.

```
/// Install an already-built sync-test session as the new frame-zero baseline.
...three paragraphs of rationale, including the 2026-09-17 review's...
/// Warn when frame zero has no constructed session world:
fn warn_if_no_world_to_rewind(world: &World) {
```

Rust attaches both runs to the item below. So the install function's entire
rationale — the carefully-written *"this sentence has now been wrong twice"*
paragraph — rendered under an unrelated warning helper four hundred lines from
the function it is about, while `install_rebased_sync_test_session` had no
summary line at all. **Two owners for one fact, and the better-written one was
attached to nothing that could contradict it.**

⭐ **AND IT HAS A ONE-LINE SIGNATURE.** A `///` run, then a genuinely BLANK
line, then another `///` run, with no item between: the first run is stranded
and rustdoc silently prepends it to the second's item, so the stranded summary
becomes the item's summary. Censused tree-wide over `git ls-files '*.rs'`:
**25 instances.** Sampled four blind before repairing any — `resolve_attack_intent`'s
parameter semantics parked on a struct, a demo roster doc on a sheet
registrar, *"The scripted stick."* on `fn step`, and a HUD publisher's whole
rationale 270 lines from it — and **4 of 4 were real.** Seven are repaired,
eighteen remain.

⚠ The repairs split three ways, which is why this is not a mechanical rewrite:
some MOVE (the parameter semantics are still true and still undocumented on the
live function), some DELETE (`character_sprites/assets.rs` described probing
and gating that `load_character_sprites_in`'s own doc now explicitly contradicts
— *"WITHOUT decoding any of it"*), and one was simply a botched edit —
`body_seed/src/lib.rs` carried `/// Convert an authored LDtk actor rectangle}`,
truncated mid-sentence with a stray brace, directly above the complete version
of the same sentence.

⛔ **NOT WIRED AS A CHECK, DELIBERATELY.** `AGENTS.md` line 737: source-text
machinery needs a *"concrete, recurring, materially harmful failure that cannot
be enforced more naturally"*, and *"prefer testing real behavior over names,
phrases, file locations, exact symbols."* Eighteen wrong rustdoc summaries is
recurring and concrete but not materially harmful, and the signature above is
three lines of Python anybody can re-run — which is the form this belongs in.
Same call the corpus already made when `--comment-paths` was registered as a
gate and demoted the same day.

⚠ And the signature has a blind spot it cannot close: the `session.rs` case
that started this was ONE contiguous `///` block holding two subjects, with no
blank line at all. Nothing syntactic separates that from a long doc. ⇒ The
detectable form is a lower bound, and the undetectable one is the shape that
produced the best-written orphan in the tree.
