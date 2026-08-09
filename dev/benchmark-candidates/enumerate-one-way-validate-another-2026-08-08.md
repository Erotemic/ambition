# A checker that ENUMERATES one way and VALIDATES another

**Tags:** `tooling-invariant`, `false-absence`, `checker-coverage`,
`gitignore`, `agent-verification`

## The shape

A checker asks a question about a population — every lockfile, every source
file, every workspace. It builds that population one way and then acts on it
another way:

```text
enumeration:  git ls-files          ← "what is TRACKED"
validation:   a filesystem walk     ← "what EXISTS"
```

Those are different questions, and **the gap between them is exactly where
generated artifacts and deliberately-ignored fixtures live, by construction.**
Anything in the gap is checked by the second half and invisible to the first, so
the checker reports success over a population it never enumerated.

⛔ **the failure is always in the reassuring direction.** A missed file cannot
produce a red; it can only fail to produce one. So the checker looks like it is
working right up until the thing it could not see breaks.

## Three instances in this repo

**1. Sub-workspace lockfiles (2026-08-08).** The repo has three workspaces
outside the root, each with its own `Cargo.lock`:

```sh
git ls-files '*Cargo.lock'                          # → 4
find . -name Cargo.lock -not -path "*/target/*"     # → 5
git check-ignore -v fixtures/external_consumer/Cargo.lock
#   fixtures/external_consumer/.gitignore:2:Cargo.lock
```

The fifth is excluded by a **nested** `.gitignore` two directories down —
invisible from the root and deliberate, because that fixture is meant to resolve
fresh. A brief written for the `bevy_material_ui` removal named only
`fixtures/minimal_game`, and the repo-tooling job failed on the one it had not
named.

**2. `.goal/*.json` and the platformer2d rename.** `.goal/active.json` holds the
goal harness's own check COMMANDS and is not in git. The rename retired
`ambition_actors` and `ambition`, two of those commands kept naming them, and
`cargo check -p ambition` failed on an unknown package — so the harness reported
*"S1 slice H is not done"* about work that was finished and green.
`check_retired_crate_names.py` sweeps `git ls-files`, so nothing caught it.
⛔ **a broken instrument reading as unfinished WORK is the most expensive
failure mode this repository has.**

**3. The guard's own test file.** `check_retired_crate_names.py` skips
`test_retired_crate_names.py` because its FIXTURES are retired names. That
skip went unnoticed at first because `git ls-files` did not yet list the file —
the live-tree ratchet passed while its own counter-example was untracked.
**Green at minute zero, one level in.**

**4. The symlink guard read the INDEX while the damage was in the WORKTREE
(2026-08-08).** The six LDtk worlds are tracked symlinks. A generator wrote a
real file over one of them, producing a **typechange**: `git status` says `T`,
and the index still reports mode `120000`. Every assertion in
`scripts/tests/test_map_symlinks_stay_links.py` read the index, so three passed
and the fourth — the only one touching the filesystem — died with
`OSError: [Errno 22] Invalid argument` out of `readlink`.

⭐ **this is the nastiest variant, because the two sources agree about the PAST.**
The index is not stale by accident; it correctly records what was committed. The
worktree correctly records what is there now. Neither is wrong, and a check that
consults one can be confidently, permanently blind to a break in the other.

⚠ **and the first repair made it worse.** An earlier fix for that same `OSError`
filtered the loop on the index mode — which is exactly the source that cannot
see the problem, so it silenced the symptom and preserved the blindness.

## The rule

⭐ **Derive the population and the action from the SAME source.** If a checker
will act on filesystem workspaces, discover its candidates from the filesystem
too — or state explicitly why the ignored population is excluded.

Both shapes are legitimate; what is not legitimate is mixing them silently:

* `scripts/tests/test_sub_workspace_lockfiles_are_current.py` discovers with
  `REPO.rglob("Cargo.lock")` and runs `cargo tree --locked` in each — filesystem
  to filesystem, and it carries a vacuity guard so an empty population is a
  failure rather than a pass.
* `scripts/check_retired_crate_names.py` sweeps `git ls-files` **and then names
  the untracked population it also needs** (`extra_paths()` adds `.goal/*.json`),
  with the incident above written above the function as the reason.

⚠ **and a nested `.gitignore` is invisible to the obvious check.** Reading the
root `.gitignore` would not have shown instance 1. Use
`git check-ignore -v <path>`, which names the file and the line that did it.

## How to check a checker in ten seconds

Count both populations and compare:

```sh
git ls-files '<pattern>' | wc -l
find . -name '<pattern>' -not -path './target/*' | wc -l
```

A disagreement is not automatically a bug — it is a question the checker's
comments should already answer. If they do not, the checker has the defect.

## Related

* [`grep -r` and friends lying about absence](../journals/) — same family at the
  SEARCH layer: recursive grep skips gitignored files and symlinked assets, and
  `| head -N` turns presence into apparent absence. When a search touches
  `assets/`, use `find … | xargs grep`.
* [`one-question-two-checkers-only-the-first-runs-2026-08-08.md`](one-question-two-checkers-only-the-first-runs-2026-08-08.md)
  — the sibling at the VALIDATION layer: the right population, checked twice,
  with only the first check reachable.
