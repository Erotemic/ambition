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


def default_profile() -> BackgroundProfile:
    # 512x512 tileable-ish textures keep runtime memory low while the Bevy
    # side repeats them over a large sprite. The sky is stretched instead of
    # tiled, but it uses the same size for consistent tooling.
    return BackgroundProfile(
        name="default",
        layers=(
            LayerSpec("sky", 512, 512, 1001),
            LayerSpec("far", 512, 512, 2002),
            LayerSpec("mid", 512, 512, 3003),
            LayerSpec("near", 512, 512, 4004),
        ),
    )


def profiles() -> dict[str, BackgroundProfile]:
    default = default_profile()
    return {default.name: default}


def iter_profiles(selected: str | None = None) -> Iterable[BackgroundProfile]:
    all_profiles = profiles()
    if selected in (None, "all"):
        return all_profiles.values()
    try:
        return (all_profiles[selected],)
    except KeyError as ex:
        known = ", ".join(sorted(all_profiles))
        raise KeyError(f"unknown background profile {selected!r}; known profiles: {known}") from ex
