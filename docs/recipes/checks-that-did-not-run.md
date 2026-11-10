# When a verification result is misleading

Use this recipe when a specific test, check, or report gives a result that does
not match the behavior you can observe. It is a diagnostic reference, not a
checklist for every code change. Routine verification follows
[`cheapest-sufficient-check.md`](cheapest-sufficient-check.md) and the
**Avoid bullshit guardrails** policy in `AGENTS.md`.

## First identify what happened

A green result can mean different things:

- **Not executed:** the lane, feature, target, or machine skipped the check.
- **Executed against the wrong subject:** the fixture was empty, the query
  omitted the relevant code, or a guard matched nothing.
- **Executed correctly but reported too broadly:** a compilation result was
  described as gameplay acceptance, or a subset was described as the whole
  repository.

Start with the actual command, exit code, enabled features, selected jobs, and
subject population. The evidence should answer the question you are asking.

## Fast diagnosis

1. **Read the process exit status.** `cmd | tail` gives the status of `tail`
   unless `pipefail` is enabled. Save the command output to a file when
   inspecting a long run.
2. **Check whether the job was selected.** `./run_tests.sh --list` shows the
   planned jobs. The default, heavy, maintenance, and exhaustive plans are not
   interchangeable. The runner's status file and coverage footer report skipped
   work; a clean count covers only work that ran.
3. **Check the real population.** Source-only scans miss `tests/`, `examples/`,
   or feature-gated code. A tool that scans generated or ignored files measures
   the current machine as well as the repository.
4. **If a result appears empty, inspect one known subject.** Confirm that the
   command can find what it claims to inspect. If the tool cannot see it, fix
   the query or the check rather than adding an assertion around its empty
   output.
5. **Distinguish execution from behavior.** `cargo check` verifies compilation
   of selected code; it does not execute the wasm app or an Android input path.
   An app constructed for a test may not advance its fixed tick on every
   `app.update()`.

When checking a predicate once, a temporary control or deliberately changed
input can be useful. **Do not turn that one-off probe into a permanent
meta-test** unless it protects an important behavior and justifies its cost.

## Historical runner cases

These fifteen cases explain why a particular earlier verification claim was
wrong. Their numeric identifiers appear in other repository notes. The status
column is a historical observation, not a promise about current `HEAD`.

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
| 9 | feature-gated tests | Tests behind non-default features (for example `ambition_asset_manager`'s `image_stages` under `--features bevy`) run only in the feature-union job, which is in the exhaustive plan only. `scripts/measure/feature_gated_tests.py` counts them (several hundred across many crates). | Structurally live. The gate footer states the count; `scripts/tests/test_the_gate_states_how_many_tests_it_skips.py` ratchets it. |
| 10 | `[census] owners` / `owners_in` | Printed a top 20 under `crates=82`. Absence from the row looked authoritative and meant nothing. | Fixed. Both rows print every owner. |
| 11 | `./run_tests.sh --rust` with no `tree_sitter_rust` | Exits 2 having run nothing. The message reads like a normal preamble. | Fixed by `scripts/setup/python_tools.sh` on a provisioned host. Live on any host without the tool: check `$?`. Advice that names a tool (`python3`) and not a path inherits the reader's environment, so the runner prints `sys.executable`. |
| 12 | CI-only ratchets (`check_doc_link_ratchet`, `check_zone_name_ratchet`) | Only CI invoked them, so local work never saw their regressions. | Fixed. Both run in `./run_tests.sh --maintenance`. Every `scripts/check_*.py` must be reachable from a lane, CI, or a pytest arm that aims at the real tree. |
| 13 | `./run_tests.sh --heavy` | `--include-ignored` re-enabled tests that must run alone and probes that panic by design. | Fixed. Probes are named `probe_*`; the heavy job runs `--include-ignored --skip probe_ --test-threads=1`. Guard: `scripts/tests/test_probe_tests_are_named_probe.py`. |
| 14 | `check_planning_citations.py --vanished` | Only its own unit test invoked it. A tested instrument that nobody aims at the corpus covers the function, not the corpus. | Fixed. It runs in `--maintenance` with `--strict` (without it, findings exit 0) against the fixed ref `PLANNING_VANISHED_BASELINE`. Guard: `scripts/tests/test_maintenance_vanished_job.py`. |
| 15 | `./run_tests.sh --maintenance` vs `pytest scripts/tests` | The lane runs the repo-hygiene checkers but not the tests of those checkers. | Fixed. The lane notice derives its omissions from the plan and names `scripts/tests`. Guard: `test_the_maintenance_notice_names_every_job_that_lane_drops`. |

The detailed case records are in
[`dev/journals/blind-checks-2026-09-03.md`](../../dev/journals/blind-checks-2026-09-03.md).
Read them when the current symptom resembles one of the cases, not before
ordinary development.

## Common mechanisms

### Checks that accidentally match nothing

Text guards can become blind when Rust syntax or formatting changes, when a
source file moves, or when a list of enum names stops covering new variants.
Prefer compiler-visible types, package boundaries, and semantic APIs when
those express the rule. If a source scan is still needed, inspect its inputs
and decide what an empty result means. Do not create a guard to police another
guard as a routine remedy.

### Checks that compare two empty or dependent values

A parity assertion can pass when both sides are empty or derive from the same
incorrect source. Before relying on it, identify the actual gameplay state it
exercises. A permanent regression test is justified by a meaningful failure,
not merely by the availability of an additional assertion.

### Fixed-step and rollback scenarios

A render-frame call is not necessarily a simulation tick. A headless test that
needs exact simulation time should pin the time strategy or step the actual
simulation harness. For rollback comparisons, deliver inputs through the path
the rollback host replays. Compare state at the same authoritative tick.

### Shell and machine state

- A pipeline or subshell can hide the program's exit code. Record that code.
- A warm incremental build can omit diagnostics emitted by a fresh build.
- A missing wasm target or Android device means that target did not run.
- Submodule pointers, ignored generated assets, and local target storage can
  differ from another agent's checkout.
- A source-text search cannot find references that exist only under another
  spelling, generated registration, or compiled feature.

## Reporting

State the command, revision, selected features, and result that actually ran.
Record `NOT RUN` when a required target or lane did not execute. Distinguish
that from `FAIL` and from a real `PASS`.

A one-time diagnostic need not become an executable guard or an additional
suite member. Repair the source behavior or the misleading check, record the
finding where it will help, and continue development.

Related: [`re-measuring-a-planning-claim.md`](re-measuring-a-planning-claim.md),
[`../concepts/testing-and-validation.md`](../concepts/testing-and-validation.md).
