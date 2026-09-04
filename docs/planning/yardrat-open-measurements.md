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
brings.

⚠⚠ **RE-MEASURED 2026-09-04, ONE DAY LATER, AND IT ROUGHLY DOUBLED.** Same method
— every `` `file.rs:NN` `` in `docs/planning`, matched against `git ls-files
'*.rs'` basenames:

| | 2026-09-03 | 2026-09-04 |
|---|---:|---:|
| distinct bare `file.rs:NN` citations | 17 | **35** |
| of those, non-unique filenames | 2 | **5** |

⇒ The five are now `systems.rs` (10 tracked files), `world.rs` (3),
`duel_arena.rs` (2), `facts.rs` (2), `options.rs` (2). ⭐ **So the prediction in
this row was right and the doubling took a single day**, which is a stronger
argument for the habit than the original measurement was.

⭐ **And I supplied one of the instances myself, which is the useful part.** A
citation I wrote on 2026-09-04 — `snapshot_impls.rs:451` <!-- cite-ok: this row's
subject IS the ambiguous citation --> — was flagged AMBIGUOUS
by the checker against **twelve** tracked files with that name, and I only learned
because the checker said so. ⇒ The guard is doing its job and the habit is what is
missing: I knew this row existed, had written it, and still typed a bare filename
the next day. Repointed to the crate-qualified path.

⭐⭐ **AND IT HAPPENED A THIRD TIME, INSIDE THIS PARAGRAPH.** Quoting the bad
citation above created a fresh ambiguous citation, which the checker flagged on
the very next run — so the row about decaying filenames decayed the moment it
described itself. ⇒ Marked `cite-ok`, which is exactly what that marker is for
(*"a row quoting a mistake it is recording"*), and left visible because **three
instances in two days by the person who wrote the warning** is the strongest
evidence available that this is a habit problem and not a knowledge one. ⇒ **In planning prose, cite a crate-qualified path** —
`crates/<crate>/src/systems.rs:177`, which the checker resolves unambiguously —
and treat a bare filename as a citation with a shelf life. That is a HABIT, not
a check: the guard exists and is already doing its job.

⚠ And the same finding names a defect I did not fix, because the file is held:
line 1105's own subject is a corrected measurement (*"five and two, not four and
one"*), and the crate it credits, `ambition_characters`, has **no `systems.rs`
anywhere in its source**. The ambiguity report is what surfaced it. Passed to
the session that owns the file.

## ⚠ THREE MUSIC-RENDERER TESTS NEED AN UNDECLARED DEPENDENCY (measured 2026-09-04)

⭐ **Jon's standing ask is that a fresh clone reaches a runnable game.** Checking
the other standing ask — no General-MIDI stand-ins — turned this up beside it.

⇒ **`tools/ambition_music_renderer` suite: 252 passed, 1 skipped, 3 FAILED.** All
three fail the same way: an assertion that a plot file exists
(`plots/stem_loudness_timeline.jpg`, and the spectrogram equivalent).

⛔ **The cause is `matplotlib`, which the package does not declare.**
`tools/ambition_music_renderer/pyproject.toml` lists twelve dependencies and two
`optional-dependencies` (`pedalboard`, `dawdreamer`) — **`matplotlib` is in
neither**. `import matplotlib` in that venv is `ModuleNotFoundError`. ⇒ So a
machine set up exactly as `python_tools.sh` intends gets a renderer whose own
suite is three red, and the three name a missing FILE rather than a missing
module, which is why nobody has traced it to a dependency.

✔ **What is NOT affected, checked separately because it is the ask that matters:**
the General-MIDI guard family is fully green — **38 passed, 1 skipped** across
`test_refuses_the_general_midi_fallback.py`,
`test_preflight_refuses_a_missing_named_library.py`, `test_bulk_render_preflight.py`
and the MIDI marker tests. And this machine's instrument environment is present
(2,351 sfz files, `sfizz_render` installed). ⇒ The fallback ask is guarded and
holding.

⛔ **NOT FIXED HERE, deliberately: `ambition_music_renderer` is a SUBMODULE**, so
declaring the dependency is a commit in another repository, and the parent's
recorded commit already differs from that submodule's HEAD. ⚠ The submodule's own
working tree is clean, so this is a committed state and not somebody's scratch
edit.

⇒ **Reproduce:**
`tools/ambition_music_renderer/.venv/bin/python -m pytest tools/ambition_music_renderer/tests/ -q`

⇒ **Not decided:** whether `matplotlib` becomes a real dependency (it is imported
on a path three tests assert) or the three tests learn to skip without it. The
second is cheaper; the first is honest if those plots are part of the artefact.

## ⚠ A PORTAL TEST HARNESS `.chain()`s WHERE PRODUCTION ONLY `.after()`s (2026-09-04)

⭐ **Verified, and it is a difference in FLUSH SEMANTICS rather than in order.**

| | how the two systems are wired |
|---|---|
| test (`portal/tests.rs`, `app_with_the_shot_adapter`) | `(portal_fire_system, portal_projectile_step).chain()` |
| production (`portal/plugin.rs:136`) | `portal_projectile_step.after(portal_fire_system)` |

⇒ `.chain()` inserts an `ApplyDeferred` between them; `.after()` does not, and the
sets involved (`PortalSet::WeaponAndProjectiles`,
`Platformer2dSimulationPhaseMonolith::PlayerSimulation`) are configured with
ordering relations only — **no `.chain()` on the sets** in
`portal_schedule.rs:49`. ⚠ `portal_fire_system` spawns the shot through
`Commands`, so the entity does not exist until a flush.

⛔ **THE QUESTION, and it is worth an answer because a guard rests on it.**
`two_same_channel_shots_landing_on_one_tick_leave_exactly_one_portal` documents a
real defect — two same-channel shots resolving in one tick each despawned the old
portal and spawned a new one, leaving two — and its own comment says *"both
origins are 25px from the left wall, inside one 31.7px step, so both resolve on
the same tick."* ⇒ That requires the shot entity to be VISIBLE to
`portal_projectile_step` on the tick it was fired, which the test's `.chain()`
guarantees and production's `.after()` may not.

✔✔ **ANSWERED BY EXPERIMENT, and the answer is (1).** I switched the harness from
`.chain()` to production's exact `.after(portal_fire_system)` and re-ran: **the
test still passes.** ⇒ The two shots still resolve on the tick they are fired, so
a sync point IS reached between the two systems — Bevy inserts one automatically
when a `Commands` writer is ordered before a reader of the affected data. ⭐ The
guard defends a reachable state and the harness is NOT stricter than production.

⛔⛔ **AND IT INVERTS WHAT I EXPECTED ON THE OTHER SIDE, which is why it was worth
running.** Because a shot fired within one step's travel of a wall spawns AND
despawns inside a single `sim.step()`, **portal shots ARE an instance of S4's
residual** (*"an anchor spawned and despawned inside one step is invisible to a
between-steps census"*), not an exception to it. ⇒ Such a shot genuinely exists at
an internal system boundary — so save/load inside that step can see it — while a
census walking the world BETWEEN steps never can.

⚠ **Which makes it the sharpest known example of that residual**, because it is
not hypothetical: it needs only a player firing within ~32px of a wall, and that
entity carries a minted `SimId` precisely so that it rewinds correctly.

✔ **CONFIRMED FROM THE OTHER SIDE, and it bounds the claim.** A census ordered
`.after(portal_fire_system)` INSIDE the simulated frame sees that shot on
**exactly 1 frame**; the same census left unordered sees it on **0**; an
open-space shot is seen on **371**. ⇒ So the entity is *not* invisible in
principle — it is visible for one frame to an instrument standing on the right
edge, and invisible to every instrument that walks the world between steps.

⛔ **I had told the S4 author the stronger version** — that the shot is invisible
to a between-steps census *and therefore* a case no census can witness. The first
half is right and the second does not follow. ⚠ And the correction came from
varying the INSTRUMENT (the scan's ordering), not the subject: **a negative result
is a claim about the instrument until you have varied the instrument**, which is
the clamp rule pointed the other way.

⭐ **Method note.** I wrote "two possibilities and I have not separated them" and
was about to leave it there. Separating them cost one edit and one test run, and
the result reversed my expectation on the second half. ⇒ The reproduction I
recorded — *do not answer this by reading the schedule* — was right, and I nearly
took my own bad advice by filing the question instead of answering it.

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

---

## ⛔ OPEN — the composed app cannot be built on this VM, and `df` says otherwise

**What is blocked:** every measurement that needs `ambition_app`. Concretely
`report_the_smash_kit_every_selectable_fighter_has`
(`game/ambition_app/tests/smash_roster_movesets.rs`), which hard-asserts that
every composition-selectable fighter reaches the full 16-press genre kit and
prints a per-fighter census as it goes. ⇒ That single command answers the open
roster question in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md),
so this is not a niche blocker.

```
cargo test -p ambition_app --test app_it report_the_smash_kit -- --nocapture
```

**The failure, and it lied twice before it told the truth:**

| attempt | what it said | what it was |
|---|---|---|
| 1 (incremental) | ~40 × `mold: error: undefined symbol: anon.<hash>.llvm.<n>` | truncated rlibs — reads exactly like stale incremental codegen, and I diagnosed it as that. **Wrong.** |
| 2 (`CARGO_INCREMENTAL=0`) | `No space left on device (os error 28)` | the real class, stated plainly for the first time |
| 3 (sequential) | `mold: error: undefined symbol: <bevy_ggrs::…>::Rollback` | a *named* symbol — the rlib attempt 1 truncated was still cached, and cargo's fingerprint called it fresh |
| 4 (after `touch`ing that crate's `lib.rs`) | `mold: failed to write to an output file. Disk full?` | the rlib rebuilt clean; the final `libambition_app.so` is what cannot be written |

⭐ **`df` is not the instrument here.** It reports **188G free, inodes at 5%** —
and those numbers are the HOST's, because the worktree is a **virtiofs**
passthrough (`findmnt` → `aivm-persistent-root … virtiofs`). Measured against the
filesystem instead of asked of it:

- 50MB write: fine, 953 MB/s.
- 300MB write: fine.
- 1GB write: fails.
- 2GB write: stops at **576MB**.
- 800MB write: killed with no message at all.

⇒ **The usable ceiling for a single file is somewhere near half a gigabyte**, and
a debug `.so` for the composed app is far past it. ⚠ Nothing about the repository
is wrong: `ambition_demo_smash` builds in seconds and its 154 tests pass, and
attempt 4 proves the dependency graph links right up to the final artifact.

⛔ **NOT ATTEMPTED, deliberately.** `rm -rf` under `target/` is a standing repo
rule, and `cargo clean` is the same deletion with a friendlier name — `target/` is
**187G** (debug 148G), so it is the obvious lever and it is not mine to pull.

**Two things worth knowing before pulling it:**

1. ⚠ **The corrupt-cache trap will outlive the disk problem.** Once a write is
   truncated by ENOSPC, cargo keeps serving the broken rlib because its
   fingerprint is still valid — so the build fails with *undefined symbols* long
   after space is restored, and the error names a linker problem, not a disk one.
   `touch`ing that crate's `lib.rs` fixes it without deleting anything, and that
   is what moved attempt 3 to attempt 4.
2. ⭐ **Cheapest non-destructive thing to try first is a smaller artifact**, not a
   bigger disk: `CARGO_PROFILE_DEV_DEBUG=0` drops the debug info that makes the
   `.so` enormous.

ⓘ **Relevant to the standing "a fresh clone must reach a runnable game" ask** —
but as a MACHINE finding, not a repo one. A fresh clone on a host with room is
untouched by any of this. What it does mean is that a fresh clone *on this VM*
would fail at the final link, and would report it as an undefined symbol.

---

## ⚠ OPEN — a submodule pointer rode into an unrelated commit of mine

**Mine to own.** I staged with `git add -A`, and it swept
`tools/ambition_music_renderer` into `e2fe33e2e` — a commit whose message is
entirely about George's movesets and does not mention a submodule. The bump is
`4e5695c38 → 8b10c5a0d`. ⇒ I have stopped using `-A` and stage explicit paths.

⭐ **I did not revert it, and the checks are why.** In order:

- `8b10c5a` is the sibling session's music work — *"The refusal was
  all-or-nothing; one missing family still shipped wrong music"*, sitting on
  *"The refusal that protects Jon's no-GM-fallbacks ask had no test"*.
- It **is** pushed: `git branch -r --contains` finds it on
  `origin/agent/sfizz-source-fallback-and-cue-fanout`. ⇒ A fresh clone can fetch
  it, so the standing no-fallbacks ask is not broken by the pointer itself.
- ⛔ **The pointer it REPLACED does not exist in this clone.**
  `git cat-file -t 4e5695c38` → *"could not get object info"*. ⇒ Reverting would
  aim the superproject back at an object nothing here resolves, which is worse
  than what it replaced. **An accidental change is not automatically the wrong
  state**, and undoing it reflexively would have been the actual damage.

**What is still open, and it is not mine to close:** the new pointer names a
commit on an **agent feature branch**, not the submodule's `main` (which is at
`26b87bf`). A superproject pointing into a branch that may later be deleted is a
dangling pointer waiting to happen. ⇒ Landing that work on the submodule's `main`
and re-pointing is the durable form; if it is not meant to land yet, the
superproject should not point at it at all. Raised with the session that owns it.

ⓘ **The transferable bit:** `git add -A` in a superproject stages submodule
pointer moves, and they are invisible in a diff that scrolls — one line, no
content. A commit can silently change which version of another repository the
build uses. ⇒ Stage paths, and read `git status --porcelain` for ` M <submodule>`
lines before committing.
