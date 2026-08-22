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

# # Why the reference exists at all

ADR 0031's gate is that an author never opens a file under `crates/`. The SDK
was recommending `cargo doc -p ambition_platformer2d -p ambition_platformer2d_world`, so its own advice
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

APP_RS = REPO / "crates/ambition_platformer2d/src/app.rs"
ROLLBACK_RS = REPO / "crates/ambition_platformer2d/src/rollback.rs"
REFERENCE = REPO / "docs/sdk/api-reference.md"

# Every public type the reference documents, and the file it is defined in.
DOCUMENTED_TYPES = {
    "PlatformerApp": APP_RS,
    "ModuleDraft": APP_RS,
    "HostStatus": APP_RS,
    "RollbackPlan": ROLLBACK_RS,
    "RollbackSession": ROLLBACK_RS,
    "RollbackHealth": ROLLBACK_RS,
}

# Methods deliberately absent from the reference, each with a reason. Anything
# not listed here must be documented — the default is "public means documented".
# EMPTY as of slice F. Its one entry was `unstable_rollback_session`, hidden
# because documenting it would have made the promise ADR 0031 reserved for its
# own slice. Slice F made that promise properly and the method is gone, so the
# waiver went with it — a waiver outliving its subject is the stale entry this
# file's sibling ratchets exist to forbid.
INTENTIONALLY_UNDOCUMENTED: set[str] = set()


def public_methods(type_name: str) -> set[str]:
    """`pub fn` names in `impl <type_name> { … }`."""
    source = DOCUMENTED_TYPES[type_name].read_text(encoding="utf-8")
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
    for type_name in DOCUMENTED_TYPES:
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
        set().union(*(public_methods(name) for name in DOCUMENTED_TYPES))
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
            # `ambition_platformer2d::rollback`'s free function, and the verbs a consumer
            # calls on its own `App` rather than on a builder type. These live
            # on extension traits in engine crates (`AmbitionRollbackApp`,
            # `SimScheduleExt`), which is exactly why the reference has to name
            # them: a method reachable only through a trait a reader has not
            # imported is one they will go into `crates/` looking for.
            "start",
            "rollback_component_canonical",
            "require_rollback",
            "sim_schedule",
            "health",
        }
    )
    named = set(re.findall(r"`([a-z_][a-z0-9_]*)\(", text))
    invented = sorted(named - real)
    assert not invented, (
        f"the SDK reference names methods that do not exist: {invented}. A "
        "reader writes them, they do not compile, and nothing else on the page "
        "is trusted afterwards."
    )


# Public enums the SDK reference names variants of, and where they are defined.
#
# This is the FOURTH instance of the family's founding failure — a document describing something
# that does not exist — and the third syntactic category it has appeared in (module paths,
# methods, now variants).
DOCUMENTED_ENUMS = {
    "RollbackRefused": ROLLBACK_RS,
    "RollbackHealth": ROLLBACK_RS,
    "HostStatus": APP_RS,
}


def variants(enum_name: str) -> set[str]:
    """Variant names in `pub enum <enum_name> { … }`."""
    source = DOCUMENTED_ENUMS[enum_name].read_text(encoding="utf-8")
    start = source.index(f"pub enum {enum_name} {{")
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                block = source[start : index + 1]
                break
    # Variants are the CamelCase names at the head of a line inside the block,
    # skipping doc comments and attributes.
    return {
        match
        for match in re.findall(r"^\s{4}([A-Z][A-Za-z0-9]*)", block, re.M)
    }


def test_the_variant_parser_finds_a_real_enum():
    """Non-vacuity: an empty variant set passes every assertion below."""
    found = variants("RollbackRefused")
    assert len(found) >= 3, sorted(found)
    assert "NeverActivated" in found, sorted(found)


def test_the_reference_names_no_variant_that_does_not_exist():
    text = referenced()
    invented: dict[str, list[str]] = {}
    for enum_name in DOCUMENTED_ENUMS:
        real = variants(enum_name)
        # Only variants of THIS enum that the reference spells in backticks.
        named = {
            match
            for match in re.findall(r"`([A-Z][A-Za-z0-9]*)`", text)
            if match not in real
        }
        # A backticked CamelCase word could be any type name, so only flag one
        # that looks like a variant of this enum: it must have been a variant
        # at some point, which we cannot know — so instead require that the
        # reference's own refusal list is exact.
        del named
        listed = re.search(
            rf"`{enum_name}` names the fix[^\n]*(?:\n[^\n]+)*?\.", text
        )
        if not listed:
            continue
        spelled = set(re.findall(r"`([A-Z][A-Za-z0-9]*)`", listed.group(0)))
        spelled.discard(enum_name)
        missing = sorted(spelled - real)
        if missing:
            invented[enum_name] = missing
    assert not invented, (
        f"the SDK reference names enum variants that do not exist: {invented}. "
        "A reader matches on them and it does not compile. This check exists "
        "because `RollbackRefused::ParticipantsDisagree` was deleted from the "
        "code and left in the reference, and every method guard in this file "
        "passed — a variant is not a `pub fn`."
    )


def test_every_refusal_variant_is_documented():
    """The other direction: a refusal a reader cannot look up sends them into `crates/`."""
    text = referenced()
    undocumented = sorted(
        variant for variant in variants("RollbackRefused") if f"`{variant}`" not in text
    )
    assert not undocumented, (
        f"these refusals are not named in the SDK reference: {undocumented}. "
        "A consumer who hits one has no way to find out what it means without "
        "reading the engine."
    )
