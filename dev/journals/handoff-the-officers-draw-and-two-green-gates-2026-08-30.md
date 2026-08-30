# Handoff — the Officer's draw, and the first two green gates

**Session ended 2026-08-30.** It began with two handoffs
([queue rows](../../docs/planning/queue.md), `D-QTT-*`, `D-RASTER-*`,
`D-BUILD-*`) and Jon's own charge: *"ensure that the officer side b fires in the
right direction and has correct visuals to match."*

Tasking and receipts live in `docs/planning/queue.md`. **This file is what the
rows do not say**: what the failures had in common, and the two traps that cost
real time.

---

## The one sentence

**Every failure this session was a MISSING ASSET wearing a code bug's clothes.**
The Officer's shot, `cargo test --workspace --lib`, and the `app_it` sweep each
presented as a logic defect and each turned out to be art that was never
published, or art that was published under the wrong identity.

## The Officer (D-OFFICER-1) — the simulation was innocent all along

`officer_probe` had already established the aim, the timing, the spawn and the
body flip as correct, and the previous handoff concluded from that "it is
presentation." That was right. What it named as the suspect was not.

* **No `Discharge`** ⇒ `Muzzle::BodyOrigin`, whose spawn offset is
  `origin + (0, -8)` — **purely vertical**. The round was born at his sternum
  while the gun and its flare are drawn out at his hand.
* **No `visual`** ⇒ an empty id resolving to `ProjectileArt::generic()`, the
  engine's orange-red **quad**.

⛔⛔ **THE "+9.3 (AHEAD)" IN THE LAST HANDOFF WAS NOT A MUZZLE OFFSET, AND IT IS
WHAT CLEARED THE REAL BUG.** `officer_probe` samples the first tick a round is
VISIBLE — one tick after it spawns. 560 px/s ÷ 60 Hz = **9.33**. It was
reporting one tick of TRAVEL as the spawn offset, so `BodyOrigin` — horizontally
ON him — printed as *"ahead of him, as it should be"*. Any probe that samples
after `app.update()` has this bug latent in it; check what your instrument can
actually see before trusting its verdict.

⛔ **AND THE GENERIC QUAD IS WHY THE FLIP LOOKED INNOCENT.** The last handoff's
⭐ candidate was `projectile_visuals.rs` flipping with `flip_x = vel.x < 0.0` on
art that might be drawn facing left. It could not have been the bug: **a solid
colour quad is symmetric**, so that flip was a no-op which could be neither
right nor wrong on screen. Real directional art is what makes the axis testable
at all.

**Verified both ways** (`officer_probe left|right`): born at **+18.6 / -18.6
AHEAD**, `vel.x` ±560 agreeing with facing, 71 round-ticks each. And **seen**,
via `moveset_render --character officer --verb special_forward` on lavapipe:
each round draws with its hot tip LEADING its direction of travel.

⚠ **THE PROBE WAS BLIND IN ONE DIRECTION AND SAID SO BACKWARDS.** Once the
muzzle moved to the hand, the `right` run printed *"NO ROUND EVER SPAWNED"*
beside an `impact_hitstop` in the sim clock: both seats are Officers standing
close and the probe HOLDS the stick, so firing right walked into the sparring
partner and the round landed on the tick it spawned. A point-blank hit is a hit.
It now stands the partner behind the shooter.

## The gates (D-QTT-1, D-QTT-2) — both failed on their first complete run

| gate | result | what it caught |
|---|---|---|
| `cargo test --workspace --lib` | **exit 0, 50 crates, 3695 passed** | `goblin_cave_dagger` unpublished — a handedness panic |
| `app_it --test-threads=2` | **508 passed, 0 failed** | `gnu_ton_apple.png` absent — the boss's apple rain drew a quad |

Neither logs. Neither crashes. `resolve` falls back silently, so the game just
draws the wrong thing — and **generated art is gitignored**, so *"it works on my
checkout"* is the EXPECTED symptom of this class rather than a surprising one.

⭐ **FRESHNESS AND PRESENCE ARE DIFFERENT FAILURES AND ONLY ONE HAD AN
INSTRUMENT.** `check_quality_variants_are_fresh.py` asks whether published art
is OLDER than its source — **a file that does not exist cannot be stale**. New
`scripts/check_published_sheets_are_present.py` closes the other half, and
`run_developer_setup.sh` now GENERATES what it finds missing.

⛔ That checker asks each target what it DECLARES it installs
(`Target.claimed_install_names`). The first draft guessed
`<target>_spritesheet.ron` and reported four healthy targets as missing —
`town_tileset` installs `.png`/`.yaml`, `interdimensional_gate` installs
`_ring_`/`_portal_` sheets, `gnu_ton_boss` installs into a subdir. A checker
that cries wolf teaches people to skip the step.

⚠ And it must read `publish_targets`, which is the REAL roster and is mostly
`"${array[@]}"` expansions. Reading only `review_cues`/`tackon_targets` is how
`gnu_ton_apple` stayed invisible to it. 163 targets covered → 173.

## ⛔ Traps that cost real time

1. **⛔⛔ THE BINDMOUNT IS THE FIRST TOOL CALL OF A SESSION THAT WILL BUILD.**
   `AGENTS.md:88` says run `target_bindmount.sh --status` BEFORE the first
   build, every session. This session ran `cargo test` first and checked after —
   the mount was NOT bound, so that build compiled through virtiofs into Jon's
   shared host-side `target/` and was then killed mid-flight. Jon: *"you just
   fucked my warm cache"* … *"This mistake can't be undone."* ⚠ And `pgrep`
   inside the VM CANNOT see a host-side build, so "nothing is running" is never
   proof the tree is idle. Bind first, then build.
2. **⛔ KILLING A BUILD IS NOT FREE, AND `-9` IS THE EXPENSIVE ONE.** A
   `pkill -9` mid-archive left `syn` with an `.rmeta` and no `.rlib`; cargo's
   pipelining then started dependents that died at LINK with *"can't find crate
   for `proc_macro2`"* — a fake error naming crates that were fine. Let `rustc`
   drain instead: `while pgrep rustc >/dev/null; do sleep 5; done`.
3. **⚠ ASSET WORK INVALIDATES THE RUST BUILD.** `build.rs` bakes every
   `*_spritesheet.ron` under `assets/sprites` **and** the `sprites_0_5x` /
   `_0_25x` / `_potato` tiers. Regenerating art mid-build is the same class of
   mid-flight tree mutation as a `git pull`. Do all asset work first, then build
   once.
4. **⚠ `--test-threads=2` IS WHAT MAKES `app_it` SURVIVE.** 15 GB, no swap,
   Bevy apps. It finished with ~13 GB free; the default 6-way had OOM-killed it
   twice.

## What is NOT done, and why

* **D-RASTER-3** (split the 2.76x between the DPI cap and MSAA) and
  **D-RASTER-2**'s remaining half (read the new area ratio, then do the engine
  work) **cannot be run from `aivm-2404`.** The VM has no GPU — only lavapipe, a
  CPU rasteriser. A DPI-cap arm measured here reports the speed of a renderer
  the game never ships on. ⛔ Do not "unblock" these by running them here: a
  number from the wrong rasteriser is worse than no number, because it will be
  quoted later. `calculex` (Intel HD 630 since the discrete card died) is the
  only host that can answer them.
* **D-BG-1 — Jon's *"backgrounds aren't rendering"* is UNCONFIRMED, not fixed.**
  Everything static checks out (art present and opaque at all four tiers,
  Medium's `take(3)` keeps the sky, every id registered), and a render at
  `AMBITION_QUALITY_PROFILE=medium` loads 4/4 layers and draws them. ⚠ **But the
  game was never run BEFORE the regeneration**, so this session cannot say
  whether the 184 stale tier files were the cause. If it recurs, capture a frame
  FIRST, and read the `[game_assets] loaded N/4` line — `spawn_parallax_layers`
  returns early when `assets.parallax_layers.is_empty()`.
