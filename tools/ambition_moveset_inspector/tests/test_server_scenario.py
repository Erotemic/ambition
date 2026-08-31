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


def test_a_mirror_match_is_still_a_scenario() -> None:
    """⛔⛔ A TARGET THAT EQUALS THE SUBJECT IS STILL A TARGET.

    The recorder DEFAULTS to a mirror match, so `--target-behavior cpu` with no
    explicit `--target` is an ordinary, supported scenario: George vs George,
    CPU. The key skipped the whole scenario clause whenever `target ==
    character`, so a CPU mirror and a passive mirror shared one cache directory
    and the CPU take was shown beside a render of a target standing still.

    ⭐ REPRODUCED FROM THE 2026-08-31 review before the fix: both keys were
    `george__jab__at40_000`.

    ⚠ The `None` case is genuinely different and stays collapsed — no opponent
    has nothing to behave — which is why omission must never be how a mirror is
    expressed.
    """
    passive = server.scenario_key("george", "jab", "george", 40.0, "passive")
    cpu = server.scenario_key("george", "jab", "george", 40.0, "cpu")
    assert passive != cpu, (
        "a CPU mirror and a passive mirror share a cache directory, so one is "
        "served as evidence for the other"
    )
    # …and the mirror is not the same experiment as having no opponent at all.
    assert passive != server.scenario_key("george", "jab", None, 40.0, "passive")


def test_canonical_scenario_makes_mirror_and_behavior_explicit() -> None:
    passive = server.CombatScenario.from_mapping(
        {"subject": "george", "verb": "attack", "target_behavior": "passive", "spacing": 40}
    )
    cpu = server.CombatScenario.from_mapping(
        {"subject": "george", "verb": "attack", "target_behavior": "cpu", "spacing": 40}
    )
    assert passive.target == "george"
    assert passive.document()["target"] == "george"
    assert passive.identity() != cpu.identity()
    assert passive.cache_name() != cpu.cache_name()


def test_canonical_chain_requires_an_explicit_schedule() -> None:
    import pytest

    with pytest.raises(ValueError, match="chain_at is required"):
        server.CombatScenario.from_mapping(
            {"subject": "george", "verb": "attack", "target": "george", "chain": "smash_down"}
        )
    scenario = server.CombatScenario.from_mapping(
        {
            "subject": "george",
            "verb": "attack",
            "target": "george",
            "chain": {"verb": "smash_down", "at": 37},
        }
    )
    assert scenario.document()["chain"] == {"verb": "smash_down", "at": 37}
    assert not scenario.renderable


def test_chain_render_is_refused_before_invoking_renderer(monkeypatch) -> None:
    scenario = server.CombatScenario.from_mapping(
        {
            "subject": "george",
            "verb": "attack",
            "target": "george",
            "chain": {"verb": "smash_down", "at": 37},
        }
    )
    monkeypatch.setattr(server, "render_animation", lambda *a, **kw: (_ for _ in ()).throw(AssertionError()))
    status, doc = server.render_scenario(scenario, 75, 2)
    assert status == 422
    assert doc["state"] == "unsupported"
    assert doc["scenario"] == scenario.document()


def test_single_take_generation_uses_exact_scenario_and_scenario_cache(tmp_path, monkeypatch) -> None:
    import json
    import os
    from types import SimpleNamespace

    fake_binary = tmp_path / "moveset_takes"
    fake_binary.write_text("fake")
    os.chmod(fake_binary, 0o755)
    cache = tmp_path / "takes"
    scenario = server.CombatScenario.from_mapping(
        {
            "subject": "george",
            "target": "george",
            "target_behavior": "cpu",
            "verb": "smash_down",
            "spacing": 40,
        }
    )
    calls = []

    def fake_run(command, **kwargs):
        calls.append(command)
        out = command[command.index("--out") + 1]
        take = {
            "character": "george",
            "subject": "george",
            "target": "george",
            "target_behavior": "cpu",
            "verb": "smash_down",
            "requested_spacing": 40.0,
            "frames": [],
        }
        pathlib.Path(out).write_text(json.dumps({"schema": "ambition.moveset_takes.v2", "sim_hz": 60, "takes": [take]}))
        return SimpleNamespace(returncode=0, stdout="generated one take", stderr="")

    monkeypatch.setattr(server, "TAKE_CACHE", cache)
    monkeypatch.setattr(server, "find_binary", lambda name: fake_binary if name == "moveset_takes" else None)
    monkeypatch.setattr(server, "_repository_identity", lambda: "source-1")
    monkeypatch.setattr(server, "_bulk_take", lambda scenario: None)
    monkeypatch.setattr(server.subprocess, "run", fake_run)

    status, doc = server.take_evidence(scenario)
    assert status == 200
    assert doc["scenario"] == scenario.document()
    command = calls[0]
    assert command[command.index("--characters") + 1] == "george"
    assert command[command.index("--verbs") + 1] == "smash_down"
    assert command[command.index("--target") + 1] == "george"
    assert command[command.index("--target-behavior") + 1] == "cpu"
    assert command[command.index("--spacing") + 1] == "40.0"
    assert (cache / scenario.cache_name() / "evidence.json").exists()

    calls.clear()
    status2, doc2 = server.take_evidence(scenario)
    assert status2 == 200
    assert doc2["cache_hit"] is True
    assert calls == [], "a current scenario-addressed take should not rerun the generator"

    # Regeneration is semantic: it must execute the generator again.
    status3, _ = server.take_evidence(scenario, force=True)
    assert status3 == 200
    assert len(calls) == 1


def test_render_horizon_is_not_a_hard_coded_short_capture() -> None:
    assert server.render_frames_for_horizon(149, 2) == 75
    assert (server.render_frames_for_horizon(149, 2) - 1) * 2 == 148
    assert server.render_frames_for_horizon(150, 2) == 76


def test_cpu_and_passive_mirrors_drive_distinct_renderer_commands(tmp_path, monkeypatch) -> None:
    import json
    import os
    from types import SimpleNamespace

    fake_binary = tmp_path / "moveset_render"
    fake_binary.write_text("fake")
    os.chmod(fake_binary, 0o755)
    renders = tmp_path / "renders"
    calls = []

    def fake_run(command, **kwargs):
        calls.append(command)
        out_dir = pathlib.Path(command[command.index("--out") + 1])
        out_dir.mkdir(parents=True, exist_ok=True)
        behavior = command[command.index("--target-behavior") + 1]
        spacing = float(command[command.index("--spacing") + 1])
        (out_dir / "manifest.json").write_text(json.dumps({
            "frames": 4,
            "stride": 2,
            "target": "george",
            "target_behavior": behavior,
            "requested_spacing": spacing,
            "intended_move": "jab",
            "observed_moves": ["jab"],
            "reached_intended_move": True,
            "shots": [{"file": "000.png", "action_tick": 0, "sim_tick": 100}],
        }))
        return SimpleNamespace(returncode=0, stdout="", stderr="")

    monkeypatch.setattr(server, "RENDERS", renders)
    monkeypatch.setattr(server, "find_renderer", lambda: fake_binary)
    monkeypatch.setattr(server, "_repository_identity", lambda: "source-1")
    monkeypatch.setattr(server.subprocess, "run", fake_run)

    passive = server.CombatScenario.from_mapping({
        "subject": "george", "target": "george", "target_behavior": "passive",
        "verb": "attack", "spacing": 40,
    })
    cpu = server.CombatScenario.from_mapping({
        "subject": "george", "target": "george", "target_behavior": "cpu",
        "verb": "attack", "spacing": 40,
    })

    passive_status, passive_doc = server.render_scenario(passive, 4, 2, force=True)
    cpu_status, cpu_doc = server.render_scenario(cpu, 4, 2, force=True)
    assert passive_status == cpu_status == 200
    assert passive_doc["scenario"] == passive.document()
    assert cpu_doc["scenario"] == cpu.document()
    assert passive_doc["scenario_id"] != cpu_doc["scenario_id"]
    assert len(calls) == 2
    assert calls[0][calls[0].index("--target") + 1] == "george"
    assert calls[1][calls[1].index("--target") + 1] == "george"
    assert calls[0][calls[0].index("--target-behavior") + 1] == "passive"
    assert calls[1][calls[1].index("--target-behavior") + 1] == "cpu"


def test_stale_evidence_never_becomes_a_current_cache_hit(tmp_path, monkeypatch) -> None:
    binary = tmp_path / "moveset_takes"
    binary.write_text("fake")
    scenario = server.CombatScenario.from_mapping({
        "subject": "george", "target": "george", "verb": "attack", "spacing": 40,
    })
    monkeypatch.setattr(server, "_repository_identity", lambda: "source-1")
    doc = {
        "scenario": scenario.document(),
        "scenario_id": scenario.identity(),
        "source_identity": "source-1",
        "generator": {"mtime": binary.stat().st_mtime},
        "stale": "bulk corpus provenance could not be revalidated",
    }
    assert not server._evidence_is_current(doc, scenario, binary)
