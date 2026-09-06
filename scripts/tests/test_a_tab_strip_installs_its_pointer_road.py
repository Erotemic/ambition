"""A surface with more than one tab must install the tab pointer road.

⛔⛔ THE BUG THIS EXISTS FOR, reported by Jon 2026-09-06: "in the title screen there
is no way for me to select the settings menu. I can't click, tap, nothing."

The shell's title screen built a two-tab strip and installed
`install_bevy_ui_menu_actions` (its ROWS) but never `install_bevy_ui_menu_tabs`. So
`publish_bevy_ui_menu_tabs` — the system that turns a tab press into
`MenuTabActivated` — was not registered on that screen at all. The renderer drew
each tab as a real `Button` and nothing listened; the tab was reachable only by
Escape/Start.

⭐ EVERY TEST WAS GREEN THROUGH IT, and that is the reason this guard is
structural rather than behavioural. The suite drove `MenuControlFrame` edges — the
keyboard/controller road — which was never broken. **A surface can lose an entire
INPUT DEVICE with its suite green, because the devices do not share a seam.** No
amount of testing the gesture road can see that the pointer road is absent.

⚠ ONE TAB IS A TITLE, NOT NAVIGATION, and that is the discriminator this encodes.
`pause_menu.rs` builds a single-tab strip (`"Paused"`) and correctly installs
nothing: there is nowhere to switch to. A guard without this exemption would
demand a road for a label and teach its next reader to silence it.
"""

from __future__ import annotations

import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[2]
_TAB_SPEC = re.compile(r"BevyUiMenuTabSpec::new\b")
_INSTALL = re.compile(r"install_bevy_ui_menu_tabs\s*\(")
# The renderer that DEFINES the installer is not a consumer of it.
_DEFINES = "crates/ambition_menu/src/render/bevy_ui/mod.rs"


def _rust_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    return out


def _crate_root(path: str) -> str:
    """The crate a file belongs to — an install anywhere in it serves the App."""
    parts = pathlib.Path(path).parts
    for i, p in enumerate(parts):
        if p == "src":
            return "/".join(parts[:i])
    return str(pathlib.Path(path).parent)


def test_a_multi_tab_strip_installs_the_pointer_road():
    files = _rust_files()
    assert len(files) > 500, (
        f"only {len(files)} rust files listed; the corpus moved and this guard is "
        f"reading almost nothing"
    )

    installs_by_crate: dict[str, bool] = {}
    for f in files:
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        if _INSTALL.search(text) and f != _DEFINES:
            installs_by_crate[_crate_root(f)] = True

    multi_tab = []
    single_tab = []
    for f in files:
        if f == _DEFINES:
            continue
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        n = len(_TAB_SPEC.findall(text))
        if n >= 2:
            multi_tab.append(f)
        elif n == 1:
            single_tab.append(f)

    # ⚠ ANTI-VACUITY on BOTH populations. Zero multi-tab files would pass the
    # assertion below while proving nothing, and it is the exact state a renamed
    # type would produce.
    assert multi_tab, (
        "no file builds a multi-tab strip; `BevyUiMenuTabSpec::new` was renamed or "
        "moved and this guard now checks nothing"
    )
    assert single_tab, (
        "no single-tab file found; the one-tab exemption below is untested by the "
        "real corpus and may be silently wrong"
    )

    missing = [f for f in multi_tab if not installs_by_crate.get(_crate_root(f))]
    assert not missing, (
        "these surfaces draw MORE THAN ONE tab as buttons but their crate never "
        "calls `install_bevy_ui_menu_tabs`, so a click or tap on a tab reaches no "
        f"system and the strip is navigable only by gesture: {missing}"
    )
