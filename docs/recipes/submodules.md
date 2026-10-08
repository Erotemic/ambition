# Submodules

Ambition keeps its authoring tools and large content in five submodules. This
page is the policy for them and the commands that carry it out. It is the one
place to look before you update, commit in or repoint a submodule.

## The rule

This is the rule for **normal development**: the superproject is on its `main`
branch, and so is each submodule. A stale gitlink on active development is not an
error, just bookkeeping that lags. Looking at an old commit is a different
situation with a different rule; see [Looking at an old commit](#looking-at-an-old-commit-review-mode).

1. **Every submodule is on its `main` branch**, tracking `origin/main`.
2. **The pin is the commit this repository records for a submodule** (the
   gitlink). When a pin disagrees with the submodule's `main`, **the pin is
   stale: update the pin.** Do not hold a checkout back to match an old pin, and
   do not work on a detached HEAD.
3. **Never lose work to make the rule true.** A checkout moves only when the move
   can neither orphan a commit nor overwrite local work (judged by what it would
   write, not by whether `git status` is clean). Anything else is left where it
   is and reported.

The history of the music, sprite and other tool repositories was split once, on
purpose (see [The history split](#the-history-split)). It is not an error.

## The submodules

| Path | Repository | What it is |
| --- | --- | --- |
| `tools/ambition_sprite2d_renderer` | `Erotemic/ambition_sprite2d_renderer` | sprite and rig renderer; also holds the design-history viewer |
| `tools/ambition_music_renderer` | `Erotemic/ambition_music_renderer` | music scores, renderer and the instrument-library downloader |
| `tools/ambition_sfx_renderer` | `Erotemic/ambition_sfx_renderer` | procedural SFX renderer |
| `game/ambition_map_assets` | `Erotemic/ambition_map_assets` | LDtk worlds and tilesets |
| `dev/ambition_dev_measurements` | `Erotemic/ambition_dev_measurements` | measurement ledgers (**append-only JSONL, see below**) |

An empty submodule directory does not mean the capability is absent; it means
the submodule is not initialized (`.gitmodules` lists them).

## Getting set up

```bash
./run_developer_setup.sh               # the whole machine, submodules included
scripts/setup/submodules.sh            # only the submodule phase
scripts/setup/submodules.sh --bump-pins   # also stage pins that lag main
scripts/setup/submodules.sh --follow-main  # development behaviour on a non-main checkout
```

`scripts/setup/submodules.sh` is idempotent and is what the rule above looks
like as code. For each submodule it fetches `origin`, then judges the move by
**what it would overwrite or orphan**, not by whether `git status` is clean. Four
facts are kept apart: where HEAD is, where **local `main`** is, where `origin/main`
is, and what the move would write.

| The state is | Setup does |
| --- | --- |
| not initialized | initializes it, then applies the rows below |
| local `main` is **ahead** of `origin/main` (unpushed commits), whichever branch is checked out | never resets or moves `main`; leaves the checkout; says what the commits are and what to do |
| local `main` has **diverged** from `origin/main` | leaves both sides; counts each; asks for reconciliation |
| the checkout (a branch or a detached HEAD) is ahead of, or diverged from, `origin/main` | leaves it (`LEFT ALONE`) |
| on a **named non-main branch with tracked edits** | leaves it on that branch, even when its commit equals `origin/main` |
| the move would **overwrite a modified tracked path** | changes nothing; names the paths |
| the move would write over an **untracked or ignored path** that exists | changes nothing; names the paths |
| otherwise: `main` absent, equal, or behind `origin/main` | attaches `main`, creating it or fast-forwarding it; tracked edits the move does not touch are carried through |
| on the **pre-split history** (no common commit with `origin/main`) | keeps the old HEAD and local `main` as `refs/backup/pre-split/<sha>`, then moves to `origin/main` |

Untracked files are **not** "dirty". An untracked `notes.txt` blocks nothing unless
`origin/main` adds a tracked `notes.txt`; an ignored file is held to the same rule
(git itself would overwrite it silently, so setup checks). Operations are
non-forcing: a fast-forward merge, a ref update guarded by the old value, and a
checkout that git refuses when it would lose work. `checkout -B` runs only for the
pre-split move, after both old positions are kept. Setup never commits, merges,
rebases or discards on your behalf.

### When work blocks convergence

The message names the state (it never just says "dirty"). In every case the
guidance is the same: **inspect the work; if it is important, review it, commit
it appropriately on the right branch, integrate it into the submodule's `main`
and push; then rerun setup.** Setup will not discard it.

| You see | It means | Do |
| --- | --- | --- |
| `local main has N commit(s) origin/main lacks` | unpushed development on `main` | `git -C <path> log --oneline origin/main..main`; integrate/push |
| `local main has diverged ... needs reconciliation` | `main` and `origin/main` each have commits the other lacks | merge `origin/main` into `main` keeping the semantic superset; push |
| `local tracked edits that would be overwritten` | an incoming change touches a path you modified | commit the edits on the right branch and integrate, or set them aside |
| `an untracked path that following origin/main needs to write` | an untracked or ignored file sits where the update writes | move it, or add and commit it if it is meaningful |
| `on branch 'X' with tracked edits; LEFT on 'X'` | work in progress that belongs to branch `X` | commit it on `X`, integrate into `main`, push |

Afterwards it compares each pin with the submodule's `origin/main`. A pin that
differs prints `the PIN needs updating`; `--bump-pins` stages those gitlinks
(`git add <path>`) and commits nothing.

## Looking at an old commit (review mode)

`git checkout <sha>`, `git bisect`, a rebase and a deliberate non-`main` branch
all leave the superproject somewhere other than `main`. Running a general setup
command there must not quietly turn historical Ambition code into a mixture of
that code and today's submodule tips, so the script picks its mode from what the
**superproject** is:

| The superproject is | Mode | Already-present submodules |
| --- | --- | --- |
| on branch `main` | development | follow their own `origin/main` (the table above) |
| on a detached HEAD, or on any other branch | **review** | **not moved**; any difference from the commit this checkout records is reported |
| anywhere, with `--follow-main` | development | follow `origin/main`, deliberately |

In review mode a **missing** submodule is initialized at the commit this checkout
records (not at today's `main`), the recorded pin is not called stale (it is the
historical truth), and `--bump-pins` is refused. A submodule left at a different
commit than the checkout records is named, with the command that restores the
recorded state (`git submodule update -- <path>`, safe only when the move would overwrite none of your local edits).

Dirty, ahead and diverged submodules are never touched in either mode. A fresh
`git worktree` on a feature branch is review mode by this rule, which is the
conservative reading; pass `--follow-main` there if you want it to develop
against each submodule's `main`.

## Updating a pin

A pin moves when a submodule's `main` has moved and this repository should use
the new state.

1. Make sure the submodule's work is **pushed** to its `main`. Check with
   `git -C <submodule> rev-list --count origin/main..HEAD` (it must be `0`), or
   `git merge-base --is-ancestor <sha> origin/main`.
2. `scripts/setup/submodules.sh --bump-pins`
3. Review `git diff --cached --submodule`, then commit:
   `git commit -m "Bump <name> to main: <what changed>"`.
4. Push the superproject. A pin that names a commit that is not on the
   submodule's `main` breaks every other clone.

Push a submodule **before** committing the superproject pointer to it. Do not
push another agent's uncommitted submodule work.

## Working inside a submodule

Commit on `main` and push. The usual flow is: change, commit and push in the
submodule, then bump the pin (above). If the submodule is on another branch
because you are mid-change, setup leaves it alone, which is the point.

Do not resolve a divergent submodule by recency or by `git submodule update`.
List each line's unique commits and keep the semantic superset
(`git -C <path> log --oneline origin/main..HEAD` and `HEAD..origin/main`), then
merge.

### The measurement ledgers are the exception to "fast-forward"

`dev/ambition_dev_measurements` holds append-only JSONL ledgers that several
machines write. Its `.gitattributes` declares `*.jsonl merge=union`, and
`.gitmodules` sets `update = merge` so a pointer bump merges instead of
checking out and discarding the other machine's rows. If it is **ahead or
diverged**, setup leaves it alone; merge it yourself so the union driver runs:

```bash
git -C dev/ambition_dev_measurements merge origin/main
```

## The history split

The history of the tool repositories was split once, intentionally, so a clone
made before the split has different root commits from `origin/main`. They are
**unrelated histories**: `git pull` and `git merge` in such a checkout stop with

```text
fatal: refusing to merge unrelated histories
```

That is expected. Do not merge, do not `--allow-unrelated-histories`. Run
`scripts/setup/submodules.sh`: it records the old position (and local `main`, if
it differs) under `refs/backup/pre-split/<sha>` and moves to `origin/main`, unless
the move would overwrite a local tracked edit or an untracked path.

By hand, the same thing is:

```bash
git -C <path> update-ref refs/backup/pre-split/$(git -C <path> rev-parse --short=12 HEAD) HEAD
git -C <path> checkout -B main origin/main
```

To look at what the old line held: `git -C <path> log refs/backup/pre-split/<sha>`
and `git -C <path> diff origin/main refs/backup/pre-split/<sha>`. Delete the ref
with `git -C <path> update-ref -d refs/backup/pre-split/<sha>` when you are sure
nothing there is wanted. A pre-split checkout whose tracked edits the move would overwrite is not moved;
commit the change on the right branch first.

## Troubleshooting

| You see | It means | Do |
| --- | --- | --- |
| `REVIEW mode` | the superproject is on a detached HEAD or a non-`main` branch | nothing, if you are reviewing; `--follow-main` if you are developing |
| `refusing to merge unrelated histories` | the checkout is on the pre-split lineage | `scripts/setup/submodules.sh` |
| `LEFT ALONE`, ahead | unpushed commits in the submodule | push them to `main`, then bump the pin |
| `LEFT ALONE`, diverged | both sides have commits | merge them, keeping the superset |
| `the PIN needs updating` | the pin lags the submodule's `main` | `scripts/setup/submodules.sh --bump-pins`, commit |
| `could not fetch origin (offline?)` | no network; judged from what is already fetched | rerun online |
| a fresh worktree has empty submodules | worktrees do not share initialized submodules | `python3 scripts/mirror_assets_for_worktree.py` (see [adding-an-asset](adding-an-asset.md)) |

Setting up system packages and audio libraries is a separate phase
(`scripts/setup/audio_libraries.sh`); a third-party apt repository with an
expired key does not stop it (see `tools/ambition_music_renderer/apt_helpers.sh`).

## Checking state

```bash
scripts/sync_status.sh     # every submodule against its recorded pin
git submodule status
```

The behaviour above is pinned by
`scripts/tests/test_submodules_phase_is_idempotent.py`, which builds real
superproject and submodule repositories and asks what happened to each checkout.
