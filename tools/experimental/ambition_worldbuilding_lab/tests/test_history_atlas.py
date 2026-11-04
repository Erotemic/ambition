"""The History atlas keeps its promises: the data is whole, it agrees with
the game, and every player has an answer to "where's my history?"."""

from __future__ import annotations

from collections import Counter
from pathlib import Path
import sys
import unittest

import networkx as nx

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import history_atlas as ha  # noqa: E402


class AtlasTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.atlas = ha.load()

    def test_the_data_is_whole_and_agrees_with_the_game(self):
        # The game half needs the world file; without it the data is still checked.
        self.assertEqual(ha.check(self.atlas, against_game=ha.WORLD.exists()), [])

    def test_every_country_has_at_least_a_planned_place(self):
        weak = [c["name"] for c in ha.country_coverage(self.atlas) if c["best"] not in ("built", "planned")]
        self.assertEqual(weak, [], "a player from here has no planned answer")

    def test_every_people_has_at_least_a_planned_place(self):
        weak = [p["name"] for p in ha.people_coverage(self.atlas) if p["best"] not in ("built", "planned")]
        self.assertEqual(weak, [])

    def test_no_topic_is_left_to_the_candidates(self):
        weak = [t["name"] for t in ha.topic_coverage(self.atlas) if t["best"] not in ("built", "planned")]
        self.assertEqual(weak, [], "an obvious topic we are neglecting, or a neglected one we still neglect")

    def test_the_built_and_planned_places_are_one_world(self):
        g = ha.graph(self.atlas, ("built", "planned"))
        parts = list(nx.connected_components(g))
        self.assertEqual(len(parts), 1, sorted(parts, key=len)[0])

    def test_every_cluster_has_a_planned_place_in_most_horizons(self):
        # "Do not treat an empty age as a time when no people lived there":
        # each part of the world reaches at least 7 of the 11 horizons.
        m = ha.matrix(self.atlas)
        for cluster in self.atlas["frame"]["clusters"]:
            if cluster == "seas":
                continue
            reached = {h for (c, h), n in m.items() if c == cluster and (n["built"] or n["planned"])}
            self.assertGreaterEqual(len(reached), 7, f"{cluster}: {sorted(reached)}")

    def test_a_sober_place_is_not_a_parody(self):
        sober_topics = {t for t, spec in self.atlas["topics"].items() if spec["kind"] == "sober"}
        for n in self.atlas["nodes"].values():
            if n["mode"] == "parody":
                self.assertFalse(set(n["topics"]) & sober_topics, n["id"])

    def test_the_floor_order_puts_heavy_links_side_by_side(self):
        w = Counter({frozenset("ab"): 5, frozenset("bc"): 4, frozenset("ac"): 1, frozenset("cd"): 3})
        order, score = ha.best_floor_order(list("abcd"), w)
        self.assertEqual(score, 12)
        self.assertIn(order, (list("abcd"), list("dcba")))

    def test_a_country_answer_names_its_places(self):
        text = ha.where(self.atlas, "Puerto Rico")
        self.assertIn("The Taíno islands", text)
        self.assertIn("BUILT", text)


if __name__ == "__main__":
    unittest.main()
