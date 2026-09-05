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
  | a NEW PARAM on a shared `SystemParam` struct (a writer, a reader, a resource) | `git grep -l '<the system>'` → `cargo test -p` **each crate that names it**, not just the one that owns it | ⛔ nothing else. See the note below: the compiler cannot help here |
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

## ⛔⛔ PROSE IS THE ONLY PART OF THIS REPO WITH NO GUARD, AND IT IS MOST OF WHAT WE WRITE (2026-09-05)

**Four instances in one day, across two sessions, on four different subjects.**
Every one was a sentence describing something that was not there, and every one
survived every check the repository runs:

```text
sim_core_resources.rs:85    cited for a resource initialised at :89
room_transition_assets:1271 cited for a budget defined at :1367
quality.rs:182              cited for a function defined at :136
"a `close_on_transit` pair"  a doc comment on a test, citing a code path that
                             DID NOT EXIST — written by the person who owned the file
```

⇒ **The tests pass, the citation checker passes (1,842 citations, all resolved
on 2026-09-05; 1,866 a few hours later, because writing this page ADDED some),
the link checker passes.** None of them reads a sentence and asks whether it is
true. ⚠ And the fourth is the worst shape: not a stale pointer but an
INVENTED MECHANISM, cited as the reason a neighbouring invariant mattered.

⛔ **There is no gate to build here, and this section is not proposing one** —
see the line-citation measurement below, where both the weak and the strong
predicate were measured and neither is gateable. The strong one runs at ~50%
precision because prose legitimately cites a USE site rather than a definition,
and judging that needs a person.

⭐⭐ **THERE IS EXACTLY ONE PLACE WHERE PROSE-ABOUT-CODE HAS A REAL CHECKER, AND
IT WATCHES 12 CRATES OF 78.** rustdoc resolves intra-doc links, so
`` [`SomeType`] `` in a doc comment IS verified — `cargo doc -p <crate>
--no-deps` warns on every broken one, and `scripts/check_doc_link_ratchet.py`
runs it as a ratchet. ⚠ Over a HAND-KEPT list of 12 crates.

Measured 2026-09-05, `cargo doc --workspace --no-deps` over all **78** crates:

```text
unresolved intra-doc links, whole workspace   274
  in the crates the ratchet watches            34
  in crates NOBODY watches                    240
worst uncovered: asset_manager 44, app_tools 35, shared_tangle 30,
                 platformer2d 26, persistence 21
```

⛔ **THE 34 IS NOT A COMPARABLE NUMBER TO THE RATCHET'S BASELINE OF 147, and
saying so matters more than the figure.** The ratchet runs `cargo doc -p <crate>`
per crate; this was one workspace pass with `--no-deps`. Different feature
resolution, different counts. ⇒ **do not read "34" as the baseline having
improved** — it is a different measurement of the same crates, and only the
UNCOVERED total (240) is a claim this run can make.

⇒ The guard is real, it works, and its REACH is the finding: a broken link in
66 of 78 crates is invisible, and the list is hand-kept — its own comment records
having already fallen behind once, when a carve moved code out of a tracked crate
and the count "read as a repair".

⭐⭐ **AND RUNNING THE GUARD FOUND ITS PREDICTED FAILURE ALREADY LANDED
(2026-09-05).** `check_doc_link_ratchet.py` is RED at HEAD: `ambition_combat`
rose 16 → 17. The cause is exactly the shape its own `CRATES` comment warns
about — commit `42a391d49` carved `actor_tuning.rs` out of the monolith and into
`ambition_combat`, and **doc links that resolved in the old crate do not resolve
in the new one**:

```text
unresolved link to `CombatGeometry`
unresolved link to `ambition_characters::actor::CharacterDefinition::preserves_mirror_symmetry`
unresolved link to `ambition_characters::features::ecs::enemy_default_brain`
```

⇒ **A CARVE BREAKS DOC LINKS SILENTLY**, because the compiler is happy either
way: the prose moved to a crate where the names it cites are not in scope. That
is the one case where a broken intra-doc link IS a real signal rather than
tidy-up — it marks prose that has been separated from what it describes.
ⓘ The ratchet also shows `actor_monolith` FELL 59 → 43 in the same window, which
is the other half of the same carve and is precisely the "reduced coverage reads
as improvement" the comment predicts. ⇒ **add a carve's DESTINATION crate to the
tracked list in the carve's own commit**, and re-check doc links on both sides.

⛔ **AND THE REASON IT SAT RED UNSEEN IS ITS LANE: the ratchet is in
`build_maintenance_jobs()`, not the default or `--rust` lane** — reasonably, it
is a cold `cargo doc` measured in minutes. But that places the guard DAYS from
the change it exists to catch, and a carve is precisely the moment you want it.
⇒ the cheap habit is not a lane change (which would slow every run); it is
`python3 scripts/check_doc_link_ratchet.py --check` IN THE CARVE'S OWN COMMIT,
the same way its `CRATES` list is supposed to be updated there.

✔ **AND THE LANE IS NOT SYSTEMICALLY ROTTEN — checked, so the finding is not
over-read.** `build_maintenance_jobs()` holds five jobs; the other two drift
checks were run the same day and both pass: the zone-name ratchet (*"no world
gained an id-shaped zone name"*) and the vanished-name check against its fixed
baseline, both exit 0. ⇒ **one red, isolated, with a known cause** — not "nobody
has run maintenance in weeks". Worth saying, because "a guard in a lane nobody
runs" invites the assumption that everything behind it is rotten, and here it was
not.

⚠⚠ **AND 274 IS NOT 274 FALSE CLAIMS — I nearly left that implication standing.**
Characterised: **249 DISTINCT targets in 274 links**, almost no repetition, and
the most-repeated is four. They are one-off references to types NOT IN SCOPE
where they are written — `` [`SimId`] ``, `` [`Hitbox`] ``, `` [`UserSettings`]
`` — so the prose usually NAMES A REAL TYPE and only the hyperlink fails. ⇒ the
population is dominated by *"correct sentence, unlinkable from here"*, which is
what my own `InteractableSpec` case turned out to be.

⛔ What that does NOT establish, and it is the half that would matter: whether
each names a type that still EXISTS. A link that fails because the type was
renamed or deleted IS a false claim, and rustdoc's warning cannot tell the two
apart. ⇒ **the 274 sizes an unlinked-prose problem, not a wrong-prose problem**,
and anyone acting on it should de-link rather than hunt — unless they check the
name, which is the expensive part and the only part that finds a lie.
ⓘ ⭐ Worked example the same day: I wrote `` [`InteractableSpec`] `` in
`ambition_interaction`, a crate not on the list. It does not resolve — the type
is in another crate — and nothing in the normal build says so. `cargo check`
does not check doc links; I found it by running rustdoc on a hunch, an hour
after writing this page.

⭐⭐ **AND THE CHECK THAT MAKES THIS ACTIONABLE RATHER THAN DESPAIRING: prose is
verified by being USED.** All four instances were found by somebody trying to
BUILD on the sentence — following a citation to make an edit, or relying on a
comment's claim. Nobody found one by proofreading. ⇒ **treat any comment you are
about to rely on as a claim to TEST, not a fact to cite.** That is the whole
mechanism, it costs one grep, and it is the only thing that has ever caught one.

ⓘ **A vocabulary for what you find, because the three want different responses:**

```text
DORMANT   a field authored content never turns on      a DESIGN CHOICE, not a defect
DEAD      a field nothing reads                        a defect
STRANDED  a field carried end to end with no consumer,
          under prose claiming one                     a defect, and the hardest to find
```

⚠ STRANDED is invisible to both obvious censuses: a scan for unused symbols
cannot see it (every hop HAS a caller — authored, threaded, stored), and a
dormancy census cannot either (the field IS named in content). Two turned up on
2026-09-05 in one crate, both by taking a single census hit seriously instead of
publishing the list.

⭐ **So the discipline is the whole mechanism, and it is cheap:**
1. **When you cite a line, OPEN it.** Three of the four above were found that
   way and by nothing else.
2. **When you write prose naming a behaviour, grep the behaviour.** The fourth
   was a comment about a feature its own author believed existed.
3. **When you touch a comment claiming sole authority, re-derive it** — 11 such
   claims audited the same day, 3 wrong
   ([`simulation-authority-and-determinism.md`](../planning/engine/simulation-authority-and-determinism.md)).
4. **State a claim, cite where the number lives.** A copied count goes stale
   silently; a claim goes stale loudly, because the next reader checks it.

ⓘ The asymmetry worth internalising: code that describes a thing that does not
exist FAILS TO COMPILE. Prose that describes a thing that does not exist reads
exactly like prose that does.

## ⛔ A CHECK THAT IS CHEAP, DECISIVE, AND USELESS — the line-citation case (2026-09-05)

Two planning pages cited `platformer2d_runtime/src/sim_core_resources.rs:85` as
where `MovingPlatformSet` is initialised. It is line 89; line 85 is
`RequestedClockScale`. `scripts/check_planning_citations.py` validates that the
FILE exists — 1,825 citations, all resolved, before AND after the fix — so a
drifted line number passes it forever.

The obvious next gate is "check the line number too". ⭐ **It was measured before
being built, and it should not be built.** Across `docs/planning/**`:

```text
distinct line citations   383      (2026-09-05; every count here is a timestamp)
resolved and in range     355
PAST END OF FILE            0   <- the only thing this check can decide
path not resolvable         3   <- all deliberate examples: `file.rs:123`,
                                   `path.rs:123`, `semantic_NOPE.rs:9999`
ambiguous basename         25   <- abbreviated paths matching several files
```

⇒ **Zero findings, and the one real defect was IN RANGE.** A line-number gate
can only decide "past EOF", which nothing in the tree is; the failure that
actually costs a reader — an in-range citation pointing at an unrelated line —
is invisible to it, because deciding it requires reading the PROSE and the LINE
and judging whether they agree.

⭐⭐ **AND A STRONGER PREDICATE WAS MEASURED TOO, because "don't build it" was
the right answer to the WEAK one and not obviously to every one.** The defect is
"the cited line does not show what the prose names", so the sharper check is
SYMBOL PROXIMITY: for a citation of the form `` `SYMBOL` … (`path:NNN`) ``, does
`SYMBOL` appear within a few lines of `NNN`? That WOULD have caught both of
today's real drifts. Measured across `docs/planning/**`:

```text
citations with an adjacent backticked symbol   65
symbol found within +/-3 lines                 35
symbol NOT found                               30   <- ~46%, and NOT all drift
```

⛔ **Roughly half of those 30 are correct citations.** A citation legitimately
points at the USE site rather than the definition — `` `AbilitySet`
(`body_conditions.rs:59`) `` cites `pub fn can`, the READER of the type, which is
exactly what that sentence was about. Others sit just outside the window
(`upgrade_actor_sprites` cited at `:639`, defined at `:633`).
⇒ **Real drift is in there** — `log_quality_profile_override` cited at `:182`,
defined at `:136`, fixed today — but at roughly 50% precision.

⇒ **So: not a GATE in either form.** The weak predicate decides nothing; the
strong one would fail builds on correct citations about half the time, and a
gate that cries wolf gets suppressed and then ignored. ⭐ It is worth running BY
HAND as a periodic audit where a person judges each hit, and the 30-line script
above is reproducible from this description.

⭐ **The transferable rule: when a defect suggests a gate, ask what the gate
could DECIDE before writing it.** Here the cheap, automatable predicate and the
defect are disjoint, so the gate would have run forever, passed forever, and
certified nothing — while reading as coverage. That is worse than no gate.
⇒ The discipline that works is the one that found it: when you cite a line,
open it; when you follow a citation and it looks wrong, fix it and say so.

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
  2026-07-31).
  ⚠ **AND THE INVERTED FORM IS THE ONE THAT GETS PAST THIS PARAGRAPH.** Waiting
  for a process to be GONE — `until ! pgrep -f "run_tests.py"; do sleep 5; done`
  — reads as the opposite of polling for it, and strands identically and for the
  identical reason: the `until` shell's own command line contains the pattern,
  so the condition is never true and the wait outlives the thing it waited for.
  ⛔ **FOURTEEN stranded that way, counted on 2026-09-03 — and the oldest had
  been waiting 27 HOURS**, which predates the session that found them. Six were
  mine, from an evening in which I had read this bullet and quoted this file in
  a commit message; the rest came from earlier sessions. ⇒ It is a recurring
  cross-session pattern, not one agent's lapse, and the cost is invisible: a
  stranded waiter consumes nothing and reports nothing, so it is only ever found
  by someone auditing process lists for another reason.
  ⚠ **AND THE SELF-MATCH IS ONLY ONE VARIANT.** A fifteenth was found the same
  day: a `tail -f` monitor on a scratch log, still running after **1 day 4.5
  hours** because the file it tails had been DELETED. `tail -f` on a removed
  inode never emits and never exits — the pattern was fine, the subject was
  gone. ⇒ The family is *a watcher that outlives its subject*, and the fix is
  the same either way: wait on something that ends. A command that exits, a
  pid, a status file with a terminal state — never an open-ended follow. The rule is about
  the PATTERN matching the WAITER, not about the direction of the test.
  ⇒ Match something that cannot contain the pattern — the status file's `state`
  field, `wait` on a known pid, or the runner's own exit — and if you must use
  `pgrep`, `pgrep -f "[r]un_tests.py"` keeps the bracket out of the match. Better still, don't poll — a backgrounded command reports its exit.

### When the suite REFUSES on headroom (2026-08-05)

`check_disk_headroom.py` blocks a run below 40 GB free, and it has fired four
times across three long runs.

⛔⛔⛔ **DO NOT RECLAIM IT YOURSELF.** This section used to open with a
`rm -rf /home/joncrall/ambition-target/outlander` and call it the cheapest
object to delete. AGENTS.md forbids that in its strongest terms — *"NEVER
`rm -rf` anything under a `target/` … NOT AS A FAVOUR WHEN THE DISK IS FULL …
the reclaim is Jon's call, on Jon's machine"* — and the advice was followed on
2026-09-03 by an agent pruning a live target with the bind mount present.
Corrected the same day.

⇒ **The first move is `scripts/setup/target_bindmount.sh --status`**, because a
target that has grown enormous is usually an ABSENT BIND and repairing it
returns the space without deleting anything. If the bind is present and the
volume is genuinely full, report the numbers and STOP.

⭐ **What is still worth knowing here is WHICH object is large, since that is
what you report**: `fixtures/external_consumer` has its own
`.cargo/config.toml` pointing at `/home/joncrall/ambition-target/outlander`,
which reached **44 GB** — the largest single object outside the workspace's own
`debug/deps`. Only the `external consumer: outlander` job needs it, and that
runs only under the exhaustive plan, so it is the cheapest thing for JON to
reclaim if he chooses to.

⚠ **the cause is usually a worktree agent.** One `cargo test` in a second
checkout links against whatever `main` is mid-edit; recovering from that means
touching the workspace, which rebuilds it, which is a fresh copy of the graph.
That is the other reason the write-ahead worktree must not BUILD.

⛔⛔ **WHY THAT LAST ROW EXISTS — measured 2026-09-05, nine tests red.** Folding
three loose `MessageWriter` params into one `SystemParam` struct and adding a
fourth produced:

```text
Parameter `StrikeOutcomeWriters<'_>::parried` failed validation: Message not initialized
```

in every app that builds that system BY HAND. Those apps register the messages
the system used to need and then `add_systems(…)` it directly, so the new param
has no registration. ⇒ **A hand-listed set of dependencies is a POPULATION, and
adding to the SOURCE does not update the LISTS.** The compiler cannot help: this
is runtime parameter validation, not a type error, so the owning crate builds and
tests clean while its consumers panic.
⭐ **The cheapest sufficient check is therefore a GREP FIRST** — `git grep -l` the
system's name — because the population you must test is not derivable from the
crate you edited. That is the one shape on this page where the right command
depends on a search rather than on what you changed.

✔ **AND THE ROW ABOVE IT WAS ALREADY RIGHT AND WAS NOT RUN.** The same landing
added a message channel; `python3 -m pytest scripts/tests/ -q` catches
`stable_schema_names: message.parried_body_hit` and it went unrun for hours. This
page's problem has never been that the rows are wrong.
