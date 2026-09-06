"""The inspector's `/data/` route may not serve files outside its bundle.

⛔⛔ IT COULD. `translate_path` joined the raw remainder of the URL onto the data
directory, so a request carrying enough `../` walked out and the stdlib file
server happily read whatever the process could (GPT 5.6, 2026-08-27). The
default host is `127.0.0.1`, which limits the blast radius — but `--host` is a
deliberate CLI option, and on a bound interface this is file disclosure.

⚠ THE ARMS STRADDLE THE FIX. A test that only proves traversal is refused agrees
with a server that refuses everything; the legitimate paths are asserted beside
it.
"""

from __future__ import annotations

import pathlib
import sys
from urllib.parse import urlparse

REPO = pathlib.Path(__file__).resolve().parents[2]
INSPECTOR = REPO / "tools" / "ambition_moveset_inspector"


def _translate(path: str) -> str:
    """Run the server's own `translate_path` without opening a socket."""
    sys.path.insert(0, str(INSPECTOR))
    try:
        from ambition_moveset_inspector.server import InspectorHandler
    finally:
        sys.path.pop(0)
    return InspectorHandler.translate_path(object.__new__(InspectorHandler), path)


def _data_root() -> pathlib.Path:
    sys.path.insert(0, str(INSPECTOR))
    try:
        from ambition_moveset_inspector.server import DATA
    finally:
        sys.path.pop(0)
    return DATA.resolve()


def test_a_data_request_cannot_escape_the_bundle():
    root = _data_root()
    escapes = [
        "/data/../../../../etc/passwd",
        "/data/../../Cargo.toml",
        "/data/takes/../../../AGENTS.md",
        "/data/%2e%2e/%2e%2e/Cargo.toml",
    ]
    for probe in escapes:
        served = pathlib.Path(_translate(probe)).resolve()
        assert served.is_relative_to(root), (
            f"`{probe}` was translated to {served}, which is outside {root} — "
            "the inspector would serve a file from the repository to anyone who "
            "can reach the port"
        )


def test_an_ordinary_data_request_still_resolves_into_the_bundle():
    root = _data_root()
    for probe, tail in [
        ("/data/takes/takes.json", "takes/takes.json"),
        ("/data/./movesets.json", "movesets.json"),
    ]:
        served = pathlib.Path(_translate(probe)).resolve()
        assert served == (root / tail).resolve(), (
            f"`{probe}` no longer reaches its file ({served}) — the containment "
            "check refuses the traversal AND the bundle, which serves nothing"
        )
