"""The selected-file contract, read by the packaging reader.

The cases are shared with the runtime's reader
(`crates/ambition_asset_manager/tests/selected_files_contract.rs`): each is a
library and the members it selects, with origin and generation, or the word
an error must contain. Agreement is by fixture, not by a shared service.
"""
from __future__ import annotations

import json
from pathlib import Path
import sys

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from lib.content_libraries import read_libraries  # noqa: E402

CASES = REPO / "crates/ambition_asset_manager/tests/data/selected_files"


def selected(case: Path) -> dict:
    library = next(iter(read_libraries([case / "library.json"]).values()))
    rows = library.selected_files()
    return {"files": {logical: {"origin": row.origin, "generation": row.generation} for logical, row in rows.items()}}


@pytest.mark.parametrize("case", sorted(path.name for path in CASES.iterdir()))
def test_the_packaging_reader_selects_what_each_shared_case_expects(case):
    expected = json.loads((CASES / case / "expected.json").read_text())
    if "error" in expected:
        with pytest.raises(ValueError) as error:
            selected(CASES / case)
        assert expected["error"] in str(error.value).lower()
    else:
        assert selected(CASES / case) == expected


def test_a_stale_identity_is_reported_not_refused():
    library = next(iter(read_libraries([CASES / "a_stale_identity_still_selects/library.json"]).values()))
    assert library.selected_files()
    assert any("identity" in note for note in library.advisories())


def test_the_selected_paths_are_the_files_the_package_reads():
    library = next(iter(read_libraries([CASES / "a_publication_owns_its_roots/library.json"]).values()))
    rows = library.selected_files()
    assert library.files() == {logical: row.path for logical, row in rows.items()}
    assert all(path.is_file() for path in library.files().values())


def test_only_the_package_boundary_requires_a_publication():
    """An empty store is a usable library and an unformable package: the
    runtime and the resolvers select its other files, and the packager names
    the missing publication."""
    empty = next(iter(read_libraries([CASES / "nothing_published_is_an_empty_selection/library.json"]).values()))
    assert empty.selected_files(), "the library's other files stay selected"
    assert "has no publication" in (empty.requires_publication() or "")
    published = next(iter(read_libraries([CASES / "a_publication_owns_its_roots/library.json"]).values()))
    assert published.requires_publication() is None
