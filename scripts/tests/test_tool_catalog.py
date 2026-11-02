"""Command discovery follows intent and checks current source paths."""
import importlib.util
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from lib.tool_catalog import commands, inventory

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('catalog_query', ROOT / 'scripts/agent_query.py')
query = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = query
spec.loader.exec_module(query)


@pytest.mark.parametrize(('intent', 'expected'), [
    ('screenshot', 'capture_scene'), ('deterministic capture', 'capture_scene'),
    ('tune a move', 'moveset'), ('edit an LDtk room', 'ldtk'),
    ('inspect published assets', 'assets'), ('inspect ECS systems', 'ecs'),
])
def test_intent_routes_to_owner(intent, expected):
    matches = query.ranked(commands(ROOT), intent, ('title', 'id', 'inputs', 'outputs'), 'title', 1)
    assert matches[0][1]['id'] == expected


def test_missing_source_remains_visible_without_claiming_availability(tmp_path):
    (tmp_path / 'scripts').mkdir()
    descriptor = tmp_path / 'scripts/commands.json'
    descriptor.write_text(json.dumps({'commands': [{'id': 'missing', 'requires': ['absent.py']}]}))
    assert commands(tmp_path)[0]['availability'] == 'missing source: absent.py'
    (tmp_path / 'absent.py').write_text('')
    assert commands(tmp_path)[0]['availability'].startswith('source present')
    descriptor.unlink()
    assert commands(tmp_path) == []


def test_inventory_reports_path_risks(tmp_path):
    (tmp_path / 'scripts').mkdir()
    (tmp_path / 'scripts/leaf.py').write_text('root = Path(__file__).parents[1]')
    report = inventory(tmp_path)
    assert report['root_scripts'] == [{'path': 'scripts/leaf.py', 'status': 'unknown', 'path_risks': ['__file__']}]
    assert not report['consumer_scan_available']
