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
