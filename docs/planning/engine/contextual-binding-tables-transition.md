# Contextual binding tables and input opinions

## Architecture

The engine owns input acquisition, contextual resolution, participant routing and remapping. Shared conventions live in `opinions/ambition_input_opinions/src/defaults.rs`; games choose a catalog and can supply their own button and axis overrides. A game can replace the catalog altogether. Ordinary Rust constants and constructors declare defaults; no binding DSL or custom declaration macros are required.

A participant's binding recipe composes the catalog, game overrides, and persisted player overrides into an effective `BindingTable` and Leafwing acquisition map. Device availability and seat ownership constrain acquisition. Rebuilding the recipe updates the table without recompiling the game. Context changes route the appropriate rows; physical press history prevents a held button from activating a newly opened interface. Explicit unbind rows remove inherited bindings. Same-context conflicts replace a binding, while physical reuse across contexts is intentional.

## Source map

- `opinions/ambition_input_opinions/src/defaults.rs`: keyboard, controller, touchscreen, and axis defaults.
- `game/ambition_demo_smash/src/input_bindings.rs`: game-owned gameplay overrides.
- `crates/ambition_input/src/table.rs`: effective button and axis declarations, composition, contextual projections, direction-edge generation.
- `crates/ambition_input/src/axis.rs`: physical axis sources and acquisition adapters. Digital movement derives from the same resolved directional button rows as press edges. Analog movement generates threshold edges for the selected physical stick. A paired burst trigger is removed when its owning button binding is replaced; a separately specified axis can be remapped independently.
- `crates/ambition_input/src/contextual.rs`: contextual button routing and physical press history. No action-combination inference.
- `crates/ambition_input/src/bindings.rs`, `layout.rs`, `settings.rs`: participant recipes, dynamic layout/catalog rebuilds and persisted button/axis overrides.
- `crates/ambition_input/src/touch_state.rs`, `touch_device.rs`: raw virtual touch controls, independent of touch presentation or game policy.
- `crates/ambition_touch_input/src/bevy_plugin.rs`: touch presentation, hit testing, and gesture acquisition; uses the current primary participant table and context rather than a parallel default action assignment.
- `crates/ambition_platformer2d_runtime/src/input.rs`: routed control frames and published effective binding information. Recipe-backed participants do not fall back to raw menu actions when their table is absent. Raw-map-only injection remains available for explicit fixtures and existing integrations.
- `game/ambition_app/src/app/binding_census.rs`: enumerates effective button and axis rows; the game compositions it inspects remain explicitly listed.
- `crates/ambition_input/tests/support/catalog.rs`: fixed engine test data, deliberately independent of the production opinions crate. Engine production and tests need no Cargo dependency on opinions.

## Validation

This cumulative implementation was authored against d49467d5b877 plus the Monolith renames. Static checks are not a substitute for a real Rust build. Follow AGENTS.md and avoid worktrees. Run targeted validation, formatting only affected Rust files if necessary.

1. `cargo check -p ambition_input --features input`; `cargo test -p ambition_input --features input --lib`; `cargo test -p ambition_input_opinions`.
2. Run input runtime, touch-input mobile feature, sim-view and standalone-host tests with their supported feature bundles. Test minimal-game/external-consumer fixtures with independently supplied defaults.
3. Run the gate-terminal real-input tests with both menu backends. Verify Enter confirms, Escape pauses the gate, Backspace backs out, controller Start pauses/resumes, A confirms, B backs out, Select/Touch Reset does not pause gameplay, and a held button never activates a newly opened menu.
4. Test keyboard direction remaps against actual movement and direction edges; reassignment of the movement stick; burst button/trigger pairing; independent trigger remapping; touch overrides; per-seat filtering; game transitions; live catalog replacement and remaps. The input table tests include targeted cases for these relationships.
5. Run `cargo run -p ambition_app --bin ambition_game_bin -- --explain-bindings` under the usual app features. Compare effective rows with prompts, binding settings, and observed behavior. Validate physical controllers, Android touch, and Steam Deck hardware separately.

## Deliberate scope boundaries

- User overrides retain action/device scope in the existing settings model; game authors can declare context-specific rows. A profile-specific player remapping editor is not included.
- Touch physical positions remain fixed; the effective table determines their actions, availability, and presentation. Touch overrides can be declared and persisted; an interactive touch layout/rebinding editor is not included.
- The engine still has the existing built-in action vocabulary and four preset identifiers. Generalizing them is a separate decision, not necessary for binding ownership.
- Explicit raw-map integrations, semantic-provider inputs and non-binding UI pointer/gesture handling remain. Device acquisition and game simulation are not being replaced.
- Development reset authorization, unanimous pause synchronization and independent island execution are separate from the binding migration.

Prefer direct declarations and one binding authority over inference, redundant policy or compatibility layers. The goal is that a developer edits a row, reads the effective table, and understands the resulting control behavior.
