# Yardrat's open measurements

Numbers this machine took that are executable by someone else, kept out of
[`queue.md`](queue.md) because that file is worked by several sessions at once
and a measurement is not yet a task with an owner. Each row states what was
measured, how to reproduce it, and what is NOT yet decided. When a row acquires
an owner and an acceptance criterion, promote it into `queue.md` and delete it
here.

⛔ This is not a staffing table and not a second review ledger. If a row here
stops being reproducible, delete it rather than annotating it.

---

## ✔ CLOSED — path citations in Rust comments (2026-09-03)

`check_planning_citations.py`'s bare-path class runs over `docs/planning/` only,
so the same blind spot existed one level down in `.rs` comments. Measured on
`79262265a`: **252 path citations, 21 distinct unresolved in 28 places.** All
triaged and fixed; the asset half needed no change.

⛔ **THE STANDING LESSONS, which is all that survives:**

- **Triage, never `sed`.** Two of the seven code paths were NOT repoints:
  `rendering/foreground.rs` and `player/systems.rs` name things that were ENDED  <!-- cite-ok: this row's subject IS the dead citation -->
  (`d09229ceb`, `5ba894709`), so their sentences wanted re-deriving. A third,
  `dialog/yarn_bindings.rs`, had a plausible-looking wrong target  <!-- cite-ok: this row's subject IS the dead citation -->
  (`yarn_harness.rs`) that is not the same thing.
- ⛔ **A file's history is not its function's home.** The mapping recorded here
  said `features/ecs/bosses.rs` → `game/ambition_content/src/bosses/mod.rs` from  <!-- cite-ok: this row's subject IS the dead citation -->
  basename history; the comment names `tick_boss_brains_system`, which is in
  `ambition_boss_encounter/src/ecs/tick.rs`. Verify at the DEFINITION SITE.
- ⚠ **Repointing moves any claim attached to the path with it.**
  `dialog/yarn_bindings.rs`'s sentence asserted "both are `ui`-gated"; the new  <!-- cite-ok: this row's subject IS the dead citation -->
  target had to be checked for that before the sentence could follow it.
- ⇒ **If the comment lane is ever extended, skip by `git check-ignore`, not by a
  `target/` prefix.** Confirmed empirically: the sprite manifests it would
  otherwise flag are on disk and ignored after a clean regen — generated files a
  built tree has and a checkout does not.

## Checked and CLEAN: cross-links between planning pages (do not build a checker)

Recorded so the next person does not build the tool this obviously suggests.
After the bare-path class landed, the natural follow-up is "validate the
`[text](other.md#anchor)` links too". ⇒ **Measured 2026-09-03: 328 internal
links across `docs/planning/**/*.md`, 0 dead files, 0 dead anchors — and only
ONE link carries an anchor at all.** A checker for that class would be a
permanent maintenance surface guarding a single citation.

⭐ The anchors that DO rot are the ones in policy `source_doc` fields (239 of
them, 15 repointed off dead `decomposition.md#…` anchors on 2026-09-02), and
those are guarded now by `every_source_doc_names_a_real_file_and_heading` in
`tests/ambition_workspace_policy/tests/policy.rs`. That is where the class
lives; the prose links are not it.

---

## Retiring `planning/engine/architecture.md` is a CONTENT job, not a path edit

`docs/planning/engine/architecture.md` is a redirect receipt: the canonical
architecture moved to `docs/architecture/engine-architecture.md`, and the
receipt says it *"remains temporarily because policy metadata, code comments,
ADRs, and historical documents still link here"*. Measured 2026-09-03 — it is
earning its keep, and the retirement is bigger than it looks.

**12 files reference it: 7 archived, 5 live.** The live ones are
`dev/journals/code_smells.md` and the workspace-policy set —
**15 `source_doc` citations** (10 in `engine.toml`, 2 in `game.toml`, 1 in
`repository.toml`, 2 custom metas in `src/custom/session_world.rs`).

⛔ **DO NOT BULK-REPOINT THEM, and the reason is a trap I nearly walked into.**
The obvious move is `sed` the path to the durable doc; it would keep
`every_source_doc_names_a_real_file_and_heading` GREEN and make the citations
worse. The durable doc is 283 lines and does not mention `ambition_load`,
`ambition_game_shell` or `ambition_load_presentation` **at all** — so the row
naming those three would cite a page that RESOLVES and does not state its rule,
which is strictly worse than citing a redirect that leads to the right place.

⇒ The order is: the durable doc absorbs the rules these rows cite, THEN the
paths move, THEN the receipt retires. ⚠ And the guard cannot help with step one
— it verifies that a document exists at the other end, never that the rule is
written there. That limitation is now recorded next to the guard itself.

⛔ **AND STEP ONE MAY BE THE WRONG MOVE, which a measurement 2026-09-03 makes
concrete.** All five durable docs together are **817 lines carrying 34 backticked
tokens and TWO distinct `ambition_*` crate names** (`engine-architecture.md` and
`package-and-capability-boundaries.md`, one each; the other three name none).
That is not an oversight — it is the register those documents are written in.
So "absorb the rules these rows cite" means teaching crate-level specifics to
pages whose whole style is crate-free, and the result would read like planning
status pasted into doctrine. ⇒ Before doing it, ask whether those 15 rows want a
durable doc AT ALL: a rule about `ambition_load`'s dependency direction may
belong in that crate's own `MODULES.md`, where it stays next to the code that
can falsify it. The retirement is blocked on a DESTINATION question, not on
writing effort.

⭐ **Corollary, measured at the same time: do NOT extend
`check_planning_citations.py` to `docs/architecture/`.** Ran it there — **5
citations across 5 files, all resolved.** A doctrine page states principles, so
there is almost nothing concrete for a citation checker to judge, and the guard
would cost a permanent lane to watch five references.

---

## ✔ CLOSED — "this box cannot run its own suite without a clean" (2026-09-03, re-measured 2026-09-04)

⛔ **NOT REPRODUCIBLE, in both halves.** This file's rule is that a row which
stops being reproducible goes, so it is collapsed to what survives — the same
shape as the citation row above.

⇒ **The capacity fact is dead.** Re-measured 2026-09-04: `target/debug/deps` is
**70 GB**, not the 141 GB recorded; the volume is **915 GB with 188 GB free**, not
a 290 GB volume being exhausted mid-run. A `cargo check --workspace --all-targets`
and several crate suites ran the same day without incident.

⇒ **And the gap it named is FIXED.** The row's finding was that
`check_disk_headroom.py` runs twice — before the first job and after the last —
and never between, so a long suite could exhaust the disk halfway and die
incoherently. `scripts/run_tests.py:1232` now calls `free_gb_on_target()` **before
each job** against a hard `ABORT_FREE_GB` floor, and its comment names the exact
failure this row described: *"`clang failed` whose reason line never reached the
log, under a job header that belonged to something else entirely."*

⭐ **THE ONE LESSON THAT SURVIVES BOTH, because it is not about disk:**
**free `target/`, never the evidence.** Reclaiming space during the first ENOSPC
deleted the exhaustive plan's log, and with it the runner's own
`disk: N GB free (±M this run)` line — the measured spend for a full plan, which
is the number the whole row existed to report. ⇒ A cleanup that removes the
measurement is worse than the condition it was clearing.

## ⚠ A BARE FILENAME CITATION DECAYS WITHOUT ANYONE TOUCHING IT

Found 2026-09-03, and the mechanism is new this week: **a carve can make an
existing citation ambiguous by adding a file nobody edited.**
`actor-monolith-decomposition.md:1105` cites `systems.rs:177` and now  <!-- cite-ok: this row's subject IS the ambiguous citation -->
matches TEN tracked files, because calculex's `ambition_encounter_features`
carve added one more `src/systems.rs`. The sentence was fine when written and is
unreadable now.

⇒ Measured across all of `docs/planning`: **17 distinct `file.rs:NN` citations,
and only two filenames are non-unique** — `systems.rs` (10 matches) and
`options.rs` (2). So the class is SMALL and the checker already reports it as
AMBIGUOUS rather than resolving it wrongly, which is the right behaviour. No new
tool is wanted.

⛔ **But the population is growing on the wrong side.** Five crates were carved
out of the actor monolith on 2026-09-03 alone, and generic module names
(`systems.rs`, `mod.rs`, `options.rs`, `tests.rs`) are exactly what a new crate
brings. ⇒ **In planning prose, cite a crate-qualified path** —
`crates/<crate>/src/systems.rs:177`, which the checker resolves unambiguously —
and treat a bare filename as a citation with a shelf life. That is a HABIT, not
a check: the guard exists and is already doing its job.

⚠ And the same finding names a defect I did not fix, because the file is held:
line 1105's own subject is a corrected measurement (*"five and two, not four and
one"*), and the crate it credits, `ambition_characters`, has **no `systems.rs`
anywhere in its source**. The ambiguity report is what surfaced it. Passed to
the session that owns the file.

## ⛔ AN IDENTIFIER THAT DOES NOT RESOLVE IS USUALLY HISTORY, NOT ROT

Two sweeps, both of which looked like rich seams and both of which were almost
entirely false positives. Recorded together because the SHAPE is the finding and
a third session will otherwise try a third one.

| swept | places | real findings |
|---|---:|---:|
| bare file paths in prose | 460 | 1 |
| policy `source_doc` anchors | 239 | 0 (ratchet added) |
| **absence claims — "no X exists"** | **26** | **4** |
| SHA-shaped citations in `docs/planning` | 371 | 0 |
| `D<number>` queue-row citations | 78 | 0 |

⭐ **THE ABSENCE-CLAIM SWEEP IS THE BEST YIELD OF THE FIVE — 4 in 26** — and the
reason generalises: a sentence saying "no X exists" is written to justify an
open row, so it stops being true exactly when someone does the work the row
asked for. Nothing re-reads it at that moment. The four:
`ambition_registry_core` "does not exist" in TWO pages (it landed `479f9d3e4`,
and the crate cites one of those pages as its own justification); "no
residency-state type exists anywhere in `crates/`", true about ROOMS and
falsifiable by a grep since `FxResidency` arrived; and two comment citations
whose fix shipped in the same commit that found them. ⇒ Sweep this class after
any week of landings, and grep the claim as written — half the value is finding
sentences that are RIGHT and now read as wrong.

⚠ **AND THE RE-SWEEP THE SAME NIGHT FOUND NOTHING, which sharpens the advice
rather than repeating it.** The rule above says to sweep this class after any
week of landings. 2026-09-03 was a night of landings — the pickup carve, the
`string_id!` consolidation, S2, registry_core R2–R4 — so the class was re-swept
across all of `docs/planning` a few hours later. **Zero new findings**, and
every crate-existence claim still held on BOTH trees (`ambition_test_support`,
`ambition_snapshot`, `ambition_platformer2d_input` absent locally and on
`origin/main`; `ambition_registry_core` present). ⇒ The mechanism matters: a
sentence goes stale when someone does the work and *nobody re-reads the page*,
so the sweep pays across a gap in attention, not across a volume of commits. On
a night when the sessions doing the landings were also working the pages, there
is nothing for it to find. Sweep after an unattended week, not after a busy
night.

⭐ **BOTH CLASSES ARE STABLE HISTORICAL IDENTIFIERS BY DESIGN.** `queue.md`
names only 8 D-numbers because *"a closed row is a receipt, not a case file"* —
the README's own closed-row template is `✔ **D123 — …**`, so a D-number
deliberately OUTLIVES the row and resolves through `git log`, not through the
current file. And a SHA that resolves nowhere in this checkout is routinely a
submodule commit or an unpushed branch elsewhere.

⇒ **Before building a checker for an identifier class, measure its false-positive
rate against the tree — not its finding count.** Both of these would have shipped
a worklist of 78 and 33 entries that a reader must dismiss one at a time, which
is the "teaches its reader to skim" failure `check_planning_citations.py`'s own
docstring warns about. ⚠ The classes that DID pay were the ones where the
identifier is supposed to resolve NOW: bare file paths in prose (1 real finding
in 460, and it had been dead since `00030e603`) and policy `source_doc` anchors.

---

## ⛔ A SHA you cite for your OWN unmerged commit does not survive your rebase

Cost me ten dead citations and would have shipped them. Writing
*"Fixed in `<sha>`"* about a commit on your own branch is a citation to an
object that the next `git rebase` REWRITES — the commit survives, its hash does
not, and nothing warns you. Ten citations across seven files (the D33 rule,
`ambition_world_items/MODULES.md`, `queue.md`, four verification receipts) all
pointed at orphans after one rebase onto main. ⚠ They still `git cat-file`
successfully, which is what makes this invisible: the object is reachable in the
reflog, just not from `HEAD`. **The test is
`git merge-base --is-ancestor <sha> HEAD`, not `git cat-file -t`.**

⇒ **Either cite after the merge, or leave the slot blank and say why.** The D33
rule did the second — *"SHA deliberately not cited here … Fill it in when it
does"* — and that turns out to be the robust habit rather than a courtesy.

⚠ **AND A CHECKER FOR THIS WOULD BE NOISY, measured before proposing one:** 238
distinct SHA-shaped citations in `docs/planning`, 371 places. A
`` `[0-9a-f]{7,40}` `` sweep reports 33 "unresolvable", and MOST ARE NOT SHAS —
32-hex asset ids in `bevy-0.19-leverage-campaign.md`, 16-hex scenario ids in
`engine/runtime-frame-history.md`. Of the genuine ones, two resolve inside
SUBMODULES (`db7e72f` in the map assets, `5e1ee9b` in the sprite renderer) and
are correct cross-repo citations; three resolve nowhere on this machine, which
is NOT evidence they are wrong — an unpushed branch elsewhere resolves them.
⛔ **AND THE ONE I CALLED "PROVABLY STALE" WAS NOT — I made the same mistake the
paragraph above warns about, one line later.** `queue.md:1930` cites
`2d623308f`; it resolves here and is unreachable from HEAD, which is the exact
signature of a rebase orphan. It is not one. The row says so in its own words:
*"fix written, ON ITS OWN BRANCH `web-gpu-wait` (`2d623308f`), DELIBERATELY NOT
MERGED."* An unmerged branch in the SAME repository resolves and is unreachable
from `HEAD`, and that is correct.

⇒ **So "resolves but unreachable from HEAD" is AMBIGUOUS**: it is a rebase
orphan OR a deliberate unmerged-branch citation, and only the row's own prose
distinguishes them. A checker cannot. That takes the class from "one real
finding in thirty-three" to **zero**, and settles it — this is a habit, not a
check, and the habit is: cite after the merge, or say in the row why the SHA is
not on `HEAD` (which `queue.md:1930` does, and is why it reads correctly to a
person and wrongly to a grep).

---

## The `zero_duration_pump` bisect recipe, and the correction to it

Kept because the recipe worked and the correction is the part that would be
re-learned the hard way.

Bisect only the commits in the range that touch COMPILED files — a docs commit
cannot change behaviour, and on the 2026-09-02 range that was 21 of 70, turning
7 builds into 5. Each probe is a full `cargo test` of one test, so the saving is
real (roughly 20 minutes a probe on this VM).

⛔ **BUT BISECT THE ANCESTRY CHAIN, NOT THE `rev-list` INDEX.** On a branch with
merges the two are different, and I reported an exclusion that did not hold
because of it: a GOOD result at one index bounded nothing, because that commit
sat on a side branch that merged in later and was an ancestor of neither
candidate. Check with `git merge-base --is-ancestor A B` before treating a
result as a bound. The verdict survived — `06a494f4e`, confirmed independently
from the mechanism side — but it survived for a different reason than the index
suggested.
