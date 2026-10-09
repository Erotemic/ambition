#!/usr/bin/env python3
"""Install the pre-push hook that runs `scripts/required_checks.py` (TEST-LANES).

    python3 scripts/install_pre_push_hook.py            # install or refresh
    python3 scripts/install_pre_push_hook.py --check    # exit 1 when it is not installed

The hook lives in the common git directory, so every worktree of this clone
gets it. It refuses a push to `main` whose change requires a check that
`run_tests.py` has not recorded as passed on it, and prints the command that
runs that check. A checkout without `required_checks.py` (a branch from before
it) pushes as before.

The hook is one managed block. Text outside the block is kept.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BEGIN = "# >>> ambition required checks (managed: scripts/install_pre_push_hook.py) >>>"
END = "# <<< ambition required checks (managed) <<<"
BLOCK = f"""{BEGIN}
root="$(git rev-parse --show-toplevel 2>/dev/null)" || root=""
if [ -n "$root" ] && [ -f "$root/scripts/required_checks.py" ]; then
    python3 -B "$root/scripts/required_checks.py" --pre-push || exit 1
fi
{END}
"""


def hook_path() -> Path:
    common = subprocess.run(
        ["git", "rev-parse", "--git-common-dir"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.strip()
    common_dir = Path(common) if Path(common).is_absolute() else REPO / common
    return common_dir / "hooks" / "pre-push"


def with_block(text: str) -> str:
    if BEGIN in text and END in text:
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        return head + BLOCK.rstrip("\n") + tail
    if not text:
        text = "#!/bin/sh\n"
    return text.rstrip("\n") + "\n" + BLOCK


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    path = hook_path()
    current = path.read_text() if path.exists() else ""
    if args.check:
        installed = BLOCK.rstrip("\n") in current
        print(f"{path}: {'installed' if installed else 'NOT installed'}")
        return 0 if installed else 1
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(with_block(current))
    path.chmod(0o755)
    print(f"installed {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
