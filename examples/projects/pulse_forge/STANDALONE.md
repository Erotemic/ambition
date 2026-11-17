# Run and build outside the SDK checkout

The runtime reads the game's files. Rust builds use an explicit SDK source
checkout for the public crates. The final runtime package needs no SDK source.
Use the same SDK revision for the prebuilt tools and Rust dependencies.
Start with a prepared SDK checkout and its generated content, as described in
the [repository setup](../../../README.md). The exporter needs the selected
shared library's runtime files on disk.

Build these SDK artifacts once. The optimized build needs
`CARGO_INCREMENTAL=0`, as the repository's release policy requires.

```bash
CARGO_INCREMENTAL=0 cargo build --release -p ambition_app --no-default-features --features desktop_player --bin ambition_game_bin
cargo build -p ambition_app_tools --features ambition_app/static_content --bin project_export --bin extension_inspect
```

Set the paths from the SDK checkout. The source-kit helper needs the Python
TOML writer (`python3 -m pip install toml`). It copies the authored runtime
files through `project_export`, then copies the example's Rust sources and
writes a separate Cargo workspace. It copies the SDK's dependency patches;
Cargo does not inherit them across workspaces. It does not modify the SDK.

```bash
SDK_ROOT="$PWD"
GAME_ROOT=/tmp/pulse-forge-authoring
python3 examples/projects/pulse_forge/make_standalone.py --sdk "$SDK_ROOT" --out "$GAME_ROOT"
cd "$GAME_ROOT"
PATH=/nonexistent "$SDK_ROOT/target/debug/extension_inspect" --project . --ticks 30
PATH=/nonexistent "$SDK_ROOT/target/release/ambition_game_bin" --project . --headless-ticks 60
```

The initial manifest selects Rhai. Edit the scripts in `modules/000` or the
catalogs in `content`. The same executable loads these changes. A script can
declare a new module and state schema through `declare()`. The engine still
owns physics, costs, damage, state storage, rollback, and publication.

Compile this directory's portable Rust crate to WASM. The existing builder
accepts its Cargo manifest and writes to the SDK's checked target storage.
Only the module and its public value dependencies compile.

```bash
"$SDK_ROOT/scripts/regen/build_extension_modules.sh" --manifest-path "$GAME_ROOT/Cargo.toml" pulse_forge_modules
cp "$SDK_ROOT/target/extension-modules/wasm32-unknown-unknown/release/pulse_forge_modules.wasm" modules/pulse_forge.wasm
PATH=/nonexistent "$SDK_ROOT/target/debug/extension_inspect" --project . --compare modules/pulse_forge.wasm --ticks 0
```

In `project.ron`, change the emitter selection from
`File("modules/000/pulse_forge.rhai")` to `File("modules/pulse_forge.wasm")`.
Restart the same engine executable. No engine compilation occurs in this step.
The sentinel remains Rhai. Its world, character rows, and control binding stay
the same.

```bash
PATH=/nonexistent "$SDK_ROOT/target/release/ambition_game_bin" --project . --headless-ticks 60
```

For linked Rust, change that selection to `Linked`. Build this directory's host
with the same portable crate. This step compiles and links an executable. The
SDK config and target directory below reuse the assigned SDK build storage.

```bash
CARGO_INCREMENTAL=0 cargo --config "$SDK_ROOT/.cargo/config.toml" build --manifest-path "$GAME_ROOT/Cargo.toml" --target-dir "$SDK_ROOT/target" --release -p pulse_forge_host --no-default-features --features desktop_player
PATH=/nonexistent "$SDK_ROOT/target/release/pulse_forge_host" --project . --headless-ticks 60 --compare modules/pulse_forge.wasm
```

Package that host and this directory's selected runtime files. Rust source and
Cargo's SDK paths do not enter the runtime package. Run its `run.sh` without a
probe flag for a desktop window.

```bash
python3 "$SDK_ROOT/scripts/package/package_project.py" --project "$GAME_ROOT" --host "$SDK_ROOT/target/release/pulse_forge_host" --exporter "$SDK_ROOT/target/debug/project_export" --out /tmp/pulse-forge-release --ticks 30 --presentation-frames 600
cd /
/tmp/pulse-forge-release/run.sh --headless-ticks 30
```

The package check verifies bytes, project identity, selected module runners,
and file reads from outside the checkout. The no-window probe also loads
presentation assets; it does not render pixels. Browser and Android project
loaders are not established by this desktop workflow. Their native artifact
identity is also unavailable. Cross-target SDK compilation alone proves
neither loader.
