# The check that was correct and did not run

A test that fails is information. A test that did not execute and reports green
is worse than no test. It also uses the attention that would find the defect by
hand.

This page is the dual of
[`cheapest-sufficient-check.md`](cheapest-sufficient-check.md). That page asks
*what is the least I can run to settle this change*. This page asks *did what I
ran actually run, and could it fail*.

There are three families:

1. **The check did not run.** A lane, a plan or a machine skipped it, and the
   report still reads as coverage.
2. **The check ran and could not fail.** The guard is sound in form but its
   population, input or fixture is empty or wrong.
3. **The check was correct and the claim about it was wrong.** The measurement
   was good. The sentence written about it said more than was measured.

Families one and two are checkable by tools. Family three has no automated
detector. Do not write a script for it. Its remedy is a second reader who knows
what was measured and reads what was claimed.

Re-check a member against the current `HEAD` before you repeat it. A catalogue of
gate holes is a claim about a changing repository.

## The question before the four

**Did anyone run a check at all?** A check that nobody ran in the window where it
mattered has the same outcome as a blind check: nobody knows.

A conflict-free merge can carry a semantic conflict. If one side renames a module
file and the other side keeps the `mod` line, Git merges both sides cleanly and
the crate does not compile. After a merge whose stat shows a `rename … => …`,
compile the affected crate.

## The four questions

Ask these of any check before you trust it. Ask them together: one job in the
table below is blind in two independent ways, and each question alone gave a
reassuring answer.

1. **What does this flag skip?** A scope flag is a promise about what you are
   NOT running. That promise is often only in the help text.
2. **Which plan is this job in?** A job in the exhaustive plan and not in the
   default plan does not run on the command people type.
3. **Does it re-derive, or does it reuse a cache?** A scan over incremental
   build output sees only what recompiled.
4. **What would this look like if the feature were silently off?** If the answer
   is "the same, and green", the check measures the harness.

Cite a job by its name string or its gate expression, not by a line number in
`scripts/run_tests.py`. Line numbers change on every edit.

## The fifteen, and what each one teaches

The numbers are stable. Code and scripts cite them (for example "member 11").

| # | the check | how it lied | status |
|---|---|---|---|
| 1 | `./run_tests.sh --rust` | Skipped the whole Python lane, so Python ratchets stayed red behind a green Rust lane. | Fixed. `--rust-alone` still skips them, and its notice derives the omissions from the plan. |
| 2 | the wasm build CHECK | Was in the exhaustive plan only, so a default run never checked `wasm32`. | Fixed. It is in the default plan when the target is installed. |
| 3 | `check_no_warnings.py` | Parses diagnostics. A cached crate does not re-emit warnings, so a warm tree reported clean. | Fixed. The gate runs `check_no_warnings.py --fresh` and states the cost at the job. |
| 4 | the repo-tooling lane | The flag people used did not invoke it. | Fixed with #1. |
| 5 | the wasm CHECK | Is type-only. `cargo check` cannot see a `#[cfg]` that removes behaviour. | Structurally live. |
| 6 | "web persona BOOTS" | Runs the web composition natively (`--features visible_web_base`), so it compiles the `not(wasm32)` branch. It is also in the exhaustive plan only. | Structurally live, twice. |
| 7 | the coverage footer | Said the wasm CHECK ran when the target was missing and the job was not planned. | Fixed. The footer derives from the planned jobs. |
| 8 | the Android font path | Bevy `system_font_discovery` under `android_platform` is type-checked and never run. A desktop green says nothing about fonts the host does not have. | Structurally live. Closing it needs a device. |
| 9 | feature-gated tests | Tests behind non-default features (for example `ambition_asset_manager`'s `image_stages` under `--features bevy`) run only in the feature-union job, which is in the exhaustive plan only. `scripts/feature_gated_tests.py` counts them (several hundred across many crates). | Structurally live. The gate footer states the count; `scripts/tests/test_the_gate_states_how_many_tests_it_skips.py` ratchets it. |
| 10 | `[census] owners` / `owners_in` | Printed a top 20 under `crates=82`. Absence from the row looked authoritative and meant nothing. | Fixed. Both rows print every owner. |
| 11 | `./run_tests.sh --rust` with no `tree_sitter_rust` | Exits 2 having run nothing. The message reads like a normal preamble. | Fixed by `scripts/setup/python_tools.sh` on a provisioned host. Live on any host without the tool: check `$?`. Advice that names a tool (`python3`) and not a path inherits the reader's environment, so the runner prints `sys.executable`. |
| 12 | CI-only ratchets (`check_doc_link_ratchet`, `check_zone_name_ratchet`) | Only CI invoked them, so local work never saw their regressions. | Fixed. Both run in `./run_tests.sh --maintenance`. Every `scripts/check_*.py` must be reachable from a lane, CI, or a pytest arm that aims at the real tree. |
| 13 | `./run_tests.sh --heavy` | `--include-ignored` re-enabled tests that must run alone and probes that panic by design. | Fixed. Probes are named `probe_*`; the heavy job runs `--include-ignored --skip probe_ --test-threads=1`. Guard: `scripts/tests/test_probe_tests_are_named_probe.py`. |
| 14 | `check_planning_citations.py --vanished` | Only its own unit test invoked it. A tested instrument that nobody aims at the corpus covers the function, not the corpus. | Fixed. It runs in `--maintenance` with `--strict` (without it, findings exit 0) against the fixed ref `PLANNING_VANISHED_BASELINE`. Guard: `scripts/tests/test_maintenance_vanished_job.py`. |
| 15 | `./run_tests.sh --maintenance` vs `pytest scripts/tests` | The lane runs the repo-hygiene checkers but not the tests of those checkers. | Fixed. The lane notice derives its omissions from the plan and names `scripts/tests`. Guard: `test_the_maintenance_notice_names_every_job_that_lane_drops`. |

Standing rules from the table:

- **A test that only the exhaustive plan runs runs only when somebody already
  suspects a problem.** Ask whether the check exists in the DEFAULT plan.
- **A lane's green is a claim about that lane.** The lane notice is the only
  place that claim gets its boundary, so derive the notice from the plan. A
  hand-written notice drifts.
- **Use a fixed baseline for `--vanished`, never a rolling window.** Advancing
  `PLANNING_VANISHED_BASELINE` is the act of accepting a triage. It belongs in
  the commit that clears the rows.
- **A sentence that names a deleted symbol on purpose carries a `cite-ok`
  marker.** Writing about removals triggers the vanished check often. Budget the
  marker into the edit. Mark the row; do not advance the baseline to quiet it.
- **Do not aim `--vanished` at `docs/adr/`, `docs/learning/` or other historical
  or teaching corpora.** An ADR describes the code on the day of the decision.
  Teaching pages name Rust and Bevy items that are not in this tree. The check's
  domain is the pages that describe current behaviour.
- **Make the job run, make the silence audible, or compensate elsewhere.** Making
  a job run costs time on every run, so state the price at the job. When
  coverage is too expensive, print that the target is UNCHECKED. When a gap
  cannot close (#5, #6, #8), keep it written down here.

## The sibling family: it RAN, and it could not have failed

The other family executed perfectly and asked the wrong question. Its
forty-seven instances, each with the commit that fixed it, are in
[`../../dev/journals/blind-checks-2026-09-03.md`](../../dev/journals/blind-checks-2026-09-03.md).
Do not add that count to the table above. The populations are different.

### A parser inherits vocabulary, not meaning

An emitter tells you what a line contains. It does not tell you what to compare
it against. A parser written from the emitter reproduces its words and loses its
ordering, thresholds and population bounds. Before you use a parsed number, read
the emitter's filter (for example "prints only decodes ≥ 1.0 MP").

### A source-text guard has inputs it does not control

`rustfmt` line wrapping and Rust construction rules (for example
`#[non_exhaustive]` removing struct-literal syntax) change the text a guard
matches. The dangerous direction is a guard that goes blind and reports "no
offenders". Ask of every source-text guard: *if the scan matched nothing, would
it still pass?*

Remedies, strongest first:

1. **Give it a second input of a different kind.** The `*_it_sync` guards
   compare `mod <name>;` lines against a directory listing. If one side goes
   blind, the other side reddens the test.
2. **Match a spelling the language makes canonical.** For example, match
   `GroundItem::` for a sealed type, not an open literal any caller can vary.
3. **Use Rust's shape, not a list of names.** An associated function is
   `Type::snake_case(`; a method is `value.snake_case(`.
4. **Otherwise add an anti-vacuity floor** so blindness is loud.

The sweep and its population are in
[`../planning/engine/source-text-guard-exposure.md`](../planning/engine/source-text-guard-exposure.md).

When a generator script writes a guard's regex, use a raw string in the
generator. In a plain Python string `\b` is a backspace byte, and the written
pattern looks correct and cannot match. Scan generated files for control
characters (`{c for c in text if ord(c) < 32}`) before you believe either
verdict.

### A guard keyed on a classification sees only what the classification encodes

If a guard copies an enum's semantics as a list of strings, a new variant cannot
break the guard. Put the predicate on the enum (for example
`RollbackEntryKind::feeds_peer_checksum`) so that a new variant does not compile
until someone answers the question.

- **Needing an exception list to describe a change you just made is the tell.**
  Check whether the taxonomy is missing a member first.
- **A new kind shrinks every population keyed on the old one.** Change the
  taxonomy and its readers in one commit.
- **Show coverage with a poison pair**, not with an assertion that the guard has
  no blind spot. Break the value and break the registration separately. Each
  poison must redden a different arm.

### A vacuous guard wants a floor; an inert subject wants a reader

Both print green, from opposite causes.

- **Vacuous guard:** the scan under it saw nothing, which is also what a healthy
  tree looks like. Ask: *would this still pass if the scan matched nothing?*
  Add a floor.
- **Inert subject:** the assertion is true about a stored fact that nothing in
  production reads. Ask: *who, outside the test, consults this?* If nobody does,
  you found dead state, not a weak test.

### A test whose subject comes from a fixture must assert the fixture supplied it

`assert_eq!(left, right)` where both sides come from one helper passes when both
sides are empty. Determinism, order-independence and parity tests have this
shape. Add one line that proves the subject exists:

```rust
assert!(presses.iter().any(|p| *p), "the fixture produced no press, so the
        comparison below is between two empty vectors");
```

A test that builds both sides of a comparison from one source measures its own
fixture. `scripts/measure_floorless_equality_tests.py` screens for the shape. A
hit is a shape, not a verdict: comparison against a non-empty constant carries
its own floor. Read the hits; do not report the count.

### An assertion whose subject the system never computed

A report field that echoes the caller's input (for example `ticks_run:
max_ticks`) makes `assert_eq!(report.ticks_run, 8)` a tautology. The test name
then claims what the assertion does not check.

Headless apps built with `MinimalPlugins` use `TimeUpdateStrategy::Automatic`, so
`app.update()` steps the fixed schedule by elapsed wall time, not by call count.
The session world arrives on frames, not ticks: `settle_until_session_world` can
return `Ok` with zero fixed steps. After a settle loop:

- an assertion about STRUCTURE (an entity exists, a route is active) is honest
  without a pinned clock;
- an assertion about a value the FIXED schedule computes is not.

To test fixed-schedule values:

1. Pin the clock at the fixture:
   `app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep))`, where
   `timestep` is `Time<Fixed>`'s own. One frame is then one tick.
2. Deliver ticks, not frames. The first `update()` runs `Startup` and steps
   nothing. Loop until the tick budget is met, with a bounded frame budget.
3. Count what ran with a `FixedUpdate` system that increments a resource.

A test that passes on wall time the startup banked can fail on a faster machine.
The risky population is code that runs the production loop in a test process.

### A green that claims an improvement

A ratchet that reports every tracked row repaired at once probably measured
nothing. Ask what it would print if the measurement did not happen. If the
answer is the same page, the improvement is not evidence. `check_doc_link_ratchet.py`
refuses an all-zero result against a banked baseline; `--update` stays exempt so
a real universal repair is a deliberate act.

### A guard that parses a tool's human output owns a contract it did not write

Colour, message format and cached replay all change cargo's rendering, and each
turns a line-anchored parser into a silent zero. `scripts/run_tests.py` exports
`CARGO_TERM_COLOR=always` to child jobs.

- Read cargo through the one owner, `scripts/lib/cargo_output.py`
  (`plain_env()`, `COLOR_NEVER`, `strip_ansi()`).
  `scripts/tests/test_cargo_diagnostics_are_read_plain.py` requires every script
  that invokes cargo and anchors on `warning`/`error` to import it.
- Key on the part of the output that carries meaning (a location
  `--> path:line:col`, a crate name), not on a prefix only one rendering has.
- Prove a fixed parser with a defect it must see, run against the old and new
  versions. A fixed guard that is green on a clean tree proves nothing.

### A guard that swallows a read error reports its own finding

Policy and census rules usually report an absence. If the guard treats a read
error as an empty file, an IO failure reads as "the migration is complete".
Tolerate only `ErrorKind::NotFound`, and only where deletion is a real answer.
On any other error, name the path and the OS error, and say that no verdict from
this run is valid.

### Two-stage instruments and relation tests

- **Ask which stage's failure looks like a PASS.** A confirmer that captures only
  warning lines cannot tell a crate that failed to build (or timed out) from a
  clean crate. Read the status, not the output. Mark rows UNKNOWN on a non-zero
  status.
- **A test that asserts two things agree is a test of whatever makes them agree.**
  Ask what enforces the relationship and whether it is guaranteed or only usual.

### Poisons

- **A poison must change the ANSWER, not only the code.** Pick an input on which
  the two implementations differ. A poison that passes is a finding about the
  arm.
- **Check that the poison landed in the guard's population.** A poison in a file
  the guard ignores prints green.
- **When you loosen a guard to admit a legitimate case, run the poison again.** A
  loosened predicate is a new predicate.
- **A poison verdict is evidence about a pair: this assertion against that
  implementation.** Replacing either half retires it.

## Reading what a run says

### The status you read belongs to the last command that ran

- `cmd | tail` returns `tail`'s status. A subshell that ends in `echo` returns the
  `echo`'s status. Write the command's own status first: `s=$?` on the next
  line, or `${PIPESTATUS[0]}`.
- Do not pipe a long run into `| tail`. `tail` writes only at EOF, so a live run, a
  hung run and a dead run look the same. Redirect to a file and read the file
  (`wc -l`, `grep`) while it runs.

### The runner's footer is not the runner's verdict

- Read the exit status first and the prose second. `16/16 jobs passed` counts the
  jobs that ran. Look for `INCOMPLETE: ... did not run` below it.
- The machine-readable record is the authority: `target/run_tests_status.json`
  while a run is live, and
  `dev/ambition_dev_measurements/run_tests_cost.jsonl` afterwards (`per_job`,
  with the failing job and the commit).
- A resource limit turns a suite into a smaller suite that still reports a clean
  ratio. The disk floor drops expensive jobs. Before you reclaim space, run
  `scripts/setup/target_bindmount.sh --status`. If `target/` is bound, it is the
  agent's and `cargo clean` is the tool. If it is not bound, report and stop.

### A correct fix can raise the failure count

A schedule that dies on the first bad system hides every later one. After a fix
to such a suite, diff the CAUSES, not the total:

```bash
grep -oE "in system \`[^\`]+\`" run.log | sort | uniq -c | sort -rn
```

When a doc says failures come in succession (for example the missing
`Assets<TextureAtlasLayout>`, then `GizmoConfigStore`, then `Assets<Mesh>` in
[`../planning/engine/headless-verification.md`](../planning/engine/headless-verification.md)),
fix the whole chain in one pass. An expensive measurement that you defer is not a
measurement.

### Your error list is a property of your environment

Compare it with one you did not build before you believe it.

- **Worktrees.** Worktree seeding makes some files symlinks, so a
  `Path.resolve()` check can fail in every worktree and in no clean checkout.
- **Flags.** A narrower command than the gate runs (`--lib`,
  `--no-default-features`) invents "unused" warnings for code used by tests or
  other features. A warmer build hides warnings that exist.
- **Your own concurrent work.** Recursive searches beside a long test run can
  exhaust file descriptors. The failures then name repository facts ("no ancestor
  Cargo.toml declares [workspace]"). Do not read a tree heavily while a long run
  uses it.
- **Submodules.** `git submodule status` prints `+` before a SHA that differs from
  the recorded pointer. Before you call a generated-artifact mismatch repo rot,
  check that the generator's input is at the recorded commit. Do not run a
  "regenerate" command a failure message suggests until you check this; it can
  delete data and go green. Check every submodule. Do not sync a submodule that
  is on someone's `agent/` branch; report it.
- **Generated output.** A floor over generated, untracked output (for example
  the baked spritesheet record index) is a claim about the machine as much as
  about the tree. Do not lower the floor to pass on one machine.

### A search that finds nothing has told you about your pattern

Before you believe an empty result, run the search against something you KNOW it
must match. Use `grep -c` on an expected hit, poison a checker with the form it
actually reads, and capture `PIPESTATUS`. A false negative feels like evidence of
absence, and absence is what this page is about.

## The machine you are on decides which members are live

Members #2, #7 and #8 depend on installed toolchains and devices. A green gate on
a host without `wasm32-unknown-unknown` carries no web coverage; the runner
prints `SKIPPING the web build CHECK` in that case. `rustup target add
wasm32-unknown-unknown` changes the answer in about a minute.

"The gate passed" is not a portable claim. When you report a green gate, say
which targets were installed. When a target is cheap to install, install it.

## Claims about correct measurements (family three)

- **A green result names its lane and its commit.** `cargo test --workspace` and
  `scripts/run_tests.py --rust` run different sets. "N passed at `<sha>`" is a
  claim about one lane.
- **A blast-radius probe measures a specific edit.** It expires when the edit
  widens. Quote the reading with the change that produced it.
- **A proposal says WOULD; a receipt names a commit.** "Turns seven authorities
  into one" reads the same in a proposal and in a receipt. The next reader will
  guess "done".
- **A count keeps its population in the same sentence.** See
  [re-measuring a planning claim](re-measuring-a-planning-claim.md).
- **Ask the composed app, not a source file.** A list in the tree is a claim
  about the composition. For a population (seatable characters, authored moves,
  schema rows), ask the running composition: `PreparedCharacterRegistry`,
  `MovesetContract`, `RollbackRegistry::schema_dump()`.
- **Measure one non-accused subject before you believe a distinguishing
  feature.** A property measured only on the failing subject always looks like a
  clue.
- **Widening a population does not repair an instrument that looks in the wrong
  place.** A census keyed on the consumer's spelling (`"rune_burst"`) cannot see
  the owner's registration (`vfx.generic_exotic.rune_burst` in a `.txt` bank).
- **Check when and over what the reading was taken.** A mutable field read at the
  end of a run (`state.noise`) is a stream position, not the seed. A per-seat
  fact printed from an end-of-run query inherits that query's survivorship.
  `git log origin/main..HEAD` reads a local ref; `git branch -r --contains <sha>`
  asks the remote. A knob validated against a wider vocabulary than the
  composition seats has been type-checked, not validated.
- **Absence of output is a value.** Every parser gives it a meaning. Record the
  reason for an empty result so that "decided" and "never ran" do not share a
  verdict.
- **Before a sweep, run one row in a corner of the parameter space you have not
  visited.**
- **A corrected sentence is the least-suspected sentence in the file.** Re-check
  lines you have already fixed once.

## Deleting a named seam leaves references no compiler checks

Policy TOML, `contains = [...]` clauses, baseline files, planning prose, agent
instructions and scan roots name symbols as data. Before you commit a deletion,
run:

```sh
git grep -n 'DeletedSymbolName'
```

Fix each hit outside the diff in the same commit. When the hit is a policy or a
guard, also fix the sentence that explains it. Do NOT restore the deleted thing
to satisfy a stale reference. Remove the clause that names it.

## Retired proposal: pairing a backticked symbol with a test-region citation

Do not propose a rule that flags a citation inside `#[cfg(test)]` when a
backticked symbol beside it occurs only in tests. A correct row that names a
test on purpose has the same shape as a row that mis-cites a test as
production. Only prose separates them. `check_planning_citations.py --roles`
narrows the reading to a short report and does not gate. Reproduce the
measurement with `python3 scripts/measure_citation_symbol_roles.py [--adjacent]`.

General rule: ask whether a CORRECT case also matches before you ask how many
cases match.

## Running this audit yourself

Cheapest first:

1. **Run every guard and read its real exit code.**
   ```bash
   for f in scripts/check_*.py; do
     out=$(timeout 240 python3 "$f" 2>&1); code=$?     # NOT `| head`
     printf '%-42s exit=%-3s %s\n' "$(basename "$f" .py)" "$code" "$(printf '%s' "$out" | head -1)"
   done
   ```
   Look for a traceback, an empty success, and any message that says it checked
   nothing.
2. **Compare each guard's denominator with the repository's.** Ask which source
   produced the population.
3. **Ask which guards assert against the live tree**, not only on fixtures.
4. **Poison each guard**, and check that the poison landed in its population.
5. **Ask which lane names each guard.** For each `scripts/check_*.py`, does the
   gate, the pytest lane or CI reach it? Search for the module name as well as
   the file name: callers `import check_x`.
6. **Ask which guards have a test of their own.** Prioritise guards whose best
   score is zero. Their population floor is the arm to test.
7. **Build a crate under its own defaults.** `cargo check -p <crate>` with no
   feature flags is a build no lane performs. A feature set is a program.

## What this page cannot do

It cannot make a gate honest. Each member was found by a person who asked one
of the four questions about a specific job. These are the shapes that have lied
in this repository, so that the next one is recognised.

Related: [`cheapest-sufficient-check.md`](cheapest-sufficient-check.md),
[`../reviewer-guide.md`](../reviewer-guide.md) (§Testing).
