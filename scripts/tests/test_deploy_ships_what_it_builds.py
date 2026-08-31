"""The Steam Deck deploy must ship the binary it just built.

The script built `ambition_game_bin` and then rsync'd, launched and verified
`ambition_platformer2d_actor_monolith` — a name that stopped being an executable
when that crate became a library. Nothing failed on the machine that ran it,
because an old `target/release` still held a file of the old name; a clean tree
would have failed at rsync, after the whole asset compose and release build.

⭐ THE CHECK IS "ONE NAME, AND CARGO DECLARES IT" rather than a spelling match
against a constant here. A hard-coded expected name in this file would have to be
edited in lockstep with a rename, which is exactly the lockstep that failed.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DEPLOY = REPO / "deploy_to_steamdeck.sh"
APP_MANIFEST = REPO / "game/ambition_app/Cargo.toml"


def _script() -> str:
    return DEPLOY.read_text(encoding="utf8")


def _code_lines() -> list[str]:
    """Script lines with comments dropped — the comment names the OLD binary."""
    return [
        line
        for line in _script().splitlines()
        if not line.lstrip().startswith("#")
    ]


def _built_bin() -> str:
    # ⛔ ACROSS LINES. The invocation is a shell continuation — `cargo build \`
    # then `-p`, `--bin`, `--release` on their own lines — and `.` does not match
    # a newline without DOTALL, so this guard failed with "no recognisable cargo
    # build --bin" while the script plainly had one. A guard that goes red for
    # formatting is a guard people learn to ignore.
    match = re.search(
        r'cargo build .*?--bin\s+"?\$?\{?(\w+)\}?"?',
        "\n".join(_code_lines()),
        re.S,
    )
    assert match, "deploy_to_steamdeck.sh no longer has a recognisable `cargo build --bin`"
    return match.group(1)


def _declared_bins() -> set[str]:
    text = APP_MANIFEST.read_text(encoding="utf8")
    return set(re.findall(r'\[\[bin\]\]\s*\nname\s*=\s*"([^"]+)"', text))


def test_the_deployed_binary_is_one_ambition_app_declares() -> None:
    built = _built_bin()
    if built == "BIN":  # the script builds through its own variable
        assign = re.search(r'^BIN="\$\{BIN:-(\w+)\}"', _script(), re.MULTILINE)
        assert assign, "BIN is used but never given a default"
        built = assign.group(1)

    declared = _declared_bins()
    assert declared, "ambition_app declares no [[bin]] at all"
    assert built in declared, (
        f"deploy builds `{built}`, which ambition_app does not declare as a binary; "
        f"declared: {sorted(declared)}"
    )


def test_every_deploy_path_names_the_same_executable() -> None:
    """rsync source, launcher `exec`, and the remote `test -x` must agree."""
    code = "\n".join(_code_lines())

    rsync = re.search(r'rsync[^\n]*\\\n\s*"?([^\s"]*target/release/[^\s"]+)"?', code)
    assert rsync, "no `target/release/...` rsync source found"
    launcher = re.search(r'exec\s+"[^"]*APPDIR\}?/([^"]+)"', code)
    assert launcher, "the generated launcher has no `exec` of the app"
    verify = re.search(r'test -x\s+"[^"]*APPDIR\}?/([^"\n]+)"', code)
    assert verify, "the remote check no longer asserts the executable exists"

    shipped = Path(rsync.group(1)).name
    names = {shipped, launcher.group(1), verify.group(1)}
    assert len(names) == 1, (
        "the deploy script builds, ships, launches or verifies more than one "
        f"executable name: {sorted(names)}"
    )
