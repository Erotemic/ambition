"""Check that the test audit scans only the selected checkout."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'audit_test_corpus.py'
spec = importlib.util.spec_from_file_location('audit_test_corpus', SCRIPT)
assert spec is not None and spec.loader is not None
scan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scan)


def write_rust(root: Path, relative: str) -> Path:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('#[test]\nfn proves_something() { assert_eq!(2 + 2, 4); }\n')
    return path


class TestCheckoutBoundaries(unittest.TestCase):
    def test_filesystem_scan_prunes_worktrees_nested_git_and_build_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            wanted = write_rust(root, 'game/ambition_app/tests/real.rs')
            write_rust(root, '.worktrees/agent-worktree1/game/ambition_app/tests/copied.rs')
            write_rust(root, 'worktrees/other/tests/copied.rs')
            write_rust(root, 'target/debug/very_large.rs')
            nested = write_rust(root, 'scratch-checkout/tests/copied.rs')
            (root / 'scratch-checkout/.git').write_text('gitdir: elsewhere\n')
            (root / 'linked-checkout').symlink_to(nested.parents[1], target_is_directory=True)

            sources, skipped = scan.rust_source_files(root, root)
            self.assertEqual(sources, [wanted])
            excluded = {item['path']: item['reason'] for item in skipped}
            self.assertEqual(excluded['.worktrees'], 'alternate worktree')
            self.assertEqual(excluded['worktrees'], 'alternate worktree')
            self.assertEqual(excluded['target'], 'build or metadata directory')
            self.assertEqual(excluded['scratch-checkout'], 'nested Git checkout')
            self.assertEqual(excluded['linked-checkout'], 'symlink directory')

            report_dir = root / 'report'
            process = subprocess.run(
                [sys.executable, str(SCRIPT), '--root', str(root), '--scope', '.',
                 '--out', str(report_dir), '--window', '3'],
                capture_output=True, text=True, check=True,
            )
            self.assertIn('filesystem-pruned', process.stdout)
            report = json.loads((report_dir / 'audit.json').read_text())
            self.assertEqual(report['summary']['files'], 1)
            self.assertEqual(report['summary']['test_functions'], 1)
            self.assertEqual(report['summary']['excluded_trees'], 5)
            self.assertFalse(any('.worktrees' in f['path'] for f in report['files']))

    def test_scopes_cannot_select_an_alternate_worktree(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            wanted = write_rust(root, 'game/ambition_app/tests/real.rs')
            write_rust(root, '.worktrees/alt/tests/copied.rs')
            found, _ = scan.rust_source_files(root, root / 'game' / 'ambition_app')
            self.assertEqual(found, [wanted])
            with self.assertRaisesRegex(ValueError, 'excluded directory'):
                scan.rust_source_files(root, root / '.worktrees' / 'alt')

    def test_tracked_only_uses_git_index_and_ignores_untracked_tests(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            wanted = write_rust(root, 'game/ambition_app/tests/tracked.rs')
            write_rust(root, 'game/ambition_app/tests/untracked.rs')
            write_rust(root, '.worktrees/alt/tests/copied.rs')
            subprocess.run(['git', '-C', str(root), 'add', 'game/ambition_app/tests/tracked.rs'], check=True)

            sources, _ = scan.rust_source_files(root, root, tracked_only=True)
            self.assertEqual(sources, [wanted])
            filesystem_sources, _ = scan.rust_source_files(root, root)
            self.assertEqual(len(filesystem_sources), 2)


if __name__ == '__main__':
    unittest.main()
