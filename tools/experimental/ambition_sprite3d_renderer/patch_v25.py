from pathlib import Path
path = Path('gen3d_blender_lab/blender_backend/scene_builder.py')
text = path.read_text()
start = text.index('# BEGIN SIMPLIFIED_TARGET_OVERRIDES')
end = text.index('# END SIMPLIFIED_TARGET_OVERRIDES') + len('# END SIMPLIFIED_TARGET_OVERRIDES')
new_block = r'''# BEGIN SIMPLIFIED_TARGET_OVERRIDES
# v25 principled convergence pass.
#
# This block deliberately stops treating "side pose" as a generic rotation of a
# construction model.  Instead it defines a tiny view-coordinate convention that
# every side-pose builder follows:
#
#   +X = gameplay/facing direction
#   -Y = camera-visible surface
#   +Z = up
#
# All screens, eyes, belts, and cloth panels are placed with helpers that put
# them on the visible -Y surface of a core primitive.  This prevents the common
# regression where a screen or face exists but is buried inside the head/body.
# Robot and goblin use separate builders and separate pose roots; the only
# shared code is low-level primitive placement and view-surface helpers.


def _soften_form(obj, levels: int = 1, factor: float = 0.12):
    subsurf = obj.modifiers.new(name='Subsurf', type='SUBSURF')
    subsurf.levels = levels
    subsurf.render_levels = levels
    smooth = obj.modifiers.new(name='Smooth', type='SMOOTH')
    smooth.factor = factor
    smooth.iterations = 3
    return obj


def _visible_y(center_y: float, half_depth: float, lift: float = 0.010) -> float:
    """Y coordinate just in front of a primitive's camera-facing surface."""
    return float(center_y) - float(half_depth) - float(lift)


def _panel_cube(bpy, collection, name: str, center: Sequence[float], half_width: float, half_height: float, half_depth: float, mat, *, bevel: float = 0.012, outline: float = 0.0, rotation_z: float = 0.0):
    return primitive_cube(
        bpy, collection, name, center,
        (half_width, half_depth, half_height), mat,
        rotation=(0.0, 0.0, math.radians(rotation_z)), bevel=bevel, outline=outline,
    )


def _clean_materials_robot(bpy, spec: Dict[str, object], texture_paths: Dict[str, str] | None = None):
    texture_paths = texture_paths or {}
    return {
        'body': ensure_toon_material(bpy, 'SimpleRobotCream', '#E8E1D6', '#C8BEB2', texture_path=texture_paths.get('primary'), texture_mix=0.090, texture_scale=1.15),
        'dark': ensure_toon_material(bpy, 'SimpleRobotScreen', '#3F4751', '#222831', texture_path=texture_paths.get('dark'), texture_mix=0.070, texture_scale=1.25),
        'cyan': ensure_toon_material(bpy, 'SimpleRobotCyan', '#BDFBFF', '#58D8E7', emission_strength=0.38),
        'purple': ensure_toon_material(bpy, 'SimpleRobotPurple', '#D4C0EA', '#9E83BF', texture_path=texture_paths.get('accent'), texture_mix=0.060, texture_scale=1.05),
        'joint': ensure_toon_material(bpy, 'SimpleRobotJoint', '#BCB5AD', '#857E78', texture_path=texture_paths.get('metal'), texture_mix=0.055, texture_scale=1.10),
    }


def _clean_materials_goblin(bpy, spec: Dict[str, object], texture_paths: Dict[str, str] | None = None):
    texture_paths = texture_paths or {}
    return {
        'skin': ensure_toon_material(bpy, 'SimpleGoblinSkin', '#BDB6C7', '#8D849A', texture_path=texture_paths.get('skin'), texture_mix=0.090, texture_scale=0.95),
        'skin_dark': ensure_toon_material(bpy, 'SimpleGoblinSkinDark', '#78708A', '#554D63', texture_path=texture_paths.get('skin'), texture_mix=0.080, texture_scale=0.95),
        'eye': ensure_toon_material(bpy, 'SimpleGoblinEye', '#F0D9FF', '#B98BDF', emission_strength=0.18),
        'cloth': ensure_toon_material(bpy, 'SimpleGoblinCloth', '#DAC6EE', '#AD92C8', texture_path=texture_paths.get('cloth'), texture_mix=0.080, texture_scale=0.88),
        'belt': ensure_toon_material(bpy, 'SimpleGoblinBelt', '#D0BCA8', '#9A816D', texture_path=texture_paths.get('accent'), texture_mix=0.055, texture_scale=0.95),
        'metal': ensure_toon_material(bpy, 'SimpleGoblinMetal', '#CDC8D8', '#837B91', texture_path=texture_paths.get('metal'), texture_mix=0.055, texture_scale=1.05),
        'dark': ensure_toon_material(bpy, 'SimpleGoblinDark', '#4C4658', '#2E2937'),
    }


def _dagger_simple(bpy, collection, hand: Sequence[float], metal, accent, angle_deg: float = -62.0, prefix: str = 'dagger'):
    a = math.radians(angle_deg)
    ux, uz = math.cos(a), math.sin(a)
    base = (hand[0] - 0.016 * ux, hand[1], hand[2] - 0.016 * uz)
    guard = (hand[0] + 0.044 * ux, hand[1], hand[2] + 0.044 * uz)
    tip = (guard[0] + 0.155 * ux, guard[1], guard[2] + 0.155 * uz)
    primitive_cylinder_segment(bpy, collection, f'{prefix}_grip', base, guard, 0.010, accent, outline=0.003)
    primitive_cylinder_segment(bpy, collection, f'{prefix}_guard', (guard[0], hand[1] - 0.022, guard[2]), (guard[0], hand[1] + 0.022, guard[2]), 0.0045, accent, outline=0.002)
    primitive_cone_segment(bpy, collection, f'{prefix}_blade', guard, tip, 0.016, 0.002, metal, outline=0.004)


def _goblin_front_face(bpy, collection, center: Sequence[float], m, *, head_depth: float = 0.260):
    y = _visible_y(center[1], head_depth, 0.012)
    _panel_cube(bpy, collection, 'goblin_eye_socket_left', (center[0] - 0.070, y, center[2] + 0.030), 0.050, 0.036, 0.004, m['dark'], bevel=0.012)
    _panel_cube(bpy, collection, 'goblin_eye_socket_right', (center[0] + 0.070, y, center[2] + 0.030), 0.050, 0.036, 0.004, m['dark'], bevel=0.012)
    _panel_cube(bpy, collection, 'goblin_eye_left', (center[0] - 0.070, y - 0.006, center[2] + 0.030), 0.022, 0.016, 0.002, m['eye'], bevel=0.004)
    _panel_cube(bpy, collection, 'goblin_eye_right', (center[0] + 0.070, y - 0.006, center[2] + 0.030), 0.022, 0.016, 0.002, m['eye'], bevel=0.004)
    _panel_cube(bpy, collection, 'goblin_mouth', (center[0], y - 0.004, center[2] - 0.092), 0.036, 0.007, 0.002, m['dark'], bevel=0.002)
    _panel_cube(bpy, collection, 'goblin_tooth_left', (center[0] - 0.010, y - 0.006, center[2] - 0.102), 0.004, 0.008, 0.0015, m['metal'], bevel=0.001)
    _panel_cube(bpy, collection, 'goblin_tooth_right', (center[0] + 0.010, y - 0.006, center[2] - 0.102), 0.004, 0.008, 0.0015, m['metal'], bevel=0.001)


def _goblin_side_face(bpy, collection, center: Sequence[float], m, *, head_depth: float = 0.238):
    # Side pose is really a readable 3/4 pose.  The head is offset to the right,
    # but the facial controls live on a camera-visible plane in front of it.
    y = _visible_y(center[1], head_depth, 0.014)
    cx = center[0] + 0.030
    cz = center[2] + 0.005
    # A dark rounded face mask is more intentional than a pale rectangular sticker.
    _panel_cube(bpy, collection, 'goblin_face_mask', (cx, y, cz), 0.112, 0.084, 0.006, m['dark'], bevel=0.028, outline=0.002, rotation_z=-3.0)
    _panel_cube(bpy, collection, 'goblin_eye_main', (cx + 0.034, y - 0.006, cz + 0.022), 0.024, 0.015, 0.002, m['eye'], bevel=0.004, rotation_z=-3.0)
    _panel_cube(bpy, collection, 'goblin_eye_far', (cx - 0.032, y - 0.005, cz + 0.020), 0.016, 0.012, 0.002, m['eye'], bevel=0.003, rotation_z=-3.0)
    primitive_cone_segment(bpy, collection, 'goblin_nose', (cx + 0.060, y - 0.008, cz - 0.004), (cx + 0.094, y - 0.020, cz - 0.010), 0.010, 0.0, m['skin_dark'], outline=0.0)
    _panel_cube(bpy, collection, 'goblin_mouth', (cx + 0.014, y - 0.007, cz - 0.045), 0.028, 0.006, 0.002, m['eye'], bevel=0.001, rotation_z=-3.0)
    _panel_cube(bpy, collection, 'goblin_tooth_left', (cx + 0.008, y - 0.009, cz - 0.055), 0.004, 0.007, 0.0015, m['metal'], bevel=0.001)
    _panel_cube(bpy, collection, 'goblin_tooth_right', (cx + 0.022, y - 0.009, cz - 0.055), 0.004, 0.007, 0.0015, m['metal'], bevel=0.001)


def _robot_front_face(bpy, collection, center: Sequence[float], m, *, head_depth: float = 0.260):
    y = _visible_y(center[1], head_depth, 0.012)
    _panel_cube(bpy, collection, 'robot_face_screen', (center[0], y, center[2] + 0.008), 0.180, 0.100, 0.005, m['dark'], bevel=0.030)
    primitive_uv_sphere(bpy, collection, 'robot_eye_left', (center[0] - 0.050, y - 0.006, center[2] + 0.010), (0.017, 0.003, 0.036), m['cyan'], outline=0.0)
    primitive_uv_sphere(bpy, collection, 'robot_eye_right', (center[0] + 0.050, y - 0.006, center[2] + 0.010), (0.017, 0.003, 0.036), m['cyan'], outline=0.0)
    _panel_cube(bpy, collection, 'robot_mouth', (center[0], y - 0.007, center[2] - 0.044), 0.040, 0.006, 0.0015, m['joint'], bevel=0.002)


def _robot_side_face(bpy, collection, center: Sequence[float], m, *, head_depth: float = 0.260):
    y = _visible_y(center[1], head_depth, 0.012)
    cx = center[0] + 0.040
    _panel_cube(bpy, collection, 'robot_face_screen', (cx, y, center[2] + 0.008), 0.150, 0.092, 0.005, m['dark'], bevel=0.028)
    primitive_uv_sphere(bpy, collection, 'robot_eye_left', (cx - 0.044, y - 0.006, center[2] + 0.012), (0.015, 0.003, 0.032), m['cyan'], outline=0.0)
    primitive_uv_sphere(bpy, collection, 'robot_eye_right', (cx + 0.044, y - 0.006, center[2] + 0.012), (0.015, 0.003, 0.032), m['cyan'], outline=0.0)
    _panel_cube(bpy, collection, 'robot_mouth', (cx, y - 0.007, center[2] - 0.042), 0.034, 0.006, 0.0015, m['joint'], bevel=0.002)


def build_goblin_construction(bpy, collection, spec: Dict[str, object], texture_paths: Dict[str, str] | None = None):
    m = _clean_materials_goblin(bpy, spec, texture_paths)
    torso = (0.0, 0.0, 0.96)
    head = (0.0, 0.0, 1.47)
    body = primitive_uv_sphere(bpy, collection, 'goblin_body', torso, (0.215, 0.168, 0.270), m['skin_dark'], outline=0.015)
    head_obj = primitive_uv_sphere(bpy, collection, 'goblin_head', head, (0.338, 0.260, 0.292), m['skin'], outline=0.015)
    _soften_form(body, levels=1, factor=0.12)
    _soften_form(head_obj, levels=1, factor=0.12)
    primitive_cone_segment(bpy, collection, 'goblin_ear_left', (-0.198, -0.004, 1.530), (-0.485, -0.004, 1.595), 0.050, 0.000, m['skin'], outline=0.007)
    primitive_cone_segment(bpy, collection, 'goblin_ear_right', (0.198, -0.004, 1.530), (0.485, -0.004, 1.595), 0.050, 0.000, m['skin'], outline=0.007)
    _goblin_front_face(bpy, collection, head, m, head_depth=0.260)
    _panel_cube(bpy, collection, 'goblin_belt', (0.0, _visible_y(torso[1], 0.168, 0.014), 0.690), 0.155, 0.020, 0.004, m['belt'], bevel=0.006, outline=0.003)
    _panel_cube(bpy, collection, 'goblin_loincloth', (0.0, _visible_y(torso[1], 0.168, 0.020), 0.590), 0.050, 0.064, 0.004, m['cloth'], bevel=0.007, outline=0.003)
    for side, sx in (('l', -0.185), ('r', 0.185)):
        shoulder = (sx, 0.0, 1.00)
        elbow = (sx * 1.08, 0.0, 0.82)
        hand = (sx * 1.10, 0.0, 0.610)
        primitive_cylinder_segment(bpy, collection, f'goblin_upperarm_{side}', shoulder, elbow, 0.036, m['skin'], outline=0.008)
        primitive_cylinder_segment(bpy, collection, f'goblin_forearm_{side}', elbow, hand, 0.032, m['skin'], outline=0.008)
        primitive_uv_sphere(bpy, collection, f'goblin_hand_{side}', hand, (0.035, 0.030, 0.035), m['skin'], outline=0.006)
    for side, sx in (('l', -0.088), ('r', 0.088)):
        hip = (sx, 0.0, 0.638)
        knee = (sx, 0.0, 0.378)
        ankle = (sx, 0.0, 0.144)
        primitive_cylinder_segment(bpy, collection, f'goblin_thigh_{side}', hip, knee, 0.040, m['skin'], outline=0.008)
        primitive_cylinder_segment(bpy, collection, f'goblin_shin_{side}', knee, ankle, 0.036, m['skin'], outline=0.008)
        primitive_cube(bpy, collection, f'goblin_foot_{side}', (sx + (0.022 if side == 'r' else -0.022), -0.005, 0.058), (0.074, 0.048, 0.030), m['skin'], bevel=0.018, outline=0.007)
    _dagger_simple(bpy, collection, (0.198, 0.0, 0.606), m['metal'], m['cloth'], angle_deg=-64.0, prefix='goblin_dagger')


def build_goblin(bpy, collection, spec: Dict[str, object], animation: str, index: int, frame_count: int, texture_paths: Dict[str, str] | None = None):
    m = _clean_materials_goblin(bpy, spec, texture_paths)
    bob = 0.006 * math.sin(2.0 * math.pi * (index / max(1, frame_count))) if animation in {'idle', 'walk', 'run'} else 0.0
    torso = (0.02, 0.0, 0.94 + bob)
    head = (0.115, 0.0, 1.42 + bob)
    body = primitive_uv_sphere(bpy, collection, 'goblin_body', torso, (0.172, 0.138, 0.248), m['skin_dark'], outline=0.015)
    head_obj = primitive_uv_sphere(bpy, collection, 'goblin_head', head, (0.306, 0.238, 0.278), m['skin'], outline=0.015)
    _soften_form(body, levels=2, factor=0.16)
    _soften_form(head_obj, levels=2, factor=0.16)
    primitive_cone_segment(bpy, collection, 'goblin_ear_front', (head[0] + 0.132, -0.004, head[2] + 0.040), (head[0] + 0.342, -0.008, head[2] + 0.088), 0.042, 0.000, m['skin'], outline=0.007)
    primitive_cone_segment(bpy, collection, 'goblin_ear_back', (head[0] - 0.138, 0.012, head[2] + 0.034), (head[0] - 0.020, 0.016, head[2] + 0.058), 0.018, 0.000, m['skin'], outline=0.004)
    _goblin_side_face(bpy, collection, head, m, head_depth=0.238)
    _panel_cube(bpy, collection, 'goblin_belt', (torso[0] + 0.006, _visible_y(torso[1], 0.138, 0.014), 0.630 + bob), 0.112, 0.018, 0.004, m['belt'], bevel=0.006, outline=0.003)
    _panel_cube(bpy, collection, 'goblin_loincloth', (torso[0] + 0.026, _visible_y(torso[1], 0.138, 0.020), 0.545 + bob), 0.026, 0.044, 0.004, m['cloth'], bevel=0.006, outline=0.003)
    shoulder_front = (torso[0] + 0.112, -0.006, torso[2] + 0.040)
    elbow_front = (torso[0] + 0.145, -0.006, torso[2] - 0.078)
    hand_front = (torso[0] + 0.160, -0.006, torso[2] - 0.202)
    shoulder_back = (torso[0] - 0.078, 0.012, torso[2] + 0.038)
    elbow_back = (torso[0] - 0.100, 0.014, torso[2] - 0.064)
    hand_back = (torso[0] - 0.102, 0.014, torso[2] - 0.176)
    primitive_cylinder_segment(bpy, collection, 'goblin_upperarm_front', shoulder_front, elbow_front, 0.032, m['skin'], outline=0.008)
    primitive_cylinder_segment(bpy, collection, 'goblin_forearm_front', elbow_front, hand_front, 0.029, m['skin'], outline=0.008)
    primitive_uv_sphere(bpy, collection, 'goblin_hand_front', hand_front, (0.032, 0.026, 0.032), m['skin'], outline=0.006)
    primitive_cylinder_segment(bpy, collection, 'goblin_upperarm_back', shoulder_back, elbow_back, 0.024, m['skin'], outline=0.005)
    primitive_cylinder_segment(bpy, collection, 'goblin_forearm_back', elbow_back, hand_back, 0.021, m['skin'], outline=0.005)
    primitive_uv_sphere(bpy, collection, 'goblin_hand_back', hand_back, (0.024, 0.020, 0.024), m['skin'], outline=0.004)
    _dagger_simple(bpy, collection, hand_front, m['metal'], m['cloth'], angle_deg=-70.0, prefix='goblin_dagger')
    hip_front = (torso[0] + 0.044, -0.008, 0.596 + bob)
    knee_front = (torso[0] + 0.080, -0.008, 0.376 + bob)
    ankle_front = (torso[0] + 0.104, -0.008, 0.156 + bob)
    hip_back = (torso[0] - 0.044, 0.010, 0.602 + bob)
    knee_back = (torso[0] - 0.052, 0.010, 0.388 + bob)
    ankle_back = (torso[0] - 0.056, 0.010, 0.164 + bob)
    primitive_cylinder_segment(bpy, collection, 'goblin_thigh_front', hip_front, knee_front, 0.036, m['skin'], outline=0.008)
    primitive_cylinder_segment(bpy, collection, 'goblin_shin_front', knee_front, ankle_front, 0.032, m['skin'], outline=0.008)
    primitive_cube(bpy, collection, 'goblin_foot_front', (ankle_front[0] + 0.016, -0.008, 0.058), (0.064, 0.040, 0.028), m['skin'], bevel=0.018, outline=0.007)
    primitive_cylinder_segment(bpy, collection, 'goblin_thigh_back', hip_back, knee_back, 0.028, m['skin'], outline=0.005)
    primitive_cylinder_segment(bpy, collection, 'goblin_shin_back', knee_back, ankle_back, 0.025, m['skin'], outline=0.005)
    primitive_cube(bpy, collection, 'goblin_foot_back', (ankle_back[0] + 0.008, 0.008, 0.056), (0.048, 0.032, 0.024), m['skin'], bevel=0.014, outline=0.004)


def build_robot_construction(bpy, collection, spec: Dict[str, object], texture_paths: Dict[str, str] | None = None):
    m = _clean_materials_robot(bpy, spec, texture_paths)
    head = (0.0, 0.0, 1.56)
    torso = (0.0, 0.0, 1.00)
    head_obj = primitive_cube(bpy, collection, 'robot_head', head, (0.340, 0.260, 0.248), m['body'], bevel=0.110, outline=0.012)
    torso_obj = primitive_cube(bpy, collection, 'robot_torso', torso, (0.198, 0.156, 0.236), m['body'], bevel=0.070, outline=0.012)
    _soften_form(head_obj, levels=1, factor=0.05)
    _soften_form(torso_obj, levels=1, factor=0.04)
    _robot_front_face(bpy, collection, head, m, head_depth=0.260)
    primitive_cube(bpy, collection, 'robot_ear_left', (-0.362, 0.000, 1.555), (0.026, 0.068, 0.086), m['purple'], bevel=0.016, outline=0.004)
    primitive_cube(bpy, collection, 'robot_ear_right', (0.362, 0.000, 1.555), (0.026, 0.068, 0.086), m['purple'], bevel=0.016, outline=0.004)
    primitive_cylinder_segment(bpy, collection, 'robot_antenna_stem', (0.0, 0.0, 1.812), (0.0, 0.0, 1.986), 0.010, m['joint'], outline=0.0)
    primitive_uv_sphere(bpy, collection, 'robot_antenna_tip', (0.0, 0.0, 2.048), (0.040, 0.040, 0.040), m['purple'], outline=0.003)
    _panel_cube(bpy, collection, 'robot_chest_screen', (0.0, _visible_y(torso[1], 0.156, 0.012), 1.006), 0.056, 0.066, 0.004, m['dark'], bevel=0.014)
    _panel_cube(bpy, collection, 'robot_chest_core', (0.0, _visible_y(torso[1], 0.156, 0.018), 1.006), 0.032, 0.043, 0.002, m['cyan'], bevel=0.008)
    for side, sx in (('l', -0.235), ('r', 0.235)):
        shoulder = (sx, 0.0, 1.05)
        elbow = (sx * 1.07, 0.0, 0.845)
        hand = (sx * 1.07, 0.0, 0.665)
        primitive_cylinder_segment(bpy, collection, f'robot_upperarm_{side}', shoulder, elbow, 0.040, m['body'], outline=0.008)
        primitive_cylinder_segment(bpy, collection, f'robot_forearm_{side}', elbow, hand, 0.036, m['body'], outline=0.008)
        primitive_uv_sphere(bpy, collection, f'robot_hand_{side}', hand, (0.044, 0.038, 0.044), m['body'], outline=0.005)
    for side, sx in (('l', -0.090), ('r', 0.090)):
        hip = (sx, 0.0, 0.732)
        knee = (sx, 0.0, 0.448)
        ankle = (sx, 0.0, 0.164)
        primitive_cylinder_segment(bpy, collection, f'robot_thigh_{side}', hip, knee, 0.042, m['body'], outline=0.008)
        primitive_cylinder_segment(bpy, collection, f'robot_shin_{side}', knee, ankle, 0.038, m['body'], outline=0.008)
        primitive_cube(bpy, collection, f'robot_foot_{side}', (sx + (0.020 if side == 'r' else -0.020), -0.006, 0.058), (0.084, 0.054, 0.032), m['body'], bevel=0.018, outline=0.006)


def build_robot(bpy, collection, spec: Dict[str, object], animation: str, index: int, frame_count: int, texture_paths: Dict[str, str] | None = None):
    m = _clean_materials_robot(bpy, spec, texture_paths)
    bob = 0.006 * math.sin(2.0 * math.pi * (index / max(1, frame_count))) if animation in {'idle', 'walk', 'run'} else 0.0
    head = (0.06, 0.0, 1.56 + bob)
    torso = (0.00, 0.0, 1.00 + bob)
    head_obj = primitive_cube(bpy, collection, 'robot_head', head, (0.340, 0.260, 0.248), m['body'], bevel=0.110, outline=0.012)
    torso_obj = primitive_cube(bpy, collection, 'robot_torso', torso, (0.198, 0.156, 0.236), m['body'], bevel=0.070, outline=0.012)
    _soften_form(head_obj, levels=1, factor=0.05)
    _soften_form(torso_obj, levels=1, factor=0.04)
    _robot_side_face(bpy, collection, head, m, head_depth=0.260)
    primitive_cube(bpy, collection, 'robot_side_ear_back', (head[0] - 0.240, 0.040, head[2]), (0.026, 0.060, 0.080), m['purple'], bevel=0.016, outline=0.004)
    primitive_cylinder_segment(bpy, collection, 'robot_antenna_stem', (head[0], 0.0, 1.812 + bob), (head[0], 0.0, 1.986 + bob), 0.010, m['joint'], outline=0.0)
    primitive_uv_sphere(bpy, collection, 'robot_antenna_tip', (head[0], 0.0, 2.048 + bob), (0.040, 0.040, 0.040), m['purple'], outline=0.003)
    _panel_cube(bpy, collection, 'robot_chest_screen', (torso[0] + 0.040, _visible_y(torso[1], 0.156, 0.012), torso[2] + 0.014), 0.050, 0.060, 0.004, m['dark'], bevel=0.012)
    _panel_cube(bpy, collection, 'robot_chest_core', (torso[0] + 0.040, _visible_y(torso[1], 0.156, 0.018), torso[2] + 0.014), 0.028, 0.038, 0.002, m['cyan'], bevel=0.006)
    shoulder_front = (torso[0] + 0.150, -0.006, torso[2] + 0.072)
    elbow_front = (torso[0] + 0.174, -0.006, torso[2] - 0.074)
    hand_front = (torso[0] + 0.172, -0.006, torso[2] - 0.214)
    shoulder_back = (torso[0] - 0.105, 0.010, torso[2] + 0.064)
    elbow_back = (torso[0] - 0.126, 0.010, torso[2] - 0.060)
    hand_back = (torso[0] - 0.124, 0.010, torso[2] - 0.188)
    primitive_cylinder_segment(bpy, collection, 'robot_upperarm_front', shoulder_front, elbow_front, 0.037, m['body'], outline=0.008)
    primitive_cylinder_segment(bpy, collection, 'robot_forearm_front', elbow_front, hand_front, 0.033, m['body'], outline=0.008)
    primitive_uv_sphere(bpy, collection, 'robot_hand_front', hand_front, (0.041, 0.035, 0.041), m['body'], outline=0.005)
    primitive_cylinder_segment(bpy, collection, 'robot_upperarm_back', shoulder_back, elbow_back, 0.029, m['body'], outline=0.005)
    primitive_cylinder_segment(bpy, collection, 'robot_forearm_back', elbow_back, hand_back, 0.026, m['body'], outline=0.005)
    primitive_uv_sphere(bpy, collection, 'robot_hand_back', hand_back, (0.030, 0.026, 0.030), m['body'], outline=0.004)
    hip_front = (torso[0] + 0.064, -0.006, 0.732 + bob)
    knee_front = (torso[0] + 0.116, -0.006, 0.470 + bob)
    ankle_front = (torso[0] + 0.150, -0.006, 0.168 + bob)
    hip_back = (torso[0] - 0.054, 0.008, 0.732 + bob)
    knee_back = (torso[0] - 0.066, 0.008, 0.468 + bob)
    ankle_back = (torso[0] - 0.070, 0.008, 0.170 + bob)
    primitive_cylinder_segment(bpy, collection, 'robot_thigh_front', hip_front, knee_front, 0.041, m['body'], outline=0.008)
    primitive_cylinder_segment(bpy, collection, 'robot_shin_front', knee_front, ankle_front, 0.037, m['body'], outline=0.008)
    primitive_cube(bpy, collection, 'robot_foot_front', (ankle_front[0] + 0.030, -0.006, 0.058), (0.082, 0.052, 0.031), m['body'], bevel=0.018, outline=0.006)
    primitive_cylinder_segment(bpy, collection, 'robot_thigh_back', hip_back, knee_back, 0.033, m['body'], outline=0.005)
    primitive_cylinder_segment(bpy, collection, 'robot_shin_back', knee_back, ankle_back, 0.030, m['body'], outline=0.005)
    primitive_cube(bpy, collection, 'robot_foot_back', (ankle_back[0] + 0.016, 0.006, 0.056), (0.060, 0.041, 0.027), m['body'], bevel=0.014, outline=0.004)
# END SIMPLIFIED_TARGET_OVERRIDES'''
text = text[:start] + new_block + text[end:]
# Render-request callers remain character-specific, but their roots now apply zero rotation.
path.write_text(text)
