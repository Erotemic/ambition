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
| 21 | `test_absence_contracts.py` green over 25 architectural contracts | Its docstring promises a test that feeds each contract a violating line "and requires a hit". That test did not exist — `VIOLATING_LINE` was read only by the SILENCE test, so no fixture had ever been checked for violating anything | `e4e545d29` |
| 20 | `check_published_sheets_are_present.py` exiting 0 | Its `claimed_install_names` import was bare, so on any checkout without the tool venv it returned `None`, printed "cannot check" and exited 0 — a guard off on every machine that skipped a setup step | `f4693ac7b` |
| 19 | `check_planning_citations.py` reporting "all resolved" over tonight's rows | `SYMBOL` needs a `::`, so bare backticked names — the commonest form in these docs — are never extracted; a fabricated one left the count unchanged at 526 | `d3c86dc79` |
| 18 | "The four unshrunk variants are `_authored_swing_fighter.render` dropping the scale" | A coherent mechanism, in the wrong road: that module is never called for tiers — the variant script resizes for every module-kind target. Two boxes disagreeing is what exposed it | `c77f66425` → reconciled |
| 17 | "44% of the tree" as the occupancy denominator | Half the MEGAPIXELS are unclaimed but only a fifth of the BYTES — stranded pages are large and empty, so they compress to nearly nothing | `334086d9c` |

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

⇒ And the one that would have saved the most TIME: items 13 and 16 are the same
mistake twice in one night — **reading code to infer what a tool would report,
when the tool was one call away.** `discover_all_targets()` and
`grep -l <shared module>` each ended a chain of failed hypotheses immediately.

## The second set (ambition-df, the integrator's half)

| # | The check | Why it could not fail | Fixed in |
|---|---|---|---|
| 19 | The default gate (`./run_tests.sh --rust`) run over a tree another agent was editing | A verdict about a tree that changed under it; two "green" gates were about no commit at all — announced windows and pathspec commits since | `2ea4ef21a` (the lesson), rule in `feedback` memory |
| 20 | `cargo check -p <crate>` as a peer's evidence for a schedule-set membership guard | Bevy names a system `<Enable the debug feature to see the name>` unless `bevy_ecs/debug` is on; the monolith's name-lookup guard passed under `--workspace` and failed under `-p` — count by shape, not by name (yardrat, `dbec94824`) | `dbec94824` |
| 21 | `cargo check --workspace --all-targets` as proof a merge was clean | An EXACT dependency allowlist (`engine.world-ir-dependency-allowlist`) is a policy test, not a compile; the merge of R4a added an edge the check could not see | `2b6e6561b` |
| 22 | The gate's Python lane resolving the FIRST usable-looking interpreter | An in-repo `.venv` from July predated the per-machine store and had no numpy; two red jobs read as "tests failed" until `python_tools.sh` was run — the runner now refuses an interpreter that cannot host the lane (yardrat) | `bd18a028f` |
| 23 | `\| tail -30` on a workspace lane | The exit status was `tail`'s; the lane was red on a missing bundled font and read `[exited with code 0]` (ambition-da) — `PIPESTATUS` or nothing | *(method; `grab_font_assets.py` is the fix)* |
| 24 | Probing whether an asset reloads after its handle drops, with a path that did not exist | `Failed(NotFound)` twice, both arms "consistent" — a probe that cannot distinguish its own typo from the defect; the real path reloaded in 2 updates | *(probe deleted; the defect was the fixture, `0a112fcb5`)* |
| 25 | A settle helper that ran ONE update and then read "pending is None" as "settled" | A launcher command becomes a route one update later than a `GoTo`; the fixture read the relaunch as done the frame before it started — masked for months while the relaunched room's cast was still resident | `0a112fcb5` |
| 28 | A docstring that describes a test, taken as the test | Three guards green for reasons unrelated to what they guard: the citation checker's population (no `::` → never extracted), the sheet-presence check exiting 0 when it could not import, the absence contracts' "require a hit" half never written (ambition-e7, `d3c86dc79` / `f4693ac7b` / `e4e545d29`) | *(theirs; listed here because the shape is the integrator's to watch for)* |
| 27 | A pathspec commit that names a file you edited | A `git reset` to origin by another agent in the shared tree reverted the file first; the commit landed without it and nothing complained — read the reflog before re-typing | `986f61d83` |
| 26 | Bisecting a merge-heavy day by `rev-list` index | An index is not an ancestry chain; a GOOD on a side branch bounded nothing on the line that mattered (yardrat) — bisect the ancestry, not the list | *(method)* |
