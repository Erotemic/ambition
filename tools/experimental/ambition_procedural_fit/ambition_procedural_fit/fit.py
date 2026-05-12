from __future__ import annotations

import csv
import json
import math
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import torch

from .imageio import load_target_tensor, save_tensor_image, write_comparison, write_debug_gif, write_loss_curve
from .losses import LossWeights, image_fit_loss
from .schema import load_spec, write_spec
from .soft_render import ProceduralScene, RenderOptions


@dataclass
class FitResult:
    out_dir: Path
    best_loss: float
    metrics_path: Path
    template_path: Path
    render_path: Path
    comparison_path: Path


PROFILE_WEIGHTS: dict[str, dict[str, float]] = {
    "generic": {"rgb": 1.0, "pyramid": 0.45, "edge": 0.15, "detail": 0.0, "color_stats": 0.05},
    "background": {"rgb": 1.0, "pyramid": 0.60, "edge": 0.12, "detail": 0.04, "color_stats": 0.08},
    "sprite": {"rgb": 1.0, "pyramid": 0.28, "edge": 0.48, "detail": 0.30, "color_stats": 0.02},
}

PROFILE_SHARPNESS: dict[str, tuple[float, float]] = {
    "generic": (90.0, 90.0),
    "background": (60.0, 110.0),
    "sprite": (36.0, 220.0),
}


def _configure_torch_threads(threads: int | None) -> None:
    if threads and threads > 0:
        torch.set_num_threads(int(threads))
        try:
            torch.set_num_interop_threads(max(1, min(4, int(threads))))
        except RuntimeError:
            # Interop threads may already be initialized in embedded sessions.
            pass


def _choose_device(requested: str) -> str:
    if requested == "auto":
        return "cuda" if torch.cuda.is_available() else "cpu"
    return requested


def _jitter_trainable(scene: ProceduralScene, amount: float, seed: int) -> None:
    if amount <= 0:
        return
    gen = torch.Generator(device="cpu")
    gen.manual_seed(seed)
    with torch.no_grad():
        for name, param in scene.named_parameters():
            noise = torch.randn(param.detach().cpu().shape, generator=gen, dtype=param.dtype).to(param.device)
            if "color" in name:
                scale = amount * 0.35
            elif "angle" in name:
                scale = amount * 0.45
            else:
                scale = amount
            param.add_(noise * scale)


def _resolve_weights(mode: str, spec_loss: dict[str, Any]) -> LossWeights:
    weights_dict = dict(PROFILE_WEIGHTS.get(mode, PROFILE_WEIGHTS["generic"]))
    weights_dict.update({k: float(v) for k, v in (spec_loss.get("weights") or {}).items() if k in LossWeights.__annotations__})
    return LossWeights(**weights_dict)


def _resolve_sharpness(mode: str, sharpness: float, sharpness_start: float | None, sharpness_end: float | None) -> tuple[float, float]:
    default_start, default_end = PROFILE_SHARPNESS.get(mode, PROFILE_SHARPNESS["generic"])
    start = float(sharpness_start if sharpness_start is not None else default_start)
    end = float(sharpness_end if sharpness_end is not None else max(sharpness, default_end))
    if sharpness_start is None and sharpness_end is None and mode == "generic":
        start = float(sharpness)
        end = float(sharpness)
    return start, end


def _annealed_value(start: float, end: float, step: int, steps: int) -> float:
    if steps <= 1:
        return float(end)
    t = step / float(steps - 1)
    t = 0.5 - 0.5 * math.cos(math.pi * t)
    return float(start + (end - start) * t)


def _should_capture_debug(step: int, steps: int, every: int, max_frames: int) -> bool:
    if step == 0 or step + 1 == steps:
        return True
    if every <= 0:
        return False
    if (step + 1) % every != 0:
        return False
    if max_frames <= 0:
        return True
    estimate = 2 + max(0, steps // every)
    stride = max(1, math.ceil(estimate / max_frames))
    marker = (step + 1) // every
    return marker % stride == 0


def fit_template(
    template_path: str | Path,
    target_path: str | Path,
    out_dir: str | Path,
    *,
    steps: int = 300,
    lr: float = 0.05,
    size: int = 192,
    device: str = "auto",
    supersample: int = 1,
    sharpness: float = 90.0,
    sharpness_start: float | None = None,
    sharpness_end: float | None = None,
    restarts: int = 1,
    seed: int = 0,
    log_every: int = 25,
    threads: int | None = 4,
    mode: str = "generic",
    save_debug: bool = False,
    debug_every: int = 50,
    debug_max_frames: int = 24,
) -> FitResult:
    """Optimize a YAML primitive template to a target crop."""
    _configure_torch_threads(threads)
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    device = _choose_device(device)
    torch.manual_seed(seed)
    random.seed(seed)
    spec = load_spec(template_path)
    target = load_target_tensor(target_path, (size, size), background=spec.canvas.background[:3]).to(device)
    weights = _resolve_weights(mode, spec.loss)
    fit_sharpness_start, fit_sharpness_end = _resolve_sharpness(mode, sharpness, sharpness_start, sharpness_end)

    best_state: dict[str, torch.Tensor] | None = None
    best_loss = math.inf
    best_metrics: dict[str, Any] = {}
    best_losses: list[float] = []
    all_restart_metrics: list[dict[str, Any]] = []

    debug_root = out_dir / "debug"
    if save_debug:
        debug_root.mkdir(parents=True, exist_ok=True)

    for restart in range(max(1, restarts)):
        scene = ProceduralScene(spec, device=device)
        _jitter_trainable(scene, amount=0.45 if restart else 0.0, seed=seed + restart * 101)
        opt = torch.optim.Adam(scene.parameters(), lr=lr)
        sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, T_max=max(1, int(steps)))
        losses: list[float] = []
        last_scalars: dict[str, float] = {}
        restart_debug_frames: list[Path] = []
        restart_dir = debug_root / f"restart_{restart + 1:03d}"
        restart_renders_dir = restart_dir / "renders"
        restart_comparisons_dir = restart_dir / "comparisons"
        restart_templates_dir = restart_dir / "templates"
        if save_debug:
            restart_renders_dir.mkdir(parents=True, exist_ok=True)
            restart_comparisons_dir.mkdir(parents=True, exist_ok=True)
            restart_templates_dir.mkdir(parents=True, exist_ok=True)
        for step in range(int(steps)):
            opt.zero_grad(set_to_none=True)
            step_sharpness = _annealed_value(fit_sharpness_start, fit_sharpness_end, step, int(steps))
            render = scene(RenderOptions(width=size, height=size, sharpness=step_sharpness, supersample=supersample))
            loss, scalars = image_fit_loss(render, target, weights=weights)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(scene.parameters(), 2.0)
            opt.step()
            sched.step()
            loss_value = float(loss.detach().cpu())
            losses.append(loss_value)
            last_scalars = dict(scalars)
            last_scalars["sharpness"] = float(step_sharpness)
            last_scalars["lr"] = float(opt.param_groups[0]["lr"])
            if log_every and (step == 0 or (step + 1) % log_every == 0 or step + 1 == steps):
                print(
                    f"restart={restart + 1}/{restarts} step={step + 1:04d}/{steps} "
                    f"loss={loss_value:.6f} sharpness={step_sharpness:.1f} lr={opt.param_groups[0]['lr']:.6f}"
                )
            if save_debug and _should_capture_debug(step, int(steps), int(debug_every), int(debug_max_frames)):
                frame_tag = f"step_{step + 1:04d}"
                render_path = restart_renders_dir / f"{frame_tag}.png"
                comp_path = restart_comparisons_dir / f"{frame_tag}.png"
                template_snapshot_path = restart_templates_dir / f"{frame_tag}.yaml"
                save_tensor_image(render.detach().cpu(), render_path)
                write_comparison(target.detach().cpu(), render.detach().cpu(), comp_path, title=f"restart {restart + 1} / step {step + 1}")
                write_spec(scene.to_spec(), template_snapshot_path)
                restart_debug_frames.append(comp_path)
        restart_summary: dict[str, Any] = {
            "restart": restart,
            "final_loss": float(losses[-1]) if losses else math.inf,
            "steps": len(losses),
            "final_terms": last_scalars,
        }
        if save_debug and restart_debug_frames:
            gif_path = restart_dir / "optimization.gif"
            write_debug_gif(restart_debug_frames, gif_path)
            restart_summary["debug_gif"] = str(gif_path)
            restart_summary["debug_comparison_frames"] = [str(p) for p in restart_debug_frames]
            restart_summary["debug_render_dir"] = str(restart_renders_dir)
            restart_summary["debug_comparison_dir"] = str(restart_comparisons_dir)
            restart_summary["debug_template_dir"] = str(restart_templates_dir)
        all_restart_metrics.append(restart_summary)
        if losses and losses[-1] < best_loss:
            best_loss = float(losses[-1])
            best_state = {k: v.detach().cpu().clone() for k, v in scene.state_dict().items()}
            best_metrics = {"restart": restart, "final_terms": last_scalars}
            best_losses = losses

    final_scene = ProceduralScene(spec, device=device)
    if best_state is not None:
        final_scene.load_state_dict({k: v.to(device) for k, v in best_state.items()})
    final_render = final_scene(
        RenderOptions(width=size, height=size, sharpness=fit_sharpness_end, supersample=max(1, supersample))
    )
    optimized_spec = final_scene.to_spec()
    optimized_spec.canvas.width = size
    optimized_spec.canvas.height = size
    optimized_spec.metadata.update(
        {
            "source_template": str(template_path),
            "target_image": str(target_path),
            "fit_steps": int(steps),
            "fit_restarts": int(restarts),
            "fit_size": int(size),
            "final_loss": float(best_loss),
            "fit_mode": str(mode),
            "fit_sharpness_start": float(fit_sharpness_start),
            "fit_sharpness_end": float(fit_sharpness_end),
        }
    )

    template_out = out_dir / "optimized_template.yaml"
    render_out = out_dir / "optimized_render.png"
    comparison_out = out_dir / "comparison.png"
    metrics_out = out_dir / "metrics.json"
    curve_out = out_dir / "loss_curve.png"
    csv_out = out_dir / "loss_curve.csv"

    write_spec(optimized_spec, template_out)
    save_tensor_image(final_render, render_out)
    write_comparison(target.detach().cpu(), final_render.detach().cpu(), comparison_out)
    write_loss_curve(best_losses, curve_out)
    with csv_out.open("w", newline="") as file:
        writer = csv.writer(file)
        writer.writerow(["step", "loss"])
        for idx, value in enumerate(best_losses):
            writer.writerow([idx, value])
    metrics = {
        "best_loss": best_loss,
        "steps": int(steps),
        "lr": float(lr),
        "device": str(device),
        "size": int(size),
        "supersample": int(supersample),
        "sharpness": float(sharpness),
        "sharpness_start": float(fit_sharpness_start),
        "sharpness_end": float(fit_sharpness_end),
        "mode": str(mode),
        "weights": {k: float(v) for k, v in weights.__dict__.items()},
        "outputs": {
            "optimized_template": str(template_out),
            "optimized_render": str(render_out),
            "comparison": str(comparison_out),
            "loss_curve_png": str(curve_out),
            "loss_curve_csv": str(csv_out),
            **({"debug_dir": str(debug_root)} if save_debug else {}),
        },
        "restarts": all_restart_metrics,
        **best_metrics,
    }
    metrics_out.write_text(json.dumps(metrics, indent=2))
    return FitResult(out_dir, best_loss, metrics_out, template_out, render_out, comparison_out)


def render_template(
    template_path: str | Path,
    out_path: str | Path,
    *,
    width: int | None = None,
    height: int | None = None,
    device: str = "auto",
    supersample: int = 2,
    threads: int | None = 4,
) -> Path:
    _configure_torch_threads(threads)
    spec = load_spec(template_path)
    device = _choose_device(device)
    width = int(width or spec.canvas.width)
    height = int(height or spec.canvas.height)
    scene = ProceduralScene(spec, device=device)
    img = scene(RenderOptions(width=width, height=height, supersample=supersample))
    save_tensor_image(img, out_path)
    return Path(out_path)
