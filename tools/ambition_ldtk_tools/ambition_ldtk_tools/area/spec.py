"""Area spec loading for Ambition LDtk authoring.

RON is the canonical room-spec format. JSON is accepted for generated/ad-hoc
specs. YAML specs are intentionally rejected because the content migration moved
area specs to RON.
"""

from __future__ import annotations

import json
from pathlib import Path


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() == ".ron":
        from ambition_ldtk_tools.ron_parse import load as ron_load

        return ron_load(text)
    if path.suffix.lower() in {".yaml", ".yml"}:
        raise SystemExit(
            f"YAML area specs are no longer supported (Phase 6). "
            f"Migrate '{path}' to RON with "
            f"dev/migration-scripts/migrate_specs_to_ron.py"
        )
    return json.loads(text)
