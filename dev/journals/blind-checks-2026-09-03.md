# Blind checks: measurements that could not have failed

A list, not an argument. Each line is one check that looked like evidence and
was not, with the commit that fixed it so a reader can verify rather than take
it. Entries are from the working night of **2026-09-02 UTC**; the filename
carries the day it was collated.

⚠ These are **ambition-e7's** instances only. A second set from the same night
(debug-feature system names under `-p`, exact allowlists after a merge, `.venv`
resolution, `head -N` truncating the grep that had the answer) belongs here too
and should be appended by whoever holds those commits — an unattributed line
with no SHA is exactly the kind of claim this page exists to distrust.

⭐ **THE DURABLE HALF OF THIS PAGE NOW LIVES IN A RECIPE, and that page is the
one to read first:**
[`docs/recipes/checks-that-did-not-run.md`](../../docs/recipes/checks-that-did-not-run.md)
§ "The sibling family: it RAN, and it could not have failed". It holds the
recurring shape and the four-pass audit; **this page is where they came from** —
the instance table below, with a SHA per line, so a reader can verify rather
than take it.

⚠ That recipe's own numbered list is a DIFFERENT family: checks that never
executed. **Do not add the two counts.** Ask "did it run?" there and "did it
check what I think?" here.

## The recurring shape

**An emitter tells you what a line CONTAINS; it never tells you what to compare
it against.** A parser written from the emitter reproduces its vocabulary and
inherits none of its ordering, thresholds or population bounds — so the parse
succeeds, the number prints, and the number is about a different question.

## The instances

| # | The check | Why it could not fail | Fixed in |
|---|---|---|---|
| 1 | Ordered before/after `room-loaded` by game clock | The census runs in `Last`, so its clock reads after a `room-loaded` the same frame's `PreUpdate` preceded — 7.6 MP landed in the wrong bucket | `d1c63cd5a` |
| 2 | Counted "images decoded at boot" from `[image]` lines | `[image]` prints only decodes ≥ 1.0 MP (`NOTABLE_MEGAPIXELS`): 7 printed lines standing for 252 decodes | `fa531334a` |
| 3 | Read pack usage at Quarter from `[image]` lines | All 154 quarter pack pages have a median of 0.26 MP — not one can ever print | `8c95af427` |
| 4 | Keyed an asset census on `Path.resolve()` | A worktree's assets are symlinks into the main checkout, so every path resolved to a different tree | `859c496d4` |
| 5 | Took `git ls-files '*.ldtk'` + `grep -l` as proof the worlds were ours | `ls-files` lists a tracked symlink and `grep -l` follows it into a submodule; mode `120000` settled it | `4f9938097` |
| 6 | Compared two asset roots with `rglob` and got "0 files in common" | `rglob` does not descend a symlinked directory — seconds after I had `sha256sum`'d the same relative path in both | `d41ec0666` |
| 7 | Two scripts answering "which pages are orphaned" with two definitions | Neither was wrong yet; the disagreement was scheduled, not present | `d0d0e9617` |
| 8 | Poisoned a bucket by filtering against an empty set | The filter was a no-op, so the suite printed green and the poison proved nothing | `0035b097b` |
| 9 | Fixture where "unclaimed" and "has no `.ron`" always coincided | Deleting the `.ron` check entirely left six tests green; the discriminating case had to be constructed | `854a0c888` |
| 10 | A heredoc edit script whose `assert` failed | The `git add && git commit` on the next line still ran, landing a commit whose message described an edit it did not contain | `dcf9eaeaf` |
| 11 | Portrait headroom computed from parsed draw sizes | A stale regex parses zero boxes and reports infinite headroom — "every tier is big enough" | `1ccbb8e39` |
| 12 | "Stale output or live defect? — wait for a clean regen elsewhere" | Written three times, mine included, while a local discriminator existed: 44/44 stranded pages predate their manifest, 439/475 portraits do not | `f3b4bbd04` |
| 13 | Three hypotheses about which targets fail to downscale | All built by READING the generator and diffing configs; `discover_all_targets()` answered it in one call and inverted the third | `b88df4663` |
| 14 | A clamp stopping an extent rounding to zero, applied to origins too | Turned every `"x": 0, "y": 0` into `"x": 1, "y": 1` across five world files; only the diff showed it | `4c4c59581` |
| 15 | Grepping for a lesson's own heredoc delimiter inside that heredoc | Bash ended the script early and the trailing command ran on garbage | *(memory only, no commit)* |
| 16 | Three theories of which targets fail to downscale, from configs and mtimes | The four share one imported module; `grep -l _authored_swing_fighter` answered it, and the shared file was visible in the first `ls` of that directory | `c77f66425` |
| 25 | `check_rollback_mutators_run_in_sim` reporting "OK: 4 systems mutate rollback state, none in a non-rewinding schedule" | Its population was ONE type — `rollback_types()` read a single file holding 1 of the repo's 87 canonical registrations (113 types). A green that reads as a clean bill of health for rollback, over under 1% of it. Widened to 113 types / 318 systems / 6 waived | `7793c78df`, `04cbab8c5` |
| 27 | A census of "families where some members author a height and some do not" returning ZERO | The grouping key took the first 2–3 underscore tokens, putting `npc_puppy_slug` and `npc_puppy_slug_variant2` in DIFFERENT groups — so it could not have found the case I already knew existed | `e3e5681da` |
| 26 | Knockdown-row coverage measured at 4 sheets against a doc count of 10 | Sprite manifests have TWO shapes — `body_metrics` map keys vs `animation:` row fields — and the regex saw one, calling `officer` (which contains all four words) empty | `44e5e8804` |
| 24 | `check_engine_systems_are_engine_installed` exiting 0 with NO OUTPUT AT ALL | Genuinely passing (0 unclaimed at budget 0) — but silence is what a check that never ran also prints, and every sibling `check_*.py` says `OK: …` | `335eb2d8a` |
| 23 | `check_authored_levels_survive` reporting "found no .ldtk worlds at all" | `SKIP_PARTS` contains `.worktrees` and was matched against the ABSOLUTE path — an agent slot's own root is `<repo>/.worktrees/<slot>`, so every world was skipped. 12 on disk, 0 kept. Inoperative in exactly the trees the agents work in | `2edf629b0` |
| 22 | Five suite failures dismissed as "cargo is not on this PATH" | `~/.cargo/bin/cargo` exists; four scripts resolved it and two call sites did not. Fixing that turned 4 into passes and left ONE REAL finding the crash had hidden — `examples/portal_tutorial/Cargo.lock` stale since `ambition_registry_core` landed | `3959d8b27` |
| 21 | `test_absence_contracts.py` green over 25 architectural contracts | Its docstring promises a test that feeds each contract a violating line "and requires a hit". That test did not exist — `VIOLATING_LINE` was read only by the SILENCE test, so no fixture had ever been checked for violating anything | `e4e545d29` |
| 20 | `check_published_sheets_are_present.py` exiting 0 | Its `claimed_install_names` import was bare, so on any checkout without the tool venv it returned `None`, printed "cannot check" and exited 0 — a guard off on every machine that skipped a setup step | `f4693ac7b` |
| 19 | `check_planning_citations.py` reporting "all resolved" over tonight's rows | `SYMBOL` needs a `::`, so bare backticked names — the commonest form in these docs — are never extracted; a fabricated one left the count unchanged at 526 | `d3c86dc79` |
| 18 | "The four unshrunk variants are `_authored_swing_fighter.render` dropping the scale" | A coherent mechanism, in the wrong road: that module is never called for tiers — the variant script resizes for every module-kind target. Two boxes disagreeing is what exposed it | `c77f66425` → reconciled |
| 17 | "44% of the tree" as the occupancy denominator | Half the MEGAPIXELS are unclaimed but only a fifth of the BYTES — stranded pages are large and empty, so they compress to nearly nothing | `334086d9c` |

## How to run this audit again

Moved, so there is one copy to keep current: the four passes are
[`docs/recipes/checks-that-did-not-run.md`](../../docs/recipes/checks-that-did-not-run.md)
§ "Running this audit yourself". It found eight real defects in one evening.

## ⭐ What the sweep did NOT find, recorded so nobody repeats it

⛔ **A `path:line` CITATION CANNOT BE CONTENT-CHECKED CHEAPLY, and I measured
that before building the guard rather than after.** `check_planning_citations.py`
verifies that a cited path EXISTS and has enough LINES — never that the line
holds what the row says. That gap is real: the D33 cut-1 pass found
`ActorClusterSeed::new_character_in` cited at `spawn_actors.rs:2010` when 2010
is a comment and the call is at 1975.

The obvious guard is "the symbol named on the row should appear within N lines
of the cited line". Measured over `docs/planning`: 48 hits, **17 misses**, 70
citations with no symbol on the row. At 26% the misses would drown the signal —
and spot-checking three showed why they are not defects:

* `` `drive_wave_encounters` ends with ~90 lines (`encounter/systems.rs:337`–`427`) ``
  cites an INNER RANGE. The function starts at 140; 337 is inside it and is
  exactly the line the row means.
* `` ✔ `features/mod.rs:350` — `runtime_census` — CLOSED `` cites where the thing
  WAS when the row was written. `runtime_census` is at 320 now.

⇒ **Do not build it.** A checker that reports a legitimate inner-range citation
as drift trains its reader to skim, which is how a real finding gets missed —
the same reason bare-name checking was rejected at `d3c86dc79`. The one true
drift was caught by the post-carve checklist's TARGETED grep over the symbols a
known carve moved, where the population is small enough to read. A narrow
question with a knowable answer beat a general checker, twice.


⭐ **THE PLANNING DOCS ARE CLEAN ON THE BARE-CITATION AXIS, and the negative is
worth as much as a finding here.** `check_planning_citations.py --vanished`
(added 2026-09-03) reports a BARE backticked name that was a definition at a
baseline and is not one now — the form the default check cannot see, and the
form a carve's removals are usually spelled in. Over a week-old baseline it
returned 37 hits across 22 names, and every name checked was an item a NAMED
COMMIT genuinely removed. **But essentially every ROW was already recording the
removal** — "Deleted: `FpsOverlayState`", "the view is DELETED", "THAT PRELOAD
ROAD IS GONE as of `301a07009`". Four authors' docs, none stale.

⇒ Two things follow. The docs are better maintained on this axis than the sweep
assumed, and the mode belongs at a CARVE'S PARENT rather than a wide baseline:
a fresh window catches rows before anyone rewrites them in past tense, and a
wide one is archaeology.

⛔ TWO FALSE POSITIVES CAME OUT OF MY OWN TOOL FIRST, both LIVE crates
(`ambition_demo_pocket`, then `ambition_platformer2d_actor_monolith`) — one via
a field rule, one via a `mod` line. Fixed in `9137e4356`. I had committed the
tool before triaging the half of its output that contained them.

A crude scan flagged 13 test files that assert an absence against the live tree
with no obvious population floor. **Spot-checking them found them healthy**, and
the ways they were healthy are worth copying:

* `test_tracked_symlinks_resolve` — `assert links` before `assert not dangling`:
  a truthiness floor my regex could not see.
* `test_map_symlinks_stay_links` — a dedicated
  `test_there_are_tracked_worlds_at_all` whose message is *"no tracked .ldtk
  files; this whole file is vacuous"*.
* `test_text_spawns_resolve_a_font` — `assert path.is_file()` per watched file,
  *"so this guard is watching nothing"*.
* `test_catalog_rows_are_in_the_characters_map` — a live assertion, a
  `characters_seen > 80` floor, AND a synthetic misplaced row proving the scan
  fires.

⚠ **AND A SECOND ATTEMPT TO WIDEN `check_planning_citations` DIED THE SAME WAY
AS THE FIRST.** It validates `file.rs:123` but not a backticked path with no
line number, so I measured those: **120 in `docs/planning`, 9 that do not exist
from the repo root — and all 9 are legitimate.** Four are doc-relative
(`game/multiplayer.md` inside `docs/planning/roadmap.md` resolves fine), two are
relative to a submodule root (`docs/actor_contract.md` lives in the sprite
renderer), one is a short form of a real file, one contains a literal `…`, and
the last is a HISTORICAL PROVENANCE note — *"relocated from
`docs/vision/driving_decision_principles.md`"* — which is correct precisely
because the file is gone.

⚠ **AND A THIRD: COMMIT SHAs.** `docs/planning` cites **203 SHA-shaped tokens**
(7–12 hex, backticked). 196 resolve to main-repo commits, **6 resolve in a
SUBMODULE** (`tools/ambition_sprite2d_renderer`, `dev/ambition_dev_measurements`,
`game/ambition_map_assets` — correct citations of commits this repo does not
contain), and the last one is not a commit at all: `c0312413be50` is labelled
*"(md5 `c0312413be50`)"*, a capture hash. **All 203 legitimate.**

⇒ **Three proposed extensions, three all-false-positive populations.** Bare
names (~408), bare paths (9 of 9), SHAs (7 of 7). That is not three near
misses; it is the same fact three times — **prose cites more kinds of thing than
a resolver can enumerate, and each kind has a legitimate reason to be
unresolvable.** The checker's narrow scope is a decision, not an oversight, and
this page is now the record so nobody spends a fourth evening on it.

⇒ A path in a doc resolves against THREE different bases and sometimes against
none on purpose. That is the same reason the checker refuses to match
schematics, and it is now measured twice: the bare-name population is 408
false-ish findings, and the bare-path population is 9 of 9. **The scope is
right; the gap should stay documented rather than closed.**

Beyond `scripts/`, three more checked and CLEAR:

* `rollback_coverage.rs` is `#![cfg(feature = "rl_sim")]`, which is the shape
  that usually means "runs nowhere" — but `run_tests.py:515` runs
  `cargo test -p ambition_app --features "rl_sim causal"` deliberately, so it
  does run. ⓘ Worth knowing rather than acting on: its `WAIVED` list is 31
  patterns of which **22 are namespace-wide** (`ambition_asset_manager::`,
  `ambition_input::`, …), so its effective population is "sim state minus 22
  namespaces". The file says so itself — *"this list is the part of the test
  that can lie"* — and already documents one waiver as narrower than it reads.
* `check_zone_name_ratchet` is CI-wired with `--check` and already refuses a
  zero-observation sweep in its own words.
* `feature_gated_tests.py` reports 29 crates hiding 783 tests behind features,
  with its own over/under-counting caveat — an honest instrument, and the gate
  runs the union anyway.

⇒ The guards in `scripts/tests/` are, on the whole, built with the vacuity
question already asked. The entries above are exceptions, not a pattern — and
`test_absence_contracts` (21) is the one that had the floor and was missing the
FIRE direction instead, which is the rarer half to forget.

## What the fixes have in common

Every one of 1–3, 11 and 12 was found by a number disagreeing with another
number, never by re-reading the code that produced it. 4–6 are one hazard —
**a mirrored tree makes one file look like two and two files look like one** —
and the check that settles it is `git ls-files -s` (mode `120000`) or `ls -i`,
not a checksum. 8–10 and 14 are the same discipline turned on the guard itself:
a poison that does not apply, a fixture whose conditions coincide, and an edit
script whose failure does not stop the commit are all checks that cannot fail.

⇒ The one habit that would have caught the most of them: **before reporting a
count, say out loud what population it is a count OF, in what UNIT, and what
would make it zero.** Items 2, 3, 11 and 12 each print a confident number about
a population the instrument cannot see; item 17 prints an honest number in the
unit that does not answer the question being asked.

⇒ **RUN A CENSUS AGAINST A CASE YOU ALREADY KNOW.** Items 26 and 27 were both
invisible in their output and obvious the instant a known answer was checked
against them — a count of 4 where the doc said 10, and a ZERO from a sweep for
a thing I had found by hand ten minutes earlier. A census with no known positive
reports its own blindness as good news.

⇒ The one worth generalising: **compare the guard's denominator against the
repository's.** "4 systems mutate rollback state" is implausibly small for a
game with rollback netcode, and that ratio — not the pass/fail — is what
exposed items 19, 20 and 25. A guard reports a number about a population; ask
what SOURCE produced the population before reading the number.

⇒ And the one that would have saved the most TIME: items 13 and 16 are the same
mistake twice in one night — **reading code to infer what a tool would report,
when the tool was one call away.** `discover_all_targets()` and
`grep -l <shared module>` each ended a chain of failed hypotheses immediately.

## The second set (ambition-df, the integrator's half)

| # | The check | Why it could not fail | Fixed in |
|---|---|---|---|
| 28 | The default gate (`./run_tests.sh --rust`) run over a tree another agent was editing | A verdict about a tree that changed under it; two "green" gates were about no commit at all — announced windows and pathspec commits since | `2ea4ef21a` (the lesson), rule in `feedback` memory |
| 29 | `cargo check -p <crate>` as a peer's evidence for a schedule-set membership guard | Bevy names a system `<Enable the debug feature to see the name>` unless `bevy_ecs/debug` is on; the monolith's name-lookup guard passed under `--workspace` and failed under `-p` — count by shape, not by name (yardrat, `dbec94824`) | `dbec94824` |
| 30 | `cargo check --workspace --all-targets` as proof a merge was clean | An EXACT dependency allowlist (`engine.world-ir-dependency-allowlist`) is a policy test, not a compile; the merge of R4a added an edge the check could not see | `2b6e6561b` |
| 31 | The gate's Python lane resolving the FIRST usable-looking interpreter | An in-repo `.venv` from July predated the per-machine store and had no numpy; two red jobs read as "tests failed" until `python_tools.sh` was run — the runner now refuses an interpreter that cannot host the lane (yardrat) | `bd18a028f` |
| 32 | `\| tail -30` on a workspace lane | The exit status was `tail`'s; the lane was red on a missing bundled font and read `[exited with code 0]` (ambition-da) — `PIPESTATUS` or nothing | *(method; `grab_font_assets.py` is the fix)* |
| 33 | Probing whether an asset reloads after its handle drops, with a path that did not exist | `Failed(NotFound)` twice, both arms "consistent" — a probe that cannot distinguish its own typo from the defect; the real path reloaded in 2 updates | *(probe deleted; the defect was the fixture, `0a112fcb5`)* |
| 34 | A settle helper that ran ONE update and then read "pending is None" as "settled" | A launcher command becomes a route one update later than a `GoTo`; the fixture read the relaunch as done the frame before it started — masked for months while the relaunched room's cast was still resident | `0a112fcb5` |
| 35 | A docstring that describes a test, taken as the test | Three guards green for reasons unrelated to what they guard: the citation checker's population (no `::` → never extracted), the sheet-presence check exiting 0 when it could not import, the absence contracts' "require a hit" half never written (ambition-e7, `d3c86dc79` / `f4693ac7b` / `e4e545d29`) | *(theirs; listed here because the shape is the integrator's to watch for)* |
| 36 | A pathspec commit that names a file you edited | A `git reset` to origin by another agent in the shared tree reverted the file first; the commit landed without it and nothing complained — read the reflog before re-typing | `986f61d83` |
| 37 | Bisecting a merge-heavy day by `rev-list` index | An index is not an ancestry chain; a GOOD on a side branch bounded nothing on the line that mattered (yardrat) — bisect the ancestry, not the list | *(method)* |
