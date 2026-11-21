"""Boss inspection keeps source identities separate from shared art."""
from copy import deepcopy

from asset_index.boss_roster import entries
from asset_index.inventory import assemble


def test_boss_entries_include_unplaced_profiles_and_art_without_a_profile():
    prepared = {'provider': 'game', 'scope': {'bosses': {
        'marshal_gallus': 'marshal_gallus_spritesheet.png',
        'mount': 'pair/giant_spritesheet.png'}, 'rooms': {'corral': {'spec': {'boss_spawns': [
            {'name': 'Marshal Gallus', 'payload': {'PhaseScript': {'script_id': 'marshal_gallus'}}}]}}}},
        'references': [{'from': 'game:boss/marshal_gallus', 'to': 'game:boss_encounter/marshal_gallus'},
                       {'from': 'game:boss_encounter/unplaced', 'to': 'game:boss/unplaced'},
                       {'from': 'other:boss/foreign', 'to': 'other:boss_encounter/foreign'}]}
    before = deepcopy(prepared)
    found = {row['boss_id']: row for row in entries(prepared)}
    assert set(found) == {'marshal_gallus', 'mount', 'unplaced'}
    assert found['marshal_gallus']['catalog']['display_name'] == 'Marshal Gallus'
    assert found['marshal_gallus']['catalog']['manifest'] == 'sprites/marshal_gallus_spritesheet.ron'
    assert found['mount']['catalog']['manifest'] == 'sprites/pair/giant_spritesheet.ron'
    assert found['marshal_gallus']['boss_profile'] and not found['mount']['boss_profile']
    assert found['unplaced']['catalog']['manifest'] == ''
    assert prepared == before


def test_boss_and_hall_character_share_a_bundle_without_merging_identity(tmp_path):
    unit = {'id': 'target:behemoth', 'kind': 'actor', 'root': 'sprites',
            'generator': {'id': 'behemoth'}, 'required': ['behemoth_spritesheet.ron'],
            'optional': [], 'source': {}, 'status': {'state': 'missing', 'tiers': {}}, 'repair': ''}
    prepared = {'state': 'available', 'provider': 'game', 'characters': [{
        'id': 'npc_behemoth', 'provider': 'game', 'sheet_target': 'behemoth',
        'catalog': {'manifest': 'sprites/behemoth_spritesheet.ron', 'display_name': 'Behemoth'}}],
        'scope': {'bosses': {'behemoth': 'behemoth_spritesheet.png'}}}
    report = assemble(tmp_path, {'units': {'target:behemoth': unit}}, prepared, lineages={})
    assert set(report['identities']) == {'game:npc_behemoth', 'game:boss/behemoth'}
    assert report['units']['target:behemoth']['identities'] == ['game:npc_behemoth', 'game:boss/behemoth']
    assert len(report['character_groups']) == 2
    boss = report['identities']['game:boss/behemoth']
    assert boss['inspection_origin'] == 'boss'
    assert boss['coverage'] == []
    assert boss['units'] == ['target:behemoth']


def test_a_boss_without_an_art_binding_stays_visible_without_claiming_missing_art(tmp_path):
    prepared = {'provider': 'game', 'references': [{'from': 'game:boss/absent', 'to': 'game:boss_encounter/absent'}]}
    report = assemble(tmp_path, {'units': {}}, prepared, lineages={})
    boss = report['identities']['game:boss/absent']
    assert boss['catalog']['manifest'] == '' and not boss['coverage']
    assert report['units'][boss['units'][0]]['status']['state'] == 'unknown'
