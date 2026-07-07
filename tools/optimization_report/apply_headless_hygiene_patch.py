#!/usr/bin/env python3
"""Apply the first optimization hygiene patch to large Rust files.

This script is intentionally idempotent. It edits the large source files in
place instead of shipping full replacements for app.rs/audio.rs, because those
files are high-churn and large enough that a full-file overlay is more brittle
than anchored edits.
"""

from __future__ import annotations

import argparse
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> tuple[str, bool]:
    if new in text:
        print(f"[headless-hygiene] already applied: {label}")
        return text, False
    if old not in text:
        raise RuntimeError(f"anchor not found for {label}")
    print(f"[headless-hygiene] applying: {label}")
    return text.replace(old, new, 1), True


def remove_once(text: str, old: str, label: str) -> tuple[str, bool]:
    if old not in text:
        print(f"[headless-hygiene] already removed or not present: {label}")
        return text, False
    print(f"[headless-hygiene] removing: {label}")
    return text.replace(old, "", 1), True


def remove_line_stripped(text: str, stripped_line: str, label: str) -> tuple[str, bool]:
    lines = text.splitlines(keepends=True)
    removed = False
    kept = []
    for line in lines:
        if not removed and line.strip() == stripped_line:
            print(f"[headless-hygiene] removing: {label}")
            removed = True
            continue
        kept.append(line)
    if not removed:
        print(f"[headless-hygiene] already removed or not present: {label}")
        return text, False
    return "".join(kept), True


def patch_audio(repo: Path) -> bool:
    path = repo / "crates" / "ambition_actors" / "src" / "audio.rs"
    text = path.read_text(encoding="utf-8")
    old = """use ambition_engine as ae;\n#[cfg(feature = \"audio\")]\nuse ambition_sfx::{self as sfx, SfxId, SfxProvider};\n"""
    new = """use ambition_engine as ae;\nuse ambition_sfx::SfxId;\n#[cfg(feature = \"audio\")]\nuse ambition_sfx::{self as sfx, SfxProvider};\n"""
    text2, changed = replace_once(
        text,
        old,
        new,
        "make SfxId available when the audio backend feature is disabled",
    )
    if changed:
        path.write_text(text2, encoding="utf-8")
    return changed


def patch_app(repo: Path) -> bool:
    path = repo / "crates" / "ambition_actors" / "src" / "app.rs"
    text = path.read_text(encoding="utf-8")
    changed_any = False

    text, changed = remove_line_stripped(
        text,
        "crate::map_menu::handle_map_menu_hotkeys,",
        "remove input-gated map hotkeys from the unconditional presentation tuple",
    )
    changed_any |= changed

    map_dismiss_chain = """        // Mouse / touch dismissal for the map menu — separate from the\n        // big presentation tuple to keep that under Bevy's 16-system\n        // tuple budget.\n        .add_systems(Update, crate::map_menu::map_menu_pointer_dismiss)\n"""
    text, changed = remove_once(
        text,
        map_dismiss_chain,
        "remove input-gated map pointer dismissal from the unconditional chain",
    )
    changed_any |= changed

    insert_anchor = """        .add_systems(Update, vfx_spawn_messages.after(sandbox_update));\n    // Live blink-destination preview ring. Reads leafwing action state to\n"""
    insert_block = """        .add_systems(Update, vfx_spawn_messages.after(sandbox_update));\n\n    #[cfg(feature = \"input\")]\n    {\n        // Input-dependent map menu systems must stay behind the input\n        // feature. They are compiled even when add_presentation_plugins\n        // is not called, so unconditional references break headless/RL\n        // no-default-feature builds.\n        app.add_systems(Update, crate::map_menu::handle_map_menu_hotkeys.after(sandbox_update));\n        app.add_systems(Update, crate::map_menu::map_menu_pointer_dismiss);\n    }\n\n    // Live blink-destination preview ring. Reads leafwing action state to\n"""
    text, changed = replace_once(
        text,
        insert_anchor,
        insert_block,
        'register input-gated map menu systems behind cfg(feature = "input")',
    )
    changed_any |= changed

    signature_old = """    asset_server: Res<AssetServer>,\n    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,\n    asset_config: Res<GameAssetConfig>,\n    scene_entities: Res<SceneEntities>,\n) {\n"""
    signature_new = """    asset_server: Res<AssetServer>,\n    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,\n    asset_config: Res<GameAssetConfig>,\n    scene_entities: Res<SceneEntities>,\n    ui_fonts: Option<Res<ui_fonts::UiFonts>>,\n) {\n"""
    text, changed = replace_once(
        text,
        signature_old,
        signature_new,
        "give the no-audio presentation setup the same optional UiFonts param",
    )
    changed_any |= changed

    setup_old = """    setup::presentation_world(\n        &mut commands,\n        setup::PresentationSetup {\n            world: &world,\n            room_set: &room_set,\n            sandbox_data: &sandbox_data,\n            physics_settings: *physics_settings,\n            game_assets: &game_assets,\n        },\n        scene_entities.player,\n    );\n"""
    setup_new = """    setup::presentation_world(\n        &mut commands,\n        setup::PresentationSetup {\n            world: &world,\n            room_set: &room_set,\n            sandbox_data: &sandbox_data,\n            physics_settings: *physics_settings,\n            game_assets: &game_assets,\n            ui_fonts: ui_fonts.as_deref(),\n        },\n        scene_entities.player,\n    );\n"""
    text, changed = replace_once(
        text,
        setup_old,
        setup_new,
        "pass UiFonts through the no-audio PresentationSetup initializer",
    )
    changed_any |= changed

    if changed_any:
        path.write_text(text, encoding="utf-8")
    return changed_any


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "repo", nargs="?", default=".", type=Path, help="repository root"
    )
    args = parser.parse_args()
    repo = args.repo.resolve()
    if not (repo / "Cargo.toml").exists():
        raise SystemExit(f"error: {repo} does not look like a repo root")

    changed = False
    changed |= patch_audio(repo)
    changed |= patch_app(repo)
    if changed:
        print("[headless-hygiene] source files updated")
    else:
        print("[headless-hygiene] no source changes needed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
