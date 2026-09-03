"""queue.md's "if your carve moves… it trips…" table must match the script.

`check_absence_contracts.py` has eleven contracts that pin *"this belongs to ONE
file"* by EXCLUDING that file by path. Such a contract is invisible when you
grep for the file it protects -- the filename appears in a `:(exclude)`
pathspec, never in the rule -- so `queue.md`'s post-carve checklist carries a
hand-written table mapping file to contract. A hand-written map of a machine
-readable fact drifts.

⛔ IT HAD ALREADY DRIFTED when this was written, 2026-09-03. The table filed
`snapshot_impls` under `characters/src/brain/{...}`; the real path is
`characters/src/snapshot_impls.rs`, at the CRATE ROOT, and it trips TWO
contracts rather than the one the table named. A carve that moved it would have
gone looking under `brain/` and found nothing -- in the table whose only job is
to stop exactly that.
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
QUEUE = REPO / "docs/planning/queue.md"


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
    """Contract id -> its paths, for those that EXCLUDE a path."""
    out = {}
    for c in contracts:
        paths = [str(p) for p in c.get("paths", [])]
        if any(p.startswith(":(exclude)") or p.startswith(":!") for p in paths):
            out[c["id"]] = paths
    return out


@pytest.fixture(scope="module")
def table() -> list[tuple[str, str]]:
    """`(left cell, right cell)` for every row of the checklist's table."""
    text = QUEUE.read_text()
    start = text.index("| If your carve moves…")
    body = text[start:].split("\n\n", 1)[0]
    rows = []
    for line in body.splitlines():
        line = line.strip()
        if not line.startswith("|") or line.startswith("|---") or "If your carve" in line:
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) >= 2:
            rows.append((cells[0], cells[1]))
    return rows


def test_the_checklist_items_are_numbered_in_order():
    """⛔ THE LIST IS ADVICE READ TOP TO BOTTOM, so its numbering is not cosmetic.
    It ran 1-9, 9b, 11, 10 on 2026-09-03 because item 11 was inserted with an
    anchor chosen for uniqueness rather than for position, and nobody read the
    result back. Third ordering-or-count defect introduced into a document about
    counts and orderings in one week.

    ⚠ `9b` is deliberate and must keep working: it is a sub-step of 9 (the app's
    rollback oracles, contributed by ambition-df), not a tenth item.
    """
    text = QUEUE.read_text()
    start = text.index("WHAT EVERY CARVE OWES AFTER IT LANDS")
    # ⛔ BOUND IT AT THE END OF THE CHECKLIST, not at the next top-level bullet.
    # The D33 row runs ~44,000 characters and contains a SECOND numbered list
    # further down, so slicing to "\n- " swallowed it and parsed
    # 1..11 followed by 1..6 -- the test then failed on a correct file, which is
    # how I found out.
    end_marker = "\n  ⛔⛔ RE-MEASURED 2026-08-31"
    end = text.find(end_marker, start)
    assert end != -1, (
        "the checklist's end marker is gone; this test would silently parse "
        "whatever numbered list came next"
    )
    body = text[start:end]
    items = re.findall(r"^  (\d+)([a-z]?)\. ", body, re.M)
    assert len(items) >= 10, f"only {len(items)} checklist items parsed"
    order = [(int(n), suffix) for n, suffix in items]
    assert order == sorted(order), (
        "the checklist is out of order: "
        + ", ".join(f"{n}{s}" for n, s in order)
    )


def test_the_table_was_found_and_has_rows(table):
    """⛔ THE PREMISE. A parser that silently matched nothing would make every
    assertion below vacuous -- the failure this whole file is about."""
    assert len(table) >= 8, f"only {len(table)} rows parsed from the table"


def test_every_contract_the_table_names_exists(table, contracts):
    known = {c["id"] for c in contracts}
    for left, right in table:
        for name in re.findall(r"`([a-z0-9-]{10,})`", right):
            assert name in known, (
                f"the table maps `{left}` to `{name}`, which is not a contract "
                "in check_absence_contracts.py"
            )


def test_every_path_excluding_contract_appears_in_the_table(table, contracts):
    """⛔ The table claims to cover them. A contract added later and never
    tabled is invisible in exactly the way the table exists to prevent."""
    named = " ".join(right for _, right in table)
    missing = [cid for cid in excluding_contracts(contracts) if cid not in named]
    assert not missing, (
        f"{len(missing)} contract(s) exclude a path and are NOT in the "
        f"checklist table: {missing}"
    )


def test_every_path_in_the_table_is_named_by_the_contract_beside_it(table, contracts):
    """⭐ THE ONE THAT WOULD HAVE CAUGHT THE REAL ERROR. A path in the left cell
    must actually appear in the paths of a contract named in the right cell."""
    by_id = {c["id"]: json.dumps(c.get("paths", [])) for c in contracts}
    problems = []
    for left, right in table:
        ids = [n for n in re.findall(r"`([a-z0-9-]{10,})`", right) if n in by_id]
        if not ids:
            continue
        blob = " ".join(by_id[i] for i in ids)
        # ⚠ AT LEAST ONE, not every one. A cell may legitimately name a
        # HISTORICAL path beside the live one -- D33 cut 2b moved a contract's
        # exclusion from `prepared_match.rs` to `character_runtime/match_activation.rs`
        # and the row keeps the old name, marked `cite-ok`, so the reader can
        # follow the move. Requiring every token to resolve fails such a row for
        # being MORE informative.
        # ⛔ It still catches the defect this test was written for: the
        # `snapshot_impls` row named exactly one path and it was the wrong one,
        # so NO token matched.
        pieces = [
            piece
            for token in re.findall(r"`([^`]+)`", left)
            for piece in expand(token)
            if piece
        ]
        if pieces and not any(piece in blob for piece in pieces):
            problems.append((left, ", ".join(pieces), ids))
    assert not problems, "\n".join(
        f"  the table says `{p}` trips {i}, but that path is not in its pathspec"
        for _, p, i in problems
    )


def expand(token: str) -> list[str]:
    """`a/{b, c}` -> `a/b`, `a/c`; a plain path -> itself. Prose is dropped.

    ⛔ A BARE DIRECTORY FRAGMENT IS NOT A PATH HERE. The `snapshot_impls` row's
    prose says "NOT under `brain/`", and counting `brain/` as a path made the
    at-least-one rule satisfiable by the very word the row uses to say where the
    file ISN'T -- which silently un-did this file's original finding. A path
    needs two non-empty components.
    """
    token = token.strip()
    m = re.match(r"^([\w./-]*)\{([^}]*)\}$", token)
    if m:
        stem, inner = m.groups()
        candidates = [stem + part.strip() for part in inner.split(",") if part.strip()]
    elif re.search(r"[./]", token):
        candidates = [token]
    else:
        return []
    return [c for c in candidates if len([p for p in c.split("/") if p]) >= 2]


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
