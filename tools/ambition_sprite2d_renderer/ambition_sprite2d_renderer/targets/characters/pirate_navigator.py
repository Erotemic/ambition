from __future__ import annotations

from pathlib import Path

from ._pirate_common import render_target

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_pirate_navigator",
        "display_name": "Pirate Navigator",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "traits": [
            "story",
            "humanoid",
            "enemy",
            "combatant",
            "ranged",
            "pirate",
            "navigator",
        ],
        "locomotion_hint": "Walk",
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
            "crawl": None,
            "use_lifts": True,
            "door_access": ["public"],
        },
        "interactions": {
            "talk": True,
            "trade": None,
            "carry": None,
            "open_doors": ["public"],
        },
    },
    "brain": "skirmisher_ranger",
    "actions": {"default_preset": "ranger_arrow"},
    "visual": {"default_pose": "idle"},
    "tags": [
        "story",
        "humanoid",
        "enemy",
        "combatant",
        "ranged",
        "pirate",
        "navigator",
    ],
    "sockets": {
        "head": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 24.0},
        },
        "chest": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 54.0},
        },
        "hand_l": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 48.0, "y": 64.0},
        },
        "hand_r": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "speech_bubble": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 8.0},
        },
        "weapon_grip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "weapon_tip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 104.0, "y": 60.0},
        },
        "muzzle": {
            "source": "explicit.profile.ranged_humanoid",
            "point": {"x": 96.0, "y": 58.0},
        },
        "projectile_origin": {
            "source": "explicit.profile.ranged_humanoid",
            "point": {"x": 96.0, "y": 58.0},
        },
        "compass": {
            "source": "explicit.profile.pirate",
            "point": {"x": 52.0, "y": 58.0},
        },
    },
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
        "action.melee.primary": {
            "animation": "slash",
            "events": [
                {
                    "t": 0.34,
                    "event": "hitbox_active_start",
                    "source": "explicit.profile.combat_humanoid",
                },
                {
                    "t": 0.58,
                    "event": "hitbox_active_end",
                    "source": "explicit.profile.combat_humanoid",
                },
            ],
        },
        "action.ranged.primary": {
            "animation": "slash",
            "events": [
                {
                    "t": 0.5,
                    "event": "projectile_release",
                    "source": "explicit.profile.ranged_humanoid",
                }
            ],
        },
    },
}


TARGET_NAME = "pirate_navigator"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]


def render(out_dir: str | Path, **opts):
    out_dir = Path(out_dir)
    frame_size = opts.get("frame_size")
    outputs = render_target(TARGET_NAME, out_dir, frame_size=frame_size or (128, 128))
    return [outputs["spritesheet"], outputs["yaml"]]
