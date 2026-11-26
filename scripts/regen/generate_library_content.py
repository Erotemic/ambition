#!/usr/bin/env python3
"""Stage and publish the selected library's generated media."""
from __future__ import annotations

import sys as _sys
from pathlib import Path as _Path
_sys.path.insert(0, str(_Path(__file__).resolve().parents[2]))
_sys.path.insert(0, str(_Path(__file__).resolve().parents[2] / "scripts"))

from concurrent.futures import ProcessPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile

from lib.content_libraries import selected_manifest, read_libraries, relative

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.build_selection import BuildMode
from ambition_sprite2d_renderer.publication_store import Store
from ambition_sprite2d_renderer.products import Digests, generation

OWNER = "ambition.library_producer.v1"
TOOLS = {"backgrounds": "ambition_sprite2d_renderer", "variants": "ambition_sprite2d_renderer",
         "music": "ambition_music_renderer", "sfx": "ambition_sfx_renderer"}
SCRIPTS = {"backgrounds": "scripts/regen/backgrounds.sh", "variants": "scripts/regen/quality_variants.sh",
           "music": "scripts/regen/music.sh", "sfx": "scripts/regen/sfx.sh", "fonts": "scripts/regen/grab_font_assets.py"}
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
        renderer = ROOT / "tools" / TOOLS[family]
        # Use the same renderer files for imports and publication hashes.
        entry = '''
audio_env="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}/env.sh"
if [ -f "$audio_env" ]; then source "$audio_env"; fi
export PYTHONPATH="$1${PYTHONPATH:+:$PYTHONPATH}"
shift
exec "$@"
'''
        os.execvpe("bash", ["bash", "-c", entry, "producer", str(renderer),
                            interpreter(family), __file__, *arguments], env)
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
        from scripts.regen import regen_music_registry as registry
        from ambition_music_renderer.cli import find_score
        for cue in registry.generation_cues():
            path = find_score(cue)
            if path is None:
                raise ValueError(f"Music cue has no authored score: {cue}")
            files.add(path)
        files.add(ROOT / "scripts/regen/regen_music_registry.py")
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
            print(f"music dependencies: {cue}", file=sys.stderr, flush=True)
    return dependencies


def owner_dependencies(family: str, cues=None) -> dict:
    key = (family, tuple(cues) if cues is not None else None)
    if key not in _DEPENDENCIES:
        dependencies = {}
        if family == "music":
            from scripts.regen import regen_music_registry as registry
            backend = os.environ.get("AMBITION_MUSIC_BACKEND", "pretty-midi")
            cues = registry.generation_cues() if cues is None else cues
            if not cues:
                return {}
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
        _DEPENDENCIES[key] = dependencies
        if family == "music":
            for cue, fingerprint in dependencies.items():
                _DEPENDENCIES[(family, (cue,))] = {cue: fingerprint}
    return _DEPENDENCIES[key]


def evidence(family: str, selected, hashes: Digests, parents=None, scope=None) -> dict:
    inputs = [{"path": os.path.relpath(path, ROOT), "sha256": hashes.file(path)} for path in input_files(family, selected)]
    return generation(inputs, [], {"producer": family, "config": selected.manifest["producers"][family],
                                   "published_parents": parents or {}, "scope": scope,
                                   "owner_dependencies": owner_dependencies(family, scope.get("cues") if family == "music" and scope else None)}, complete=True)


def current(key: str, family: str, selected, store: Store, hashes: Digests, selection=None) -> bool:
    return published_status(key, family, selected, store, hashes, selection or store.selected())[0] == "current"


MISSING_FLAGS = {"--only-missing", "--missing-only"}
FORCE_FLAGS = {"--force", "--force-render", "--clean"}
SELECTION_FLAGS = MISSING_FLAGS | FORCE_FLAGS | {"--loose", "--only-possibly-stale"}


def build_mode(arguments) -> BuildMode:
    flags = set(arguments)
    mode = BuildMode.from_flags(loose="--loose" in flags, missing_only=bool(flags & MISSING_FLAGS),
                                possibly_stale_only="--only-possibly-stale" in flags, force=bool(flags & FORCE_FLAGS))
    if "--include-shared" in flags and mode in {BuildMode.LOOSE, BuildMode.POSSIBLY_STALE, BuildMode.MISSING}:
        raise ValueError(f"--include-shared and --{mode.value} cannot be combined")
    return mode


def published_status(key, family, selected, store, hashes, selection) -> tuple[str, str | None]:
    """Classify staleness using the recipe saved with the accepted unit."""
    if not key:
        return "missing", None
    defects = store.integrity(key, hashes=hashes)
    if defects:
        return ("partial" if any(row["code"] == "missing" for row in defects) else "stale"), "direct"
    recipe = store.bundle(key).get("recipe") or {}
    previous = recipe.get("generation")
    if recipe.get("owner") != OWNER or recipe.get("producer") != family or not previous:
        return "unknown", None
    params = previous["parameters"]
    parents = {unit: selection["bundles"].get(unit) for unit in params.get("published_parents", {})}
    expected = evidence(family, selected, hashes, parents, params.get("scope"))
    if previous["key"] == expected["key"]:
        return "current", None
    if params != expected["parameters"]:
        return "stale", "direct"
    own = set()
    policy_path = params["config"].get("policy")
    if policy_path:
        own.add(os.path.relpath(selected.path.parent / policy_path, ROOT))
    old_inputs = {row["path"]: row["sha256"] for row in previous["inputs"]}
    new_inputs = {row["path"]: row["sha256"] for row in expected["inputs"]}
    changed = {path for path in old_inputs.keys() | new_inputs.keys() if old_inputs.get(path) != new_inputs.get(path)}
    if family == "backgrounds":
        own.update(path for path in changed if path.startswith("tools/ambition_background_renderer/") or (
            path.startswith("tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/")
            and Path(path).parts[3] in {"backgrounds", "terrain"}
        ))
    return "stale", "direct" if changed & own else "shared"


def show_selection(family, states, arguments, *, checking=False) -> int:
    mode = build_mode(arguments)
    if checking:
        mode.check()
    pending = [unit for unit, status in states.items() if mode.includes(*status)]
    if "--json" in arguments:
        print(json.dumps({"family": family, "states": {unit: {"state": state, "scope": scope}
                          for unit, (state, scope) in states.items()}, "would_build": pending}, indent=1))
    else:
        for unit, (state, scope) in states.items():
            print(f"{'*' if unit in pending else ' '} {unit}: {'possibly stale' if scope == 'shared' else state}")
        print(f"{family}: {len(pending)} selected for generation")
    return int(checking and bool(pending))

def selected_music_cues(arguments: list[str]) -> list[str]:
    from scripts.regen import regen_music_registry as registry
    requested = []
    for index, argument in enumerate(arguments):
        if argument == "--cue":
            if index + 1 == len(arguments):
                raise ValueError("--cue needs a comma-separated cue list")
            requested.extend(arguments[index + 1].split(","))
        elif argument.startswith("--cue="):
            requested.extend(argument.split("=", 1)[1].split(","))
    available = registry.generation_cues()
    if not requested:
        return available
    unknown = set(requested) - set(available)
    if unknown:
        raise ValueError("Unknown library music cues: " + ", ".join(sorted(unknown)))
    return sorted(set(requested))


MUSIC_STATES = ("missing", "stale", "damaged", "shared", "current")
MUSIC_STATE_WORDS = {
    "missing": "never published",
    "stale": "its own score, instruments or render code changed",
    "damaged": "its published files fail their check",
    "shared": "only shared inputs changed (another score, the tool)",
    "current": "published from today's inputs",
}


def music_cue_states(selected, store: Store, hashes: Digests, cues: list[str], selection) -> dict[str, str]:
    """Each cue's state against its published bundle.

    A cue's own key is its render fingerprint (its score, the instruments it
    names, the render code), recorded in its bundle. The bundle's whole key
    also covers every other score and the tool tree, so an edit to one score
    leaves the others `shared`, not `stale`.
    """
    fingerprints = owner_dependencies("music", cues) if cues else {}
    states = {}
    for cue in cues:
        key = selection["bundles"].get("producer:music:cue:" + cue)
        if not key:
            states[cue] = "missing"
            continue
        recipe = store.bundle(key).get("recipe") or {}
        recorded = (recipe.get("generation") or {}).get("parameters", {}).get("owner_dependencies", {}).get(cue)
        if json.loads(json.dumps(fingerprints.get(cue))) != recorded:
            states[cue] = "stale"
        elif store.integrity(key, hashes=hashes):
            states[cue] = "damaged"
        elif current(key, "music", selected, store, hashes, selection):
            states[cue] = "current"
        else:
            states[cue] = "shared"
    return states


def music_query(arguments: list[str], selected, store: Store, hashes: Digests) -> int:
    """`--list`, `--plan` and `--check`: what is there and what a run would render."""
    cues = selected_music_cues(arguments)
    as_json = "--json" in arguments
    if "--list" in arguments:
        print(json.dumps(cues, indent=1) if as_json else "\n".join(cues))
        return 0
    selection = store.selected()
    states = music_cue_states(selected, store, hashes, cues, selection)
    registry = store_music_registry_status(arguments, selected, store, hashes, selection)
    hashes.save()
    checking = "--check" in arguments
    mode = build_mode(arguments).check() if checking else music_build_mode(arguments)
    render = music_render_selection(states, mode)
    refresh_registry = refresh_music_registry(mode, registry, bool(render) and not checking)
    if as_json:
        print(json.dumps({"states": states, "would_render": render, "would_build": render,
                          "registry": registry, "would_refresh_registry": refresh_registry}, indent=1, sort_keys=True))
    else:
        width = max((len(cue) for cue in cues), default=0)
        for cue in cues:
            mark = "*" if cue in render else " "
            print(f"{mark} {cue:<{width}}  {states[cue]}")
        counts = ", ".join(f"{sum(state == name for state in states.values())} {name}" for name in MUSIC_STATES
                           if name in states.values())
        print(f"music: {len(cues)} cue(s): {counts}")
        for name in MUSIC_STATES:
            if name in states.values():
                print(f"  {name}: {MUSIC_STATE_WORDS[name]}")
        print(f"a run would render {len(render)} (*)" + ("" if render else ": nothing to do"))
        if registry is not None:
            state, scope = registry
            print(f"{'*' if refresh_registry else ' '} registry: {'possibly stale' if scope == 'shared' else state}")
    return int(checking and bool(render or refresh_registry))


def music_build_mode(arguments) -> BuildMode:
    mode = build_mode(arguments)
    if mode == BuildMode.DEFAULT and "--include-shared" not in arguments:
        return BuildMode.LOOSE
    return mode


def music_render_selection(states: dict[str, str], mode: BuildMode) -> list[str]:
    return [cue for cue, state in states.items()
            if mode.includes("stale" if state == "shared" else state, "shared" if state == "shared" else None)]


def store_music_registry_status(arguments, selected, store, hashes, selection):
    if any(arg == "--cue" or arg.startswith("--cue=") for arg in arguments):
        return None
    return published_status(selection["bundles"].get("producer:music:registry"), "music", selected, store, hashes, selection)


def refresh_music_registry(mode: BuildMode, status, rendering: bool) -> bool:
    return status is not None and (mode.includes(*status) or (rendering and mode != BuildMode.POSSIBLY_STALE))


def accept_music(selected, store: Store, files: dict, records: dict, before: dict, hashes: Digests) -> dict:
    cues = before["parameters"]["scope"]["cues"]
    registry = selected.manifest["producers"]["music"]["registry"]
    groups = {cue: {} for cue in cues}
    registry_files = {}
    for name, item in files.items():
        logical = name.removeprefix("review/music/")
        if logical == registry:
            registry_files[name] = item
            continue
        parts = Path(logical).parts
        if parts[:3] != ("audio", "music", "generated") or len(parts) < 5 or parts[3] not in groups:
            raise ValueError(f"Music wrote outside its selected cues: {logical}")
        groups[parts[3]][name] = item
    if registry not in registry_files:
        raise ValueError("Music made no registry")
    for logical, path in records.items():
        registry_files["review/provenance/" + logical] = (path, "review", "full")
    result = {}
    for cue, members in groups.items():
        prefix = "audio/music/generated/" + cue
        if prefix + "/full.ogg" not in members:
            raise ValueError(f"Music made no full mix for {cue}")
        scoped = evidence("music", selected, hashes, scope={"cues": [cue]})
        unit = "producer:music:cue:" + cue
        result[unit] = store.accept(unit, members, {"owner": OWNER, "producer": "music", "generation": scoped}, roots=[prefix])
    if before["parameters"]["scope"].get("registry", cues == selected_music_cues([])):
        scoped = evidence("music", selected, hashes, scope={"cues": []})
        result["producer:music:registry"] = store.accept("producer:music:registry", registry_files,
            {"owner": OWNER, "producer": "music", "generation": scoped}, roots=[registry])
    return result


def runtime_member(family: str, path: Path, config: dict) -> bool:
    if family == "fonts":
        return path.suffix.lower() in {".ttf", ".otf", ".woff", ".woff2"} or "licenses" in path.parts
    if family == "music":
        return (path.suffix.lower() in {".ogg", ".wav", ".flac"}
                or path.name == f"{path.parent.name}.adaptive_manifest.json"
                or path.as_posix() == config["registry"])
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


def accept_outputs(family: str, selected, store: Store, staging: Path, before: dict, hashes: Digests,
                   arguments: list[str], *, variant_units=None) -> None:
    params = before["parameters"]
    validate_owner_products(family, params["owner_dependencies"])
    _INPUTS.clear()
    _DEPENDENCIES.clear()
    if evidence(family, selected, Digests(), params["published_parents"], params.get("scope"))["key"] != before["key"]:
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
    accepted = store.selected()
    if family == "backgrounds":
        # A scoped scene bundle owns its selected files. Bulk generation can
        # refresh the remaining backgrounds without replacing those bundles.
        files = {logical: item for logical, item in files.items()
                 if accepted["files"].get(logical, {}).get("unit", "").split(":", 1)[0]
                 not in {"theme", "skin", "sky"}}
    replacements = {}
    if family == "variants":
        for (role, logical), members in by_variant.items():
            receipt_name = str(Path(logical).parent / (Path(logical).stem + ".quality.json"))
            if receipt_name not in records:
                raise ValueError(f"No conversion receipt for {logical}")
            members["review/provenance/" + receipt_name] = (records[receipt_name], "review", role)
            unit = f"producer:variants:{role}:{logical}"
            owner = params["scope"]["sources"][logical]
            parents = {owner: params["published_parents"][owner]}
            scoped = evidence(family, selected, hashes, parents, {"role": role, "logical": logical})
            replacements[unit] = store.accept(unit, members, {"owner": OWNER, "producer": family, "generation": scoped},
                                              roots=owned_roots, parents=parents)
        if len(replacements) != len(params["scope"]["sources"]):
            raise ValueError("Variant worker did not generate all selected outputs")
    elif family == "music":
        replacements = accept_music(selected, store, files, records, before, hashes)
    else:
        if not any(role == "runtime" for _, role, _ in files.values()):
            raise ValueError(f"Producer {family} made no runtime products")
        for logical, path in records.items():
            files["review/provenance/" + logical] = (path, "review", "full")
        unit = "producer:" + family
        replacements[unit] = store.accept(unit, files, {"owner": OWNER, "producer": family, "generation": before}, roots=owned_roots)
    bundles = dict(accepted["bundles"])
    if variant_units is not None:
        bundles = {unit: key for unit, key in bundles.items()
                   if not unit.startswith("producer:variants:") or unit in variant_units}
    if family == "music":
        bundles.pop("producer:music", None)
    bundles.update(replacements)
    configured = selected.manifest["producers"]
    bundles = {unit: key for unit, key in bundles.items() if not unit.startswith("producer:") or unit.split(":")[1] in configured}
    if bundles != accepted["bundles"]:
        store.publish(bundles, accepted, replace=True,
                      only_missing=any(arg in {"--only-missing", "--missing-only"} for arg in arguments),
                      refresh_units={"producer:music:registry"} if family == "music" else ())
    print(f"{family}: accepted {len(replacements)} bundle(s)")



def option_values(arguments, option):
    values = []
    for index, argument in enumerate(arguments):
        if argument == option:
            if index + 1 == len(arguments) or arguments[index + 1].startswith("--"):
                raise ValueError(f"{option} requires a value")
            values.append(arguments[index + 1])
        elif argument.startswith(option + "="):
            values.append(argument.split("=", 1)[1])
    return tuple(values)


def parallax_sources(selection):
    return {name: item for name, item in selection["files"].items()
            if name.startswith("backgrounds/parallax_layers/") and name.endswith(".png")}


def validate_variant_arguments(arguments):
    one_value = {"--target", "--tier", "--jobs", "--library", "--asset-root", "--source-root"}
    switches = (SELECTION_FLAGS - {"--force-render"}) | {"--sprites-only", "--backgrounds-only", "--plan", "--check", "--json"}
    values = iter(arguments)
    for argument in values:
        name, equal, value = argument.partition("=")
        if name in one_value:
            if not equal:
                value = next(values, "")
            if not value or value.startswith("--"):
                raise ValueError(f"{name} requires a value")
        elif argument not in switches:
            raise ValueError(f"unknown variant option: {argument}")
    if {"--sprites-only", "--backgrounds-only"} <= set(arguments):
        raise ValueError("--sprites-only and --backgrounds-only cannot be combined")


def variant_plan(selection, arguments):
    from ambition_sprite2d_renderer.quality import resolve_tiers, selects
    selectors = option_values(arguments, "--target")
    tiers = resolve_tiers(option_values(arguments, "--tier"))
    roles = {"0_5x": "half", "0_25x": "quarter", "potato": "potato"}
    plan = {}
    for logical, source in parallax_sources(selection).items():
        name = Path(logical.removeprefix("backgrounds/parallax_layers/")).with_suffix("")
        if not (selects(name.as_posix(), selectors) or selects(name.name, selectors)):
            continue
        for tier in tiers:
            output = logical.replace("backgrounds/parallax_layers/", f"backgrounds/parallax_layers_{tier.suffix}/", 1)
            # The full-size producer replaces its native tiers with the source.
            if selection["files"].get(output, {}).get("unit") == source["unit"]:
                continue
            plan[f"producer:variants:{roles[tier.suffix]}:{output}"] = (logical, source, output)
    return plan


def sprite_variant_states(arguments):
    from ambition_sprite2d_renderer import publication
    from ambition_sprite2d_renderer.quality import selects
    selectors = option_values(arguments, "--target")
    plan = publication.plan("all" if selectors else None)
    if not plan["complete"]:
        raise ValueError("Sprite target discovery did not complete")
    names = {unit["target"] for unit in plan["units"].values()
             if selects(unit["target"], selectors)
             or any(selects(Path(path).stem, selectors) or selects(path, selectors) for path in unit["required"])}
    inventory = publication.inventory(requested=names, include_atlases=not selectors)
    return {unit: (row["status"]["state"], row["status"]["scope"]) for unit, row in inventory["units"].items()}


def variant_states(selected, store, hashes, selection, arguments):
    validate_variant_arguments(arguments)
    if option_values(arguments, "--asset-root") or option_values(arguments, "--source-root"):
        raise ValueError("variant publication queries use the selected library")
    producers = selected.manifest["producers"]
    states = {}
    if "--backgrounds-only" not in arguments and "sprites" in producers:
        states.update(sprite_variant_states(arguments))
    if "--sprites-only" not in arguments and "backgrounds" in producers:
        if not parallax_sources(selection):
            states["source:backgrounds"] = ("missing", None)
        for unit in variant_plan(selection, arguments):
            states[unit] = published_status(selection["bundles"].get(unit), "variants", selected, store, hashes, selection)
    return states


def run_variants(selected, store, hashes, arguments, mode):
    validate_variant_arguments(arguments)
    if os.environ.get("AMBITION_QUALITY_VARIANTS") == "0":
        print("variants: disabled by AMBITION_QUALITY_VARIANTS=0")
        return 0
    if option_values(arguments, "--asset-root") or option_values(arguments, "--source-root"):
        if mode in {BuildMode.LOOSE, BuildMode.POSSIBLY_STALE}:
            raise ValueError(f"--{mode.value} requires a selected library publication")
        return subprocess.call([interpreter("variants"), str(ROOT / "scripts/regen/generate_visual_quality_variants.py"), *arguments], cwd=ROOT)
    producers = selected.manifest["producers"]
    command = [interpreter("variants"), str(ROOT / "scripts/regen/generate_visual_quality_variants.py")]
    if "--backgrounds-only" not in arguments and "sprites" in producers:
        status = subprocess.call([*command, "--sprites-only", *arguments], cwd=ROOT)
        if status:
            return status
    if "--sprites-only" in arguments or "backgrounds" not in producers:
        return 0
    selection = store.selected()
    if not parallax_sources(selection):
        if mode == BuildMode.POSSIBLY_STALE:
            return 0
        status = subprocess.call([str(ROOT / "scripts/regen/backgrounds.sh"),
                                  *(["--only-missing"] if mode == BuildMode.MISSING else [])], cwd=ROOT,
                                 env={key: value for key, value in os.environ.items() if key != "AMBITION_PRODUCER_OWNER"})
        if status:
            return status
        selection = store.selected()
        if not parallax_sources(selection):
            raise ValueError("Background variants require published parallax sources")
    plan = variant_plan(selection, arguments)
    pending = {unit: spec for unit, spec in plan.items()
               if mode.includes(*published_status(selection["bundles"].get(unit), "variants", selected, store, hashes, selection))}
    full = not option_values(arguments, "--target") and not option_values(arguments, "--tier")
    variant_units = set(plan) if full and mode in {BuildMode.DEFAULT, BuildMode.FORCE} else None
    if not pending:
        if variant_units is not None:
            kept = {unit: key for unit, key in selection["bundles"].items()
                    if not unit.startswith("producer:variants:") or unit in variant_units}
            if kept != selection["bundles"]:
                store.publish(kept, selection, replace=True)
        print("variants: no selected background products need generation")
        hashes.save()
        return 0
    parents = {source["unit"]: source["bundle"] for _, source, _ in pending.values()}
    scope = {"sources": {output: source["unit"] for _, source, output in pending.values()}}
    before = evidence("variants", selected, hashes, parents, scope)
    with tempfile.TemporaryDirectory(prefix="ambition-variants-") as temporary:
        staging = Path(temporary)
        source_root = staging / "sources"
        for logical, source, _ in pending.values():
            target = source_root / logical
            if not target.exists():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(store.root / source["path"], target)
        env = dict(os.environ, AMBITION_PRODUCER_WORKER="variants",
                   AMBITION_GENERATION_ROOT=str(staging / "runtime/full"),
                   AMBITION_ART_PROVENANCE_ROOT=str(staging / "provenance"),
                   AMBITION_QUALITY_OUTPUTS=json.dumps(scope["sources"]),
                   AMBITION_ART_PUBLISHED_PARENTS=json.dumps(parents))
        status = subprocess.call([*command, "--backgrounds-only", "--source-root", str(source_root),
                                  *(["--force"] if mode == BuildMode.FORCE else [])], cwd=ROOT, env=env)
        if status:
            return status
        accept_outputs("variants", selected, store, staging, before, hashes, arguments, variant_units=variant_units)
    hashes.save()
    return 0

MUSIC_QUERIES = {"--list", "--plan", "--check"}
MUSIC_RUN_FLAGS = {"--include-shared", "--loose", "--only-possibly-stale", "--json"}


def take_jobs(arguments: list[str]) -> list[str]:
    """`--jobs N` sets the music worker budget (`AMBITION_MUSIC_JOBS`)."""
    rest, index = [], 0
    while index < len(arguments):
        argument = arguments[index]
        if argument == "--jobs":
            if index + 1 == len(arguments):
                raise ValueError("--jobs needs a number")
            os.environ["AMBITION_MUSIC_JOBS"] = str(int(arguments[index + 1]))
            index += 2
            continue
        if argument.startswith("--jobs="):
            os.environ["AMBITION_MUSIC_JOBS"] = str(int(argument.split("=", 1)[1]))
        else:
            rest.append(argument)
        index += 1
    return rest


def accepted_scenery_status(store, selected, hashes):
    from ambition_sprite2d_renderer import publication, scenery_publication
    units = [unit for unit in selected["bundles"] if unit.split(":", 1)[0] in scenery_publication.SCENERY_KINDS]
    if not units:
        return {}
    _, states = scenery_publication.selection(units, store, selected, publication.source_files(),
                                              publication.toolchain(), hashes)
    return states


def run(family: str, arguments: list[str]) -> int:
    if family == "music":
        arguments = take_jobs(arguments)
    mode = build_mode(arguments)
    only_missing = mode == BuildMode.MISSING
    worker_arguments = [arg for arg in arguments if arg not in MISSING_FLAGS | {"--loose", "--only-possibly-stale"}]
    if family == "backgrounds":
        worker_arguments = [arg for arg in worker_arguments if arg != "--force"]
    elif family == "music":
        worker_arguments = [arg for arg in worker_arguments if arg not in MUSIC_RUN_FLAGS]
    # The producer discovers its inputs in its tool's tree: a tool checkout
    # behind its pin would publish old products and call new ones present.
    from lib.submodule_pin import behind_pin
    stale = TOOLS.get(family) and behind_pin(ROOT, "tools/" + TOOLS[family])
    if stale and not os.environ.get("AMBITION_ALLOW_STALE_TOOL"):
        print(f"{family}: refused: {stale}. (AMBITION_ALLOW_STALE_TOOL=1 runs it anyway, "
              "on the old sources.)", file=sys.stderr)
        return 2
    if stale:
        print(f"{family}: AMBITION_ALLOW_STALE_TOOL=1, so running on old sources; a module or score "
              f"the pin added will be missing: {stale}", file=sys.stderr)
    selected = library()
    if family not in selected.manifest.get("producers", {}):
        raise ValueError(f"The selected library declares no {family} producer")
    if not selected.manifest.get("publication"):
        raise ValueError("A generated content library must declare its publication store")
    store = Store(selected.path.parent / relative(selected.manifest["publication"]))
    hashes = Digests(store.root / "producer-inputs.json")
    if family == "variants":
        return run_variants(selected, store, hashes, arguments, mode)
    unit = "producer:" + family
    selection = store.selected()
    if family == "backgrounds":
        from ambition_sprite2d_renderer import scenery_publication
        states = accepted_scenery_status(store, selection, hashes)
        pending = [uid for uid, status in states.items() if mode.includes(*status)]
        if pending:
            scenery_publication.build(pending, force=mode == BuildMode.FORCE,
                                      loose=mode == BuildMode.LOOSE,
                                      possibly_stale_only=mode == BuildMode.POSSIBLY_STALE)
            selection = store.selected()
    scope = {"cues": selected_music_cues(arguments), "registry": not any(
        arg == "--cue" or arg.startswith("--cue=") for arg in arguments)} if family == "music" else None
    if family == "music" and mode != BuildMode.FORCE:
        # Render what changed: a missing cue, or one whose own inputs changed
        # (`--include-shared` adds those only shared inputs made stale), and
        # refresh the registry. `--plan` lists them without rendering.
        states = music_cue_states(selected, store, hashes, scope["cues"], selection)
        mode = music_build_mode(arguments)
        scope["cues"] = music_render_selection(states, mode)
        shared = sorted(cue for cue, state in states.items() if state == "shared" and cue not in scope["cues"])
        if shared:
            print(f"music: {len(shared)} cue(s) stale only through shared inputs, kept "
                  "(--include-shared renders them): " + ", ".join(shared))
        registry_status = store_music_registry_status(arguments, selected, store, hashes, selection)
        scope["registry"] = refresh_music_registry(mode, registry_status, bool(scope["cues"]))
        if not scope["cues"] and not scope["registry"]:
            print("music: no missing products" if only_missing else "music: no cue needs rendering")
            hashes.save()
            return 0
        print("music: rendering " + (", ".join(scope["cues"]) if scope["cues"] else "the registry only"))
        worker_arguments = []
        skip_value = False
        for arg in arguments:
            if skip_value:
                skip_value = False
                continue
            if arg == "--cue":
                skip_value = True
            elif not arg.startswith("--cue=") and arg not in {"--only-missing", "--missing-only"} | MUSIC_RUN_FLAGS:
                worker_arguments.append(arg)
        worker_arguments += ["--cue", ",".join(scope["cues"])]
    elif family in {"fonts", "backgrounds", "sfx"}:
        status = published_status(selection["bundles"].get(unit), family, selected, store, hashes, selection)
        if not mode.includes(*status):
            print(f"{family}: no selected products need generation")
            hashes.save()
            return 0
    before = evidence(family, selected, hashes, scope=scope)
    with tempfile.TemporaryDirectory(prefix=f"ambition-{family}-") as temporary:
        staging = Path(temporary)
        env = dict(os.environ, AMBITION_PRODUCER_WORKER=family,
                   AMBITION_GENERATION_ROOT=str(staging / "runtime/full"),
                   AMBITION_ART_PROVENANCE_ROOT=str(staging / "provenance"))
        env.pop("AMBITION_MUSIC_PUBLISH_ROOT", None)
        command = [str(ROOT / SCRIPTS[family]), *worker_arguments]
        if family == "music" and not scope["cues"]:
            command = [sys.executable, str(ROOT / "scripts/regen/regen_music_registry.py")]
        if family == "fonts":
            command.insert(0, sys.executable)
        status = subprocess.call(command, cwd=ROOT, env=env)
        if status:
            return status
        accept_outputs(family, selected, store, staging, before, hashes, arguments)
    hashes.save()
    return 0


def query(family: str, arguments: list[str]) -> int:
    selected = library()
    store = Store(selected.path.parent / relative(selected.manifest["publication"]))
    hashes = Digests(store.root / "producer-inputs.json")
    if family == "music":
        return music_query(take_jobs(arguments), selected, store, hashes)
    selection = store.selected()
    unit = "producer:" + family
    if family == "variants":
        states = variant_states(selected, store, hashes, selection, arguments)
    else:
        states = {unit: published_status(selection["bundles"].get(unit), family, selected, store, hashes, selection)}
        if family == "backgrounds":
            states.update(accepted_scenery_status(store, selection, hashes))
    hashes.save()
    return show_selection(family, states, arguments, checking="--check" in arguments)


def check(family: str, arguments=None) -> int:
    arguments = ["--check", *(arguments or [])]
    if family == "sprites":
        return subprocess.call([str(ROOT / "scripts/regen/sprites.sh"), *arguments], cwd=ROOT)
    return query(family, arguments)


def main() -> int:
    arguments = sys.argv[1:]
    if not arguments or arguments == ["--help"]:
        print("generate_library_content.py <family> [--plan|--check] [--loose|--only-missing|--only-possibly-stale|--force]")
        return 0
    if arguments[0] in SCRIPTS and any(arg in {"--help", "-h"} for arg in arguments[1:]):
        command = [str(ROOT / SCRIPTS[arguments[0]]), "--help"]
        if arguments[0] == "fonts":
            command.insert(0, sys.executable)
        return subprocess.call(command, cwd=ROOT)
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
        if checking:
            return check(family, arguments[2:])
        flags = arguments[1:]
        queries = set(flags) & MUSIC_QUERIES
        if len(queries) > 1:
            raise ValueError("choose one query from --list, --plan and --check")
        if queries:
            if family != "music" and "--list" in flags:
                raise ValueError("--list is a music cue query; use --plan")
            return query(family, flags)
        return run(family, flags)
    except (ValueError, OSError, KeyError) as error:
        print(f"{family}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
