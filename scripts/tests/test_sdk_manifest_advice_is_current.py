"""The SDK's `[patch.crates-io]` advice must match the workspace it describes.

`docs/sdk/README.md` tells a third party to copy a `bevy_ggrs` git pin out of the
workspace root, because Cargo patch tables do not cross a workspace boundary and
without it a consumer that selects the rollback backend does not compile. The 2026-07-30 blind
agent run found that gap the expensive way: it hit `cannot find type
GgrsFrameTiming` before it could ask a single API question.

That paragraph is now the first thing in the SDK, and it contains a **pinned
revision** — which makes it the kind of documentation that rots silently. The
day somebody bumps the fork, the README keeps confidently telling strangers to
pin a rev that no longer matches, and the failure looks exactly like the one the
paragraph exists to prevent.

⚠ **This is here because the underlying leak is DEFERRED, not fixed.** Jon
decided 2026-07-30 to wait for the accessor to ship in a released `bevy_ggrs`
(see `docs/planning/maintainer-decisions.md`). "Come back to it later" is only
safe if later arrives to a document that is still true, and the deferral could
last as long as an upstream release cycle.

It is a text check, which this repo is otherwise sceptical of — see AGENTS.md,
"avoid bullshit guardrails". It earns its place because the subject genuinely IS
a string that must equal another string, in two files that nothing else relates,
and because being wrong is not a cosmetic docs issue: it is a consumer who
cannot build. There is no type, API boundary, or behavioural test that can
express "the README's rev equals the manifest's rev".

Delete this the day the fork does — when the workspace has no `bevy_ggrs` patch
entry, the README should have no paragraph about one, and the check below says
so rather than passing vacuously.
"""

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
