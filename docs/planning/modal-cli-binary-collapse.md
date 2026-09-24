# Modal CLI: collapsing the probe binaries

Jon's ruling 2026-09-03: build a modal CLI with `clap` and combine related
binaries. Standalone executables stay standalone for the game and the demos:
`ambition_game_bin`, `sanic_demo`, `smash_demo`, `mary_o_demo`,
`twintrack_demo`.

**Status:** built 2026-09-03. `smash_tool` in `ambition_demo_smash_app`
replaces nine probe binaries (`capture_probe`, `ladder_probe`, `ladder_rig`,
`match_diagram`, `match_report`, `match_shots`, `roll_probe`,
`select_walkthrough`, `stage_diagram`). Callers are retargeted, and a guard
checks that every `--bin <name>` in `docs/` and `scripts/` is a bin that cargo
builds. One measurement is open (below).

## Why

The nine binaries were one program with nine `main`s: the union of their
defined symbols was 0.16% larger than the largest one. In the default feature
cell the eight default-cell binaries took 4.79 GB on disk; one `smash_tool`
takes 0.50 GB. Its output is byte-identical to the old `ladder_rig` on the same
flags.

## Rules that stay

- Collapse within a crate, never across. Do not combine the `capture_*` family
  (`capture_scene`, `capture_twintrack`, `capture_mary_o`, `capture_sanic`,
  `capture_probe`). Each lives in the demo crate it photographs. One binary for
  all of them would link every demo and couple demos that do not know about
  each other, which `scripts/check_absence_contracts.py`
  (`capability-footprint-may-not-grow`) exists to prevent.
- The product entry points (the game binary and the four `*_demo` shells) stay
  standalone. A subcommand is the wrong shape for what a player launches.
- The single `[[bin]]` declares no `required-features`. Each subcommand is
  gated at its own definition: the default cell has six subcommands,
  `--features causal` adds `ladder-probe` and `match-report`, and
  `--features visible,capture` adds `match-shots`. A disabled subcommand keeps
  its name and exits non-zero with the features to rebuild with. `visible` is
  not a supported test cell for this crate, so a bin that required it would
  inherit that.
- Subcommand names are kebab-case and are public surface. Flags keep their old
  spelling (`--sweep-below`, `--scenarios`, `--weight`, `--no-rollout`). clap
  does not accept a flag after `--` or repeated, as the old hand parsers did.
- `split-debuginfo` stays unset. Measured on `smash_tool`: the executable
  shrank 41% but total disk use rose 324 MB, and the profile change recompiled
  352 crates (342 third-party). The note is beside `[profile.dev]` in the
  workspace-root `Cargo.toml`. Revisit only if a crate again has many binaries.

## Operational notes

- `cargo clean -p` does not remove a binary whose target no longer exists.
  After you rename or remove a bin, delete its old artifact by name.
- When the volume is short, `cargo clean -p` over the workspace packages is the
  right reset. It keeps third-party compiles. Most space is in `debug/deps/`
  test and bin executables, not `.rlib` files.
- A modal binary hides which tool is heavy. If that signal matters, measure
  symbol counts per module.

## Open

- Measure the featured cell (`--features visible,capture,causal`) before and
  after on a machine with disk headroom. The default-cell result is measured;
  the featured build pulls in the render stack and filled a shared disk when
  tried.
