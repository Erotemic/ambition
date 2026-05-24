#!/usr/bin/env python3
"""One-shot codegen: synthesize missing entries in
`crates/ambition_sandbox/assets/data/character_catalog.ron` from the
renderer's `list-targets` output.

Phase 3 of the character-catalog refactor (see
`TODO-character-catalog-and-hall.md`). Goal: every renderer-registered
character has a catalog entry so the catalog is the authoritative
spawnable-character list.

The renderer recognizes two flavors of targets:
  - `[characters]` — Python tackon targets in
    `tools/ambition_sprite2d_renderer/.../targets/characters/`.
  - `[review_npcs]` — YAML-adapter rigs + Python tackons used as
    review-only NPCs.

Both produce `<name>_spritesheet.png` + `<name>_spritesheet.ron` under
`assets/sprites/` when installed.

This script:
  1. Loads the renderer's `list-targets` output.
  2. Reads the current `character_catalog.ron` to find which
     character_ids already exist.
  3. Synthesizes a catalog entry for every missing target, choosing
     brain/action_set/tier/body_kind via a small heuristic table.
  4. Writes the missing entries out as a fresh RON snippet ready to
     splice into the catalog file. (Manual splice keeps the diff
     reviewable rather than auto-rewriting a hand-curated file.)

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.codegen_character_catalog \\
    > /tmp/missing_catalog_entries.ron
```

The output is a `// === auto-generated additions ===` block —
copy/paste into `character_catalog.ron` before the closing `},)` of
the `characters` map.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
RENDERER_DIR = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"
CATALOG_PATH = REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "data" / "character_catalog.ron"


# Heuristics — character_id prefix → (default_brain, default_action_set,
# tier, body_kind, base tags). The runtime classifies via tags rather
# than character_id substring; these heuristics are only for the
# initial generation. Authors retune individual entries afterward.
ENEMY_BRAIN = "melee_brute_striker"
ENEMY_ACTION = "striker_swipe"
BRUTE_BRAIN = "melee_brute_brute"
BRUTE_ACTION = "brute_lunge"
RANGER_BRAIN = "skirmisher_ranger"
RANGER_ACTION = "ranger_arrow"
PEACEFUL_BRAIN = "patrol_peaceful"
PEACEFUL_ACTION = "peaceful"
STAND_STILL = "stand_still"

# Explicit category for each renderer target. Drives the heuristic.
# Catches things the simple substring rules miss.
CATEGORY: dict[str, str] = {
    # ===== Bosses (Basement tier).
    "boss":                            "boss",  # legacy archetype
    "dark_lord":                       "boss",
    "flying_spaghetti_monster_boss":   "boss",
    "gnu_ton_boss":                    "boss",
    "mockingbird_boss":                "boss",
    "smart_house":                     "boss",
    "trex_enemy":                      "boss",   # large; basement
    "bear_mauler":                     "boss",   # large; basement
    "raptor_stalker":                  "basement_enemy",
    "mantis_lancer":                   "basement_enemy",
    # ===== Robots (enemy variants).
    "robot_guardian":                  "enemy_brute",
    "robot_heavy":                     "enemy_brute",
    "robot_runner":                    "enemy_swipe",
    # ===== Goblin variants.
    "goblin_forest_spear":             "enemy_ranger",
    "goblin_brute_hammer":             "enemy_brute",
    "goblin_cave_dagger":              "enemy_swipe",
    "goblin_desert_bow":               "enemy_ranger",
    "goblin_frost_sword":              "enemy_swipe",
    "goblin_shaman_staff":             "enemy_ranger",
    # ===== Pirates / vikings / ninjas.
    "pirate_cutlass_viper":            "enemy_swipe",
    "pirate_heavy":                    "enemy_brute",
    "viking_warrior":                  "enemy_swipe",
    "viking_shieldmaiden":             "enemy_swipe",
    "viking_heavy_warrior":            "enemy_brute",
    "viking_heavy_shieldmaiden":       "enemy_brute",
    "ninja_heavy":                     "enemy_brute",
    # ===== AI-era enemies (recently added sprite generators).
    "agent_swarm":                     "enemy_swipe",
    "ai_slop":                         "enemy_swipe",
    "spaghetti_event":                 "enemy_swipe",
    "synthetic_friend":                "enemy_swipe",
    "helpful_liar":                    "enemy_ranger",
    "hand_saint":                      "enemy_brute",
    "puppy_slug_variant2":             "wanderer",
    # ===== Hostile humans / wildlife.
    "fascist_enforcer":                "enemy_swipe",
    "ghoul_skulker":                   "enemy_swipe",
    "weird_hermit":                    "peaceful",   # dialogue-flavored
    "galwah":                          "peaceful",
    "girdle":                          "peaceful",
    # ===== Hub / story stand-ins.
    "colonial_statesman":              "peaceful",
    "president_portrait":              "stand_still",  # mounted portrait
    # ===== Player variants (catalog covers them so the
    #       coverage gate test holds; runtime uses Brain::Player).
    "player_robot":                    "player",
    "player_extended":                 "player",
    "player_combat_review":            "player",
    "player_social_review":            "player",
    "player_traversal_review":         "player",
    # ===== Sandbag variants.
    "sandbag_armored_review":          "training",
    "sandbag_full_review":             "training",
    # ===== Review NPCs (hub-style dialogue-only).
    "craig":                           "peaceful",
    "eve":                             "peaceful",
    "judy":                            "peaceful",
    "mallory":                         "peaceful",
    "olivia":                          "peaceful",
    "peggy":                           "peaceful",
    "sybil":                           "peaceful",
    "trent":                           "peaceful",
    "trudy":                           "peaceful",
    "victor":                          "peaceful",
    "walter":                          "peaceful",
    "general_hero":                    "peaceful",
    # ===== Robot specialty roles.
    "robot_archivist":                 "peaceful",
    "robot_caster":                    "peaceful",
    "robot_diver":                     "peaceful",
    "robot_engineer":                  "peaceful",
    "robot_medic":                     "peaceful",
    "robot_miner":                     "peaceful",
}

CATEGORY_TEMPLATE: dict[str, dict] = {
    "boss":             dict(brain=STAND_STILL, action=PEACEFUL_ACTION, tier="Basement", body="Wide",       tags=["boss", "placeholder_brain"]),
    "basement_enemy":   dict(brain=ENEMY_BRAIN, action=ENEMY_ACTION,    tier="Basement", body="Wide",       tags=["enemy", "large"]),
    "enemy_swipe":      dict(brain=ENEMY_BRAIN, action=ENEMY_ACTION,    tier="MainHall", body="Standard",   tags=["enemy"]),
    "enemy_brute":      dict(brain=BRUTE_BRAIN, action=BRUTE_ACTION,    tier="MainHall", body="Wide",       tags=["enemy", "heavy"]),
    "enemy_ranger":     dict(brain=RANGER_BRAIN, action=RANGER_ACTION,  tier="MainHall", body="Standard",   tags=["enemy", "ranged"]),
    "wanderer":         dict(brain="wanderer_puppy_slug", action="peaceful_slither", tier="MainHall", body="Crawler", tags=["enemy", "wanderer"]),
    "peaceful":         dict(brain=PEACEFUL_BRAIN, action=PEACEFUL_ACTION, tier="MainHall", body="Standard", tags=["hub", "peaceful"]),
    "stand_still":      dict(brain=STAND_STILL, action=PEACEFUL_ACTION, tier="MainHall", body="Standard",   tags=["hub", "static"]),
    "player":           dict(brain=STAND_STILL, action=PEACEFUL_ACTION, tier="MainHall", body="Standard",   tags=["player", "variant"]),
    "training":         dict(brain=STAND_STILL, action="sandbag_punch", tier="MainHall", body="Standard",   tags=["training"]),
}


def display_name_of(target: str) -> str:
    # camelize from snake_case: "robot_guardian" -> "Robot Guardian".
    return " ".join(word.capitalize() for word in target.split("_"))


def character_id_for(target: str) -> str:
    """Map renderer target → catalog character_id.

    Targets that already match an existing catalog id (no `npc_`
    prefix, like `goblin` / `sandbag` / `robot`) are kept as-is;
    everything else gets a `npc_` prefix for symmetry with the NPC
    register's existing convention. Bosses keep the trailing
    `_boss` so the id reads naturally.
    """
    if target in ("player_robot",):
        # player_robot is the dedicated player sheet; alias it to
        # `player`. The renderer keeps the longer name for clarity
        # in its target list.
        return "player"
    if target == "absurd_general":
        return "npc_general"
    if target in ("goblin", "robot", "sandbag"):
        # Already covered by base entries.
        return target
    if target == "puppy_slug":
        # Already covered by `npc_puppy_slug`.
        return "npc_puppy_slug"
    return f"npc_{target}"


def read_renderer_targets() -> tuple[list[str], list[str]]:
    """Return (characters, review_npcs) lists."""
    result = subprocess.run(
        [sys.executable, "-m", "ambition_sprite2d_renderer", "list-targets"],
        cwd=str(RENDERER_DIR),
        env={"PYTHONPATH": ".", "PATH": "/usr/bin:/bin"},
        capture_output=True,
        text=True,
        check=True,
    )
    chars: list[str] = []
    reviews: list[str] = []
    section: str | None = None
    for line in result.stdout.splitlines():
        if line.startswith("  [characters]"):
            section = "characters"
            continue
        if line.startswith("  [props]") or line.startswith("  [tiles]") or line.startswith("  [icons]"):
            section = None
            continue
        if line.startswith("  [review_npcs]"):
            section = "review"
            continue
        if section and line.startswith("    "):
            tok = line.strip().split()[0]
            (chars if section == "characters" else reviews).append(tok)
    return chars, reviews


def read_existing_ids(catalog_path: Path) -> set[str]:
    """Quick-and-dirty scan of the RON file for top-level character_id
    keys. The catalog uses `"id": (` lines under the `characters:`
    map, which is good enough to parse without a full RON parser."""
    text = catalog_path.read_text()
    # Find the `characters: {` block.
    start = text.index("characters: {")
    # Naive: every `    "name":` indented line under that block.
    ids: set[str] = set()
    for raw in text[start:].splitlines():
        line = raw.strip()
        if not line.startswith('"'):
            continue
        end = line.find('"', 1)
        if end <= 0:
            continue
        ident = line[1:end]
        if not ident:
            continue
        ids.add(ident)
    return ids


def render_entry(character_id: str, target: str, category: str) -> str:
    tpl = CATEGORY_TEMPLATE[category]
    display = display_name_of(target)
    sprite = f"sprites/{target}_spritesheet.png"
    manifest = f"sprites/{target}_spritesheet.ron"
    tags = tpl["tags"] + ([f"renderer:{target}"] if target != character_id.removeprefix("npc_") else [])
    tags_ron = ", ".join(f'"{t}"' for t in tags)
    return (
        f'        "{character_id}": (\n'
        f'            display_name: "{display}",\n'
        f'            spritesheet: "{sprite}",\n'
        f'            manifest: "{manifest}",\n'
        f'            tier: {tpl["tier"]},\n'
        f'            body_kind: {tpl["body"]},\n'
        f'            composition: None,\n'
        f'            default_brain: "{tpl["brain"]}",\n'
        f'            default_action_set: "{tpl["action"]}",\n'
        f'            tags: [{tags_ron}],\n'
        f'        ),\n'
    )


def main() -> int:
    characters, reviews = read_renderer_targets()
    existing = read_existing_ids(CATALOG_PATH)

    chunks: list[str] = []
    missing_no_category: list[str] = []
    seen_ids: set[str] = set()

    for target in sorted(set(characters) | set(reviews)):
        character_id = character_id_for(target)
        if character_id in existing or character_id in seen_ids:
            continue
        category = CATEGORY.get(target)
        if category is None:
            missing_no_category.append(target)
            continue
        chunks.append(render_entry(character_id, target, category))
        seen_ids.add(character_id)

    if missing_no_category:
        print("// [warning] no category mapping — review and add to CATEGORY:", file=sys.stderr)
        for t in missing_no_category:
            print(f"//   {t}", file=sys.stderr)

    print("        // === auto-generated entries (Phase 3 codegen) ===")
    sys.stdout.write("".join(chunks))
    print(f"\n// total auto-generated: {len(chunks)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
