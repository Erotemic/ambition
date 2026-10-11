"""The cargo package a workspace member is.

`cargo test -p` takes the manifest's `[package] name`, not the member's
directory name. The two differ for a member such as
`fixtures/extension_fixture_modules`, whose package is
`ambition_extension_fixture_modules`, and a runner that asked for the
directory name asked cargo for a package that does not exist.
"""

from __future__ import annotations

import functools
import tomllib
from pathlib import Path


@functools.cache
def package_name(member: Path) -> str:
    """The `[package] name` of the member at `member`, or its directory name
    when it has no manifest that names one (a test fixture's bare member)."""
    manifest = member / "Cargo.toml"
    try:
        name = tomllib.loads(manifest.read_text()).get("package", {}).get("name")
    except (OSError, tomllib.TOMLDecodeError):
        name = None
    return name if isinstance(name, str) else member.name
