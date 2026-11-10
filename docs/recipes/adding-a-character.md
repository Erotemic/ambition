---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/concepts/content-and-provider-boundaries.md
---

# Add a character

A character's reusable identity, body, abilities, actions, presentation, and
behavior are resolved into one `PreparedCharacterDefinition`. Placement,
disposition, encounter role, and control assignment remain contextual. Do not
create a player/enemy-specific body-construction branch for a new character.

## 1. Find the provider's character sources

```bash
python scripts/agent_query.py "CharacterDefinition character catalog prepared character"
python scripts/agent_query.py tests "try_register_character PreparedCharacterDefinition"
```

Ambition's `game/ambition_content/assets/data/character_catalog.ron` contains
provider cast rows, including presentation, writing, and character composition
fragments. Code-authored characters register `CharacterDefinition` values.
Preparation folds the registered definitions and provider fragments into the
single runtime value. Read a nearby character that uses the features you need.

For a game that defines a character entirely in its catalog, a row specifying
`locomotion` or `locomotion_preset` participates in character construction. A
host-code character can register intrinsic facts separately. Avoid duplicating
those same facts in an unrelated runtime lookup.

## 2. Author the intrinsic character

Choose existing typed fields or presets where possible:

- **Body:** shape, size, health, death behavior, hurt regions, locomotion, and
  physical tuning.
- **Actions:** moveset/action-set presets, abilities, ranged weapons, and
  contact behavior.
- **Autonomy:** an intrinsic profile or a shared named profile; a provoked
  profile where needed.
- **Presentation:** sheet/posed body, sizing, portrait, and other visual facts.
- **Writing:** bark pools, fallback dialogue, and character descriptions.

These are examples, not a parallel schema. Consult the actual catalog row and
`CharacterDefinition` API for supported fields. For Ambition's human characters,
`character_catalog.ron` documents the current standing-height convention; the
standard default body is intentionally shorter than most people in that cast.

A character's faction/hostility in one encounter, session seat, spawn point, and
controller ownership are supplied by placement or ruleset, not fixed into the
reusable character identity.

If the character needs genuinely new shared behavior, extend the corresponding
engine action/body vocabulary and make the authoring path express it.

## 3. Prepare the art

A sprite target with `ACTOR_METADATA` can seed authoring notes and the catalog
row:

```bash
PYTHONPATH=tools/ambition_ldtk_tools:tools/ambition_sprite2d_renderer \
  python3 -m ambition_ldtk_tools.character_notes --splice --target <target>
```

The authored sprite target owns its source and generation inputs. Publish the
runtime art through the renderer's target commands:

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer list
python -m ambition_sprite2d_renderer canonical <target>
python -m ambition_sprite2d_renderer sheet <target>
python -m ambition_sprite2d_renderer publish <target>
```

Treat generated notes as a draft. Keep generated binaries out of Git and do
not require every character to use the same rig/art generator.

## 4. Place the character

Use the LDtk editor or provider room-generation tools and reference its stable
character ID. Keep controller, disposition, and per-instance overrides on the
placement/ruleset. Do not hand-edit `.ldtk` JSON or modify generated world output
without changing its source.

## 5. Exercise the character

Prepare the provider and spawn the character through the actual game. Check its
body scale, actions, control/AI behavior, art, and transition/respawn behavior
as relevant to the change. Reuse a focused existing test for a significant
construction defect; avoid reproducing the entire construction pipeline in a
new test harness.

See [actors and character content](../systems/actors-brains-and-character-content.md)
for the ownership model and
[extending brains and action sets](extending-brains-and-action-sets.md) for new
reusable action capabilities.
