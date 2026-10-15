"""The per-sheet cache key of `scripts/regen/sprites.sh` (`scripts/lib/sprite_cache_key.sh`).

The functions run as the publish runs them, on a small renderer tree: three
targets, two with a rig each, a helper that the core imports, and art that two
targets read. The closure map, the core's files and each target's read record
are written here as `sprite_cache_key_prepare` and a recorded publish would
write them (those steps ask the renderer, which this tree has not).
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
LIB = REPO / "scripts" / "lib" / "sprite_cache_key.sh"
PKG = "ambition_sprite2d_renderer"


@pytest.fixture
def renderer(tmp_path: Path) -> Path:
    root = tmp_path / "renderer"
    for rel, text in {
        f"{PKG}/__init__.py": "",
        f"{PKG}/core/__init__.py": "",
        f"{PKG}/core/draw.py": "from ..targets.fam import _shared\n",
        f"{PKG}/targets/__init__.py": "",
        f"{PKG}/targets/fam/__init__.py": "",
        f"{PKG}/targets/fam/_shared.py": "S = 1\n",
        f"{PKG}/targets/fam/_rig_a.py": "A = 1\n",
        f"{PKG}/targets/fam/_rig_b.py": "B = 1\n",
        f"{PKG}/targets/fam/alpha.py": "from ._rig_a import A\n",
        f"{PKG}/targets/fam/beta.py": "from ._rig_b import B\n",
        f"{PKG}/targets/fam/gamma.py": "G = 1\n",
        f"{PKG}/data/characters/villain/villain.svg": "<svg/>\n",
        f"{PKG}/data/characters/alpha/alpha.svg": "<svg/>\n",
        f"{PKG}/data/characters/gamma/gamma.svg": "<svg/>\n",
    }.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    fam = f"{PKG}/targets/fam"
    (tmp_path / "closures.tsv").write_text(
        f"alpha\t{fam}/alpha.py\nalpha\t{fam}/_rig_a.py\nbeta\t{fam}/beta.py\nbeta\t{fam}/_rig_b.py\n"
        f"gamma\t{fam}/gamma.py\n"
    )
    # What each target's last render read, relative to the repository (here
    # `tmp_path`). Alpha and beta both draw the villain's art, as
    # `gnu_ton_scholar_fighter` draws `gnu_ton_scholar_villain.svg`.
    data = f"renderer/{PKG}/data/characters"
    reads = tmp_path / "reads"
    reads.mkdir()
    (reads / "alpha.reads").write_text(
        f"f {data}/alpha/alpha.svg\nf {data}/villain/villain.svg\n"
    )
    (reads / "beta.reads").write_text(f"f {data}/villain/villain.svg\n")
    (reads / "gamma.reads").write_text(f"f {data}/gamma/gamma.svg\nd {data}/gamma\n")
    return root


def keys(renderer: Path) -> dict[str, str]:
    """`unit_key` of each target, by the library's own functions."""
    script = f"""
        set -euo pipefail
        source {LIB}
        renderer_dir={renderer}
        repo_root={renderer.parent}
        target_closures_file={renderer.parent / 'closures.tsv'}
        target_reads_dir={renderer.parent / 'reads'}
        core_target_files={PKG}/targets/fam/_shared.py
        core_shared_fingerprint="$(compute_core_shared)"
        for name in alpha beta gamma; do printf '%s %s\\n' "$name" "$(unit_key "$name")"; done
    """
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    return dict(line.split() for line in out.splitlines())


def edit(renderer: Path, rel: str) -> None:
    path = renderer / PKG / "targets" / "fam" / rel
    path.write_text(path.read_text() + "# edited\n")


def test_an_edit_to_one_targets_rig_makes_only_that_target_stale(renderer: Path) -> None:
    """The defect: every `_*.py` helper was in the shared core, so the crew's
    rig made all 271 sheets stale, a republish of about one hour."""
    before = keys(renderer)
    edit(renderer, "_rig_a.py")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"], "the edit to alpha's rig did not make alpha stale"
    assert after["beta"] == before["beta"], "the edit to alpha's rig made beta stale"


def test_an_edit_to_a_target_module_makes_it_stale(renderer: Path) -> None:
    before = keys(renderer)
    edit(renderer, "beta.py")
    after = keys(renderer)
    assert after["beta"] != before["beta"] and after["alpha"] == before["alpha"]


def test_an_edit_to_a_helper_the_core_imports_makes_every_target_stale(renderer: Path) -> None:
    """The control: the core runs for every target."""
    before = keys(renderer)
    edit(renderer, "_shared.py")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"] and after["beta"] != before["beta"]


def edit_art(renderer: Path, character: str) -> None:
    path = renderer / PKG / "data" / "characters" / character / f"{character}.svg"
    path.write_text(path.read_text() + "<!-- edited -->\n")


def test_an_edit_to_shared_art_makes_each_target_that_read_it_stale_and_no_other(renderer: Path) -> None:
    """The defect (review of 2026-10-14): the key read art by the target's own
    name, so `gnu_ton_scholar_fighter` kept its key when the villain's SVG it
    draws changed."""
    before = keys(renderer)
    edit_art(renderer, "villain")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"], "alpha read the villain's art and kept its key"
    assert after["beta"] != before["beta"], "beta read the villain's art and kept its key"
    assert after["gamma"] == before["gamma"], "gamma did not read the villain's art and went stale"


def test_an_edit_to_a_targets_own_art_makes_only_it_stale(renderer: Path) -> None:
    before = keys(renderer)
    edit_art(renderer, "alpha")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"]
    assert after["beta"] == before["beta"] and after["gamma"] == before["gamma"]


def test_a_new_file_in_a_directory_a_target_listed_makes_it_stale(renderer: Path) -> None:
    before = keys(renderer)
    (renderer / PKG / "data" / "characters" / "gamma" / "gamma_extra.svg").write_text("<svg/>\n")
    after = keys(renderer)
    assert after["gamma"] != before["gamma"] and after["alpha"] == before["alpha"]


def test_a_target_with_no_record_is_never_fresh(renderer: Path) -> None:
    """A target that never published under a record has no data half to key,
    so no stored key may equal its key."""
    (renderer.parent / "reads" / "beta.reads").unlink()
    assert keys(renderer)["beta"] != keys(renderer)["beta"]


def fingerprint(renderer: Path) -> str:
    """`compute_fingerprint`, the key of the whole publish: when it equals the
    saved one and every output is present, the publish renders nothing."""
    regen = renderer.parent / "scripts" / "regen" / "sprites.sh"
    regen.parent.mkdir(parents=True, exist_ok=True)
    if not regen.exists():
        regen.write_text("# the orchestrator\n")
    script = f"""
        set -euo pipefail
        source {LIB}
        renderer_capability_fingerprint() {{ echo 'capability resvg'; }}
        renderer_dir={renderer}
        repo_root={renderer.parent}
        target_reads_dir={renderer.parent / 'reads'}
        compute_fingerprint
    """
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout.strip()


def test_an_edit_to_art_makes_the_whole_publish_stale(renderer: Path) -> None:
    """The defect (measured 2026-10-14): the whole-publish key hashed only the
    renderer's `.py`, `.yaml` and `.json`. A comment added to
    `gnu_ton_scholar_villain.svg` gave "regen cache hit: renderer sources +
    outputs unchanged", and no target was published. Here the SVG is one that
    no record lists, as the phases with no per-sheet key (entities, factions)
    read theirs."""
    art = renderer / PKG / "data" / "entities" / "torch.svg"
    art.parent.mkdir(parents=True)
    art.write_text("<svg/>\n")
    before = fingerprint(renderer)
    art.write_text("<svg><!-- edited --></svg>\n")
    assert fingerprint(renderer) != before, "an edit to an SVG kept the whole-publish key"


def test_an_edit_to_the_renderers_top_level_assets_makes_the_whole_publish_stale(
    renderer: Path,
) -> None:
    """Six renders read the renderer's top-level `assets/` (`alice.svg`),
    which is not in the package; a phase with no record reads it unlisted."""
    art = renderer / "assets" / "alice.svg"
    art.parent.mkdir()
    art.write_text("<svg/>\n")
    before = fingerprint(renderer)
    art.write_text("<svg><!-- edited --></svg>\n")
    assert fingerprint(renderer) != before


def test_an_edit_to_a_recorded_file_outside_the_renderer_makes_the_whole_publish_stale(
    renderer: Path,
) -> None:
    """The per-sheet keys hash what each record lists, wherever it is. A
    whole-publish key that omits one would hit where a sheet key misses."""
    art = renderer.parent / "shared" / "banner.svg"
    art.parent.mkdir()
    art.write_text("<svg/>\n")
    reads = renderer.parent / "reads" / "gamma.reads"
    reads.write_text(reads.read_text() + "f shared/banner.svg\n")
    before = fingerprint(renderer)
    art.write_text("<svg><!-- edited --></svg>\n")
    assert fingerprint(renderer) != before


def test_what_no_render_reads_does_not_make_the_whole_publish_stale(renderer: Path) -> None:
    """The control: bytecode, the cache and a file outside the renderer that
    no record lists."""
    before = fingerprint(renderer)
    (renderer / PKG / "__pycache__").mkdir()
    (renderer / PKG / "__pycache__" / "draw.cpython-311.pyc").write_bytes(b"\0")
    (renderer / ".cache").mkdir()
    (renderer / ".cache" / "publish_seconds.json").write_text("{}\n")
    (renderer.parent / "unrelated.txt").write_text("x\n")
    assert fingerprint(renderer) == before


def publish(renderer: Path, during: str) -> dict[str, str]:
    """The fingerprint steps of one publish, in the order `sprites.sh` runs
    them: the key at the start, `during` (shell run where the renders run),
    then what it saves and what the next run (and `--check`) computes."""
    regen = renderer.parent / "scripts" / "regen" / "sprites.sh"
    regen.parent.mkdir(parents=True, exist_ok=True)
    if not regen.exists():
        regen.write_text("# the orchestrator\n")
    script = f"""
        set -euo pipefail
        source {LIB}
        renderer_capability_fingerprint() {{ echo 'capability resvg'; }}
        renderer_dir={renderer}
        repo_root={renderer.parent}
        target_reads_dir={renderer.parent / 'reads'}
        current="$(compute_fingerprint)"
        started="$(compute_authored_fingerprint)"
        {during}
        if saved="$(fingerprint_after_publish "$started")"; then :; else saved=none; fi
        printf 'start %s\\nsaved %s\\nnext %s\\n' "$current" "$saved" "$(compute_fingerprint)"
    """
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    return dict(line.split() for line in out.splitlines())


def test_a_publish_that_writes_a_new_record_is_fresh_when_it_ends(renderer: Path) -> None:
    """The defect (review of 2026-10-14): the publish saved the fingerprint
    from before it rendered, and a render that wrote a record changed the
    fingerprint, so `--check` called the new set stale and the next run
    took the slow path."""
    data = f"renderer/{PKG}/data/characters"
    run = publish(
        renderer,
        f"printf 'f {data}/gamma/gamma.svg\\nf {data}/villain/villain.svg\\n' > {renderer.parent / 'reads' / 'gamma.reads'}",
    )
    assert run["start"] != run["next"], "the premise: the render's record changed the fingerprint"
    assert run["saved"] == run["next"], "the saved fingerprint is not the one the next run computes"


def test_a_publish_with_no_change_saves_the_fingerprint_it_started_with(renderer: Path) -> None:
    run = publish(renderer, ":")
    assert run["start"] == run["saved"] == run["next"]


def test_an_authored_edit_during_the_publish_saves_no_fingerprint(renderer: Path) -> None:
    """The render may have read the file before the edit, so the run cannot
    call the edit published."""
    run = publish(renderer, f"echo '<!-- edited -->' >> {renderer / PKG / 'data' / 'characters' / 'gamma' / 'gamma.svg'}")
    assert run["saved"] == "none"


def script_keys(renderer: Path, record: Path) -> str:
    """`script_key` of a script that imports target `alpha`, by the library."""
    script = f"""
        set -euo pipefail
        source {LIB}
        renderer_dir={renderer}
        repo_root={renderer.parent}
        target_closures_file={renderer.parent / 'closures.tsv'}
        target_reads_dir={renderer.parent / 'reads'}
        core_target_files={PKG}/targets/fam/_shared.py
        core_shared_fingerprint="$(compute_core_shared)"
        script_key {record} alpha scripts/card.py
    """
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout.strip()


def test_a_script_that_is_not_a_target_is_keyed_as_a_target_is(renderer: Path) -> None:
    """The director vanity card is a script, not a target, and it re-rendered
    on every publish (56 s, measured 2026-10-15). Its key moves with its own
    file, the target it imports and what it read, and with nothing else."""
    (renderer / "scripts").mkdir()
    (renderer / "scripts" / "card.py").write_text("CARD = 1\n")
    data = f"renderer/{PKG}/data/characters"
    record = renderer.parent / "reads" / "card.reads"
    record.write_text(f"f {data}/gamma/gamma.svg\n")
    before = script_keys(renderer, record)
    assert script_keys(renderer, record) == before, "control: the key is stable"
    edit(renderer, "beta.py")
    assert script_keys(renderer, record) == before, "an edit the script does not read moved its key"
    (renderer / PKG / "data" / "characters" / "gamma" / "gamma.svg").write_text("<svg>edited</svg>\n")
    after_read = script_keys(renderer, record)
    assert after_read != before, "an edit to a file the script read kept its key"
    edit(renderer, "_rig_a.py")
    after_target = script_keys(renderer, record)
    assert after_target != after_read, "an edit to the target the script imports kept its key"
    (renderer / "scripts" / "card.py").write_text("CARD = 2\n")
    assert script_keys(renderer, record) != after_target, "an edit to the script kept its key"
    record.unlink()
    assert script_keys(renderer, record) != script_keys(renderer, record), "a script with no record matched a key"


def ultrapack_key(sheets: Path, plan: Path) -> str:
    script = f"""
        set -euo pipefail
        source {LIB}
        core_shared_fingerprint=core-a
        ultrapack_inputs_key {sheets} {plan}
    """
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout.strip()


def test_the_ultrapack_key_moves_with_a_pooled_sheet_and_the_plan_and_nothing_else(tmp_path: Path) -> None:
    sheets = tmp_path / "sprites"
    sheets.mkdir()
    for name in ("alice_spritesheet.yaml", "alice_spritesheet.png", "goblin_spritesheet.png"):
        (sheets / name).write_text(name)
    plan = tmp_path / "pack_plan.yaml"
    plan.write_text("groups: []\n")
    before = ultrapack_key(sheets, plan)
    assert ultrapack_key(sheets, plan) == before, "the key is stable"

    # Control: a file the tiers do not pool (a part sheet, a nested sheet).
    (sheets / "alice_parts.png").write_text("parts")
    (sheets / "entities").mkdir()
    (sheets / "entities" / "chest_spritesheet.png").write_text("nested")
    assert ultrapack_key(sheets, plan) == before

    page = sheets / "goblin_spritesheet.png"
    page.write_text("goblin, drawn again")
    assert ultrapack_key(sheets, plan) != before, "a re-rendered page"
    after_page = ultrapack_key(sheets, plan)
    plan.write_text("groups: [hall]\n")
    assert ultrapack_key(sheets, plan) != after_page, "an edit to the plan"
