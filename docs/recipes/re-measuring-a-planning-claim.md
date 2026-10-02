# Re-measuring a planning claim

A planning row is a claim about a repository that changes under it.
`docs/planning/README.md` tells every session to re-measure a row against `HEAD`
before acting on it. This page lists how that re-measurement goes wrong and what
to do instead.

Most failures here are instrument errors: a filter that cannot match, a scan at
the wrong granularity, a signature read instead of a body, a count that is a
property of its flags. The one-line defence for all of them:

> **State in one clause what your instrument actually resolves. Then check
> whether your sentence is larger than that clause.**

This page carries no total on purpose. The sections are the list.

Sibling page: [`checks-that-did-not-run.md`](checks-that-did-not-run.md). It
covers the same subject one level down: a check that is correct and does not
run.

## Scope the claim to the instrument

### The error is a claim wider than the tool's scope

Each tool below succeeded and answered its own narrower question. The error was
the sentence written about it.

| the tool measured | the sentence claimed |
|---|---|
| occurrences in `src/` | the whole crate (uses in `tests/` were missed) |
| occurrences of TEXT | usage (an intra-doc link counted as a use) |
| `ambition_*` names only | every dependency (missed `thiserror`, `ron`) |
| a default-feature build | the crate's dependencies (code behind `#[cfg(feature = ...)]` was missed) |
| symbol resolution (`check_planning_citations.py --strict`) | that prose about where code LIVES is true |

Write the scope into the claim. *"Never named in `src/` on a default-feature
build"* is a sentence a reviewer can test. *"Never used"* is not.

When a claim is about a whole crate, scan the whole crate: `src/`, `tests/`,
`benches/`, `examples/`, `build.rs`. `--all-targets` is that scope.

### A green tool is not a green claim

Before you quote a passing check as support, say in one clause what that check
resolves: symbols for the citation checker, default-feature compilation for the
dependency lint, `src/` for a source sweep. If that clause is narrower than your
sentence, the green is not evidence for it.

- `check_planning_citations.py` resolves cited symbols and paths. It does not
  check prose about location, and it does not check test names.
  `scripts/check_planning_test_citations.py` checks backticked test names.
- After a carve, sweep old paths and sort the hits into HISTORY and STALE.
  Re-tense stale sentences; do not purge correct history.
- A remembered rule is a hypothesis about a situation. Before you "fix" a pipe,
  ask where the tool puts its verdict. `check_absence_contracts.py` prints the
  verdict inside each line, so `| grep` keeps it.
- Say what the evidence IS, not only what you concluded. A wrong conclusion over
  a recorded observation is one edit to fix. A retraction that deletes the
  observation can itself be the mistake.
- When a mis-scan also finds a smaller true thing, report the retraction first and
  the residue second.

### A control proves the instrument can see, not that it is aimed at the whole subject

Before you believe a zero, run the instrument against something you know is
there. Then ask: *if my instrument were pointed at only part of the subject,
would this control still pass?* A `src/`-only scan passes a positive control and
still misses `tests/`.

Common false zeros:

- `grep -E "a\|b"` searches for a literal pipe. Under `-E`, alternation is `|`.
- Searching the wrong tree. Authored levels are in `game/ambition_map_assets/`.
  Generated character sprites are under
  `crates/ambition_platformer2d_actor_monolith/assets/sprites*`.
- A quoted search (`"room_id"`) when the data spells it differently.
- A search for `check_x.py` when callers `import check_x`.
- `ls` colour codes leaking into a comparison.

## Read the code, not the declaration

### An inventory that reads TYPES mis-reports BEHAVIOUR

A signature shows the shape of a decision. The body shows the decision. A count
from signatures finds every place that could be wrong and cannot tell which ones
are. An inventory row is not finished until somebody reads the function.

### Before you call something an omission, look for the reason it is a decision

A deliberate absence usually says so in place, or mirrors code that does. Ask of
every "X does not do Y": is there a comment, a sibling, or a scope note that makes
Y wrong here? If yes, the finding is at most that the reason is hard to find.

Copying a call shape copies its arguments and not its justification. A sibling
that documents itself is weak evidence for the copy. Ask whether the sibling's
reason is about your road or its own.

### A premise that is still true and no longer the point

A plan's premise can verify clean by `grep` while its conclusion is dead, because
something else now does the job. Do not stop at confirming the sentence. Ask what
the sentence was FOR. Grep for the concept, not only the identifier, and read the
tests: test names and assertion messages state intent.

### Read the doc comment on the type a row names

A decision sometimes lives only in a code comment. Before you implement a plan
row, read the doc comment on the type it names. The code is often the newer
document.

## Counts and numbers

### A count is not a finding unless the instrument travels with it

- A count from an ad-hoc `grep` is a property of its flags (`-A` window, globs).
- A count over generated or untracked files is a property of the machine.

Do this instead, in order of preference:

1. Track a row by NAMED items, not by a total.
2. If the number is the point, record the exact command beside it and whether
   its inputs are tracked.
3. Before you report that a count MOVED, reproduce the OLD number with your new
   instrument. If you cannot, you measured two different things.

A re-count that returns exactly the old number after a week of changes is
suspicious. Check the instrument.

### The row names its instrument. Run that one.

If a row cites a script (for example `scripts/architecture_census.py`), run that
script. A hand-rolled scan makes different scoping decisions (test boundary,
roots) without knowing it. A large unexplained change in a population that
nobody was adding to points at the instrument first.

### A figure travels with its noun, its method and its reference point

- Ask what the number is a duration OF, a count OF, a size OF. When a number
  crosses a document boundary, it keeps its noun. A killed run has no duration;
  say so.
- Promote a figure into doctrine only after you re-derive it from the source, not
  from the document you read it in.
- Drift from a stored baseline is not comparable with change over a day. When two
  figures disagree, run the instrument (for example `scripts/compile_ratchet.py`)
  instead of reasoning about what it might measure.
- Two instruments can use one word for different things. Find each instrument's
  own definition before you reconcile two outputs.

### A `pub type` alias splits the population

A scan keyed on what an alias expands to does not see the alias's uses. A scan
keyed on the alias name sees nothing else. Before you quote "everything that does
X", ask whether X has a NAME as well as a SHAPE, and count both.

### A count plus a list is two claims

A reader spot-checks the cheap part (the count), and the list rides on that
credibility. Take the names from the file with the same command that produced the
count.

When you correct a count, apply the rule to the WHOLE population, including the
members you inherited. If that is too expensive, say which members you re-tested.

### A cut-off metric reads a slowdown as a reduction

Anything measured at a fixed cut-off (stocks left at the timer, items collected in
N frames) cannot separate "less happens" from "the same happens later". Use a
clock long enough for the counted thing to finish. If every arm reaches the same
terminal value on the longer clock, the earlier spread was the timer. Write the
prediction before the run.

### Generated output: drift or two machines?

Before you correct a number, ask whether it counts repository content or build
output. Repository content that disagrees is drift. Build output that disagrees
is two machines. Run `scripts/check_quality_variants_are_fresh.py` on each tree
before you compare anything downstream of generated art.

Write claims that survive: a ratio or a single named consumer ("one target opts
in") outlives a size ("442.6 MB").

## Sweeps

### Sweep at the granularity a reader looks for

One script can hold many rules. `check_absence_contracts.py` runs dozens of named
contracts. Readers search for the rule they are about to break, not the script
file name.

### Some findings exist only between two plans

When a plan says "waits for X", "owned by Y" or "the residual is Z", read X, Y and
Z before you believe the row. Each page states its own half; the join is written
nowhere. `Owner:` lines and `See also` links are the only followable record of
that join.

### Five shapes a stale planning page takes

1. **The page contradicts itself.** Re-measure blocks at the top, original tense
   in the body. When you append a block, grep the body for what it refutes.
2. **A heading tallies its own list.** Name the subject in the heading; let the
   list carry the count.
3. **A decision lives only in a code comment.** Read the type's doc comment.
4. **A dead section reference.** A link to `vision.md` resolves after its "§8"
   is gone. Prose section references are not checked.
5. **A number with no instrument.** See above.

Four of the five are invisible to every gate. Re-measurement is a person's job.

### Presence sweeps over rule data

A presence or absence sweep over rule data means nothing until you know what each
rule asserts. In `tests/ambition_workspace_policy/policies/*.toml`, a
`kind = "forbidden-path"` rule passes when its path does NOT exist. Group findings
by `kind` before you read them.

A type-citation sweep is useful against `docs/systems/`, which claims things
exist. Against `docs/planning/`, absent names are often correct (proposals, or
names a page says must not exist). Filter out upstream types, enum variants, LDtk
entity identifiers and deliberately forbidden names first.

### Sweeping every named test at once

```bash
# every sentence-shaped backticked identifier in docs/planning …
grep -rhoE '`[a-z][a-z0-9_]{19,}`' docs/planning --include='*.md' | tr -d '`' | sort -u
# … minus every function that exists anywhere in the tree
grep -rhoE '\bfn [a-z0-9_]+' --include='*.rs' --exclude-dir=target . | sed 's/fn //' | sort -u
```

Search `.py` and `.mjs` too. The yield is low and most hits are renames, so do not
gate on it. `git log -S"<name>" --all -- '*.rs'` answers "renamed or deleted?".

### Symbol presence fails in both directions

When you ask whether a branch's work landed on `main`:

1. `git cherry origin/main <branch>` proves LANDED by patch id. A miss proves
   nothing: reworked content shows as not equivalent.
2. Symbol presence is unreliable both ways. A related substring is not the
   symbol, and a landed feature can be renamed. Use it only to find candidates.
3. Test the branch's stated PURPOSE against `main`. This is decisive.

A rename is sometimes visible only in a merge conflict.

### Write down negative results and the row that held

Record a clean sweep once, with its command and commit. Also record a row that
survived re-measurement. A page that records only drift teaches the next reader
that drift is the answer.

## Use the instruments that already exist

### Ask whether the compiler already answers it

For unused dependencies, use rustc, not a text search:

```sh
cargo rustc -p <crate> --lib -- -W unused_crate_dependencies
```

`cargo rustc` applies the flag to one crate, so the shared target cache stays
warm. The default-feature lint over-reports; confirm with `--all-features`. Only
a dependency unused under both is unused. Three residues need judgement:

- a dependency used only by `tests/` is misfiled; move it to
  `[dev-dependencies]`;
- an optional dependency named in `[features]` is public feature surface;
- a dependency reached only by an intra-doc link needs the doc link removed too.

### Check whether a test already drives it

Before you accept that a measurement is impossible here, search the test suite.
Acceptance tests boot real compositions; their `eprintln!` output is swallowed by
libtest. For example, `game/ambition_app/tests/hall_transition_cover.rs` drives a
real door transition:

```bash
cargo test -p ambition_app --test app_it <module>::<test> -- --nocapture --test-threads=1
```

A residency figure belongs to its composition (quality tier, sprite tree). Name
the composition beside the number.

## Guards and machine state

### A red commit citation may be about your checkout

A superproject `git fetch` does not fetch submodule branches. Before you treat a
commit-citation red as a bad citation, run `git submodule foreach git fetch`. To
tell a rewritten SHA from a foreign one, run `git cat-file -t <sha>` inside the
submodule. For the reader, name the submodule beside a submodule SHA.

### A guard that reads state git does not carry answers about the machine

Ask what the guard reads and whether git carries it. Generated, gitignored art
differs between machines, and no command makes them agree.

- Never edit a shared allowlist from one machine's generated output.
- Fail on the direction whose evidence is a PRESENCE. A newly stranded sheet is
  a regression on any machine. An absence in unshared input can mean "fixed" or
  "never rendered", so report it and do not gate on it.
- An anti-vacuity floor proves the guard walked a corpus. It does not prove the
  corpus matches someone else's.

### The shell answers a narrower question than you asked

- `cmd | grep pattern` exits with grep's status.
- `grep -oE "[0-9]+/[0-9]+ jobs passed"` accepts `9/10`. Grep the literal passing
  value.
- `sort | head -N` drops the same corner of the tree every time.
- A parse that matches nothing reads as a clean tree.

Make the predicate name the value it accepts, and make an empty result refuse.

### Ask what a constant would do to your check

`f(a) == f(b)` passes for every `f` that discards information, including a
constant. Add a disagreement arm: two inputs that must differ must project
differently. To audit a guard, collapse its key reader to a constant and see
which arm fails. A control shaped as an equality inherits the same blind spot.
Assert on the one term the rule decides, not on a hash that contains it.

### When both systems share a schedule, read the change tick

A presence check sampled at frame start cannot order two writers in the same
schedule. `World::get_resource_change_ticks::<T>()` read at frame end shows which
system ran first. Add an anti-vacuity floor: at least one frame must contain both
events.

### Price a row by its INSERT SITE, not by the event it is named for

Before you price a coverage row as needing an expensive fixture, grep for the
code that constructs its component. A field named after a dramatic moment (a
boss death) is often defaulted at spawn, so entering the room is enough.

### The two worlds you compare do not run the same number of ticks

A GGRS sync-test harness steps one more time than a session-less one. Assert a
RATE (advance per tick) and read `SimTick` as the control. Before you treat a
one-frame difference between harnesses as a finding, look for an existing record
of it.

### A value your test writes does not survive a rewind

A message the test writes from outside the rewinding schedule is undone by the
rewind. Under a rollback session, drive input through the road GGRS replays, or
choose a subject that needs no stimulus. Assert that the stimulus arrived in both
worlds before you compare anything downstream.

## Where two models are on offer, prose drifts to the short one

When one model has a four-word summary and the other needs a diagram, summaries
drift to the short one. Name the asymmetry on the row. Treat any short paraphrase
of that row as a claim to re-check. Watch the verb: a verb nobody supplied (for
example "stops") is where a model gets chosen.

## Leave a receipt

When you re-measure a planning file against `HEAD`, record the SHA, the date and
what you found, including "nothing changed". A dated `Verified against <sha>`
header is a receipt of a re-read. Do not add one without the re-read.
