"""Tests for the bone-toolkit player robot candidate.

Covers the reusable skeleton module (IK correctness, channel sampling with
loop wrap) and the target's animation invariants: planted feet move at a
constant treadmill rate during stance (no foot sliding), leg IK targets stay
within reach (no overstretch popping), feet stay on the ground line across
walk frames, and the sheet build emits the standard PNG/YAML/RON bundle.
"""

from __future__ import annotations

import math

import pytest
import yaml

from ambition_sprite2d_renderer.authoring.skeleton import (
    Channel,
    Skeleton,
    two_bone_ik,
)
from ambition_sprite2d_renderer.targets.characters import player_robot_fable as prf


def _dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


class TestTwoBoneIk:
    def test_reaches_reachable_targets(self):
        root = (3.0, -2.0)
        len1, len2 = 10.0, 8.0
        for deg in range(0, 360, 17):
            for r in (3.5, 9.0, 17.4):
                target = (
                    root[0] + r * math.cos(math.radians(deg)),
                    root[1] + r * math.sin(math.radians(deg)),
                )
                a1, a2 = two_bone_ik(root, target, len1, len2)
                joint = (
                    root[0] + len1 * math.cos(math.radians(a1)),
                    root[1] + len1 * math.sin(math.radians(a1)),
                )
                tip = (
                    joint[0] + len2 * math.cos(math.radians(a2)),
                    joint[1] + len2 * math.sin(math.radians(a2)),
                )
                assert _dist(tip, target) < 1e-6

    def test_unreachable_targets_clamp_to_reach(self):
        root = (0.0, 0.0)
        a1, a2 = two_bone_ik(root, (100.0, 0.0), 10.0, 8.0)
        joint = (10.0 * math.cos(math.radians(a1)), 10.0 * math.sin(math.radians(a1)))
        tip = (
            joint[0] + 8.0 * math.cos(math.radians(a2)),
            joint[1] + 8.0 * math.sin(math.radians(a2)),
        )
        assert _dist(tip, (18.0, 0.0)) < 1e-3

    def test_bend_sign_picks_knee_side(self):
        # Target straight below the root: bend=+1 puts the joint on +x
        # (knee forward for a right-facing character), bend=-1 on -x.
        root = (0.0, 0.0)
        target = (0.0, 15.0)
        for bend, sign in ((1.0, 1.0), (-1.0, -1.0)):
            a1, _ = two_bone_ik(root, target, 10.0, 8.0, bend=bend)
            joint_x = 10.0 * math.cos(math.radians(a1))
            assert joint_x * sign > 0.5


class TestChannel:
    def test_interior_segment_with_linear_ease(self):
        ch = Channel((0.0, 0.0), (1.0, 10.0, "linear"))
        assert ch.sample(0.5, loop=False) == pytest.approx(5.0)

    def test_one_shot_clamps_at_ends(self):
        ch = Channel((0.2, 1.0), (0.8, 2.0))
        assert ch.sample(0.0, loop=False) == pytest.approx(1.0)
        assert ch.sample(1.0, loop=False) == pytest.approx(2.0)

    def test_loop_wraps_across_boundary(self):
        ch = Channel((0.2, 1.0, "linear"), (0.8, 2.0, "linear"), default_ease="linear")
        # Wrap segment runs 0.8 -> 1.2 (== 0.2); t=0.0 is halfway through it.
        assert ch.sample(0.0, loop=True) == pytest.approx(1.5)
        # Exactly at the keys.
        assert ch.sample(0.2, loop=True) == pytest.approx(1.0)
        assert ch.sample(0.8, loop=True) == pytest.approx(2.0)


class TestSkeleton:
    def test_child_offset_rotates_with_parent(self):
        sk = Skeleton()
        sk.bone("a", length=10.0)
        sk.bone("b", parent="a", offset=(10.0, 0.0), length=5.0)
        world = sk.world({"a": 90.0})
        # Parent rotated 90deg (clockwise, y down): its tip — and b's
        # origin — is straight below the root.
        assert world["b"].origin[0] == pytest.approx(0.0, abs=1e-9)
        assert world["b"].origin[1] == pytest.approx(10.0)

    def test_pose_angle_for_world_round_trips(self):
        sk = Skeleton()
        sk.bone("a", rest_angle=30.0)
        sk.bone("b", parent="a", rest_angle=15.0)
        world = sk.world({"a": 10.0})
        pose = sk.pose_angle_for_world("b", 77.0, world)
        world2 = sk.world({"a": 10.0, "b": pose})
        assert world2["b"].angle == pytest.approx(77.0)


class TestWalkCycle:
    def test_planted_foot_moves_at_constant_rate(self):
        # Flat stance (after heel-strike settle, before toe-off) must be a
        # constant-velocity backward slide in the in-place cycle — any
        # nonuniformity reads as foot skating in-game.
        for side, phase_off in (("near", 0.0), ("far", 0.5)):
            ts = [0.12 + 0.025 * i for i in range(11)]  # ph in [0.12, 0.37]
            xs = []
            for t in ts:
                sampled = prf.CLIPS["walk"].sample((t - phase_off) % 1.0)
                assert sampled[f"{side}_foot_lift"] == pytest.approx(0.0)
                assert sampled[f"{side}_foot_pitch"] == pytest.approx(0.0)
                xs.append(sampled[f"{side}_foot_x"])
            deltas = [xs[i + 1] - xs[i] for i in range(len(xs) - 1)]
            for d in deltas:
                assert d == pytest.approx(deltas[0], abs=1e-9)
            assert deltas[0] < 0  # moving backward

    def test_feet_stay_on_ground_across_frames(self):
        # The lowest visible pixel (the sole of the planted foot) must hold
        # steady across the walk cycle; drift means floating or clipping.
        lows = []
        for i in range(8):
            img = prf.render_frame("walk", i, 8)
            bbox = img.getchannel("A").getbbox()
            assert bbox is not None
            lows.append(bbox[3])
        assert max(lows) - min(lows) <= 2
        assert all(96 <= y <= 106 for y in lows)


class TestLegReach:
    @pytest.mark.parametrize("animation", ["idle", "walk", "slash"])
    def test_ik_targets_always_reachable(self, animation):
        # If a clip asks a foot to go farther than the leg can reach, IK
        # clamps and the foot pops off its authored target. Authoring bug —
        # catch it here rather than in the rendered sheet.
        max_reach = prf.LEG_U + prf.LEG_L
        for i in range(33):
            t = i / 32.0
            world, sampled = prf._solve(animation, t)
            for side in ("near", "far"):
                hip = world[f"{side}_leg_u"].origin
                ankle = world[f"{side}_leg_l"].tip
                target = prf._foot_target(sampled, side)
                assert _dist(hip, target) <= max_reach + 1e-6, (
                    f"{animation} t={t} {side}: target {target} beyond reach"
                )
                assert _dist(ankle, target) < 0.05, (
                    f"{animation} t={t} {side}: ankle {ankle} != target {target}"
                )


class TestSlashStaysStanding:
    def test_no_crouch(self):
        # F-tilt, not a crouch poke: the root never dips during the slash.
        for i in range(16):
            sampled = prf.CLIPS["slash"].sample(i / 15.0)
            assert sampled.get("root_y", 0.0) == pytest.approx(0.0)


class TestSheetBuild:
    def test_render_frame_shape(self):
        img = prf.render_frame("idle", 1, 8)
        assert img.size == (128, 128)
        assert img.mode == "RGBA"
        assert img.getchannel("A").getbbox() is not None

    def test_build_outputs_and_manifest(self, tmp_path):
        paths = prf.render(tmp_path)
        names = {p.name for p in paths}
        assert "player_robot_fable_spritesheet.png" in names
        assert "player_robot_fable_spritesheet.yaml" in names
        assert "player_robot_fable_spritesheet.ron" in names
        manifest = yaml.safe_load(
            (tmp_path / "player_robot_fable_spritesheet.yaml").read_text()
        )
        rows = {r["animation"]: r for r in manifest["rows"]}
        assert list(rows) == ["idle", "walk", "slash"]
        for name, (anim, frames, duration) in zip(rows, prf.ROWS):
            assert rows[name]["frame_count"] == frames
            assert rows[name]["duration_ms"] == duration
            assert len(rows[name]["rects"]) == frames
        assert manifest["body_metrics"]["feet_anchor_norm"] is not None
        ron = (tmp_path / "player_robot_fable_spritesheet.ron").read_text()
        assert 'target: "player_robot_fable"' in ron
        assert 'animation: "idle"' in ron
