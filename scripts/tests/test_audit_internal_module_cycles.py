"""Minimal fixture coverage for the workspace module-cycle auditor."""
from __future__ import annotations

import importlib.util
import subprocess
from pathlib import Path
import tempfile
import sys
import textwrap
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'audit_internal_module_cycles.py'
spec = importlib.util.spec_from_file_location('audit_internal_module_cycles', SCRIPT)
audit = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = audit
spec.loader.exec_module(audit)


class CycleAuditTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'crates' / 'sample' / 'src' / 'a').mkdir(parents=True)
        self.write('Cargo.toml', '''
            [workspace]
            members = ["crates/sample"]
        ''')
        self.write('crates/sample/src/lib.rs', '''
            pub mod a;
            pub mod b;
            pub mod c;
            #[cfg(test)] mod test_only;
            pub use a::PublicThing;
        ''')
        self.write('crates/sample/src/a.rs', '''
            pub struct PublicThing;
            pub mod left;
            pub mod right;
            use crate::{b::{Thing, Other}};
            #[cfg(test)] mod tests { use crate::c::Nothing; }
        ''')
        self.write('crates/sample/src/a/left.rs', 'use super::right::Right;\n')
        self.write('crates/sample/src/a/right.rs', 'use super::left::Left;\n')
        self.write('crates/sample/src/b.rs', 'use crate::PublicThing;\npub struct Thing;\n')
        self.write('crates/sample/src/c.rs', 'pub struct Nothing;\n')
        self.write('crates/sample/src/test_only.rs', 'use crate::c::Nothing;\n')

    def write(self, relative, source):
        target = self.root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(textwrap.dedent(source))

    def test_root_alias_and_braced_use_detect_real_cycle(self):
        result = audit.scan_crate(self.root, 'crates/sample')
        self.assertEqual([['a', 'b']], result.cycles)
        self.assertNotIn('test_only', result.modules)
        self.assertNotIn('c', result.edges.get('a', {}))
        self.assertEqual(result.edges['b']['a'], 1)

    def test_nested_sibling_cycle(self):
        nested = audit.scan_nested_sibling_cycles(self.root, 'crates/sample')
        self.assertEqual(len(nested), 1)
        self.assertEqual(nested[0]['parent'], 'a')
        self.assertEqual(nested[0]['modules'], ['left', 'right'])

    def test_nested_only_crate_appears_in_cli_details(self):
        # Break the top-level a <-> b cycle but keep the nested left/right one.
        self.write('crates/sample/src/b.rs', 'pub struct Thing;\n')
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), '--root', str(self.root),
             '--crate', 'sample', '--top', '1', '--details'],
            check=True, capture_output=True, text=True,
        )
        self.assertIn('Crates with cyclic top-level modules: 0', completed.stdout)
        self.assertIn('Crates with any detected module cycle: 1', completed.stdout)
        self.assertIn('NESTED a (2): left, right', completed.stdout)

    def test_test_gated_function_does_not_create_cycle(self):
        self.write('crates/sample/src/a.rs', '''
            pub struct PublicThing;
            #[cfg(test)] fn tests_only() { let _ = crate::b::Thing; }
        ''')
        result = audit.scan_crate(self.root, 'crates/sample')
        self.assertEqual(result.cycles, [])

    def test_ignore_comment_doc_string_and_raw_string(self):
        self.write('crates/sample/src/b.rs', '''
            pub struct Thing;
            // use crate::a::PublicThing;
            /* crate::a::PublicThing */
            const EXAMPLE: &str = r#"crate::a::PublicThing"#;
        ''')
        result = audit.scan_crate(self.root, 'crates/sample')
        self.assertEqual(result.cycles, [])
        self.assertNotIn('a', result.edges.get('b', {}))


if __name__ == '__main__':
    unittest.main()
