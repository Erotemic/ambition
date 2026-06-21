"""Optional official LDtk JSON schema validation rule."""

from __future__ import annotations

import json
from pathlib import Path

PKG_DIR = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA_PATH = PKG_DIR.parent / "schemas" / "ldtk" / "JSON_SCHEMA.json"
OFFICIAL_SCHEMA_URL = "https://ldtk.io/files/JSON_SCHEMA.json"

def validate_official_schema(project, schema_path: Path | None, require_schema: bool):
    errors = []
    warnings = []
    if schema_path is None:
        default_schema = DEFAULT_SCHEMA_PATH
        if default_schema.exists():
            schema_path = default_schema
        else:
            if require_schema:
                errors.append(
                    f"official LDtk JSON schema not checked; fetch {OFFICIAL_SCHEMA_URL} to {DEFAULT_SCHEMA_PATH} "
                    "and install python package `jsonschema`"
                )
            return errors, warnings
    try:
        import jsonschema  # type: ignore[import-not-found]
    except Exception as ex:  # noqa: BLE001 - command line validator should explain environment issues
        message = f"python package `jsonschema` is required for official LDtk schema validation: {ex}"
        if require_schema:
            errors.append(message)
        else:
            warnings.append(message)
        return errors, warnings
    try:
        schema = json.loads(schema_path.read_text())
        jsonschema.Draft7Validator.check_schema(schema)
        validator = jsonschema.Draft7Validator(schema)
        schema_errors = sorted(
            validator.iter_errors(project), key=lambda error: list(error.path)
        )
    except Exception as ex:  # noqa: BLE001
        errors.append(f"failed to validate official LDtk schema {schema_path}: {ex}")
        return errors, warnings
    for error in schema_errors:
        path = ".".join(str(part) for part in error.absolute_path) or "<root>"
        errors.append(f"LDtk JSON schema: {path}: {error.message}")
    return errors, warnings
