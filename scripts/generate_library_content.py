#!/usr/bin/env python3
"""Stage and publish the selected library's generated media."""
from __future__ import annotations

from concurrent.futures import ProcessPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile

from lib.content_libraries import selected_manifest, read_libraries, relative

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.publication_store import Store
from ambition_sprite2d_renderer.products import Digests, generation

OWNER = "ambition.library_producer.v1"
TOOLS = {"backgrounds": "ambition_sprite2d_renderer", "variants": "ambition_sprite2d_renderer",
         "music": "ambition_music_renderer", "sfx": "ambition_sfx_renderer"}
SCRIPTS = {"backgrounds": "scripts/regen/backgrounds.sh", "variants": "scripts/regen/quality_variants.sh",
           "music": "scripts/regen/music.sh", "sfx": "scripts/regen/sfx.sh", "fonts": "scripts/grab_font_assets.py"}
_INPUTS = {}
_DEPENDENCIES = {}
IGNORED = {"__pycache__", ".cache", ".venv", "build", ".profiles", ".git"}


def interpreter(family: str) -> str:
    tool = TOOLS.get(family)
    if tool is None:
        return sys.executable
    variable = "AMBITION_" + ("SPRITE" if family in {"backgrounds", "variants"} else family.upper()) + "_PYTHON"
    result = subprocess.run(["bash", "-c", 'source "$1/scripts/lib/tool_python.sh"; ambition_select_tool_python "$1/tools/$2" "$3"',
                             "_", str(ROOT), tool, variable], check=True, capture_output=True, text=True)
    return shutil.which(result.stdout.strip()) or result.stdout.strip()


def in_owner(family: str, arguments: list[str]) -> None:
    if os.environ.get("AMBITION_PRODUCER_OWNER") == family:
        return
    env = dict(os.environ, AMBITION_PRODUCER_OWNER=family)
    if family == "music":
        os.execvpe("bash", ["bash", "-c", 'audio_env="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}/env.sh"; if [ -f "$audio_env" ]; then source "$audio_env"; fi; exec "$@"',
                            "producer", interpreter(family), __file__, *arguments], env)
    os.execve(interpreter(family), [interpreter(family), __file__, *arguments], env)


def library():
    return next(iter(read_libraries([selected_manifest(ROOT)]).values()))


def input_files(family: str, selected) -> list[Path]:
    cache_key = (family, str(selected.path))
    if cache_key in _INPUTS:
        return _INPUTS[cache_key]
    files = {selected.path, Path(__file__), ROOT / SCRIPTS[family], ROOT / "scripts/lib/content_libraries.py"}
    config = selected.manifest["producers"][family]
    if config.get("policy"):
        files.add(selected.path.parent / relative(config["policy"]))
    roots = []
    tool = TOOLS.get(family)
    if tool and family != "variants":
        base = ROOT / "tools" / tool
        roots.append(base / tool)
        files.update(base / name for name in ("pyproject.toml", "uv.lock") if (base / name).is_file())
    if family == "variants":
        files.update(ROOT / "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer" / name for name in ("quality.py", "products.py", "content_library.py", "provenance.py"))
    elif family == "backgrounds":
        roots.append(ROOT / "tools/ambition_background_renderer/ambition_background_renderer")
    elif family == "music":
        import regen_music_registry as registry
        from ambition_music_renderer.cli import find_score
        for cue in registry.generation_cues():
            path = find_score(cue)
            if path is None:
                raise ValueError(f"Music cue has no authored score: {cue}")
            files.add(path)
        files.add(ROOT / "scripts/regen_music_registry.py")
    elif family == "sfx":
        roots.append(ROOT / "tools/ambition_sfx_renderer/sounds" / config["collection"])
        files.update([ROOT / "tools/ambition_sfx_renderer/sounds/public_samples.yaml", ROOT / "tools/ambition_sfx_pack/pack.py"])
    for root in roots:
        files.update(path for path in root.rglob("*") if path.is_file()
                     and not any(part in IGNORED for part in path.parts) and path.suffix != ".pyc")
    result = sorted(path.resolve() for path in files if path.is_file())
    _INPUTS[cache_key] = result
    return result


def music_batch_dependencies(batch: tuple[str, list[str]]) -> dict:
    """Read one cue batch with one instrument scan."""
    from ambition_music_renderer.cli import find_score
    from ambition_music_renderer.render.generated_layout import compute_score_render_dependencies
    from ambition_music_renderer.render.dependencies import dependency_scan
    backend, cues = batch
    dependencies = {}
    with dependency_scan():
        for cue in cues:
            dependencies[cue] = compute_score_render_dependencies(find_score(cue), backend).fingerprint
            print(f"music dependencies: {cue}", flush=True)
    return dependencies


def owner_dependencies(family: str) -> dict:
    if family not in _DEPENDENCIES:
        dependencies = {}
        if family == "music":
            import regen_music_registry as registry
            backend = os.environ.get("AMBITION_MUSIC_BACKEND", "pretty-midi")
            cues = registry.generation_cues()
            jobs = max(1, min(len(cues), int(os.environ.get("AMBITION_MUSIC_JOBS", "6"))))
            batches = [(backend, cues[index::jobs]) for index in range(jobs)]
            if jobs == 1:
                dependencies = music_batch_dependencies(batches[0])
            else:
                with ProcessPoolExecutor(max_workers=jobs) as workers:
                    for result in workers.map(music_batch_dependencies, batches):
                        dependencies.update(result)
        elif family == "sfx":
            from ambition_sfx_renderer.schema import iter_cue_files, load_cue
            from ambition_sfx_renderer.render import cue_hash
            collection = library().manifest["producers"][family]["collection"]
            dependencies = {load_cue(path).cue_id: cue_hash(path) for path in iter_cue_files(group=collection)}
        _DEPENDENCIES[family] = dependencies
    return _DEPENDENCIES[family]


def evidence(family: str, selected, hashes: Digests, parents=None, scope=None) -> dict:
    inputs = [{"path": os.path.relpath(path, ROOT), "sha256": hashes.file(path)} for path in input_files(family, selected)]
    return generation(inputs, [], {"producer": family, "config": selected.manifest["producers"][family],
                                   "published_parents": parents or {}, "scope": scope,
                                   "owner_dependencies": owner_dependencies(family)}, complete=True)


def current(key: str, family: str, selected, store: Store, hashes: Digests, selection=None) -> bool:
    bundle = store.bundle(key)
    recipe = bundle.get("recipe") or {}
    if recipe.get("owner") != OWNER or recipe.get("producer") != family:
        return False
    previous = recipe["generation"]
    selection = selection or store.selected()
    parents = {unit: selection["bundles"].get(unit) for unit in previous["parameters"].get("published_parents", {})}
    expected = evidence(family, selected, hashes, parents, previous["parameters"].get("scope"))
    return previous["key"] == expected["key"] and not store.integrity(key, hashes=hashes)


def runtime_member(family: str, path: Path, config: dict) -> bool:
    if family == "fonts":
        return path.suffix.lower() in {".ttf", ".otf", ".woff", ".woff2"} or "licenses" in path.parts
    if family == "music":
        return path.suffix.lower() in {".ogg", ".wav", ".flac"} or path.as_posix() == config["registry"]
    return path.suffix.lower() not in {".txt", ".json"} or family == "sfx"


def validate_owner_products(family: str, dependencies: dict) -> None:
    if family == "music":
        from ambition_music_renderer.cli import generated_root, find_latest_manifest, resolve_latest_generated_dir
        for cue, fingerprint in dependencies.items():
            manifest = find_latest_manifest(resolve_latest_generated_dir(generated_root() / cue), cue)
            recorded = json.loads(manifest.read_text()).get("render_dependencies", {}) if manifest else {}
            if recorded.get("fingerprint") != fingerprint:
                raise ValueError(f"Music cue {cue} has no current render. Run generation without --skip-render.")
    elif family == "sfx":
        from ambition_sfx_renderer.schema import iter_cue_files, load_cue
        from ambition_sfx_renderer.render import expected_output_paths
        from ambition_sfx_renderer.paths import output_root
        collection = library().manifest["producers"][family]["collection"]
        for path in iter_cue_files(group=collection):
            cue = load_cue(path)
            manifest = expected_output_paths(cue, output_root())["manifest"]
            if not manifest.is_file() or json.loads(manifest.read_text()).get("hash") != dependencies[cue.cue_id]:
                raise ValueError(f"SFX cue {cue.cue_id} has no current render. Run generation without --skip-render.")


def accept_outputs(family: str, selected, store: Store, staging: Path, before: dict, hashes: Digests, arguments: list[str]) -> None:
    params = before["parameters"]
    validate_owner_products(family, params["owner_dependencies"])
    _INPUTS.clear()
    _DEPENDENCIES.clear()
    if evidence(family, selected, Digests(), params["published_parents"])["key"] != before["key"]:
        raise ValueError("Authored producer inputs changed during generation. The accepted selection was kept.")
    config = selected.manifest["producers"][family]
    owned_roots = [relative(root) for root in config["roots"]]
    files = {}
    by_variant = {}
    for role in ("full", "half", "quarter", "potato"):
        root = staging / "runtime" / role
        for path in sorted(root.rglob("*")):
            if not path.is_file():
                continue
            logical = path.relative_to(root).as_posix()
            if runtime_member(family, Path(logical), config) and not any(logical == prefix or logical.startswith(prefix + "/") for prefix in owned_roots):
                raise ValueError(f"{family} wrote outside its declared roots: {logical}")
            if family == "variants":
                if role != "full" and path.suffix == ".png":
                    by_variant[(role, logical)] = {logical: (path, "runtime", role)}
            elif runtime_member(family, Path(logical), config):
                files[logical] = (path, "runtime", role)
            else:
                files[f"review/{family}/{logical}"] = (path, "review", role)
    records = {path.relative_to(staging / "provenance").as_posix(): path
               for path in (staging / "provenance").rglob("*.json")}
    replacements = {}
    if family == "variants":
        for (role, logical), members in by_variant.items():
            receipt_name = str(Path(logical).parent / (Path(logical).stem + ".quality.json"))
            if receipt_name not in records:
                raise ValueError(f"No conversion receipt for {logical}")
            members["review/provenance/" + receipt_name] = (records[receipt_name], "review", role)
            unit = f"producer:variants:{role}:{logical}"
            scoped = evidence(family, selected, hashes, params["published_parents"], {"role": role, "logical": logical})
            replacements[unit] = store.accept(unit, members, {"owner": OWNER, "producer": family, "generation": scoped}, roots=owned_roots)
    else:
        if not any(role == "runtime" for _, role, _ in files.values()):
            raise ValueError(f"Producer {family} made no runtime products")
        for logical, path in records.items():
            files["review/provenance/" + logical] = (path, "review", "full")
        unit = "producer:" + family
        replacements[unit] = store.accept(unit, files, {"owner": OWNER, "producer": family, "generation": before}, roots=owned_roots)
    accepted = store.selected()
    bundles = dict(accepted["bundles"])
    if family == "variants":
        partial = any(arg in {"--target", "--tier"} or arg.startswith(("--target=", "--tier=")) for arg in arguments)
        if not partial:
            bundles = {unit: key for unit, key in bundles.items() if not unit.startswith("producer:variants:")}
    bundles.update(replacements)
    configured = selected.manifest["producers"]
    bundles = {unit: key for unit, key in bundles.items() if not unit.startswith("producer:") or unit.split(":")[1] in configured}
    if bundles != accepted["bundles"]:
        store.publish(bundles, accepted["generation"], replace=True, hashes=hashes)
    print(f"{family}: accepted {len(replacements)} bundle(s)")


def run(family: str, arguments: list[str]) -> int:
    selected = library()
    if family not in selected.manifest.get("producers", {}):
        raise ValueError(f"The selected library declares no {family} producer")
    if not selected.manifest.get("publication"):
        raise ValueError("A generated content library must declare its publication store")
    store = Store(selected.path.parent / relative(selected.manifest["publication"]))
    hashes = Digests(store.root / "producer-inputs.json")
    if family == "variants":
        if "--asset-root" in arguments or any(arg.startswith("--asset-root=") for arg in arguments):
            return subprocess.call([interpreter(family), str(ROOT / "scripts/generate_visual_quality_variants.py"), *arguments], cwd=ROOT)
        if "--backgrounds-only" not in arguments and "sprites" in selected.manifest["producers"]:
            status = subprocess.call([interpreter(family), str(ROOT / "scripts/generate_visual_quality_variants.py"), "--sprites-only", *arguments], cwd=ROOT)
            if status:
                return status
        if "--sprites-only" in arguments or "backgrounds" not in selected.manifest["producers"]:
            return 0
        if "producer:backgrounds" not in store.selected()["bundles"]:
            status = subprocess.call([str(ROOT / "scripts/regen/backgrounds.sh")], cwd=ROOT,
                                     env={key: value for key, value in os.environ.items() if key != "AMBITION_PRODUCER_OWNER"})
            if status:
                return status
    unit = "producer:" + family
    old = store.selected()["bundles"].get(unit)
    force = any(arg in {"--force", "--clean"} for arg in arguments)
    if family in {"fonts", "backgrounds", "music", "sfx"} and not force and old and current(old, family, selected, store, hashes):
        print(f"{family}: accepted products are current")
        hashes.save()
        return 0
    parents = {}
    if family == "variants":
        parents["producer:backgrounds"] = store.selected()["bundles"]["producer:backgrounds"]
    before = evidence(family, selected, hashes, parents)
    with tempfile.TemporaryDirectory(prefix=f"ambition-{family}-") as temporary:
        staging = Path(temporary)
        env = dict(os.environ, AMBITION_PRODUCER_WORKER=family,
                   AMBITION_GENERATION_ROOT=str(staging / "runtime/full"),
                   AMBITION_ART_PROVENANCE_ROOT=str(staging / "provenance"))
        env.pop("AMBITION_MUSIC_PUBLISH_ROOT", None)
        command = [str(ROOT / SCRIPTS[family]), *arguments]
        if family == "fonts":
            command.insert(0, sys.executable)
        if family == "variants":
            source = store.root / "bundles" / parents["producer:backgrounds"] / "runtime/full"
            env["AMBITION_ART_PUBLISHED_PARENTS"] = json.dumps(parents)
            command += ["--backgrounds-only", "--source-root", str(source)]
        status = subprocess.call(command, cwd=ROOT, env=env)
        if status:
            return status
        accept_outputs(family, selected, store, staging, before, hashes, arguments)
    hashes.save()
    return 0


def check(family: str) -> int:
    selected = library()
    if family == "sprites":
        return subprocess.call([str(ROOT / "scripts/regen/sprites.sh"), "--check"], cwd=ROOT)
    if family == "variants" and "backgrounds" not in selected.manifest["producers"]:
        return subprocess.call([sys.executable, str(ROOT / "scripts/check_quality_variants_are_fresh.py")], cwd=ROOT)

    store = Store(selected.path.parent / relative(selected.manifest["publication"]))
    hashes = Digests(store.root / "producer-inputs.json")
    selection = store.selected()
    keys = {unit: key for unit, key in selection["bundles"].items()
            if unit == "producer:" + family or unit.startswith("producer:" + family + ":")}
    failures = [unit for unit, key in keys.items() if not current(key, family, selected, store, hashes, selection)]
    if not keys:
        failures.append("missing producer publication")
    if family == "variants":
        expected = {f"producer:variants:{role}:{logical.replace('backgrounds/parallax_layers/', 'backgrounds/parallax_layers_' + suffix + '/', 1)}"
                    for logical in selected.files() if logical.startswith("backgrounds/parallax_layers/") and logical.endswith(".png")
                    for role, suffix in (("half", "0_5x"), ("quarter", "0_25x"), ("potato", "potato"))}
        failures += sorted(expected - keys.keys())
    print(f"{family}: {len(keys)} accepted bundle(s), {len(failures)} missing or stale")
    for failure in failures[:20]:
        print("  " + failure)
    hashes.save()
    return int(bool(failures))


def main() -> int:
    arguments = sys.argv[1:]
    if not arguments or arguments == ["--help"]:
        print("generate_library_content.py <fonts|backgrounds|variants|music|sfx> [options] | --check [family]")
        return 0
    for index, argument in enumerate(arguments):
        if argument == "--library":
            os.environ["AMBITION_CONTENT_LIBRARY"] = str((ROOT / arguments[index + 1]).resolve())
        elif argument.startswith("--library="):
            os.environ["AMBITION_CONTENT_LIBRARY"] = str((ROOT / argument.split("=", 1)[1]).resolve())
    if arguments == ["--check"]:
        selected = library()
        return int(any([subprocess.call([sys.executable, __file__, "--check", family], cwd=ROOT,
                                       env={key: value for key, value in os.environ.items() if key != "AMBITION_PRODUCER_OWNER"})
                       for family in selected.manifest["producers"]]))
    checking = bool(arguments and arguments[0] == "--check")
    family = arguments[1] if checking else arguments[0]
    in_owner(family, arguments)
    try:
        return check(family) if checking else run(family, arguments[1:])
    except (ValueError, OSError, KeyError) as error:
        print(f"{family}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
