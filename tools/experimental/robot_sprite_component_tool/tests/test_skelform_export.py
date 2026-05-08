from __future__ import annotations

import json
import zipfile
from pathlib import Path

from tools.skelform_export import export_skelform


ROOT = Path(__file__).resolve().parents[1]


def test_skelform_export_uses_integer_schema_fields(tmp_path: Path):
    out = tmp_path / 'robot_pose.skf'
    export_skelform(ROOT / 'examples' / 'robot_rig_job.yaml', out, animation='run', frame_index=0)
    with zipfile.ZipFile(out) as zf:
        armature = json.load(zf.open('armature.json'))

    assert armature['version'] == '0.4.0'
    assert armature['img_format'] == 'PNG'
    assert all(isinstance(v, int) for v in armature['ik_root_ids'])
    for atlas in armature['atlases']:
        assert isinstance(atlas['size']['x'], int)
        assert isinstance(atlas['size']['y'], int)
    for style in armature['styles']:
        assert isinstance(style['id'], int)
        for texture in style['textures']:
            assert isinstance(texture['atlas_idx'], int)
            assert isinstance(texture['offset']['x'], int)
            assert isinstance(texture['offset']['y'], int)
            assert isinstance(texture['size']['x'], int)
            assert isinstance(texture['size']['y'], int)
    for bone in armature['bones']:
        assert isinstance(bone['id'], int)
        assert isinstance(bone['parent_id'], int)
        assert isinstance(bone['ik_family_id'], int)
        if 'zindex' in bone:
            assert isinstance(bone['zindex'], int)
        if 'ik_target_id' in bone:
            assert isinstance(bone['ik_target_id'], int)
        if 'ik_bone_ids' in bone:
            assert all(isinstance(v, int) for v in bone['ik_bone_ids'])
    for animation in armature['animations']:
        assert isinstance(animation['id'], int)
        assert isinstance(animation['fps'], int)
        for keyframe in animation['keyframes']:
            assert isinstance(keyframe['frame'], int)
            assert isinstance(keyframe['bone_id'], int)


def test_skelform_export_ik_targets_are_top_level_and_correctly_wired(tmp_path: Path):
    out = tmp_path / 'robot_pose.skf'
    export_skelform(ROOT / 'examples' / 'robot_rig_job.yaml', out, animation='run', frame_index=0)
    with zipfile.ZipFile(out) as zf:
        armature = json.load(zf.open('armature.json'))

    bones = armature['bones']
    by_name = {bone['name']: bone for bone in bones}
    expected_targets = {
        'BackArm': 'BackHandTarget',
        'FrontArm': 'FrontHandTarget',
        'BackLeg': 'BackFootTarget',
        'FrontLeg': 'FrontFootTarget',
    }
    # SkelForm v0.4 examples keep IK targets unparented and before the rig
    # chains. This avoids target-selector confusion where a foot can appear to
    # follow a hand target.
    for target in expected_targets.values():
        assert by_name[target]['parent_id'] == -1
        assert by_name[target]['id'] < by_name['Root']['id']

    assert set(armature['ik_root_ids']) == {by_name[k]['id'] for k in expected_targets}
    for root, target in expected_targets.items():
        rb = by_name[root]
        assert rb['ik_target_id'] == by_name[target]['id']
        chain_names = [bones[idx]['name'] for idx in rb['ik_bone_ids']]
        assert chain_names[0] == root
        if root.endswith('Arm'):
            assert chain_names[1].endswith('Wrist')
            visible_child = 'BackHand' if root == 'BackArm' else 'FrontHand'
            assert by_name[visible_child]['parent_id'] == by_name[chain_names[1]]['id']
            assert by_name[visible_child].get('tex')
        else:
            assert chain_names[1].endswith('Ankle')
            visible_child = 'BackFoot' if root == 'BackLeg' else 'FrontFoot'
            assert by_name[visible_child]['parent_id'] == by_name[chain_names[1]]['id']
            assert by_name[visible_child].get('tex')
        # The visible hand/foot attachment must not be part of the IK chain.
        # It can then be moved along the wrist/ankle endpoint independently of
        # the chain length / bend solution.
        assert len(chain_names) == 2
