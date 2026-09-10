# The check that was correct and did not run

A test that fails is information. A test that never executed and reports green
is worse than no test at all, because it also spends the attention that would
have found the defect by hand.

⭐ **AND IT HAS A SIBLING FAMILY, which #10 below already notices when it calls
itself "a DIFFERENT SPECIES from 1-9"** — the check that ran perfectly and could
not have failed. It has its own section below, and its instances live in a
journal.

⭐⭐ **AND A THIRD FAMILY ARRIVED ON 2026-09-10, LARGE ENOUGH TO NEED
ANNOUNCING: the check ran, could have failed, and DID NOT LIE — and the SENTENCE
WRITTEN ABOUT IT was wrong anyway.** A blast-radius probe quoted about a wider
edit than it measured. A green quoted as a claim about the repository when it
was a claim about one lane. A proposal read as a receipt. A property measured
only on the accused. A reading taken at the wrong tick, or over a population
that had survivorship in it.

⇒ **Families one and two are about instruments. This one is about the CLAIMS
people carry away from working instruments**, and it is the family this page had
no map for while it accumulated six members in a day. ⚠ **Its members are
harder to see than a gate hole, because in every one of them the measurement
was correct** — there is no red to find, no skipped job, and nothing to grep
for. The only tell is the gap between what was measured and what was said.

⛔⛔ **AND FAMILY THREE HAS NO AUTOMATED DETECTOR, AND PROBABLY CANNOT HAVE ONE.
Do not write a script for it.** Families one and two are checkable — a job that
does not appear in a lane, a corpus that is empty, an assertion that cannot
fail. **Family three needs a reader who knows what was measured and reads what
was claimed**, and no artifact holds both. ⇒ **Its remedy is a second reader,
which is a process claim rather than a tooling one.** Every one of the six was
caught by somebody other than the author, and **not one was found by the author
re-reading their own sentence** — which is also why six appeared in a single day
of two agents comparing notes, and none in the weeks before it.

This page is the dual of
[`cheapest-sufficient-check.md`](cheapest-sufficient-check.md). That page asks
*what is the least I can run to settle this change*. This one asks *did what I
ran actually run* — and it exists because on 2026-09-02 a single day's work
turned up SEVEN members of the same family in one gate script; an eighth was
already sitting in the backlog unrecognised, and a ninth surfaced the same night.
Several more followed on 2026-09-03, from two agents on the same day.

⭐ **THE FINDING IS NOT THE COUNT. It is that seven of the first ten were found
by accident** — by running a suite for an unrelated reason, by an external reviewer
reading source, by running `cargo check` by hand before the gate. ⭐ **#12 IS
THE FIRST ONE FOUND ON PURPOSE**, by the audit in "Running this audit yourself"
below — asking of every `scripts/check_*.py` whether the gate, the pytest lane
or CI names it. That is what a deliberate search buys, and it is why the ratio
above is a fact about the past rather than a law. Not one was
found by the checking system noticing its own hole. A gate cannot audit its own
coverage, because the same assumption that makes a job skip also makes the
report say it ran.

⚠ **AND THIS PAGE ROTS.** Member #3 was live when the first draft of this
sentence was written and fixed by `234bcc686` about an hour later, while the
draft was still open. That is not an embarrassment to correct quietly — it is
the argument for the rule at the bottom of this page. Re-check a member against
the current `HEAD` before you repeat it to anybody; a catalogue of gate holes is
a claim about a changing repository, exactly like a queue row.

## The question before the four

⛔ **DID ANYONE RUN A CHECK AT ALL?** Everything below assumes a check ran and
lied about its coverage. The plainer failure underneath the whole family is a
check that was simply never run in the window where it mattered, and it produces
an identical outcome: nobody knows.

On 2026-09-02 `main` could not compile for several commits. `perception_census.rs`
was renamed out of `ambition_dev_tools` in one commit and its
`pub mod perception_census;` line in `lib.rs` was not removed until `01b7c7ca0`,
so every commit in between failed with **E0583, file not found for module** — the
loudest, least subtle error Rust has. It survived because the commits in that
window were documentation-shaped and nobody compiled that crate.

⭐ **AND IT SURVIVED A CONFLICT-FREE MERGE, WHICH IS THE PART TO REMEMBER.** A
branch based before the rename merged `main` cleanly: its side kept the `lib.rs`
line because it had never touched it, the other side deleted the file, and git
was correct both times. A `rename … => …` in a merge's stat output for a file
whose module is declared elsewhere is a semantic conflict a textual merge cannot
see — and the merge reports success.

So: "the gate is slow, I'll run it later" and "the gate is blind" have the same
consequence. The four questions below are for the second case; this one is
answered by running something at all, on the crate the merge just renamed.

## The four questions

Ask these of any check you are about to trust. They are cheap, and each one
found at least one member below.

1. **What does this flag skip?** A scope flag is a promise about what you are
   NOT running, and that promise is usually only in the help text.
2. **Which plan is this job in?** A job that exists in the exhaustive plan and
   not the default one does not run on the command anybody actually types.
3. **Does it re-derive, or does it reuse a cache?** A scan over incremental
   build output sees only what recompiled.
4. **What would this look like if the feature were silently off?** If the answer
   is "exactly the same, and green", the check is measuring the harness.

⛔ **ASK THEM TOGETHER, NOT SEPARATELY.** Member #6 below is blind in two
independent ways at once, and each question in isolation returned a locally
reassuring answer. "Which plan is this in" and "which branch does this compile"
were both asked of that job, months apart, and it stayed broken.

⛔ **AND CITE THE JOB NAME, NOT THE LINE NUMBER.** The first draft of this page
cited `run_tests.py:368`, `:588` and `:590`. One merge later they were `:375`,
`:595` and `:597` — three dead citations in a page whose whole subject is
claims that quietly stop being true. Grep for the job's name string or its gate
expression; those survive edits that line numbers do not.

## The fourteen, and what each one teaches

| # | the check | how it lied | status |
|---|---|---|---|
| 1 | `./run_tests.sh --rust` | skipped the whole Python lane, so the rollback stable-name ratchet, the codec-shape baseline and a stale `MODULES.md` sat red **for a day** behind a gate reporting 4/4 GREEN | fixed `2945f3381` |
| 2 | the wasm build job | gated on `if not only and everything` — the exhaustive plan only, so a default run never checked `wasm32` at all | fixed `b85b4db20` |
| 3 | `check_no_warnings.py` | parses diagnostics instead of setting `-D warnings`, so it reuses the build fingerprint — and **cached crates do not re-emit warnings**. In a warm tree it reports clean while real warnings exist | fixed `234bcc686` — the gate job is now `check_no_warnings.py --fresh`, and the price of the cold re-fingerprint is stated at the job |
| 4 | the repo-tooling lane | simply not invoked by the flag anybody was using | fixed `2945f3381` |
| 5 | the wasm CHECK | is **TYPE-ONLY**. `cargo check` cannot see a `#[cfg]` that removes BEHAVIOUR rather than breaking a build | ⛔ **STRUCTURALLY LIVE** |
| 6 | "web persona BOOTS" | runs the web composition **NATIVELY** (`--features visible_web_base`, native target), so it compiles the `not(wasm32)` branch — and its `if not only and everything:` gate puts it in the exhaustive plan only | ⛔ **STRUCTURALLY LIVE, TWICE.** ✔ **RUN 2026-09-03 on the calculex host (no GPU): it SURVIVES startup** — route `ambition_launcher`, 23 UI nodes, 10 UI texts, 0 sprites, 2 cameras, simulation host `Rollback`; 16 m 01 s to build. ⚠ That closes NEITHER half: the run is still native, so it still compiles the `not(wasm32)` branch, and the gate still plans it only in the exhaustive plan. What it does establish is that the job passes when someone runs it, which had not been checked |
| 8 | the Bevy 0.19 **Android font path** | is TYPECHECKED, NEVER RUN. The port deleted the hand-rolled `seed_android_system_fonts` and turned on Bevy's `system_font_discovery` for `android_platform` instead. ⛔ Its whole job is to find fonts the HOST does not have, so a desktop green says nothing about it | ⛔ **STRUCTURALLY LIVE.** Recorded in `../planning/tracks.md`; closing it needs a device, not a build | <!-- cite-ok: member 8 records this deleted function by name -->
| 9 | the **16 `image_stages` tests**, including the reveal-readiness guard | exist only under `--features bevy`. `ambition_asset_manager`'s DEFAULT features exclude `bevy`, so the module does not exist in `cargo test -p ambition_asset_manager`: **56 tests run, not 83**. The gate's feature-union job would cover them — but it is built inside `if everything:`, so it is EXHAUSTIVE-PLAN ONLY | ⛔ **STRUCTURALLY LIVE, AND FAR BIGGER THAN THIS ROW — 783 TESTS ACROSS 29 CRATES**, measured 2026-09-03 (⚠ 784 by the evening of the same day — one test joined; the count is a moving target by construction, which is why the gate footer is ratcheted and this row is not the place to read it) with `scripts/feature_gated_tests.py` (which already existed): the 16 here are one module of a class that includes 53 in `ambition_content`'s `portal`, 26 in `ambition_input`'s `local_seats` and 25 in `ambition_app`'s `grid_backend`. The union job that runs them is inside `if not only and everything` in `run_tests.py`, so a DEFAULT green says nothing about any of them. The gate's coverage footer named the gap qualitatively and gave no magnitude; it now states the count, and `test_the_gate_states_how_many_tests_it_skips.py` ratchets it so the figure cannot rot. Found 2026-09-02 when a new test in that module printed `running 0 tests` and PASSED. ✔ **RUN 2026-09-03 on the calculex host, and every number reproduces: 83 with `--features bevy`, 56 without, 16 of the difference in `image_stages` — and all 83 PASS.** So the blindness is not currently hiding a failure, which is worth knowing and is NOT the same as it being fixed: the gate still does not run them, and the next break here is still invisible to it |
| 10 | `[census] owners` (and its sibling `owners_in`) | is a **TOP-20**. The row prints `crates=82` and then names twenty, so a reader who greps it for a crate and finds nothing cannot tell *registers no systems* from *ranked 21st* — and the emitter's own doc comment says the row answers *"should a shipped title carry this at all"*, which is an ABSENCE question. Absence was uninformative for 62 of 82 crates while looking authoritative | fixed 2026-09-03 — both emitters now append `+N_more_not_shown`. ⚠ A DIFFERENT SPECIES from 1-9: not a gate that skipped, an instrument that answered a narrower question than it appeared to |
| 11 | `./run_tests.sh --rust` itself | **exits 2 having run NOTHING** when the Python lane's `tree_sitter_rust` is missing, and says so in a voice that reads as informational: *"this interpreter cannot run the Python lane … affected: 1 planned job(s) … fix: scripts/setup/python_tools.sh"*. One affected job aborts the whole RUST lane, and the header it prints is indistinguishable from a normal preamble — a reader who does not check `$?` sees a run that appears to have started | fixed on this host by `scripts/setup/python_tools.sh`; the lane then ran 6957 tests. ⚠ **STRUCTURALLY LIVE ELSEWHERE**: any host missing that tool gets the same silent no-op |
| 7 | the coverage footer | said `- the wasm/web build LINK (the wasm CHECK ran)` **unconditionally**, while the job is appended only `if wasm_target_installed()`. No target → no web job, all green, exit 0, and a report that it was checked | fixed `159e76ba8` |
| 12 | the two **CI-ONLY ratchets** (`check_doc_link_ratchet`, `check_zone_name_ratchet`) | are invoked by `.github/workflows/test.yml --check` and by NOTHING ELSE — not the gate, not the pytest lane. Measured 2026-09-03 by asking, for every `scripts/check_*.py`, whether the gate, the pytest lane or CI names it: 14 of 16 are reachable locally and these two are not. ⛔ So a developer never sees them, and the doc-link one had accumulated TWO unseen regressions (`ambition_characters` 24→26, `ambition_platformer2d_core` 34→35). ⚠ It is also the slowest guard in the repo — a cold `cargo doc` over 9 crates, which TIMED OUT at 300 s in the sweep that found it, so "run every guard" quietly skips it unless you budget for it | ✔ **FIXED the same day.** The two regressions are repaired and `ambition_body_seed` added to its tracked crates (`bf4e6f353`), and both ratchets now run in `./run_tests.sh --maintenance` — the lane that exists for periodic hygiene and already owned `check_agent_kb`. 3/3 jobs, 147 s, of which the doc-link ratchet is 133 s. ⭐ NOT in the default plan on purpose: a cold `cargo doc` over nine crates is the wrong price on every dev cycle, and "reachable by one local command" was the actual gap, not "runs constantly". ✔ **RE-MEASURED 2026-09-03 late: 16 of 16 now reachable, and the shape of the answer changed.** Four (`check_engine_systems_are_engine_installed`, `check_retired_crate_names`, `check_rollback_mutators_run_in_sim`, `check_set_pins_have_engine_members`) are named by no lane and no workflow — their only caller is a test module. ⚠ That is the exact shape `--vanished` had in member 14 before it was aimed, so each was opened rather than counted: **all four carry a real-tree arm** (`test_the_live_tree_names_no_retired_crate`, `test_the_real_tree_has_no_unwaived_rows`, `app_only_systems(REPO)`, and a `collect()` that defaults to REPO), so the corpus is covered and not only the function. ⛔ **AND THE FIRST MEASUREMENT SAID FOUR WERE UNREACHABLE, WRONGLY** — it grepped for the `.py` FILENAME while the callers `import check_retired_crate_names` by module name. A reachability census is only as good as the shape of reference it looks for, which is the same failure this table records everywhere else, arriving in the tool used to audit it |
| 13 | `./run_tests.sh --heavy` | is re-enabled wholesale by `--heavy`, which runs `cargo test --workspace --include-ignored`. `#[ignore]` conflates TWO unrelated reasons — *slow, run on demand* and **invalid unless run alone** — and `--include-ignored` cannot tell them apart. `parallax_theme_retires_on_walk` says so in its own header: `ambition_app` has ONE `[[test]]` target, so every file under `tests/` is a module of `app_it` sharing a process, and *"a sibling booting its own app would populate `Assets<Image>` underneath this one's assertions"*. Under `--heavy` it runs beside 6957 others | ⛔ **STRUCTURALLY LIVE, AND THE LANE CANNOT BE GREEN.** `#[ignore]` in this repo carries at least three unrelated meanings and `--include-ignored` honours none of them: (a) **must run alone** — `parallax_theme_retires_on_walk`, `hall_redecode_census`, whose assertions a sibling's `Assets<Image>` can satisfy, so the failure is a GREEN result that measured nothing; (b) **panics BY DESIGN to print a census** — `d71_transaction_census`'s two probes end in `panic!`, VERIFIED 2026-09-03 by running one with `--ignored`: `test result: FAILED. 0 passed; 1 failed`; (c) print-only probes and audit listings across 7 files whose own reasons say *"read it, do not assert on it"*. ⇒ So the plan that exists to run EVERYTHING was red the moment it ran, for reasons all by design. ✔ **FIXED 2026-09-03**: probes are named `probe_*` and the heavy job runs `--include-ignored --skip probe_ --test-threads=1` — the skip removes the by-design red, the serial run makes the isolation-required ones mean what they say. Guarded by `scripts/tests/test_probe_tests_are_named_probe.py`, poisoned both ways, matching on the ignore REASON rather than a name list because the reason is where the intent is written |
| 14 | `scripts/check_planning_citations.py --vanished REF` | is invoked by **NOTHING but its own unit test**. `scripts/tests/test_planning_citation_vanished_mode.py` exercises `vanished_report` six ways and proves the instrument WORKS; no gate, no CI job and no lane ever points it at the real `docs/planning` with a real baseline. ⭐ A DISTINCT SPECIES AGAIN: not a gate that skipped (1-9) and not an instrument answering a narrower question than it looks (10-12), but a **correct, tested instrument that has never been aimed**. The tests are the trap — a green suite makes it look covered, and what is covered is the function, not the corpus | ⛔ **OPEN, and it has a measured yield.** Run 2026-09-03 against a 2026-08-13 baseline it reports **45 bare citations across 16 planning documents** naming something that WAS a definition in that tree and is not one at HEAD. For scale, a hand sweep of the same corpus the same day — every sentence-shaped backticked identifier, minus every `fn` in the tree — found **2 in 162**; the differential asks the precise question and yields twenty times more. ✔ **WIRED 2026-09-03, into `./run_tests.sh --maintenance`**, beside the two ratchets that arrived there for the same reason — with a FIXED baseline ref in `PLANNING_VANISHED_BASELINE`, never a rolling window, because a sliding baseline lets a finding stop being one without anyone deciding that; advancing the constant IS the act of accepting a triage and belongs in the commit that clears the rows. ⛔ **AND THE WIRING NEEDED `--strict` TO MEAN ANYTHING**: bare, the check prints its findings and exits **0** — 13 findings, exit 0 without it, exit 1 with it — so the first attempt at fixing this member would have recreated it one level down, as a maintenance job that lists real problems and reports success. Guarded by `scripts/tests/test_maintenance_vanished_job.py`, poisoned. ⚠ The original note read: it would be red on arrival, and the 45 need triage first — some rows RECORD an old name on purpose and want the `cite-ok` marker, not a repoint. Choosing the baseline (a fixed ref? a rolling 30 days?) is a maintainer call, because the answer changes what the check MEANS. ⭐ **TRIAGED 2026-09-03, 45 → 17**, and the breakdown is the useful part: **25 were rows RECORDING a deleted name on purpose** — deletion inventories under `Delete:`, `This should remove:` and `## Original plan`, and dated decision/investigation rows naming the code as it stood that day — all now carrying `cite-ok`; **2 were real renames** worth repointing (`sync_moving_platform` → `sync_moving_platform_visuals`, and `body_is_corpse`, whose intangibility promise moved to the `OutOfPlay` marker and `is_intangible`); **4 were FALSE POSITIVES** on names that are still definitions at HEAD — `attacks`, `grounded`, `conversation` and `combat` — every one of them a short generic name that is a **FIELD** now and was an ITEM at the baseline. ✔ **FIXED THE SAME DAY, in the instrument rather than in the rows.** `item_names` drops the `FIELD` rule so a field cannot INVENT a vanished name (a documented earlier bug: `ambition_demo_pocket` reported vanished while the crate was alive); dropping it on HEAD's side too created the mirror error. `still_a_field` now subtracts HEAD's field names from the differential — asymmetric ON PURPOSE, because subtracting can only REMOVE findings and so cannot resurrect the earlier bug. Pinned by `test_an_item_that_became_a_field_is_not_reported`, poisoned both ways. ⇒ **13 remained and every one was a true positive**, all in hot files another agent owned — and that agent triaged them the same day (`e2aadd36a`, *"the thirteen names --vanished found in my rows are removal records, marked as such"*). ✔ **`./run_tests.sh --maintenance` is now 5/5 green and the check reports 0.** ⛔ **AND THE CHECK HAS A RUNNING COST, MEASURED THE SAME DAY: it reddens on
CORRECT prose.** Hours after it went green, a new line — *"…the per-frame despawn
sweep, the `DebugOverlayLabel` marker…"*, a true sentence about something the <!-- cite-ok: this row RECORDS a name the 0.19 port deleted; that is the point of the sentence -->
0.19 port deleted — turned the lane red again. That will happen every time
somebody writes accurately about a removal, and the `cite-ok` marker is the toll.
⇒ **If that cost ever outweighs the catch, narrow the CORPUS, not the baseline.**
Advancing the baseline to quiet it would hide the real findings in the same
range, which is the one thing the pinned ref exists to prevent.
⭐⭐ **AND THE OBVIOUS NEXT MOVE — AIM IT AT THE REMAINING DOC TREES — IS THE
WRONG ONE, MEASURED 2026-09-03 late.** The maintenance job scans five corpora;
`docs/` has seventeen subdirectories. Aimed at the six substantive ones it does
not cover (`adr`, `mechanics`, `sdk`, `tools`, `vision`, `learning`) it reports
**12 bare citations**, and every one of them is a reason NOT to widen the job:

* **7 are in `docs/adr/`** — `LegacyConstructionRoot`, `Unmigrated`, <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  `ProjectileOwnerId` (twice), `PendingLocalInput`, `RollbackSessionContract`, <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  `RoomReplayApplied`, `platformer_runtime`. All are true removals, and an <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  **architecture decision record describes the code as it stood on the day the
  decision was made** — that is the genre's whole contract. Aiming a
  currency check at a corpus that is deliberately historical would mean
  `cite-ok` on essentially every finding forever, which is not triage, it is a
  tax with no yield.
* **2 are historical by the same logic in `docs/tools/`** — `BROKEN` is a <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  status string a tool now prints, and `run_checks` appears in a sentence <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  explaining what the goal guard *used to* do.
* **3 are FALSE POSITIVES of the documented generic-name class** — `where` (a <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  Rust keyword, in a page teaching `where` clauses), `RenderApp` (a **Bevy** <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  type the page correctly says a headless app omits), and `BROKEN` again. Two <!-- cite-ok: this paragraph is an INVENTORY of names that vanished; naming them is its content -->
  of the three are in `docs/learning/`, which teaches Rust and Bevy
  vocabulary — a corpus where the check's premise (a backticked identifier
  names something in THIS tree) is false by construction.

⇒ **The five corpora it scans are the ones that describe CURRENT behaviour, and
that is not an accident of what someone got round to — it is the check's actual
domain.** The genre of a document predicts the verdict before the check runs:
records of past decisions want `cite-ok` by default, teaching material about
other people's APIs wants exclusion. ⛔ So the answer to "the check has never
been aimed at eleven of seventeen doc trees" is that it should not be, and this
paragraph exists so nobody spends an evening discovering that a fourth time.

✔ **AND THE TOLL WAS PAID, on this very line, 2026-09-03 late.** The
`--maintenance` lane came back 4/5 with that one finding — the sentence above,
reporting itself. It is marked rather than reworded: the row's subject IS the
deleted name, so rewording to dodge the check would destroy the record it
exists to keep. ⭐ Two things this priced that the prediction did not: the toll
falls on the page DESCRIBING the check as often as on any other, and the lane
stays red until somebody marks it — nothing degrades gracefully, which is
correct and is also why the marker has to be cheap to apply.
⛔ **THREE TIMES IN ONE EVENING, AND ALL THREE WERE PROSE ABOUT THE CHECK
ITSELF** (this line; the paragraph below arguing against widening the corpus,
which is an inventory of vanished names and so is made of them; and a queue row
citing `demote_stale_realizations` <!-- cite-ok: the reroute's own example, named here for the same reason --> as the motivating case for the orphan
census). ⇒ **Writing about removals reddens this lane essentially every time**,
so budget the marker into the edit rather than treating each one as a surprise.
That is a stable, cheap toll and not an argument against the check — but it is
the reason the marker must never become expensive to apply.
⇒ Note which half of the choice closed it: the rows were MARKED, not the baseline advanced. Marking says *this name is gone on purpose*; advancing the baseline would have said *stop looking before here*, and hidden anything else in the same range. ⭐ **AND THE SAME INSTRUMENT HAD A SECOND UNAIMED HALF:** it takes `paths` and its default is `docs/planning`, so the pages describing CURRENT behaviour had never been scanned. Aimed at `docs/{concepts,systems,architecture,recipes}` the same day it found `HitSource` documented with seven role-shaped variants that do not exist — on the page whose subject is the unification that replaced them — and a deleted message still listed among current ones in a sentence I had hand-corrected in that very file twice. ⇒ **A check can be aimed and still be aimed at only part of its subject**, and the tell is a default argument nobody has ever overridden. ⭐ The lesson worth keeping: a check nobody has aimed has never had its false-positive rate measured either, so the first real run prices BOTH — and its docstring's *"almost no false positives"* was a reasonable claim that one run turned into a number |

Between #5 and #6 the web path had **zero behavioural coverage**: one job could
not execute code, and the other executed the wrong branch of it. The hole that
exposed them was real — the web reveal barrier never waited for the GPU, because
the entire render-world half was `#[cfg(not(wasm32))]`. The commit that fixed it
names the mechanism exactly: *"The web reveal never waited for the GPU, because
the FACT was gated with the CLOCK"* (`2d623308f`, branch `web-gpu-wait`). A
`#[cfg]` that removes a timing concern took the fact it was timing with it, and
no type check can see that shape.

⭐ **#9 IS #2 WEARING DIFFERENT CLOTHES, AND THAT IS THE POINT OF LISTING IT.**
The wasm CHECK was exhaustive-plan-only until `b85b4db20` moved it; the
feature-union job that runs every gated test still is. So a correctness guard on
whether a room's cover may lift — `the_gpu_readiness_term_wants_the_gpu_stamp_while_a_render_world_is_present`
— does not run on the command anybody types. ⛔ The failure is not that the tests
are bad or missing. They exist, they pass, and they are thorough. They are simply
not in the plan.

⚠ **HOW BIG IS IT? UPPER BOUND ONLY, AND THE BOUND IS STATED AS ONE ON PURPOSE.**
Applying `run_tests.py`'s own selection rules by hand — non-default features, not
in `DENY_EXACT`, no denied prefix, crate not in `SKIP_FEATURE_JOB`, crate
contains `#[test]` — **31 workspace crates qualify** for the exhaustive-only
feature-union job. ⛔ That is a count of CRATES THAT COULD BE AFFECTED, not of
blind tests: "the crate has tests" and "the crate has tests behind those
features" are different questions and I measured the first. For the one crate I
measured properly, `--features bevy` adds 27 tests, 16 of them in the module that
mattered. The other 30 are unmeasured.

⇒ **The question this adds to the four: does the thing I am about to trust exist
in the DEFAULT plan, or only in the one nobody runs?** A test that only the
exhaustive plan executes is a test that runs when somebody already suspects a
problem.

### The negative result #10 was hiding, now that the instrument reports honestly

Worth finishing, because the aborted version of this was going to be a dramatic
finding. With every owner named, the shipped headless composition in
`hall_of_characters` bills systems to **12 of the 17** capability crates in the
facade's `all_capabilities`. The five that bill none are
`ambition_cutscene`, `ambition_settings_menu`, `ambition_sfx`,
`ambition_sfx_bank` and `ambition_ui_nav`.

⇒ **And all five are correctly absent.** None of them defines a Bevy `Plugin`,
and the only `add_systems` calls anywhere in the five are two inside
`ambition_sfx`'s own unit tests (`World::new()`, `Schedule::default()`). They are
data and vocabulary crates; a system census has nothing to say about them. Audio
itself is composed here — `ambition_audio` bills 10 systems — so their silence is
not a headless artifact either.

⭐ **The finding is that there is no finding.** Read through the truncated row
the same evidence said 16 of 17 capabilities were dead. Read through the fixed
row it says twelve do work, five are the wrong shape of thing to ask about, and
nothing is unaccounted for. An instrument that narrows silently does not just
lose precision — it manufactures the more interesting answer.

## The sibling family: it RAN, and it could not have failed

Everything above is a check that did not execute. The other half of the family
executed perfectly and asked the wrong question, and it is the larger half:
forty-seven instances, each with the commit that fixed it, are tabulated in
[`../../dev/journals/blind-checks-2026-09-03.md`](../../dev/journals/blind-checks-2026-09-03.md).
⇒ **Do not add that count to the fourteen above** — different question,
different population. #10 is the boundary case and belongs to both lists, and so
is #14: the vanished check both never ran anywhere AND, once wired, would have
run and reported success. ⚠ This sentence said *"the ten above"* until
2026-09-03 while the list had grown to fourteen — a heading-style tally going
stale under an appended list, which is
[shape 2 in the sibling recipe](re-measuring-a-planning-claim.md). Prefer not to
restate a count that another agent appends to.

⭐ **THE RECURRING SHAPE.** An emitter tells you what a line CONTAINS; it never
tells you what to compare it against. A parser written from the emitter
reproduces its vocabulary and inherits none of its ordering, thresholds or
population bounds — so the parse succeeds, the number prints, and the number is
about a different question. The green is real. The question is not the one you
asked.

Worked examples of each: a census ordered by game clock when the emitter added a
frame column for exactly that reason; "images decoded at boot" counted from a
line that prints only decodes ≥ 1.0 MP, so 7 lines stood for 252; a rollback
guard reporting "4 systems, none unsafe" whose population was 1 canonical type
of 113.

### ⭐⭐ A SECOND RECURRING SHAPE: a guard whose input is SOURCE TEXT has inputs it does not control

The FORMATTER is one, and the LANGUAGE'S OWN CONSTRUCTION RULES are the other.
Three instances landed on 2026-09-10 and **none of them was found by looking**:

- `rustfmt` wrapped one `commands.insert_resource(` call because its path was
  long, and a guard anchored on the contiguous spelling reported a leak that was
  not there;
- sealing a type with `#[non_exhaustive]` removed struct-literal syntax, and a
  guard recognising a drop by the literal `GroundItem {` went blind — the table
  it builds read as empty;
- a census recognising construction by a hand-kept list of blessed method names
  (`::new`, `::default`, `::from`) lost six sites to that same seal, and
  correcting it surfaced a minting site that had **never** been visible.

⛔ **The axis is DIRECTION, not likelihood.** A guard blinded into reporting a
PHANTOM is self-limiting — somebody chases it, which is what happened with the
first of the three. A guard blinded into reporting **"no offenders"** prints a
clean bill of health forever. Ask of every source-text guard: *if the scan under
it matched nothing, would it still pass?*

**The remedies, strongest first** — this ranking came out of the sweep, not out
of its scoping:

1. **Give it a second input of a different KIND.** The `*_it_sync` guards derive
   one set from source text (`mod <name>;`) and one from a directory listing, and
   assert each difference is empty. Blind either side and the other is still
   full, so it reddens. A floor says "I saw N things"; cross-evidence says "two
   independent worlds agree", and only one of those survives the instrument going
   blind.
2. **Repoint it at a spelling the language makes CANONICAL.** After the seal,
   `the_death_drop_table_is_complete` watches `GroundItem::` — a closed spelling
   the type owns, because no crate outside the owner can construct it another
   way — instead of an open one any caller could vary.
3. **Use Rust's shape rather than a list of names.** An associated function is
   `Type::snake_case(`; a method is `value.snake_case(`. No list to keep.
4. **Failing all three, add an anti-vacuity floor** so the blindness is loud.

The sweep and its population live in
[`../planning/engine/source-text-guard-exposure.md`](../planning/engine/source-text-guard-exposure.md).

### ⛔⛔ `| tail` on a long run hides whether there is any output at all

`tail` writes only at EOF. A `cargo nextest run --workspace 2>&1 | tail -60 >
log` therefore produced a **0-byte file for eighty minutes** while the run was
perfectly healthy — and a live run, a hung run and a dead run are the same
reading. When cargo finally exited the pipeline never flushed at all: bash alive,
no cargo child, empty file, nothing to show for it.

⇒ Same species as the `| grep` that voids a non-zero exit status: **the shape
that makes the output convenient is the shape that hides whether there is any.**
For anything long, redirect straight to a file and read the file — progress is
then a `wc -l` and a failure is a `grep` away while it runs, instead of a
question you cannot answer until it ends.

### ⛔⛔ ASK THE COMPOSED APP, NOT A SOURCE FILE — four instances in one day

Every time a population came from what the running composition actually holds it
was right; every time it came from a list in the tree it was wrong.

| the question | asked of a file | asked of the app |
|---|---|---|
| which fighters can be duelled | `authored_movesets::tables()` — whose own header says *"NOT THE SELECTABLE CAST"* — gave ids the grid does not carry | the harness's refusal message prints the assembled grid |
| which `--character` ids can be seated | nothing checked at all; `__nope__` printed as a fighter | `PreparedCharacterRegistry` after the warm-up updates |
| how many moves a fighter authors | counting `MoveSpec` literals gives `pirate_admiral: 1` against **12 distinct started** | the composed `MovesetContract` |
| whether a token is a real schema row | a near-miss rule over condition ids read a correct citation as a typo | `RollbackRegistry::schema_dump()` beside the condition catalog |

⇒ **A list in the tree is a claim about the composition; the composition is the
composition.** The failure is not that the lists are careless — three of the four
carry a warning about themselves — it is that a list *reads as authoritative* at
the moment you need it, and the app is one subprocess away.

⚠ And the tell is cheap: **run the sanity check the answer implies.** `pirate_admiral: 1`
against a fighter observed starting twelve distinct moves is a contradiction the
table itself hands you, before anything is published.

### ⛔⛔ The RUNNER'S FOOTER is not the runner's VERDICT

A suite that reports well explains what it did *not* cover, and those paragraphs
contain numbers. On 2026-09-10 the `--rust` lane printed

> ⇒ `49/49 jobs passed` from this lane is true of the lane that ran, not of the
> repository.

as boilerplate about the everything-plan — **in the same output as a run that was
6 of 7, with `workspace (default features)` red.** The sentence is not wrong; it
is about a different plan. Read at a glance it is a green result naming somebody
else's lane, printed beside the red it is not about.

⇒ **The authority is the machine-readable record**: `target/run_tests_status.json`
while a run is live (it names the current job and every completed one with its
`ok`), and `dev/ambition_dev_measurements/run_tests_cost.jsonl` afterwards, which
carries `per_job` with the failing job's name and the commit it ran at. Both are
one `python -c` away and neither has prose in it.

⚠ The same record is what makes "was this red before my change?" answerable at
all: the cost ledger showed that job failing at one commit, passing at the next,
and failing again — three data points that no footer would have given.

### ⛔⛔ And the mirror image: the guard is PERFECT and the SUBJECT is inert

**The two produce the same passing green from opposite causes, and the remedies
differ, which is why they belong side by side.**

**The vacuous side, worked:** `test_text_spawns_resolve_a_font` asserts that no
menu text is spawned without a resolved font. It anchored on
`Text::(new|default)` — two blessed constructor names — with no anti-vacuity
floor anywhere in the file. `bevy_ui` declares `pub struct Text(pub String);`, so
`Text("Play".into())` is legal, unfonted, and was invisible. MEASURED by
appending exactly that spawn to a copy of the menu renderer: **the guard passed.**
Nothing was wrong with the assertion; the scan under it saw nothing, and seeing
nothing is what a healthy tree looks like. ⇒ **A vacuous guard wants a FLOOR.**

**The inert side, worked.** This one *can* fail, does exactly what it says, and
still tells you nothing — because **a fact only an instrument reads stays correct
forever while meaning nothing.**

Worked example: `SeatCredit` is written on one entity in one place, and its two
tests assert that the entity *carries* `SeatCredit(0)`. Both pass. Nothing in
production reads a seat credit — attribution in that engine runs on
`HitEvent::attacker`, an `Entity`, and the match verdict is decided by stocks
remaining. The assertion is true, the guard is sound, and the subject does no
work.

⇒ **An inert subject wants a READER** — and if there is no reader and no road
that wants one, what you have found is dead state, not a weak test. Ask of a
passing assertion about a stored fact: *who, in production, consults this?*

⚠ **And the diagnostic that separates them is one question, not two.** Both
print green. For the vacuous guard, ask *would this still pass if the scan under
it matched nothing?* For the inert subject, ask *who reads this outside the
test?* A guard can be neither, either, or — as `SeatCredit`'s tests are — sound
about a subject that does no work.

### Running this audit yourself

It found eight real defects in one evening, so it is worth repeating rather than
rediscovering. Four passes, cheapest first:

1. **Run every guard and read its REAL exit code.**
   ```bash
   for f in scripts/check_*.py; do
     out=$(timeout 240 python3 "$f" 2>&1); code=$?     # NOT `| head`
     printf '%-42s exit=%-3s %s\n' "$(basename "$f" .py)" "$code" "$(printf '%s' "$out" | head -1)"
   done
   ```
   ⛔ The first pass piped into `head` and captured `tr`'s status, so every
   check read `exit=0` — the bug being hunted, in the tool hunting it. Look for
   a traceback, an EMPTY success, and any message saying it checked nothing.

2. **Compare each guard's denominator against the repository's.** Ask what
   SOURCE produced the population, not whether the check passed. A guard that
   reads one file in a repo whose convention is one-file-per-crate is the shape
   to expect.

3. **Ask which guards assert against the LIVE tree**, not only on fixtures. Most
   do; the exceptions are where the coverage gaps hide.

4. **Poison it — and check the poison landed in the guard's POPULATION.** Two of
   three poisons on the sheet-presence check hit files it deliberately ignores,
   and each printed a green that could have been taken for proof.

5. **Ask WHICH LANE names each guard.** For every `scripts/check_*.py`, does the
   gate, the pytest lane, or CI mention it? On 2026-09-03: 14 of 16 reachable
   locally, and the two that were not (`check_doc_link_ratchet`,
   `check_zone_name_ratchet`) were CI-only — one of them holding two unread
   regressions. This is the pass that found entry #12, and it is the first
   entry on this page found on purpose rather than by accident.

7. **Build a crate under its OWN defaults.** `cargo check -p <crate>` with no
   feature flags is a build NO LANE PERFORMS: the workspace check unifies
   features on, and the union enables everything. On 2026-09-03, of 25 crates
   with an empty `default` and cfg-gated source, FOUR warned this way and every
   lane was clean. ⭐ It is the jab-string lesson in reverse — that test was
   green per-crate and red under the union; these are green under the union and
   red per-crate. Both say the same thing: a feature set is a PROGRAM.

6. **Ask which guards have a TEST OF THEIR OWN.** Three of sixteen had none,
   including `check_doc_links`, which the DEFAULT GATE runs. Writing one found a
   rule the module applied to one matcher of two. ⭐ Prioritise the guards whose
   best possible score is ZERO — they cannot distinguish "nothing is wrong" from
   "I measured nothing", so their population floor is the arm to test, and the
   floor is exactly what nobody writes a test for.

⛔⛔ **AND WHEN YOU LOOSEN A GUARD TO ADMIT A LEGITIMATE CASE, RE-RUN THE
POISON AFTERWARDS.** This is how a working guard quietly stops working, and it
happened on 2026-09-03 inside an hour of the guard being written. A carve-table
check required every path in a row to resolve; a row that named a HISTORICAL
path beside the live one failed for being more informative, so the rule was
loosened to "at least one path must resolve". ⇒ The original defect stopped
firing: that row's prose says *"NOT under `brain/`"*, and `brain/` counted as a
path that resolves, so the rule was satisfiable by the very word the row uses to
say where the file ISN'T. Only re-poisoning AFTER the change caught it.
⭐ The generalisation: a loosened predicate is a NEW predicate, and it inherits
none of the old one's evidence. Ask which of its old failures it still catches
before you believe it.

⛔⛔ **AND THE GUARD DOES NOT HAVE TO CHANGE FOR ITS POISON VERDICT TO GO
STALE — THE CODE UNDER IT IS ENOUGH.** Later the same day, an unmodified guard
had its implementation replaced beneath it, and the person holding it said so
before anyone asked: *"my earlier poison verdict was stale the moment the
implementation under the guard changed, and I would have been carrying it
forward as if it still meant something."* ⇒ A poison result is evidence about a
PAIR — this assertion against that implementation — and replacing either half
retires it. The loosening rule above is the special case where you changed the
assertion; this is the commoner one, where you did not touch the test at all
and it stopped meaning what it meant.

⭐⭐ **THE SHARPEST FORM, AND THE ONE WORTH REMEMBERING: A TEST ASSERTING THAT
TWO THINGS AGREE IS SILENTLY A TEST OF WHATEVER MAKES THEM AGREE.** The guard in
question asserted that a detector's verdict and its stated reason match. They
matched *usually* — both functions shelled out to the same external checker,
separately, over a mutable generated tree, so anything regenerating between the
two calls would have made them disagree for no defect at all. The guard did not
catch that; **it was resting on it.** ⇒ When a test asserts a relationship
rather than a value, ask what enforces the relationship, and whether that thing
is guaranteed or merely usual. If it is merely usual, the guard has a hidden
dependency and will one day fail for a reason that is not a defect.

⚠ Two habits that make it cheaper: a tool one call away beats an hour of reading
(`discover_all_targets()`, `grep -l <shared module>`), and when two of your own
measurements disagree, the coherent one is not automatically the true one.

## The three remedies, and which one you are actually reaching for

Reading the fixes together is more useful than reading any one of them, because
they are not the same kind of fix.

- **Make the job run.** #2 moved the wasm CHECK into the default plan; #3 made
  the no-warnings job pay for `--fresh`. This is the only remedy that adds
  coverage, and it is the most expensive, because it costs time on every run —
  505 s cold and 26 s warm for the wasm CHECK, and the team took that for the
  CHECK and refused it for the LINK. ⭐ Both fixes state the price **at the job**
  rather than in a commit message, which is what lets the next person re-decide
  it instead of rediscovering why it is slow.
- **Make the silence audible.** The dominant remedy. #7's footer now derives
  from the *planned* jobs rather than asserting; #2's `elif not only:` branch
  exists for no purpose but to print that the web build is UNCHECKED; and #1's
  fix kept a `--rust-alone` that still skips everything — but its help text now
  reads *"those went red unnoticed for a day the last time that happened."*
  ⭐ Note what that means: the blindness was not removed, it was **named and made
  loud**. That is a legitimate outcome, and it is the one to aim for when
  coverage is genuinely too expensive.
- **Accept that it cannot be fixed, and compensate elsewhere.** #5 and #6. A
  type check will never execute code and a native run will never be a wasm run.
  The only defence is a human knowing the gap exists, which is why it is written
  down here instead of filed as a bug.

## The machine you are on decides which of these are live

Members #2, #7 and #8 are not properties of the code alone — they fire or do not
fire depending on what is installed where you are standing.

⛔ **AND A STALE SUBMODULE CHECKOUT LOOKS EXACTLY LIKE REPOSITORY ROT.** Measured
2026-09-03: `test_the_committed_report_matches_a_fresh_generation` failed here
for a day — the committed runtime-frame-history reports **43** records and
regenerating produced **41** — and its failure message says the committed file is
stale and prints the command to regenerate it. ⇒ **Running that command would
have deleted two Ultra host captures and gone green doing it.** The report was
not stale; this checkout's `dev/ambition_dev_measurements` was, sitting at
`8a35405` while the superproject recorded `0255e29` on both the branch and
`main`. `git submodule update` fixed it and the Python lane went 1-failed →
**780 passed, 0 failed**.

⇒ **The tell is one character.** `git submodule status` prints `+` before the
SHA when the checked-out commit differs from the one the superproject records —
that `+` is the whole diagnosis, and it is easy to read past. ⚠ I reported this
failure to a coordinator FOUR times as a pre-existing repository defect before
looking at it. It was never in the repository; everyone else's gate was green.
⇒ Before calling a generated-artifact mismatch repo rot, check whether the
generator's INPUT is at the commit the repository asked for.

⚠ **And check every submodule, not the one that failed.** The same `git
submodule status` on calculex 2026-09-03 showed **two more** off-pointer —
`tools/ambition_music_renderer` (checkout `a113b786`, recorded `b2c005b5`, on a
branch named `agent/…`) and `tools/ambition_sprite2d_renderer` (`125adf81` vs
`aba1c1eb`). Neither breaks the default lane, because detached developer-tool  <!-- cite-ok: SUBMODULE commits, unresolvable in the superproject by design -->
tests are omitted from it. ⇒ **But `./run_tests.sh --tool-tests` on this machine
is not testing what the repository records**, and a green or red result from it
here would describe somebody's work-in-progress branch. Left alone deliberately:
an `agent/` branch in a submodule is someone's state, and syncing it to the
pointer would discard work no failure asked me to touch.

On the calculex VM on 2026-09-02, `rustup target list --installed` returned
exactly one target, `x86_64-unknown-linux-gnu`. No `wasm32-unknown-unknown`, no
`aarch64-linux-android`, `ANDROID_NDK_HOME` unset. A full green gate on that box
therefore carried **zero** web and **zero** Android coverage — and said so out
loud rather than in a footer claiming otherwise, which is #7's fix working on a
machine it was not written on:

```text
run_tests: SKIPPING the web build CHECK — the wasm32-unknown-unknown target is
not installed … The web build is UNCHECKED in this run, and a #[cfg] break on
that target is invisible to every other job.
```

⭐ **THEN `rustup target add wasm32-unknown-unknown` CHANGED THE ANSWER, in about
a minute.** The same commit, the same command, the same repository — different
coverage, because the machine changed. Nothing in the code moved. The Android
path (#8) did not change with it: it needs a device, not a toolchain, which is
what makes it the structural member and the web one the situational member.

⛔ **SO "THE GATE PASSED" IS NOT A PORTABLE CLAIM.** It is a claim about one
machine's installed toolchains at one moment. When you report a green gate to
somebody on different hardware, say which targets were installed, or you have
handed them member #7 in social form — a report that something was checked when
it was skipped. And when a target is cheap to install, installing it is a better
answer than documenting the gap.

## ⛔⛔ A CORRECT FIX CAN RAISE THE FAILURE COUNT — diff the CAUSES, not the total

When a schedule dies on the first bad system it meets, every later bad system is
INVISIBLE until you fix the first one. So failures are LAYERED, and the count is
a bad instrument for progress.

Measured on the feature union, 2026-09-03:

```text
before                            6,968 passed   48 failed
guard sync_portal_view_cones      6,980 passed   49 failed   ← UP, and correct
guard debug_portal_view_zones     6,991 passed   38 failed
guard attach_hit_flash_overlays   7,016 passed   13 failed
```

⛔ **Round one removed 37 failures and exposed 37 more.** On the total that is
"no progress"; to anyone watching only the number it is a REGRESSION, and the
obvious response — revert the fix — is exactly wrong. The right reading was in
the CAUSE MIX: `ConeRigAssets` had gone to zero and a new system had appeared.

⇒ **After any fix to a suite that dies on first failure, diff the causes:**

```bash
grep -oE "in system \`[^\`]+\`" run.log | sort | uniq -c | sort -rn
```

One line, every round, and it is the only thing that distinguished progress from
regression here.

⭐ **AND THE LAYERS CAN BE WRITTEN DOWN IN ADVANCE.**
[`../planning/engine/headless-verification.md`](../planning/engine/headless-verification.md)
records three of these hiding in succession — *"a missing
`Assets<TextureAtlasLayout>`, then `GizmoConfigStore`, then `Assets<Mesh>` …
each looking identical to the last"*. Those are the three systems above, in that
order. The agent who walked them had quoted that sentence in the commit that
fixed the first one and still guarded a single system out of a four-system
chain. ⇒ When a doc says failures come in succession, fix the whole CHAIN in one
pass and check every member's parameters — not the member that happened to fail.

⚠ **The corollary about deferred measurement.** "37 are fixed" was filed as an
inference from the class, with its bound stated honestly (verified on one
target; the union not re-run, because the run was expensive). The bound was
honest and the inference was still wrong — 48 went to 49, not to 11. An
expensive measurement deferred is not a measurement, and the case an inference
cannot see is precisely the layered one.

## Before you believe an error list, diff it against a clean checkout

`check_agent_kb.py` could not pass in an agent worktree. It used
`Path.resolve()`, which follows symlinks, and worktree seeding makes
`.agent/README.md` a symlink into the primary tree — so two phantom "links
outside repo" errors appeared **in every worktree and in no clean checkout**.

An external reviewer saw three errors. The session in the worktree saw five. Two
of them were its environment.

⭐ **A THIRD INSTANCE, 2026-09-03, and it is the purest form of this heading:**
`test_the_committed_report_matches_a_fresh_generation` failed here for a day and
in no clean checkout, because `dev/ambition_dev_measurements` was two commits
behind the pointer the superproject records. Reported to a coordinator four
times as a repository defect before anyone looked at the submodule. Full account
under [the machine-dependent section](#the-machine-you-are-on-decides-which-of-these-are-live),
including why the failure message's own suggested fix would have deleted data
and gone green.

⭐ **AND THE SAME TRAP HAS A FLAGS-SHAPED TWIN, met the same day.** The wasm32
CHECK was run here by hand — `cargo check -p ambition_app --lib --target
wasm32-unknown-unknown --no-default-features --features web_served_assets`. It
passed in 5m02s and emitted three warnings that looked like real rot:

| warning | why it is NOT a defect |
|---|---|
| unused imports `ITEM_GRID_COLS`, `ITEM_GRID_ROWS` | they ARE used, in code gated `#[cfg(feature = "kaleidoscope_menu")]` — a feature this invocation's `--no-default-features` did not enable |
| unused import `VisualQualityProfile` | same feature gate |
| `prefetch_preparations` is never used | it IS used, by `tests/neighbor_prefetch_prepares_rooms.rs` — which `--lib` does not build, and which the gate's `cargo check --all-targets` does |

Three warnings, zero defects, produced entirely by running a narrower command
than the gate runs. ⛔ The mirror of member #3 exactly: there, a WARMER build
hid warnings that existed; here, a NARROWER build invented warnings that did
not. Both are the same mistake — reading a diagnostic list without knowing what
produced it.

⛔ **THE GENERAL RULE: your error list is a property of your environment until
you have compared it with one you did not build** — and "environment" includes
your flags, not just your machine. This cuts both ways — the
extras may be phantom, and a clean checkout may also show you something your
warm tree has been hiding since member #3.

## ⛔⛔ And your OTHER WORK is part of that environment

The section above says an error list is a property of your environment until you
compare it with one you did not build. **Your own concurrent commands are in that
environment**, and this is the form that produces the most confident wrong
answer, because the failures name repository facts rather than machine ones.

⭐ **MEASURED 2026-09-04.** A feature-union run reported **7,104 passed / 32
failed**. Every one of the 32 was caused by recursive `grep`s run beside it,
which exhausted file descriptors. Thirty of them came from
`ambition_workspace_policy`, and the loudest read:

```text
could not find the workspace root above <repo>/tests/ambition_workspace_policy
 — no ancestor Cargo.toml declares [workspace]
```

of a workspace whose root manifest declares it on line one. **Nothing in the
output said "I could not read a file."**

⇒ **The two lessons are separable and both are load-bearing.**

1. **Do not read a long run you are sharing the machine with.** A union run is
   thousands of processes and file handles; a recursive search over the same tree
   is thousands more. "I was only reading" is not true of the operating system.
2. ⛔⛔ **A GUARD THAT SWALLOWS A READ ERROR REPORTS ITS OWN FINDING.** Every rule
   in a policy or census crate reports an ABSENCE — a missing owner, an unscanned
   root, a legacy function that is gone. So `if let Ok(text) =
   read_to_string(..)` makes the guard announce exactly what it exists to detect
   whenever the MACHINE, not the code, is what failed. The worst instance
   returned an EMPTY set on a read error with the comment *"a deleted legacy file
   has no pending functions"* — an IO failure reading as **"the migration is
   complete."** Fixed by `0b58767f2`: one reader owns the message, it names the
   path and the OS error and says *"do not read any policy verdict from this
   run"*, and only `ErrorKind::NotFound` is tolerated where deletion is a real
   answer.

⚠ **A "the scan reached no sources — vacuous" guard is the right instinct and is
NOT enough**, which two of the five sites had: a run where SOME files are
unreadable still contributes and silently under-scans. ⭐ The best-behaved of the
three failure messages in that run was
`the_reaction_timer_clock_forks_on_purpose`'s — *"the scan is broken, not the
code"* — which is the sentence every source-scanning assertion should be able to
print.

ⓘ This is the third instance in one day of the instrument sitting inside the
population it measures: a binary measured before it was rebuilt, a `pgrep -f`
that matched the shell containing its own pattern, and this. The family is worth
naming as one.

## ⛔ The filter you wrote yourself is the one you will not suspect

The section above says your error list is a property of your environment. The
sharper form, learned four separate times on 2026-09-02 by the person writing
this page:

> **A search that finds nothing has told you about your PATTERN until you have
> checked that the pattern can match what you are looking for.**

All four had the same shape and none of them looked alike at the time:

| what was searched | what the search could not find | what it "proved" |
|---|---|---|
| `check_planning_citations.py` poisoned with a bare `` `path.rs` `` | the checker only reads `` `path.rs:123` `` and `` `foo::bar` `` | "the checker ignores table cells" — it does not | <!-- cite-ok: a poison EXAMPLE, not a citation -->
| planning docs for `scripts/…` paths that exist | bare basenames (`tests.rs`, `fx.rs`) used as prose shorthand | "186 broken citations" — there were none |
| `cargo check --workspace` output through `\| tail` | the exit status, which a pipeline takes from its LAST command | "the lane is green" — it was RED |
| `(./run_tests.sh --rust > log 2>&1; echo "EXIT=$?" >> log)` **2026-09-03** | the LANE's status: a subshell exits with its LAST command, and that was the `echo`. The harness reported the wrapper's **0** while the log's own last line read `EXIT=1` | "the full Rust lane passed" — it was 5/6 jobs, and I had already told the coordinator it would be an exit code |
| asset paths matched with `sprites_[a-z0-9_]+/` | `sprites/`, the Full path, which has no underscore | "`AMBITION_QUALITY_PROFILE` does not work" — it works |

⛔ **AND THE EXIT-STATUS ROW HAPPENED TWICE, A DAY APART, TO THE SAME PERSON WHO
WROTE THIS PAGE** — first through `| tail`, then through a subshell ending in
`echo`. ⇒ Knowing the trap did not prevent it, because the second shape did not
look like the first: there was no pipeline. The invariant is not "beware pipes",
it is **the status you read belongs to the last thing that ran, which is rarely
the thing you care about**. Write the command's own status down before anything
else runs — `s=$?` on the next line, or `${PIPESTATUS[0]}` — and never after a
convenience `echo`.

⭐ **Three of the first four produced a FALSE NEGATIVE that read as a finding**, which
is the dangerous direction: a missing result feels like evidence of absence, and
absence is what this whole page is about. The fourth produced a false positive
and was caught in seconds.

⇒ **The cheap defence is a positive control.** Before believing a search found
nothing, run it against something you KNOW it should match. `grep -c` on a
pattern you expect to hit; poison the checker with the form it actually reads;
capture `PIPESTATUS` instead of trusting a pipeline's exit. Every one of the four
above would have taken under a minute to catch and cost between ten minutes and
a twenty-minute build.

⚠ And note where these landed: two of them were reported to a coordinator before
being caught. A wrong finding sent to somebody acting on it costs more than the
time to check it.

⛔⛔ **A SIXTH, 2026-09-03, AND IT IS THE WORST SHAPE: A CONFIRMER WHOSE FAILURE
READS AS A CLEARANCE.** The unused-dependency census uses a two-stage
instrument — a default-features lint that over-reports, then an `--all-features`
run to clear the deps that are only used behind a `cfg`. The confirmer was:

```
allf=$(timeout 900 cargo rustc -p "$crate" --lib --all-features -- -W unused_crate_dependencies 2>&1 | sed -n 's/…unused…/\1/p')
if echo "$allf" | grep -qx "$dep"; then CONFIRMED; else FEATURE-ONLY; fi
```

⇒ It captures only the WARNING LINES, so **an empty result cannot be told apart
from a clean build**. A crate that failed to compile, or hit the 900 s timeout,
yields no warnings — and every one of its flagged deps is then silently written
down as "used behind a feature, not removable". ⚠ The direction is what makes it
bad: the failure mode CLEARS things, and a cleared row is one nobody
re-examines. Had `ambition_dialog --all-features` timed out (it was 15 minutes
into a cold `ui`-feature graph when I noticed), three deps would have been
recorded as feature-conditional on the strength of a build that never finished.

⇒ **The fix is to read the STATUS, not the output** — the same invariant as the
exit-status rows above, arriving for a third time in a shape that again did not
look like the first two:

```
timeout 900 cargo rustc … > "$log" 2>&1; rc=$?
if [ $rc -ne 0 ]; then mark every dep UNKNOWN(rc); continue; fi
```

⭐ **What caught it was not a control.** No positive control would have fired —
the confirmer works perfectly on every crate that compiles. It was noticing that
the run had been going for fifteen minutes and asking *what does this script do
if that timeout expires?* ⇒ **For any two-stage instrument, ask which stage's
failure looks like a PASS.** A detector that dies is loud, because you get no
findings at all; a confirmer that dies is silent, because "nothing to report"
is also what success looks like.

## ⛔⛔ A test whose SUBJECT comes from a FIXTURE must assert the fixture supplied it

`the_same_seed_produces_the_same_fighter` guarded the fighter brain's replay
determinism: run two identically-seeded brains, compare the ticks they pressed
on, and a rollback resimulation is safe. It passed for the life of the project.

Measured 2026-09-10, before it was changed: **0 presses of 90 frames, and
`a.noise` still exactly the initial seed.** The runner handed the brain
`BrainSnapshot::idle()` — an empty `attack_kit` — so no attack was ever wanted,
the noise stream was never sampled, and the test compared **two empty vectors
and two seeds that had not moved.** It could not have failed for its stated
reason.

⇒ **The rule is not "test the shipped value". It is that a test whose subject is
supplied by a fixture must assert the fixture supplied it.** One line:

```rust
assert!(presses.iter().any(|p| *p), "the fixture produced no press, so the
        comparison below is between two empty vectors");
```

⚠ **This is a different defect from a fixture that measures the WRONG subject,
and it is worse.** The same file's noise fixtures all used
`execution_noise = 0.9`, a value no authored rung produces — that is a guard
certifying a population the game never seats, and it is bad. But it *ran*. Here
the subject **was never constructed at all**, and the assertion was still true.
A test can be wrong about what it measured; this one measured nothing and said
so in the affirmative.

⭐ **The shape is `assert_eq!(left, right)` where both sides come from one
helper** — determinism, order independence, replay safety, cross-backend parity.
It is one of the most common guards there is, and its single failure mode is
that an empty answer equals an empty answer.
`scripts/measure_floorless_equality_tests.py` screens for it: **7916 test bodies
across 1075 files, two hits.**

⚠ **A hit is a shape, not a verdict, and one of the two is sound.** Comparing
against a non-empty **constant** carries its own floor: if the built side came
back empty it would differ from the constant and fail. Read the survivors; do
not report the count.

⛔ **The other survivor shows the shape's worse cousin, where emptiness is the
lesser half.** `cross_backend_model_parity_inventory_and_system` reads:

```rust
let cube_pages = build();
let grid_pages = build();
```

One closure, no backend argument anywhere. It asserts `build() == build()` —
the purity of a single function — while its own doc claims *"the active tab's
`MenuPageModel` is built from the SAME backend-agnostic builders regardless of
which backend renders it."* ⇒ **The fixture ASSUMES the fact the test claims to
check**, so if a backend ever stopped using the shared builder the test stays
green. A test that constructs both sides of a comparison from one source is
measuring its own fixture.

⭐ **And the screen needed two guards of its own, both of which fired.** Its
first run scanned **0 files** — the `git grep` pathspec preceded the pattern —
and printed a clean bill of health; a corpus floor now refuses that. And it
carries a **positive control** that re-runs it against the sha that still had
the defect, because a screen narrowed until it recognises nothing produces
exactly the same output as a clean tree.

## ⛔⛔ The instrument was pointed correctly and the SHUTTER opened at the wrong moment

Five defects in one night, across three subsystems, and not one of them was a
wrong value. Each was a correct reading of the right thing taken at the wrong
moment, or over the wrong population, or against the wrong vocabulary — and in
every case the wrong answer was indistinguishable from a right one.

**1. A mutable field read at the end of a run.** `FighterState::new(cfg, seed)`
stores the seed **in** `state.noise` and advances it on every draw. A probe read
`state.noise` after 3613 ticks and reported it as the seat's seed; it is a
stream POSITION. ⇒ The check built on it — *"the two seats have distinct
seeds"* — was sound in exactly one direction: equal end positions do imply a
shared stream, but two seats can share a seed and end apart merely by drawing a
different NUMBER of samples, which is what a divergent bout does. **It was green
on every row we had and blind on every row we were about to collect.**

⇒ **A mutable field read at the end of a run is not the value it was
initialised with, and a name that describes the initialisation — `noise`,
`seed`, `state` — will not tell you that.**

**2. A git ref read one cache layer short of the remote.** `git log
origin/main..HEAD` is the standard "have I pushed?" check, and it reads
`refs/remotes/origin/main` — a file on your disk that a plain `git pull` does
not always move. Two agents disagreed for twenty minutes about whether a commit
existed. `git branch -r --contains <sha>` asks the remote. ⇒ **A verification
step can be one cache-layer short of the thing it claims to verify, and it
reports confidently on the layer it actually reached.**

**3. A per-seat fact printed through a survivorship filter.** The same probe
recorded each seat's brain at BIRTH — precisely to avoid defect 1 — then printed
those birth facts from a loop iterating an END-OF-BOUT query. A seat knocked out
before the end is not in that query, so it printed no line at all. At the top
rung nobody dies and the hole was invisible; one rung down, a fighter scoring 5
knockouts silently lost a seat. **The clock was fixed and the population was
left.** ⇒ **A per-seat fact printed from a per-entity query inherits that
query's survivorship, and nothing in the printing code says so.**

**4. A knob validated against the wider of two vocabularies.** A rung override
was given a panic so a bad value could not fail quietly, and it validated
`1..=9` because the brain ladder's `for_level` clamps there. But the roster
seats a rung by naming a **published policy**, and only five of the nine exist.
Asking for rung 8 refused every seat and returned `0 of 3600 ticks (decided on
None)` after a full-length run — per fighter. ⇒ **Two vocabularies for one
concept, and the validator was pointed at the wider one. A knob that accepts a
value the composition cannot seat has not been validated; it has been
type-checked.**

⛔⛔ **AND ALL FOUR FAILED IN THE SAME DIRECTION: an EMPTY RUN, not a wrong
number.** A missing probe line, a sentinel with no verdict of its own, a
zero-tick bout, a refused seat. Every one arrived at the consumer as
`UNMEASURABLE`, and **a wall of UNMEASURABLE reads as a careful null result** —
nobody re-examines an instrument that declined to answer. ⇒ **Absence of output
is a value, and every parser assigns it a meaning whether or not the author
chose one.** The fix that caught all four was capturing the failure's REASON and
not just the fact of it: *"the match was decided"* and *"the seating never
happened"* must not share a verdict.

⚠ **A REPAIRED SENTENCE IS THE LEAST SUSPECTED SENTENCE IN THE FILE.** When
rung 8 first came back empty, the sweep's summary line was rewritten to say *"a
rung-8 CPU decides the match"* — a correction, made carefully, and wrong,
because the real cause was not yet known. It then printed under three more
tables before anyone re-read it. ⇒ **A correction inherits the confidence of the
thing it replaced.** Re-check the lines you have already fixed once; they are
the ones nobody looks at twice.

⚠ **What none of these would have been caught by.** Not review — each line is
correct in isolation. Not a poison of the value — the values were right. The
first two were caught by a second party reading the same number and asking where
it came from. The rest were caught by **running the instrument ONCE, on a subject
outside the regime it had always been used in, before trusting it fifteen
times.** ⇒ **Before a sweep, run one row in the corner of the parameter space
you have never visited.** A knob only ever used at its default has only ever
been tested there.

## ⛔⛔ A property measured only on the ACCUSED always looks like a clue

One character out of twenty-one could not be measured; the harness died two
seconds into her bout. Investigating her moveset turned up something striking:
**all six of the VFX effect ids she names are defined nowhere in the tree** — no
`.rs`, no `.ron`, nothing. A distinguishing feature, on the one subject that
fails, pointing at an unresolved reference falling through to a spawn path.

It is evidence of nothing. **Nobody's are defined.** `electric_arc`,
`gear_scatter`, `evidence_ping` — every other fighter's ids come back with the
same zero, and those fighters measure clean.

⇒ **The property was real, the measurement was correct, and it distinguished
nothing — because it was only ever measured on the accused.** That is a
different failure from a wrong value or a stale conclusion, and it has the worst
incentive structure of the three: you are investigating a subject, so you
measure that subject, and any unusual thing you find arrives already looking
like an explanation.

⭐ **The test is one command and it kills candidates fast: measure one
NON-accused subject before believing any distinguishing feature.** Four
candidates for that failure died to it in an afternoon, including the one held
with most confidence:

| candidate | killed by |
|---|---|
| she uses a lot of VFX | another fighter uses **more**, and is clean |
| her effect ids look undefined | the same grep returns **zero for every fighter** — and was later shown wrong about all of them (below) |
| her sprite sheet is 7.8MP and decodes mid-gameplay | another loads **7.5MP** mid-gameplay, flagged by the same log line, and is clean |
| she dies and a death despawns something | another scores **5 knockouts** in one bout without failing |

⚠ **Note what each row of that table costs: one run of one other subject.** None
of these needed a theory, a reading of the code, or an argument — they needed a
control, and the control was always cheap and always available.

⛔ **And the failure compounds when it reaches a report.** "Her effect ids are
undefined" was about to be filed as a second finding scoped to her — so it was
widened to the corpus, where it looked much stronger: *nothing in the tree
defines any of them, across every fighter.* **Before writing a distinguishing
feature into a row, ask what the denominator is.**

⛔⛔ **AND THEN THE WIDENED FINDING WAS REFUTED TOO, WHICH IS THE PART THAT
GENERALISES FURTHEST. ALL NINE IDS ARE DEFINED.** They ship in the sound bank and
in every tier of the spritesheet manifests. The grep missed them because **the
CONSUMER spells the bare row and the OWNER spells it compositely**:
`vfx_at(.., "rune_burst", ..)` against `vfx.generic_exotic.rune_burst` in
`sfx.bank.txt`. Two instrument defects in one line — it required the id in
quotes, and it searched `*.rs` and `*.ron` while the registration lives in a
`.txt` manifest. `vfx_at`'s own doc says so and was read past: *"The bank ships
one `vfx.<family>.<row>` cue per authored row."*

⇒ **A census keyed on the CONSUMER's spelling cannot see the OWNER's
registrations.** And the widening — correct, disciplined, exactly the fix for the
sampling error above — **did not touch the defect, because both readings shared
one instrument.** ⛔ **Widening a population does not repair an instrument that is
looking in the wrong place; it makes the wrong answer bigger, and the size feels
like corroboration.** The corpus-wide zero was more convincing than the
single-subject zero and just as false.

⚠ **What survives is much smaller, and stating so is how it stays that way.**
`vfx_cued`'s doc says an id neither the registry nor the packed bank authorises
is *"counted and dropped, not heard — so a typo here is silence."* ⇒ **There IS a
mechanism, and it OBSERVES rather than REFUSES.** Whether that is the right
policy for authored content is a real question and a small one — nothing like
"the whole flourish layer reaches nothing", which is what two rounds of widening
had built.

## ⛔⛔ A PROPOSAL and a RECEIPT are one tense apart

A planning page argued for sealing a component's construction and ended the
argument like this:

> *A constructor that takes what an occurrence needs, with the fields sealed,
> **turns** seven minting authorities into one.*

The seal was then built, and the queue row was marked done on that sentence.
**It describes what a fix WOULD buy; it was read as what the fix DID buy.** The
seal is real and useful — it made the writer set enumerable, which is how the
codec and `serde` writers a grep misses were found — but the callers still mint
the occurrence's identity, custody, provenance and attempt state themselves.
**Four of the five facts never moved.**

⇒ **This is not overclaiming, which is what makes it dangerous.** Nobody wrote a
false sentence. **A forward-looking claim and a completion claim are one tense
apart, and English hides the difference** — *"turns"* is equally at home in a
proposal and in a receipt, and a careful reader has no way to tell which one a
page is holding.

⭐ **The remedy is cheap and mechanical: a proposal says WOULD; a receipt names a
COMMIT.** A sentence with neither is a sentence whose tense the next reader will
guess, and they will guess "done", because a page that has been edited recently
reads as a page describing the present.

⚠ **Every "⇒ this is the edge in the packet" paragraph has this shape.** They are
written in the argumentative present precisely because they are arguing — and
they sit in the same document as the receipts, under the same headings, in the
same voice.

## ⛔⛔ TWO HONEST GREENS, ONE TREE, CONTRADICTING EACH OTHER

On 2026-09-10 two agents measured the same commit within minutes of each other.
One reported **"178 blocks, 7703 passed, 0 failed"**. The other reported **seven
failing tests**. Neither was wrong, neither had a stale checkout, and the trees
were identical.

**They ran different lanes.** `cargo test --workspace` and
`scripts/run_tests.py --rust` do not execute the same set: the `--rust` lane
carries the repo-coupled pytest guards and a `--maintenance` sub-run, and the
workspace build carries neither. ⇒ **The seven reds were invisible to a green
that had been quoted four times.**

⭐ **A verdict's reference point has TWO halves and this repository had been
recording one.** A commit says *which tree*; a lane says *which questions were
asked of it*. **"7703 passed at `<sha>`" is not a claim about the repository** —
it is a claim about one lane at one commit, and it reads exactly like the
stronger claim.

⚠ **This is the same failure as a number crossing a document boundary without
its method, arriving through a gate instead of a table.** The remedy is
identical and just as cheap: **a green result names its lane.**

⛔ **And the asymmetry matters: a lane you did not run is a guard that does not
exist.** The bracketing worked here only because two agents happened to disagree
out loud. **One agent, one lane, one green would have shipped all seven.**

## ⛔⛔ The BLAST-RADIUS PROBE measured a narrower change than the one that shipped

Before widening an admission check, an author probed the cost: point the
**backstop** road at an empty support table, run everything, see what breaks.
The answer was **"178 blocks, 7703 passed, 0 failed"**, and it was reported —
correctly, for what it tested — as *no composition was relying on the unchecked
reading.*

The change that then shipped was wider: the argument stopped being optional, so
**every caller that had passed `None` also got a real check**, not just the
backstop. Full workspace at the shipped change: **`7693 passed, 11 failed`.**

⇒ **The probe was honest and the sentence it produced was quoted about a
different change.** *"No composition was relying on it"* is true of the backstop
road and false of the direct callers — and nothing in the sentence says which
road it was measured on.

⭐ **A blast-radius measurement is a measurement of a SPECIFIC EDIT, and it
expires the moment the edit widens.** The remedy is the same as for every other
number in this page: **the reading travels with what was changed to produce
it**, not with the intention behind it. *"Empty blast radius"* means nothing;
*"empty blast radius with the backstop pointed at an empty table"* means
something and visibly does not cover the wider edit.

⚠ **And the shipped failures were the change WORKING** — fixtures that prepare a
cast with no technique handlers installed now correctly withhold characters
naming native effects. **A probe that had measured the shipped edit would have
predicted them.** The defect was never in the code; it was in the scope of the
sentence.

## ⛔⛔ A CHECK THAT CANNOT BE BUILT, AND THE REASON IS NOT ITS PRECISION

**Measured 2026-09-10 and RETIRED. Do not propose this rule again without
reading this section.** Reproduce with
`python3 scripts/measure_citation_symbol_roles.py [--adjacent]`.

**THE PROPOSAL.** `check_planning_citations.py --roles` reports a line citation
that lands inside a `#[cfg(test)]` region. The class it reports is MIXED — a row
can name a test deliberately, or count a test as a production writer, reader or
definition — so it reports and does not gate. Two rows in that report shared a
narrower shape that looked machine-checkable: they put a backticked SYMBOL beside
the citation, and every occurrence of that symbol in the cited file was inside a
test. ⇒ *Report that, and you catch the rows that overstate a population without
reading any prose.*

**WHAT THE COUNTS SAID.** Three versions, each killed by its own measurement:

| pairing | hits | why it fails |
|---|---|---|
| symbol and citation on the same LINE | 11 | 3 are TAUTOLOGIES: the symbol IS a test function's name, so "every occurrence is a test" is true by construction |
| minus the tautologies | 8 | 6 are CROSS-PAIRING. A long table row holds several citations and many symbols; pairing all with all invents pairs no author or reader would connect |
| symbol within 40 chars of the citation | 2 | it LOSES one of the two rows that motivated it, and one survivor is a false positive |

⛔ **AND IT ACCUSES ROWS THAT ARE CORRECT.** `smash-parity-inventory.md:766`
cites `hit_response.rs:98` for `HitReaction` — correctly — and
`hit_reaction.rs:293` for a different claim; `hit_reaction.rs` has its
`#[cfg(test)]` at 472, so that citation is production and the row is right.
`george_grab_dash` appears only in a test because the running form is DERIVED by
`dash_stance_verb` rather than authored, **which is what its row says.**

⭐⭐ **THE DECISIVE OBJECTION IS STRUCTURAL, AND IT IS NOT ABOUT PRECISION.** The
counts above could be survived — a larger corpus, a looser window, and they move.
This cannot:

> **A row that deliberately names a test writes it in backticks beside the
> citation. So does a row that mis-cites a test as production. THE SHAPE IS HOW
> HUMANS WRITE BOTH.**

`tracks.md:125` satisfies every condition of the rule and is a correct row. ⇒
Prose is the only thing that separates the two classes, so a rule built
specifically to need no prose **still needs prose — it just fails less visibly.**

⚠ **AND THE OBVIOUS PATCH IS NOT AN ANSWER.** Excluding symbols that are
themselves test function names removes that example, and it is tempting to call
the objection handled. It is not: a deliberate row can name a PRODUCTION symbol
whose only use in the cited file is a test. **The arm narrows false positives; it
does not separate the classes.**

⇒ **The class is real — two rows prove it — and it is found by READING.**
`--roles` already narrows the reading to fifteen rows, which is the whole value
it can honestly deliver.

⭐ **THE TRANSFERABLE RULE: ask whether a CORRECT case also matches, before
asking how many cases match.** A population count can be survived by a bigger
corpus. A correct row that matches cannot be tuned away, and it is usually
cheaper to look for.

## What this page cannot do

It cannot make a gate honest. Every member above was found by a person asking
one of the four questions about a specific job, and the gate is still the thing
that will tell you it is green. The honest claim is narrow: these are the shapes
that have actually lied in this repository, so that the next one is recognised
rather than rediscovered.

Related: [`cheapest-sufficient-check.md`](cheapest-sufficient-check.md),
[`../reviewer-guide.md`](../reviewer-guide.md) (§Testing).
