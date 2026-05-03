# Procedural 2D Character Lab

Pure Python / Pillow procedural side-scroller sprite generation for Ambition prototypes.

The package currently includes two right-facing targets:

- **robot**: cute side-scroller robot with a rigid, local-layer 2.5D-inspired head.
- **goblin**: opaque green side-view goblin with foreground/background arm ordering.

## Art / animation notes

- `blink` means the Ambition teleport / precision-blink ability, not an eyelid blink.
- Incidental eyelid blinks are still allowed in idle acting.
- Right-facing z-order convention: far arm behind torso, near arm/weapon in front.
- Heads are rendered as rigid grouped layers so face parts do not shear apart during tilt/death poses.
- Death rows keep fixed canvas, stable ground anchors, and consistent visible mass instead of per-frame bbox normalization.

## Default commands

From this directory:

```bash
python draw_all_character_spritesheets.py
python draw_character_canonicals.py
python -m proc2d_character_lab.cli draw-all
python -m proc2d_character_lab.cli draw-canonicals
```

The default package reads YAML jobs from `./proc2d_character_lab/configs` and writes generated PNG/YAML assets to `./assets`.

## Expected outputs

```text
assets/robot_spritesheet.png
assets/robot_spritesheet.yaml
assets/goblin_spritesheet.png
assets/goblin_spritesheet.yaml
assets/canonicals/robot_canonical.png
assets/canonicals/goblin_canonical.png
assets/canonicals/canonicals_contact_sheet.png
```

## Console output

`draw_character_canonicals.py` and `python -m proc2d_character_lab.cli draw-canonicals` print generated paths and, when Rich is installed, finish with a clickable `file://` link to `assets/canonicals/canonicals_contact_sheet.png`.

Render non-character gameplay entity sprites:

```bash
python draw_game_entity_sprites.py
python -m proc2d_character_lab.cli draw-entities
```

This writes individual static/state sprites to `./assets/entities`, plus `entity_manifest.yaml` and `entity_contact_sheet.png`. These are intended for chests, pickups, hazards, breakables, NPC terminals, boss placeholders, room fixtures, and moving-platform style objects that should not share the character animation schema.


Additional default character target:
- `proc2d_character_lab/configs/boss.yaml` -> `assets/boss_spritesheet.png` (AI Slop Zeta boss-specific attack sheet)
