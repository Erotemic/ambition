"""A policy `skip_paths` entry must name a path that exists.

## The drift this closes

`engine.sim-no-presentation-import` exempted four paths. THREE of them —
`dialog/ui.rs`, `runtime/setup.rs`, `runtime/reset/` — named files that no longer
exist, killed by the crate reorganisation and never removed. They exempted
nothing, and they misrepresented the rule: they implied three gameplay files
legitimately import `crate::presentation`, when none does.

The rule still passed the whole time, which is why nobody looked. A dead
exemption is invisible until the path is RE-CREATED, at which point the new file
is silently exempt from a rule it was never argued out of.

⚠ **this checks only the "names nothing" class, deliberately.** A skip_path that
exists but currently holds no forbidden token is a different thing and may be
entirely legitimate — `presentation/` under that same rule is kept for exactly
that reason, because presentation code referring to `crate::presentation` is the
one case that would be allowed. Flagging those would turn a real distinction into
noise and get the check waived.

⚠ and it is scoped to the WORKSPACE POLICY config, not to code. The bar in
AGENTS.md is a concrete recurring failure that types cannot catch: three dead
entries were found on the first run, they came from a rename, and the repository
has more renaming ahead (queue S30/S31).
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
POLICY_DIR = REPO / "tests/ambition_workspace_policy/policies"

# `skip_paths = [ "a", "b" ]` — the LIST body only.
SKIP_BLOCK = re.compile(r"^\s*skip_paths\s*=\s*\[(.*?)\]", re.S | re.M)
POLICY_ID = re.compile(r'^\s*id\s*=\s*"([^"]+)"', re.M)


def _strip_comments(text: str) -> str:
    """Drop `#` comments before parsing.

    ⛔ the first version of this parser did not, and a quoted phrase inside a
    comment in one rule's `skip_paths` block was read as a path — an invented
    finding from the checker's own input handling.
    """
    return "\n".join(line.split("#", 1)[0] for line in text.splitlines())


def _tracked() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout
    return out.split()


def test_every_policy_exemption_names_something_that_exists():
    tracked = _tracked()
    dead: list[str] = []
    for policy_file in sorted(POLICY_DIR.glob("*.toml")):
        text = _strip_comments(policy_file.read_text(encoding="utf-8"))
        for chunk in text.split("[[policy]]")[1:]:
            ident = POLICY_ID.search(chunk)
            block = SKIP_BLOCK.search(chunk)
            if not (ident and block):
                continue
            for path in re.findall(r'"([^"]+)"', block.group(1)):
                if not any(path in candidate for candidate in tracked):
                    dead.append(
                        f"{policy_file.name}: {ident.group(1)} exempts {path!r}, "
                        "which matches no tracked file"
                    )

    assert not dead, (
        "a workspace policy exempts a path that does not exist:\n  "
        + "\n  ".join(dead)
        + "\n\nThe rule passes either way, which is why this rots unnoticed. Delete "
        "the entry — and if the path was RENAMED rather than deleted, move the "
        "exemption to the new path in the same commit, because the argument for "
        "exempting it did not go away just because the file moved."
    )
