"""The art index on a small synthetic repository: each file gets one
generator link with the right confidence, sheets group as one asset with
their tiers, and uses resolve through content data and LDtk placements."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import pytest
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from art_index import build, sheets  # noqa: E402

ASSETS = "crates/ambition_platformer2d_actor_monolith/assets"
PROV = "crates/ambition_platformer2d_actor_monolith/asset_provenance"

LAYOUT = """target: foo
image: foo_spritesheet.png
frame_width: 32
frame_height: 32
rows:
- animation: idle
  row_index: 0
  frame_count: 2
  duration_ms: 100
  page: 0
  rects:
  - {x: 0, y: 0, w: 16, h: 16, fpage: 0, 'off': [8, 8]}
  - {x: 16, y: 0, w: 16, h: 16, fpage: 0, 'off': [8, 8]}
- animation: idle~mirrored
  row_index: 1
  frame_count: 2
  duration_ms: 100
  page: 0
  mirror_of: idle
  rects:
  - {x: 0, y: 0, w: 16, h: 16, fpage: 0, 'off': [8, 8]}
  - {x: 16, y: 0, w: 16, h: 16, fpage: 0, 'off': [8, 8]}
"""


def _png(path: Path, size=(32, 16), color=(200, 40, 40, 255)) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.new("RGBA", size, color).save(path)


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    repo = tmp_path_factory.mktemp("repo")
    sprites = repo / ASSETS / "sprites"
    _png(sprites / "foo_spritesheet.png")
    (sprites / "foo_spritesheet.yaml").write_text(LAYOUT)
    (sprites / "foo_spritesheet.ron").write_text('(target: "foo")\n')
    (sprites / "foo_actor.ron").write_text('(character_id: "npc_foo", display_name: Some("Foo"), provenance: Some((renderer_target: "foo")), tags: ["enemy"])\n')
    _png(sprites / "bar.png", color=(0, 0, 255, 255))  # an orphan: no generator
    _png(repo / ASSETS / "sprites_0_5x" / "foo_spritesheet.png", (16, 8))
    _png(repo / ASSETS / "room_dressing" / "hub_fill.png", (8, 8), (90, 90, 90, 255))
    _png(repo / "assets" / "concept_art" / "x" / "a.png", (8, 8))
    record = {
        "schema": "ambition.art_provenance.v1", "unit": "foo", "basis": "rendered", "root": "sprites",
        "generator": {"family": "sprite_target", "id": "foo", "source": "tools/foo.py", "command": "regen foo"},
        "outputs": [{"path": n, "sha256": _sha(sprites / n), "bytes": 1} for n in ("foo_spritesheet.png", "foo_spritesheet.yaml", "foo_spritesheet.ron", "foo_actor.ron")],
        "inputs": [{"path": "tools/foo.py", "sha256": "0" * 64}],
    }
    (repo / PROV / "sprites").mkdir(parents=True)
    (repo / PROV / "sprites" / "foo.json").write_text(json.dumps(record))
    hub = repo / ASSETS / "room_dressing" / "hub_fill.png"
    (repo / PROV / "room_dressing").mkdir(parents=True)
    (repo / PROV / "room_dressing" / "hub.json").write_text(json.dumps({
        "schema": "ambition.art_provenance.v1", "unit": "hub", "basis": "plan", "root": "room_dressing",
        "generator": {"family": "terrain_skin", "id": "hub"},
        "outputs": [{"path": "hub_fill.png", "sha256": _sha(hub), "bytes": 1, "made_by": {"callable": "terrain.skins:fill", "source": "skins.py", "line": 3}}],
        "inputs": [],
    }))
    (repo / "tools").mkdir()
    (repo / "tools" / "foo.py").write_text("# changed since the render\n")
    data = repo / "game" / "demo" / "assets" / "data"
    data.mkdir(parents=True)
    (data / "character_catalog.ron").write_text('{\n    "npc_foo": (\n        manifest: "sprites/foo_spritesheet.ron",\n    ),\n}\n')
    world = {"levels": [{"identifier": "room_a", "fieldInstances": [{"__identifier": "parallax_theme", "__value": "hub"}], "layerInstances": [{"entityInstances": [
        {"__identifier": "EnemySpawn", "fieldInstances": [{"__identifier": "character_id", "__value": "npc_foo"}]},
        {"__identifier": "EnemySpawn", "fieldInstances": [{"__identifier": "character_id", "__value": "npc_missing"}]},
    ]}]}]}
    (repo / "game" / "ambition_map_assets").mkdir(parents=True)
    (repo / "game" / "ambition_map_assets" / "w.ldtk").write_text(json.dumps(world))
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    out = repo / ".agent" / "art_inspector"
    catalog = build.build(repo, out, jobs=1, targets={"foo": {"family": "sprite_target", "id": "foo", "category": "characters", "source": "tools/foo.py", "outputs": []}}, log=lambda *_: None)
    return repo, out, catalog


def test_each_file_links_to_its_generator_with_its_confidence(built):
    _repo, _out, c = built
    prov = {p: r["provenance"] for p, r in c["products"].items()}
    assert prov["sprites/foo_spritesheet.png"]["confidence"] == "verified"
    assert prov["sprites/foo_spritesheet.png"]["generator"] == "target:foo"
    assert prov["sprites_0_5x/foo_spritesheet.png"] == {"confidence": "derived", "derived_from": "sprites/foo_spritesheet.png", "generator": "script:quality_variants"}
    assert prov["sprites/bar.png"]["confidence"] == "unknown"
    assert prov["room_dressing/hub_fill.png"]["confidence"] == "declared"
    assert prov["room_dressing/hub_fill.png"]["made_by"]["callable"] == "terrain.skins:fill"
    assert prov["assets/concept_art/x/a.png"]["confidence"] == "reference"


def test_a_sheet_is_one_asset_with_its_animations_tiers_and_previews(built):
    _repo, out, c = built
    foo = c["assets"]["sheet:foo"]
    assert foo["display"] == "Foo" and foo["character_id"] == "npc_foo" and foo["confidence"] == "verified"
    assert "sprites_0_5x/foo_spritesheet.png" in foo["products"]
    assert [r["name"] for r in foo["layout"]["rows"]] == ["idle", "idle~mirrored"]
    assert foo["layout"]["rows"][1]["mirror_of"] == "idle"
    assert foo["frames"] == 4
    assert all((out / r["strip"]).is_file() for r in foo["layout"]["rows"])
    assert (out / foo["cover"]).is_file()
    assert c["assets"]["skin:hub"]["family"] == "terrain skin"


def test_uses_resolve_through_content_rows_and_ldtk_placements(built):
    _repo, _out, c = built
    foo = c["assets"]["sheet:foo"]
    (use,) = foo["uses"]
    assert use["path"] == "game/demo/assets/data/character_catalog.ron" and use["line"] == 3 and use["row"] == "npc_foo"
    (place,) = foo["placements"]
    assert place == {"world": "game/ambition_map_assets/w.ldtk", "level": "room_a", "field": "EnemySpawn.character_id", "count": 1}
    assert any(p["field"] == "parallax_theme" for p in c["assets"]["skin:hub"]["placements"])


def test_findings_name_unknown_files_changed_inputs_and_unresolved_placements(built):
    _repo, _out, c = built
    codes = {(f["code"], f["subject"]) for f in c["findings"]}
    assert ("unknown-generator", "sprites/bar.png") in codes
    assert ("inputs-changed", "target:foo") in codes
    assert ("ldtk-unresolved", "EnemySpawn.character_id=npc_missing") in codes


def test_the_page_and_its_catalog_are_written_beside_each_other(built):
    _repo, out, _c = built
    for name in ("index.html", "app.js", "app.css", "catalog.js"):
        assert (out / name).is_file()
    text = (out / "catalog.js").read_text()
    assert text.startswith("window.ART_CATALOG = ") and "<" not in text
    c = json.loads(text[len("window.ART_CATALOG = "):].rstrip().rstrip(";"))
    # Every link is relative, so the page opens in place from file://.
    href = c["products"]["sprites/foo_spritesheet.png"]["href"]
    assert not href.startswith("/") and (out / href).resolve() == (Path(_repo) / ASSETS / "sprites" / "foo_spritesheet.png").resolve()


@pytest.mark.parametrize("shape", ["image", "images", "pages"])
def test_each_shape_of_the_sheet_layout_names_its_pages(tmp_path, shape):
    head = {"image": "image: foo_spritesheet.png\n", "images": "images: [foo_spritesheet.png, foo_spritesheet.1.png]\n", "pages": "pages: 2\n"}[shape]
    path = tmp_path / "foo_spritesheet.yaml"
    path.write_text(head + "frame_width: 32\nframe_height: 32\nrows: []\n")
    layout = sheets.read_layout(path)
    assert layout["schema"] == shape
    assert layout["pages"][0] == "foo_spritesheet.png"
    if shape != "image":
        assert layout["pages"] == ["foo_spritesheet.png", "foo_spritesheet.1.png"]


def test_a_character_page_names_its_lineage_from_the_design_history(built, tmp_path, monkeypatch):
    """The lineage comes from the renderer's design history: found through
    the sheet's target and through its catalog id. With no history store and
    no renderer checkout the eras keep their commit and label only."""
    repo, _out, _c = built
    real = Path(__file__).resolve().parents[2] / "ambition_sprite2d_renderer" / "design_history"
    pkg = repo / "tools" / "ambition_sprite2d_renderer" / "design_history"
    pkg.mkdir(parents=True, exist_ok=True)
    for source in real.glob("*.py"):
        (pkg / source.name).write_text(source.read_text())
    (pkg / "lineages.yaml").write_text(
        'lineages:\n  foo:\n    name: Foo\n    ids: [foo]\n    paths: [foo]\n    summary: "two looks"\n'
        '    eras:\n      - commit: "abcdef1"\n        label: "First"\n      - commit: "abcdef2"\n        label: "Second"\n'
    )
    monkeypatch.setenv("AMBITION_HISTORY_STORE", str(tmp_path / "no-store"))
    monkeypatch.setenv("AMBITION_DESIGN_HISTORY_CACHE", str(tmp_path / "cache"))
    for name in [m for m in sys.modules if m == "design_history" or m.startswith("design_history.")]:
        monkeypatch.delitem(sys.modules, name)
    monkeypatch.syspath_prepend(str(pkg.parent))
    targets = {"foo": {"family": "sprite_target", "id": "foo", "category": "characters", "source": "tools/foo.py", "outputs": []}}
    c = build.build(repo, tmp_path / "out", jobs=1, previews=False, targets=targets, log=lambda *_: None)
    (lineage,) = c["assets"]["sheet:foo"]["lineages"]
    assert lineage["lineage"] == "foo" and lineage["found_via"] == "target foo"
    assert [e["label"] for e in lineage["eras"]] == ["First", "Second"]
    assert not any(e["in_store"] for e in lineage["eras"])
    assert any(f["code"] == "lineage-warning" for f in c["findings"])
