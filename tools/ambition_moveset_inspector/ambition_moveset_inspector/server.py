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
  ``/api/take``         GET/POST one canonical runtime take, generating if needed
  ``/api/render``       GET/POST matching GPU evidence for one canonical scenario
  ``/render/...``       the rendered frames themselves
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
import threading
import time
from dataclasses import dataclass
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
TAKE_CACHE = HERE / "data" / "takes" / "by-scenario"

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


def binary_candidates(name: str) -> list[Path]:
    """Every ordinary target path for one inspector binary.

    ⛔⛔ NOT JUST `target/debug`. `CARGO_TARGET_DIR` relocates the whole directory
    (this repo ships `scripts/setup/target_bindmount.sh` for exactly that) and a
    release build lands in `target/release`. Either produced "not built" against
    a tree where it plainly was. Same convention `scripts/profile_desktop.sh`
    uses.
    """
    configured = os.environ.get("CARGO_TARGET_DIR")
    root = Path(configured) if configured else REPO / "target"
    if not root.is_absolute():
        root = REPO / root
    return [root / "release" / name, root / "debug" / name]


def renderer_candidates() -> list[Path]:
    """Backward-compatible spelling used by status/error text and tests."""
    return binary_candidates("moveset_render")


def _newest(candidates: list[Path]) -> Path | None:
    """The most recently built of these, or `None` if none exist.

    ⛔⛤ NEWEST, NOT FIRST. Taking the first existing candidate meant `release`
    always outranked `debug` — so a release binary from last week beat a debug
    binary built a minute ago, while the build hint this same server prints tells
    the reader to run an ordinary debug `cargo build`. The tool preferred the
    binary its own advice does not produce, and nothing on the page said which
    one had answered.
    """
    live = [(c.stat().st_mtime, c) for c in candidates if c.is_file() and os.access(c, os.X_OK)]
    return max(live)[1] if live else None


def find_binary(name: str) -> Path | None:
    """The exact inspector binary this process will run.

    ⭐ AN EXPLICIT OVERRIDE WINS OUTRIGHT. Somebody testing a build elsewhere has
    said which one they mean, and a freshness heuristic has no business
    second-guessing that.
    """
    override = os.environ.get(f"AMBITION_{name.upper()}")
    if override:
        path = Path(override)
        return path if path.is_file() and os.access(path, os.X_OK) else None
    return _newest(binary_candidates(name))


def find_renderer() -> Path | None:
    """Backward-compatible convenience wrapper."""
    return find_binary("moveset_render")


def _built_at(path: Path) -> str | None:
    """When this binary was built, for the viewer to show beside its pictures."""
    import datetime

    try:
        stamp = path.stat().st_mtime
    except OSError:
        return None
    return datetime.datetime.fromtimestamp(stamp).strftime("%Y-%m-%d %H:%M")


def _plain_id(value: object, field: str) -> str:
    text = str(value or "").strip()
    safe = "".join(ch for ch in text if ch.isalnum() or ch in "_-")
    if not text or safe != text:
        raise ValueError(f"{field} must be a plain catalog/repertoire id")
    return text


@dataclass(frozen=True)
class CombatScenario:
    """One simulation-defining inspector request.

    Omission is deliberately not semantic. A mirror is represented literally as
    ``target == subject``; passive and CPU mirrors therefore have distinct
    identities. The current move exercise has one canonical charge/hold policy,
    so that policy is named in the identity instead of copying its tick count
    into Python.
    """

    subject: str
    target: str
    target_behavior: str
    verb: str
    spacing: float | None = None
    chain: str | None = None
    chain_at: int | None = None
    hold_policy: str = "move_exercise_default"

    @classmethod
    def from_mapping(cls, raw: dict) -> "CombatScenario":
        subject = _plain_id(raw.get("subject") or raw.get("character"), "subject")
        verb = _plain_id(raw.get("verb"), "verb")
        # A missing target in the public API means the canonical mirror, and is
        # normalized immediately. Nothing below this boundary sees the omission.
        target = _plain_id(raw.get("target") or subject, "target")
        behavior = str(raw.get("target_behavior") or raw.get("behavior") or "passive")
        if behavior not in {"passive", "cpu"}:
            raise ValueError("target_behavior must be 'passive' or 'cpu'")

        spacing = raw.get("spacing", raw.get("requested_spacing"))
        if spacing in ("", None):
            spacing = None
        else:
            try:
                spacing = float(spacing)
            except (TypeError, ValueError) as error:
                raise ValueError("spacing must be a number of pixels") from error
            if not 0.0 <= spacing <= 2000.0:
                raise ValueError("spacing must be between 0 and 2000 px")

        chain_raw = raw.get("chain")
        chain = None
        chain_at = raw.get("chain_at")
        if isinstance(chain_raw, dict):
            chain = chain_raw.get("verb")
            if chain_at is None:
                chain_at = chain_raw.get("at")
        elif chain_raw:
            chain = chain_raw
        if chain:
            chain = _plain_id(chain, "chain")
            if chain_at is None:
                raise ValueError("chain_at is required for a canonical chain scenario")
            try:
                chain_at = int(chain_at)
            except (TypeError, ValueError) as error:
                raise ValueError("chain_at must be an action tick") from error
            if chain_at < 0:
                raise ValueError("chain_at must be non-negative")
        else:
            chain_at = None

        hold_policy = str(raw.get("hold_policy") or "move_exercise_default")
        if hold_policy != "move_exercise_default":
            raise ValueError("only the canonical move_exercise_default hold policy is supported")
        return cls(
            subject=subject,
            target=target,
            target_behavior=behavior,
            verb=verb,
            spacing=spacing,
            chain=chain,
            chain_at=chain_at,
            hold_policy=hold_policy,
        )

    def document(self) -> dict:
        return {
            "subject": self.subject,
            "target": self.target,
            "target_behavior": self.target_behavior,
            "verb": self.verb,
            "spacing": self.spacing,
            "chain": (
                {"verb": self.chain, "at": self.chain_at}
                if self.chain is not None
                else None
            ),
            "hold_policy": self.hold_policy,
        }

    def identity(self) -> str:
        payload = json.dumps(self.document(), sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(payload.encode()).hexdigest()

    def cache_name(self) -> str:
        readable = scenario_key(
            self.subject,
            self.verb,
            self.target,
            self.spacing,
            self.target_behavior,
        )
        if self.chain:
            readable += f"__then_{self.chain}__at_{self.chain_at if self.chain_at is not None else 'default'}"
        return f"{readable}__{self.identity()[:12]}"

    @property
    def renderable(self) -> bool:
        # moveset_render does not yet reproduce an A→B schedule.
        return self.chain is None


def scenario_from_take(take: dict) -> CombatScenario:
    return CombatScenario.from_mapping(
        {
            "subject": take.get("subject") or take.get("character"),
            "target": take.get("target") or take.get("subject") or take.get("character"),
            "target_behavior": take.get("target_behavior") or "passive",
            "verb": take.get("verb"),
            "spacing": take.get("requested_spacing"),
            "chain": take.get("chain"),
            "hold_policy": take.get("hold_policy") or "move_exercise_default",
        }
    )


def scenario_key(
    character: str,
    verb: str,
    target: str | None,
    spacing: float | None,
    target_behavior: str,
) -> str:
    """The cache directory for one SCENARIO — every input that changes the fight.

    ⭐ CACHED BY WHAT WAS ASKED FOR. Caching by character alone once served the
    up-B's frames for a jab; caching by character and verb alone served a render
    taken from across the stage as evidence for a take recorded at 40px.

    ⛔⛔ AND NOT `int(spacing)`. Truncating put 40.1 and 40.9 in ONE directory, so
    the second request was served the first one's pictures under its own number —
    reproduced by the 2026-08-31 review. Three decimals is finer than any spacing
    a scenario asks for and is stable across platforms; `.` becomes `_` so the key
    stays a path component.

    ⚠ THE BEHAVIOUR RIDES WITH THE TARGET IT DESCRIBES. A scenario with no
    opponent has nothing to behave, and putting `__passive` in that key would
    make two identical solo renders look like different experiments.

    ⛔⛔ AND A MIRROR IS STILL A SCENARIO. This read `target != character`, so
    George-vs-George contributed NOTHING — not even its behaviour — and a CPU
    mirror and a passive mirror shared one directory. The recorder DEFAULTS to a
    mirror match, so that is the ordinary case, not an exotic one. Omission is
    never how a scenario says something; only a genuinely absent target is
    absent.
    """
    scenario = ""
    if target:
        scenario += f"__vs_{target}__{target_behavior}"
    if spacing is not None:
        scenario += "__at" + f"{spacing:.3f}".replace(".", "_")
    return f"{character}__{verb}{scenario}"


def _same_scenario_value(asked, drawn) -> bool:
    """Do a requested and a rendered scenario field agree?

    ⚠ FLOATS COMPARE WITH A TOLERANCE, and it is not laziness: the request comes
    from JSON and the manifest from Rust's `f32`, so an exact `==` on a spacing
    would call every cache entry a mismatch and re-render the world. The
    tolerance is far tighter than the truncation it replaces.
    """
    if asked is None or drawn is None:
        return asked is None and drawn is None
    if isinstance(asked, (int, float)) and isinstance(drawn, (int, float)):
        return abs(float(asked) - float(drawn)) < 1e-3
    return asked == drawn


def _repository_identity() -> str:
    """Commit plus working-tree diff, for generated-evidence provenance."""
    try:
        head = subprocess.run(
            ["git", "-C", str(REPO), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=5,
            check=True,
        ).stdout.strip()
        dirty = subprocess.run(
            ["git", "-C", str(REPO), "diff", "--no-ext-diff", "HEAD", "--"],
            capture_output=True,
            timeout=10,
            check=True,
        ).stdout
        status = subprocess.run(
            ["git", "-C", str(REPO), "status", "--porcelain=v1", "--untracked-files=all"],
            capture_output=True,
            timeout=5,
            check=True,
        ).stdout
        digest = hashlib.sha256(dirty + status).hexdigest()[:16]
        return f"{head}:{digest}"
    except (OSError, subprocess.SubprocessError):
        return "unknown"


_REPORT_MODULE = None


def _report_for_take(take: dict, sim_hz: float, source_doc: dict | None = None) -> dict:
    """Run the repository's existing runtime-measurement reader, not a JS copy."""
    global _REPORT_MODULE
    if _REPORT_MODULE is None:
        import importlib.util

        path = REPO / "scripts" / "moveset_report.py"
        spec = importlib.util.spec_from_file_location("ambition_moveset_report", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _REPORT_MODULE = module
    return _REPORT_MODULE.report(take, sim_hz=sim_hz, source=None, bundle=source_doc or {})


_TAKE_LOCKS: dict[str, threading.Lock] = {}
_TAKE_LOCKS_GUARD = threading.Lock()


def _scenario_lock(identity: str) -> threading.Lock:
    with _TAKE_LOCKS_GUARD:
        return _TAKE_LOCKS.setdefault(identity, threading.Lock())


def _take_cache_path(scenario: CombatScenario) -> Path:
    return TAKE_CACHE / scenario.cache_name() / "evidence.json"


def _evidence_is_current(doc: dict, scenario: CombatScenario, binary: Path | None) -> bool:
    if doc.get("scenario_id") != scenario.identity() or doc.get("scenario") != scenario.document():
        return False
    if doc.get("stale"):
        return False
    if doc.get("source_identity") != _repository_identity():
        return False
    if binary is None:
        return False
    stamp = ((doc.get("generator") or {}).get("mtime"))
    return isinstance(stamp, (int, float)) and stamp >= binary.stat().st_mtime


def _bulk_take(scenario: CombatScenario) -> tuple[dict, dict, Path] | None:
    """Reuse a full-corpus take only when it is the exact same scenario."""
    path = DATA / "takes" / "takes.json"
    if not path.exists():
        return None
    try:
        doc = json.loads(path.read_text())
        rows = doc.get("takes", doc) if isinstance(doc, dict) else doc
        for take in rows or []:
            try:
                candidate = scenario_from_take(take)
            except ValueError:
                continue
            if candidate.document() == scenario.document():
                return take, doc if isinstance(doc, dict) else {}, path
    except (OSError, ValueError, TypeError):
        return None
    return None


def _take_evidence_document(
    scenario: CombatScenario,
    take: dict,
    source_doc: dict,
    *,
    generator: Path | None,
    cache_source: str,
    stale: str | None = None,
    output: dict | None = None,
) -> dict:
    sim_hz = float(source_doc.get("sim_hz") or 60.0)
    evidence = {
        "schema": "ambition.inspector_evidence.v1",
        "scenario": scenario.document(),
        "scenario_id": scenario.identity(),
        "source_identity": _repository_identity(),
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "cache_source": cache_source,
        "generator": {
            "path": str(generator) if generator else None,
            "built": _built_at(generator) if generator else None,
            "mtime": generator.stat().st_mtime if generator else None,
        },
        "take_schema": source_doc.get("schema"),
        "observation_schema": source_doc.get("observation_schema"),
        "sim_hz": sim_hz,
        "take": take,
        "report": _report_for_take(take, sim_hz, source_doc),
    }
    if stale:
        evidence["stale"] = stale
    if output:
        evidence["output"] = output
    return evidence


def take_evidence(scenario: CombatScenario, force: bool = False) -> tuple[int, dict]:
    """Load or generate exactly one runtime take for one canonical scenario."""
    binary = find_binary("moveset_takes")
    cache_path = _take_cache_path(scenario)
    identity = scenario.identity()
    with _scenario_lock(identity):
        cached = None
        if cache_path.exists():
            try:
                cached = json.loads(cache_path.read_text())
            except (OSError, ValueError):
                cached = None
        if not force and cached is not None and _evidence_is_current(cached, scenario, binary):
            cached["cache_hit"] = True
            return 200, cached

        bulk = _bulk_take(scenario)
        if not force and bulk is not None:
            take, source_doc, source_path = bulk
            bulk_generator = source_doc.get("generator") or {}
            bulk_mtime = bulk_generator.get("mtime")
            bulk_current = (
                binary is not None
                and source_doc.get("source_identity") == _repository_identity()
                and isinstance(bulk_mtime, (int, float))
                and bulk_mtime >= binary.stat().st_mtime
            )
            if binary is None or bulk_current:
                stale = None if bulk_current else (
                    "moveset_takes is not built; bulk corpus source provenance cannot be revalidated"
                )
                evidence = _take_evidence_document(
                    scenario,
                    take,
                    source_doc,
                    generator=binary,
                    cache_source="bulk_corpus",
                    stale=stale,
                )
                cache_path.parent.mkdir(parents=True, exist_ok=True)
                cache_path.write_text(json.dumps(evidence))
                return 200, evidence

        if binary is None:
            if not force and cached is not None and cached.get("scenario") == scenario.document():
                cached["stale"] = "moveset_takes is not built; showing the last matching take"
                cached["cache_hit"] = True
                return 200, cached
            return 503, {
                "available": False,
                "state": "error",
                "reason": "moveset_takes is not built",
                "hint": "cargo build -p ambition_app_tools --bin moveset_takes",
                "scenario": scenario.document(),
                "looked_in": [str(p) for p in binary_candidates("moveset_takes")],
            }

        TAKE_CACHE.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="generate-", dir=TAKE_CACHE) as temp:
            out = Path(temp) / "take.json"
            command = [
                str(binary),
                "--characters", scenario.subject,
                "--verbs", scenario.verb,
                "--target", scenario.target,
                "--target-behavior", scenario.target_behavior,
                "--out", str(out),
            ]
            if scenario.spacing is not None:
                command += ["--spacing", str(scenario.spacing)]
            if scenario.chain is not None:
                command += ["--chain", scenario.chain]
                if scenario.chain_at is not None:
                    command += ["--chain-at", str(scenario.chain_at)]
            try:
                result = subprocess.run(
                    command,
                    cwd=str(REPO),
                    capture_output=True,
                    text=True,
                    timeout=300,
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                return 503, {
                    "available": False,
                    "state": "error",
                    "reason": f"moveset_takes did not run: {error}",
                    "command": command,
                    "scenario": scenario.document(),
                }
            output = {
                "returncode": result.returncode,
                "stdout": result.stdout[-12000:],
                "stderr": result.stderr[-12000:],
            }
            if result.returncode != 0 or not out.exists():
                return 503, {
                    "available": False,
                    "state": "error",
                    "reason": f"moveset_takes exited {result.returncode}",
                    "command": command,
                    "output": output,
                    "scenario": scenario.document(),
                }
            try:
                source_doc = json.loads(out.read_text())
                rows = source_doc.get("takes") or []
                take = next(
                    t for t in rows if scenario_from_take(t).document() == scenario.document()
                )
            except (OSError, ValueError, TypeError, StopIteration) as error:
                return 503, {
                    "available": False,
                    "state": "error",
                    "reason": f"moveset_takes returned no matching scenario: {error}",
                    "command": command,
                    "output": output,
                    "scenario": scenario.document(),
                }

            evidence = _take_evidence_document(
                scenario,
                take,
                source_doc,
                generator=binary,
                cache_source="scenario_cache",
                output=output,
            )
            cache_path.parent.mkdir(parents=True, exist_ok=True)
            cache_path.write_text(json.dumps(evidence))
            return 200, evidence


def render_animation(
    character: str,
    verb: str,
    frames: int,
    stride: int,
    target: str | None = None,
    spacing: float | None = None,
    target_behavior: str | None = None,
    force: bool = False,
    *,
    _scenario: CombatScenario | None = None,
) -> tuple[int, dict]:
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
    if _scenario is None:
        # Legacy/direct-call boundary: normalize once into the same canonical
        # scenario object used by the HTTP API. The normal inspector path passes
        # `_scenario` and therefore never reconstructs scenario semantics here.
        try:
            scenario = CombatScenario.from_mapping(
                {
                    "subject": character,
                    "target": target or character,
                    "target_behavior": target_behavior or "passive",
                    "verb": verb,
                    "spacing": spacing,
                }
            )
        except ValueError as error:
            return 400, {"error": str(error)}
    else:
        scenario = _scenario

    safe = scenario.subject
    safe_verb = scenario.verb
    safe_target = scenario.target
    target_behavior = scenario.target_behavior
    spacing = scenario.spacing

    # ⭐ CACHED BY WHAT WAS ASKED FOR — character AND verb AND shape. Caching by
    # character alone served the up-B's frames for a jab.
    #
    # ⛔⛔ AND BY THE SCENARIO, for the same reason. A render taken from across
    # the stage and a take recorded at 40px are two different fights, and the
    # browser shows them SIDE BY SIDE — so a cache key that ignored the target
    # and the spacing would serve one as evidence for the other.
    out_dir = RENDERS / scenario.cache_name()
    manifest = out_dir / "manifest.json"
    binary = find_renderer()

    # ⛔⛔ A CACHE THAT OUTLIVES THE BINARY THAT FILLED IT IS A STALE ANSWER
    # WEARING A CURRENT ONE'S CLOTHES. This accepted any manifest with enough
    # frames at the right stride, so a render taken before an hour of engine
    # changes was served as this build's picture of the move — the exact failure
    # the provenance stamp exists to make visible, silently defeated one layer
    # above it.
    cached = None
    if manifest.exists() and not force:
        try:
            have = json.loads(manifest.read_text())
            # ⛔⛔ AND THE MANIFEST MUST DESCRIBE THE FIGHT THAT WAS ASKED FOR.
            # The key is derived from the request, so a mismatch here means a
            # directory holding somebody else's render — a truncated key that
            # once collided, a hand-edited path, a renderer whose flags moved.
            # Checking the frames and the binary but not the SCENARIO is what let
            # a stale key serve one experiment as evidence for another.
            recorded_scenario = have.get("scenario")
            if recorded_scenario is not None:
                same_scenario = recorded_scenario == scenario.document()
            else:
                asked = {
                    "target": safe_target,
                    "target_behavior": target_behavior,
                    "requested_spacing": spacing,
                }
                drawn = {
                    "target": have.get("target"),
                    "target_behavior": have.get("target_behavior"),
                    "requested_spacing": have.get("requested_spacing"),
                }
                same_scenario = all(
                    _same_scenario_value(asked[k], drawn[k]) for k in asked
                )
            if (
                same_scenario
                and have.get("source_identity") == _repository_identity()
                and have.get("frames", 0) >= frames
                and have.get("stride") == stride
            ):
                cached = have
        except (OSError, ValueError):
            cached = None
    if cached is not None and binary is not None:
        drawn_by = cached.get("renderer_mtime")
        if drawn_by is not None and drawn_by >= binary.stat().st_mtime:
            return 200, cached
        cached["stale"] = (
            f"drawn by an older {binary.name}; re-rendering with the one built "
            f"{_built_at(binary)}"
        )

    if binary is None:
        # ⭐ A CACHED PICTURE IS BETTER THAN NO PICTURE, **SAID PLAINLY**. Nothing
        # here builds, so "there is no renderer" is a state a reader can be in
        # for a long time; serving the last render unlabelled would make it look
        # like this build's.
        if cached is not None:
            cached = dict(cached)
            cached["cached_only"] = True
            cached["reason"] = (
                "moveset_render is not built — this is the last render, drawn by "
                f"{cached.get('renderer_built') or 'an unknown build'}"
            )
            return 200, cached
        return 503, {
            "available": False,
            "reason": "moveset_render is not built",
            "hint": "cargo build -p ambition_app_tools --bin moveset_render",
            "looked_in": [str(p) for p in renderer_candidates()],
        }

    out_dir.mkdir(parents=True, exist_ok=True)
    # ⛔⛔ THE OLD MANIFEST GOES FIRST. This re-rendered and then read whatever
    # manifest was on disk, so a renderer that failed to write one left the
    # PREVIOUS render's document to be read back and served as the new one —
    # returning 200 with a stale picture through the very branch added to refuse
    # stale pictures. What is on disk after the run must be what the run wrote.
    manifest.unlink(missing_ok=True)
    try:
        result = subprocess.run(
            [
                str(binary),
                "--character", safe,
                "--verb", safe_verb,
                "--out", str(out_dir),
                "--frames", str(frames),
                "--stride", str(stride),
            ]
            # The scenario, so the picture is of the fight the take recorded.
            + ["--target", safe_target]
            + (["--spacing", str(spacing)] if spacing is not None else [])
            # ⛔ NOT GATED ON `safe_target`. `moveset_render`'s missing-target
            # default is a MIRROR opponent, not "no opponent" — so a mirror
            # scenario that omitted the target also silently dropped its
            # behaviour and rendered a passive stand-in for a CPU take.
            + (["--target-behavior", target_behavior] if target_behavior else []),
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
        scenario=scenario.document(),
        scenario_id=scenario.identity(),
        source_identity=_repository_identity(),
        renderer_path=str(binary),
        renderer_built=_built_at(binary),
        # ⭐ THE RAW STAMP BESIDE THE READABLE ONE, so the next request can ask
        # whether this picture predates the binary on disk. A minute-resolution
        # string cannot answer that.
        renderer_mtime=binary.stat().st_mtime,
        urls=[f"/render/{out_dir.name}/{shot['file']}" for shot in document.get("shots", [])],
    )
    # ⛔⛔ A MISMATCH IS REPORTED, NEVER CACHED UNDER THE REQUESTED NAME. A press
    # is a REQUEST; the engine decides what comes out. Serving another move's
    # animation as this one's is the worst thing a reference tool can do.
    # ⛔⛔ A MISMATCH IS THE INTENDED MOVE NOT APPEARING, not "no move appeared".
    # This read `observed_moves` and then ignored it, declaring success whenever
    # ANY move played — so a verb that resolved to a different move would have
    # been cached and served under the name that was asked for.
    document["mismatch"] = not document.get("reached_intended_move")
    if document["mismatch"]:
        intended = document.get("intended_move")
        observed = document.get("observed_moves") or []
        document["reason"] = (
            f"`{safe_verb}` is bound to {intended!r} and the engine played {observed!r}"
            if intended
            else f"`{safe_verb}` is not bound on {safe}; the engine played {observed!r}"
        )
    manifest.write_text(json.dumps(document))
    return 200, document


def render_scenario(
    scenario: CombatScenario,
    frames: int,
    stride: int,
    *,
    force: bool = False,
) -> tuple[int, dict]:
    """Render the exact canonical scenario, or state why that is impossible."""
    if not scenario.renderable:
        return 422, {
            "available": False,
            "state": "unsupported",
            "reason": "GPU rendering is not available for this chain scenario",
            "scenario": scenario.document(),
            "scenario_id": scenario.identity(),
        }
    return render_animation(
        scenario.subject,
        scenario.verb,
        frames,
        stride,
        scenario.target,
        scenario.spacing,
        scenario.target_behavior,
        force=force,
        _scenario=scenario,
    )


def render_frames_for_horizon(through_tick: int, stride: int) -> int:
    """Number of samples needed to cover every sample point through a take horizon."""
    if through_tick < 0 or stride < 1:
        raise ValueError("through_tick must be non-negative and stride positive")
    return through_tick // stride + 1


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

    # ⛔⛔ THE NEWEST ONE, NOT THE FIRST ONE. This took `release` before `debug`,
    # so a release binary from last week outranked a debug binary built a minute
    # ago — while the build hint beside it told the reader to run an ordinary
    # debug `cargo build`. The tool answered with the binary its own advice did
    # not produce.
    binaries = {}
    for name in ("moveset_export", "moveset_takes", "moveset_render"):
        found = find_binary(name)
        binaries[name] = {
            "found": found is not None,
            "path": str(found) if found else None,
            "built": _built_at(found) if found else None,
            "build_command": f"cargo build -p ambition_app_tools --bin {name}",
            "looked_in": [str(p) for p in binary_candidates(name)],
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
            # ⭐ THE OTHER HALF OF THE INTERACTION, counted. A recording with
            # strikes and no damageable geometry cannot answer why an attack
            # missed, and looks identical to one that can.
            hurt = roles = 0
            for take in rows or []:
                for frame in take.get("frames") or []:
                    for body in frame.get("bodies") or []:
                        bodies += 1
                        art += 1 if body.get("art") else 0
                        hurt += 1 if body.get("hurtbox_source") else 0
                        roles += 1 if body.get("role") else 0
                    for hit in frame.get("hitboxes") or []:
                        boxes += 1
                        shapes += 1 if hit.get("shape") else 0
            takes_meta.update(
                takes=len(rows or []),
                bodies=bodies,
                with_art=art,
                with_hurtboxes=hurt,
                with_role=roles,
                hitboxes=boxes,
                with_shape=shapes,
                schema=doc.get("schema") if isinstance(doc, dict) else None,
            )
            stale = []
            if bodies and not art:
                stale.append("no sprite art (bodies carry no `art`)")
            if boxes and not shapes:
                stale.append("no hitbox geometry (strikes carry no `shape`)")
            if bodies and not hurt:
                stale.append(
                    "no damageable geometry (bodies carry no `hurtbox_source`)"
                )
            if bodies and not roles:
                stale.append("no scenario roles (bodies carry no `role`)")
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
        "take_cache_dir": str(TAKE_CACHE),
        "cached_scenarios": sum(1 for _ in TAKE_CACHE.glob("*/evidence.json")) if TAKE_CACHE.exists() else 0,
        "binaries": binaries,
        # ⛔⛔ WHY THE ENGINE RENDER IS OR IS NOT AVAILABLE, BEFORE ANYBODY ASKS
        # FOR ONE. `/api/render` answers 503 only after composing the whole game,
        # which is minutes late and names the renderer whatever went wrong — a
        # 2026-08-29 review read one such message, concluded this machine had no
        # Vulkan adapter, and recommended a package that was already installed.
        # This says what the loader and the ICD directory actually hold.
        "render_capability": _render_capability(),
    }


def _render_capability() -> dict:
    """The offscreen-render verdict, or why it could not be reached.

    ⛔ FAILURE IS A FIELD, NOT AN EXCEPTION. The Status page is what somebody
    opens when the render is missing; a doctor that raises takes the page down
    with it.
    """
    try:
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "render_capability_doctor", REPO / "scripts/render_capability_doctor.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module.report()
    except Exception as error:  # noqa: BLE001 - reported, never raised
        return {"offscreen_capture": "unknown", "error": str(error)}


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


def _scenario_from_query(query: dict[str, list[str]]) -> CombatScenario:
    def one(name: str):
        return (query.get(name) or [None])[0]

    raw = {
        "subject": one("subject") or one("character"),
        "target": one("target"),
        "target_behavior": one("target_behavior"),
        "verb": one("verb"),
        "spacing": one("spacing"),
        "chain": one("chain"),
        "chain_at": one("chain_at"),
        "hold_policy": one("hold_policy"),
    }
    return CombatScenario.from_mapping(raw)


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
        if parsed.path == "/api/take":
            query = parse_qs(parsed.query)
            try:
                scenario = _scenario_from_query(query)
            except ValueError as error:
                return self._json(400, {"error": str(error)})
            force = (query.get("force") or [""])[0].lower() in {"1", "true", "yes"}
            return self._json(*take_evidence(scenario, force=force))
        if parsed.path == "/api/render":
            query = parse_qs(parsed.query)
            try:
                scenario = _scenario_from_query(query)
                stride = max(1, min(int((query.get("stride") or ["2"])[0]), 30))
                # Prefer an action-tick horizon because it states the actual
                # synchronization contract. `frames` remains accepted for the
                # CLI/browser checks that predate it.
                through_raw = (query.get("through_tick") or [""])[0]
                if through_raw:
                    through_tick = max(0, min(int(through_raw), 5000))
                    frames = render_frames_for_horizon(through_tick, stride)
                else:
                    through_tick = None
                    frames = max(1, min(int((query.get("frames") or ["24"])[0]), 2500))
                force = (query.get("force") or [""])[0].lower() in {"1", "true", "yes"}
            except ValueError as error:
                return self._json(400, {"error": str(error)})
            status, doc = render_scenario(scenario, frames, stride, force=force)
            if isinstance(doc, dict):
                doc.setdefault(
                    "requested_through_tick",
                    through_tick if through_tick is not None else (frames - 1) * stride,
                )
                doc.setdefault("last_sampled_action_tick", (frames - 1) * stride)
            return self._json(status, doc)
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
        length = int(self.headers.get("content-length") or 0)
        try:
            payload = json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError as err:
            return self._json(400, {"error": f"bad json: {err}"})

        if parsed.path == "/api/take":
            try:
                scenario = CombatScenario.from_mapping(payload.get("scenario") or payload)
            except ValueError as error:
                return self._json(400, {"error": str(error)})
            return self._json(*take_evidence(scenario, force=bool(payload.get("force"))))

        if parsed.path == "/api/render":
            try:
                scenario = CombatScenario.from_mapping(payload.get("scenario") or payload)
                stride = max(1, min(int(payload.get("stride", 2)), 30))
                through_tick = payload.get("through_tick")
                if through_tick is not None:
                    through_tick = max(0, min(int(through_tick), 5000))
                    frames = render_frames_for_horizon(through_tick, stride)
                else:
                    frames = max(1, min(int(payload.get("frames", 24)), 2500))
            except (TypeError, ValueError) as error:
                return self._json(400, {"error": str(error)})
            status, doc = render_scenario(
                scenario,
                frames,
                stride,
                force=bool(payload.get("force")),
            )
            if isinstance(doc, dict):
                doc.setdefault(
                    "requested_through_tick",
                    through_tick if through_tick is not None else (frames - 1) * stride,
                )
                doc.setdefault("last_sampled_action_tick", (frames - 1) * stride)
            return self._json(status, doc)

        if parsed.path != "/api/review":
            return self._json(404, {"error": "no such endpoint"})

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
