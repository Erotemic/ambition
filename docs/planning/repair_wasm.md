Repair Ambition’s supported WASM builds.

Repository root: use the current Ambition checkout. Read the root `AGENTS.md` and any nearer `AGENTS.md` files before editing.

## Goal

Make these exact commands compile successfully:

```bash
cargo check \
    -p ambition_app \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features web
```

```bash
cargo check \
    -p ambition_app \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features web_served_assets
```

Also preserve the normal native desktop build and existing feature combinations.

## Important diagnosis

The current failures are not evidence that Bevy Lunex fundamentally cannot support WASM. The `web` persona intentionally excludes several desktop and optional presentation features, but `ambition_app` still contains code that assumes those features are enabled.

The currently observed failures include:

1. `menu/dispatch.rs` and `menu/grid_backend.rs` import shared menu state and helpers from `crate::menu::kaleidoscope_app`, even though that module only exists with the `kaleidoscope_menu` feature.

2. `app/plugins.rs` unconditionally references functions in `kaleidoscope_app`.

3. The desktop app builder in `app/cli.rs` is still compiled for WASM even though functions it calls, such as `desktop_asset_root` and `game_asset_source_builder`, are correctly excluded on WASM.

4. The desktop builder references `bevy::app::TerminalCtrlCHandlerPlugin`, which is unavailable for the WASM target.

5. `run_web` contains a `match render` block even though no `render` variable exists. This appears to have been copied from the native visible-app builder.

6. `app/shell_host.rs` unconditionally references `ambition::game_shell::ShellPauseMenuSuppressed`, but that type is exported only with the `basic_presentation` feature.

## Required repair strategy

Do not solve this by simply adding `kaleidoscope_menu`, `basic_shell_presentation`, or all default desktop features to `web`.

The minimal browser build is intentional. Repair the module and feature boundaries instead.

### 1. Separate shared menu logic from the Lunex backend

Inspect `game/ambition_app/src/menu/kaleidoscope_app.rs`, `dispatch.rs`, `grid_backend.rs`, `mod.rs`, and relevant plugin installation code.

Move backend-neutral menu concepts out of `kaleidoscope_app` into an always-compiled shared module. Likely candidates include items such as:

* cursor or selected-entry state,
* system-menu navigation state,
* shared menu parameters,
* shared navigation helpers,
* shared sound-effect helpers,
* installation functions needed by both flat Bevy UI and Kaleidoscope.

The exact names may differ; inspect their implementations and dependencies before deciding.

The shared module must not depend on Bevy Lunex, rich 3D text, or the Kaleidoscope rendering backend.

Keep actual Lunex/Kaleidoscope rendering and backend installation behind:

```rust
#[cfg(feature = "kaleidoscope_menu")]
```

Update imports and plugin installation so:

* `bevy_ui_menu` works without `kaleidoscope_menu`,
* `kaleidoscope_menu` still works when enabled,
* shared systems are not installed twice when both backends are available.

Do not retain misleading Kaleidoscope-specific names for genuinely shared concepts when a clear neutral rename is practical. Avoid a broad unrelated rename.

### 2. Gate desktop-only app construction at the definition

In `game/ambition_app/src/app/cli.rs`, locate the native visible-app builder and any associated desktop-only types.

A `cfg` on a re-export is insufficient. Put the appropriate target gate on the actual definitions and implementations that use:

* desktop filesystem asset readers,
* desktop asset-root discovery,
* terminal Ctrl-C handling,
* other APIs unavailable on `wasm32`.

Prefer a clear desktop-only function or module boundary, for example:

```rust
#[cfg(not(target_arch = "wasm32"))]
```

Do not duplicate the complete native builder merely to satisfy the compiler.

### 3. Repair `run_web`

Remove the invalid dependency on an undefined `render` variable.

The web entry point should install the browser-appropriate visible simulation, LDtk, and presentation plugins directly, unless repository architecture indicates a better existing shared composition function.

Do not introduce terminal, filesystem, process-exit, or native-window assumptions into `run_web`.

Check both embedded-assets and served-assets web modes.

### 4. Repair the shell presentation boundary

Inspect `game/ambition_app/src/app/shell_host.rs` and `crates/ambition_game_shell`.

Code that names `ShellPauseMenuSuppressed` must compile only when the feature that exports that type is enabled, unless the resource is actually core shell state and can be safely moved into the always-compiled shell API.

Choose the smallest architecturally correct repair.

Do not enable `basic_shell_presentation` merely to make the type exist in the web persona.

Ensure systems that require the resource are not registered when the resource or its plugin is absent.

### 5. Clean related cfg warnings

Fix the observed WASM-only unused import involving `ambition::render::ui_fonts` by placing the import under the same feature or target conditions as its uses.

Look for immediately adjacent imports or functions with the same class of incorrect cfg boundary, but do not turn this task into a repository-wide cleanup.

## Regression protection

Add automated checks for the exact negative feature combinations. An `--all-features` build is not sufficient because it masks these failures.

Add or update an appropriate repository script and CI workflow so the following are checked:

```bash
cargo check \
    -p ambition_app \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features web
```

```bash
cargo check \
    -p ambition_app \
    --lib \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features web_served_assets
```

If the WASM target is missing in CI, install it explicitly with rustup.

Keep the CI addition focused and avoid downloading browser automation tools unless a runtime smoke test already exists and is inexpensive.

Where practical, add native feature checks that prove the menu separation:

```bash
cargo check -p ambition_app --lib --no-default-features --features bevy_ui_menu
```

Use the repository’s actual required base features if `bevy_ui_menu` alone is not intended to be a complete supported persona. Do not invent a public feature contract without inspecting `Cargo.toml` and existing scripts.

## Validation

Run the two exact WASM checks first.

Then run the repository’s relevant native checks and tests, including the ordinary desktop feature set. At minimum, validate:

```bash
cargo check -p ambition_app
```

Run formatting:

```bash
cargo fmt --all --check
```

Run Clippy or the repository’s standard lint command if documented and reasonably scoped.

If a command cannot run because of an environment or dependency limitation, report the exact command and error. Do not claim it passed.

## Constraints

* Preserve the intent of a minimal web feature set.
* Do not hide the problem by enabling all features.
* Do not make Lunex mandatory for browser builds.
* Do not remove the Kaleidoscope backend.
* Do not weaken native functionality.
* Do not use broad `allow` attributes to suppress real cfg mistakes.
* Do not add dummy WASM implementations that panic unless the function is provably unreachable and the repository already uses that pattern.
* Keep command execution and build-script conventions consistent with the repository.
* Avoid unrelated formatting or refactoring.
* Do not modify generated assets or vendored dependencies.
* Do not commit unless explicitly instructed.

## Final report

At completion, report:

1. The root causes found.
2. The files changed.
3. How shared menu code was separated from the optional Lunex backend.
4. How desktop-only code was excluded from WASM compilation.
5. How the shell presentation feature boundary was repaired.
6. Every validation command run and whether it passed.
7. Any remaining WASM compiler or runtime concern, especially anything involving Lunex, system fonts, audio, or asset loading that appears only after `ambition_app` compiles.

Do not stop after fixing only the first compiler error. Iterate until both exact WASM commands pass or until you encounter a concrete external blocker that cannot be repaired in this repository.
