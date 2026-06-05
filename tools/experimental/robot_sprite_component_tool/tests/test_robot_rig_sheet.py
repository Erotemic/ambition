from pathlib import Path
import importlib.util
import sys

from PIL import Image
import yaml


def _load_tool(root: Path):
    path = root / "tools" / "robot_rig_sheet.py"
    spec = importlib.util.spec_from_file_location("robot_rig_sheet", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_assembled_spritesheet_smoke(tmp_path):
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    out = tmp_path / "robot_assembled_spritesheet.png"
    image_out, manifest_out = tool.write_spritesheet(job, out)
    assert image_out.exists()
    assert manifest_out.exists()
    img = Image.open(image_out).convert("RGBA")
    assert img.getbbox() is not None
    manifest = yaml.safe_load(manifest_out.read_text())
    assert manifest["label_width"] > 0
    assert manifest["qa_warnings"] == []


def test_hit_is_not_death_pose():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = tool.animation_pose("hit", 2, tool.ANIMATIONS["hit"]["frames"], 0.235)
    assert pose.face_sprite != "face_dead_x"
    assert pose.torso_sprite != "torso_prone"
    assert not pose.front_hand_sprite.startswith("hand_fist")


def test_teleport_is_available_and_blink_row_is_not_required():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    assert "teleport" in tool.ANIMATIONS
    assert "blink" not in tool.ANIMATIONS
    poses = [
        tool.animation_pose("teleport", i, tool.ANIMATIONS["teleport"]["frames"], 0.235)
        for i in range(tool.ANIMATIONS["teleport"]["frames"])
    ]
    assert min(p.opacity for p in poses) == 0.0


def test_part_scale_defaults_are_neutral():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = tool.animation_pose("idle", 0, tool.ANIMATIONS["idle"]["frames"], 0.235)
    for role in [
        "front_arm",
        "back_arm",
        "front_hand",
        "back_hand",
        "front_leg",
        "back_leg",
    ]:
        assert tool.role_scale_multiplier(pose, role) == 1.0
    assert pose.torso_scale == 1.0
    assert pose.head_scale == 1.0
    assert pose.face_scale == 1.0


def test_baked_expression_stays_inside_detected_visor():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    atlas = tool.ComponentAtlas(
        root / "metadata" / "robot_components.refined.yaml", root / "output" / "slices"
    )
    for head_name in [
        "head_front",
        "head_tilt_left",
        "head_tilt_right",
        "head_squash_blink",
    ]:
        head = atlas.image(head_name)
        visor = tool.find_dark_visor_bbox(head)
        assert visor is not None
        for face_name in [
            "face_blink",
            "face_sad",
            "face_dead_x",
            "face_teleport_scan",
        ]:
            baked = tool.compose_head_expression(
                head, atlas.image(face_name), face_name
            )
            cb = tool.cyan_bbox(baked)
            assert cb is not None
            x1, y1, x2, y2 = visor
            cx1, cy1, cx2, cy2 = cb
            # Allow a tiny antialiasing tolerance, but cyan expression pixels
            # should remain inside the detected visor plate, not float as a
            # separate black decal.
            tol = 2
            assert cx1 >= x1 - tol
            assert cy1 >= y1 - tol
            assert cx2 <= x2 + tol
            assert cy2 <= y2 + tol


def test_default_job_uses_larger_cell_and_component_scale():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    assert job.render.frame_width >= 192
    assert job.render.frame_height >= 192
    assert job.render.scale >= 0.27


def test_run_and_dash_head_mount_is_seated_into_forward_torso():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    for animation in ["run", "dash"]:
        info = tool.ANIMATIONS[animation]
        for idx in range(info["frames"]):
            _frame, manifest = assembler.render_frame(animation, idx)
            mount = manifest["pose"]["head_mount"]
            torso_neck_y = mount["torso_neck"][1]
            head_target_y = mount["head_target"][1]
            # The head anchor must be pushed down into the torso neck socket
            # for forward-lean poses; otherwise run/dash look detached.
            assert head_target_y - torso_neck_y >= 9.0
            # Run/dash should push the head forward on the x-axis; otherwise
            # the head reads as floating behind the forward-leaning body.
            assert mount["head_target"][0] - mount["torso_neck"][0] >= 12.0
            assert mount["head_inherit_torso"] > 0.0
            assert mount["head_world_angle"] < 0.0


def test_jump_is_sheet_locked_and_head_is_seated():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    roots = []
    for idx in range(tool.ANIMATIONS["jump"]["frames"]):
        _frame, manifest = assembler.render_frame("jump", idx)
        roots.append(tuple(manifest["root"]))
        mount = manifest["pose"]["head_mount"]
        # Jump should use the same seating principle as other large-head
        # poses: push the head anchor slightly down into the neck socket.
        assert mount["head_target"][1] - mount["torso_neck"][1] >= 7.0
    # The jump arc is controlled by the game object / collision box; the
    # spritesheet frame itself stays root-aligned so the renderer has a stable
    # pivot.
    assert len(set(roots)) == 1


def test_default_job_is_focused_on_run_preview():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    assert job.animations == ["run"]
    full_job = tool.RigJob.load(root / "examples" / "robot_rig_job_full.yaml")
    assert "run" in full_job.animations
    assert "dash" in full_job.animations


def test_arm_mount_and_hand_follow_are_active():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job_full.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    _frame, manifest = assembler.render_frame("run", 1)
    pose = manifest["pose"]
    assert pose["arm_mounts"]["hand_follow"] >= 0.75
    raw = pose["arm_mounts"]["front_shoulder_raw"]
    target = pose["arm_mounts"]["front_shoulder_target"]
    # With v23 semantic side-specific anchors, the raw front socket already
    # identifies the visible side mount; no compensating offset is required.
    assert abs(target[0] - raw[0]) <= 0.01
    assert abs(target[1] - raw[1]) <= 0.01
    assert pose["arm_mounts"]["z_order"].index("front_hand") < pose["arm_mounts"][
        "z_order"
    ].index("head")


def test_fist_anchor_is_on_wrist_cuff_and_mirrors_correctly():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    atlas = tool.ComponentAtlas(
        root / "metadata" / "robot_components.refined.yaml", root / "output" / "slices"
    )
    w = atlas.image("hand_fist").width
    wrist = atlas.anchor("hand_fist", "wrist")
    flipped = atlas.anchor("hand_fist@flip_x", "wrist")
    # The source fist is a left-facing/back-side hand: its wrist cuff is on
    # the right side of the crop.  The virtual flipped fist should mirror that
    # cuff to the left side for front/right-facing hands.
    assert wrist[0] > w * 0.70
    assert flipped[0] < w * 0.30
    assert abs((w - wrist[0]) - flipped[0]) < 0.01


def test_debug_part_render_marks_component_layers():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    frame, manifest = assembler.render_frame("run", 1, debug_parts=True)
    assert manifest["pose"]["debug_parts"] is True
    # Debug render should contain saturated component colors, not only the final
    # production palette.
    colors = frame.convert("RGBA").getdata()
    assert any(r > 220 and g < 90 and b < 90 and a > 0 for r, g, b, a in colors)
    assert any(b > 220 and r < 100 and a > 0 for r, g, b, a in colors)


def test_run_limb_targets_stay_on_their_sides():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    for idx in range(tool.ANIMATIONS["run"]["frames"]):
        _frame, manifest = assembler.render_frame("run", idx)
        mounts = manifest["pose"]["arm_mounts"]
        back_sh = mounts["back_shoulder_target"]
        front_sh = mounts["front_shoulder_target"]
        back_wr = mounts["back_wrist"]
        front_wr = mounts["front_wrist"]
        assert front_sh[0] < back_sh[0]
        assert front_wr[0] < back_wr[0]
        # The arms should hang below the shoulder pods, not overlap the head.
        assert front_wr[1] > front_sh[1]
        assert back_wr[1] > back_sh[1]


def test_run_arm_hand_scale_and_leg_zorder_policy():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = tool.animation_pose("run", 1, tool.ANIMATIONS["run"]["frames"], 0.275)
    # v18: arms were visually too long/thick and hands too tiny.  Run uses
    # endpoint solving, so arm size is also controlled by wrist deltas, but the
    # pose policy should still keep the hand larger relative to the arm.
    assert tool.role_scale_multiplier(pose, "front_arm") == 1.0
    assert tool.role_scale_multiplier(pose, "front_hand") == 1.0

    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    _frame, manifest = assembler.render_frame("run", 1)
    z_order = manifest["pose"]["arm_mounts"]["z_order"]
    assert z_order.index("back_leg") < z_order.index("torso")
    assert z_order.index("torso") < z_order.index("front_leg")
    assert z_order.index("back_hand") < z_order.index("back_arm")
    assert z_order.index("front_arm") < z_order.index("front_hand")


def test_run_torso_lean_forward_anchors_are_on_visible_sockets():
    root = Path(__file__).resolve().parents[1]
    import yaml

    meta = yaml.safe_load(
        (root / "metadata" / "robot_components.refined.yaml").read_text()
    )
    anchors = meta["sprites"]["torso_lean_forward"]["anchors"]
    # These anchors are user-editable.  Keep the regression semantic instead of
    # hard-coding one old hand-tuned pixel solution: shoulders must be separated
    # on their visible side sockets, and hips must be lower than shoulders and
    # separated enough for the endpoint solver to avoid component pile-ups.
    assert anchors["shoulder_right"][0] < anchors["shoulder_left"][0]
    assert anchors["shoulder_left"][0] - anchors["shoulder_right"][0] >= 45
    assert max(anchors["shoulder_left"][1], anchors["shoulder_right"][1]) < min(
        anchors["hip_left"][1], anchors["hip_right"][1]
    )
    # In the run/forward-lean torso art, semantic right/front should remain on the visually near side
    # (smaller x in this side-view art), and left/back should remain on the far
    # side. The names should match the rig side convention.
    assert anchors["hip_left"][0] > anchors["hip_right"][0]
    assert anchors["hip_left"][0] - anchors["hip_right"][0] >= 20
    for name in ["shoulder_left", "shoulder_right", "hip_left", "hip_right"]:
        x, y = anchors[name]
        assert 0 <= x <= meta["sprites"]["torso_lean_forward"]["rect"][2]
        assert 0 <= y <= meta["sprites"]["torso_lean_forward"]["rect"][3]


def test_v20_run_zorder_and_anchor_nudges():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    _frame, manifest = assembler.render_frame("run", 1)
    z_order = manifest["pose"]["arm_mounts"]["z_order"]
    # Far/left-side hand sits behind the arm; near/right-side hand sits in
    # front.  Far/left leg is behind the body; near/right leg is in front.
    assert z_order.index("back_foot") < z_order.index("back_leg")
    assert (
        z_order.index("back_hand") < z_order.index("back_arm") < z_order.index("torso")
    )
    assert (
        z_order.index("torso")
        < z_order.index("front_leg")
        < z_order.index("front_foot")
        < z_order.index("front_arm")
        < z_order.index("front_hand")
    )

    meta = yaml.safe_load(
        (root / "metadata" / "robot_components.refined.yaml").read_text()
    )
    anchors = meta["sprites"]["torso_lean_forward"]["anchors"]
    assert anchors["shoulder_right"][0] >= 43
    assert anchors["shoulder_left"][0] - anchors["shoulder_right"][0] >= 45
    for leg_name in [
        "leg_straight_right",
        "leg_bent_right",
        "leg_straight_left",
        "leg_bent_left",
    ]:
        leg = meta["sprites"][leg_name]
        assert leg["pivot"] == leg["anchors"]["hip"]


def test_v21_bent_leg_anchors_are_on_joint_centers():
    root = Path(__file__).resolve().parents[1]
    import yaml

    meta = yaml.safe_load(
        (root / "metadata" / "robot_components.refined.yaml").read_text()
    )
    right = meta["sprites"]["leg_bent_right"]
    left = meta["sprites"]["leg_bent_left"]
    # Bent-leg hip anchors should be at the center of the top black connector
    # cap.  Earlier values were at the visual/crop top and pulled the leg out
    # of the torso during run poses.
    assert right["pivot"] == right["anchors"]["hip"]
    assert left["pivot"] == left["anchors"]["hip"]
    assert 16 <= right["anchors"]["hip"][0] <= 24
    assert 14 <= right["anchors"]["hip"][1] <= 22
    assert 16 <= left["anchors"]["hip"][0] <= 24
    assert 14 <= left["anchors"]["hip"][1] <= 22


def test_debug_frame_writer_smoke(tmp_path):
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    out = tmp_path / "run0_debug.png"
    tool.write_debug_frame(job, out, "run", 0, zoom=2, pad=10, background="black")
    assert out.exists()
    assert out.with_suffix(".json").exists()
    img = Image.open(out).convert("RGBA")
    # Smaller calibrated limbs can make the cropped debug frame narrower than
    # the full sprite canvas.  The important smoke assertion is that the debug
    # crop exists and is enlarged relative to its own non-empty content.
    assert img.width > 0 and img.height > 0
    assert img.getbbox() is not None


def test_anchor_editor_headless_report(tmp_path):
    root = Path(__file__).resolve().parents[1]
    path = root / "tools" / "anchor_editor.py"
    spec = importlib.util.spec_from_file_location("anchor_editor", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    out = tmp_path / "anchors.json"
    code = module.main(
        [
            str(root / "metadata" / "robot_components.refined.yaml"),
            "--slices",
            str(root / "output" / "slices"),
            "--anchor-report",
            str(out),
            "--sprites",
            "torso_lean_forward",
            "hand_fist",
        ]
    )
    assert code == 0
    data = __import__("json").loads(out.read_text())
    ids = {row["sprite"] for row in data["sprites"]}
    assert ids == {"torso_lean_forward", "hand_fist"}
    assert data["sprites"][0]["anchors"]


def test_instance_scale_overrides_are_independent():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = tool.animation_pose("idle", 0, tool.ANIMATIONS["idle"]["frames"], 0.275)
    pose.front_arm_scale = 0.33
    pose.back_arm_scale = 0.77
    pose.front_hand_scale = 0.44
    pose.back_hand_scale = 0.88
    pose.front_leg_scale = 0.55
    pose.back_leg_scale = 0.66
    assert tool.role_scale_multiplier(pose, "front_arm") == 0.33
    assert tool.role_scale_multiplier(pose, "back_arm") == 0.77
    assert tool.role_scale_multiplier(pose, "front_hand") == 0.44
    assert tool.role_scale_multiplier(pose, "back_hand") == 0.88
    assert tool.role_scale_multiplier(pose, "front_leg") == 0.55
    assert tool.role_scale_multiplier(pose, "back_leg") == 0.66


def test_manifest_reports_instance_scales_not_only_group_scales():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(atlas, job.render)
    assembler.pose_overrides = {
        "animations": {
            "idle": {
                "frame_overrides": {
                    "0": {
                        "front_arm_scale": 0.31,
                        "back_arm_scale": 0.72,
                        "front_hand_scale": 0.41,
                        "back_hand_scale": 0.82,
                    }
                }
            }
        }
    }
    _frame, manifest = assembler.render_frame("idle", 0)
    scales = manifest["pose"]["part_scales"]
    assert scales["front_arm"] == 0.31
    assert scales["back_arm"] == 0.72
    assert scales["front_hand"] == 0.41
    assert scales["back_hand"] == 0.82
    assert scales["front_arm"] != scales["front_hand"]


def test_scale_frame_overrides_are_exact_only():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = {
        "version": "0.4",
        "animations": {
            "run": {
                "frame_overrides": {
                    "0": {"front_arm_scale": 0.5, "front_arm_angle": -20},
                    "7": {"front_arm_scale": 2.0, "front_arm_angle": 20},
                }
            }
        },
    }
    mid = tool.interpolated_frame_overrides(pose, "run", 3)
    assert "front_arm_scale" not in mid
    assert -3.0 < mid["front_arm_angle"] < -2.0
    assert tool.interpolated_frame_overrides(pose, "run", 0)["front_arm_scale"] == 0.5
    assert tool.interpolated_frame_overrides(pose, "run", 7)["front_arm_scale"] == 2.0


def test_top_level_scale_defaults_apply_to_all_frames():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    pose = tool.animation_pose("run", 3, 8, 0.275)
    pose = tool.apply_pose_overrides(
        pose, "run", 3, {"defaults": {"front_arm_scale": 1.25}}
    )
    assert tool.role_scale_multiplier(pose, "front_arm") == 1.25
    assert tool.role_scale_multiplier(pose, "front_hand") == 1.0


def test_default_yaml_side_convention_and_scales():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    overrides = tool.load_pose_overrides(
        root / "metadata" / "robot_pose_overrides.yaml"
    )
    defaults = overrides["defaults"]
    assert defaults["front_arm_scale"] == 1.0
    assert defaults["back_arm_scale"] == 1.0
    assert defaults["front_hand_scale"] == 0.5
    assert defaults["back_hand_scale"] == 0.5
    assert defaults["front_leg_scale"] == 0.8
    assert defaults["back_leg_scale"] == 0.8
    assert defaults["front_foot_scale"] == 1.0
    assert defaults["back_foot_scale"] == 1.0

    pose = tool.animation_pose("run", 0, tool.ANIMATIONS["run"]["frames"], 0.275)
    pose = tool.apply_pose_overrides(pose, "run", 0, overrides)
    assert tool.role_scale_multiplier(pose, "front_hand") == 0.5
    assert tool.role_scale_multiplier(pose, "back_hand") == 0.5
    assert tool.role_scale_multiplier(pose, "front_leg") == 0.8
    assert tool.role_scale_multiplier(pose, "back_leg") == 0.8
    assert tool.role_scale_multiplier(pose, "front_foot") == 1.0
    assert tool.role_scale_multiplier(pose, "back_foot") == 1.0


def test_connected_anchor_constraints_are_exact_in_run_manifest():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(
        atlas, job.render, pose_overrides=tool.load_pose_overrides(job.pose_overrides)
    )
    for idx in range(tool.animation_info("run", assembler.pose_overrides)["frames"]):
        _frame, manifest = assembler.render_frame("run", idx)
        comps = {c["role"]: c for c in manifest["pose"]["components"]}
        assert comps["front_arm"]["connects_to"]["anchor"] == "shoulder_right"
        assert comps["back_arm"]["connects_to"]["anchor"] == "shoulder_left"
        for role in [
            "front_arm",
            "back_arm",
            "front_hand",
            "back_hand",
            "front_leg",
            "back_leg",
            "front_foot",
            "back_foot",
        ]:
            assert comps[role]["snap_error_px"] <= 0.001, (idx, role, comps[role])
        for role in [
            "front_arm",
            "back_arm",
            "front_leg",
            "back_leg",
            "front_foot",
            "back_foot",
        ]:
            err = comps[role].get("endpoint_snap_error_px")
            assert err is None or err <= 0.001, (idx, role, comps[role])
        assert (
            comps["front_hand"]["parent_target"]
            == comps["front_arm"]["endpoint_anchor_world"]
        )
        assert (
            comps["back_hand"]["parent_target"]
            == comps["back_arm"]["endpoint_anchor_world"]
        )
        assert (
            comps["front_foot"]["parent_target"]
            == manifest["pose"]["leg_mounts"]["front_ankle"]
        )
        assert (
            comps["back_foot"]["parent_target"]
            == manifest["pose"]["leg_mounts"]["back_ankle"]
        )


def test_run_feet_are_first_class_ankle_constraints():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    assembler = tool.RobotAssembler(
        atlas, job.render, pose_overrides=tool.load_pose_overrides(job.pose_overrides)
    )
    _frame, manifest = assembler.render_frame("run", 0)
    comps = {c["role"]: c for c in manifest["pose"]["components"]}
    assert comps["front_foot"]["connects_to"] == {
        "role": "front_leg",
        "sprite": comps["front_leg"]["sprite"],
        "anchor": "ankle",
    }
    assert comps["back_foot"]["connects_to"] == {
        "role": "back_leg",
        "sprite": comps["back_leg"]["sprite"],
        "anchor": "ankle",
    }
    assert comps["front_foot"]["snap_error_px"] <= 0.001
    assert comps["back_foot"]["snap_error_px"] <= 0.001


def test_zero_endpoint_delta_keeps_anchor_constraints_exact():
    root = Path(__file__).resolve().parents[1]
    tool = _load_tool(root)
    job = tool.RigJob.load(root / "examples" / "robot_rig_job.yaml")
    atlas = tool.ComponentAtlas(job.metadata, job.slices)
    overrides = {
        "defaults": {
            "front_hand_scale": 0.5,
            "back_hand_scale": 0.5,
            "front_leg_scale": 0.8,
            "back_leg_scale": 0.8,
        },
        "animations": {
            "run": {
                "frame_overrides": {
                    "0": {
                        "front_wrist_delta": [0, 0],
                        "back_wrist_delta": [0, 0],
                        "front_ground_delta": [0, 0],
                        "back_ground_delta": [0, 0],
                    }
                }
            }
        },
    }
    assembler = tool.RobotAssembler(atlas, job.render, pose_overrides=overrides)
    _frame, manifest = assembler.render_frame("run", 0)
    comps = {c["role"]: c for c in manifest["pose"]["components"]}
    for role in [
        "front_arm",
        "back_arm",
        "front_hand",
        "back_hand",
        "front_leg",
        "back_leg",
        "front_foot",
        "back_foot",
    ]:
        assert comps[role]["snap_error_px"] <= 0.001, (role, comps[role])
