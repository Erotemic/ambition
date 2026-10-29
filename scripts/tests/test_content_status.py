"""The content status query tells what generated content is missing or stale, fast, and does not refuse."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/content_status.py"


def checkout(tmp_path: Path) -> tuple[Path, Path]:
    """A checkout with one library (two producers) and one pack that reads a generated and an authored source."""
    repo = tmp_path / "repo"
    (repo / ".git").mkdir(parents=True)
    library = repo / "game/library"
    (library / "authored/runtime/audio").mkdir(parents=True)
    (library / "authored/runtime/audio/sfx_registry.ron").write_text("()")
    (library / "authored/music.json").write_text("{}")
    manifest = library / "library.json"
    manifest.write_text(json.dumps({
        "schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"authored": "authored/runtime", "generated": "generated/runtime/full"},
        "publication": "generated/publication",
        "producers": {
            "music": {"roots": ["audio/music", "audio/music_registry.ron"], "registry": "audio/music_registry.ron"},
            "sfx": {"roots": ["audio/sfx.bank"]},
        }}))
    pack = repo / "game/fixture_pack/assets"
    pack.mkdir(parents=True)
    (pack / "pack.ron").write_text("""(
    id: "fixture", version: "1", namespace: "fixture",
    libraries: ["../../library/library.json"],
    sources: [
        (path: "audio/music_registry.ron", library: Some("fixture"), schema: "music_registry", version: 1),
        (path: "audio/sfx_registry.ron", library: Some("fixture"), schema: "sfx_registry", version: 1),
    ],
)
""")
    return repo, manifest


def query(manifest: Path, *arguments: str) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable, str(SCRIPT), "--library", str(manifest), *arguments],
                          cwd=ROOT, text=True, capture_output=True)


def publish(manifest: Path, unit: str, inputs: list[Path]) -> None:
    """Accept one bundle for `unit` whose recipe records `inputs` as they are now."""
    store = manifest.parent / "generated/publication"
    key = hashlib.sha256(unit.encode()).hexdigest()
    rows = [{"path": path.relative_to(manifest.parents[2]).as_posix(),
             "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for path in inputs]
    (store / "bundles" / key).mkdir(parents=True)
    (store / "bundles" / key / "manifest.json").write_text(json.dumps(
        {"unit": unit, "files": {}, "recipe": {"generation": {"inputs": rows}}}))
    (store / "published.json").write_text(json.dumps(
        {"schema": "ambition.art_publication.v1", "generation": "g", "bundles": {unit: key}, "files": {}, "roots": []}))


def test_an_empty_store_reports_each_family_missing_and_exits_zero(tmp_path):
    _repo, manifest = checkout(tmp_path)
    started = time.monotonic()
    result = query(manifest, "--json")
    assert result.returncode == 0, result.stderr
    assert time.monotonic() - started < 5, "the query must be fast"
    report = json.loads(result.stdout)
    assert report["missing"] == ["music", "sfx"]
    assert report["complete"] is False
    assert report["libraries"][0]["store"]["present"] is False
    sources = {row["path"]: row for row in report["sources"]}
    assert sources["audio/music_registry.ron"]["state"] == "missing"
    assert sources["audio/music_registry.ron"]["producer"] == "music"
    assert sources["audio/sfx_registry.ron"]["state"] == "authored"
    text = query(manifest)
    assert text.returncode == 0
    assert "2 generated families missing (music, sfx)" in text.stdout
    assert "scripts/setup/generated_content.sh" in text.stdout


def test_require_complete_is_the_only_road_to_a_failing_status(tmp_path):
    _repo, manifest = checkout(tmp_path)
    assert query(manifest, "--require-complete").returncode == 1
    generated = manifest.parent / "generated/runtime/full"
    (generated / "audio/music").mkdir(parents=True)
    (generated / "audio/music_registry.ron").write_text("()")
    (generated / "audio/sfx.bank").write_bytes(b"bank")
    complete = query(manifest, "--require-complete", "--json")
    assert complete.returncode == 0, complete.stdout
    report = json.loads(complete.stdout)
    assert report["missing"] == [] and report["complete"] is True
    assert {row["path"]: row["state"] for row in report["sources"]}["audio/music_registry.ron"] == "loose"


def test_a_family_filter_reports_only_that_family(tmp_path):
    _repo, manifest = checkout(tmp_path)
    report = json.loads(query(manifest, "--family", "music", "--json").stdout)
    assert list(report["libraries"][0]["families"]) == ["music"]
    assert [row["path"] for row in report["sources"]] == ["audio/music_registry.ron"]


def test_a_published_unit_is_present_until_a_recorded_input_changes(tmp_path):
    _repo, manifest = checkout(tmp_path)
    policy = manifest.parent / "authored/music.json"
    publish(manifest, "producer:music", [policy])
    current = json.loads(query(manifest, "--family", "music", "--json").stdout)
    assert current["libraries"][0]["families"]["music"]["state"] == "present"
    assert current["stale"] == []
    policy.write_text('{"changed": true}')
    stale = query(manifest, "--family", "music", "--json")
    assert stale.returncode == 0
    report = json.loads(stale.stdout)
    assert report["libraries"][0]["families"]["music"]["state"] == "stale"
    assert report["stale"] == ["music"]
    assert report["libraries"][0]["families"]["music"]["stale_units"][0]["changed"] == ["game/library/authored/music.json"]
    unchecked = json.loads(query(manifest, "--family", "music", "--json", "--no-stale").stdout)
    assert unchecked["libraries"][0]["families"]["music"]["state"] == "present"


def test_an_absent_authored_source_is_an_authoring_error(tmp_path):
    _repo, manifest = checkout(tmp_path)
    (manifest.parent / "authored/runtime/audio/sfx_registry.ron").unlink()
    report = json.loads(query(manifest, "--json").stdout)
    sfx = {row["path"]: row for row in report["sources"]}["audio/sfx_registry.ron"]
    # The sfx producer declares only the bank, so the registry is authored.
    assert sfx["state"] == "missing-authored"
    assert len(report["authoring_errors"]) == 1
    assert report["authoring_errors"][0].endswith("game/fixture_pack/assets/pack.ron: audio/sfx_registry.ron")
    assert query(manifest, "--require-complete").returncode == 1


def test_the_real_checkout_answers_without_a_build():
    result = subprocess.run([sys.executable, str(SCRIPT), "--json", "--no-stale"], cwd=ROOT, text=True, capture_output=True)
    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report["schema"] == "ambition.content_status.v1"
    assert {"music", "sfx", "sprites"} <= set(report["libraries"][0]["families"])
