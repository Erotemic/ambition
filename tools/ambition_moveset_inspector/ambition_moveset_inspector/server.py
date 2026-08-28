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


def renderer_candidates() -> list[Path]:
    """Every place `moveset_render` might be, in the order worth trying.

    ⛔⛔ NOT JUST `target/debug`. `CARGO_TARGET_DIR` relocates the whole directory
    (this repo ships `scripts/setup_target_bindmount.sh` for exactly that) and a
    release build lands in `target/release`. Either produced "not built" against
    a tree where it plainly was. Same convention `scripts/profile_desktop.sh`
    uses.
    """
    import os

    root = Path(os.environ.get("CARGO_TARGET_DIR") or (REPO / "target"))
    return [root / "release" / "moveset_render", root / "debug" / "moveset_render"]


def find_renderer() -> Path | None:
    for candidate in renderer_candidates():
        if candidate.exists():
            return candidate
    return None


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


def render_animation(character: str, verb: str, frames: int, stride: int) -> tuple[int, dict]:
    """Render one fighter PERFORMING ONE MOVE, through the real engine.

    ⭐⭐ CHARACTER **AND VERB**. This took only a character and photographed a
    fighter STANDING in `hall_of_characters` — twenty-four frames of somebody
    doing nothing, cached under whatever move you happened to be looking at.
    `moveset_render` drives the move through the ordinary control-frame seam and
    names the exact `SimTick` of every PNG.

    ⛔ EVERY FAILURE IS A JSON ANSWER, NOT AN EXCEPTION. This route may
    legitimately be unavailable — no GPU, no binary, a driver that will not
    start — and the viewer has a CPU fallback. A 500 would make "this machine
    cannot render" look like a bug in the inspector.
    """
    import subprocess

    safe = "".join(ch for ch in character if ch.isalnum() or ch in "_-")
    safe_verb = "".join(ch for ch in verb if ch.isalnum() or ch in "_-")
    if not safe or safe != character:
        return 400, {"error": "character must be a plain catalog id"}
    if not safe_verb or safe_verb != verb:
        return 400, {"error": "verb must be a plain repertoire verb"}

    # ⭐ CACHED BY WHAT WAS ASKED FOR — character AND verb AND shape. Caching by
    # character alone served the up-B's frames for a jab.
    out_dir = RENDERS / f"{safe}__{safe_verb}"
    manifest = out_dir / "manifest.json"
    if manifest.exists():
        try:
            have = json.loads(manifest.read_text())
            if have.get("frames", 0) >= frames and have.get("stride") == stride:
                return 200, have
        except (OSError, ValueError):
            pass

    binary = find_renderer()
    if binary is None:
        return 503, {
            "available": False,
            "reason": "moveset_render is not built",
            "hint": "cargo build -p ambition_app_tools --bin moveset_render",
            "looked_in": [str(p) for p in renderer_candidates()],
        }

    out_dir.mkdir(parents=True, exist_ok=True)
    try:
        result = subprocess.run(
            [
                str(binary),
                "--character", safe,
                "--verb", safe_verb,
                "--out", str(out_dir),
                "--frames", str(frames),
                "--stride", str(stride),
            ],
            cwd=str(REPO),
            capture_output=True,
            text=True,
            timeout=1800,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return 503, {"available": False, "reason": f"the renderer did not run: {error}"}

    if result.returncode != 0 or not manifest.exists():
        tail = (result.stderr or result.stdout or "").strip().splitlines()
        return 503, {
            "available": False,
            "reason": "the renderer produced no frames",
            "detail": tail[-3:] if tail else [],
        }

    document = json.loads(manifest.read_text())
    document.update(
        available=True,
        renderer_path=str(binary),
        renderer_built=_built_at(binary),
        urls=[f"/render/{out_dir.name}/{shot['file']}" for shot in document.get("shots", [])],
    )
    # ⛔⛔ A MISMATCH IS REPORTED, NEVER CACHED UNDER THE REQUESTED NAME. A press
    # is a REQUEST; the engine decides what comes out. Serving another move's
    # animation as this one's is the worst thing a reference tool can do.
    observed = document.get("observed_moves") or []
    document["mismatch"] = not document.get("reached_a_move")
    if document["mismatch"]:
        document["reason"] = f"no move became active for `{safe_verb}`"
    manifest.write_text(json.dumps(document))
    return 200, document


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
            verb = (query.get("verb") or [""])[0]
            if not character or not verb:
                return self._json(400, {"error": "character and verb are required"})
            frames = min(int((query.get("frames") or ["24"])[0]), 120)
            stride = min(int((query.get("stride") or ["2"])[0]), 30)
            return self._json(*render_animation(character, verb, frames, stride))
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
