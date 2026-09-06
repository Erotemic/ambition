# `scripts/setup/` — host and toolchain preparation

Everything here prepares a MACHINE so that a fresh clone can build and run the
game. Most of it installs toolchains and environments and touches no content.

⚠ ONE PHASE IS THE EXCEPTION, and the opening line used to deny it existed:
`generated_content.sh` regenerates every runtime asset and the `.agent/` index.
It does not reimplement that work — it calls `scripts/regen/assets.sh` and
`scripts/regen/source_navigation.sh`, which own it — because a fresh clone is
not prepared until the generated content exists, and `scripts/regen/` is where
you go to re-run one category by hand afterwards.

`./run_developer_setup.sh` at the repo root is the umbrella and calls what a
fresh clone needs. These are the pieces, runnable on their own when you are
fixing one thing rather than bootstrapping.

## The phases `run_developer_setup.sh` runs

The umbrella is an ORCHESTRATOR: it calls these in order and owns no copy of
their logic, so the two cannot drift. Run one directly when you are fixing one
thing, or to assemble a lighter setup than the default.

| phase | what it does | needs first |
|---|---|---|
| `system_packages.sh` | host apt packages for a desktop build and the asset pipeline | — |
| `rust_toolchain.sh` | rustup + stable + rustfmt/clippy/llvm-tools | — |
| `submodules.sh` | recursive authoring submodules | — |
| `resource_tally.sh` | arms the accounting git hook | — |
| `python_tools.sh` | a per-machine venv for each authoring tool, plus the `scripts/` environment. `--verify` checks without installing | submodules |
| `audio_libraries.sh` | sampled instruments + sfizz. `--status` reports what this machine has | system packages |
| `generated_content.sh` | every runtime asset, plus the `.agent/` navigation index | python tools, audio libraries |
| `desktop_check.sh` | `cargo fetch --locked` + checks `ambition_app` | rust, submodules |
| `profiling_tools.sh` | ⭐ the ONLY thing the default run leaves out — `--profile` | rust |

The first four are independent of each other and safe to run in any order or
concurrently; the rest follow the dependencies in the last column. The umbrella
runs them sequentially because apt takes a machine-wide lock and the asset
pipeline already saturates the CPU on its own.

⛔ **`audio_libraries.sh` IS NOT OPTIONAL FOR A COMPLETE SETUP.** Without it
every sampled instrument becomes a General-MIDI stand-in that nothing downstream
can distinguish from the real cue, so the machine ships the wrong music and
reports success. It is the one phase whose cost (tens of GB) tempts people to
skip it, which is exactly why it is its own script with a `--status`: assemble a
compile-only machine deliberately, and know that it cannot regenerate music.

| script | when you need it |
|---|---|
| `target_bindmount.sh` | ⛔ **first, every session, on a virtiofs checkout.** `--status` says whether `target/` is bound to local disk. An unbound session builds through the shared mount — minutes per check, and a second full copy of every artifact. See AGENTS.md. |
| `profile_deps.sh` | before the first `scripts/profile_desktop.sh` on a machine. Asks whether the profiling toolchain WORKS rather than whether it is installed — `--check` reports without changing anything. |
| `android_prereqs.sh` | Android SDK/NDK for `build_for_android.sh`. |
| `web_prereq.sh` | wasm target and bindgen for `build_for_web.sh`. |

## Why `profile_deps.sh` is separate from `run_developer_setup.sh --profile`

That flag INSTALLS the profiling toolchain. It cannot tell you the toolchain is
broken, and on `calculex` it was: `apt install g++` reported "already the newest
version" and the profiling build still died at the final link with
`mold: library not found: stdc++`, after all 537 rlibs had compiled.

clang resolves `-lstdc++` against exactly ONE gcc version directory — the newest
complete install under `/usr/lib/gcc/<triple>/` — and passes only that as `-L`.
gcc 9 and gcc 11 both had `libstdc++.so`; clang had selected gcc 12, which did
not. ⭐ **No package list can express that. A question to the compiler answers it
in a second**, which is what `profile_deps.sh` does.

## Adding one

- Take `--help`, and print a header that says what the script prepares and when
  someone needs it.
- Be idempotent: these get re-run, and a second run must be a no-op, not a
  reinstall.
- Prefer CHECKING and naming the fix over silently installing, and make the
  automatic fix the flag rather than the default where the change is large.
- Compute `repo_root` two levels up — ⚠ these live in `scripts/setup/`, not at
  the repo root.
