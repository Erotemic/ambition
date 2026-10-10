# The cheapest command that settles a change

Pick the row for what you changed. Run it. Read what the row says it does not
cover. Stop.

`./run_tests.sh` looks like the safe default, and it keeps agents from running
the focused test that matters. Knowing which narrow command is SUFFICIENT is the
fix. The GitHub workflow runs only on manual dispatch, and the maintainer runs the
exhaustive sweep periodically. Running it mid-edit duplicates that sweep and adds
no safety.

## The matrix

| what you changed | run this | what it does NOT cover |
|---|---|---|
| a doc, a plan | per edit, the planning guards (`python3 scripts/check_planning_citations.py --strict`, `python3 scripts/check_planning_anchors_resolve.py`, `python3 scripts/check_doc_links.py`); at a push, the repo tooling job, which runs them and reads `AGENTS.md` and the goal file | a crate: no crate compiles prose |
| Rust inside ONE crate, no public API moved | `cargo test -p <crate>` | anything composing that crate; feature-gated tests |
| anything the app composes | `cargo check -p ambition_app` | behaviour. A per-crate `cargo check` can be green while the app build fails. |
| anything a `#[cfg(feature = …)]` gates | `cargo test -p <crate> --features <f>` (or `cargo check -p <crate> --features <combo> --all-targets`) | that the app still builds; other feature combinations |
| a crate SEAM: a trait, a re-export, a dependency edge, a registration | `cargo check -p ambition_app`, then `cargo test -p ambition_app --test app_it -- <module>` | a defect that exists only where two CONTENT crates meet |
| a rollback registration, a schedule pin, a message channel | `cargo test -p ambition_app --test app_it -- rollback_` **and** `python3 -m pytest scripts/tests/ -q` | feature-gated channels: only the union job compiles them |
| session death, a checkpoint restore, room replay, a sandbox reset, or the rollback host | the row above, **and** the demo host lane in [`running-the-heavy-app-it-lane.md`](running-the-heavy-app-it-lane.md#the-demo-host-apps-have-their-own-lane) | a network transport. `app_it` does not run a demo app's tests, and a demo fixture can read an instrument that no `app_it` arm reads. |
| a new param on a shared `SystemParam` struct | `git grep -l '<the system>'`, then `cargo test -p` **each crate that names it** | nothing else. See "Hand-built apps" below. |
| Bevy app WIRING (plugins, systems, ordering) | `cargo test -p ambition_app --test app_it -- <module>` | Pin `TimeUpdateStrategy` in any new test app, or it measures machine load. |
| authored CONTENT (LDtk, catalogs, characters) | `cargo test -p ambition_app --test app_it -- declared_art_resolves registered_character_art` | that it plays; use `capture_scene` |
| anything you can SEE | `cargo run -p ambition_app_tools --bin capture_scene -- --route <id> out.png 1280x720 --warmup N` | correctness; it proves only what is drawn |
| generated assets or a regen script | the regen script, then the guard it feeds | another session may regenerate the same tree |
| Python tooling or a guard in `scripts/` | `python3 -m pytest scripts/tests/ -q` | everything Rust |
| LDtk tooling | `tools/ambition_ldtk_tools/.venv/bin/python -m pytest tests -q` (from that directory) | anything that consumes what it emits |
| a runner or workspace-wide change | `./run_tests.sh` | feature-gated tests, the consumer fixtures, the wasm link |
| ONE job in a lane failed and you fixed it | `./run_tests.sh <lane> --only-job "<substring of the job name>"` | every sibling job in that lane |
| features, the SDK surface, or the web path, before a release | `./run_tests.sh --run-everything-you-probably-dont-need-this` | `#[ignore]`d tests and acceptance cycles; add `--heavy` |

Notes:

- **Some rows are enforced.** `python3 scripts/required_checks.py` reads the
  paths your branch changed since `origin/main` and names the checks these
  rows require: the repo tooling job for a doc, `cargo test -p <crate>` for each crate you changed, the demo
  host lane for death, restore, replay and rollback-host paths, the repo
  tooling job for `scripts/`, the LDtk tool tests for that tool, the two
  content arms for `game/ambition_content/assets` and the map assets, and the
  `rollback_` arms with the repo tooling job for a rollback registration or
  `sim_phase_pins.rs`. A check named by test name counts for a run whose
  filter is part of that name, or that had no filter. A check counts only when `run_tests.py` recorded
  it as passed on a tree with your change in it
  (`target/lane_ledger.jsonl`); a plain `cargo test` records nothing. It
  prints the `./run_tests.sh` command for each check that is missing. It
  reports; a push does not wait for it (Q166). The other rows are still
  prose.
- **To go faster, go narrower, not to another lane.** `--rust` keeps the two
  heavy jobs (`workspace (default features)` and the `capture_scene` acceptance
  run) and drops the no-warnings check, doc links, planning citations and the
  compile-cost ratchet. Use it when you want the Rust verdict now and the doc
  gates later.
- **A warm, filtered `app_it` run takes about a second.** The slow part is the
  relink after an `ambition_app` source edit. Edit less of `ambition_app`.
- **Do not quote a wall-clock figure for a lane.** Costs depend on the machine
  and build state. Ask the plan (`./run_tests.sh <lane> --list`, which also
  accepts `--only-job`) and the ledger
  (`dev/ambition_dev_measurements/run_tests_cost.jsonl`).
- **Do not run the exhaustive plan out of caution.** Run it when a row names your
  change.
- **Do not skip `repo tooling (scripts/tests)`.** It runs
  `test_every_contract_holds_against_the_live_tree` in
  `scripts/tests/test_absence_contracts.py`, which is the only check that catches
  a registration or a dependency edge in the wrong place.
  `python3 scripts/check_absence_contracts.py` exits 1 on a violation, with or
  without `--check` (before 2026-10-04 the run without the flag exited 0).

## Why each caveat is there

- **Two content crates meeting.** Two demos each registered the same component
  for rollback. Each demo's tests passed and `cargo check -p ambition_app`
  passed. The composed app panicked on the first frame.
- **Feature-gated channels.** Message channels behind a feature sat outside both
  rollback oracles because the default job did not compile them.
- **`TimeUpdateStrategy`.** Under `Automatic`, `app.update()` advances by real
  time. A test that asserts a distance or a count then measures how busy the
  machine is.
- **Another session regenerating the tree.** A long run can outlive its inputs;
  `include_str!` fails on files that exist before and after but not during.
- **Hand-built apps.** Apps that `add_systems(…)` a system by hand also register
  its messages by hand. A new `MessageWriter` param then fails runtime parameter
  validation ("Message not initialized") in those apps, while the owning crate
  builds and tests clean. A hand-listed set of dependencies is a population;
  `git grep` finds it.

## Sweeping failures: one big run, then targeted only

Run the big suite once. Fix each failure and verify it with a targeted run
(`cargo test -p <crate> --test <target> <filter>`). Do not re-run the full suite
to confirm a fix the targeted run already confirmed. The next periodic sweep
catches anything else.

## Interlace architecture and feature work

Alternate architecture tasks and feature tasks on purpose. A feature complaint is
a report from the place where the whole stack is assembled, and it often exposes
an engine defect that architecture work cannot see.

## Write ahead in a worktree, build on the main tree

Every job reads the live tree, so a running suite freezes editing. Write the next
change in a second worktree while the main tree builds. See
[`../tools/agent-worktrees.md`](../tools/agent-worktrees.md).

- In an agent run, hand a subagent an independent task with
  `isolation: "worktree"`. Give it files you are not editing, and say so in its
  prompt. `cd` to the repository root first: a nested repository such as
  `tools/ambition_sprite2d_renderer` gives the subagent the wrong tree.
- Do NOT build in the worktree. A second `target/` is a full cold duplicate and
  the volume is shared.
- Mirror the assets first: `python3 scripts/mirror_assets_for_worktree.py`.
  Generated art is gitignored. An assetless tree compiles an empty sheet table,
  and unrelated tests fail.

## When a run is slow, get the distribution

`--report-time` is nightly-only. On stable, run serially and timestamp the
output:

```
cargo test -p <pkg> --test <target> <filter> -- --test-threads=1 --nocapture \
  | python3 -u -c "
import sys, time
t0=time.time(); last=t0; cur=None; rows=[]
for line in sys.stdin:
    if line.startswith('test ') and ' ... ' in line:
        now=time.time()
        if cur: rows.append((now-last, cur))
        cur=line.split(' ... ')[0][5:]; last=now
for d,n in sorted(rows, reverse=True)[:15]: print(f'{d:7.1f}s  {n}')
"
```

A suite total divided by a test count is not a per-test cost. The wall clock of a
parallel run cannot go below its longest test; suspect one long pole before a lock.

## Waiting on a long command

- Read the state the command WROTE. For the suite that is
  `target/run_tests_status.json`: `state` (running, done, aborted, incomplete,
  crashed), `current_job`, `current_started`, and `completed`. Check `state`, not
  only `failed`. A run the disk floor stopped has an empty `failed` list and
  `aborted` with `never_ran`. A lane that could not run at all is `incomplete`
  with `unrunnable`. `scripts/last_test_run.py` applies these rules and refuses
  rather than guessing.
- Every run appends its cost to
  `dev/ambition_dev_measurements/run_tests_cost.jsonl`.
- ⛔ Do not wait with `pgrep -f <pattern>` or `until ! pgrep -f <pattern>`. The
  waiting shell's command line contains the pattern, so it matches itself and
  never ends. Wait on something that ends: a backgrounded command's exit, `wait
  <pid>`, or the status file's terminal `state`. If you must use `pgrep`, write
  `pgrep -f "[r]un_tests.py"`, and run `pgrep -af '<pattern>'` once to check that
  your own shell is not in the output. Do not `tail -f` a file that can be
  deleted.

## When the suite refuses on disk headroom

`scripts/check_disk_headroom.py` blocks a run below 40 GB free.

1. Run `scripts/setup/target_bindmount.sh --status` first. A huge target is often
   an absent bind; repairing the bind returns the space.
2. If the bind is present, cleanup under `target/` is allowed (`cargo clean`,
   `cargo clean -p <crate>`). The volume is shared: delete only what is yours.
   If the bind is absent, delete nothing; report the numbers and stop. See
   `AGENTS.md`.
3. On a short volume, run `./scripts/clean_workspace_crates.sh --incremental-only
   --apply` first. It reclaims the incremental directories and rebuilds nothing.
4. Then `scripts/sweep_target.py` can sweep precisely. Its mark step is a build,
   so it needs headroom to run.

`fixtures/external_consumer` has its own `.cargo/config.toml` and target. Only
the exhaustive plan's `external consumer: outlander` job uses it, so it is a
cheap thing to report for reclaim.

## Prose has no guard

Tests, the citation checker and the link checker do not read a sentence and ask
whether it is true. Prose is verified by being used. Rules:

1. **When you cite a line, open it.** `check_planning_citations.py` checks that a
   file exists, not that the line shows what the prose names. Cite full paths; a
   bare basename can be ambiguous.
2. **When you write prose that names a behaviour, grep the behaviour.**
3. **Name the test, or do not claim one.** "Pinned by a test" with no name is
   unfalsifiable and often false. A named test dangles when it is renamed.
4. **Naming a deleted function is fine when the sentence says it is gone.**
   Naming it in the present tense as the current mechanism is the defect.
5. **When you touch a comment that claims sole authority, re-derive the claim.**
6. **Read the verdict line, not the exit code or the tail.**
   `check_planning_citations.py` is non-gating and exits 0 with findings unless
   given `--strict`. Decide which line proves the outcome before you run the tool:

   ```bash
   python3 scripts/check_planning_citations.py > /tmp/cit.log 2>&1
   grep -E 'checked|unresolved|all resolved' /tmp/cit.log
   ```

7. **Before you conclude that a guard is not enforced, look for a wrapper.** A
   test that imports the checker (for example
   `scripts/tests/test_absence_contracts.py`) can enforce it without any lane
   passing `--check`.

### Doc links are the one prose check

rustdoc resolves intra-doc links. `scripts/check_doc_link_ratchet.py` runs
`cargo doc` as a ratchet over a hand-kept list of crates, in `--maintenance`.

- **A carve breaks doc links silently.** Prose moves to a crate where its names
  are not in scope. In the carve's own commit, add the destination crate to the
  ratchet's `CRATES` list and run
  `python3 scripts/check_doc_link_ratchet.py --check`.
- **A `pub(crate)` item cannot be the target of a cross-crate doc link.** Write
  prose that names the crate and module, and say that it is deliberately not a
  link.
- A broken-link count mixes three populations: fixable by repointing, unfixable
  by visibility, and actually wrong. Only the last is a false claim.

### Do not gate on line numbers

A line-number gate can decide only "past end of file", and nothing in the tree is.
A symbol-proximity check catches real drift but is about 50% precise, because
prose often cites a USE site. Run such checks by hand as audits. Before you build
a gate, ask what it could DECIDE.

### Vocabulary for findings about fields

```text
DORMANT   a field authored content never turns on      a design choice
DEAD      a field nothing reads                        a defect
STRANDED  a field carried end to end with no consumer,
          under prose claiming one                     a defect, hardest to find
```

## Related

- [`checks-that-did-not-run.md`](checks-that-did-not-run.md): did what you ran
  actually run, and could it fail.
- [`re-measuring-a-planning-claim.md`](re-measuring-a-planning-claim.md): how a
  re-measurement goes wrong.
