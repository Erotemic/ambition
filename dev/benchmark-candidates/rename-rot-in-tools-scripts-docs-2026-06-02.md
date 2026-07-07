# A crate/directory rename rots tool scripts, shell defaults, and help text silently

**Date:** 2026-06-02
**Tags:** `rename-refactor`, `dev-tooling`, `silent-failure`, `shell`, `python`, `docs`

## Mistake

Two completed renames in this repo —
`tools/audio/music_renderer/` → `tools/ambition_music_renderer/` and the
`ambition_engine` crate collapsed into
`crates/ambition_actors/src/engine_core/` — left **five dev tools broken
without anyone noticing**, because none of the stale references were in
Rust source (which would have failed `cargo build`):

1. `audit_cue_balance.py` defaulted its `root` arg to the deleted
   `tools/audio/music_renderer/output/first_goblin_tune_v2`, so a no-arg
   run (exactly as the recipe documents) died with `missing root`.
2. `regen_sprites.sh`'s header pointed adapter configs at
   `tools/ambition_sprite2d_renderer/configs/` — a path that doesn't
   exist (the YAML lives in the *inner* package dir
   `…/ambition_sprite2d_renderer/configs/`), sending any agent that
   followed it to an empty directory.
3. `test_coverage_report.sh` scanned `crates/ambition_engine/src`, so its
   `engine` and `both` modes silently found zero files.
4. `collect_optimization_report.py` ran `cargo check -p ambition_engine`,
   which errors out (`no such package`), failing one report step.
5. `install_first_goblin_tune_v2.py`'s `--help` text advertised the old
   `tools/audio/music_renderer/output/<cue>` autodetect path (the
   detection *logic* was already correct; only the help string lied).

Each "works on my machine"-passed at rename time because the rename
author only re-ran the Rust build, which is blind to shell defaults,
argparse defaults, doc comments, and `--help` strings.

## The principle the agent missed

**A rename is not done when the code compiles.** Code references to a
renamed symbol/path fail loudly; *everything else* fails silently and
lazily — at the moment a human or agent next runs the tool, reads the
help, or follows the comment. The blast radius of a crate/dir rename
includes:

- argparse / CLI **default values** (`default="old/path"`),
- shell script **path literals** and `cd` targets,
- **doc comments** and `--help` strings that name paths,
- `cargo -p <pkg>` invocations in CI/report scripts,
- recipe/README command blocks.

The grep sweep that finishes a rename must cover `*.sh *.py *.md *.toml`
and help/string literals — not just `*.rs`. The compiler is not your
safety net here.

## Pre-mistake context

The agent doing the rename had:
- A green `cargo build` / `cargo test` after moving the code.
- The new paths in place and working from Rust.
- No failing signal from any tool, because the tools aren't exercised by
  the Rust build or the default test run.

The available-but-unused signal: a repo-wide `grep -rn "old_name"` across
non-Rust files would have listed every silent reference. The mistake was
treating "the crate builds" as "the rename is complete."

## Repair shape

Resolve tool paths relative to the script, and sweep for the old name in
all file types:

```bash
# 1. Find every silent reference (not just code):
rg -n 'tools/audio/music_renderer|ambition_engine' \
   --glob '*.sh' --glob '*.py' --glob '*.md' --glob '*.toml'

# 2. Make tool defaults robust to cwd instead of hardcoding a tree path:
```
```python
# audit_cue_balance.py — was: default="tools/audio/music_renderer/output/..."
DEFAULT_ROOT = Path(__file__).resolve().parent / "output" / "first_goblin_tune_v2"
parser.add_argument("root", nargs="?", default=str(DEFAULT_ROOT))
```
```bash
# test_coverage_report.sh — was: crates/ambition_engine/src
engine) targets=("$REPO_ROOT/crates/ambition_actors/src/engine_core") ;;
```

## Why this is a good benchmark question

The agent must:
1. Recognize that a "completed" rename (code green) still has a silent
   tail in non-compiled artifacts.
2. Enumerate the *categories* of silent reference (CLI defaults, shell
   literals, doc/help strings, `cargo -p`), not just one.
3. Choose a robust fix (resolve relative to `__file__` / `$BASH_SOURCE`)
   over re-hardcoding the new path, so the next move doesn't rot again.

The agent bias to resist: declaring victory at the green build.

## Compact question

> The crate `ambition_engine` was just deleted and its code moved into
> `crates/ambition_actors/src/engine_core/`. `cargo build` and
> `cargo test` are green. List the categories of reference that a Rust
> build will NOT catch but that are now broken, and write the grep that
> finds them. Then fix `tools/test_coverage_report.sh`, which has
> `engine) targets=("$REPO_ROOT/crates/ambition_engine/src")`, so the
> tool keeps working from any cwd.

## Validation

```bash
# After the sweep, this should return only intentional historical mentions:
rg -n 'crates/ambition_engine|-p ambition_engine|tools/audio/music_renderer' \
   --glob '*.sh' --glob '*.py' --glob '*.toml'
# And the repaired tool should list files (not error / be empty):
bash tools/test_coverage_report.sh engine | head
```
