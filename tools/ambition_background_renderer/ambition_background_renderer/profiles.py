from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


@dataclass(frozen=True)
class LayerSpec:
    name: str
    width: int
    height: int
    seed: int


@dataclass(frozen=True)
class BackgroundProfile:
    name: str
    layers: tuple[LayerSpec, ...]


DEFAULT_LAYER_NAMES = ("sky", "far", "mid", "near")


def _layers(seed_base: int) -> tuple[LayerSpec, ...]:
    # 512x512 tileable-ish textures keep runtime memory low while the Bevy
    # side repeats them over a large sprite. The sky is stretched instead of
    # tiled, but it uses the same size for consistent tooling.
    return tuple(
        LayerSpec(name, 512, 512, seed_base + index * 1001)
        for index, name in enumerate(DEFAULT_LAYER_NAMES)
    )


PROFILE_SEEDS: tuple[tuple[str, int], ...] = (
    ("default", 1000),
    ("hub", 2000),
    ("lab", 3000),
    ("basement", 4000),
    ("cove", 5000),
    ("skybridge", 6000),
    ("boss", 7000),
    ("water", 8000),
    ("cave", 9000),
)


def profiles() -> dict[str, BackgroundProfile]:
    return {
        name: BackgroundProfile(name=name, layers=_layers(seed_base))
        for name, seed_base in PROFILE_SEEDS
    }


def iter_profiles(selected: str | None = None) -> Iterable[BackgroundProfile]:
    all_profiles = profiles()
    if selected in (None, "all"):
        return all_profiles.values()
    try:
        return (all_profiles[selected],)
    except KeyError as ex:
        known = ", ".join(sorted(all_profiles))
        raise KeyError(
            f"unknown background profile {selected!r}; known profiles: {known}"
        ) from ex
