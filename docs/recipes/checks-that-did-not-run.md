# The check that was correct and did not run

A test that fails is information. A test that never executed and reports green
is worse than no test at all, because it also spends the attention that would
have found the defect by hand.

⭐ **AND IT HAS A SIBLING FAMILY, which #10 below already notices when it calls
itself "a DIFFERENT SPECIES from 1-9"** — the check that ran perfectly and could
not have failed. It has its own section below, and its instances live in a
journal.

This page is the dual of
[`cheapest-sufficient-check.md`](cheapest-sufficient-check.md). That page asks
*what is the least I can run to settle this change*. This one asks *did what I
ran actually run* — and it exists because on 2026-09-02 a single day's work
turned up SEVEN members of the same family in one gate script; an eighth was
already sitting in the backlog unrecognised, and a ninth surfaced the same night.
A tenth, eleventh and twelfth followed on 2026-09-03.

⭐ **THE FINDING IS NOT THE COUNT. It is that most of them were found by
accident** — by running a suite for an unrelated reason, by an external reviewer
reading source, by running `cargo check` by hand before the gate. Not one was
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

## The twelve, and what each one teaches

| # | the check | how it lied | status |
|---|---|---|---|
| 1 | `./run_tests.sh --rust` | skipped the whole Python lane, so the rollback stable-name ratchet, the codec-shape baseline and a stale `MODULES.md` sat red **for a day** behind a gate reporting 4/4 GREEN | fixed `2945f3381` |
| 2 | the wasm build job | gated on `if not only and everything` — the exhaustive plan only, so a default run never checked `wasm32` at all | fixed `b85b4db20` |
| 3 | `check_no_warnings.py` | parses diagnostics instead of setting `-D warnings`, so it reuses the build fingerprint — and **cached crates do not re-emit warnings**. In a warm tree it reports clean while real warnings exist | fixed `234bcc686` — the gate job is now `check_no_warnings.py --fresh`, and the price of the cold re-fingerprint is stated at the job |
| 4 | the repo-tooling lane | simply not invoked by the flag anybody was using | fixed `2945f3381` |
| 5 | the wasm CHECK | is **TYPE-ONLY**. `cargo check` cannot see a `#[cfg]` that removes BEHAVIOUR rather than breaking a build | ⛔ **STRUCTURALLY LIVE** |
| 6 | "web persona BOOTS" | runs the web composition **NATIVELY** (`--features visible_web_base`, native target), so it compiles the `not(wasm32)` branch — and its `if not only and everything:` gate puts it in the exhaustive plan only | ⛔ **STRUCTURALLY LIVE, TWICE.** ✔ **RUN 2026-09-03 on the calculex host (no GPU): it SURVIVES startup** — route `ambition_launcher`, 23 UI nodes, 10 UI texts, 0 sprites, 2 cameras, simulation host `Rollback`; 16 m 01 s to build. ⚠ That closes NEITHER half: the run is still native, so it still compiles the `not(wasm32)` branch, and the gate still plans it only in the exhaustive plan. What it does establish is that the job passes when someone runs it, which had not been checked |
| 8 | the Bevy 0.19 **Android font path** | is TYPECHECKED, NEVER RUN. The port deleted the hand-rolled `seed_android_system_fonts` and turned on Bevy's `system_font_discovery` for `android_platform` instead. ⛔ Its whole job is to find fonts the HOST does not have, so a desktop green says nothing about it | ⛔ **STRUCTURALLY LIVE.** Recorded in `../planning/tracks.md`; closing it needs a device, not a build |
| 9 | the **16 `image_stages` tests**, including the reveal-readiness guard | exist only under `--features bevy`. `ambition_asset_manager`'s DEFAULT features exclude `bevy`, so the module does not exist in `cargo test -p ambition_asset_manager`: **56 tests run, not 83**. The gate's feature-union job would cover them — but it is built inside `if everything:`, so it is EXHAUSTIVE-PLAN ONLY | ⛔ **STRUCTURALLY LIVE, AND FAR BIGGER THAN THIS ROW — 783 TESTS ACROSS 29 CRATES**, measured 2026-09-03 with `scripts/feature_gated_tests.py` (which already existed): the 16 here are one module of a class that includes 53 in `ambition_content`'s `portal`, 26 in `ambition_input`'s `local_seats` and 25 in `ambition_app`'s `grid_backend`. The union job that runs them is inside `if not only and everything` in `run_tests.py`, so a DEFAULT green says nothing about any of them. The gate's coverage footer named the gap qualitatively and gave no magnitude; it now states the count, and `test_the_gate_states_how_many_tests_it_skips.py` ratchets it so the figure cannot rot. Found 2026-09-02 when a new test in that module printed `running 0 tests` and PASSED. ✔ **RUN 2026-09-03 on the calculex host, and every number reproduces: 83 with `--features bevy`, 56 without, 16 of the difference in `image_stages` — and all 83 PASS.** So the blindness is not currently hiding a failure, which is worth knowing and is NOT the same as it being fixed: the gate still does not run them, and the next break here is still invisible to it |
| 10 | `[census] owners` (and its sibling `owners_in`) | is a **TOP-20**. The row prints `crates=82` and then names twenty, so a reader who greps it for a crate and finds nothing cannot tell *registers no systems* from *ranked 21st* — and the emitter's own doc comment says the row answers *"should a shipped title carry this at all"*, which is an ABSENCE question. Absence was uninformative for 62 of 82 crates while looking authoritative | fixed 2026-09-03 — both emitters now append `+N_more_not_shown`. ⚠ A DIFFERENT SPECIES from 1-9: not a gate that skipped, an instrument that answered a narrower question than it appeared to |
| 11 | `./run_tests.sh --rust` itself | **exits 2 having run NOTHING** when the Python lane's `tree_sitter_rust` is missing, and says so in a voice that reads as informational: *"this interpreter cannot run the Python lane … affected: 1 planned job(s) … fix: scripts/setup/python_tools.sh"*. One affected job aborts the whole RUST lane, and the header it prints is indistinguishable from a normal preamble — a reader who does not check `$?` sees a run that appears to have started | fixed on this host by `scripts/setup/python_tools.sh`; the lane then ran 6957 tests. ⚠ **STRUCTURALLY LIVE ELSEWHERE**: any host missing that tool gets the same silent no-op |
| 12 | `#[ignore]` as a parallel-safety marker | is re-enabled wholesale by `--heavy`, which runs `cargo test --workspace --include-ignored`. `#[ignore]` conflates TWO unrelated reasons — *slow, run on demand* and **invalid unless run alone** — and `--include-ignored` cannot tell them apart. `parallax_theme_retires_on_walk` says so in its own header: `ambition_app` has ONE `[[test]]` target, so every file under `tests/` is a module of `app_it` sharing a process, and *"a sibling booting its own app would populate `Assets<Image>` underneath this one's assertions"*. Under `--heavy` it runs beside 6957 others | ⛔ **STRUCTURALLY LIVE.** The failure mode is the bad one: not a red test but a GREEN one whose assertions were satisfied by somebody else's app. Its real driver is `scripts/measure_parallax_retire.sh`, which runs it alone with an exact filter |
| 7 | the coverage footer | said `- the wasm/web build LINK (the wasm CHECK ran)` **unconditionally**, while the job is appended only `if wasm_target_installed()`. No target → no web job, all green, exit 0, and a report that it was checked | fixed `159e76ba8` |

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
thirty-seven instances from the same two nights, each with the commit that
fixed it, are tabulated in
[`../../dev/journals/blind-checks-2026-09-03.md`](../../dev/journals/blind-checks-2026-09-03.md).
⇒ **Do not add that count to the ten above** — different question, different
population. #10 is the boundary case and belongs to both lists.

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

## Before you believe an error list, diff it against a clean checkout

`check_agent_kb.py` could not pass in an agent worktree. It used
`Path.resolve()`, which follows symlinks, and worktree seeding makes
`.agent/README.md` a symlink into the primary tree — so two phantom "links
outside repo" errors appeared **in every worktree and in no clean checkout**.

An external reviewer saw three errors. The session in the worktree saw five. Two
of them were its environment.

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

## ⛔ The filter you wrote yourself is the one you will not suspect

The section above says your error list is a property of your environment. The
sharper form, learned four separate times on 2026-09-02 by the person writing
this page:

> **A search that finds nothing has told you about your PATTERN until you have
> checked that the pattern can match what you are looking for.**

All four had the same shape and none of them looked alike at the time:

| what was searched | what the search could not find | what it "proved" |
|---|---|---|
| `check_planning_citations.py` poisoned with a bare `` `path.rs` `` | the checker only reads `` `path.rs:123` `` and `` `foo::bar` `` | "the checker ignores table cells" — it does not |
| planning docs for `scripts/…` paths that exist | bare basenames (`tests.rs`, `fx.rs`) used as prose shorthand | "186 broken citations" — there were none |
| `cargo check --workspace` output through `\| tail` | the exit status, which a pipeline takes from its LAST command | "the lane is green" — it was RED |
| asset paths matched with `sprites_[a-z0-9_]+/` | `sprites/`, the Full path, which has no underscore | "`AMBITION_QUALITY_PROFILE` does not work" — it works |

⭐ **Three of the four produced a FALSE NEGATIVE that read as a finding**, which
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

## What this page cannot do

It cannot make a gate honest. Every member above was found by a person asking
one of the four questions about a specific job, and the gate is still the thing
that will tell you it is green. The honest claim is narrow: these are the shapes
that have actually lied in this repository, so that the next one is recognised
rather than rediscovered.

Related: [`cheapest-sufficient-check.md`](cheapest-sufficient-check.md),
[`../reviewer-guide.md`](../reviewer-guide.md) (§Testing).
