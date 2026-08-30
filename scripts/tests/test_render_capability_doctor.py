"""The doctor must distinguish the two ways to have no adapter.

⛔⛔ A REVIEW SPENT ITS HEADLINE FINDING ON THIS QUESTION AND GOT IT WRONG: it
read `moveset_render`'s failure message, concluded the VM had no Vulkan ICD, and
recommended installing `mesa-vulkan-drivers` — which was already installed, on a
machine where the offscreen pipeline produced real engine PNGs. A one-second
answer is the whole point of the tool, so the arms are about what it SAYS, not
about this machine's own state.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "render_capability_doctor", REPO / "scripts/render_capability_doctor.py"
)
doctor = importlib.util.module_from_spec(_spec)
sys.modules["render_capability_doctor"] = doctor
_spec.loader.exec_module(doctor)


def _report(monkeypatch, *, loader, icds):
    monkeypatch.setattr(doctor, "loader", lambda: loader)
    monkeypatch.setattr(doctor, "icds", lambda: [Path(f"/x/{n}") for n in icds])
    return doctor.report()


def test_a_loader_with_no_icd_names_the_package_to_install(monkeypatch):
    data = _report(monkeypatch, loader="libvulkan.so.1", icds=[])
    assert data["offscreen_capture"] == "unavailable"
    assert "mesa-vulkan-drivers" in data["hint"]
    # ⛔ AND IT SAYS THE LOADER IS THERE. "no Vulkan" would send a reader to
    # install the loader, which is the half that already works.
    assert data["vulkan_loader"] == "libvulkan.so.1"


def test_no_loader_at_all_is_a_different_answer(monkeypatch):
    data = _report(monkeypatch, loader=None, icds=["lvp_icd.json"])
    assert data["offscreen_capture"] == "unavailable"
    assert "loader" in data["hint"]


def test_lavapipe_alone_is_enough(monkeypatch):
    # OffscreenGpu creates no window and disables winit, so a software adapter
    # qualifies — no physical GPU and no Xvfb.
    data = _report(monkeypatch, loader="libvulkan.so.1", icds=["lvp_icd.json"])
    assert data["offscreen_capture"] == "likely"
    assert data["software_adapter"] == ["lvp_icd.json"]


def test_it_refuses_to_claim_more_than_it_looked_at(monkeypatch):
    """⛔ AN ICD ON DISK IS NECESSARY AND NOT SUFFICIENT — a driver can refuse."""
    data = _report(monkeypatch, loader="libvulkan.so.1", icds=["radeon_icd.json"])
    assert data["offscreen_capture"] == "likely", "hardware ICDs count too"
    assert "no adapter was created" in data["checked"]
    assert "authoritative" in data["hint"]
