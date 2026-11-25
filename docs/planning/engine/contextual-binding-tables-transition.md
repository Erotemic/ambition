# Contextual binding tables and input opinions

## Intent

One authoritative contextual button table per participant. Read the default rows, game changes, and saved overrides to determine the effective binding. The engine resolves declarations; games own choices. Use ordinary Rust data, explicit imports, and small constructors. No custom macros or game-name enum in the engine.

## Implementation map

- `opinions/ambition_input_opinions/src/defaults.rs`: keyboard and pad defaults, including explicit per-preset menu rows. The app chooses to install this catalog. A new game can install a different `BindingDefaults` instead.
- `crates/ambition_input/src/table.rs`: binding row, scheme, runtime table, device filtering and precedence. An explicit `unbind` removes a row. Game `BindingLayout` entries replace individual profile/control rows. The existing persisted action/device override moves the action in its declared profiles; same-context collisions replace, cross-context reuse is allowed. Changing the catalog, declared game layout or participant recipe rebuilds the map.
- `game/ambition_demo_smash/src/input_bindings.rs`: game-owned gameplay overrides; UI defaults are untouched.
- `crates/ambition_input/src/contextual.rs`: physical press history and direct profile lookup. No inference from Start+Confirm or Start+Back.
- `crates/ambition_platformer2d_runtime/src/input.rs`: participant acquisition and publication; Leafwing still acquires raw controls and supplies gameplay state. The table is the menu button routing authority, not a replacement for the simulation control-frame pipeline.
- `crates/ambition_touch_input/src/virtual_device.rs`: explicit touch rows installed into the same effective table. The touch adapter still owns these default positions and axis adapters.
- `game/ambition_app/src/app/binding_census.rs`: prints effective resolved rows. It does not implement a second button resolver. Its list of game compositions is still explicit.

## Validation before committing

This overlay is statically authored against d49467d5b877 plus the three monolith renames. It has not been compiled in the authoring environment. Validate real feature combinations, not just the default workspace build. Follow AGENTS.md; do not create worktrees. Make necessary adjustments directly; do not reintroduce implicit action-combination routing as a compatibility fix.

1. `cargo fmt --all -- --check` (format changed Rust as needed; avoid unrelated formatting changes). `cargo check -p ambition_input --features input`; `cargo test -p ambition_input --features input --lib`; `cargo test -p ambition_input_opinions`.
2. `cargo test -p ambition_platformer2d_runtime --lib input::`; `cargo test -p ambition_touch_input --features mobile_touch`; `cargo test -p ambition_sim_view --features input` (use the repository's actual supported feature bundles).
3. Build the main app and standalone Mary-O, Sanic, Smash and TwinTrack hosts using their existing supported desktop/mobile/headless profiles. Verify the minimal-game/external-consumer fixtures can install their own catalog; do not make engine production code depend on opinions. Check Cargo.lock and any independent fixture lockfiles.
4. Run the gate-network and menu real-input tests on both menu backends. Enter confirms the gate; Escape pauses it; Backspace backs out; gamepad Start pauses, A confirms and B backs out; Select/Touch Reset must not pause in gameplay. Verify held keys do not bleed into a new context. Test per-seat device filtering, game transitions, preset swaps, user remaps and replacing a live catalog.
5. Run `cargo run -p ambition_app --bin ambition_game_bin -- --explain-bindings` with the usual game features. Compare against actual input behavior. In particular verify keyboard/pad movement, page shortcuts, party return, and every default in all four presets.

## Remaining boundaries (do not mistake for completed work)

- This migration covers contextual buttons. Analog axes and chords retain adapters; the persisted remap schema is still action/device-wide, not a profile-specific remap editor. Axis adapter dependencies (notably BurstAnalog) need review for independent axis customization.
- The touch defaults still live in the adapter. A follow-up can move authored touch policy into opinions without coupling the engine to mobile UI, and add dynamic virtual-button layout changes. The current touch UI position set remains fixed.
- Some raw ActionState-only fixtures and direct InputMap consumers remain. Audit those consumers for context bypasses; fixture injection should provide an explicit table rather than rely on union projection. UI glyphs have profile projections, but remaining callers of the union projection must be checked for context-sensitive labels.
- Engine unit tests reuse the real opinion declarations through a test-only source include; a few other engine test crates have a dev-dependency on opinions. Move policy-specific tests into opinions or replace them with neutral fixtures before repository extraction. Production engine crates do not depend on opinions.
- The four preset identifiers and metadata shapes are still engine types. Do not add new preset names to the engine merely to support another game: generalize the identifier when there is a concrete customer.
- Development reset gating, network pause, and independent island suspension are outside this migration. Preserve their current policy and finish separately.

Prefer deleting obsolete inference and duplicated policy over adding another layer. The desired result is that editing one row changes the corresponding input, and a static census describes what the runtime will do.
