"""ECS browser uses generated evidence without inventing runtime facts."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import ecs_browser


class BrowserFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.ecs_dir = self.root / '.agent' / 'ecs_inventory'
        self.index_dir = self.root / '.agent' / 'index'
        (self.ecs_dir / 'crates').mkdir(parents=True)
        self.index_dir.mkdir(parents=True)
        source = self.root / 'crates' / 'foo' / 'src' / 'lib.rs'
        source.parent.mkdir(parents=True)
        source.write_text('\n'.join(['// source line'] * 80), encoding='utf-8')
        for name in ['foo', 'bar']:
            (self.root / 'crates' / name / 'src').mkdir(parents=True, exist_ok=True)
            (self.root / 'crates' / name / 'Cargo.toml').write_text('[package]\nname="'+name+'"\n', encoding='utf-8')
        self.project = {
            'schema_version': 4, 'crate_count': 2, 'counts': {}, 'crates': [
                {'crate_name': 'foo', 'json': 'crates/foo.json'},
                {'crate_name': 'bar', 'json': 'crates/bar.json'},
            ],
        }
        self.foo = {
            'schema_version': 4, 'crate_name': 'foo', 'crate_root': 'crates/foo',
            'counts': {'registrations': 1}, 'message_bus': {
                'Update': {
                    'read_by': ['tick (crates/foo/src/lib.rs:21)'],
                    'written_by': [], 'registered_at': ['crates/foo/src/lib.rs:15'],
                },
            },
            'ecs_items': [
                {'name': 'Body', 'derives': ['Component'], 'file': 'crates/foo/src/lib.rs', 'line': 3},
                {'name': 'Authority', 'derives': ['Resource'], 'file': 'crates/foo/src/lib.rs', 'line': 5},
                {'name': 'Update', 'derives': ['Message'], 'file': 'crates/foo/src/lib.rs', 'line': 7},
            ],
            'system_like_functions': [{
                'name': 'tick', 'file': 'crates/foo/src/lib.rs', 'line': 21,
                'queries': ['(&Body, &mut Body), With<Body>', '(&bar::Body, &Other)'],
                'resources_read': ['Authority'], 'resources_written': [],
                'messages_read': ['Update'], 'messages_written': [],
                'params': 'commands: Query<&Body>', 'commands': False,
            }],
            'registrations': [{
                'kind': 'add_systems', 'file': 'crates/foo/src/lib.rs', 'line': 15,
                'context': 'CorePlugin::build', 'systems': ['tick'],
            }],
            'plugins': [{'name': 'CorePlugin', 'file': 'crates/foo/src/lib.rs', 'line': 10}],
            'spawn_sites': [{
                'file': 'crates/foo/src/lib.rs', 'line': 31, 'expression': '(Body,)',
                'matched_ecs_items': ['Body'], 'name_labels': ['"Local body"'],
            }],
        }
        self.bar = {
            'schema_version': 4, 'crate_name': 'bar', 'crate_root': 'crates/bar',
            'counts': {'registrations': 0}, 'message_bus': {},
            'ecs_items': [{'name': 'Body', 'derives': ['Component'],
                           'file': 'crates/bar/src/lib.rs', 'line': 1}],
            'system_like_functions': [], 'registrations': [],
            'plugins': [], 'spawn_sites': [],
        }
        self._write()

    def _write(self) -> None:
        def dump(path: Path, data: dict) -> None:
            path.write_text(json.dumps(data), encoding='utf-8')
        dump(self.ecs_dir / 'project.json', self.project)
        dump(self.ecs_dir / 'crates' / 'foo.json', self.foo)
        dump(self.ecs_dir / 'crates' / 'bar.json', self.bar)
        dump(self.index_dir / 'generation_stamp.json', {
            'generated_from_commit': 'a1b2c3d4e5f6', 'generated_at': 'stable source time',
        })

    def build(self, allow_stale: bool = False):
        with mock.patch.object(ecs_browser, 'ROOT', self.root), \
             mock.patch.object(ecs_browser, 'INVENTORY', self.ecs_dir), \
             mock.patch.object(ecs_browser, 'INDEX', self.index_dir), \
             mock.patch.object(ecs_browser, '_git_head', return_value=None):
            return ecs_browser.create_model(allow_stale=allow_stale)

    def test_existing_inventory_powers_system_to_component_to_spawn_links(self) -> None:
        data = self.build()
        self.assertEqual(2, len(data['sections']['components']))
        sys = data['sections']['systems'][0]
        self.assertEqual(2, sys['queryCount'])
        self.assertTrue(sys['registeredHere'])
        self.assertEqual(1, len(sys['registrations']))
        body = next(r for r in data['sections']['components'] if r['crate'] == 'foo')
        other = next(r for r in data['sections']['components'] if r['crate'] == 'bar')
        self.assertEqual([sys['id']], body['systems'])
        self.assertEqual([sys['id']], other['systems'])
        self.assertEqual(1, len(body['spawnIds']))
        self.assertEqual(1, len(data['sections']['plugins'][0]['registrations']))
        self.assertEqual([sys['id']], data['sections']['channels'][0]['systems'])
        self.assertEqual([sys['id']], data['sections']['resources'][0]['systems'])

    def test_ambiguous_base_names_do_not_get_cross_linked(self) -> None:
        data = self.build()
        raw = ecs_browser._resolve_type('Body', 'unknown', {
            'Body': data['sections']['components'],
        }, 'Component')
        self.assertEqual([], raw)
        raw = ecs_browser._resolve_type('some_alias::Body', 'foo', {
            'Body': data['sections']['components'],
        }, 'Component')
        self.assertEqual([], raw)
        raw = ecs_browser._resolve_type('crate::Body', 'foo', {
            'Body': data['sections']['components'],
        }, 'Component')
        self.assertEqual([next(x['id'] for x in data['sections']['components']
                               if x['crate'] == 'foo')], raw)

    def test_safe_html_embedding_and_determinism(self) -> None:
        self.foo['system_like_functions'][0]['params'] = '"</script><script>evil()</script>"'
        self._write()
        data = self.build()
        out = self.root / 'browser' / 'index.html'
        ecs_browser.write_browser(out, data)
        html = out.read_text(encoding='utf-8')
        self.assertIn('"ecs-data"', html)
        self.assertIn('\\u003c/script\\u003e', html)
        self.assertNotIn('</script><script>evil()', html)
        baseline = out.read_bytes()
        ecs_browser.write_browser(out, self.build())
        self.assertEqual(baseline, out.read_bytes())

    def test_stale_index_requires_override(self) -> None:
        with mock.patch.object(ecs_browser, 'ROOT', self.root), \
             mock.patch.object(ecs_browser, 'INVENTORY', self.ecs_dir), \
             mock.patch.object(ecs_browser, 'INDEX', self.index_dir), \
             mock.patch.object(ecs_browser, '_git_head', return_value='ffffffffffff'):
            with self.assertRaisesRegex(ValueError, 'Regenerate'):
                ecs_browser.create_model()
            data = ecs_browser.create_model(allow_stale=True)
            self.assertTrue(data['meta']['stale'])

    def test_no_source_traversal(self) -> None:
        with mock.patch.object(ecs_browser, 'ROOT', self.root):
            self.assertIsNone(ecs_browser._repo_file('../outside.txt'))


class QueryReferenceTest(unittest.TestCase):
    def test_read_write_filter_extraction(self) -> None:
        result = ecs_browser._query_mentions([
            '(Entity, &Foo, Option<&mut Bar>), (With<Baz>, Without<Qux>)',
        ])
        self.assertEqual([('Foo','read'),('Bar','write'),('Baz','with'),('Qux','without')],
                         [(r['name'],r['role']) for r in result])


if __name__ == '__main__':
    unittest.main()
