"""The SDK's Cargo patch guidance must match the workspace manifest.

When the workspace carries a `bevy_ggrs` patch, the SDK guidance must name the
same revision required by an external consumer. When that patch disappears, the
corresponding SDK workaround must disappear too."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
)

_PIN = re.compile(r'bevy_ggrs\s*=\s*\{[^}]*\brev\s*=\s*"([0-9a-f]{7,40})"', re.DOTALL)


def _pinned_rev(text: str) -> str | None:
    match = _PIN.search(text)
    return match.group(1) if match else None


def test_the_sdk_pins_the_same_bevy_ggrs_revision_as_the_workspace():
    workspace = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    readme = (REPO / "docs" / "sdk" / "README.md").read_text(encoding="utf-8")

    workspace_rev = _pinned_rev(workspace)
    readme_rev = _pinned_rev(readme)

    if workspace_rev is None:
        # The happy ending: the fork retired. Then the advice must retire too,
        # or the SDK is telling strangers to add a patch entry for a dependency
        # that no longer needs one.
        assert readme_rev is None, (
            "the workspace no longer patches `bevy_ggrs`, but docs/sdk/README.md "
            f"still tells a consumer to pin rev {readme_rev}. Delete that section "
            "and this test together — the leak it documents is closed."
        )
        return

    assert readme_rev is not None, (
        "the workspace patches `bevy_ggrs` to a git fork, and docs/sdk/README.md "
        "does not say so. A third party selecting rollback gets `cannot find "
        "type GgrsFrameTiming` with no path back to the cause — the exact failure "
        "the 2026-07-30 blind agent run ranked as this engine's highest-cost leak."
    )
    assert readme_rev == workspace_rev, (
        f"docs/sdk/README.md tells consumers to pin bevy_ggrs rev {readme_rev}, "
        f"but the workspace pins {workspace_rev}. A consumer following the SDK "
        "would build against a different fork than the engine is developed "
        "against. Update the README in the commit that moves the pin."
    )


# ⛔⛔ THE ROLLBACK CORE'S OWN REVISION IS PINNED BY `Cargo.lock` ALONE, and
# nothing said so until this test. The manifest pins `bevy_ggrs` by `rev`, but
# THAT revision takes `ggrs` from its git main rather than the v0.13.0 release,
# so the resolved commit of the deterministic-rollback core itself is recorded
# only in the lockfile. A `cargo update` moves it with no diff a reviewer would
# read as a rollback change.
_GGRS_GIT_MAIN = "e97e3d2416cc68af2d2876d41180d950c2939b6e"

_LOCK_GGRS = re.compile(
    r'^name = "ggrs"\n^version = "([^"]+)"\n^source = "([^"]+)"',
    re.MULTILINE,
)


def test_the_resolved_ggrs_revision_is_the_one_this_engine_was_measured_against():
    """A `cargo update` must not move the rollback core silently.

    ⭐ IT IS CONDITIONAL ON THE FORK, like its sibling above: when a released
    `bevy_ggrs` contains gschup/bevy_ggrs#134 and the workspace patch goes away,
    `ggrs` comes from crates.io by version and this test retires with it. Delete
    it in that commit rather than loosening it.

    ⛔ WHEN A MOVE IS DELIBERATE, UPDATE `_GGRS_GIT_MAIN` IN THE SAME COMMIT and
    say in the message what was re-measured. The determinism contract (ADR 0023)
    is a claim about a specific implementation of rollback, not about the name of
    one — a desync ratchet taken against a different commit is a baseline for
    software this engine is not running.
    """
    workspace = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    if _pinned_rev(workspace) is None:
        return

    lock = (REPO / "Cargo.lock").read_text(encoding="utf-8")
    match = _LOCK_GGRS.search(lock)
    assert match is not None, (
        "Cargo.lock has no `ggrs` package with a `source`, which either means the "
        "rollback core left the tree or it now resolves from a path/vendor "
        "override. Both are changes to what determinism means here."
    )
    version, source = match.group(1), match.group(2)
    assert source.startswith("git+"), (
        f"`ggrs` {version} now resolves from `{source}` rather than git. The "
        "patched `bevy_ggrs` revision depends on ggrs's git main, so a crates.io "
        "resolution means a DIFFERENT implementation of the rollback core than "
        "the one every desync ratchet in this repository was taken against."
    )
    resolved = source.partition("#")[2]
    assert resolved == _GGRS_GIT_MAIN, (
        f"Cargo.lock resolves `ggrs` to {resolved or '(no commit)'}, but this "
        f"engine's rollback numbers were taken against {_GGRS_GIT_MAIN}. Nothing "
        "in the manifest pins this -- `bevy_ggrs`'s pinned rev takes ggrs from "
        "git MAIN -- so a `cargo update` moves the deterministic core with no "
        "diff that reads as a rollback change. If the move is deliberate, update "
        "`_GGRS_GIT_MAIN` here and say what was re-measured."
    )
