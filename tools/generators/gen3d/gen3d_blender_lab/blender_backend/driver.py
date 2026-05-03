from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

from ..adapters import BaseAdapter
from ..config import CharacterJob


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


def build_payload(adapter: BaseAdapter, job: CharacterJob, requests: Iterable[Dict[str, Any]], mode: str) -> Dict[str, Any]:
    spec = adapter.spec_dict(adapter.sample_spec(job))
    return {
        "mode": mode,
        "target": adapter.target,
        "job": job.model_dump(exclude_none=True),
        "spec": spec,
        "requests": list(requests),
        "canonical_pose": list(adapter.canonical_pose()),
    }


def _run_blender(payload_path: Path, blender_executable: str) -> None:
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
    subprocess.run(cmd, check=True)


def render_requests(adapter: BaseAdapter, job: CharacterJob, requests: Iterable[Dict[str, Any]], mode: str = "frames", blender_executable: Optional[str] = None) -> Dict[str, Any]:
    payload = build_payload(adapter, job, requests, mode)
    blender_executable = blender_executable or default_blender_executable(job)
    with tempfile.TemporaryDirectory(prefix="gen3d_blender_lab_") as d:
        payload_path = Path(d) / "payload.json"
        payload_path.write_text(json.dumps(payload), encoding="utf-8")
        _run_blender(payload_path, blender_executable)
    for req in payload["requests"]:
        path = Path(req["out_path"])
        if not path.exists():
            raise RuntimeError(f"Expected Blender output was not created: {path}")
    return payload
