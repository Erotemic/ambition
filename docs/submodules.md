# Submodules

Ambition keeps its authoring tools and large content in five submodules. This
page is the policy for them and the commands that carry it out. It is the one
place to look before you update, commit in or repoint a submodule.

## The rule

1. **Every submodule is on its `main` branch**, tracking `origin/main`.
2. **The pin is the commit this repository records for a submodule** (the
   gitlink). When a pin disagrees with the submodule's `main`, **the pin is
   stale: update the pin.** Do not hold a checkout back to match an old pin, and
   do not work on a detached HEAD.
3. **Never lose work to make the rule true.** A checkout moves only when it is
   clean and the move cannot orphan a commit. Anything else is left where it is
   and reported.

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
```

`scripts/setup/submodules.sh` is idempotent and is what the rule above looks
like as code. For each submodule it fetches `origin` and then does exactly one
of these:

| The checkout is | It does |
| --- | --- |
| not initialized | initializes it, then applies the rows below |
| at `origin/main`, detached or on another branch name | attaches `main` there; no content moves |
| strictly behind `origin/main`, clean | fast-forwards `main` to `origin/main` |
| on the **pre-split history** (no common commit with `origin/main`), clean | keeps the old position as `refs/backup/pre-split/<sha>` and moves to `origin/main` |
| ahead of `origin/main` (unpushed commits) | **leaves it** and says `LEFT ALONE` |
| diverged from `origin/main` | **leaves it**, counts the commits on each side |
| dirty (uncommitted changes) in any of the moving cases | **leaves it** |

Afterwards it compares each pin with the submodule's `origin/main`. A pin that
differs prints `the PIN needs updating`; `--bump-pins` stages those gitlinks
(`git add <path>`) and commits nothing.

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
`scripts/setup/submodules.sh`: for a clean checkout it records the old position
under `refs/backup/pre-split/<sha>` and moves to `origin/main`.

By hand, the same thing is:

```bash
git -C <path> update-ref refs/backup/pre-split/$(git -C <path> rev-parse --short=12 HEAD) HEAD
git -C <path> checkout -B main origin/main
```

To look at what the old line held: `git -C <path> log refs/backup/pre-split/<sha>`
and `git -C <path> diff origin/main refs/backup/pre-split/<sha>`. Delete the ref
with `git -C <path> update-ref -d refs/backup/pre-split/<sha>` when you are sure
nothing there is wanted. A dirty pre-split checkout is not moved; commit or stash
the change first.

## Troubleshooting

| You see | It means | Do |
| --- | --- | --- |
| `refusing to merge unrelated histories` | the checkout is on the pre-split lineage | `scripts/setup/submodules.sh` |
| `LEFT ALONE`, ahead | unpushed commits in the submodule | push them to `main`, then bump the pin |
| `LEFT ALONE`, diverged | both sides have commits | merge them, keeping the superset |
| `the PIN needs updating` | the pin lags the submodule's `main` | `scripts/setup/submodules.sh --bump-pins`, commit |
| `could not fetch origin (offline?)` | no network; judged from what is already fetched | rerun online |
| a fresh worktree has empty submodules | worktrees do not share initialized submodules | `python3 scripts/mirror_assets_for_worktree.py` (see [adding-an-asset](recipes/adding-an-asset.md)) |

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
