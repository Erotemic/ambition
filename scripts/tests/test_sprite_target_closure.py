"""Which renderer files each sprite target's cache key reads (`sprite_target_closure.py`).

A small renderer tree stands in for the real one: two families of helpers, a
helper package that loads its modules by name, a shared module with no
underscore, a module that defines two targets, a config target, and a core
module that imports one helper.
"""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))

import sprite_target_closure as closure_of  # noqa: E402

PKG = "ambition_sprite2d_renderer"


@pytest.fixture
def renderer(tmp_path: Path) -> Path:
    files = {
        f"{PKG}/__init__.py": "",
        f"{PKG}/core/__init__.py": "",
        f"{PKG}/core/draw.py": "from ..targets.fam import _shared_by_core\n",
        f"{PKG}/targets/__init__.py": "",
        f"{PKG}/targets/common.py": "X = 1\n",
        f"{PKG}/targets/fam/__init__.py": "",
        f"{PKG}/targets/fam/_base.py": "",
        f"{PKG}/targets/fam/_rig_a.py": "from . import _base\n",
        f"{PKG}/targets/fam/_rig_b.py": "",
        f"{PKG}/targets/fam/_shared_by_core.py": "",
        f"{PKG}/targets/fam/_presets/__init__.py": (
            "import importlib, pkgutil\n"
            "for info in pkgutil.iter_modules(__path__):\n"
            "    importlib.import_module('.' + info.name, __name__)\n"
        ),
        f"{PKG}/targets/fam/_presets/one.py": "",
        f"{PKG}/targets/fam/alpha.py": "from ._rig_a import thing\nfrom ...core import draw\n",
        f"{PKG}/targets/fam/beta.py": f"from {PKG}.targets.fam._rig_b import thing\n",
        f"{PKG}/targets/fam/crew.py": "from . import _rig_b\nTARGETS = {'kim': 1, 'lee': 2}\n",
        f"{PKG}/targets/fam/gamma.py": "from . import _presets\n",
        f"{PKG}/targets/fam/delta.py": "from ..common import X\n",
        f"{PKG}/configs/cfg.yaml": "name: cfg\n",
    }
    for rel, text in files.items():
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    closure_of._imports.cache_clear()
    return tmp_path


def module(name: str) -> SimpleNamespace:
    return SimpleNamespace(module_path=f"{PKG}.targets.fam.{name}", _config_path=None)


def files_of(renderer: Path, registry: dict, name: str) -> set[str]:
    roots = closure_of.target_roots(renderer, registry)
    targets = renderer / PKG / "targets"
    return {
        str(file.relative_to(targets)) if targets in file.parents else str(file.relative_to(renderer))
        for file in closure_of.closure(renderer, roots[name])
    }


def test_a_target_reads_the_helpers_it_imports_and_no_others(renderer: Path) -> None:
    """The defect: every `_*.py` helper was in the shared key, so an edit to
    one family's rig made every sheet stale (about an hour of rendering)."""
    registry = {"alpha": module("alpha"), "beta": module("beta")}
    alpha = files_of(renderer, registry, "alpha")
    beta = files_of(renderer, registry, "beta")
    assert {"fam/alpha.py", "fam/_rig_a.py", "fam/_base.py"} <= alpha, alpha
    assert "fam/_rig_b.py" not in alpha, "alpha reads beta's rig"
    assert {"fam/beta.py", "fam/_rig_b.py"} <= beta, beta
    assert not {"fam/_rig_a.py", "fam/_base.py"} & beta, "beta reads alpha's rig"


def test_each_target_of_one_module_reads_that_module(renderer: Path) -> None:
    """A module can define several targets (the crew), so a target name is not
    a file name: `kim` is made of `crew.py`."""
    registry = {"kim": module("crew"), "lee": module("crew")}
    for name in ("kim", "lee"):
        assert {"fam/crew.py", "fam/_rig_b.py"} <= files_of(renderer, registry, name)


def test_a_package_that_loads_modules_by_name_brings_them_all(renderer: Path) -> None:
    """`_presets/one.py` is named by no import statement; it was in no key."""
    assert "fam/_presets/one.py" in files_of(renderer, {"gamma": module("gamma")}, "gamma")


def test_a_shared_module_with_no_underscore_is_read(renderer: Path) -> None:
    assert "common.py" in files_of(renderer, {"delta": module("delta")}, "delta")


def test_a_config_target_is_its_config_file(renderer: Path) -> None:
    config = SimpleNamespace(module_path=None, _config_path=str(renderer / PKG / "configs" / "cfg.yaml"))
    assert files_of(renderer, {"cfg": config}, "cfg") == {f"{PKG}/configs/cfg.yaml"}


def test_the_core_brings_the_helpers_it_imports_into_the_shared_key(renderer: Path) -> None:
    """The core runs for every target, so what it imports is everyone's."""
    targets = renderer / PKG / "targets"
    reached = {
        str(file.relative_to(targets))
        for file in closure_of.closure(renderer, closure_of.core_files(renderer))
        if targets in file.parents
    }
    assert reached == {"fam/_shared_by_core.py", "fam/__init__.py"}, reached


def test_a_file_that_does_not_parse_is_an_error(renderer: Path) -> None:
    (renderer / PKG / "targets" / "fam" / "_rig_a.py").write_text("def (:\n")
    with pytest.raises(SyntaxError):
        files_of(renderer, {"alpha": module("alpha")}, "alpha")
