"""Prerequisite D: an optional capability must not register rollback state unconditionally.

⭐⭐ THE CONTRACT, from the architecture program: *"Capability A composed -> A's
rollback declarations participate. Capability B absent -> B's rollback state is
never registered or expected."* The runtime keeps one list of domain registration
calls, and a capability that can be compiled out must have its line compiled out
with it, or an App without the capability declares state nothing will ever produce.

⛔⛔ AND THE MEASUREMENT SAYS THIS IS NOT A BUG TODAY, WHICH IS WHY IT IS A GUARD
AND NOT A REPAIR. Measured 2026-09-06: the runtime has exactly THREE optional
dependencies (`ldtk`, `causal`, `portal2d`), only one of them declares rollback
state, and that one is already `#[cfg(feature = "portal")]`-gated. The registration
list is consistent with the composition surface as it stands.

⚠ WHAT MAKES IT WORTH GUARDING IS THE PRESSURE, NOT THE STATE. **Eight** of the
domains in that list are already optional dependencies in OTHER manifests —
`portal2d` in seven, `persistence` and `cutscene` in two each, `vfx`,
`sim_view`, `projectiles`, `items` and `encounter` in one. The runtime is the
lagging manifest, so the next capability to become optional there is the one that
breaks the contract, and it will do so by OMISSION — a line that was correct
before the dependency changed under it.

⇒ THE ARCHITECTURE DECISION THIS ENCODES, stated so it can be argued with rather
than rediscovered: **composition here is COMPILE-TIME, and that is a choice with a
reason.** Storing per-domain registration callbacks for later replay — the obvious
route to runtime composition — does not work: `RollbackRegistrar` has twenty
GENERIC methods and is not object-safe, so `Vec<fn(&mut impl RollbackRegistrar)>`
cannot exist. Type-erasing the operation instead of the registrar fails for a
second reason worth writing down, because it is not obvious: the thunk that would
apply a registration needs the backend's `AmbitionRollbackApp` extension trait in
scope, and that lives in the backend crate — so building thunks inside domain
crates would make every domain depend on a rollback backend, giving back exactly
the property the `GgrsBackendPlugin` / `AmbitionRollbackPlugin` split bought.
⇒ Cargo features are the composition granularity that keeps domains
backend-neutral. The condition that reopens this: **one binary needing to compose
different capability sets at RUNTIME** — at which point the registrar has to be
redesigned around erased descriptors plus a backend-provided applier, and that is
a real project rather than a refactor.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
RUNTIME = REPO / "crates" / "ambition_platformer2d_runtime"
MANIFEST = RUNTIME / "Cargo.toml"
ROLLBACK = RUNTIME / "src" / "rollback" / "mod.rs"

OPTIONAL_DEP = re.compile(r"^([a-z_0-9]+)\s*=\s*\{[^}]*optional\s*=\s*true", re.MULTILINE)


def _optional_dependencies() -> set[str]:
    return set(OPTIONAL_DEP.findall(MANIFEST.read_text(encoding="utf-8")))


def _registration_lines() -> list[tuple[int, str, bool]]:
    """(line number, crate, is the line preceded by a cfg gate)."""
    lines = ROLLBACK.read_text(encoding="utf-8").split("\n")
    found: list[tuple[int, str, bool]] = []
    for i, line in enumerate(lines):
        match = re.search(r"^\s*([a-z_0-9]+)::(?:\w+::)*register_\w*rollback_state\(", line)
        if not match:
            continue
        # A gate sits on the line above, or on the one above that with a comment
        # between — look back past comments only.
        gated = False
        j = i - 1
        while j >= 0 and (lines[j].strip().startswith("//") or not lines[j].strip()):
            j -= 1
        if j >= 0 and "cfg(feature" in lines[j]:
            gated = True
        found.append((i + 1, match.group(1), gated))
    return found


def test_every_optional_capability_gates_its_rollback_registration() -> None:
    """⛔ THE CONTRACT. A capability that can be compiled out must have its
    registration compiled out with it, or an App without it declares state
    nothing will produce — which is a missing-resource panic waiting for the
    first composition that leaves it out."""
    optional = _optional_dependencies()
    offenders = [
        f"{ROLLBACK.relative_to(REPO)}:{line}  {crate}"
        for line, crate, gated in _registration_lines()
        if crate in optional and not gated
    ]
    assert not offenders, (
        "these crates are OPTIONAL dependencies of the runtime and register "
        "rollback state unconditionally, so a build without them declares state "
        "nothing will ever produce:\n  " + "\n  ".join(offenders)
    )


def test_the_guard_can_see_the_registration_list_at_all() -> None:
    """⭐⭐ THE POSITIVE CONTROL, and it is not decoration here.

    The assertion above is a filter over an intersection: optional dependencies
    THAT ALSO register. Either half reading empty makes it pass, and both halves
    are parsed out of files that move — a renamed function, a restructured
    manifest, a registration list that grows a wrapper. ⇒ A guard whose subject
    can silently become the empty set is a guard that reports success for the one
    reason nobody wants."""
    lines = _registration_lines()
    assert len(lines) >= 15, (
        f"only {len(lines)} rollback registrations found; the list held 21 when "
        "this was written, so the parser has stopped matching rather than the "
        "list having shrunk by six"
    )
    optional = _optional_dependencies()
    assert optional, "no optional dependencies parsed from the runtime manifest"
    assert any(crate in optional for _, crate, _ in lines), (
        "no optional dependency appears in the registration list, so the "
        "intersection the test above filters is empty and it cannot fail"
    )
    assert any(gated for _, _, gated in lines), (
        "no registration line is cfg-gated, so the gate detector has stopped "
        "recognising the one shape it was written against (`portal`)"
    )
