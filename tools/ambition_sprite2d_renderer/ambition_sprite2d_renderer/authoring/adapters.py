from __future__ import annotations

from dataclasses import asdict, fields, is_dataclass, replace
from typing import Any, Dict, List, Tuple

from PIL import Image

from .animation_vocab import FULL_PLAYER_ANIMATION_ORDER, ordered_subset
from ..registry import CharacterJob
from ..targets.characters.boss_side import (
    AISlopZetaGenerator,
    parse_background as boss_parse_background,
)
from ..targets.characters.goblin_side import (
    SideGoblinGenerator,
    parse_background as goblin_parse_background,
)
from ..targets.characters.ninja_side import (
    NinjaSideGenerator,
    parse_background as ninja_parse_background,
)
from ..targets.characters.robot_side import SideRobotGenerator
from ..targets.characters.sandbag import (
    ADAPTER_ANIMATIONS as SANDBAG_ANIMATIONS,
    SandbagSpec,
    render_frame as render_sandbag_frame,
)
from ..targets.characters.robot25d import parse_background as robot_parse_background
from ..targets.characters.toon_side import (
    ToonSideGenerator,
    parse_background as toon_parse_background,
)
from ..targets.characters.trent_elder import (
    TrentElderGenerator,
    parse_background as trent_parse_background,
)
from ..targets.characters.bob_engineer import (
    BobEngineerGenerator,
    parse_background as bob_parse_background,
)
from ..targets.characters.alice_cryptographer import (
    AliceCryptographerGenerator,
    parse_background as alice_parse_background,
)


def _dataclass_dict(obj: Any) -> Dict[str, Any]:
    data = asdict(obj) if is_dataclass(obj) else dict(obj)
    return _strip_callables(data)


def _strip_callables(value: Any) -> Any:
    # Spec dataclasses may carry hook callables (e.g. ToonSpec.pose_override).
    # asdict passes them through unchanged, and yaml/RON can't represent
    # functions — drop them so the manifest stays serialisable.
    if isinstance(value, dict):
        return {k: _strip_callables(v) for k, v in value.items() if not callable(v)}
    if isinstance(value, list):
        return [_strip_callables(v) for v in value if not callable(v)]
    if isinstance(value, tuple):
        return tuple(_strip_callables(v) for v in value if not callable(v))
    return value


class BaseAdapter:
    target: str

    def animations(self) -> Dict[str, Dict[str, int]]:
        raise NotImplementedError

    def default_animations(self) -> List[str]:
        return list(self.animations().keys())

    def canonical_pose(self) -> Tuple[str, int]:
        return ("idle", 1)

    def sample_spec(self, job: CharacterJob) -> Any:
        raise NotImplementedError

    def spec_dict(self, spec: Any) -> Dict[str, Any]:
        return _dataclass_dict(spec)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        raise NotImplementedError

    def attack_hitboxes(self, size: Tuple[int, int]) -> Dict[str, Dict[str, Any]]:
        """Per-animation attack-hitbox metadata.

        Returns a dict mapping animation name to a hitbox descriptor:

            {
                "floor_slam": {
                    "bbox": (x, y, w, h),
                },
                "side_sweep": {
                    "parts": [
                        {"name": "left",  "x": ..., "y": ..., "w": ..., "h": ...},
                        {"name": "right", "x": ..., "y": ..., "w": ..., "h": ...},
                    ],
                },
            }

        Coordinates are in **source canvas** pixel space (before
        the renderer's auto-crop). The renderer translates to
        cropped-frame coordinates before emitting.

        Override per adapter to author attack-hitbox shapes that
        match the sprite's pose during the strike beat. Default
        empty (no attack hitboxes; gameplay falls back to its
        hardcoded volume math).

        ``size`` is the source render canvas the adapter is using
        (e.g. ``(128, 128)``) so coordinates can be specified in
        absolute pixels rather than normalized. Adapters that
        prefer normalized coords can compute against ``size`` and
        return absolute pixels.
        """
        del size
        return {}

    def hurtbox_parts(self, size: Tuple[int, int]) -> Dict[str, Dict[str, Any]]:
        """Per-animation hurtbox-parts override.

        Returns a dict mapping animation name → ``{"parts": [...]}``.
        When declared, the listed parts REPLACE the renderer's
        auto-derived alpha-bbox hurtbox for that animation, letting
        the adapter carve a multi-rect hurtbox that excludes
        cosmetic / unsafe extensions (e.g. extended arms during
        an attack). The parts shape mirrors ``attack_hitboxes``:

            {
                "rest": {
                    "parts": [
                        {"name": "head", "x": ..., "y": ..., "w": ..., "h": ...},
                        {"name": "body", "x": ..., "y": ..., "w": ..., "h": ...},
                    ],
                },
                ...
            }

        Coordinates are in source canvas pixel space; the renderer
        translates to cropped-frame coordinates and clips against
        the frame.

        Default empty (no override → use auto-derived alpha bbox).
        """
        del size
        return {}

    def render_single(
        self, spec: Any, animation: str, frame_index: int, job: CharacterJob
    ) -> Image.Image:
        r = job.render
        return self.render_frame(
            spec, animation, frame_index, (r.single_width, r.single_height), job
        )

    def render_canonical(self, spec: Any, job: CharacterJob) -> Image.Image:
        animation, frame_index = self.canonical_pose()
        return self.render_single(spec, animation, frame_index, job)

    def _apply_overrides(self, spec: Any, job: CharacterJob) -> Any:
        overrides = dict(getattr(job, "spec_overrides", {}) or {})
        if not overrides:
            return spec
        if not is_dataclass(spec):
            raise TypeError(
                f"spec overrides are only supported for dataclass specs (target={job.target})"
            )
        known = {f.name for f in fields(spec)}
        unknown = sorted(set(overrides) - known)
        if unknown:
            raise KeyError(
                f"unknown spec override keys for {job.target}: {unknown}; available={sorted(known)}"
            )
        return replace(spec, **overrides)


class GoblinAdapter(BaseAdapter):
    target = "goblin"

    def __init__(self) -> None:
        self.generator = SideGoblinGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.SPRITESHEET_ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype, job.held_item)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=goblin_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class BossAdapter(BaseAdapter):
    target = "boss"

    def __init__(self) -> None:
        self.generator = AISlopZetaGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def canonical_pose(self) -> Tuple[str, int]:
        return ("rest", 1)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=boss_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )

    def attack_hitboxes(self, size: Tuple[int, int]) -> Dict[str, Dict[str, Any]]:
        """Per-attack hitbox shapes for the Gradient Sentinel boss.
        Coordinates are in source canvas pixels (128×128 by default).

        Animation → attack mapping (gameplay-side
        ``BossAttackProfile`` → animation name):

        - ``floor_slam``  → FloorSlam (broad slap below the body)
        - ``side_sweep``  → SideSweep (two arm sweeps left + right)
        - ``spike_halo``  → MemorizedVolley / RotatingCross anchor
                            (ring around the body)
        - ``dash_echo``   → HazardColumn (a tall horizontal lane
                            following the dash)

        Tuned to the AI Slop Zeta's 128×128 frame; the renderer
        translates these to cropped-frame coordinates and the
        gameplay code rescales them by the spawn AABB.
        """
        canvas_w, canvas_h = size
        # The boss sprite sits roughly centered in the 128×128
        # canvas. Body alpha-bbox is (8, 5, 106, 83); arms reach
        # ±35 px out from the body center during attacks.
        return {
            # FloorSlam: ground-level slap centered below the body.
            # Tightened width from 120 → 96 so the slam only damages
            # players standing directly under / near the boss; the
            # outer 16 px on each side were "safe lateral space"
            # nobody could read as damaging.
            "floor_slam": {
                "bbox": (16, 90, canvas_w - 32, 28),
            },
            # SideSweep: two arm hitboxes — one to each side of
            # the body. Tightened to match the visible arm reach:
            # arms swing roughly y=46..82 (mid-body height, not
            # full-body), and the inner edge of the swing reads as
            # solidly part of the arm at x≈28 / x≈100.
            "side_sweep": {
                "parts": [
                    {"name": "left", "x": 0, "y": 46, "w": 30, "h": 38},
                    {"name": "right", "x": canvas_w - 30, "y": 46, "w": 30, "h": 38},
                ],
            },
            # SpikeHalo: a ring around the boss. Approximated by
            # four quadrant boxes inset from each edge so the
            # absolute corners (which the spike sprites don't
            # actually reach) aren't damaging. The four parts
            # together cover the spike radius without leaving a
            # safe inner zone (each box bumps up against the body
            # center).
            "spike_halo": {
                "parts": [
                    {"name": "top", "x": 8, "y": 0, "w": canvas_w - 16, "h": 36},
                    {
                        "name": "bottom",
                        "x": 8,
                        "y": canvas_h - 36,
                        "w": canvas_w - 16,
                        "h": 36,
                    },
                    {"name": "left", "x": 0, "y": 24, "w": 36, "h": canvas_h - 48},
                    {
                        "name": "right",
                        "x": canvas_w - 36,
                        "y": 24,
                        "w": 36,
                        "h": canvas_h - 48,
                    },
                ],
            },
            # DashEcho: an elongated horizontal lane tracking the
            # dash. Tightened vertically (50→56 start, 40→28 tall)
            # so the player can jump over the dash with reasonable
            # timing instead of needing pixel-perfect height.
            "dash_echo": {
                "bbox": (0, 56, canvas_w, 28),
            },
        }

    def hurtbox_parts(self, size: Tuple[int, int]) -> Dict[str, Dict[str, Any]]:
        """Per-animation hurtbox parts for the Gradient Sentinel boss.

        Splits the auto-derived alpha-bbox hurtbox into two parts —
        head + body — so the player's attacks register on the
        central head/torso area but NOT on the arms (which extend
        far out during ``side_sweep`` and ``floor_slam``). This
        forces the player to position close to the body to score
        hits, rather than tagging an extended arm from across the
        arena.

        Coordinates are in source canvas pixels (128×128). The
        head + body parts overlap by 1-2 pixels in y so there's no
        gap between them.
        """
        del size
        # Body alpha-bbox at rest: (8, 5, 106, 83). The visible
        # body occupies y=5..88 in canvas coords. Split into:
        # - head: y≈5..30 (height ~25 px), narrow (skull/face only)
        # - body: y≈28..86 (height ~58 px), narrow torso (no arms)
        head = {"name": "head", "x": 46, "y": 5, "w": 36, "h": 25}
        body = {"name": "body", "x": 42, "y": 28, "w": 44, "h": 58}
        # All combat animations share the same head + body parts —
        # the boss doesn't move its head/torso between poses, only
        # its arms (which we deliberately exclude). `hit` reuses
        # the rest pair so the player can keep attacking the
        # stunned boss; `death` skips parts (boss is dying, not
        # damageable).
        per_anim_parts = [head, body]
        return {
            anim: {"parts": [dict(p) for p in per_anim_parts]}
            for anim in (
                "rest",
                "floor_slam",
                "side_sweep",
                "spike_halo",
                "dash_echo",
                "hit",
            )
        }


class RobotAdapter(BaseAdapter):
    target = "robot"

    def __init__(self) -> None:
        self.generator = SideRobotGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=robot_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )

    def attack_hitboxes(self, size: Tuple[int, int]) -> Dict[str, Dict[str, Any]]:
        """Per-attack hitboxes for the player's 2-frame swings.

        Hollow-Knight-style: each box is thrust out IN the swing's
        direction, disjoint from the body, and live on frame 0 only
        (`active_frames=[0]`) — the 2-frame swings hit immediately, frame 1
        is wind-down. Boxes are authored for the right-facing robot; the
        runtime mirrors x by facing. Attack hitboxes are NOT frame-clamped
        (see sheet.py), so they reach past the sprite edge. Coords are
        source-canvas pixels — eyeball with ``debug-hitboxes player_robot``
        and nudge to taste (these are a first pass, feel-tunable).
        """
        w, h = size
        cx = w // 2

        def box(x: float, y: float, ww: float, hh: float) -> Dict[str, Any]:
            return {"bbox": (int(x), int(y), int(ww), int(hh)), "active_frames": [0]}

        return {
            # Forward forehand (the primary side attack).
            "attack_side": box(cx + w * 0.26, h * 0.12, w * 0.60, h * 0.72),
            # Overhead (up-tilt / aerial up): above the body.
            "attack_up": box(cx - w * 0.12, -h * 0.08, w * 0.58, h * 0.62),
            "air_up": box(cx - w * 0.22, -h * 0.10, w * 0.55, h * 0.62),
            # Down: ground down-poke is forward-low; aerial down spikes below.
            "attack_down": box(cx + w * 0.16, h * 0.50, w * 0.58, h * 0.46),
            "air_down": box(cx - w * 0.28, h * 0.52, w * 0.62, h * 0.58),
            # Aerial forward / back (back reaches behind = left of center).
            "air_forward": box(cx + w * 0.22, h * 0.22, w * 0.60, h * 0.58),
            "air_back": box(cx - w * 0.72, h * 0.22, w * 0.62, h * 0.58),
            # Aerial neutral: a wide spin around the whole body.
            "air_neutral": box(cx - w * 0.42, h * 0.18, w * 0.92, h * 0.68),
        }


class NinjaAdapter(BaseAdapter):
    target = "ninja"

    def __init__(self) -> None:
        self.generator = NinjaSideGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def spec_dict(self, spec: Any) -> Dict[str, Any]:
        return spec.to_dict()

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=ninja_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class SandbagAdapter(BaseAdapter):
    target = "sandbag"

    def animations(self) -> Dict[str, Dict[str, int]]:
        return ordered_subset(SANDBAG_ANIMATIONS, FULL_PLAYER_ANIMATION_ORDER)

    def canonical_pose(self) -> Tuple[str, int]:
        return ("idle", 1)

    def sample_spec(self, job: CharacterJob) -> SandbagSpec:
        spec = SandbagSpec(
            seed=job.seed,
            archetype=job.archetype,
            variant=str(job.variant or "classic"),
        )
        return self._apply_overrides(spec, job)

    def spec_dict(self, spec: SandbagSpec) -> Dict[str, Any]:
        return spec.to_dict()

    def render_frame(
        self,
        spec: SandbagSpec,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        frame = render_sandbag_frame(
            animation, frame_index % anim["frames"], anim["frames"]
        )
        if frame.size != size:
            frame = frame.resize(size, Image.Resampling.LANCZOS)
        return frame


class ToonAdapter(BaseAdapter):
    target = "toon"

    def __init__(self) -> None:
        self.generator = ToonSideGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=toon_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class TrentElderAdapter(BaseAdapter):
    """Bespoke target for Trent. Single-archetype; see
    `targets/trent_elder.py` for the design rationale."""

    target = "trent_elder"

    def __init__(self) -> None:
        self.generator = TrentElderGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=trent_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class BobEngineerAdapter(BaseAdapter):
    """Bespoke multi-view target for Bob. Single-archetype; see
    `targets/bob_engineer.py` for the design rationale, including
    the per-animation view (3/4 / side / front) routing."""

    target = "bob_engineer"

    def __init__(self) -> None:
        self.generator = BobEngineerGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=bob_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class AliceCryptographerAdapter(BaseAdapter):
    """Bespoke target for Alice. Single-archetype with 3/4 + side
    views (no front view today). See `targets/alice_cryptographer.py`
    for the scaffold rationale + comparison to bob_engineer."""

    target = "alice_cryptographer"

    def __init__(self) -> None:
        self.generator = AliceCryptographerGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(
        self,
        spec: Any,
        animation: str,
        frame_index: int,
        size: Tuple[int, int],
        job: CharacterJob,
    ) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=alice_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


TARGETS: Dict[str, BaseAdapter] = {
    "boss": BossAdapter(),
    "goblin": GoblinAdapter(),
    "ninja": NinjaAdapter(),
    "robot": RobotAdapter(),
    "sandbag": SandbagAdapter(),
    "toon": ToonAdapter(),
    "trent_elder": TrentElderAdapter(),
    "bob_engineer": BobEngineerAdapter(),
    "alice_cryptographer": AliceCryptographerAdapter(),
}


def get_adapter(target: str) -> BaseAdapter:
    try:
        return TARGETS[target]
    except KeyError as ex:
        raise KeyError(
            f"unknown target {target!r}; available={sorted(TARGETS)}"
        ) from ex
