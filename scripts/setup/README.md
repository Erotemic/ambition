# `scripts/setup/` — host and toolchain preparation

Everything here prepares a MACHINE. Nothing here builds the game or regenerates
content; that is `scripts/regen/` and `run_game.sh`.

`./run_developer_setup.sh` at the repo root is the umbrella and calls what a
fresh clone needs. These are the pieces, runnable on their own when you are
fixing one thing rather than bootstrapping.

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
