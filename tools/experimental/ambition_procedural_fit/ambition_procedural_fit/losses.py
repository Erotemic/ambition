from __future__ import annotations

from dataclasses import dataclass

import torch
import torch.nn.functional as F


@dataclass
class LossWeights:
    rgb: float = 1.0
    pyramid: float = 0.45
    edge: float = 0.15
    detail: float = 0.0
    color_stats: float = 0.05
    area: float = 0.0


def _chw(x: torch.Tensor) -> torch.Tensor:
    if x.ndim != 3 or x.shape[-1] != 3:
        raise ValueError(f"Expected HWC RGB tensor, got {tuple(x.shape)}")
    return x.permute(2, 0, 1).unsqueeze(0)


def _sobel_gray(x: torch.Tensor) -> torch.Tensor:
    chw = _chw(x)
    gray = chw[:, :1] * 0.299 + chw[:, 1:2] * 0.587 + chw[:, 2:3] * 0.114
    kx = torch.tensor(
        [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]], dtype=x.dtype, device=x.device
    ).view(1, 1, 3, 3)
    ky = torch.tensor(
        [[-1, -2, -1], [0, 0, 0], [1, 2, 1]], dtype=x.dtype, device=x.device
    ).view(1, 1, 3, 3)
    gx = F.conv2d(gray, kx, padding=1)
    gy = F.conv2d(gray, ky, padding=1)
    return torch.sqrt(gx * gx + gy * gy + 1e-6)


def _laplacian_gray(x: torch.Tensor) -> torch.Tensor:
    chw = _chw(x)
    gray = chw[:, :1] * 0.299 + chw[:, 1:2] * 0.587 + chw[:, 2:3] * 0.114
    kernel = torch.tensor(
        [[0, 1, 0], [1, -4, 1], [0, 1, 0]], dtype=x.dtype, device=x.device
    ).view(1, 1, 3, 3)
    return F.conv2d(gray, kernel, padding=1)


def image_fit_loss(
    render: torch.Tensor, target: torch.Tensor, *, weights: LossWeights | None = None
) -> tuple[torch.Tensor, dict[str, float]]:
    """Smooth multi-scale loss for matching a procedural render to target art.

    Geometry still makes the problem non-convex, but each term is smooth under
    the soft rasterizer and behaves much better than hard pixel IoU. For fixed
    primitive geometry and alpha ordering, the RGB color subproblem is close to
    an ordinary least-squares problem.
    """
    weights = weights or LossWeights()
    target = target.to(render.device, dtype=render.dtype)
    terms: dict[str, torch.Tensor] = {}
    terms["rgb"] = F.mse_loss(render, target)

    pyr_loss = render.new_tensor(0.0)
    r = _chw(render)
    t = _chw(target)
    for _level, scale_weight in enumerate([0.5, 0.25, 0.125], start=1):
        if min(r.shape[-2:]) < 8:
            break
        r = F.avg_pool2d(r, kernel_size=2, stride=2)
        t = F.avg_pool2d(t, kernel_size=2, stride=2)
        pyr_loss = pyr_loss + scale_weight * F.mse_loss(r, t)
    terms["pyramid"] = pyr_loss

    terms["edge"] = F.l1_loss(_sobel_gray(render), _sobel_gray(target))
    terms["detail"] = F.l1_loss(_laplacian_gray(render), _laplacian_gray(target))
    r_mu = render.mean(dim=(0, 1))
    t_mu = target.mean(dim=(0, 1))
    r_std = render.std(dim=(0, 1))
    t_std = target.std(dim=(0, 1))
    terms["color_stats"] = F.mse_loss(r_mu, t_mu) + 0.25 * F.mse_loss(r_std, t_std)

    total = (
        weights.rgb * terms["rgb"]
        + weights.pyramid * terms["pyramid"]
        + weights.edge * terms["edge"]
        + weights.detail * terms["detail"]
        + weights.color_stats * terms["color_stats"]
    )
    scalars = {k: float(v.detach().cpu()) for k, v in terms.items()}
    scalars["total"] = float(total.detach().cpu())
    return total, scalars
