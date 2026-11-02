"""Regression and input-contract tests for the standalone worldbuilding lab."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import worldlab  # noqa: E402


class CatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = worldlab.load_catalog()
        cls.scenarios = worldlab.load_scenarios(worldlab.DEFAULT_SCENARIOS, cls.catalog)

    def test_input_catalog_is_complete_and_unique(self):
        self.assertEqual(len(self.catalog["records"]), 144)
        self.assertEqual(len(self.catalog["features"]), 15)
        self.assertEqual(len(set(r["id"] for r in self.catalog["records"])), 144)
        originals = [r for r in self.catalog["records"] if r["origin"] == "original"]
        self.assertGreaterEqual(len(originals), 17)
        self.assertTrue(all(r["source_scope"] for r in originals))

    def test_all_scope_reproduces_previous_pca_fractions(self):
        fam = worldlab.family_data(self.catalog, "all")
        self.assertEqual(len(fam.names), 31)
        self.assertEqual(sum(fam.sizes.values()), 142)
        result = worldlab.pca(fam)
        np.testing.assert_allclose(result["variance"][:3],
                                   [0.22504517156, 0.16109458554, 0.14110491064],
                                   atol=1e-7)
        self.assertAlmostEqual(float(sum(result["variance"])), 1.0, places=10)
        self.assertGreater(float(result["loadings"][0, fam.features.index("creative")]), 0)

    def test_scope_provenance_filters(self):
        original = worldlab.selected_records(self.catalog, "original")
        core = worldlab.selected_records(self.catalog, "core")
        expanded = worldlab.selected_records(self.catalog, "expanded")
        all_rows = worldlab.selected_records(self.catalog, "all")
        self.assertLess(len(original), len(core))
        self.assertLessEqual(len(core), len(expanded))
        self.assertLess(len(expanded), len(all_rows))
        self.assertNotIn("llm", {r["origin"] for r in core})
        self.assertNotIn("conversation_proposed", {r["origin"] for r in core})

    def test_duplicate_or_unknown_annotations_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory) / "extra.json"
            duplicate = dict(self.catalog["records"][0])
            p.write_text(json.dumps({"schema_version": 1, "records": [duplicate]}))
            with self.assertRaisesRegex(worldlab.InputError, "Duplicate concept"):
                worldlab.load_catalog(supplements=(p,))
            altered = {**duplicate, "id": "new-record", "tags": ["unregistered_attribute"]}
            p.write_text(json.dumps({"schema_version": 1, "records": [altered]}))
            with self.assertRaisesRegex(worldlab.InputError, "unknown features"):
                worldlab.load_catalog(supplements=(p,))

    def test_supplement_can_extend_without_mutating_base(self):
        extra = ROOT / "examples" / "example_supplement.json"
        catalog = worldlab.load_catalog(supplements=(extra,))
        self.assertEqual(len(catalog["records"]), 147)
        self.assertEqual(len(self.catalog["records"]), 144)
        self.assertIn("hypothesis", {r["origin"] for r in catalog["records"]})

    def test_scenarios_keep_math_distinct(self):
        for scenario in self.scenarios["scenarios"]:
            math = next(w for w in scenario["worlds"] if w["id"] == "math")
            self.assertEqual(math["families"], ["Formal mathematics"])
        changed = json.loads(json.dumps(self.scenarios))
        changed["scenarios"][0]["worlds"][5]["families"].append("Rhetoric & philosophy")
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory) / "bad.json"
            p.write_text(json.dumps(changed))
            with self.assertRaisesRegex(worldlab.InputError, "protected Math"):
                worldlab.load_scenarios(p, self.catalog)


class AnalysisTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = worldlab.load_catalog()
        cls.data = worldlab.family_data(cls.catalog, "all")
        cls.scenarios = worldlab.load_scenarios(worldlab.DEFAULT_SCENARIOS, cls.catalog)

    def test_nmf_nonnegative_and_repeatable(self):
        a = worldlab.nmf(self.data, factors=5, seed=11, steps=600)
        b = worldlab.nmf(self.data, factors=5, seed=11, steps=600)
        np.testing.assert_array_equal(a["W"], b["W"])
        np.testing.assert_array_equal(a["H"], b["H"])
        self.assertTrue(np.all(a["W"] >= 0))
        self.assertTrue(np.all(a["H"] >= 0))
        self.assertLess(a["relative_residual"], 0.8)
        self.assertGreater(a["relative_residual"], 0)

    def test_bootstrap_is_seeded_and_bounded(self):
        a = worldlab.bootstrap_pca(self.data, repeats=12, seed=41)
        b = worldlab.bootstrap_pca(self.data, repeats=12, seed=41)
        self.assertEqual(a, b)
        for value in a["max_top3_loading_alignment_median"]:
            self.assertTrue(0 <= value <= 1.000001)

    def test_scenario_comparison_and_protected_placement(self):
        cases = {s["id"]: s for s in self.scenarios["scenarios"]}
        historical = worldlab.scenario_metrics(self.data, cases["historical-17"])
        merged = worldlab.scenario_metrics(self.data, cases["consolidated"])
        self.assertEqual(historical["world_count"], 17)
        self.assertLess(historical["family_coverage"], 1)
        self.assertEqual(merged["family_coverage"], 1)
        self.assertTrue("Creative culture" in historical["unassigned_families"])
        suggestion = worldlab.placements(self.data, cases["consolidated"], limit=200)
        self.assertTrue(suggestion)
        self.assertTrue(all(s["world_id"] != "math" for s in suggestion))
        self.assertTrue(all(s["family"] != "Formal mathematics" for s in suggestion))
        self.assertTrue(all(s["assigned_elsewhere"] for s in suggestion))
        self.assertEqual(worldlab.placements(self.data, cases["consolidated"], only_unassigned=True), [])

    def test_nearest_and_fusion_are_deterministic(self):
        neighbors=worldlab.nearest(self.data,"Creative culture",limit=4)
        self.assertEqual(len(neighbors),4)
        self.assertTrue(all(n["family"]!="Creative culture" for n in neighbors))
        mixes=worldlab.fusions(self.data,anchor="Creative culture",limit=5)
        self.assertEqual(len(mixes),5)
        self.assertTrue(all("Creative culture" in (r["first"],r["second"]) for r in mixes))
        self.assertTrue(all(0 <= r["score"] <= 1.00001 for r in mixes))

    def test_analyze_outputs_are_deterministic_and_readable(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory=Path(temporary)
            catalog=self.catalog
            for name in ("first", "second"):
                worldlab.write_analysis(directory/name,self.data,catalog,self.scenarios,
                                        scope="all",scenario_id="consolidated",factors=5,seed=12,bootstrap=8)
            a=directory/"first"
            b=directory/"second"
            names={"report.md","world_scenarios.md","fusions.md","placements.md","analysis.json","pca.svg","family_coords.csv"}
            self.assertEqual({p.name for p in a.iterdir()},names)
            for fname in names:
                self.assertEqual((a/fname).read_bytes(),(b/fname).read_bytes())
            info=json.loads((a/"analysis.json").read_text())
            self.assertEqual(info["scope"],"all")
            self.assertEqual(len(info["family_names"]),31)
            self.assertTrue((a/"pca.svg").read_text().startswith('<svg xmlns="http://www.w3.org/2000/svg"'))

    def test_cli_validate_and_bad_family(self):
        ok=subprocess.run([sys.executable,str(ROOT/"worldlab.py"),"validate"],capture_output=True,text=True,check=False)
        self.assertEqual(ok.returncode,0,ok.stderr)
        bad=subprocess.run([sys.executable,str(ROOT/"worldlab.py"),"neighbors","Not a real family"],
                           capture_output=True,text=True,check=False)
        self.assertEqual(bad.returncode,2,bad.stdout)
        self.assertIn("Unknown active family",bad.stderr)


if __name__=="__main__":
    unittest.main()
