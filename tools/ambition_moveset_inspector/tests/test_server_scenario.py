"""The inspector's cache identity: one directory per SCENARIO, not per name."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from ambition_moveset_inspector import server  # noqa: E402


def test_two_nearby_spacings_are_two_cache_entries() -> None:
    """⛔⛔ REPRODUCED BY THE 2026-08-31 REVIEW. The key used `int(spacing)`, so
    40.1 and 40.9 named ONE directory and the second request was served the
    first's pictures — under its own number, beside its own take, looking like
    evidence."""
    near = server.scenario_key("george", "attack_side", "goblin", 40.1, "passive")
    far = server.scenario_key("george", "attack_side", "goblin", 40.9, "passive")
    assert near != far, "40.1 and 40.9 share a cache directory"

    # ⛔ THE PREMISE. Identical requests must still share one entry, or the
    # assertion above passes because every key is unique and the cache is dead.
    assert near == server.scenario_key("george", "attack_side", "goblin", 40.1, "passive")


def test_target_behavior_is_part_of_the_identity() -> None:
    """A take recorded against a live CPU and a render of a target standing
    still are two fights. `moveset_render` DEFAULTS a missing behaviour to
    passive, so a key that ignored it served one as the other."""
    passive = server.scenario_key("george", "jab", "goblin", 40.0, "passive")
    cpu = server.scenario_key("george", "jab", "goblin", 40.0, "cpu")
    assert passive != cpu

    # ⚠ …but a scenario with NO opponent has nothing to behave, and stamping a
    # behaviour into that key would split one experiment into two.
    solo_a = server.scenario_key("george", "jab", None, 40.0, "passive")
    solo_b = server.scenario_key("george", "jab", None, 40.0, "cpu")
    assert solo_a == solo_b


def test_a_cached_manifest_from_another_scenario_is_not_a_hit() -> None:
    """The value comparison the cache validates with: floats agree by tolerance
    (JSON request vs Rust `f32`), everything else exactly, and `None` only
    matches `None`."""
    assert server._same_scenario_value(40.0, 40.0000001)
    assert not server._same_scenario_value(40.0, 40.9)
    assert server._same_scenario_value("goblin", "goblin")
    assert not server._same_scenario_value("goblin", "npc_alice")
    assert server._same_scenario_value(None, None)
    assert not server._same_scenario_value(None, "goblin")
    assert not server._same_scenario_value(40.0, None)
