# Hall of Characters and character catalog

Status: completed historical planning note. The old hand-maintained sprite registry plan has landed and should no longer be followed as current instructions.

## Current truth

- Character data lives in `crates/ambition_gameplay_core/assets/data/character_catalog.ron`.
- Catalog implementation lives in `crates/ambition_characters/src/actor/character_catalog/`.
- Sandbox integration / embedded lookup lives in `crates/ambition_gameplay_core/src/character_roster.rs`.
- Runtime sprite loading uses the catalog plus `crates/ambition_gameplay_core/src/character_sprites/` sheet registry/manifest code.
- The Hall of Characters room is generated from catalog data; do not recreate the old multi-table registry workflow.

## Current edit path

For a normal new character:

1. Add or publish sprite metadata under `crates/ambition_gameplay_core/assets/sprites/`.
2. Add a catalog row in `crates/ambition_gameplay_core/assets/data/character_catalog.ron`.
3. Add a new brain/action template only if the existing presets cannot express the behavior.
4. Regenerate or validate the Hall of Characters with the LDtk tools.
5. Run the catalog / LDtk validation tests named in `docs/recipes/adding-a-character.md`.

## References

- `docs/systems/character-catalog.md`
- `docs/recipes/adding-a-character.md`
- `docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`
