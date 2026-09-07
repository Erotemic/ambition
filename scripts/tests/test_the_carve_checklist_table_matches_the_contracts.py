"""The actor-decomposition safety map must match path-excluding contracts.

A path-excluding absence contract can become vacuous when a carve moves the owner
and leaves the old excluded path behind. The durable owner document therefore
maps carve-sensitive paths to the contracts that must move with them.
"""

from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
OWNER_DOC = REPO / "docs/planning/engine/actor-monolith-decomposition.md"


@pytest.fixture(scope="module")
def contracts() -> list[dict]:
    spec = importlib.util.spec_from_file_location(
        "absence", REPO / "scripts/check_absence_contracts.py"
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module.ABSENCE_CONTRACTS


def excluding_contracts(contracts: list[dict]) -> dict[str, list[str]]:
    out = {}
    for contract in contracts:
        paths = [str(path) for path in contract.get("paths", [])]
        if any(path.startswith(":(exclude)") or path.startswith(":!") for path in paths):
            out[contract["id"]] = paths
    return out


@pytest.fixture(scope="module")
def table() -> list[tuple[str, str]]:
    text = OWNER_DOC.read_text(encoding="utf-8")
    start = text.index("| If your carve moves…")
    body = text[start:].split("\n\n", 1)[0]
    rows = []
    for line in body.splitlines():
        line = line.strip()
        if not line.startswith("|") or line.startswith("|---") or "If your carve" in line:
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) >= 2:
            rows.append((cells[0], cells[1]))
    return rows


def expand(token: str) -> list[str]:
    token = token.strip()
    match = re.match(r"^([\w./-]*)\{([^}]*)\}$", token)
    if match:
        stem, inner = match.groups()
        candidates = [stem + part.strip() for part in inner.split(",") if part.strip()]
    elif re.search(r"[./]", token):
        candidates = [token]
    else:
        return []
    return [candidate for candidate in candidates if len([p for p in candidate.split("/") if p]) >= 2]


def test_the_table_was_found_and_has_rows(table):
    assert len(table) >= 8, f"only {len(table)} rows parsed from the carve safety map"


def test_every_contract_the_table_names_exists(table, contracts):
    known = {contract["id"] for contract in contracts}
    for left, right in table:
        for name in re.findall(r"`([a-z0-9-]{10,})`", right):
            assert name in known, f"`{left}` names unknown absence contract `{name}`"


def test_every_path_excluding_contract_appears_in_the_table(table, contracts):
    named = " ".join(right for _, right in table)
    missing = [cid for cid in excluding_contracts(contracts) if cid not in named]
    assert not missing, f"path-excluding absence contracts missing from owner map: {missing}"


def test_every_path_in_the_table_is_named_by_the_contract_beside_it(table, contracts):
    by_id = {contract["id"]: json.dumps(contract.get("paths", [])) for contract in contracts}
    problems = []
    for left, right in table:
        ids = [name for name in re.findall(r"`([a-z0-9-]{10,})`", right) if name in by_id]
        if not ids:
            continue
        blob = " ".join(by_id[cid] for cid in ids)
        pieces = [
            piece
            for token in re.findall(r"`([^`]+)`", left)
            for piece in expand(token)
            if piece
        ]
        if pieces and not any(piece in blob for piece in pieces):
            problems.append((left, pieces, ids))
    assert not problems, "\n".join(
        f"  `{left}` maps to {ids}, but none of {pieces} occur in those pathspecs"
        for left, pieces, ids in problems
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
