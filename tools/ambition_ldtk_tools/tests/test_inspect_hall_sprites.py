"""Inspect authored sprite references without fixing a game's cast size."""

from contextlib import redirect_stdout
import io
import json

from ambition_ldtk_tools import inspect_hall_sprites as inspect


def products(tmp_path):
    manifest = tmp_path / 'metadata' / 'sample.ron'
    image = tmp_path / 'images' / 'sample.png'
    manifest.parent.mkdir()
    image.parent.mkdir()
    manifest.write_text('[(target: "sample", rows: [(animation: "idle")])]')
    image.write_bytes(b'png')
    return manifest, image


def test_inspection_reads_accepted_products_from_separate_bundles(tmp_path):
    manifest, image = products(tmp_path)
    catalog = {'characters': {'sample': {'manifest': 'sprites/sample_spritesheet.ron'}}}
    files = {'sprites/sample_spritesheet.ron': manifest, 'sprites/sample_spritesheet.png': image}
    assert inspect.classify('sample', catalog, tmp_path / 'unused', files) == ('ok', '')
    del files['sprites/sample_spritesheet.png']
    assert inspect.classify('sample', catalog, tmp_path / 'unused', files)[0] == 'no_png'


def test_only_issues_omits_valid_fixture_entries(tmp_path):
    manifest, image = products(tmp_path)
    sprites = tmp_path / 'sprites'
    sprites.mkdir()
    (sprites / 'sample_spritesheet.ron').write_text(manifest.read_text())
    (sprites / 'sample_spritesheet.png').write_bytes(image.read_bytes())
    catalog = tmp_path / 'cast.ron'
    catalog.write_text('(characters: {"sample": (manifest: "sprites/sample_spritesheet.ron")})')
    world = tmp_path / 'hall.ldtk'
    world.write_text(json.dumps({'levels': [{'identifier': 'hall_of_characters', 'layerInstances': [
        {'entityInstances': [{'__identifier': 'NpcSpawn', 'fieldInstances': [
            {'__identifier': 'character_id', '__value': 'sample'},
        ]}]}],
    }]}))
    output = io.StringIO()
    with redirect_stdout(output):
        assert inspect.main(['--catalog', str(catalog), '--ldtk', str(world), '--sprites-dir', str(sprites), '--only-issues']) == 0
    assert '[ok' not in output.getvalue()


def test_provider_owned_entries_are_not_local_catalog_drift(tmp_path, monkeypatch):
    monkeypatch.setattr(inspect, 'PROVIDER_HALL_IDS', {'foreign'})
    assert inspect.classify('foreign', {'characters': {}}, tmp_path)[0] == 'provider_owned'
