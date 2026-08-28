"""Serve the moveset inspector and its review bank.

Stdlib only apart from PyYAML, which the review bank already needs. A tool that
required a web framework to look at frame data would be one more thing to
install before the first question can be asked.

    python -m ambition_moveset_inspector.server --open

Routes:
  ``/``                 the inspector UI (``web/``)
  ``/data/...``         the exported bundle and any recorded takes
  ``/api/review``       GET one review by subject, POST to save one
  ``/api/reviews``      GET every review
  ``/api/status``       GET what this server has, where, and how old it is
  ``/api/render``       GET a GPU-rendered animation of one fighter, on demand
  ``/render/...``       the rendered frames themselves
"""

from __future__ import annotations

import argparse
import json
import time
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from .reviews import Review, ReviewBank, normalize_score, snapshot_of, subject_parts

HERE = Path(__file__).resolve().parent.parent
WEB = HERE / "web"
DATA = HERE / "data"
REVIEWS = HERE / "reviews"
RENDERS = HERE / "data" / "renders"

# When THIS process booted, so the page can say how old the code answering it is.
_STARTED_AT = time.time()
_STARTED = time.strftime("%Y-%m-%d %H:%M", time.localtime(_STARTED_AT))

# ⭐⭐ THE REPO ROOT, for shelling out to the renderer. Resolved from this file so
# the wrapper works from any working directory.
REPO = HERE.parent.parent

# ⭐⭐ THE SPRITE SHEETS, SERVED WHERE THEY LIE. The inspector shows the real art
# rather than boxes, and the art is tens of megabytes of PNG that already exists
# in the engine's asset tree. Copying it into `data/` would double it on disk and
# make every export a sync problem; serving the engine's own directory read-only
# means the page always shows what the build would draw.
#
# ⛔ RESOLVED FROM THIS FILE, not from the working directory, so the wrapper can
# be run from anywhere.
SPRITES = (
    HERE.parent.parent
    / "crates"
    / "ambition_platformer2d_actor_monolith"
    / "assets"
    / "sprites"
)


def capture_scene_candidates() -> list[Path]:
    """Every place the renderer might be, in the order worth trying.

    ⛔⛔ NOT JUST `target/debug`. This hard-coded one path, which is wrong two
    ways that both bite real setups: `CARGO_TARGET_DIR` relocates the whole
    directory (this repo ships `scripts/setup_target_bindmount.sh` for exactly
    that), and a release build lands in `target/release`. Either one produced
    "capture_scene is not built" against a tree where it plainly was.

    ⭐ THE SAME CONVENTION `scripts/profile_desktop.sh` ALREADY USES —
    `${CARGO_TARGET_DIR:-$repo_root/target}`, release and debug — so the two
    agree about where this repo puts its binaries.
    """
    import os

    root = Path(os.environ.get("CARGO_TARGET_DIR") or (REPO / "target"))
    return [root / "release" / "capture_scene", root / "debug" / "capture_scene"]


def _built_at(path: Path) -> str | None:
    """When this binary was built, for the viewer to show beside its pictures."""
    import datetime

    try:
        stamp = path.stat().st_mtime
    except OSError:
        return None
    return datetime.datetime.fromtimestamp(stamp).strftime("%Y-%m-%d %H:%M")


def find_capture_scene() -> Path | None:
    for candidate in capture_scene_candidates():
        if candidate.exists():
            return candidate
    return None


def render_animation(character: str, frames: int, stride: int) -> tuple[int, dict]:
    """Render one fighter's animation through the real engine, on demand.

    ⭐⭐ THE GPU HALF OF THE DECISION. The Engine Takes view derives its animation
    frame on the CPU so the tool runs anywhere; this produces the REAL thing, by
    asking the engine to draw it. Jon, 2026-08-27: *"having the animation be
    generated on demand, and using a fallback visualization if it wasn't
    available or we didn't have the gpu."*

    ⛔ EVERY FAILURE IS A JSON ANSWER, NOT AN EXCEPTION. The whole point of this
    route is that it may legitimately be unavailable — no GPU, no binary built,
    a driver that will not start — and the UI has something to fall back to. A
    500 with a stack trace would make "this machine cannot render" look like a
    bug in the inspector.
    """
    import subprocess

    safe = "".join(ch for ch in character if ch.isalnum() or ch in "_-")
    if not safe or safe != character:
        return 400, {"error": "character must be a plain catalog id"}

    out_dir = RENDERS / safe
    manifest = out_dir / "manifest.json"
    # ⭐ CACHED BY WHAT WAS ASKED FOR. A second request for the same shape is a
    # file read; a request for MORE frames re-renders rather than silently
    # serving fewer than asked.
    if manifest.exists():
        try:
            have = json.loads(manifest.read_text())
            if have.get("frames") >= frames and have.get("stride") == stride:
                return 200, have
        except (OSError, ValueError):
            pass

    binary = find_capture_scene()
    if binary is None:
        return 503, {
            "available": False,
            "reason": "capture_scene is not built",
            "hint": "cargo build -p ambition_app_tools --bin capture_scene",
            "looked_in": [str(p) for p in capture_scene_candidates()],
        }

    out_dir.mkdir(parents=True, exist_ok=True)
    for stale in out_dir.glob("*.png"):
        stale.unlink()
    try:
        result = subprocess.run(
            [
                str(binary),
                "hall_of_characters",
                "player",
                str(out_dir / "frame.png"),
                "480x360",
                "--warmup", "60",
                "--character", safe,
                "--frames", str(frames),
                "--stride", str(stride),
            ],
            cwd=str(REPO),
            capture_output=True,
            text=True,
            timeout=900,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return 503, {"available": False, "reason": f"the renderer did not run: {error}"}

    shots = sorted(p.name for p in out_dir.glob("frame.*.png"))
    if result.returncode != 0 or not shots:
        # ⛔ THE RENDERER'S OWN LAST WORDS, not a generic failure. "no GPU" and
        # "that character is not on the roster" are different problems and the
        # person reading this is the one who can tell them apart.
        tail = (result.stderr or result.stdout or "").strip().splitlines()
        return 503, {
            "available": False,
            "reason": "the renderer produced no frames",
            "detail": tail[-3:] if tail else [],
        }

    # ⭐ THE BINARY'S OWN PROVENANCE TRAVELS WITH THE FRAMES. Nothing here builds,
    # so "which binary drew this and when was it built" is the only thing that
    # distinguishes a current picture from one an hour of engine changes ago.
    document = {
        "available": True,
        "character": safe,
        "frames": len(shots),
        "stride": stride,
        "renderer": str(binary),
        "renderer_built": _built_at(binary),
        "urls": [f"/render/{safe}/{name}" for name in shots],
    }
    manifest.write_text(json.dumps(document))
    return 200, document


def inspector_status() -> dict:
    """Everything the page needs to explain ITSELF.

    ⭐⭐ THE UI COULD NOT SAY WHAT IT WAS DOING. Jon, 2026-08-27: *"I don't see
    any art at the moment. Not sure if that's because the binaries are not built,
    because the webui is not reporting the status of what it has, or what it is
    doing. I can't tell if it is trying to call the tool or not, or if it knows
    where it is."* Every one of those was answerable on the server and none of it
    was reachable from the browser — the provenance went to a terminal the person
    looking at the pictures was not reading.
    """
    import os

    bundle_path = DATA / "moveset_bundle.json"
    takes_path = DATA / "takes" / "takes.json"
    takes_meta: dict = {"exists": takes_path.exists()}
    if takes_meta["exists"]:
        takes_meta["built"] = _built_at(takes_path)
    bundle_meta: dict = {"exists": bundle_path.exists()}
    if bundle_meta["exists"]:
        bundle_meta["built"] = _built_at(bundle_path)
        try:
            doc = json.loads(bundle_path.read_text())
            bundle_meta["schema"] = doc.get("schema")
            bundle_meta["fighters"] = len(doc.get("characters") or [])
            bundle_meta["sheets"] = len(doc.get("sheets") or {})
        except (OSError, ValueError) as error:
            bundle_meta["error"] = str(error)

    binaries = {}
    root = Path(os.environ.get("CARGO_TARGET_DIR") or (REPO / "target"))
    for name in ("moveset_export", "moveset_takes", "capture_scene"):
        found = next(
            (root / profile / name for profile in ("release", "debug")
             if (root / profile / name).exists()),
            None,
        )
        binaries[name] = {
            "found": found is not None,
            "path": str(found) if found else None,
            "built": _built_at(found) if found else None,
            "build_command": f"cargo build -p ambition_app_tools --bin {name}",
            "looked_in": [str(root / p / name) for p in ("release", "debug")],
        }

    # ⭐⭐ ARE THE TAKES CURRENT? "takes.json exists, recorded 15:53" is not the
    # question anybody has — the question is whether the recording carries the
    # fields this build DRAWS. A take made before the art fields existed shows no
    # sprites and no polygons, the Art button appears dead, and nothing on the
    # page says why. Rebuilding the binaries does NOT re-record; only running
    # moveset_takes does.
    if takes_meta["exists"]:
        try:
            doc = json.loads(takes_path.read_text())
            rows = doc.get("takes") if isinstance(doc, dict) else doc
            bodies = art = shapes = boxes = 0
            for take in rows or []:
                for frame in take.get("frames") or []:
                    for body in frame.get("bodies") or []:
                        bodies += 1
                        art += 1 if body.get("art") else 0
                    for hit in frame.get("hitboxes") or []:
                        boxes += 1
                        shapes += 1 if hit.get("shape") else 0
            takes_meta.update(
                takes=len(rows or []),
                bodies=bodies,
                with_art=art,
                hitboxes=boxes,
                with_shape=shapes,
            )
            stale = []
            if bodies and not art:
                stale.append("no sprite art (bodies carry no `art`)")
            if boxes and not shapes:
                stale.append("no hitbox geometry (strikes carry no `shape`)")
            if stale:
                takes_meta["stale"] = (
                    "recorded before this build: "
                    + ", ".join(stale)
                    + " — re-run moveset_takes"
                )
        except (OSError, ValueError) as error:
            takes_meta["error"] = str(error)

    return {
        # ⛔⛔ HOW OLD IS THE PROCESS ANSWERING YOU. `server.py` is loaded into
        # memory at start, so a server left running across an edit serves its OWN
        # ROUTES from before that edit however current the files on disk are — a
        # 19-hour-old process had no `/art/` route at all and 404'd every sprite,
        # which is unfalsifiable from the browser. This is the fact that makes it
        # falsifiable.
        "server_started": _STARTED,
        "server_uptime_minutes": int((time.time() - _STARTED_AT) / 60),
        "server_module": str(Path(__file__).resolve()),
        "repo": str(REPO),
        "sprites_dir": str(SPRITES),
        "sprites_dir_exists": SPRITES.exists(),
        "bundle": bundle_meta,
        "takes": takes_meta,
        "renders_dir": str(RENDERS),
        "cached_renders": sorted(p.name for p in RENDERS.glob("*")) if RENDERS.exists() else [],
        "binaries": binaries,
    }


def _contained(root: Path, relative: str) -> str:
    """Join ``relative`` under ``root``, refusing anything that escapes.

    ⛔⛔ THE CHECK IS LEXICAL, AND IT HAS TO BE. This resolved the CANDIDATE and
    asserted `relative_to(root)` — which is the obvious spelling and is wrong
    wherever the served files are symlinks. This repo's sprite assets are exactly
    that: symlinks into the main checkout. `resolve()` followed them OUT of the
    worktree, containment failed, and the route 404'd every one of its own
    legitimate files. That is why the inspector showed no art, and no amount of
    looking at the browser would have found it.

    ⭐ `normpath` COLLAPSES `..` WITHOUT TOUCHING THE FILESYSTEM, which is the
    question a traversal guard actually asks: does the REQUESTED PATH climb out
    of the root? Where the bytes ultimately live is the asset tree's business.
    """
    import os

    rel = os.path.normpath(relative.lstrip("/"))
    if rel.startswith("..") or os.path.isabs(rel):
        # Answer as a miss rather than an error, so a probe learns nothing about
        # what does or does not exist.
        return str(root / "__outside_the_served_root__")
    return str(root / rel)


class InspectorHandler(SimpleHTTPRequestHandler):
    """Static files out of ``web/``, with ``data/`` and ``api/`` grafted on."""

    def __init__(self, *args, bank: ReviewBank, **kwargs):
        self.bank = bank
        super().__init__(*args, directory=str(WEB), **kwargs)

    # ---- plumbing ----
    def log_message(self, fmt, *args):  # noqa: D102 - quiet by default
        pass

    def end_headers(self) -> None:  # noqa: D102
        """Never let a browser cache the UI.

        ⛔⛔ A CACHED `app.js` IS AN INVISIBLE OLD TOOL. The page has no version
        on it, so a stale script shows a stale UI — a missing control reads as a
        broken feature, and the person looking at it has no way to tell those
        apart. This tool is served from disk on localhost; there is nothing to
        gain by caching it and a whole class of phantom bugs to lose.
        """
        self.send_header("Cache-Control", "no-store, must-revalidate")
        super().end_headers()

    def _json(self, status: int, payload) -> None:
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def translate_path(self, path: str) -> str:
        """``/data/``, ``/render/`` and ``/art/`` live outside ``web/``.

        The bundle and the renders are generated artifacts and the UI is source;
        keeping them in one directory would mean either committing the artifacts
        or gitignoring a path inside the source tree. The sprites are the
        engine's own, served where they lie.
        """
        parsed = urlparse(path).path
        for prefix, root in (
            ("/data/", DATA),
            ("/render/", RENDERS),
            ("/art/", SPRITES),
        ):
            if parsed.startswith(prefix):
                return _contained(root, parsed[len(prefix):])
        return super().translate_path(path)

    # ---- routes ----
    def do_GET(self):  # noqa: N802 - stdlib naming
        parsed = urlparse(self.path)
        if parsed.path == "/api/status":
            return self._json(200, inspector_status())
        if parsed.path == "/api/render":
            query = parse_qs(parsed.query)
            character = (query.get("character") or [""])[0]
            if not character:
                return self._json(400, {"error": "character is required"})
            frames = min(int((query.get("frames") or ["24"])[0]), 120)
            stride = min(int((query.get("stride") or ["2"])[0]), 30)
            return self._json(*render_animation(character, frames, stride))
        if parsed.path == "/api/review":
            subject = (parse_qs(parsed.query).get("subject") or [""])[0]
            if not subject:
                return self._json(400, {"error": "subject is required"})
            review = self.bank.load(subject)
            if review is None:
                return self._json(404, {"error": "no review yet"})
            return self._json(200, review.to_document())
        if parsed.path == "/api/reviews":
            return self._json(200, [r.to_document() for r in self.bank.all()])
        return super().do_GET()

    def do_POST(self):  # noqa: N802 - stdlib naming
        parsed = urlparse(self.path)
        if parsed.path != "/api/review":
            return self._json(404, {"error": "no such endpoint"})
        length = int(self.headers.get("content-length") or 0)
        try:
            payload = json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError as err:
            return self._json(400, {"error": f"bad json: {err}"})

        subject = str(payload.get("subject") or "").strip()
        if not subject:
            return self._json(400, {"error": "subject is required"})
        character, move = subject_parts(subject)

        # The snapshot is read from the bundle the reviewer was looking at, so a
        # note carries the numbers it was written about even after a re-tune.
        snapshot = {}
        bundle_path = DATA / "moveset_bundle.json"
        if bundle_path.exists():
            try:
                bundle = json.loads(bundle_path.read_text())
                row = next((c for c in bundle["characters"] if c["id"] == character), None)
                if row is not None:
                    snapshot = snapshot_of(row, move)
            except (json.JSONDecodeError, KeyError, OSError):
                snapshot = {}

        review = Review(
            subject=subject,
            character=str(payload.get("character") or character),
            move=payload.get("move") or move,
            score=normalize_score(payload.get("score")),
            notes=str(payload.get("notes") or ""),
            issues=list(payload.get("issues") or []),
            snapshot=snapshot,
            cast_generation=payload.get("cast_generation"),
        )
        path = self.bank.save(review)
        return self._json(200, {"path": str(path.relative_to(HERE)), "saved": review.to_document()})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8777)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--open", action="store_true", help="open a browser at the UI")
    parser.add_argument(
        "--report",
        action="store_true",
        help="print the standing feedback and exit, for an agent asked to address it",
    )
    args = parser.parse_args(argv)

    bank = ReviewBank(REVIEWS)
    if args.report:
        from .reviews import format_report

        open_work = bank.open_work()
        print(format_report(open_work) or "no open feedback")
        return 0

    if not (DATA / "moveset_bundle.json").exists():
        print(
            "[inspector] no bundle yet — run:\n"
            "    cargo run -p ambition_app_tools --bin moveset_export\n"
            "the UI will load but every view will be empty."
        )

    handler = partial(InspectorHandler, bank=bank)
    server = ThreadingHTTPServer((args.host, args.port), handler)
    url = f"http://{args.host}:{args.port}/"
    print(f"[inspector] {url}")
    if args.open:
        import webbrowser

        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
