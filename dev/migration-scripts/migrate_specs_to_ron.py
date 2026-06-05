#!/usr/bin/env python3
"""One-shot migration: convert every `tools/ambition_ldtk_tools/specs/
*.yaml` area spec to `.ron`.

Part of Phase 4 of the character-catalog refactor (see
`TODO-character-catalog-and-hall.md`). Goal: standardize the area-
authoring format on RON so the project policy "one config format"
holds across both Rust- and Python-consumed configs.

The conversion is value-preserving: `yaml.safe_load(yaml_text)` and
`ron_parse.load(generated_ron_text)` produce identical Python
trees. Comments do NOT survive — YAML supports `#` line comments
and the area specs use them liberally. The migration prepends a
file-level header comment captured from the original YAML's leading
`#`-prefixed lines, then writes the data in RON.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.migrate_specs_to_ron --delete-yaml
```

The script:
  1. Walks `tools/ambition_ldtk_tools/specs/*.yaml`.
  2. For each, parses YAML → emits RON next to it.
  3. Verifies the RON round-trips back to the same Python tree.
  4. With `--delete-yaml`, removes the originals on success.

A `--dry-run` flag prints what would happen without writing.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SPECS_DIR = REPO_ROOT / "tools" / "ambition_ldtk_tools" / "specs"


def extract_header_comment(yaml_text: str) -> str:
    """Capture the leading `# ...` line block. Returns an empty
    string if the file doesn't start with comments."""
    lines: list[str] = []
    for raw in yaml_text.splitlines():
        if raw.startswith("#"):
            # Strip the `#` (and the optional single space after it)
            # so the comment converts cleanly to a `// ...` RON line.
            body = raw[1:]
            if body.startswith(" "):
                body = body[1:]
            lines.append(body)
            continue
        if raw.strip() == "":
            # Blank lines inside the header block stay.
            if lines:
                lines.append("")
            continue
        break
    while lines and lines[-1] == "":
        lines.pop()
    if not lines:
        return ""
    return "\n".join(f"// {line}" if line else "//" for line in lines) + "\n\n"


def migrate_file(
    yaml_path: Path, dry_run: bool, delete_yaml: bool
) -> tuple[Path | None, str]:
    """Returns (output_path_or_none, status_str)."""
    import yaml  # type: ignore
    from .ron_parse import dumps as ron_dumps, load as ron_load

    yaml_text = yaml_path.read_text()
    try:
        data = yaml.safe_load(yaml_text)
    except yaml.YAMLError as e:
        return None, f"YAML parse error: {e}"

    if not isinstance(data, dict):
        return None, f"top-level must be a dict, got {type(data).__name__}"

    header = extract_header_comment(yaml_text)
    ron_text = header + ron_dumps(data)

    # Round-trip safety check.
    try:
        parsed = ron_load(ron_text)
    except Exception as e:
        return None, f"generated RON failed to round-trip: {e}"
    if parsed != data:
        return None, "generated RON parses to a different tree than the YAML"

    ron_path = yaml_path.with_suffix(".ron")
    if dry_run:
        return ron_path, f"OK (dry-run, would write {len(ron_text)} bytes)"
    ron_path.write_text(ron_text)
    if delete_yaml:
        yaml_path.unlink()
    return ron_path, f"wrote {len(ron_text)} bytes"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="show what would happen without writing or deleting",
    )
    parser.add_argument(
        "--delete-yaml",
        action="store_true",
        help="delete the YAML originals after a successful conversion",
    )
    parser.add_argument(
        "--specs-dir",
        type=Path,
        default=SPECS_DIR,
        help="override the specs directory",
    )
    args = parser.parse_args(argv)

    any_error = False
    yaml_files = sorted(args.specs_dir.glob("*.yaml")) + sorted(
        args.specs_dir.glob("*.yml")
    )
    if not yaml_files:
        print(f"no YAML specs found under {args.specs_dir}")
        return 0

    for yaml_path in yaml_files:
        out, status = migrate_file(yaml_path, args.dry_run, args.delete_yaml)
        marker = "[ok]" if out else "[ERR]"
        rel = yaml_path.relative_to(REPO_ROOT)
        print(f"  {marker} {rel} -> {status}")
        if out is None:
            any_error = True

    return 1 if any_error else 0


if __name__ == "__main__":
    sys.exit(main())
