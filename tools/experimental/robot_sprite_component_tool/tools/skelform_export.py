#!/usr/bin/env python3
"""Export one assembled robot pose into a SkelForm-compatible .skf package.

This is a pragmatic bridge from the current YAML rig to SkelForm.  It starts
with one pose, clips the green-screen source art with an alpha ramp, packs the
needed parts into atlas0.png, and writes armature.json/editor.json so the file
can be opened and refined in SkelForm.
"""

from __future__ import annotations

import argparse
import io
import json
import math
import zipfile
from pathlib import Path
from typing import Any, Dict, Mapping, Optional, Tuple

import numpy as np
import yaml
from PIL import Image

try:
    from tools.robot_asset_tool import estimate_background_rgb, greenscreen_to_alpha
    from tools.robot_rig_sheet import (
        ComponentAtlas,
        RigJob,
        RobotAssembler,
        load_pose_overrides,
    )
except Exception:  # local execution fallback
    from robot_asset_tool import estimate_background_rgb, greenscreen_to_alpha
    from robot_rig_sheet import (
        ComponentAtlas,
        RigJob,
        RobotAssembler,
        load_pose_overrides,
    )

Point = Tuple[float, float]


def _vec(x: float, y: float) -> Dict[str, float]:
    return {"x": float(x), "y": float(y)}


def _ivec(x: int | float, y: int | float) -> Dict[str, int]:
    return {"x": int(round(x)), "y": int(round(y))}


def _bbox_from_alpha(img: Image.Image) -> Tuple[int, int, int, int]:
    alpha = np.asarray(img.getchannel("A"))
    ys, xs = np.where(alpha > 0)
    if len(xs) == 0:
        return (0, 0, img.width, img.height)
    return (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)


def _crop_from_source(
    source: Image.Image,
    sprite: Mapping[str, Any],
    bg_rgb: np.ndarray,
    transparent: float,
    opaque: float,
    flip_x: bool = False,
) -> Image.Image:
    x, y, w, h = map(int, sprite["rect"])
    pad = 4
    x0 = max(0, x - pad)
    y0 = max(0, y - pad)
    x1 = min(source.width, x + w + pad)
    y1 = min(source.height, y + h + pad)
    crop = source.crop((x0, y0, x1, y1)).convert("RGBA")
    crop = greenscreen_to_alpha(
        crop, bg_rgb, transparent=transparent, opaque=opaque, despill=True
    )
    if flip_x:
        crop = crop.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    bx0, by0, bx1, by1 = _bbox_from_alpha(crop)
    bx0 = max(0, bx0 - 2)
    by0 = max(0, by0 - 2)
    bx1 = min(crop.width, bx1 + 2)
    by1 = min(crop.height, by1 + 2)
    return crop.crop((bx0, by0, bx1, by1))


def _pack(
    textures: Dict[str, Image.Image], padding: int = 4
) -> Tuple[Image.Image, Dict[str, Dict[str, int]]]:
    items = sorted(
        textures.items(), key=lambda kv: max(kv[1].height, kv[1].width), reverse=True
    )
    area = sum((im.width + padding) * (im.height + padding) for _, im in items)
    size = 256
    while size * size < area * 1.4:
        size *= 2
    while True:
        x = padding
        y = padding
        shelf = 0
        layout = {}
        ok = True
        for name, im in items:
            if x + im.width + padding > size:
                x = padding
                y += shelf + padding
                shelf = 0
            if y + im.height + padding > size:
                ok = False
                break
            layout[name] = {"x": x, "y": y, "w": im.width, "h": im.height}
            x += im.width + padding
            shelf = max(shelf, im.height)
        if ok:
            atlas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
            for name, im in items:
                r = layout[name]
                atlas.alpha_composite(im, (r["x"], r["y"]))
            return atlas, layout
        size *= 2


def _center(bounds) -> Point:
    x0, y0, x1, y1 = map(float, bounds)
    return ((x0 + x1) / 2.0, (y0 + y1) / 2.0)


def _to_world(pt: Point, origin: Point) -> Point:
    # SkelForm space is y-up; image/manifests are y-down.
    return (float(pt[0] - origin[0]), float(-(pt[1] - origin[1])))


def _rot(pt: Point, degrees: float) -> Point:
    rad = math.radians(degrees)
    c = math.cos(rad)
    s = math.sin(rad)
    return (pt[0] * c - pt[1] * s, pt[0] * s + pt[1] * c)


def _inverse_local(
    child_world: Point, parent_world: Point, parent_scale: float, parent_rot_deg: float
) -> Point:
    dx = (child_world[0] - parent_world[0]) / max(parent_scale, 1e-8)
    dy = (child_world[1] - parent_world[1]) / max(parent_scale, 1e-8)
    return _rot((dx, dy), -parent_rot_deg)


def _key(frame: int, bone_id: int, element: str, value: float) -> Dict[str, Any]:
    return {
        "frame": int(frame),
        "bone_id": int(bone_id),
        "element": element,
        "value": float(value),
        "start_handle": {"x": 0.0, "y": 0.0},
        "end_handle": {"x": 0.0, "y": 0.0},
        "handle_preset": "Linear",
    }


def _sanitize_int_fields(armature: Dict[str, Any]) -> None:
    """Force SkelForm integer fields to JSON ints, not 4.0-style floats."""
    armature["ik_root_ids"] = [int(v) for v in armature.get("ik_root_ids", [])]
    for atlas in armature.get("atlases", []):
        if "size" in atlas:
            atlas["size"] = _ivec(atlas["size"]["x"], atlas["size"]["y"])
    for style in armature.get("styles", []):
        if "id" in style:
            style["id"] = int(style["id"])
        for tex in style.get("textures", []):
            tex["atlas_idx"] = int(tex.get("atlas_idx", 0))
            tex["offset"] = _ivec(tex["offset"]["x"], tex["offset"]["y"])
            tex["size"] = _ivec(tex["size"]["x"], tex["size"]["y"])
    for bone in armature.get("bones", []):
        for key in ["id", "parent_id", "zindex", "ik_family_id", "ik_target_id"]:
            if key in bone:
                bone[key] = int(bone[key])
        if "ik_bone_ids" in bone:
            bone["ik_bone_ids"] = [int(v) for v in bone["ik_bone_ids"]]
        if "style_ids" in bone:
            bone["style_ids"] = [int(v) for v in bone["style_ids"]]
    for anim in armature.get("animations", []):
        if "id" in anim:
            anim["id"] = int(anim["id"])
        if "fps" in anim:
            anim["fps"] = int(anim["fps"])
        for keyframe in anim.get("keyframes", []):
            keyframe["frame"] = int(keyframe["frame"])
            keyframe["bone_id"] = int(keyframe["bone_id"])


def export_skelform(
    config: str | Path,
    output: str | Path,
    animation: str = "run",
    frame_index: int = 0,
    transparent: float = 38.0,
    opaque: float = 92.0,
) -> Path:
    config = Path(config)
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    job = RigJob.load(config)
    pose = load_pose_overrides(job.pose_overrides)
    atlas = ComponentAtlas(job.metadata, job.slices)
    asm = RobotAssembler(atlas, job.render, pose_overrides=pose)
    thumb, manifest = asm.render_frame(animation, frame_index)
    comps = {
        c["role"]: c for c in manifest["pose"]["components"] if c.get("visible", True)
    }

    meta = yaml.safe_load(Path(job.metadata).read_text())
    image_file = (Path(job.metadata).parent / meta["image"]["file"]).resolve()
    source = Image.open(image_file).convert("RGBA")
    bg_rgb = np.array(
        meta.get("image", {}).get("estimated_background_rgb")
        or estimate_background_rgb(np.array(source))
    )
    sprites = meta["sprites"]

    tex_roles = [
        "torso",
        "head",
        "back_arm",
        "back_hand",
        "front_arm",
        "front_hand",
        "back_leg",
        "back_foot",
        "front_leg",
        "front_foot",
    ]
    textures: Dict[str, Image.Image] = {}
    for role in tex_roles:
        sprite = str(comps[role]["sprite"])
        base = sprite.split("@")[0]
        textures[role] = _crop_from_source(
            source,
            sprites[base],
            bg_rgb,
            transparent,
            opaque,
            flip_x=("@flip_x" in sprite),
        )
    # Add a true pelvis texture from the canonical atlas if present.
    if "pelvis_front" in sprites:
        textures["pelvis"] = _crop_from_source(
            source, sprites["pelvis_front"], bg_rgb, transparent, opaque
        )

    atlas_img, layout = _pack(textures)

    origin = _center(comps["torso"]["bounds"])
    torso_scale = float(comps["torso"]["scale"])
    bones = []
    world_cache: Dict[str, Dict[str, Any]] = {}

    def add_bone(
        name, parent, *, tex=None, z=0, world=(0, 0), final_scale=1.0, rot=0.0, ik=None
    ):
        """Add a bone using desired world pose; compute SkelForm local pose.

        Textured hand/foot bones are deliberately *not* IK chain endpoints.
        Instead, the exporter creates empty Wrist/Ankle endpoint bones and
        parents the visible hand/foot textures under those endpoint bones. That
        makes it possible to reposition a hand relative to its wrist / arm in
        SkelForm without confusing the IK solver.
        """
        bid = len(bones)
        if parent is None:
            parent_id = -1
            local_pos = (float(world[0]), float(world[1]))
            local_scale = float(final_scale)
            local_rot = float(rot)
            inherited_scale = float(final_scale)
            inherited_rot = float(rot)
        else:
            parent_id = next(b["id"] for b in bones if b["name"] == parent)
            pw = world_cache[parent]
            local_pos = _inverse_local(world, pw["world"], pw["scale"], pw["rot"])
            local_scale = float(final_scale) / max(float(pw["scale"]), 1e-8)
            local_rot = float(rot) - float(pw["rot"])
            inherited_scale = float(final_scale)
            inherited_rot = float(rot)
        b = {
            "id": int(bid),
            "name": name,
            "parent_id": int(parent_id),
            "pos": _vec(local_pos[0], local_pos[1]),
            "scale": _vec(local_scale, local_scale),
            "rot": math.radians(local_rot),
            "ik_family_id": -1,
            "init_pos": _vec(local_pos[0], local_pos[1]),
            "init_scale": _vec(local_scale, local_scale),
            "init_rot": math.radians(local_rot),
        }
        if tex:
            b.update({"tex": tex, "init_tex": tex, "zindex": int(z), "style_ids": [0]})
        if ik:
            b.update(ik)
        bones.append(b)
        world_cache[name] = {
            "world": (float(world[0]), float(world[1])),
            "scale": inherited_scale,
            "rot": inherited_rot,
        }
        return bid

    def role_world(role: str) -> Point:
        return _to_world(_center(comps[role]["bounds"]), origin)

    def joint_world(role: str) -> Point:
        # For hands / feet, the component target is the connected wrist / ankle
        # joint, whereas the component bounds center is only the art center.
        return _to_world(
            tuple(
                map(float, comps[role].get("target", _center(comps[role]["bounds"])))
            ),
            origin,
        )

    role_pos = {
        role: role_world(role)
        for role in [
            "torso",
            "head",
            "back_arm",
            "back_hand",
            "front_arm",
            "front_hand",
            "back_leg",
            "back_foot",
            "front_leg",
            "front_foot",
        ]
    }
    joint_pos = {
        "BackWrist": joint_world("back_hand"),
        "FrontWrist": joint_world("front_hand"),
        "BackAnkle": joint_world("back_foot"),
        "FrontAnkle": joint_world("front_foot"),
    }

    # SkelForm's examples keep IK targets as unparented bones near the start of
    # the list. These target the empty wrist/ankle endpoint bones, not the
    # visible hand/foot textures.
    target_world = {
        "BackHandTarget": joint_pos["BackWrist"],
        "FrontHandTarget": joint_pos["FrontWrist"],
        "BackFootTarget": joint_pos["BackAnkle"],
        "FrontFootTarget": joint_pos["FrontAnkle"],
    }
    for tname in [
        "BackHandTarget",
        "FrontHandTarget",
        "BackFootTarget",
        "FrontFootTarget",
    ]:
        add_bone(tname, None, world=target_world[tname], final_scale=1.0, rot=0.0)

    add_bone("Root", None, world=(0, 0), final_scale=1.0, rot=0.0)
    add_bone(
        "Pelvis",
        "Root",
        tex="pelvis" if "pelvis" in textures else None,
        z=4,
        world=(0, -30),
        final_scale=torso_scale,
        rot=0.0,
    )
    add_bone(
        "Torso",
        "Pelvis",
        tex="torso",
        z=5,
        world=(0, 0),
        final_scale=torso_scale,
        rot=-float(comps["torso"].get("angle", 0.0)),
    )

    # Body/head.
    add_bone(
        "Head",
        "Torso",
        tex="head",
        z=int(comps["head"].get("z_index", 10)),
        world=role_pos["head"],
        final_scale=float(comps["head"]["scale"]),
        rot=-float(comps["head"].get("angle", 0.0)),
    )

    # Arms: texture bone -> empty wrist IK endpoint -> visible hand texture.
    add_bone(
        "BackArm",
        "Torso",
        tex="back_arm",
        z=int(comps["back_arm"].get("z_index", 0)),
        world=role_pos["back_arm"],
        final_scale=float(comps["back_arm"]["scale"]),
        rot=-float(comps["back_arm"].get("angle", 0.0)),
    )
    add_bone(
        "BackWrist",
        "BackArm",
        world=joint_pos["BackWrist"],
        final_scale=float(comps["back_arm"]["scale"]),
        rot=-float(comps["back_arm"].get("angle", 0.0)),
    )
    add_bone(
        "BackHand",
        "BackWrist",
        tex="back_hand",
        z=int(comps["back_hand"].get("z_index", 0)),
        world=role_pos["back_hand"],
        final_scale=float(comps["back_hand"]["scale"]),
        rot=-float(comps["back_hand"].get("angle", 0.0)),
    )

    add_bone(
        "FrontArm",
        "Torso",
        tex="front_arm",
        z=int(comps["front_arm"].get("z_index", 0)),
        world=role_pos["front_arm"],
        final_scale=float(comps["front_arm"]["scale"]),
        rot=-float(comps["front_arm"].get("angle", 0.0)),
    )
    add_bone(
        "FrontWrist",
        "FrontArm",
        world=joint_pos["FrontWrist"],
        final_scale=float(comps["front_arm"]["scale"]),
        rot=-float(comps["front_arm"].get("angle", 0.0)),
    )
    add_bone(
        "FrontHand",
        "FrontWrist",
        tex="front_hand",
        z=int(comps["front_hand"].get("z_index", 0)),
        world=role_pos["front_hand"],
        final_scale=float(comps["front_hand"]["scale"]),
        rot=-float(comps["front_hand"].get("angle", 0.0)),
    )

    # Legs: texture bone -> empty ankle IK endpoint -> visible foot texture.
    add_bone(
        "BackLeg",
        "Pelvis",
        tex="back_leg",
        z=int(comps["back_leg"].get("z_index", 0)),
        world=role_pos["back_leg"],
        final_scale=float(comps["back_leg"]["scale"]),
        rot=-float(comps["back_leg"].get("angle", 0.0)),
    )
    add_bone(
        "BackAnkle",
        "BackLeg",
        world=joint_pos["BackAnkle"],
        final_scale=float(comps["back_leg"]["scale"]),
        rot=-float(comps["back_leg"].get("angle", 0.0)),
    )
    add_bone(
        "BackFoot",
        "BackAnkle",
        tex="back_foot",
        z=int(comps["back_foot"].get("z_index", 0)),
        world=role_pos["back_foot"],
        final_scale=float(comps["back_foot"]["scale"]),
        rot=-float(comps["back_foot"].get("angle", 0.0)),
    )

    add_bone(
        "FrontLeg",
        "Pelvis",
        tex="front_leg",
        z=int(comps["front_leg"].get("z_index", 0)),
        world=role_pos["front_leg"],
        final_scale=float(comps["front_leg"]["scale"]),
        rot=-float(comps["front_leg"].get("angle", 0.0)),
    )
    add_bone(
        "FrontAnkle",
        "FrontLeg",
        world=joint_pos["FrontAnkle"],
        final_scale=float(comps["front_leg"]["scale"]),
        rot=-float(comps["front_leg"].get("angle", 0.0)),
    )
    add_bone(
        "FrontFoot",
        "FrontAnkle",
        tex="front_foot",
        z=int(comps["front_foot"].get("z_index", 0)),
        world=role_pos["front_foot"],
        final_scale=float(comps["front_foot"]["scale"]),
        rot=-float(comps["front_foot"].get("angle", 0.0)),
    )

    # Annotate roots with IK metadata. The visible hands/feet are NOT in the IK
    # chain; they are children of the endpoint bones. This keeps them editable
    # as attachments and prevents hand motion from being interpreted as chain
    # length / bend data.
    id_by_name = {b["name"]: b["id"] for b in bones}
    ik_specs = [
        ("BackArm", "BackHandTarget", ["BackArm", "BackWrist"], "Clockwise"),
        ("FrontArm", "FrontHandTarget", ["FrontArm", "FrontWrist"], "CounterClockwise"),
        ("BackLeg", "BackFootTarget", ["BackLeg", "BackAnkle"], "Clockwise"),
        ("FrontLeg", "FrontFootTarget", ["FrontLeg", "FrontAnkle"], "CounterClockwise"),
    ]
    ik_roots = []
    for fam, (root, target, chain, constraint) in enumerate(ik_specs):
        b = bones[id_by_name[root]]
        b["ik_family_id"] = int(fam)
        b["ik_constraint"] = constraint
        b["ik_mode"] = "FABRIK"
        b["ik_target_id"] = int(id_by_name[target])
        b["ik_bone_ids"] = [int(id_by_name[n]) for n in chain]
        b["init_ik_constraint"] = constraint
        b["init_ik_mode"] = "FABRIK"
        ik_roots.append(int(id_by_name[root]))

    textures_json = []
    for name, r in layout.items():
        textures_json.append(
            {
                "name": name,
                "offset": _ivec(r["x"], r["y"]),
                "size": _ivec(r["w"], r["h"]),
                "atlas_idx": 0,
            }
        )

    keyframes = []
    for b in bones:
        bid = b["id"]
        keyframes.extend(
            [
                _key(0, bid, "PositionX", b["pos"]["x"]),
                _key(0, bid, "PositionY", b["pos"]["y"]),
                _key(0, bid, "Rotation", b["rot"]),
                _key(0, bid, "ScaleX", b["scale"]["x"]),
                _key(0, bid, "ScaleY", b["scale"]["y"]),
            ]
        )

    armature = {
        "version": "0.4.0",
        "ik_root_ids": ik_roots,
        "baked_ik": False,
        "img_format": "PNG",
        "bones": bones,
        "animations": [
            {"name": "Pose_001", "id": 0, "fps": 12, "keyframes": keyframes}
        ],
        "atlases": [
            {"filename": "atlas0.png", "size": _ivec(atlas_img.width, atlas_img.height)}
        ],
        "styles": [{"id": 0, "name": "Default", "textures": textures_json}],
    }
    _sanitize_int_fields(armature)
    editor = {
        "camera": {"pos": _vec(0, 0), "zoom": 1300.0},
        "bones": [
            {
                "folded": False,
                "ik_folded": False,
                "meshdef_folded": False,
                "effects_folded": False,
                "ik_disabled": False,
                "locked": False,
            }
            for _ in bones
        ],
        "styles": [{"active": True}],
    }
    readme = f"Generated from {config.name}, animation={animation}, frame={frame_index}. Textures are clipped from green screen using a threshold alpha ramp. IK target bones are top-level and appear before the rig bones so SkelForm assigns each target to the intended IK family."
    thumb.thumbnail((256, 256), Image.Resampling.LANCZOS)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("armature.json", json.dumps(armature, indent=2))
        zf.writestr("editor.json", json.dumps(editor, indent=2))
        zf.writestr("readme.md", readme)
        buf = io.BytesIO()
        atlas_img.save(buf, format="PNG")
        zf.writestr("atlas0.png", buf.getvalue())
        buf = io.BytesIO()
        thumb.save(buf, format="PNG")
        zf.writestr("thumbnail.png", buf.getvalue())
    return output


def main(argv: Optional[list[str]] = None) -> None:
    p = argparse.ArgumentParser()
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="run")
    p.add_argument("--frame-index", type=int, default=0)
    p.add_argument("--transparent-threshold", type=float, default=38.0)
    p.add_argument("--opaque-threshold", type=float, default=92.0)
    args = p.parse_args(argv)
    out = export_skelform(
        args.config,
        args.output,
        args.animation,
        args.frame_index,
        args.transparent_threshold,
        args.opaque_threshold,
    )
    print(out)


if __name__ == "__main__":
    main()
