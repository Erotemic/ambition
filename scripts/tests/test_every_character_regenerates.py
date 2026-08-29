"""Every assembled character must be published by the root sprite regeneration script.

Generated sprite assets are gitignored, so a target omitted from the regen roster
can exist on one developer machine and disappear on a fresh checkout. The test
derives the expected population from the assembled cast and checks that
`scripts/regen/sprites.sh` names each required target."""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"

# Sheet stems this check deliberately does not require, with the reason.
WAIVED: dict[str, str] = {
    # Test fixtures inside unit tests, which name plausible-looking sheets on
    # purpose. They are not content and nothing draws them.
    "guest": "a catalog fixture in a unit test",
    "stranger": "a catalog fixture in a unit test",
    "alpha": "a validator fixture",
    "beta": "a validator fixture",
    "hero": "a resolver fixture",
    "borrowed_face": "a portrait-resolver fixture",
    "authored": "a `portrait_ref` unit-test fixture in ambition_characters",
    "pw": "a `portrait_ref` unit-test fixture in ambition_characters",
    "sample": "a `portrait_ref` unit-test fixture in ambition_characters",
    "m": "a `portrait_ref` unit-test fixture in ambition_characters",
    "of": "a `portrait_ref` unit-test fixture in ambition_characters",
}


def _catalog_sheet_stems() -> dict[str, set[str]]:
    """`<stem> -> {character ids that name it}` across every catalog source.

    Both roads: the content pack's RON, and the catalog fragments the demos and
    the app embed as Rust string literals. ⛔ a census over the RON alone missed
    `arena_duelist_close`, which is declared in Rust inside `ambition_app` — the
    regex was reading the places somebody already knew to look.
    """
    row = re.compile(
        r'"([a-z_0-9]+)":\s*\((?:[^()]|\([^()]*\))*?'
        r'spritesheet:\s*"sprites/([a-z_0-9]+)_spritesheet\.png"',
        re.S,
    )
    sources = [REPO / "game/ambition_content/assets/data/character_catalog.ron"]
    sources += sorted((REPO / "game").glob("*/src/**/*.rs"))
    sources += sorted((REPO / "crates").glob("*/src/**/*.rs"))

    stems: dict[str, set[str]] = {}
    for path in sources:
        try:
            text = path.read_text(encoding="utf8", errors="ignore")
        except OSError:
            continue
        for character_id, stem in row.findall(text):
            stems.setdefault(stem, set()).add(character_id)
    return stems


MAIN_CONFIGS = (
    REPO / "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/configs"
)

# THE SECOND DIRECTORY SURFACE, and this census could not see it.
# `scripts/regen/sprites.sh` builds `rig_targets` by globbing exactly this:
#
#     for rig in "$renderer_dir"/…/targets/characters/rigged/*.rig.json
#
# so a character authored as a top-level `.rig.json` is published without the
# script ever spelling its name — the same shape as `draw-all` rendering every
# `configs/*.yaml`, which this file already handles two lines below. Missing it
# reported `pointed_polygon` as an orphan while the glob published it.
#
# TOP-LEVEL ONLY, mirroring the script's glob exactly. A rig that moved
# into `rigged/<name>/` is NOT matched by `*.rig.json` and must be named in the
# roster — that is why `noether` and `oiler` are listed there by hand, and
# claiming coverage for a subdirectory here would re-open the exact gap that
# comment records: Emmy was published for weeks only by a stale top-level file
# left behind by that move.
RIGGED_TARGETS = (
    REPO
    / "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/rigged"
)


def _published_by_regen() -> str:
    """`scripts/regen/sprites.sh` with comments stripped, plus what `draw-all` covers.

     comments matter: the first run of this census counted a stem mentioned
    only in a comment as covered, and reported `player_robot_v3` as fine when
    nothing publishes it.

     THREE surfaces publish by DIRECTORY, not by name. `draw-all` renders
    every `configs/*.yaml` unconditionally and `rig_targets` globs every
    top-level `rigged/*.rig.json`, so those targets are covered without the
    script ever spelling them. They used to be spelled anyway, incidentally,
    by the hand-written `expected_files` list — and when that list was replaced
    by one derived from the roster, four of them (`dividing_mite`,
    `exploding_mite`, and the two ninjas) read as orphans here despite nothing
    about their coverage changing. Mentions are evidence of coverage; the
    surface is the coverage.
    """
    script = "\n".join(
        line.split("#")[0] for line in REGEN.read_text(encoding="utf8").splitlines()
    )
    covered = [path.name[: -len(".rig.json")] for path in sorted(RIGGED_TARGETS.glob("*.rig.json"))]
    for path in sorted(MAIN_CONFIGS.glob("*.yaml")):
        covered.append(path.stem)
        # `output_name` renames the target (ninja.yaml -> ninja_shadow_duelist).
        override = re.search(
            r"^output_name:\s*([a-z_0-9]+)", path.read_text(encoding="utf8"), re.M
        )
        if override:
            covered.append(override.group(1))
    return script + "\n" + "\n".join(covered)


def _names(stem: str, script: str) -> bool:
    # `<stem>` as a bare batch entry, or as one of the products the
    # postcondition lists. a plain substring test is wrong in both
    # directions — `goblin` appears inside `goblin_brute_hammer`, and
    # `dividing_mite` only ever appears as `dividing_mite_spritesheet.png`.
    pattern = (
        rf"(^|[^a-z0-9_]){re.escape(stem)}"
        rf"(_spritesheet|_portraits|_canonical|[^a-z0-9_]|$)"
    )
    return re.search(pattern, script) is not None


def test_every_catalog_character_names_a_sheet_regen_publishes():
    script = _published_by_regen()
    stems = _catalog_sheet_stems()
    assert len(stems) > 80, (
        f"only {len(stems)} sheet stems found across every catalog source — the "
        "scan is broken and this test is about to pass over almost nothing"
    )

    orphans = {
        stem: sorted(ids)
        for stem, ids in sorted(stems.items())
        if stem not in WAIVED and not _names(stem, script)
    }
    assert not orphans, (
        f"{len(orphans)} character sheet(s) are named by a catalog and published "
        "by no `scripts/regen/sprites.sh` batch:\n"
        + "\n".join(f"  {stem:32s} used by {', '.join(ids)}" for stem, ids in orphans.items())
        + "\n\nGenerated art is gitignored, so these exist only on machines that "
        "once rendered them and are ABSENT from a fresh clone. Add the target to "
        "`tackon_targets` in the script's publish roster (its expected files are "
        "derived from that roster), or stop naming the sheet. "
        "`sprite2d_renderer list` says whether a target is registered."
    )


def test_the_scan_would_notice_an_orphan():
    """The poison: a stem nothing publishes has to be reported.

     without this the check above is one broken regex away from being a test
    that passes because it found nothing to look at — which is exactly how the
    26 stayed invisible while a portrait checker sat next to them.
    """
    script = _published_by_regen()
    assert not _names("a_character_nobody_ever_drew", script)
    # …and a real one is not reported, so the matcher is not simply always false.
    assert _names("alice", script)
