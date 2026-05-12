from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml


CANONICAL_TRAIN_FIELDS = {
    "rect": {"xy", "wh", "angle", "color"},
    "ellipse": {"xy", "wh", "angle", "color"},
    "superellipse": {"xy", "wh", "angle", "exponent", "color"},
    "segment": {"p0", "p1", "width", "color"},
}


@dataclass
class CanvasSpec:
    width: int = 256
    height: int = 256
    background: list[float] = field(default_factory=lambda: [0.04, 0.045, 0.055, 1.0])


@dataclass
class PrimitiveSpec:
    kind: str
    name: str
    params: dict[str, Any]
    train: list[str] = field(default_factory=list)


@dataclass
class FitSpec:
    canvas: CanvasSpec
    primitives: list[PrimitiveSpec]
    loss: dict[str, Any] = field(default_factory=dict)
    metadata: dict[str, Any] = field(default_factory=dict)


def _as_float_list(value: Any, expected: int | None = None) -> list[float]:
    vals = [float(v) for v in value]
    if expected is not None and len(vals) != expected:
        raise ValueError(f"Expected {expected} values, got {len(vals)}: {value!r}")
    return vals


def load_spec(path: str | Path) -> FitSpec:
    path = Path(path)
    data = yaml.safe_load(path.read_text())
    if not isinstance(data, dict):
        raise ValueError(f"Template must be a YAML mapping: {path}")
    return spec_from_mapping(data)


def spec_from_mapping(data: dict[str, Any]) -> FitSpec:
    canvas_map = data.get("canvas") or {}
    canvas = CanvasSpec(
        width=int(canvas_map.get("width", 256)),
        height=int(canvas_map.get("height", 256)),
        background=_as_float_list(canvas_map.get("background", [0.04, 0.045, 0.055, 1.0]), 4),
    )
    primitives = []
    for idx, item in enumerate(data.get("primitives") or []):
        if not isinstance(item, dict):
            raise ValueError(f"Primitive #{idx} is not a mapping")
        kind = str(item.get("kind", "rect"))
        if kind not in CANONICAL_TRAIN_FIELDS:
            raise ValueError(f"Unsupported primitive kind: {kind!r}")
        name = str(item.get("name") or f"{kind}_{idx:03d}")
        train = item.get("train", [])
        if train == "all":
            train = sorted(CANONICAL_TRAIN_FIELDS[kind])
        train = [str(t) for t in train]
        unknown_train = set(train) - CANONICAL_TRAIN_FIELDS[kind]
        if unknown_train:
            raise ValueError(f"Unsupported train fields for {name}: {sorted(unknown_train)}")
        params = {k: v for k, v in item.items() if k not in {"kind", "name", "train"}}
        primitives.append(PrimitiveSpec(kind=kind, name=name, params=params, train=train))
    return FitSpec(
        canvas=canvas,
        primitives=primitives,
        loss=dict(data.get("loss") or {}),
        metadata=dict(data.get("metadata") or {}),
    )


def spec_to_mapping(spec: FitSpec) -> dict[str, Any]:
    return {
        "metadata": spec.metadata,
        "canvas": {
            "width": int(spec.canvas.width),
            "height": int(spec.canvas.height),
            "background": [float(v) for v in spec.canvas.background],
        },
        "loss": spec.loss,
        "primitives": [
            {
                "kind": p.kind,
                "name": p.name,
                **_serializable_params(p.params),
                "train": list(p.train),
            }
            for p in spec.primitives
        ],
    }


def _serializable_params(params: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in params.items():
        if isinstance(value, tuple):
            value = list(value)
        if isinstance(value, list):
            out[key] = [float(v) if isinstance(v, (int, float)) else v for v in value]
        elif isinstance(value, (int, float)):
            out[key] = float(value)
        else:
            out[key] = value
    return out


def write_spec(spec: FitSpec, path: str | Path) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    data = spec_to_mapping(spec)
    text = yaml.safe_dump(data, sort_keys=False, width=96)
    path.write_text(text)
