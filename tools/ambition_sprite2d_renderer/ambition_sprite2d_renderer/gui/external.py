"""Round-trip JSON fragments through the user's $VISUAL editor.

The GUI blocks while the external editor runs (same contract as git's
commit-message editing): write the fragment to a temp file, launch
``$VISUAL`` (falling back to ``$EDITOR``), and read the file back when
the editor exits. Returns the edited text, or ``None`` when the editor
is unset, fails to launch, or left the text unchanged.
"""

from __future__ import annotations

import os
import shlex
import subprocess
import tempfile
from pathlib import Path
from typing import Optional


def visual_command() -> Optional[list]:
    editor = os.environ.get("VISUAL") or os.environ.get("EDITOR")
    if not editor:
        return None
    return shlex.split(editor)


def edit_text_in_visual(text: str, suffix: str = ".json") -> Optional[str]:
    cmd = visual_command()
    if cmd is None:
        return None
    fd, raw_path = tempfile.mkstemp(suffix=suffix, prefix="rig_edit_")
    path = Path(raw_path)
    try:
        os.close(fd)
        path.write_text(text, encoding="utf8")
        result = subprocess.call(cmd + [str(path)])
        if result != 0:
            return None
        edited = path.read_text(encoding="utf8")
        return edited if edited != text else None
    finally:
        path.unlink(missing_ok=True)
