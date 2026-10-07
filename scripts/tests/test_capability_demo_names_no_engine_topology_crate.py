"""The external-capability witness must not depend on engine-internal topology.

`examples/capability_demo` stands in for a capability written OUTSIDE the engine
(Gate C5). It may name the narrow crates it extends (`ambition_content_pack`,
`ambition_causal`, `ambition_input`, `ambition_platformer2d_core`,
`ambition_sim_schedule`); it may not name `ambition_platformer2d_shared_tangle`,
a crate named for the engine's internal topology. The simulation-schedule seam it
needs lives in `ambition_sim_schedule` for that reason. The `capability-demo-...`
absence contract covers the demo's TESTS going through the facade; nothing covered
its manifest and source until this.
"""

from __future__ import annotations

import re
from pathlib import Path

DEMO = Path(__file__).resolve().parents[2] / "examples/capability_demo"
FORBIDDEN = "ambition_platformer2d_shared_tangle"


def _offences(root: Path) -> list[str]:
    found = []
    for path in [root / "Cargo.toml", *sorted((root / "src").rglob("*.rs")), *sorted((root / "tests").rglob("*.rs"))]:
        for number, line in enumerate(path.read_text().splitlines(), 1):
            code = re.sub(r"^\s*(//.*|#.*)$", "", line)  # a comment may explain the history
            if FORBIDDEN in code:
                found.append(f"{path.relative_to(root)}:{number}: {line.strip()}")
    return found


def test_the_capability_demo_does_not_name_the_tangle_crate():
    assert (DEMO / "src/lib.rs").is_file(), "the demo moved; this guard would test nothing"
    assert not _offences(DEMO), (
        f"capability_demo names {FORBIDDEN}, an engine-internal topology crate. Use "
        "`ambition_sim_schedule` for the simulation schedule and phase vocabulary:\n"
        + "\n".join(_offences(DEMO))
    )


def test_the_guard_can_see_an_offence(tmp_path):
    """The control: a manifest that DOES name the crate is reported."""
    (tmp_path / "src").mkdir()
    (tmp_path / "Cargo.toml").write_text(f'[dependencies]\n{FORBIDDEN} = {{ path = "x" }}\n')
    (tmp_path / "src/lib.rs").write_text("// ambition_platformer2d_shared_tangle in a comment is fine\n")
    assert len(_offences(tmp_path)) == 1
