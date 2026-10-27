"""Preview framing follows the subject and preserves authored motion."""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from art_index import thumbs, sheets
from art_index.assets import Asset
from art_index.paths import Root
from art_index.scan import Product


def figure(size, box):
    image = Image.new("RGBA", size)
    x, y, w, h = box
    ImageDraw.Draw(image).rectangle((x, y, x + w - 1, y + h - 1), fill="green")
    return image, dict(zip(("x", "y", "w", "h"), box)), {"x": x + w / 2, "y": y + h}


def bounds(image):
    return image.getchannel("A").point(lambda alpha: 255 if alpha > 128 else 0).getbbox()


def test_subject_size_and_baseline_do_not_depend_on_canvas_padding_or_resolution():
    measured = []
    for size, box in [((100, 100), (40, 10, 20, 60)), ((400, 500), (170, 300, 20, 60)), ((400, 400), (160, 40, 80, 240))]:
        image, body, feet = figure(size, box)
        camera = thumbs.subject_camera(image, body, feet)
        measured.append(bounds(thumbs.subject_frame(image, camera, 192)))
    for actual in measured[1:]:
        assert all(abs(a - b) <= 1 for a, b in zip(actual, measured[0]))
    assert abs(measured[0][3] - 192 * .9) <= 1


def test_one_camera_preserves_movement_between_action_frames():
    neutral, body, feet = figure((100, 100), (40, 10, 20, 60))
    moved, _, _ = figure((100, 100), (46, 10, 20, 60))
    camera = thumbs.subject_camera(neutral, body, feet)
    rest_box = bounds(thumbs.subject_frame(neutral, camera, 128))
    moved_box = bounds(thumbs.subject_frame(moved, camera, 128))
    assert abs((moved_box[0] - rest_box[0]) - 6 * camera["scale"] * 128) <= 1
    assert moved_box[2] - moved_box[0] == rest_box[2] - rest_box[0]


def test_neutral_art_outside_gameplay_bounds_stays_visible():
    image, body, feet = figure((200, 200), (70, 50, 40, 80))
    ImageDraw.Draw(image).rectangle((70, 10, 109, 29), fill="red")
    camera = thumbs.subject_camera(image, body, feet)
    framed = thumbs.subject_frame(image, camera, 128)
    box = bounds(framed)
    assert camera["bounds"][1] == 10
    assert 0 < box[0] < box[2] < 128 and 0 < box[1] < box[3] < 128
    assert any(r > g and a > 128 for _, (r, g, b, a) in framed.getcolors(framed.width * framed.height))


def test_missing_body_metadata_uses_visible_art_and_reports_the_basis():
    image, _, _ = figure((300, 200), (110, 120, 100, 20))
    camera = thumbs.subject_camera(image, None, None)
    framed = thumbs.subject_frame(image, camera, 128)
    box = bounds(framed)
    assert camera["basis"] == camera["feet_basis"] == "visible"
    assert abs((box[2] - box[0]) / (box[3] - box[1]) - 5) < .3
    assert thumbs.subject_camera(Image.new("RGBA", (32, 32)), None, None)["basis"] == "empty"


def test_packed_pages_make_a_subject_cover_and_keep_the_full_frame_strip(tmp_path):
    for name in ("a", "b"):
        Image.new("RGBA", (8, 12), "green").save(tmp_path / f"{name}.png")
    spec = {"pages": [str(tmp_path / "a.png"), str(tmp_path / "b.png")], "frame": [80, 100],
            "rows": [{"rects": [[0, 0, 8, 12, 1, 32, 70], [0, 0, 8, 12, 0, 35, 70]],
                      "dest": str(tmp_path / "strip.webp"), "subject_dest": str(tmp_path / "subject.webp")}],
            "cover_row": 0, "cover_frame": 0, "cover_dest": str(tmp_path / "cover.webp"),
            "framing_dest": str(tmp_path / "framing.json")}
    assert thumbs.sheet_task(spec) is None
    raw = Image.open(tmp_path / "strip.webp")
    subject = Image.open(tmp_path / "subject.webp")
    assert raw.size == (thumbs._cell((80, 100))[0] * 2, 128)
    assert subject.size == (256, 128)
    assert bounds(subject.crop((0, 0, 128, 128)))[3] <= 128
    assert json.loads((tmp_path / "framing.json").read_text())["basis"] == "visible"
    assert Image.open(tmp_path / "cover.webp").size == (192, 192)


def test_long_animation_preview_keeps_frames_past_the_first_grid_row(tmp_path):
    for name, color in [("a", "green"), ("b", "red")]:
        Image.new("RGBA", (8, 12), color).save(tmp_path / f"{name}.png")
    frames = 130
    spec = {"pages": [str(tmp_path / "a.png"), str(tmp_path / "b.png")], "frame": [80, 100],
            "rows": [{"rects": [[0, 0, 8, 12, int(i == frames - 1), 32, 70] for i in range(frames)],
                      "dest": str(tmp_path / "strip.webp"), "subject_dest": str(tmp_path / "subject.webp")}],
            "cover_row": 0, "cover_frame": 0, "cover_dest": str(tmp_path / "cover.webp"),
            "framing_dest": str(tmp_path / "framing.json")}
    assert thumbs.sheet_task(spec) is None
    image = Image.open(tmp_path / "subject.webp").convert("RGBA")
    columns = thumbs._columns(frames, 128, 128)
    x, y = (frames - 1) % columns * 128, (frames - 1) // columns * 128
    last = image.crop((x, y, x + 128, y + 128))
    assert max(image.size) <= thumbs.WEBP_MAX_SIDE
    assert bounds(last) is not None
    assert last.getchannel("R").getextrema()[1] > last.getchannel("G").getextrema()[1]


def test_runtime_only_sheet_uses_its_shared_atlas_without_a_yaml_sidecar(tmp_path):
    root = Root("sprites", tmp_path)
    manifest = tmp_path / "nested/body_spritesheet.ron"
    page = tmp_path / "atlas/shared.png"
    manifest.parent.mkdir(); page.parent.mkdir()
    manifest.write_text("runtime metadata")
    Image.new("RGBA", (8, 12), "green").save(page)
    metadata = Product("sprites/nested/body_spritesheet.ron", root, "nested/body_spritesheet.ron", manifest, 1, 0, "manifest", role="sheet_record")
    pixels = Product("sprites/atlas/shared.png", root, "atlas/shared.png", page, 1, 0, "pixels", role="image")
    products = {product.path: product for product in (metadata, pixels)}
    asset = Asset("sheet:nested/body", "character", "nested/body", products=[metadata.path])
    layout = {"schema": "runtime-ron", "frame": [80, 100], "pages": ["../atlas/shared.png"],
              "body_bbox": {"x": 32, "y": 70, "w": 8, "h": 12}, "feet": None,
              "rows": [{"name": "idle", "frames": 1, "ms": 100, "rects": [[0, 0, 8, 12, 0, 32, 70]]}]}
    inspection = {"units": {"target:body": {"published": [{"path": str(manifest), "tier": "full", "key": "body", "layout": layout}]}}}
    sheets.use_runtime_layouts({asset.id: asset}, products, inspection)
    out = tmp_path / "previews"
    assert thumbs.plan_and_draw(out, products, {asset.id: asset}, jobs=1) == []
    assert asset.data["layout"]["schema"] == "runtime-ron"
    assert asset.data["layout"]["framing"]["feet_basis"] == "body-bounds"
    assert bounds(Image.open(out / asset.cover).convert("RGBA")) is not None
