"""`docs/sdk/api-reference.md` must match the public builder surface, both ways.

⚠ **Both ways is the point.** A reference that omits a method sends a reader to
`cargo doc` — which is what blind run 4 did, for both of its engine opens, on
the README's own recommendation. A reference that names a method which no longer
exists is worse: the reader writes it, it does not compile, and they trust
nothing else on the page.

This is the fourth guard in the SDK's staleness family, and the family exists
because prose failed three times in four blind runs. The others check that named
modules exist and that promised modules are documented; this one checks the
methods.

## Why the reference exists at all

ADR 0031's gate is that an author never opens a file under `crates/`. The SDK
was recommending `cargo doc -p ambition -p ambition_world`, so its own advice
generated the failures it is scored on. Rustdoc is genuinely better for
browsing; it is not a substitute for the SDK containing the surface.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
)

APP_RS = REPO / "crates/ambition/src/app.rs"
REFERENCE = REPO / "docs/sdk/api-reference.md"

# Methods deliberately absent from the reference, each with a reason. Anything
# not listed here must be documented — the default is "public means documented".
INTENTIONALLY_UNDOCUMENTED = {
    # `#[doc(hidden)]`, and documenting it would be making the promise ADR 0031
    # reserves for its own slice.
    "unstable_rollback_session",
}


def public_methods(type_name: str) -> set[str]:
    """`pub fn` names in `impl <type_name> { … }`."""
    source = APP_RS.read_text(encoding="utf-8")
    start = source.index(f"impl {type_name} {{")
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                block = source[start : index + 1]
                break
    return set(re.findall(r"pub fn ([a-z_][a-z0-9_]*)", block))


def referenced() -> str:
    return REFERENCE.read_text(encoding="utf-8")


def test_the_parser_finds_a_real_surface():
    """Non-vacuity: an empty method set makes every assertion below pass."""
    methods = public_methods("PlatformerApp")
    assert len(methods) > 5, sorted(methods)
    assert "mount" in methods, sorted(methods)


def test_every_public_builder_method_is_documented():
    text = referenced()
    missing: dict[str, list[str]] = {}
    for type_name in ("PlatformerApp", "ModuleDraft", "HostStatus"):
        absent = sorted(
            method
            for method in public_methods(type_name)
            if method not in INTENTIONALLY_UNDOCUMENTED and f"`{method}(" not in text
        )
        if absent:
            missing[type_name] = absent
    assert not missing, (
        f"public methods the SDK reference does not name: {missing}. A reader "
        "who cannot find a method in docs/sdk/ goes to `cargo doc` over an "
        "engine crate, which is the failure ADR 0031's blind-agent gate "
        "measures. Add it, or add it to INTENTIONALLY_UNDOCUMENTED with a "
        "reason."
    )


def test_the_reference_names_no_method_that_does_not_exist():
    """The other direction, and the more damaging one."""
    text = referenced()
    real = (
        public_methods("PlatformerApp")
        | public_methods("ModuleDraft")
        | public_methods("HostStatus")
        # Named in the reference and defined outside those impls: the free
        # function, the constructors, the `GameModule` trait methods, and
        # Bevy's own `App::update` — which the reference mentions because "one
        # update() is one sim tick" is a fact about the headless face.
        | {
            "host_status",
            "new",
            "at",
            "define",
            "manifest",
            "asset_source",
            "update",
        }
    )
    named = set(re.findall(r"`([a-z_][a-z0-9_]*)\(", text))
    invented = sorted(named - real)
    assert not invented, (
        f"the SDK reference names methods that do not exist: {invented}. A "
        "reader writes them, they do not compile, and nothing else on the page "
        "is trusted afterwards."
    )
