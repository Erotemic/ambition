from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).parents[1] / "package_asset_guard.py"
SPEC = importlib.util.spec_from_file_location("package_asset_guard", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
asset_guard = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = asset_guard
SPEC.loader.exec_module(asset_guard)


class PackageAssetGuardTests(unittest.TestCase):
    def make_repo(self, root: Path) -> tuple[Path, Path]:
        actors = root / "crates/ambition_actors/assets"
        content = root / "game/ambition_content/assets"
        actors.mkdir(parents=True)
        content.mkdir(parents=True)
        return actors, content

    def test_compose_and_zip_audit_cover_transitive_sheet_pages(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            actors, content = self.make_repo(repo)
            (actors / "sprites/hero").mkdir(parents=True)
            (actors / "sprites/hero/hero_spritesheet.png").write_bytes(b"page0")
            (actors / "sprites/hero/hero_spritesheet.1.png").write_bytes(b"page1")
            (actors / "sprites/hero/hero_spritesheet.ron").write_text(
                '[(target: "hero", image: "hero_spritesheet.png", images: ["hero_spritesheet.png", "hero_spritesheet.1.png"], label_width: 0, frame_width: 1, frame_height: 1, rows: [])]',
                encoding="utf8",
            )
            (content / "data").mkdir()
            (content / "data/character_catalog.ron").write_text(
                '(characters: {"hero": (spritesheet: "sprites/hero/hero_spritesheet.png", manifest: "sprites/hero/hero_spritesheet.ron")})',
                encoding="utf8",
            )

            contract, source_files = asset_guard.build_contract(repo, "android")
            self.assertIn("sprites/hero/hero_spritesheet.1.png", contract.entries)
            output = repo / "package"
            asset_guard.copy_composed_tree(source_files, output)
            asset_guard.audit_tree(contract, output, reject_extras=True)

            apk = repo / "test.apk"
            with zipfile.ZipFile(apk, "w") as zfile:
                for rel, path in asset_guard.walk_package_tree(output).items():
                    zfile.write(path, f"assets/{rel}")
            asset_guard.audit_zip(contract, apk, "assets/")

            broken = repo / "broken.apk"
            with zipfile.ZipFile(broken, "w") as zfile:
                for rel, path in asset_guard.walk_package_tree(output).items():
                    if rel != "sprites/hero/hero_spritesheet.1.png":
                        zfile.write(path, f"assets/{rel}")
            with self.assertRaisesRegex(asset_guard.AssetContractError, "hero/hero_spritesheet.1.png"):
                asset_guard.audit_zip(contract, broken, "assets/")

    def test_json_ldtk_rel_path_is_resolved_relative_to_world(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            actors, content = self.make_repo(repo)
            (actors / "sprites").mkdir()
            (actors / "sprites/tiles.png").write_bytes(b"tiles")
            (content / "worlds").mkdir()
            (content / "worlds/hall.ldtk").write_text(
                '{"defs": {"tilesets": [{"relPath": "../sprites/tiles.png"}]}}',
                encoding="utf8",
            )
            contract, _ = asset_guard.build_contract(repo, "android")
            self.assertIn("sprites/tiles.png", contract.entries)
            self.assertTrue(
                any(
                    origin.endswith(":relPath")
                    for origin in contract.entries["sprites/tiles.png"].origins
                )
            )

    def test_declared_asset_missing_from_both_roots_is_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            _, content = self.make_repo(repo)
            (content / "data").mkdir()
            (content / "data/character_catalog.ron").write_text(
                '(characters: {"hero": (spritesheet: "sprites/missing.png", manifest: "sprites/missing.ron")})',
                encoding="utf8",
            )
            with self.assertRaisesRegex(asset_guard.AssetContractError, "sprites/missing.png"):
                asset_guard.build_contract(repo, "android")

    def test_different_bytes_at_same_two_root_path_are_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            actors, content = self.make_repo(repo)
            (actors / "sprites").mkdir()
            (content / "sprites").mkdir()
            (actors / "sprites/shared.png").write_bytes(b"actors")
            (content / "sprites/shared.png").write_bytes(b"content")
            with self.assertRaisesRegex(asset_guard.AssetContractError, "implicit two-root asset override"):
                asset_guard.collect_source_files(repo, "android")

    def test_case_collisions_are_fatal_even_on_case_sensitive_hosts(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            actors, _ = self.make_repo(repo)
            (actors / "sprites").mkdir()
            (actors / "sprites/Hero.png").write_bytes(b"a")
            (actors / "sprites/hero.png").write_bytes(b"b")
            with self.assertRaisesRegex(asset_guard.AssetContractError, "case-colliding"):
                asset_guard.collect_source_files(repo, "android")

    def test_steamdeck_contract_excludes_local_machine_fonts(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            repo = Path(d)
            actors, _ = self.make_repo(repo)
            (actors / "fonts/local").mkdir(parents=True)
            (actors / "fonts/local/dev.ttf").write_bytes(b"local")
            contract, _ = asset_guard.build_contract(repo, "steamdeck")
            self.assertNotIn("fonts/local/dev.ttf", contract.entries)


if __name__ == "__main__":
    unittest.main()
