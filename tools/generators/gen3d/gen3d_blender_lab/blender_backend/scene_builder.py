from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple


def _blender_modules():
    import bpy
    import mathutils

    return bpy, mathutils


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--payload", required=True)
    argv = []
    import sys

    if "--" in sys.argv:
        argv = sys.argv[sys.argv.index("--") + 1 :]
    else:
        argv = sys.argv[1:]
    return parser.parse_args(argv)


def hex_to_rgba(hex_color: str, alpha: float = 1.0) -> Tuple[float, float, float, float]:
    hex_color = hex_color.strip().lstrip("#")
    if len(hex_color) != 6:
        raise ValueError(f"Expected #RRGGBB color, got {hex_color!r}")
    r = int(hex_color[0:2], 16) / 255.0
    g = int(hex_color[2:4], 16) / 255.0
    b = int(hex_color[4:6], 16) / 255.0
    return (r, g, b, alpha)


def look_at(obj, target, mathutils) -> None:
    direction = mathutils.Vector(target) - obj.location
    quat = direction.to_track_quat("-Z", "Y")
    obj.rotation_euler = quat.to_euler()


def clear_scene(bpy) -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablock in (bpy.data.meshes, bpy.data.materials, bpy.data.cameras, bpy.data.lights, bpy.data.curves):
        for item in list(datablock):
            if item.users == 0:
                datablock.remove(item)


def configure_scene(bpy, width: int, height: int, transparent: bool = True) -> None:
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.film_transparent = transparent
    scene.eevee.taa_render_samples = 64
    scene.eevee.use_gtao = False
    scene.eevee.use_bloom = True
    scene.eevee.bloom_intensity = 0.025
    scene.eevee.bloom_radius = 2.0
    scene.render.resolution_x = int(width)
    scene.render.resolution_y = int(height)
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.use_persistent_data = False
    # Freestyle is the outline mechanism. Do not use inverted solidify shells for
    # outlines: they covered the whole character on some Blender versions.
    scene.render.use_freestyle = True
    try:
        view_layer = scene.view_layers[0]
        view_layer.use_freestyle = True
        fs = view_layer.freestyle_settings
        if fs.linesets:
            line_set = fs.linesets[0]
            line_set.name = "ink_lines"
            line_set.linestyle.thickness = 2.1
            line_set.linestyle.color = (0.035, 0.032, 0.042)
    except Exception:
        pass
    world = bpy.data.worlds.get("World")
    if world is None:
        world = bpy.data.worlds.new("World")
    scene.world = world
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    if bg is not None:
        bg.inputs[0].default_value = (0.78, 0.80, 0.84, 1.0)
        bg.inputs[1].default_value = 0.70


_MATERIAL_CACHE: Dict[Tuple[str, str, str, float], object] = {}


def ensure_outline_material(bpy):
    key = ("__outline__", "#000000", "#000000", 0.0)
    if key in _MATERIAL_CACHE:
        return _MATERIAL_CACHE[key]
    mat = bpy.data.materials.new(name="InkOutline")
    mat.diffuse_color = (0.035, 0.032, 0.042, 1.0)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf is not None:
        if "Base Color" in bsdf.inputs:
            bsdf.inputs["Base Color"].default_value = mat.diffuse_color
        if "Roughness" in bsdf.inputs:
            bsdf.inputs["Roughness"].default_value = 0.8
    _MATERIAL_CACHE[key] = mat
    return mat


def _set_principled_input(bsdf, names, value) -> None:
    for name in names:
        if name in bsdf.inputs:
            bsdf.inputs[name].default_value = value
            return


def ensure_toon_material(bpy, name: str, base_hex: str, shadow_hex: str, emission_strength: float = 0.0):
    """Create a robust colored material that works across Blender versions.

    This deliberately avoids the ShaderToRGB node tree for now. The previous
    material stack rendered as a black silhouette when combined with the
    inverted-hull outline pass on some systems. We prioritize stable visible
    colors and rely on Blender Freestyle for outlines.
    """
    key = (name, base_hex, shadow_hex, emission_strength)
    if key in _MATERIAL_CACHE:
        return _MATERIAL_CACHE[key]
    mat = bpy.data.materials.new(name=name)
    base = hex_to_rgba(base_hex)
    mat.diffuse_color = base
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    if bsdf is not None:
        _set_principled_input(bsdf, ["Base Color"], base)
        _set_principled_input(bsdf, ["Roughness"], 0.55)
        _set_principled_input(bsdf, ["Specular IOR Level", "Specular"], 0.26)
        # Blender 3.x/4.x use different names for emission fields.
        _set_principled_input(bsdf, ["Emission Color", "Emission"], base)
        _set_principled_input(bsdf, ["Emission Strength"], max(0.0, float(emission_strength)))
    _MATERIAL_CACHE[key] = mat
    return mat


def add_outline_modifier(obj, bpy, thickness: float = 0.02) -> None:
    # No-op by design. The old inverted-hull Solidify outline pass caused the
    # black-silhouette failure in canonical renders. Outlines now come from
    # Freestyle, configured once per scene.
    return None

def set_smooth(obj) -> None:
    if hasattr(obj.data, "polygons"):
        for polygon in obj.data.polygons:
            polygon.use_smooth = True


def link_object(collection, obj):
    collection.objects.link(obj)
    for c in list(obj.users_collection):
        if c != collection:
            c.objects.unlink(obj)


def primitive_cube(bpy, collection, name, location, scale, material, rotation=(0.0, 0.0, 0.0), bevel=0.08, outline=0.02):
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=location, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    set_smooth(obj)
    if obj.data.materials:
        obj.data.materials[0] = material
    else:
        obj.data.materials.append(material)
    if bevel > 0.0:
        mod = obj.modifiers.new(name="Bevel", type="BEVEL")
        mod.width = bevel
        mod.segments = 3
        mod.limit_method = "ANGLE"
    if outline > 0.0:
        add_outline_modifier(obj, bpy, thickness=outline)
    link_object(collection, obj)
    return obj


def primitive_uv_sphere(bpy, collection, name, location, scale, material, rotation=(0.0, 0.0, 0.0), outline=0.018):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=1.0, location=location, rotation=rotation, segments=24, ring_count=16)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    set_smooth(obj)
    if obj.data.materials:
        obj.data.materials[0] = material
    else:
        obj.data.materials.append(material)
    if outline > 0.0:
        add_outline_modifier(obj, bpy, thickness=outline)
    link_object(collection, obj)
    return obj


def primitive_cylinder_segment(bpy, collection, name, p1, p2, radius, material, outline=0.014):
    _, mathutils = _blender_modules()
    v1 = mathutils.Vector(p1)
    v2 = mathutils.Vector(p2)
    delta = v2 - v1
    length = delta.length
    mid = (v1 + v2) * 0.5
    bpy.ops.mesh.primitive_cylinder_add(radius=radius, depth=max(length, 1e-3), location=mid)
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = delta.to_track_quat("Z", "Y")
    set_smooth(obj)
    if obj.data.materials:
        obj.data.materials[0] = material
    else:
        obj.data.materials.append(material)
    bevel = obj.modifiers.new(name="Bevel", type="BEVEL")
    bevel.width = radius * 0.35
    bevel.segments = 2
    if outline > 0.0:
        add_outline_modifier(obj, bpy, thickness=outline)
    link_object(collection, obj)
    return obj


def primitive_cone_segment(bpy, collection, name, p1, p2, radius1, radius2, material, outline=0.014):
    _, mathutils = _blender_modules()
    v1 = mathutils.Vector(p1)
    v2 = mathutils.Vector(p2)
    delta = v2 - v1
    length = delta.length
    mid = (v1 + v2) * 0.5
    bpy.ops.mesh.primitive_cone_add(radius1=radius1, radius2=radius2, depth=max(length, 1e-3), location=mid)
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = delta.to_track_quat("Z", "Y")
    set_smooth(obj)
    if obj.data.materials:
        obj.data.materials[0] = material
    else:
        obj.data.materials.append(material)
    if outline > 0.0:
        add_outline_modifier(obj, bpy, thickness=outline)
    link_object(collection, obj)
    return obj


def create_collection(bpy, name: str):
    collection = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(collection)
    return collection


def ensure_camera_and_lights(bpy, spec_view: Dict[str, float]):
    scene = bpy.context.scene
    cam_data = bpy.data.cameras.new("SpriteCamera")
    cam = bpy.data.objects.new("SpriteCamera", cam_data)
    scene.collection.objects.link(cam)
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = float(spec_view.get("ortho_scale", 4.0))
    cam.location = (
        float(spec_view.get("camera_x", 0.85)),
        float(spec_view.get("camera_y", -6.8)),
        float(spec_view.get("camera_z", 1.35)),
    )
    look_at(cam, (
        float(spec_view.get("target_x", 0.0)),
        float(spec_view.get("target_y", 0.0)),
        float(spec_view.get("target_z", 1.05)),
    ), _blender_modules()[1])
    scene.camera = cam

    sun_data = bpy.data.lights.new("KeySun", type="SUN")
    sun = bpy.data.objects.new("KeySun", sun_data)
    scene.collection.objects.link(sun)
    sun.location = (4.5, -4.5, 7.0)
    sun.rotation_euler = (math.radians(42), math.radians(8), math.radians(22))
    sun_data.energy = 2.7
    sun_data.angle = math.radians(8)

    fill_data = bpy.data.lights.new("FillArea", type="AREA")
    fill = bpy.data.objects.new("FillArea", fill_data)
    scene.collection.objects.link(fill)
    fill.location = (-2.4, -3.5, 2.7)
    fill.rotation_euler = (math.radians(78), 0.0, math.radians(-28))
    fill_data.energy = 1200.0
    fill_data.shape = "RECTANGLE"
    fill_data.size = 4.0
    fill_data.size_y = 4.0

    rim_data = bpy.data.lights.new("RimArea", type="AREA")
    rim = bpy.data.objects.new("RimArea", rim_data)
    scene.collection.objects.link(rim)
    rim.location = (2.4, 2.2, 2.4)
    rim.rotation_euler = (math.radians(110), 0.0, math.radians(145))
    rim_data.energy = 450.0
    rim_data.shape = "RECTANGLE"
    rim_data.size = 2.5
    rim_data.size_y = 2.0


def point_from(origin: Sequence[float], length: float, angle_deg: float, depth: float = 0.0) -> Tuple[float, float, float]:
    angle = math.radians(angle_deg)
    return (
        origin[0] + math.cos(angle) * length,
        origin[1] + depth,
        origin[2] + math.sin(angle) * length,
    )


def robot_pose(animation: str, index: int, frame_count: int) -> Dict[str, float]:
    t = (index / max(frame_count, 1)) % 1.0
    cycle = math.sin(t * math.tau)
    cycle2 = math.sin(t * math.tau + math.pi)
    pose = {
        "root_z": 0.0,
        "root_x": 0.0,
        "torso_tilt": 0.0,
        "head_tilt": 0.0,
        "arm_front": -82.0,
        "arm_back": -98.0,
        "forearm_front": -88.0,
        "forearm_back": -92.0,
        "leg_front": -95.0,
        "leg_back": -85.0,
        "shin_front": -88.0,
        "shin_back": -92.0,
        "feet_lift_front": 0.0,
        "feet_lift_back": 0.0,
    }
    if animation == "idle":
        pose["root_z"] = 0.03 * math.sin(t * math.tau)
        pose["torso_tilt"] = 3.0 * math.sin(t * math.tau)
        pose["head_tilt"] = 4.0 * math.sin(t * math.tau)
        pose["arm_front"] += 6.0 * cycle2
        pose["arm_back"] += 6.0 * cycle
        pose["leg_front"] += 3.0 * cycle
        pose["leg_back"] += 3.0 * cycle2
    elif animation == "walk":
        pose["root_z"] = 0.04 * abs(cycle)
        pose["torso_tilt"] = 6.0 * cycle
        pose["arm_front"] = -98.0 + 24.0 * cycle2
        pose["arm_back"] = -82.0 + 24.0 * cycle
        pose["forearm_front"] = -92.0 + 12.0 * cycle2
        pose["forearm_back"] = -88.0 + 12.0 * cycle
        pose["leg_front"] = -92.0 + 32.0 * cycle
        pose["leg_back"] = -92.0 + 32.0 * cycle2
        pose["shin_front"] = -92.0 + max(0.0, -18.0 * cycle)
        pose["shin_back"] = -92.0 + max(0.0, 18.0 * cycle)
        pose["feet_lift_front"] = max(0.0, -0.05 * cycle)
        pose["feet_lift_back"] = max(0.0, 0.05 * cycle)
    elif animation == "run":
        pose["root_z"] = 0.08 * abs(cycle)
        pose["root_x"] = 0.04
        pose["torso_tilt"] = -10.0
        pose["head_tilt"] = -5.0
        pose["arm_front"] = -108.0 + 45.0 * cycle2
        pose["arm_back"] = -72.0 + 45.0 * cycle
        pose["forearm_front"] = -105.0 + 20.0 * cycle2
        pose["forearm_back"] = -80.0 + 20.0 * cycle
        pose["leg_front"] = -95.0 + 48.0 * cycle
        pose["leg_back"] = -95.0 + 48.0 * cycle2
        pose["shin_front"] = -92.0 + max(0.0, -30.0 * cycle)
        pose["shin_back"] = -92.0 + max(0.0, 30.0 * cycle)
        pose["feet_lift_front"] = max(0.0, -0.08 * cycle)
        pose["feet_lift_back"] = max(0.0, 0.08 * cycle)
    elif animation == "jump":
        if t < 0.2:
            f = t / 0.2
            pose["root_z"] = -0.08 * f
            pose["torso_tilt"] = 6.0 * f
            pose["leg_front"] = -85.0 + 15.0 * f
            pose["leg_back"] = -85.0 + 15.0 * f
        elif t < 0.55:
            f = (t - 0.2) / 0.35
            pose["root_z"] = 0.35 * math.sin(f * math.pi)
            pose["torso_tilt"] = -8.0
            pose["arm_front"] = -40.0
            pose["arm_back"] = -40.0
            pose["leg_front"] = -120.0
            pose["leg_back"] = -120.0
            pose["shin_front"] = -50.0
            pose["shin_back"] = -50.0
        else:
            f = (t - 0.55) / 0.45
            pose["root_z"] = 0.18 * (1.0 - f)
            pose["torso_tilt"] = 4.0 * (1.0 - f)
            pose["arm_front"] = -70.0
            pose["arm_back"] = -70.0
    elif animation == "fly":
        pose["root_z"] = 0.10 + 0.03 * cycle
        pose["root_x"] = 0.12
        pose["torso_tilt"] = -18.0
        pose["head_tilt"] = -8.0
        pose["arm_front"] = -35.0
        pose["arm_back"] = -35.0
        pose["forearm_front"] = -30.0
        pose["forearm_back"] = -30.0
        pose["leg_front"] = -132.0
        pose["leg_back"] = -126.0
        pose["shin_front"] = -48.0
        pose["shin_back"] = -55.0
    elif animation == "dash":
        pose["root_z"] = 0.02
        pose["root_x"] = 0.25
        pose["torso_tilt"] = -22.0
        pose["head_tilt"] = -10.0
        pose["arm_front"] = -52.0
        pose["arm_back"] = -138.0
        pose["forearm_front"] = -46.0
        pose["forearm_back"] = -122.0
        pose["leg_front"] = -120.0
        pose["leg_back"] = -78.0
        pose["shin_front"] = -75.0
        pose["shin_back"] = -98.0
    elif animation == "slash":
        if t < 0.35:
            f = t / 0.35
            pose["torso_tilt"] = 8.0 * f
            pose["arm_front"] = -65.0 + 55.0 * f
            pose["forearm_front"] = -82.0 + 42.0 * f
            pose["arm_back"] = -110.0
        else:
            f = (t - 0.35) / 0.65
            pose["torso_tilt"] = 8.0 - 22.0 * f
            pose["arm_front"] = -10.0 - 105.0 * f
            pose["forearm_front"] = -40.0 - 25.0 * f
            pose["arm_back"] = -110.0 + 15.0 * f
            pose["root_x"] = 0.07 * (1.0 - abs(0.5 - f))
    elif animation == "hit":
        f = min(1.0, t * 1.35)
        pose["root_x"] = -0.15 * math.sin(f * math.pi)
        pose["root_z"] = -0.05 * math.sin(f * math.pi)
        pose["torso_tilt"] = 18.0 * math.sin(f * math.pi)
        pose["head_tilt"] = 15.0 * math.sin(f * math.pi)
        pose["arm_front"] = -115.0
        pose["arm_back"] = -115.0
        pose["leg_front"] = -88.0
        pose["leg_back"] = -88.0
    return pose


def goblin_pose(animation: str, index: int, frame_count: int) -> Dict[str, float]:
    t = (index / max(frame_count, 1)) % 1.0
    cycle = math.sin(t * math.tau)
    cycle2 = math.sin(t * math.tau + math.pi)
    pose = {
        "root_z": 0.0,
        "root_x": 0.0,
        "torso_tilt": 0.0,
        "head_tilt": 0.0,
        "arm_front": -95.0,
        "arm_back": -105.0,
        "forearm_front": -95.0,
        "forearm_back": -95.0,
        "leg_front": -100.0,
        "leg_back": -90.0,
        "shin_front": -92.0,
        "shin_back": -92.0,
        "weapon_angle": 0.0,
    }
    if animation == "idle":
        pose["root_z"] = 0.02 * math.sin(t * math.tau)
        pose["torso_tilt"] = 3.0 * math.sin(t * math.tau)
        pose["head_tilt"] = 5.0 * math.sin(t * math.tau)
        pose["arm_front"] += 5.0 * cycle2
        pose["arm_back"] += 5.0 * cycle
        pose["weapon_angle"] = 14.0
    elif animation == "walk":
        pose["root_z"] = 0.04 * abs(cycle)
        pose["torso_tilt"] = 5.0 * cycle
        pose["arm_front"] = -106.0 + 20.0 * cycle2
        pose["arm_back"] = -84.0 + 20.0 * cycle
        pose["leg_front"] = -96.0 + 30.0 * cycle
        pose["leg_back"] = -96.0 + 30.0 * cycle2
        pose["shin_front"] = -96.0 + max(0.0, -24.0 * cycle)
        pose["shin_back"] = -96.0 + max(0.0, 24.0 * cycle)
        pose["weapon_angle"] = 12.0
    elif animation == "run":
        pose["root_z"] = 0.07 * abs(cycle)
        pose["root_x"] = 0.04
        pose["torso_tilt"] = -10.0
        pose["arm_front"] = -118.0 + 42.0 * cycle2
        pose["arm_back"] = -76.0 + 38.0 * cycle
        pose["leg_front"] = -96.0 + 44.0 * cycle
        pose["leg_back"] = -96.0 + 44.0 * cycle2
        pose["shin_front"] = -96.0 + max(0.0, -34.0 * cycle)
        pose["shin_back"] = -96.0 + max(0.0, 34.0 * cycle)
        pose["weapon_angle"] = 18.0
    elif animation == "jump":
        if t < 0.25:
            f = t / 0.25
            pose["root_z"] = -0.08 * f
            pose["leg_front"] = -88.0 + 18.0 * f
            pose["leg_back"] = -88.0 + 18.0 * f
        elif t < 0.6:
            f = (t - 0.25) / 0.35
            pose["root_z"] = 0.32 * math.sin(f * math.pi)
            pose["torso_tilt"] = -12.0
            pose["arm_front"] = -52.0
            pose["arm_back"] = -72.0
            pose["leg_front"] = -128.0
            pose["leg_back"] = -120.0
            pose["shin_front"] = -58.0
            pose["shin_back"] = -62.0
            pose["weapon_angle"] = -18.0
        else:
            f = (t - 0.6) / 0.4
            pose["root_z"] = 0.14 * (1.0 - f)
            pose["weapon_angle"] = 10.0
    elif animation == "fall":
        pose["root_z"] = 0.14 + 0.03 * cycle
        pose["torso_tilt"] = -14.0
        pose["head_tilt"] = -8.0
        pose["arm_front"] = -44.0
        pose["arm_back"] = -72.0
        pose["forearm_front"] = -34.0
        pose["forearm_back"] = -58.0
        pose["leg_front"] = -118.0
        pose["leg_back"] = -126.0
        pose["shin_front"] = -70.0
        pose["shin_back"] = -68.0
        pose["weapon_angle"] = -30.0
    elif animation == "slash":
        if t < 0.35:
            f = t / 0.35
            pose["torso_tilt"] = 14.0 * f
            pose["arm_front"] = -55.0 + 40.0 * f
            pose["forearm_front"] = -84.0 + 38.0 * f
            pose["weapon_angle"] = -42.0 + 28.0 * f
        else:
            f = (t - 0.35) / 0.65
            pose["torso_tilt"] = 12.0 - 24.0 * f
            pose["arm_front"] = -15.0 - 112.0 * f
            pose["forearm_front"] = -46.0 - 30.0 * f
            pose["arm_back"] = -110.0
            pose["weapon_angle"] = -16.0 + 98.0 * f
            pose["root_x"] = 0.08 * math.sin(f * math.pi)
    elif animation == "hurt":
        f = min(1.0, t * 1.5)
        pose["root_x"] = -0.14 * math.sin(f * math.pi)
        pose["torso_tilt"] = 16.0 * math.sin(f * math.pi)
        pose["head_tilt"] = 12.0 * math.sin(f * math.pi)
        pose["arm_front"] = -118.0
        pose["arm_back"] = -118.0
        pose["weapon_angle"] = 30.0
    elif animation == "death":
        f = min(1.0, t * 1.1)
        pose["root_x"] = 0.12 * f
        pose["root_z"] = -0.10 * f
        pose["torso_tilt"] = -72.0 * f
        pose["head_tilt"] = -54.0 * f
        pose["arm_front"] = -110.0 - 32.0 * f
        pose["arm_back"] = -105.0 - 20.0 * f
        pose["leg_front"] = -92.0 + 38.0 * f
        pose["leg_back"] = -92.0 + 12.0 * f
        pose["weapon_angle"] = 70.0 * f
    return pose


def build_robot(bpy, collection, spec: Dict[str, object], animation: str, index: int, frame_count: int):
    pose = robot_pose(animation, index, frame_count)
    white = ensure_toon_material(bpy, "RobotWhite", spec["primary_color"], spec["primary_shadow"])
    dark = ensure_toon_material(bpy, "RobotDark", spec["dark_color"], "#07070A")
    cyan = ensure_toon_material(bpy, "RobotCyan", spec["accent_color"], "#0DA4C5", emission_strength=0.25)
    purple = ensure_toon_material(bpy, "RobotPurple", spec["accent2_color"], "#6F55C8", emission_strength=0.10)
    metal = ensure_toon_material(bpy, "RobotMetal", spec["metal_color"], "#7D8796")

    root = (pose["root_x"], 0.0, pose["root_z"])
    pelvis_center = (root[0] + 0.00, 0.0, root[2] + 0.91)
    torso_center = (root[0] + 0.04, 0.0, root[2] + 1.19)
    head_center = (root[0] + 0.19, 0.0, root[2] + 1.67)
    torso_rot = (0.0, math.radians(pose["torso_tilt"] + 2.0), 0.0)
    head_rot = (0.0, math.radians(pose["head_tilt"] + 2.0), math.radians(-3.0))

    primitive_cube(bpy, collection, "robot_pelvis", pelvis_center, (spec["body_width"] * 0.26, spec["body_depth"] * 0.28, spec["body_height"] * 0.20), white, rotation=torso_rot, bevel=0.08, outline=0.018)
    primitive_cube(bpy, collection, "robot_body", torso_center, (spec["body_width"] * 0.36, spec["body_depth"] * 0.31, spec["body_height"] * 0.34), white, rotation=torso_rot, bevel=0.12, outline=0.018)
    primitive_cube(bpy, collection, "robot_head", head_center, (spec["head_size"] * 0.43, spec["head_size"] * 0.31, spec["head_size"] * 0.36), white, rotation=head_rot, bevel=0.16, outline=0.018)
    primitive_cylinder_segment(bpy, collection, "robot_neck", (torso_center[0] + 0.03, 0.0, torso_center[2] + 0.21), (head_center[0] - 0.08, 0.0, head_center[2] - 0.27), 0.036, metal, outline=0.0)

    face_y = -spec["head_size"] * 0.34
    primitive_cube(bpy, collection, "robot_face_bezel", (head_center[0] + spec["head_size"] * 0.09, face_y, head_center[2] + 0.00), (spec["head_size"] * 0.27, 0.026, spec["head_size"] * 0.18), dark, rotation=head_rot, bevel=0.06, outline=0.0)
    primitive_cube(bpy, collection, "robot_eye_front", (head_center[0] + spec["head_size"] * 0.17, face_y - 0.016, head_center[2] + 0.03), (0.038, 0.010, 0.072), cyan, rotation=head_rot, bevel=0.012, outline=0.0)
    primitive_cube(bpy, collection, "robot_eye_back", (head_center[0] + spec["head_size"] * 0.05, face_y - 0.016, head_center[2] + 0.03), (0.038, 0.010, 0.072), cyan, rotation=head_rot, bevel=0.012, outline=0.0)
    primitive_cube(bpy, collection, "robot_smile", (head_center[0] + spec["head_size"] * 0.11, face_y - 0.015, head_center[2] - 0.07), (0.064, 0.008, 0.016), cyan, rotation=head_rot, bevel=0.006, outline=0.0)
    primitive_cube(bpy, collection, "robot_cheek_dot", (head_center[0] - 0.01, face_y - 0.014, head_center[2] - 0.01), (0.016, 0.006, 0.016), purple, rotation=head_rot, bevel=0.006, outline=0.0)

    primitive_cube(bpy, collection, "robot_chest_panel", (torso_center[0] + 0.10, -0.10, torso_center[2] + 0.00), (0.10, 0.018, 0.13), dark, rotation=torso_rot, bevel=0.028, outline=0.0)
    primitive_cube(bpy, collection, "robot_chest_core", (torso_center[0] + 0.11, -0.115, torso_center[2] + 0.02), (0.040, 0.010, 0.085), cyan, rotation=torso_rot, bevel=0.015, outline=0.0)
    primitive_cube(bpy, collection, "robot_hip_light", (pelvis_center[0] + 0.07, -0.08, pelvis_center[2] + 0.02), (0.028, 0.008, 0.040), purple, rotation=torso_rot, bevel=0.010, outline=0.0)
    primitive_cube(bpy, collection, "robot_side_pack", (torso_center[0] - 0.10, 0.14, torso_center[2] + 0.02), (0.05, 0.07, 0.08), metal, bevel=0.03, outline=0.010)
    primitive_cube(bpy, collection, "robot_side_ear", (head_center[0] - 0.10, 0.15, head_center[2] + 0.03), (0.04, 0.06, 0.08), purple, bevel=0.03, outline=0.010)
    primitive_cylinder_segment(bpy, collection, "robot_antenna_stem", (head_center[0] - 0.06, -0.06, head_center[2] + spec["head_size"] * 0.29), (head_center[0] - 0.06, -0.06, head_center[2] + spec["head_size"] * 0.52), 0.016, purple, outline=0.0)
    primitive_uv_sphere(bpy, collection, "robot_antenna_tip", (head_center[0] - 0.06, -0.06, head_center[2] + spec["head_size"] * 0.59), (0.042, 0.042, 0.042), purple, outline=0.0)

    shoulder_front = (torso_center[0] + 0.05, -0.11, torso_center[2] + 0.08)
    shoulder_back = (torso_center[0] - 0.04, 0.11, torso_center[2] + 0.06)
    elbow_front = point_from(shoulder_front, spec["arm_length"], pose["arm_front"], 0.0)
    elbow_back = point_from(shoulder_back, spec["arm_length"] * 0.96, pose["arm_back"], 0.0)
    wrist_front = point_from(elbow_front, spec["forearm_length"], pose["forearm_front"], 0.0)
    wrist_back = point_from(elbow_back, spec["forearm_length"] * 0.95, pose["forearm_back"], 0.0)
    primitive_uv_sphere(bpy, collection, "robot_shoulder_front", shoulder_front, (0.072, 0.055, 0.072), white)
    primitive_uv_sphere(bpy, collection, "robot_shoulder_back", shoulder_back, (0.064, 0.050, 0.064), white)
    primitive_cylinder_segment(bpy, collection, "robot_upperarm_front", shoulder_front, elbow_front, 0.050, white)
    primitive_cylinder_segment(bpy, collection, "robot_upperarm_back", shoulder_back, elbow_back, 0.046, white)
    primitive_uv_sphere(bpy, collection, "robot_elbow_front", elbow_front, (0.036, 0.032, 0.036), metal, outline=0.0)
    primitive_uv_sphere(bpy, collection, "robot_elbow_back", elbow_back, (0.032, 0.028, 0.032), metal, outline=0.0)
    primitive_cylinder_segment(bpy, collection, "robot_forearm_front", elbow_front, wrist_front, 0.044, white)
    primitive_cylinder_segment(bpy, collection, "robot_forearm_back", elbow_back, wrist_back, 0.040, white)
    primitive_uv_sphere(bpy, collection, "robot_hand_front", wrist_front, (0.060, 0.046, 0.060), white)
    primitive_uv_sphere(bpy, collection, "robot_hand_back", wrist_back, (0.055, 0.042, 0.055), white)

    hip_front = (pelvis_center[0] + 0.04, -0.08, pelvis_center[2] - 0.05)
    hip_back = (pelvis_center[0] - 0.03, 0.08, pelvis_center[2] - 0.05)
    knee_front = point_from(hip_front, spec["leg_length"], pose["leg_front"], 0.0)
    knee_back = point_from(hip_back, spec["leg_length"], pose["leg_back"], 0.0)
    ankle_front = point_from(knee_front, spec["shin_length"], pose["shin_front"], 0.0)
    ankle_back = point_from(knee_back, spec["shin_length"], pose["shin_back"], 0.0)
    primitive_cylinder_segment(bpy, collection, "robot_thigh_front", hip_front, knee_front, 0.054, white)
    primitive_cylinder_segment(bpy, collection, "robot_thigh_back", hip_back, knee_back, 0.050, white)
    primitive_uv_sphere(bpy, collection, "robot_knee_front", knee_front, (0.038, 0.032, 0.038), metal, outline=0.0)
    primitive_uv_sphere(bpy, collection, "robot_knee_back", knee_back, (0.036, 0.030, 0.036), metal, outline=0.0)
    primitive_cylinder_segment(bpy, collection, "robot_shin_front", knee_front, ankle_front, 0.046, white)
    primitive_cylinder_segment(bpy, collection, "robot_shin_back", knee_back, ankle_back, 0.042, white)
    foot_front = (ankle_front[0] + 0.12, ankle_front[1] - 0.01, ankle_front[2] - 0.05 + pose["feet_lift_front"])
    foot_back = (ankle_back[0] + 0.10, ankle_back[1] + 0.01, ankle_back[2] - 0.05 + pose["feet_lift_back"])
    primitive_cube(bpy, collection, "robot_foot_front", foot_front, (0.13, 0.07, 0.045), white, bevel=0.05, outline=0.012)
    primitive_cube(bpy, collection, "robot_foot_back", foot_back, (0.11, 0.065, 0.04), white, bevel=0.05, outline=0.012)

    if animation == "slash":
        primitive_cone_segment(bpy, collection, "robot_energy_blade", wrist_front, (wrist_front[0] + 0.50, wrist_front[1], wrist_front[2] + 0.02), 0.052, 0.0, cyan, outline=0.0)
    if animation in {"fly", "dash"}:
        primitive_cone_segment(bpy, collection, "robot_thruster_front", (foot_front[0] - 0.05, foot_front[1], foot_front[2] - 0.02), (foot_front[0] - 0.20, foot_front[1], foot_front[2] - 0.20), 0.035, 0.0, cyan, outline=0.0)
        primitive_cone_segment(bpy, collection, "robot_thruster_back", (foot_back[0] - 0.05, foot_back[1], foot_back[2] - 0.02), (foot_back[0] - 0.18, foot_back[1], foot_back[2] - 0.18), 0.030, 0.0, cyan, outline=0.0)



def add_goblin_weapon(bpy, collection, item: str, hand: Sequence[float], angle_deg: float, metal, accent) -> None:
    length = 0.58
    item = (item or "spear").lower()
    a = math.radians(-18 + angle_deg)
    tip = (hand[0] + math.cos(a) * length, hand[1], hand[2] + math.sin(a) * length)
    if item in {"spear", "staff"}:
        primitive_cylinder_segment(bpy, collection, "goblin_weapon_shaft", (hand[0] - 0.07, hand[1], hand[2] - 0.02), tip, 0.028, accent)
        blade_tip = (tip[0] + math.cos(a) * 0.18, tip[1], tip[2] + math.sin(a) * 0.18)
        primitive_cone_segment(bpy, collection, "goblin_weapon_blade", tip, blade_tip, 0.07, 0.0, metal)
    elif item in {"sword", "knife"}:
        guard = (hand[0] + math.cos(a) * 0.10, hand[1], hand[2] + math.sin(a) * 0.10)
        blade_tip = (hand[0] + math.cos(a) * 0.54, hand[1], hand[2] + math.sin(a) * 0.54)
        primitive_cylinder_segment(bpy, collection, "goblin_weapon_hilt", (hand[0] - 0.05, hand[1], hand[2] - 0.02), guard, 0.03, accent)
        primitive_cone_segment(bpy, collection, "goblin_weapon_blade", guard, blade_tip, 0.06, 0.02, metal)
        primitive_cylinder_segment(bpy, collection, "goblin_weapon_cross", (guard[0], hand[1] - 0.12, guard[2]), (guard[0], hand[1] + 0.12, guard[2]), 0.02, accent)
    elif item in {"club", "mace"}:
        club_tip = (hand[0] + math.cos(a) * 0.42, hand[1], hand[2] + math.sin(a) * 0.42)
        primitive_cylinder_segment(bpy, collection, "goblin_weapon_handle", (hand[0] - 0.05, hand[1], hand[2] - 0.02), club_tip, 0.03, accent)
        primitive_uv_sphere(bpy, collection, "goblin_weapon_head", (club_tip[0] + math.cos(a) * 0.06, hand[1], club_tip[2] + math.sin(a) * 0.06), (0.10, 0.10, 0.10), metal)
    elif item == "gun":
        primitive_cube(bpy, collection, "goblin_gun_body", (hand[0] + 0.18, hand[1], hand[2] + 0.01), (0.18, 0.06, 0.06), metal, rotation=(0.0, math.radians(-4 + angle_deg), 0.0), bevel=0.03, outline=0.012)
        primitive_cube(bpy, collection, "goblin_gun_grip", (hand[0] + 0.08, hand[1], hand[2] - 0.12), (0.04, 0.05, 0.10), accent, rotation=(0.0, math.radians(-24 + angle_deg), 0.0), bevel=0.02, outline=0.012)


def build_goblin(bpy, collection, spec: Dict[str, object], animation: str, index: int, frame_count: int):
    pose = goblin_pose(animation, index, frame_count)
    skin = ensure_toon_material(bpy, "GoblinSkin", spec["skin_color"], spec["skin_shadow"])
    cloth = ensure_toon_material(bpy, "GoblinCloth", spec["cloth_color"], spec["cloth_shadow"])
    accent = ensure_toon_material(bpy, "GoblinAccent", spec["accent_color"], spec["accent2_color"], emission_strength=0.06)
    eyes = ensure_toon_material(bpy, "GoblinEyes", spec["eye_color"], spec["accent_color"], emission_strength=0.25)
    dark = ensure_toon_material(bpy, "GoblinDark", "#1E1723", "#09070C")
    metal = ensure_toon_material(bpy, "GoblinMetal", spec["metal_color"], "#7C74A2")

    root = (pose["root_x"], 0.0, pose["root_z"])
    pelvis_center = (root[0] - 0.05, 0.0, root[2] + 0.86)
    torso_center = (root[0] - 0.01, 0.0, root[2] + 1.08)
    head_center = (root[0] + 0.18, 0.0, root[2] + 1.55)

    primitive_uv_sphere(bpy, collection, "goblin_body", torso_center, (spec["body_width"] * 0.62, spec["body_depth"] * 0.56, spec["body_height"] * 0.56), cloth, outline=0.018)
    primitive_cube(bpy, collection, "goblin_tunic", (torso_center[0] + 0.02, -0.02, torso_center[2] - 0.01), (0.16, 0.10, 0.16), accent, bevel=0.03, outline=0.0)
    primitive_uv_sphere(bpy, collection, "goblin_head", head_center, (spec["head_size"] * 0.44, spec["head_size"] * 0.34, spec["head_size"] * 0.40), skin, outline=0.018)
    primitive_cone_segment(bpy, collection, "goblin_nose", (head_center[0] + 0.18, -0.16, head_center[2] + 0.00), (head_center[0] + 0.28, -0.18, head_center[2] - 0.01), 0.032, 0.0, skin, outline=0.0)
    primitive_cone_segment(bpy, collection, "goblin_ear_front", (head_center[0] - 0.02, -0.14, head_center[2] + 0.06), (head_center[0] + spec["ear_length"] * 0.95, -0.18, head_center[2] + 0.18), 0.08, 0.0, skin)
    primitive_cone_segment(bpy, collection, "goblin_ear_back", (head_center[0] - 0.11, 0.12, head_center[2] + 0.06), (head_center[0] + spec["ear_length"] * 0.70, 0.15, head_center[2] + 0.15), 0.06, 0.0, skin)
    face_y = -spec["head_size"] * 0.29
    primitive_cube(bpy, collection, "goblin_eye_front", (head_center[0] + 0.12, face_y - 0.01, head_center[2] + 0.05), (0.048, 0.010, 0.030), eyes, rotation=(0.0, math.radians(-4.0), math.radians(-14.0)), bevel=0.02, outline=0.0)
    primitive_cube(bpy, collection, "goblin_eye_back", (head_center[0] + 0.01, face_y - 0.01, head_center[2] + 0.02), (0.040, 0.010, 0.026), eyes, rotation=(0.0, math.radians(-4.0), math.radians(-12.0)), bevel=0.02, outline=0.0)
    primitive_cylinder_segment(bpy, collection, "goblin_brow_front", (head_center[0] + 0.06, face_y - 0.005, head_center[2] + 0.10), (head_center[0] + 0.15, face_y - 0.010, head_center[2] + 0.11), 0.012, dark, outline=0.0)
    primitive_cylinder_segment(bpy, collection, "goblin_brow_back", (head_center[0] - 0.03, face_y - 0.005, head_center[2] + 0.07), (head_center[0] + 0.04, face_y - 0.010, head_center[2] + 0.08), 0.011, dark, outline=0.0)
    primitive_cylinder_segment(bpy, collection, "goblin_mouth", (head_center[0] + 0.10, face_y - 0.008, head_center[2] - 0.06), (head_center[0] + 0.18, face_y - 0.010, head_center[2] - 0.08), 0.010, dark, outline=0.0)
    primitive_cube(bpy, collection, "goblin_tooth", (head_center[0] + 0.16, face_y - 0.016, head_center[2] - 0.08), (0.010, 0.006, 0.020), metal, bevel=0.003, outline=0.0)
    primitive_cube(bpy, collection, "goblin_waistcloth", (pelvis_center[0] + 0.12, 0.0, pelvis_center[2] + 0.02), (0.12, 0.14, 0.12), accent, rotation=(0.0, 0.0, math.radians(8)), bevel=0.03, outline=0.010)

    shoulder_front = (torso_center[0] + 0.05, -0.10, torso_center[2] + 0.08)
    shoulder_back = (torso_center[0] - 0.08, 0.10, torso_center[2] + 0.05)
    elbow_front = point_from(shoulder_front, spec["arm_length"], pose["arm_front"], 0.0)
    elbow_back = point_from(shoulder_back, spec["arm_length"] * 0.95, pose["arm_back"], 0.0)
    wrist_front = point_from(elbow_front, spec["forearm_length"], pose["forearm_front"], 0.0)
    wrist_back = point_from(elbow_back, spec["forearm_length"] * 0.95, pose["forearm_back"], 0.0)
    primitive_cylinder_segment(bpy, collection, "goblin_upperarm_front", shoulder_front, elbow_front, 0.048, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_upperarm_back", shoulder_back, elbow_back, 0.043, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_forearm_front", elbow_front, wrist_front, 0.040, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_forearm_back", elbow_back, wrist_back, 0.036, skin)
    primitive_uv_sphere(bpy, collection, "goblin_hand_front", wrist_front, (0.050, 0.042, 0.050), skin)
    primitive_uv_sphere(bpy, collection, "goblin_hand_back", wrist_back, (0.045, 0.038, 0.045), skin)
    add_goblin_weapon(bpy, collection, str(spec.get("held_item") or "spear"), wrist_front, pose["weapon_angle"], metal, accent)

    hip_front = (pelvis_center[0] + 0.03, -0.06, pelvis_center[2] - 0.10)
    hip_back = (pelvis_center[0] - 0.08, 0.06, pelvis_center[2] - 0.10)
    knee_front = point_from(hip_front, spec["leg_length"], pose["leg_front"], 0.0)
    knee_back = point_from(hip_back, spec["leg_length"], pose["leg_back"], 0.0)
    ankle_front = point_from(knee_front, spec["shin_length"], pose["shin_front"], 0.0)
    ankle_back = point_from(knee_back, spec["shin_length"], pose["shin_back"], 0.0)
    primitive_cylinder_segment(bpy, collection, "goblin_thigh_front", hip_front, knee_front, 0.050, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_thigh_back", hip_back, knee_back, 0.045, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_shin_front", knee_front, ankle_front, 0.041, skin)
    primitive_cylinder_segment(bpy, collection, "goblin_shin_back", knee_back, ankle_back, 0.037, skin)
    primitive_cube(bpy, collection, "goblin_foot_front", (ankle_front[0] + 0.12, ankle_front[1] - 0.01, ankle_front[2] - 0.05), (0.12, 0.06, 0.04), skin, bevel=0.03, outline=0.012)
    primitive_cube(bpy, collection, "goblin_foot_back", (ankle_back[0] + 0.10, ankle_back[1] + 0.01, ankle_back[2] - 0.05), (0.10, 0.055, 0.038), skin, bevel=0.03, outline=0.012)



def render_request(bpy, req: Dict[str, object], payload: Dict[str, object]) -> None:
    scene = bpy.context.scene
    scene.render.resolution_x = int(req["width"])
    scene.render.resolution_y = int(req["height"])
    target = payload["target"]
    spec = payload["spec"]
    # Remove existing character collections
    for collection in list(bpy.data.collections):
        if collection.name.startswith("Character_"):
            for obj in list(collection.objects):
                bpy.data.objects.remove(obj, do_unlink=True)
            bpy.data.collections.remove(collection)
    collection = create_collection(bpy, f"Character_{target}")
    if target == "robot":
        build_robot(bpy, collection, spec, req["animation"], int(req["frame_index"]), int(req["frame_count"]))
    elif target == "goblin":
        build_goblin(bpy, collection, spec, req["animation"], int(req["frame_index"]), int(req["frame_count"]))
    else:
        raise KeyError(target)
    scene.render.filepath = str(Path(req["out_path"]).resolve())
    bpy.ops.render.render(write_still=True)


def main() -> None:
    args = parse_args()
    payload = json.loads(Path(args.payload).read_text())
    bpy, _ = _blender_modules()
    first = payload["requests"][0]
    clear_scene(bpy)
    configure_scene(bpy, int(first["width"]), int(first["height"]), transparent=True)
    ensure_camera_and_lights(bpy, payload["spec"]["view"])
    for req in payload["requests"]:
        Path(req["out_path"]).parent.mkdir(parents=True, exist_ok=True)
        render_request(bpy, req, payload)


if __name__ == "__main__":
    main()
