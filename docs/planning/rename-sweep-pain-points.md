# Rename-sweep sed/grep pain points (2026-06-17)

Running log of where the manual `git mv` + `git grep -l <token> | xargs sed` crate/
module-rename workflow hurt, so we can build a better tool/process afterward. The
sweep itself (trace→gameplay_trace, effects→vfx, platformer_runtime→
platformer_primitives, actor→characters, sandbox→gameplay_core, + module moves)
worked, but each of these cost real time or caused a (caught) mistake.

## Pain points observed

1. **sed renames file CONTENTS, not file NAMES.** A crate rename that shares a name
   with a `[[bin]]` target broke the build: Cargo.toml's `[[bin]] path =
   "src/bin/ambition_sandbox.rs"` got sed'd to `…/ambition_gameplay_core.rs` but the
   *file* wasn't renamed → "can't find bin" target-resolution error (NOT caught by a
   normal `cargo check` until the bin is resolved). Same class: asset files whose
   *name* contains the token (`assets/dialogue/ambition_sandbox.yarn`) aren't renamed.
   → A rename tool must enumerate `[[bin]]`/`[[example]]`/`path=` targets + token-named
   files and `git mv` them.

2. **`git grep -l <token>` over-reaches in two directions at once.** It's the *right*
   way to find scattered refs (a `git add crates/ docs/` earlier MISSED
   `scripts/run_jspcd.sh`), but it also sweeps in (a) **asset data files** that contain
   the token as content and (b) **files that are simultaneously pre-existing
   uncommitted WIP**. The WIP case bit us: the sandbox-rename `git add $(git grep -l …)`
   staged 5 WIP files (a HUD text-change opt, an android hit-flash stub, an fps-overlay
   refresh, android profiling docs, a build-script `.so` filter) **whole** — junk and
   all — into the rename commit, because those files also referenced the crate. Had to
   `reset --soft` + per-file restore-to-rename-only to un-entangle.
   → Mitigations: **require a clean working tree** before a sweep (commit/stash WIP
   first); scope the sed to source+manifest globs and handle asset refs separately;
   stage by `git grep -l <NEW token>` *minus* a known-WIP exclude list.

3. **`cargo check` does not validate assets.** The sed touched asset *content*
   (`.ldtk`, `.ron`, `.yarn`); a broken asset path/id would pass `cargo check` and fail
   only at runtime. (This time it was benign — only a path-comment in
   `character_catalog.ron`; `sandbox.ldtk` was a byte-identical pure move — but the
   blast radius was invisible to the compiler.) → A rename flow needs an asset-load /
   asset-path-integrity smoke test, or must exclude `assets/**` from the content sed.

4. **Substring collisions need a pre-scan.** Real ones hit: `ambition_trace_test_dump`
   (a temp-dir string literal), `ambition_sandbox_android` / `_bg` (build output names),
   guard-test fn names like `ambition_sandbox_cargo_toml_has_no_fundsp_dep`. All were
   renamed consistently (harmless here) but a blind sed could corrupt an identifier that
   only *contains* the token. → Pre-scan `[A-Za-z_]*<token>[A-Za-z_]*` and review.

5. **String literals vs identifiers are conflated.** The dump-filename prefix
   `format!("ambition_trace_{ts}")` and asset-path strings are data, not crate refs; sed
   can't tell them apart. Usually fine, occasionally not.

6. **Common-word module names need word boundaries.** The pending module moves
   (`app/`, `runtime/`, `inventory/`) are English words; `s/crate::app/…/` must be
   boundary-aware (`crate::app::` not `crate::application`), and must not collide with
   `crate::platformer_runtime` when renaming `crate::runtime`.

7. **Bin/example target names double as product names.** Renaming the crate blindly
   renamed the playable *game* binary to `ambition_gameplay_core` (the engine-layer
   name) — a bad product name. Target names deserve a separate decision from crate names.

8. **`Cargo.lock`** must be excluded from the sed and left for `cargo` to regenerate.

9. **A format-on-read hook reflows code in files merely *read*** (seen in the prior
   docstring sweep) — not sed-specific, but it muddies "did my edit do that?" forensics.

## Naming rule (Jon, 2026-06-17)

**If we aren't 100% sure on a name, pick something deliberately *sed-able*** — a
unique, unambiguous token we can cheaply find-and-replace later — rather than
agonizing now or settling on something that collides. (e.g. the game binary was
parked as `ambition_game_bin`: a clear unique token, trivially renamable once we
decide the real product name.) Avoid names that are common English words or
substrings of other identifiers, since those are the ones that *aren't* cleanly
sed-able (see pain points 4 & 6).

## Follow-on plan (after the rename sweep lands)

- Build a small `rename-crate`/`rename-module` script (or adopt `cargo-rename`-like
  tooling) that: (a) requires a clean tree, (b) `git mv`s the dir + token-named files +
  bin/example targets, (c) seds code/manifest globs only (assets handled separately with
  review), (d) pre-scans + reports substring collisions and string-literal hits, (e)
  runs `cargo check --workspace` AND an asset smoke test, (f) leaves target-name (bin)
  decisions explicit.
- Add `check_doc_links`-style + asset-path-integrity checks to CI so silent asset/doc
  breakage from a rename fails loudly.
