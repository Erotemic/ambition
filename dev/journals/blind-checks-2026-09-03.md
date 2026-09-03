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
| 23 | `check_authored_levels_survive` reporting "found no .ldtk worlds at all" | `SKIP_PARTS` contains `.worktrees` and was matched against the ABSOLUTE path — an agent slot's own root is `<repo>/.worktrees/<slot>`, so every world was skipped. 12 on disk, 0 kept. Inoperative in exactly the trees the agents work in | `<this commit>` |
| 22 | Five suite failures dismissed as "cargo is not on this PATH" | `~/.cargo/bin/cargo` exists; four scripts resolved it and two call sites did not. Fixing that turned 4 into passes and left ONE REAL finding the crash had hidden — `examples/portal_tutorial/Cargo.lock` stale since `ambition_registry_core` landed | `3959d8b27` |
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
