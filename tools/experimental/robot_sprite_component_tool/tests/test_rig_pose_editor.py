from pathlib import Path

import yaml

from tools import rig_pose_editor as editor
from tools import robot_rig_sheet as rig

ROOT = Path(__file__).resolve().parents[1]
JOB = ROOT / 'examples' / 'robot_rig_job.yaml'
META = ROOT / 'metadata' / 'robot_components.refined.yaml'
SLICES = ROOT / 'output' / 'slices'
POSE = ROOT / 'metadata' / 'robot_pose_overrides.yaml'


def test_pose_overrides_apply_frame_specific_art_and_zorder(tmp_path):
    pose = {
        'version': '0.2',
        'animations': {
            'run': {
                'frame_overrides': {
                    '0': {
                        'front_hand_sprite': 'hand_open_down',
                        'front_hand_angle': 17,
                        'z_order': ['back_leg', 'torso', 'front_leg', 'front_arm', 'front_hand', 'head'],
                    }
                }
            }
        }
    }
    pp = tmp_path / 'pose.yaml'
    pp.write_text(yaml.safe_dump(pose), encoding='utf8')
    job = rig.RigJob.load(JOB)
    job.pose_overrides = pp
    atlas = rig.ComponentAtlas(META, SLICES)
    asm = rig.RobotAssembler(atlas, job.render, pose_overrides=rig.load_pose_overrides(pp))
    _img, manifest = asm.render_frame('run', 0)
    comps = manifest['pose']['components']
    front_hand = [c for c in comps if c['role'] == 'front_hand'][0]
    assert front_hand['sprite'] == 'hand_open_down'
    assert abs(front_hand['angle'] - 17) > 0  # includes hand-follow contribution but is frame-specific
    assert manifest['pose']['z_order'][:2] == ['back_leg', 'torso']


def test_pose_editor_headless_preview_and_instance_report(tmp_path):
    paths = editor.resolve_job_paths(JOB, META, SLICES, POSE)
    img, manifest = editor.build_preview(paths.job, editor.load_yaml(paths.metadata), editor.load_yaml(paths.pose_overrides), animations=['run'])
    assert img.width > 0 and img.height > 0
    frame = manifest['animations']['run']['frames'][0]
    roles = {c['role'] for c in frame['pose']['components']}
    assert {'torso', 'front_arm', 'back_arm', 'front_leg', 'back_leg', 'head'} <= roles


def test_relevant_animation_filter_finds_run_for_run_torso():
    paths = editor.resolve_job_paths(JOB, META, SLICES, POSE)
    anims = editor.relevant_animations(paths.job, editor.load_yaml(paths.metadata), editor.load_yaml(paths.pose_overrides), 'torso_lean_forward')
    assert 'run' in anims


def test_sparse_pose_keyframes_interpolate_between_first_and_last(tmp_path):
    pose = {
        'version': '0.3',
        'animations': {
            'run': {
                'frames': 8,
                'frame_overrides': {
                    '0': {'front_wrist_delta': [10, 10], 'front_arm_angle': -20},
                    '7': {'front_wrist_delta': [24, 24], 'front_arm_angle': 8},
                },
            },
        },
    }
    pp = tmp_path / 'pose.yaml'
    pp.write_text(yaml.safe_dump(pose), encoding='utf8')
    job = rig.RigJob.load(JOB)
    job.pose_overrides = pp
    atlas = rig.ComponentAtlas(META, SLICES)
    asm = rig.RobotAssembler(atlas, job.render, pose_overrides=rig.load_pose_overrides(pp))
    evaluated = rig.interpolated_frame_overrides(pose, 'run', 3)
    assert 15.9 < evaluated['front_wrist_delta'][0] < 16.1
    assert 15.9 < evaluated['front_wrist_delta'][1] < 16.1
    assert -8.1 < evaluated['front_arm_angle'] < -7.9

    _img, manifest = asm.render_frame('run', 3)
    front_arm = [c for c in manifest['pose']['components'] if c['role'] == 'front_arm'][0]
    # The endpoint-solved renderer should consume the tweened wrist target.
    assert 14.0 < front_arm['endpoint'][0] - front_arm['target'][0] < 24.0


def test_hidden_parts_are_not_drawn_but_remain_in_manifest(tmp_path):
    pose = {
        'version': '0.3',
        'animations': {
            'run': {
                'frame_overrides': {
                    '0': {'hidden_parts': ['front_arm', 'front_hand', 'head']},
                },
            },
        },
    }
    pp = tmp_path / 'pose.yaml'
    pp.write_text(yaml.safe_dump(pose), encoding='utf8')
    job = rig.RigJob.load(JOB)
    job.pose_overrides = pp
    atlas = rig.ComponentAtlas(META, SLICES)
    asm = rig.RobotAssembler(atlas, job.render, pose_overrides=rig.load_pose_overrides(pp))
    _img, manifest = asm.render_frame('run', 0)
    by_role = {c['role']: c for c in manifest['pose']['components']}
    assert by_role['front_arm']['visible'] is False
    assert by_role['front_hand']['visible'] is False
    assert by_role['head']['visible'] is False
    assert by_role['torso']['visible'] is True
    assert manifest['pose']['hidden_parts'] == ['front_arm', 'front_hand', 'head']


def test_manifest_exposes_rig_constraints_and_snap_errors():
    job = rig.RigJob.load(JOB)
    atlas = rig.ComponentAtlas(META, SLICES)
    asm = rig.RobotAssembler(atlas, job.render, pose_overrides=rig.load_pose_overrides(POSE))
    _img, manifest = asm.render_frame('run', 0)
    constraints = manifest['pose']['rig_constraints']
    by_role = {row['role']: row for row in constraints}
    assert by_role['front_arm']['joint'] == 'front_shoulder'
    assert by_role['front_arm']['child'] == {'sprite': 'arm_capsule_vertical', 'anchor': 'shoulder'}
    assert by_role['front_arm']['parent']['role'] == 'torso'
    assert by_role['front_arm']['parent']['anchor'] == 'shoulder_right'
    assert by_role['front_arm']['snap_error_px'] <= 0.001
    assert by_role['front_hand']['joint'] == 'front_wrist'
    assert by_role['front_hand']['parent']['role'] == 'front_arm'
