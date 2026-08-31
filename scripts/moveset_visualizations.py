#!/usr/bin/env python3
"""Generate every move's visualization — with a GPU if there is one, without if not.

⭐⭐ TWO PATHS, ONE COMMAND, AND THE PICK IS STATED. An ENGINE RENDER needs a WGPU
adapter and shows the real art with the real runtime volumes over it; a
DIAGNOSTIC SHEET needs nothing at all and shows the same geometry as SVG. A VM
may have either, and a corpus job that assumed the first fails on half the
machines this project runs on.

⛔ IT NEVER PRETENDS. The index says which path ran and why, so a directory of
SVGs is legible as "this machine had no adapter" rather than as a broken render.

    python3 scripts/moveset_visualizations.py --out /tmp/moveset_viz
    python3 scripts/moveset_visualizations.py --out DIR --characters npc_pirate_admiral
    python3 scripts/moveset_visualizations.py --out DIR --no-gpu   # force the SVG path

⛔ IT DOES NOT RECORD TAKES. The diagnostic path reads a `takes.json`, and
recording the grid is a ~27 minute job of its own — so a missing recording is
reported with the command that makes one, never run behind your back.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_TAKES = REPO / "tools/ambition_moveset_inspector/data/takes/takes.json"


def gpu_verdict() -> dict:
    """Ask the doctor that already exists, rather than probing a second way."""
    spec = importlib.util.spec_from_file_location(
        "render_capability_doctor", REPO / "scripts/render_capability_doctor.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.report()


def renderer() -> Path | None:
    """The newest built `moveset_render`, or nothing.

    ⛔ NEWEST, NOT FIRST. A release binary from last week must not outrank a
    debug one built a minute ago — the same rule the inspector server learned.
    """
    found = [
        p
        for p in (REPO / "target" / profile / "moveset_render" for profile in ("release", "debug"))
        if p.exists()
    ]
    return max(found, key=lambda p: p.stat().st_mtime) if found else None


def render_corpus(out: Path, characters: str, verbs: str, frames: int, stride: int) -> dict:
    """The engine path: one app, every pair, real pixels."""
    binary = renderer()
    if binary is None:
        return {
            "ran": False,
            "why": "moveset_render is not built",
            "fix": "cargo build -p ambition_app_tools --bin moveset_render",
        }
    out.mkdir(parents=True, exist_ok=True)
    started = time.time()
    # ⛔⛔ STREAMED, NOT CAPTURED. A grid is hundreds of renders and several
    # minutes; a driver that swallowed the child's progress lines would go quiet
    # for all of it, which reads as hung — the same lesson `moveset_takes` has
    # written on its own `--characters grid` flag. The tail is kept for the
    # report; the lines are echoed as they arrive.
    tail: list[str] = []
    child = subprocess.Popen(
        [
            str(binary),
            "--characters", characters,
            "--verbs", verbs,
            "--frames", str(frames),
            "--stride", str(stride),
            # ⭐ SOFTWARE BY NAME. `auto` means the machine decides, and two
            # corpora drawn by different adapters are not comparable — pinning
            # Lavapipe is what makes an overnight run reproducible on a VM.
            "--adapter", "software",
            "--out", str(out),
        ],
        cwd=str(REPO),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    for line in child.stdout:  # type: ignore[union-attr]
        line = line.rstrip()
        # The engine logs a great deal; the render lines are the progress.
        if line.startswith("[render]") or line.startswith("[moveset-render]"):
            print(f"  {line}", flush=True)
        tail.append(line)
        del tail[:-40]
    code = child.wait()
    index = out / "index.json"
    doc = json.loads(index.read_text()) if index.exists() else {}
    return {
        "ran": code == 0 and index.exists(),
        "binary": str(binary),
        "seconds": round(time.time() - started, 1),
        "renders": len(doc.get("renders") or []),
        "failures": doc.get("failures"),
        "seconds_per_render": doc.get("seconds_per_render"),
        "detail": tail[-3:] if code else [],
    }


def diagnostic_corpus(out: Path, takes: Path, character: str | None) -> dict:
    """The no-GPU path: SVG sheets from a recording, no adapter, no rasterizer."""
    if not takes.exists():
        return {
            "ran": False,
            "why": f"no recording at {takes}",
            "fix": "cargo run -p ambition_app_tools --bin moveset_takes -- --characters grid",
        }
    started = time.time()
    argv = ["--takes", str(takes), "--out", str(out)]
    if character:
        argv += ["--character", character]
    result = subprocess.run(
        [sys.executable, str(REPO / "scripts/render_take_diagnostic.py"), *argv],
        cwd=str(REPO),
        capture_output=True,
        text=True,
    )
    return {
        "ran": result.returncode == 0,
        "seconds": round(time.time() - started, 1),
        "sheets": len(list(out.glob("*.svg"))) if out.exists() else 0,
        "detail": (result.stdout or result.stderr or "").strip().splitlines()[-2:],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path, help="directory for the corpus")
    parser.add_argument("--characters", default="grid", help="grid | all | id,id")
    parser.add_argument("--verbs", default="all", help="all | verb,verb")
    parser.add_argument("--frames", type=int, default=12)
    parser.add_argument("--stride", type=int, default=2)
    parser.add_argument("--takes", type=Path, default=DEFAULT_TAKES)
    parser.add_argument(
        "--no-gpu",
        action="store_true",
        help="skip the engine render even if an adapter is available",
    )
    args = parser.parse_args()

    verdict = gpu_verdict()
    usable = verdict.get("offscreen_capture") == "likely" and not args.no_gpu
    print(f"[viz] adapter: {verdict.get('offscreen_capture')} — {verdict.get('hint')}")
    if args.no_gpu:
        print("[viz] --no-gpu: taking the diagnostic path regardless")

    args.out.mkdir(parents=True, exist_ok=True)
    report: dict = {
        "schema": "ambition.moveset_visualizations.v1",
        "gpu": verdict,
        "engine_render": None,
        "diagnostic": None,
    }

    if usable:
        print("[viz] engine render: one app, every pair…")
        report["engine_render"] = render_corpus(
            args.out / "render", args.characters, args.verbs, args.frames, args.stride
        )
        state = report["engine_render"]
        if state["ran"]:
            print(
                f"[viz] engine render: {state['renders']} render(s) in {state['seconds']}s "
                f"({state.get('seconds_per_render')}s each), {state['failures']} failed"
            )
        else:
            print(f"[viz] engine render SKIPPED — {state.get('why')}")
            if state.get("fix"):
                print(f"[viz]   {state['fix']}")

    # ⭐ THE SHEETS ARE FREE AND ALWAYS WORTH HAVING. They need no adapter, they
    # diff as text, and they carry the same runtime geometry — so they are drawn
    # whether or not the pixels were.
    # A sheet run takes ONE character or all of them; `grid`, `all` and a list
    # all mean "do not filter".
    named_one = args.characters not in ("grid", "all") and "," not in args.characters
    single = args.characters if named_one else None
    print("[viz] diagnostic sheets: no adapter, no rasterizer…")
    report["diagnostic"] = diagnostic_corpus(args.out / "diagnostic", args.takes, single)
    state = report["diagnostic"]
    if state["ran"]:
        print(f"[viz] diagnostic: {state['sheets']} sheet(s) in {state['seconds']}s")
    else:
        print(f"[viz] diagnostic SKIPPED — {state.get('why')}")
        if state.get("fix"):
            print(f"[viz]   {state['fix']}")

    (args.out / "index.json").write_text(json.dumps(report, indent=2), encoding="utf8")
    print(f"file://{(args.out / 'index.json').resolve()}")
    print(f"file://{args.out.resolve()}")
    # ⛔ NOTHING PRODUCED IS A FAILURE. A run that took neither path and exited 0
    # would look like a corpus that is simply empty.
    produced = (report["engine_render"] or {}).get("ran") or report["diagnostic"]["ran"]
    return 0 if produced else 1


if __name__ == "__main__":
    raise SystemExit(main())
