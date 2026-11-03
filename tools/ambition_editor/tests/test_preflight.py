"""A scenario resolves authored references before any generated files exist."""
from pathlib import Path
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index import preflight


def unit(name, files=None, state='missing', **fields):
    return {'id': 'target:' + name, 'root': 'sprites', 'required': files or [name + '_spritesheet.ron'],
            'status': {'state': state}, 'repair': './scripts/regen/sprites.sh --target ' + name, **fields}


def fixture():
    definitions = {row['id']: row for row in [
        unit('painter'), unit('new_painter'), unit('gate', ['ring_spritesheet.ron', 'portal_spritesheet.ron']),
        unit('palette'), unit('jar'), unit('unused'), unit('exotic', semantic_effects=['sonic_boom'])]}
    for prefix in ('theme', 'skin'):
        definitions[prefix + ':paint'] = {'id': prefix + ':paint', 'root': 'scenery', 'required': [prefix + '.png'],
                                         'status': {'state': 'missing'}, 'repair': 'publish paint'}
    spec = {'placements': [{'schema': {'Npc': {'character_id': 'the_painter'}}}],
            'props': [{'id': 'ring', 'kind': 'the_ring'}],
            'moving_platforms': [{'visual': 'palette'}],
            'lock_walls': [{'look': 'jar'}],
            'metadata': {'biome': 'paint', 'visual_profile': {'parallax_theme': 'paint'}}}
    prepared = {'fingerprint': 'pack', 'characters': [
        {'id': cid, 'catalog': {'manifest': f'sprites/{target}_spritesheet.ron'}, 'sheet_target': target}
        for cid, target in [('the_painter', 'painter'), ('another_painter', 'new_painter')]],
        'scope': {'rooms': {'plaza': {'world': 'world.paint_ldtk', 'source': 'paint.ldtk', 'spec': spec, 'theme': 'paint'}},
                  'props': {'the_ring': 'ring', 'palette': 'palette', 'jar': 'jar'}}}
    return prepared, {'units': definitions}


def test_an_empty_store_resolves_a_world_and_a_shared_producer_from_authored_references():
    prepared, products = fixture()
    result = preflight.prepare(prepared, products, worlds=['paint'])
    assert result['complete'], result['diagnostics']
    assert set(result['missing']) == {'target:painter', 'target:gate', 'target:palette', 'target:jar', 'theme:paint', 'skin:paint'}
    assert result['rooms'] == ['plaza']
    assert result['characters'] == ['the_painter']
    assert result['units']['target:gate']['reasons'][0]['logical_path'] == 'sprites/ring_spritesheet.ron'
    assert not result['ready']
    prepared['scope']['rooms']['plaza']['spec']['placements'][0]['schema']['Npc']['character_id'] = 'another_painter'
    changed = preflight.prepare(prepared, products, worlds=['paint.ldtk'])
    assert 'target:new_painter' in changed['missing'] and 'target:painter' not in changed['units']
    assert changed['scope_fingerprint'] != result['scope_fingerprint']


def test_semantic_effect_lookup_does_not_need_a_published_sheet():
    prepared, products = fixture()
    result = preflight.prepare(prepared, products, effects=['sonic_boom'])
    assert result['complete']
    assert result['missing'] == ['target:exotic']


def test_headless_and_capture_plans_keep_gameplay_metadata_separate():
    prepared, products = fixture()
    products['units']['target:painter']['gameplay_targets'] = ['painter_body']
    products['units']['target:painter_body'] = unit('painter_body', ['painter_body_rig.ron'], tiers=['full'])
    simulation = preflight.prepare(prepared, products, characters=['the_painter'], purpose='headless')
    capture = preflight.prepare(prepared, products, characters=['the_painter'], purpose='capture')
    assert simulation['missing'] == ['target:painter_body']
    assert set(capture['missing']) == {'target:painter', 'target:painter_body'}
    assert not simulation['units']['target:painter']['required']


def test_missing_generation_keeps_stale_and_partial_products(tmp_path, monkeypatch):
    prepared, products = fixture()
    products['units']['target:painter']['status']['state'] = 'stale'
    products['units']['target:gate']['status']['state'] = 'partial'
    result = preflight.prepare(prepared, products, worlds=['paint'])
    calls = []
    monkeypatch.setattr(preflight.subprocess, 'run', lambda argv, **kwargs: calls.append(argv))
    generated = preflight.generate_missing(tmp_path, result)
    assert set(generated) == {'target:palette', 'target:jar', 'theme:paint', 'skin:paint'}
    assert result['stale'] == ['target:painter'] and result['partial'] == ['target:gate']
    assert len(calls) == 2
    assert '--missing-only' in calls[0] and '--missing-only' in calls[1]
    assert 'painter' not in calls[0] and 'gate' not in calls[0]


def test_a_plan_lists_all_unresolved_references_and_refuses_generation(tmp_path):
    prepared, products = fixture()
    result = preflight.prepare(prepared, products, rooms=['absent'], characters=['unknown'], effects=['no_such_effect'])
    assert {row['code'] for row in result['diagnostics']} == {'unknown-room', 'unknown-character', 'unknown-effect'}
    with pytest.raises(ValueError, match='diagnostics'):
        preflight.generate_missing(tmp_path, result)


def test_a_headless_plan_reports_optional_visual_gaps_without_requiring_them():
    prepared, products = fixture()
    del products['units']['target:painter']
    result = preflight.prepare(prepared, products, characters=['the_painter'], purpose='headless')
    assert result['ready'] and result['complete']
    assert result['diagnostics'][0]['severity'] == 'warning'
    assert not result['missing']


def test_a_semantic_product_query_does_not_compile_unrelated_game_content(tmp_path, monkeypatch):
    _, products = fixture()
    monkeypatch.setattr(preflight.sources, 'products', lambda *args, **kwargs: products)
    result = preflight.collect(tmp_path, effects=['sonic_boom'])
    assert result['complete'] and result['missing'] == ['target:exotic']


def test_availability_reads_only_the_required_units(tmp_path, monkeypatch):
    prepared, products = fixture()
    products['units']['target:painter']['gameplay_targets'] = ['painter_body']
    products['units']['target:painter_body'] = unit('painter_body', ['painter_body_rig.ron'])
    for row in products['units'].values():
        row['status']['state'] = 'not-checked'
    monkeypatch.setattr(preflight, 'content', lambda *args: prepared)
    def owner(repo, python, *, catalog=False, requested=None):
        if catalog:
            return products
        assert requested == ['target:painter_body']
        return {'units': {'target:painter_body': unit('painter_body', ['painter_body_rig.ron'], state='stale')},
                'publication': {'files': {'sprites/painter_spritesheet.ron': {}}}}
    monkeypatch.setattr(preflight.sources, 'products', owner)
    result = preflight.collect(tmp_path, characters=['the_painter'], purpose='headless')
    assert result['ready']
    assert result['units']['target:painter']['availability'] == 'not-checked'
    assert not result['units']['target:painter']['required']


def test_a_new_required_member_of_an_existing_bundle_is_partial():
    prepared, products = fixture()
    prepared['characters'][0]['catalog']['manifest'] = 'sprites/new_output_spritesheet.ron'
    products['units']['target:gate']['required'].append('new_output_spritesheet.ron')
    products['units']['target:gate']['status']['state'] = 'stale'
    products['publication'] = {'files': {'sprites/ring_spritesheet.ron': {}}}
    result = preflight.prepare(prepared, products, characters=['the_painter'])
    assert result['complete'] and not result['ready']
    assert result['partial'] == ['target:gate']
    assert not result['missing']
    assert result['units']['target:gate']['missing_paths'] == ['sprites/new_output_spritesheet.ron']
    products['publication']['files']['sprites/new_output_spritesheet.ron'] = {}
    assert preflight.prepare(prepared, products, characters=['the_painter'])['ready']


def test_an_effect_missing_from_an_accepted_sheet_is_partial():
    prepared, products = fixture()
    effect = products['units']['target:exotic']
    effect.update(semantic_effects=['old_effect', 'sonic_boom'], accepted_effects=['old_effect'])
    effect['status']['state'] = 'stale'
    old = preflight.prepare(prepared, products, effects=['old_effect'])
    new = preflight.prepare(prepared, products, effects=['sonic_boom'])
    assert old['ready']
    assert new['partial'] == ['target:exotic'] and not new['missing']
    assert new['units']['target:exotic']['missing_effects'] == ['sonic_boom']
