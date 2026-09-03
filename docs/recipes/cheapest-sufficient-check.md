# The cheapest command that settles a change

This is the durable residue of the completed test-iteration campaign; the full campaign record is archived at `docs/archive/planning-superseded/2026-08-13/test-iteration-cost-2026-08-02.md`.
Jon, 2026-08-02: *"run_tests looks so alluring to an agent, it prevents it from
running the focused test that actually matters, and instead it just runs all the
junk."* A faster front door does not fix that. Knowing which narrow command is
SUFFICIENT does, and that is what this page is.

Pick the row for what you changed, run it, read what the row says it does not
cover, and stop. There is no CI; the exhaustive sweep has an owner and a cadence
(Jon, periodically), and running it mid-edit duplicates that sweep rather than
adding safety.


  | what you changed | run this | what it does NOT cover |
  |---|---|---|
  | Rust inside ONE crate, no public API moved | `cargo test -p <crate>` | anything composing that crate; feature-gated tests |
  | anything a `#[cfg(feature = …)]` gates | `cargo test -p <crate> --features <f>` | that the app still builds; other combinations |
  | a crate SEAM: a trait, a re-export, a dependency edge, a registration | `cargo check -p ambition_app` then `cargo test -p ambition_app --test app_it -- <module>` | ⛔ a defect that only exists where two CONTENT crates meet — no per-crate job can see it (see the `Empowered` double-registration, 2026-08-03) |
  | a rollback registration, a schedule pin, a message channel | `cargo test -p ambition_app --test app_it -- rollback_` **and** `python3 -m pytest scripts/tests/ -q` | feature-gated channels: only the union job compiles those |
  | Bevy app WIRING (plugins, systems, ordering) | `cargo test -p ambition_app --test app_it -- <module>` | ⚠ pin `TimeUpdateStrategy` in any new test app or it measures the machine's load |
  | authored CONTENT (LDtk, catalogs, characters) | `cargo test -p ambition_app --test app_it -- declared_art_resolves registered_character_art` | that it plays; use `capture_scene` |
  | anything you can SEE | `capture_scene --route <id> out.png 1280x720 --warmup N` | correctness; it only proves what is drawn |
  | generated assets or a regen script | the regen script, then the guard it feeds | ⚠ another session may be regenerating the same tree |
  | Python tooling / a guard | `python3 -m pytest scripts/tests/ -q` | everything Rust |
  | a runner or workspace-wide change | `./run_tests.sh` | feature-gated tests, the consumer fixtures, the wasm check |

## Why each caveat is there

Every "does NOT cover" column above is a defect this repository actually had:

* **two content crates meeting** — `Empowered` was registered for rollback by
  Mary-O and by Sanic; each demo's own tests passed, `cargo check -p ambition_app`
  passed, and the app — which composes both — panicked on the first frame,
  killing 56 tests. No per-crate job can see that class.
* **feature-gated channels** — three `causal` message channels sat outside both
  rollback oracles because the default job never compiled them. Only the union
  graph did.
* **`TimeUpdateStrategy`** — under `Automatic`, `app.update()` advances the clock
  by REAL time, so a test that asserts a distance or a count measures how busy
  the machine is. Three separate tests have been fixed for this.
* **another session regenerating the tree** — a 63-minute suite run once outlived
  its own inputs, and two jobs failed on `include_str!` for files that existed
  before and after but not during.

---

<!-- Moved out of AGENTS.md 2026-08-04. That file is a ROUTING page with a 180-line
budget and this material had grown to 138 lines of it — 38% — while duplicating the
table above. The measurements and the four rules Jon set on 2026-08-03 are kept
verbatim here, which is where a reader who needs them is already looking. -->

## The cheapest sufficient command — MEASURED costs

Pick the narrowest row that covers what you changed. **Every number below was
measured on 2026-08-03 against a warm target directory** (`dev/ambition_dev_measurements/run_tests_cost.jsonl`
plus direct timings); they are the real loop cost, not an estimate.

| I changed… | run | s | what it does NOT cover |
|---|---|---|---|
| a doc, a plan, a ledger | *nothing* | 0 | a doc a TEST reads — the goal file and `AGENTS.md` are both read by `scripts/tests` |
| a Python tool in `scripts/` | `python -m pytest scripts/tests -q` | 12 | the Rust side, entirely |
| LDtk tooling | `tools/ambition_ldtk_tools/.venv/bin/python -m pytest tests -q` | 6 | anything that consumes what it emits |
| one engine crate's code | `cargo test -p <crate>` | ~5 | **the app build** (see below), and every `#[cfg(feature)]` item |
| anything the app composes | `cargo check -p ambition_app` | 21 | behaviour — it proves compilation and nothing else |
| one app-level behaviour | `cargo test -p ambition_app --test app_it <module>` | 31–90 first, **1.3 warm** | the other modules, and failures that only appear under LOAD |
| all app-level behaviour | `cargo test -p ambition_app --test app_it` | ~101 | non-default features |
| a feature gate | `cargo check -p <crate> --features <combo> --all-targets` | 20 | RUNNING the gated tests — only the union job does |
| every gated test | `cargo test --workspace --features <union>` (see `scripts/run_tests.py`) | 340 | little; it is the widest single graph we have |
| the default sweep | `python scripts/run_tests.py` | 393 (6.5 min) | features, external consumers, the wasm check |
| before a release, or after touching features / the SDK surface / the web path | `python scripts/run_tests.py --run-everything-you-probably-dont-need-this` | 1528 (25.5 min) | `#[ignore]`d tests and acceptance cycles — add `--heavy` |

⭐ **The one that surprises people: a warm filtered `app_it` run is 1.3 seconds.**
The 31–90s figure is the RELINK after an `ambition_app` source edit, so the loop to
optimise is *edit less of `ambition_app`*, not *test less*.

⚠ **`cargo check -p <crate>` is not the gate — `cargo check -p ambition_app` is**,
and the row above says 21s for a reason: it is cheap enough that there is no
excuse. A per-crate check has been observed green on a crate that fails to compile
in the app build.

⛔ **Do not reach for the exhaustive plan out of caution.** It is 4× the default
sweep and there is no CI to satisfy; Jon sweeps it periodically himself. Reach for
it when a row above names your change (features, SDK, web) — and ask
`run_tests.py --list` what a plan actually contains before running it.

⛔ **The 11-second job is the one to stop skipping.** `repo tooling
(scripts/tests)` runs 180 tests including
`test_every_contract_holds_against_the_live_tree` — the 25 architectural absence
contracts, which are the only thing that catches a registration or a dependency
edge landing in the wrong place. On 2026-08-03 two of them sat red through
several commits because targeted `cargo test` filters were run instead of the
plan. ⚠ and `python3 scripts/check_absence_contracts.py` **exits 0 while printing
`2 of 25 violated`** — enforcement needs `--check`.

### The write-ahead worktree (Jon, 2026-08-03)

Builds and tests are not slow because of the compiler alone — they are slow
because **every job reads the LIVE tree**, so a suite running on `main` freezes
editing for its whole duration. A second tree removes that serialization:

```
git worktree add -b workahead /home/agent/code/ambition-workahead main
cd /home/agent/code/ambition-workahead && python3 scripts/mirror_assets_for_worktree.py
```

- **Write ahead in the worktree** while `main` is mid-build. Next feature, next
  refactor — whatever the running job would have blocked.
- ⭐ **in an agent run this means SPAWNING ONE, not remembering to type faster.**
  Launch the long job, then immediately hand a subagent an independent task with
  `isolation: "worktree"`, and integrate its report when the job lands. ⚠ give it
  work that does NOT touch the files you are editing, and say so in its prompt —
  two writers on one file is worse than waiting. ⚠ `cd` to the repo ROOT before
  launching: a worktree is created from the repo containing the shell's cwd, and
  `tools/ambition_sprite2d_renderer` is a nested repo that silently hands the
  agent a tree containing none of `crates/`, `game/` or `docs/`.
- **Integrate, build and test on `main`.** Merge the worktree's branch when main
  is clean, start the next job, and go back to writing.
- ⛔ **Do NOT build in the worktree.** A second `target/` cannot be shared (they
  fight over artifacts and lockfiles) so it is a full cold duplicate — and this
  volume has hit 100% three times. Jon's call, and the reason the split is
  write-there / build-here rather than two independent checkouts.
- ⚠ **Mirror the assets first, always.** Generated art and audio are gitignored,
  so a fresh worktree has ~4 sprite files against main's 996 — and the sheet
  registry is baked from those directories, so an assetless tree compiles a binary
  with an EMPTY sheet table and ~40 tests fail for reasons unrelated to the
  change. `mirror_assets_for_worktree.py` links them file by file on purpose: a
  regenerated sprite lands as a REAL file in the worktree and main never sees it,
  which directory symlinks would not give you.

### Interlace architecture and feature work (Jon, 2026-08-03)

> *"Let's interlace architecture tasks and features tasks so we don't get so
> hyperfocused on either, they can inform each other."*

Not a scheduling preference — it finds bugs. The same day it was asked for, a
gameplay complaint (*"Maryo's fireball only shoots to her right"*) turned out to be
an **engine** defect in `dispatch_move_events`: every ranged moveset move fired
world-right regardless of facing, because `frame.fire` is an edge cleared every
tick and the fire-frame fallback resolved to the gravity frame's side axis. Fixing
it turned two `duel_arena` tests green that the architecture lane had been unable
to move for two days — that lane was looking at shield rules, and the bug was in
aim.

⭐ **A feature complaint is a report from the only place the whole stack is
assembled.** Tunnelling on architecture means never running the thing; tunnelling
on features means fixing symptoms one demo at a time. Alternate deliberately.

### Sweeping failures: ONE big run, then targeted only (Jon, 2026-08-03)

> *"Run the big suite once, then fix each test individually and verify them with
> local targeted reruns only, and then we DON'T run the entire thing again after.
> We just assume we fixed them because we did locally and move on. If anything
> else broke we catch it on the next big sweep, but we don't spend all day chasing
> those down."*

⛔ **Do not re-run the full suite to confirm a fix the targeted run already
confirmed.** On 2026-08-03 the same agent ran `run_tests.py` or
`cargo test --workspace` five times in one stretch; every failure it found was
then diagnosed and fixed by a single `-p <crate> --test <target> <filter>` run, and
no re-sweep ever caught anything the targeted run had missed. The re-sweeps cost
more than every fix combined.

⚠ **the instinct this overrides is real and still wrong here.** Re-verifying
globally *feels* like diligence; on a pre-release engine with no CI and a
maintainer who sweeps periodically, it buys a confirmation nobody was waiting for
and spends the hour that the next fix needed. A fix verified locally is fixed.

### When a run is slow, get the DISTRIBUTION before theorising

`--report-time` is nightly-only, so per-test timings on stable come from running
serially and timestamping the output:

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

⛔ **A suite total divided by a test count is not a per-test cost, and it reads
exactly like one.** On 2026-08-03 a 67s / 25-test subset produced an apparent
"2.7s per app boot"; the distribution showed boot is 370ms and **one test held
33% of the time, four held 63%**. Two fixes were built against the average before
anyone ran the two commands that show the shape.

⭐ **And the wall clock of a parallel run cannot go below its LONGEST test.** When
a run pins several cores and still feels slow, suspect a single long pole before
suspecting a lock or an I/O bottleneck — the CPU-percentage signature is the same
for both.

## Waiting on a long command

- To wait on a long command, read state it WROTE — for the suite that is
  `target/run_tests_status.json` (`state`: running/done/**aborted**/crashed,
  plus `current_job` and `current_started` so a slow job is distinguishable from
  a wedged one, and `completed` with each finished job's seconds). ⛔ **CHECK
  THE STATE, NOT JUST `failed`** — a suite the disk floor stopped part-way has
  an empty `failed` list because every job that RAN passed; `aborted` plus
  `never_ran` is the only thing that says the plan did not finish.
  `scripts/last_test_run.py` applies that rule for you and refuses rather than
  answering. Every run also
  appends what it cost to `dev/ambition_dev_measurements/run_tests_cost.jsonl` — wall clock, and how much
  of it was libtest actually executing rather than cargo building. ⛔ never poll
  with `pgrep -f <script>`: the polling shell's own command line contains the
  pattern, so it matches ITSELF and the loop sleeps forever (seven stranded,
  2026-07-31). Better still, don't poll — a backgrounded command reports its exit.

### When the suite REFUSES on headroom (2026-08-05)

`check_disk_headroom.py` blocks a run below 40 GB free, and it has fired four
times across three long runs. `cargo clean` is the obvious answer and the
expensive one — a full cold rebuild of everything.

⭐ **Delete the EXTERNAL CONSUMER target first.** `fixtures/external_consumer`
has its own `.cargo/config.toml` pointing at
`/home/joncrall/ambition-target/outlander`, which reached **44 GB** — the single
largest freeable object that is not the workspace's own `debug/deps` (327 GB, and
pruning that by hand is how you lose an afternoon).

```
rm -rf /home/joncrall/ambition-target/outlander     # 44 GB, ~105s to rebuild
```

It is pure build output (no sources — check with `find … -name '*.rs'` before
deleting anything under a target dir), and only the `external consumer: outlander`
job needs it, which runs only under the exhaustive plan.

⚠ **the cause is usually a worktree agent.** One `cargo test` in a second
checkout links against whatever `main` is mid-edit; recovering from that means
touching the workspace, which rebuilds it, which is a fresh copy of the graph.
That is the other reason the write-ahead worktree must not BUILD.
