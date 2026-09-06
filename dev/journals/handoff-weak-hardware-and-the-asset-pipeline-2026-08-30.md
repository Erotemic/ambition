# Handoff — the laptop frame, and the pipeline that kept unfixing itself

**Session ended 2026-08-30.** Two threads ran together: making the game playable
on hardware with no discrete GPU, and chasing why a character kept coming back
wearing another character's art. They turned out to share a shape — a build that
did not know what its own inputs were.

Tasking lives in [`docs/planning/queue.md`](../../docs/planning/queue.md) as
`D-RASTER-1..5` and `D-BUILD-1..3`. **This file is what the queue rows do not
say**: the machine's state, the traps, and where the one open bug actually is.

---

## The machines

| | what | note |
|---|---|---|
| `calculex` | Jon's laptop, i7-7700HQ, **Intel HD 630**, Ubuntu 22.04 | the only host that can show a window |
| `aivm-2404` | KVM guest **on calculex**, 6 vCPU, 15.6 GB, no GPU | the agent VM; ⛔ NOT the i9 the old `hardware.md` described |

⛔⛔ **THE REPO IS ONE DIRECTORY SHARED OVER VIRTIOFS.** `/home/joncrall/code/ambition`
in the VM and `~/code/ambition` on the laptop are the same files; only `target/`
is shadowed per side. Home directories are NOT shared.

⛔ **NEVER pull, rebase, commit or edit while a host-side build or profile run is
in flight.** Done once this session: a `git pull --rebase` at 21:25:39 landed in
the middle of a running cargo build and produced a compile error naming a symbol
that demonstrably existed at HEAD. Cost a ten-minute build and a wasted capture.
A compile error naming a symbol that exists is the signature — check `git reflog`
timestamps against the run's start before believing the tree is broken.

⚠ **Another agent works in this tree.** At session end it had uncommitted edits in
`ambition_input`, `platformer2d_host`, `smash_in_the_host.rs` and
`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md` (its Quit-to-Title fix). Do not commit
files you did not change; `git pull --rebase --autostash` is the safe form.

## What is set up and should be left alone

* `ambition.local.toml` (git-ignored) — calculex boots **Medium**, plus explicit
  `[raster] max_scale_factor = 1.0, msaa = 1`. The raster keys are redundant with
  Medium **on purpose**, so moving the tier to `high` to look at something does
  not silently restore 3200x1800 rasterisation.
* Tool venvs now live in `~/.cache/ambition-tool-venvs/<tool>`, resolved by
  `scripts/lib/tool_python.sh` BEFORE any in-repo `.venv`. The VM has a working
  `ambition_sprite2d_renderer` venv, so it can render sprites for real.

## The result, and what it does not say

**p50 51.0ms → 20.1ms; ~19.6 → ~49.7 FPS**, n=3 per arm, matched on build
features, same comparability key. Cause: a 1600x900 window rasterised at
3200x1800 (a 2x Wayland scale nothing capped) with Bevy's default 4x MSAA on top.

⚠ **Nothing splits that between the two knobs** — they moved together, and both
are individually settable (`AMBITION_MAX_SCALE_FACTOR`, `AMBITION_MSAA`) so the
interleaved A/B can be run. That is `D-RASTER-3` and it is cheap.

⛔ **Retired:** *"the GPU is not the problem — the transparent 2D pass is
~0.047ms."* True of an RTX 3090. The same pass ran 4.70ms here.

Full campaign, including six things it does NOT establish:
`dev/ambition_dev_measurements/journal/2026-08-29-the-laptop-frame.md`.

## ⛔ Traps that cost real time this session

1. **`build.incremental = true` applies to the OPTIMIZED profiles.** At
   release opt-level it breaks the link (`mold: error: undefined symbol:
   anon.<hash>.llvm.<hash>`) and, before that, produced binaries that linked and
   then **segfaulted at 25–32s**. Three Tracy runs died that way and the
   "Tracy is unstable here" conclusion was WRONG. `run_game.sh` now exports
   `CARGO_INCREMENTAL=0` for `release`/`ship`/`profiling`.
   ⚠ Cargo lets `build.incremental` OVERRIDE `profile.<name>.incremental`, the
   opposite of the usual precedence — the natural fix in `Cargo.toml` silently
   does nothing.
2. **A full sprite publish is not a targeted one.** `sprites.sh --target X` and
   a full `assets.sh` could produce DIFFERENT art for the same character. Fixed
   (`D-BUILD-3`), but if a character's art regresses, check that first.
3. **`--features profile` links Tracy's C++ client, so the link needs
   `libstdc++.so`** — and clang resolves it against exactly ONE gcc version
   directory. `g++` being installed proves nothing. `scripts/setup/profile_deps.sh`
   answers this in a second.
4. **A profiling bundle whose warm build failed still records a "run".** The
   ingest refuses it, correctly, but the capture is wasted — check
   `warm-build.status` before reading any bundle.

## Instruments that exist now (use them before building another)

| command | answers |
|---|---|
| `scripts/sync_status.sh` | is anything unmerged, unpushed, uncommitted, across every branch/worktree/submodule |
| `scripts/setup/profile_deps.sh --check` | can this machine profile at all, and what exactly is missing |
| `scripts/regen/sprites.sh --check-toolchain` | can this machine render sprites, or would it silently substitute |
| `scripts/check_quality_variants_are_fresh.py` | which generated variants are stale (now run by `run_developer_setup.sh`) |
| `target/debug/officer_probe left\|right` | the officer's shot, tick by tick, on a real host |
| `render/upscaling/fragment_shader_invocations` | the framebuffer's pixel count, EXACTLY — check it before reading any timing |

## The one open bug, and exactly where it is

**"The officer is still firing backwards."** `officer_probe` establishes that the
SIMULATION is correct in every measurable respect, both directions:

```
facing +1   spawn offset +9.3 (ahead)   vel.x +560.0   fires at @0.35
facing -1   spawn offset -9.3 (ahead)   vel.x -560.0   fires at @0.35
```

Aim resolution, timing (`fb9230363`'s 0.348s holds), spawn origin and body flip
are all right — `authored_faces_left: true` is correctly declared on his sheet and
matches his authoring source. ⇒ **It is presentation.**

⭐ **The candidate, and it is the same structural gap in a second place.**
`crates/ambition_render/src/rendering/projectile_visuals.rs` flips a round with
`sprite.flip_x = view.vel.x < 0.0`, which ASSUMES the art is drawn pointing +X.
Character sheets got an `authored_faces_left` declaration precisely because that
assumption is false for a handful of them (the officer is one). **Projectile art
has no equivalent declaration anywhere.**

⛔ **NOT CONFIRMED, and it needs eyes on the art**: which way the officer's round
and muzzle flare are actually drawn is the one fact the probe cannot read. If they
point left, the fix is to give projectiles the declaration characters have — not
to flip the sign, which would break every correctly-drawn projectile.

## Deliberately not done

* **The shrine.** `shrine_visuals.rs:276` loads `sprites/shrine_spritesheet.png`;
  the file is published to `sprites/props/`. `SheetRecord.image` is documented as
  a bare filename with no directory, so the real fix is that the shrine's SHEET
  should not be under `props/` at all — an asset-layout change in the renderer
  submodule. Cosmetic today: the flat PNG fallback still draws.
* **`sandbag`** has both a YAML config and a module target. Checked and LEGAL —
  the config drives the generator that lives in that module, so both routes draw
  the same art, and its 11 published rows come from the config. The
  one-sheet-one-renderer guard allows it on purpose.
