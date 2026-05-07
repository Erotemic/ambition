from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

from ..adapters import BaseAdapter
from ..config import CharacterJob
from ..textures import generate_texture_pack


def default_blender_executable(job: CharacterJob) -> str:
    return (
        job.render.blender_executable
        or os.environ.get("GEN3D_BLENDER_LAB_BLENDER_BIN")
        or os.environ.get("PROC_CHARACTER_LAB_BLENDER_BIN")
        or os.environ.get("BLENDER_BIN")
        or "blender"
    )


def scene_builder_script() -> Path:
    return Path(__file__).with_name("scene_builder.py")


def build_payload(adapter: BaseAdapter, job: CharacterJob, requests: Iterable[Dict[str, Any]], mode: str, texture_paths: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
    spec = adapter.spec_dict(adapter.sample_spec(job))
    return {
        "mode": mode,
        "target": adapter.target,
        "job": job.model_dump(exclude_none=True),
        "spec": spec,
        "requests": list(requests),
        "canonical_pose": list(adapter.canonical_pose()),
        "texture_paths": dict(texture_paths or {}),
    }


def _env_truthy(value: object) -> bool:
    return str(value or "").strip().lower() in {"1", "true", "yes", "on", "debug", "verbose"}


def _run_blender(payload_path: Path, blender_executable: str, *, log_path: Optional[Path] = None, verbose: Optional[bool] = None) -> None:
    cmd = [
        blender_executable,
        "--background",
        "--factory-startup",
        "--python",
        str(scene_builder_script()),
        "--",
        "--payload",
        str(payload_path),
    ]
    if verbose is None:
        verbose = _env_truthy(os.environ.get("GEN3D_BLENDER_LAB_VERBOSE"))
    if verbose:
        subprocess.run(cmd, check=True)
        return
    proc = subprocess.run(cmd, check=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if proc.returncode != 0:
        output = proc.stdout or ""
        if log_path is not None:
            log_path.parent.mkdir(parents=True, exist_ok=True)
            log_path.write_text(output, encoding="utf-8", errors="replace")
        tail = "\n".join(output.splitlines()[-80:])
        message = f"Blender render failed with exit code {proc.returncode}."
        if log_path is not None:
            message += f" Full Blender log: {log_path}"
        if tail:
            message += "\n--- Blender output tail ---\n" + tail
        raise RuntimeError(message)


def render_requests(adapter: BaseAdapter, job: CharacterJob, requests: Iterable[Dict[str, Any]], mode: str = "frames", blender_executable: Optional[str] = None) -> Dict[str, Any]:
    blender_executable = blender_executable or default_blender_executable(job)
    request_list = list(requests)
    log_path = None
    if request_list:
        log_path = Path(request_list[0]["out_path"]).parent / "_blender_render.log"
    with tempfile.TemporaryDirectory(prefix="gen3d_blender_lab_") as d:
        temp_root = Path(d)
        textures_dir = temp_root / "textures"
        spec = adapter.spec_dict(adapter.sample_spec(job))
        texture_paths = generate_texture_pack(textures_dir, spec, adapter.target)
        payload = build_payload(adapter, job, request_list, mode, texture_paths=texture_paths)
        payload_path = temp_root / "payload.json"
        payload_path.write_text(json.dumps(payload), encoding="utf-8")
        _run_blender(payload_path, blender_executable, log_path=log_path)
    for req in payload["requests"]:
        path = Path(req["out_path"])
        if not path.exists():
            raise RuntimeError(f"Expected Blender output was not created: {path}")
    return payload
