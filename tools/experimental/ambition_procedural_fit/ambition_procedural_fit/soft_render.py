from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

import torch
import torch.nn.functional as F

from .schema import FitSpec, PrimitiveSpec

EPS = 1e-6


def _clamp01(value: float) -> float:
    return float(max(0.001, min(0.999, value)))


def _logit(value: float) -> float:
    value = _clamp01(value)
    return float(torch.logit(torch.tensor(value)).item())


def _tensor(
    values, *, device: torch.device, dtype: torch.dtype = torch.float32
) -> torch.Tensor:
    return torch.tensor(values, device=device, dtype=dtype)


def _sigmoid_param(raw: torch.Tensor, lo: float, hi: float) -> torch.Tensor:
    return lo + (hi - lo) * torch.sigmoid(raw)


def _inverse_sigmoid_param(value: float, lo: float, hi: float) -> float:
    value = float(max(lo + 1e-6, min(hi - 1e-6, value)))
    scaled = (value - lo) / (hi - lo)
    return _logit(scaled)


@dataclass
class RenderOptions:
    width: int
    height: int
    sharpness: float = 90.0
    supersample: int = 1


class ProceduralScene(torch.nn.Module):
    """Differentiable soft rasterizer for a small procedural primitive scene.

    All coordinates are normalized to [0, 1] so the same template can be fit at
    low resolution and rendered at a larger export resolution later.
    """

    def __init__(self, spec: FitSpec, *, device: str | torch.device = "cpu") -> None:
        super().__init__()
        self.spec = spec
        self.device_obj = torch.device(device)
        self.static: dict[str, torch.Tensor | float | list[float] | str] = {}
        self._param_meta: list[tuple[int, str, str]] = []
        self._init_from_spec(spec)

    def _register_raw(
        self, prim_index: int, field_name: str, suffix: str, values: Iterable[float]
    ) -> None:
        name = f"p{prim_index:03d}_{field_name}_{suffix}"
        self.register_parameter(
            name, torch.nn.Parameter(_tensor(list(values), device=self.device_obj))
        )
        self._param_meta.append((prim_index, field_name, name))

    def _init_from_spec(self, spec: FitSpec) -> None:
        for idx, prim in enumerate(spec.primitives):
            p = prim.params
            train = set(prim.train)
            prefix = f"p{idx:03d}"
            self.static[f"{prefix}.kind"] = prim.kind
            if prim.kind in {"rect", "ellipse", "superellipse"}:
                xy = p.get("xy", [0.5, 0.5])
                wh = p.get("wh", [0.2, 0.2])
                angle = float(p.get("angle", 0.0))
                color = p.get("color", [1.0, 1.0, 1.0, 0.8])
                default_exp = (
                    10.0
                    if prim.kind == "rect"
                    else 2.0
                    if prim.kind == "ellipse"
                    else float(p.get("exponent", 4.0))
                )
                exponent = float(p.get("exponent", default_exp))
                if "xy" in train:
                    self._register_raw(
                        idx, "xy", "raw", [_logit(float(xy[0])), _logit(float(xy[1]))]
                    )
                else:
                    self.static[f"{prefix}.xy"] = _tensor(xy, device=self.device_obj)
                if "wh" in train:
                    self._register_raw(
                        idx, "wh", "raw", [_logit(float(wh[0])), _logit(float(wh[1]))]
                    )
                else:
                    self.static[f"{prefix}.wh"] = _tensor(wh, device=self.device_obj)
                if "angle" in train:
                    self._register_raw(idx, "angle", "raw", [float(angle)])
                else:
                    self.static[f"{prefix}.angle"] = float(angle)
                if "exponent" in train and prim.kind == "superellipse":
                    self._register_raw(
                        idx,
                        "exponent",
                        "raw",
                        [_inverse_sigmoid_param(exponent, 2.0, 24.0)],
                    )
                elif prim.kind == "superellipse":
                    self.static[f"{prefix}.exponent"] = float(exponent)
                if "color" in train:
                    rgba = [float(v) for v in color]
                    self._register_raw(idx, "color", "raw", [_logit(v) for v in rgba])
                else:
                    self.static[f"{prefix}.color"] = _tensor(
                        color, device=self.device_obj
                    )
            elif prim.kind == "segment":
                p0 = p.get("p0", [0.25, 0.25])
                p1 = p.get("p1", [0.75, 0.75])
                width = float(p.get("width", 0.02))
                color = p.get("color", [1.0, 1.0, 1.0, 0.8])
                if "p0" in train:
                    self._register_raw(
                        idx, "p0", "raw", [_logit(float(p0[0])), _logit(float(p0[1]))]
                    )
                else:
                    self.static[f"{prefix}.p0"] = _tensor(p0, device=self.device_obj)
                if "p1" in train:
                    self._register_raw(
                        idx, "p1", "raw", [_logit(float(p1[0])), _logit(float(p1[1]))]
                    )
                else:
                    self.static[f"{prefix}.p1"] = _tensor(p1, device=self.device_obj)
                if "width" in train:
                    self._register_raw(
                        idx, "width", "raw", [_logit(float(width) / 0.18)]
                    )
                else:
                    self.static[f"{prefix}.width"] = float(width)
                if "color" in train:
                    rgba = [float(v) for v in color]
                    self._register_raw(idx, "color", "raw", [_logit(v) for v in rgba])
                else:
                    self.static[f"{prefix}.color"] = _tensor(
                        color, device=self.device_obj
                    )

    def _raw(self, idx: int, field_name: str) -> torch.Tensor | None:
        needle = (idx, field_name)
        for prim_idx, field, param_name in self._param_meta:
            if (prim_idx, field) == needle:
                return getattr(self, param_name)
        return None

    def _field(self, idx: int, field_name: str) -> torch.Tensor | float:
        raw = self._raw(idx, field_name)
        prefix = f"p{idx:03d}"
        if raw is None:
            value = self.static[f"{prefix}.{field_name}"]
            return value  # type: ignore[return-value]
        if field_name in {"xy", "p0", "p1"}:
            return torch.sigmoid(raw)
        if field_name == "wh":
            return _sigmoid_param(raw, 0.015, 0.98)
        if field_name == "angle":
            return torch.pi * torch.tanh(raw)[0]
        if field_name == "color":
            return torch.sigmoid(raw)
        if field_name == "width":
            return _sigmoid_param(raw, 0.002, 0.18)[0]
        if field_name == "exponent":
            return _sigmoid_param(raw, 2.0, 24.0)[0]
        raise KeyError(field_name)

    def to_spec(self) -> FitSpec:
        """Return a copy of the original spec with current parameter values."""
        new_prims: list[PrimitiveSpec] = []
        for idx, old in enumerate(self.spec.primitives):
            params = dict(old.params)
            for field in sorted(set(old.train)):
                val = self._field(idx, field)
                if isinstance(val, torch.Tensor):
                    arr = val.detach().cpu().flatten().tolist()
                    if len(arr) == 1:
                        params[field] = float(arr[0])
                    else:
                        params[field] = [float(v) for v in arr]
                else:
                    params[field] = float(val)
            new_prims.append(PrimitiveSpec(old.kind, old.name, params, list(old.train)))
        metadata = dict(self.spec.metadata)
        metadata["optimized_by"] = "ambition_procedural_fit 0.1.0"
        return FitSpec(
            canvas=self.spec.canvas,
            primitives=new_prims,
            loss=dict(self.spec.loss),
            metadata=metadata,
        )

    def forward(self, opts: RenderOptions) -> torch.Tensor:
        scale = max(1, int(opts.supersample))
        height = int(opts.height) * scale
        width = int(opts.width) * scale
        yy, xx = torch.meshgrid(
            torch.linspace(0.0, 1.0, height, device=self.device_obj),
            torch.linspace(0.0, 1.0, width, device=self.device_obj),
            indexing="ij",
        )
        rgba_bg = _tensor(self.spec.canvas.background, device=self.device_obj)
        img = rgba_bg[:3].view(1, 1, 3).expand(height, width, 3).clone()
        sharpness = float(opts.sharpness) * scale
        for idx, prim in enumerate(self.spec.primitives):
            color = self._field(idx, "color")
            if not isinstance(color, torch.Tensor):
                color = _tensor(color, device=self.device_obj)  # type: ignore[arg-type]
            alpha_shape = self._primitive_alpha(idx, prim.kind, xx, yy, sharpness)
            alpha = (alpha_shape * color[3]).clamp(0.0, 1.0).unsqueeze(-1)
            img = img * (1.0 - alpha) + color[:3].view(1, 1, 3) * alpha
        if scale > 1:
            img_chw = img.permute(2, 0, 1).unsqueeze(0)
            img = (
                F.avg_pool2d(img_chw, kernel_size=scale, stride=scale)
                .squeeze(0)
                .permute(1, 2, 0)
            )
        return img.clamp(0.0, 1.0)

    def _oriented_coords(
        self, idx: int, xx: torch.Tensor, yy: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        center = self._field(idx, "xy")
        wh = self._field(idx, "wh")
        angle = self._field(idx, "angle")
        assert isinstance(center, torch.Tensor)
        assert isinstance(wh, torch.Tensor)
        if not isinstance(angle, torch.Tensor):
            angle = _tensor([angle], device=self.device_obj)[0]
        x = xx - center[0]
        y = yy - center[1]
        ca = torch.cos(angle)
        sa = torch.sin(angle)
        xr = ca * x + sa * y
        yr = -sa * x + ca * y
        return xr, yr, wh

    def _primitive_alpha(
        self, idx: int, kind: str, xx: torch.Tensor, yy: torch.Tensor, sharpness: float
    ) -> torch.Tensor:
        if kind in {"rect", "ellipse", "superellipse"}:
            xr, yr, wh = self._oriented_coords(idx, xx, yy)
            if kind == "rect":
                dx = torch.abs(xr) - wh[0] * 0.5
                dy = torch.abs(yr) - wh[1] * 0.5
                outside = torch.maximum(dx, dy)
                return torch.sigmoid(-outside * sharpness)
            rx = torch.clamp(wh[0] * 0.5, min=EPS)
            ry = torch.clamp(wh[1] * 0.5, min=EPS)
            if kind == "ellipse":
                dist = torch.sqrt((xr / rx) ** 2 + (yr / ry) ** 2 + EPS) - 1.0
                return torch.sigmoid(-dist * sharpness / 8.0)
            exponent = self._field(idx, "exponent")
            if not isinstance(exponent, torch.Tensor):
                exponent = _tensor([exponent], device=self.device_obj)[0]
            term = (torch.abs(xr / rx) + EPS).pow(exponent) + (
                torch.abs(yr / ry) + EPS
            ).pow(exponent)
            dist = term.pow(1.0 / exponent) - 1.0
            return torch.sigmoid(-dist * sharpness / 7.0)
        if kind == "segment":
            p0 = self._field(idx, "p0")
            p1 = self._field(idx, "p1")
            width = self._field(idx, "width")
            assert isinstance(p0, torch.Tensor)
            assert isinstance(p1, torch.Tensor)
            if not isinstance(width, torch.Tensor):
                width = _tensor([width], device=self.device_obj)[0]
            vx = p1[0] - p0[0]
            vy = p1[1] - p0[1]
            wx = xx - p0[0]
            wy = yy - p0[1]
            c1 = vx * wx + vy * wy
            c2 = vx * vx + vy * vy + EPS
            t = (c1 / c2).clamp(0.0, 1.0)
            px = p0[0] + t * vx
            py = p0[1] + t * vy
            dist = torch.sqrt((xx - px) ** 2 + (yy - py) ** 2 + EPS) - width * 0.5
            return torch.sigmoid(-dist * sharpness)
        raise KeyError(kind)


def render_spec(
    spec: FitSpec,
    width: int | None = None,
    height: int | None = None,
    *,
    device: str = "cpu",
    supersample: int = 2,
) -> torch.Tensor:
    scene = ProceduralScene(spec, device=device)
    return scene(
        RenderOptions(
            width=width or spec.canvas.width,
            height=height or spec.canvas.height,
            supersample=supersample,
        )
    )
