from pathlib import Path

from ambition_sprite2d_renderer.registry import CharacterJob
from ambition_sprite2d_renderer.authoring.sheet import write_spritesheet
from ambition_sprite2d_renderer.registry import discover_all_targets


CONFIGS = Path(__file__).resolve().parents[1] / "ambition_sprite2d_renderer" / "configs"


def test_write_spritesheet_emits_optional_actor_contract(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "goblin.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle", "walk", "slash"]
    image_path, manifest_path = write_spritesheet(
        job,
        tmp_path / "goblin_spritesheet.png",
        tmp_path / "goblin_spritesheet.yaml",
    )
    actor_path = tmp_path / "goblin_actor.ron"
    assert image_path.exists()
    assert manifest_path.exists()
    assert actor_path.exists()
    text = actor_path.read_text()
    assert "schema_version: 1" in text
    assert 'character_id: "goblin"' in text
    assert '"action.melee.primary"' in text
    assert "missing_information: []" in text
    assert "collision: Some" in text
    assert "hurtbox: Some" in text
    assert '"hand_r"' in text
    assert '"weapon_tip"' in text


def test_character_job_accepts_sparse_actor_contract_fields():
    job = CharacterJob.from_dict(
        {
            "target": "toon",
            "name": "Sparse Zombie",
            "output_name": "zombie_shambler",
            "actor": {"character_id": "npc_zombie_shambler"},
            "body": {"body_plan": "HumanoidBiped", "traits": ["undead", "no_hands"]},
            "capabilities": {"traversal": {"walk": True, "jump": None}},
            "brain": {"default_preset": "melee_brute_slow"},
            "actions": {"default_preset": "zombie_bite"},
            "sockets": {"mouth": {"point": {"x": 20.0, "y": 30.0}}},
        }
    )
    assert job.actor["character_id"] == "npc_zombie_shambler"
    assert job.body["traits"] == ["undead", "no_hands"]
    assert "mouth" in job.sockets


def test_contract_derives_runtime_fields_from_body_metrics_without_requiring_hands():
    from ambition_sprite2d_renderer.authoring.actor_contract import build_actor_contract, to_ron

    manifest = {
        "target": "toon",
        "image": "zombie_shambler_spritesheet.png",
        "body_metrics": {
            "body_pixel_bbox": {"x": 5, "y": 10, "w": 30, "h": 50},
            "feet_pixel": {"x": 20, "y": 63},
        },
        "rows": [
            {
                "animation": "shamble_idle",
                "row_index": 0,
                "frame_count": 1,
                "duration_ms": 100,
                "rects": [],
            },
            {
                "animation": "bite",
                "row_index": 1,
                "frame_count": 1,
                "duration_ms": 100,
                "rects": [],
            },
        ],
    }
    ron = to_ron(
        build_actor_contract(
            stem="zombie_shambler",
            target="toon",
            image="zombie_shambler_spritesheet.png",
            sheet_manifest="zombie_shambler_spritesheet.ron",
            manifest=manifest,
            job_data={"surface": "adapter", "tags": []},
            authoring={
                "body": {
                    "body_plan": "HumanoidBiped",
                    "traits": ["undead", "no_hands"],
                },
                "actions": {"default_preset": "zombie_bite"},
                "sockets": {"mouth": {"point": {"x": 22.0, "y": 26.0}}},
            },
        )
    )
    assert "collision: Some" in ron
    assert "hurtbox: Some" in ron
    assert '"mouth"' in ron
    assert '"hand_r"' not in ron
    assert "melee origin socket" not in ron


def test_tackon_target_render_sheet_includes_actor_sidecar(tmp_path: Path):
    targets = discover_all_targets().targets
    target = targets["sandbag"]
    outputs = target.render_sheet(tmp_path)
    actor_path = tmp_path / "sandbag_actor.ron"
    assert actor_path.exists()
    assert actor_path in outputs
    text = actor_path.read_text()
    assert 'character_id: "sandbag"' in text
    assert 'default_preset: Some("sandbag_punch")' in text


def test_catalog_defaults_enrich_actor_contract_when_available(tmp_path: Path):
    from ambition_sprite2d_renderer.authoring.actor_contract import build_actor_contract, to_ron

    manifest = {
        "target": "toon",
        "image": "erdish_spritesheet.png",
        "rows": [
            {
                "animation": "idle",
                "row_index": 0,
                "frame_count": 1,
                "duration_ms": 100,
                "rects": [],
            },
            {
                "animation": "walk",
                "row_index": 1,
                "frame_count": 1,
                "duration_ms": 100,
                "rects": [],
            },
        ],
    }
    ron = to_ron(
        build_actor_contract(
            stem="erdish",
            target="toon",
            image="erdish_spritesheet.png",
            sheet_manifest="erdish_spritesheet.ron",
            manifest=manifest,
            job_data={"surface": "adapter", "tags": []},
            authoring={},
        )
    )
    assert 'character_id: "npc_erdish"' in ron
    assert 'display_name: Some("Erdish")' in ron
    assert 'default_preset: Some("patrol_peaceful")' in ron
    assert 'default_preset: Some("peaceful")' in ron


def test_json_manifest_targets_can_emit_actor_sidecar(tmp_path: Path):
    from ambition_sprite2d_renderer.registry import _ensure_actor_sidecars

    manifest_path = tmp_path / "mockingbird_boss_spritesheet_manifest.json"
    manifest_path.write_text(
        '{"target":"mockingbird_boss","rows":[{"name":"hover","frames":2,"duration_ms":120}]}',
        encoding="utf8",
    )
    outputs = _ensure_actor_sidecars(
        target_name="mockingbird_boss",
        render_dir=tmp_path,
        paths=[manifest_path],
        actor_metadata={
            "actor": {"character_id": "npc_mockingbird_boss"},
            "tags": ["boss"],
        },
    )
    actor_path = tmp_path / "mockingbird_boss_actor.ron"
    assert actor_path.exists()
    assert actor_path in outputs
    text = actor_path.read_text(encoding="utf8")
    assert 'character_id: "npc_mockingbird_boss"' in text
    assert '"locomotion.hover"' in text


def test_every_registered_character_target_advertises_actor_sidecar():
    targets = discover_all_targets().targets
    missing = []
    for name, target in targets.items():
        if target.category != "characters":
            continue
        if f"{name}_actor.ron" not in target.sheet_files:
            missing.append(name)
    assert missing == []


def _contract_text_for_metadata(
    stem: str, target: str, rows: list[str], metadata: dict
) -> str:
    from ambition_sprite2d_renderer.authoring.actor_contract import build_actor_contract, to_ron

    manifest = {
        "target": target,
        "image": f"{stem}_spritesheet.png",
        "body_metrics": {
            "body_pixel_bbox": {"x": 10, "y": 12, "w": 80, "h": 64},
            "feet_pixel": {"x": 50, "y": 76},
        },
        "rows": [
            {
                "animation": name,
                "row_index": i,
                "frame_count": 1,
                "duration_ms": 100,
                "rects": [],
            }
            for i, name in enumerate(rows)
        ],
    }
    return to_ron(
        build_actor_contract(
            stem=stem,
            target=target,
            image=f"{stem}_spritesheet.png",
            sheet_manifest=f"{stem}_spritesheet.ron",
            manifest=manifest,
            job_data={"surface": "tackon", "tags": []},
            authoring=metadata,
        )
    )


def test_bespoke_burning_flying_shark_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import (
        burning_flying_shark as shark,
    )

    ron = _contract_text_for_metadata(
        "burning_flying_shark",
        shark.TARGET_NAME,
        [name for name, _, _ in shark.ROWS],
        shark.ACTOR_METADATA,
    )
    assert 'character_id: "npc_burning_flying_shark"' in ron
    assert 'body_plan: Some("Flyer")' in ron
    assert "fly: Some(true)" in ron
    assert "walk: Some(false)" in ron
    assert '"mouth"' in ron
    assert '"saddle"' in ron
    assert '"action.melee.primary"' in ron
    assert '"action.special.dive"' in ron


def test_bespoke_puppy_slug_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import puppy_slug

    ron = _contract_text_for_metadata(
        "puppy_slug",
        puppy_slug.TARGET_NAME,
        [name for name, _, _ in puppy_slug.ROWS],
        puppy_slug.ACTOR_METADATA,
    )
    assert 'character_id: "npc_puppy_slug"' in ron
    assert 'body_plan: Some("Crawler")' in ron
    assert "climb: Some(true)" in ron
    assert "crawl: Some(true)" in ron
    assert '"mouth"' in ron
    assert '"wall_contact"' in ron
    assert '"hand_r"' not in ron
    assert '"locomotion.wall_crawl"' in ron


def test_bespoke_president_portrait_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import president_portrait

    ron = _contract_text_for_metadata(
        "president_portrait",
        president_portrait.TARGET_NAME,
        [name for name, _, _ in president_portrait.ROWS],
        president_portrait.ACTOR_METADATA,
    )
    assert 'character_id: "npc_president_portrait"' in ron
    assert "door_access: [" in ron
    assert '"public"' in ron
    assert "talk: Some(true)" in ron
    assert '"speech_bubble"' in ron
    assert '"decree_origin"' in ron
    assert '"interaction.oath"' in ron


def test_bespoke_ghoul_skulker_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import ghoul_skulker

    ron = _contract_text_for_metadata(
        "ghoul_skulker",
        ghoul_skulker.TARGET_NAME,
        [name for name, _, _ in ghoul_skulker.ROWS],
        ghoul_skulker.ACTOR_METADATA,
    )
    assert 'character_id: "npc_ghoul_skulker"' in ron
    assert 'body_kind: Some("LowProfile")' in ron
    assert "crawl: Some(true)" in ron
    assert '"claw_tip"' in ron
    assert '"action.special.pounce"' in ron


def test_bespoke_mantis_lancer_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import mantis_lancer

    ron = _contract_text_for_metadata(
        "mantis_lancer",
        mantis_lancer.TARGET_BASENAME,
        [name for name, _, _ in mantis_lancer.ROWS],
        mantis_lancer.ACTOR_METADATA,
    )
    assert 'character_id: "npc_mantis_lancer"' in ron
    assert 'body_plan: Some("InsectoidBiped")' in ron
    assert "climb: Some(true)" in ron
    assert '"blade_tip"' in ron
    assert '"action.melee.sweep"' in ron


def test_bespoke_raptor_stalker_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import raptor_stalker

    ron = _contract_text_for_metadata(
        "raptor_stalker",
        raptor_stalker.TARGET_BASENAME,
        [name for name, _, _ in raptor_stalker.ROWS],
        raptor_stalker.ACTOR_METADATA,
    )
    assert 'character_id: "npc_raptor_stalker"' in ron
    assert 'body_plan: Some("BeastBiped")' in ron
    assert '"mouth"' in ron
    assert '"tail_tip"' in ron
    assert '"hand_r"' not in ron
    assert '"action.melee.tail_sweep"' in ron


def test_bespoke_trex_enemy_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import trex_enemy

    ron = _contract_text_for_metadata(
        "trex_enemy",
        trex_enemy.TARGET_NAME,
        [name for name, _, _ in trex_enemy.ROWS],
        trex_enemy.ACTOR_METADATA,
    )
    assert 'character_id: "npc_trex_enemy"' in ron
    assert 'body_kind: Some("Wide")' in ron
    assert 'mass_class: Some("Heavy")' in ron
    assert '"roar_origin"' in ron
    assert '"action.melee.stomp"' in ron


def test_bespoke_smart_house_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import smart_house

    ron = _contract_text_for_metadata(
        "smart_house",
        smart_house.TARGET_NAME,
        [name for name, _, _ in smart_house.ROWS],
        smart_house.ACTOR_METADATA,
    )
    assert 'character_id: "npc_smart_house"' in ron
    assert 'body_plan: Some("PropActor")' in ron
    assert "walk: Some(true)" in ron
    assert "talk: Some(true)" in ron
    assert '"speech_bubble"' in ron
    assert '"action.special.ram"' in ron


def test_bespoke_flying_spaghetti_monster_boss_actor_metadata():
    from ambition_sprite2d_renderer.targets.characters import (
        flying_spaghetti_monster_boss as fsm,
    )

    ron = _contract_text_for_metadata(
        "flying_spaghetti_monster_boss",
        fsm.TARGET_NAME,
        [name for name, _, _ in fsm.ROWS],
        fsm.ACTOR_METADATA,
    )
    assert 'character_id: "npc_flying_spaghetti_monster_boss"' in ron
    assert 'body_plan: Some("BossMultipart")' in ron
    assert "fly: Some(true)" in ron
    assert "walk: Some(false)" in ron
    assert '"beam_l"' in ron
    assert '"action.special.eye_beam"' in ron


def test_every_registered_character_target_has_local_actor_metadata():
    # Rig-doc targets (GUI-authored `*.rig.json` under targets/characters/rigged/,
    # e.g. `noether`) are a distinct authoring path that does not yet carry actor
    # metadata — that's deferred game-contract work (see
    # docs/planning/sprite-renderer-refactor.md, "explicitly deferred"). Exempt
    # them so this guard keeps covering the Python/YAML targets it was written for.
    from ambition_sprite2d_renderer.targets.characters import rigged

    rigdoc_targets = set(rigged.TARGETS)
    targets = discover_all_targets().targets
    missing = []
    for name, target in targets.items():
        if target.category != "characters" or name in rigdoc_targets:
            continue
        if type(target).__name__ == "AdapterTarget":
            job = getattr(target, "_job")
            local_metadata = bool(
                job.actor
                or job.body
                or job.capabilities
                or job.brain
                or job.actions
                or job.animation_bindings
                or job.sockets
            )
        else:
            local_metadata = bool(getattr(target, "_actor_metadata", None))
        if not local_metadata:
            missing.append(name)
    assert missing == []


def test_adapter_actor_metadata_is_authored_in_yaml_not_central_profile():
    from ambition_sprite2d_renderer.registry import discover_all_targets

    target = discover_all_targets().targets["robot_runner"]
    job = getattr(target, "_job")
    assert job.actor["character_id"] == "npc_robot_runner"
    assert job.capabilities["traversal"]["jump"]["height_px"] == 48.0
    assert job.brain["default_preset"] == "patrol_peaceful"
