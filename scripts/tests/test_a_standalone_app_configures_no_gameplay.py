"""A demo's standalone binary configures no GAMEPLAY of its own.

Jon, 2026-09-05: *"The smash app itself should be a thin wrapper that just
bypasses the launcher. I don't like that there are ever differences between
applications when they run standalone vs the launcher. Very unhappy about that.
I want that fixed. Structurally."*

⛔⛔ THE DIVERGENCE THIS PREVENTS SHIPPED AND A PLAYER FOUND IT. The smash portal
view cone was selected in `ambition_demo_smash_app` — the standalone binary — and
the engine default is `PortalViewConeMode::Dynamic`, "a viewer-dependent window".
So the demo app drew the static cone Jon asked for and the game a player actually
reaches Smash through, the main app's versus route, drew the viewer-dependent one.
Two binaries, one ruleset, different behaviour, silently, for as long as both
existed.

⭐ THE RULE: a ruleset's answers belong to the PLUGIN both compositions install.
A standalone binary may choose HOST concerns — a window, a fixed timestep, which
route it opens on — and nothing about how the game plays.

⚠ TEXTUAL AND DELIBERATELY NARROW. It cannot see a gameplay resource configured
through a helper it does not recognise. What it does hold is the shape the bug
actually took: a `*_app` crate reaching for a gameplay resource directly, which
is what a contributor writes when the setting "belongs to this demo".
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]

# (standalone binary crate, the plugin crate whose rules it must not restate)
STANDALONE = [
    ("game/ambition_demo_smash_app", "game/ambition_demo_smash"),
]

# ⛔⛔ WHITESPACE-TOLERANT AND SCANNED OVER THE WHOLE FILE, because the
# line-by-line version PASSED ITS OWN POISON. Rustfmt writes a long insertion as
#
#     app.insert_resource(
#         some::long::Path::default(),
#     );
#
# so the type is not on the line the call is on, and a per-line regex saw
# `insert_resource(` followed by nothing and matched nothing at all. The guard was
# green against the exact edit it exists to catch. ⇒ Found by poisoning, which is
# the only thing that could have found it.
INSERT = re.compile(
    r"\b(?:insert_resource|init_resource)\s*::\s*<\s*([\w:]+)|"
    r"\b(?:insert_resource|init_resource)\s*\(\s*([\w:]+)",
    re.S,
)

# Host concerns a standalone binary legitimately owns. Each says what makes it a
# HOST question rather than a gameplay one.
HOST_OWNED = {
    # How fast the process steps its own clock: a property of running the binary,
    # not of the game's rules.
    "bevy::time::TimeUpdateStrategy",
    "TimeUpdateStrategy",
}


def _sources(crate):
    """The crate's COMPOSITION, not its tools.

    ⛔ `src/tools/` is excluded and the exclusion is the interesting part: a
    capture probe or a screenshot harness legitimately inits its own bookkeeping
    resources, and those are neither gameplay nor shared with the launcher —
    nothing a player runs composes them. The guard's first run reported four such
    resources as divergence, which is the same population error as reading a
    fixture's spawn as a ruleset's.

    ⇒ What remains is what a player actually runs: `main.rs`, `lib.rs` and the
    modules they compose.
    """
    for path in sorted((REPO / crate / "src").rglob("*.rs")):
        if path.name == "tests.rs" or path.stem.endswith("_tests"):
            continue
        if "/tools/" in str(path) or "/bin/" in str(path):
            continue
        yield path


def test_a_standalone_binary_states_no_gameplay_rule():
    offenders = []
    checked = 0
    for app_crate, plugin_crate in STANDALONE:
        assert (REPO / app_crate).is_dir(), f"missing crate: {app_crate}"
        assert (REPO / plugin_crate).is_dir(), f"missing crate: {plugin_crate}"
        for path in _sources(app_crate):
            checked += 1
            text = path.read_text()
            # Comments stripped by line, then the file scanned WHOLE, so an
            # insertion split across lines is still one match.
            code = "\n".join(line.split("//", 1)[0] for line in text.splitlines())
            for match in INSERT.finditer(code):
                named = match.group(1) or match.group(2)
                # ⚠ ANY SEGMENT, not the last one: the capture is a PATH, and a
                # constructor makes the final segment the variant
                # (`bevy::time::TimeUpdateStrategy::ManualDuration`). Matching
                # only the tail failed the control on the one insertion this
                # crate is entitled to make.
                segments = set(named.split("::"))
                if segments & HOST_OWNED:
                    continue
                number = code.count("\n", 0, match.start()) + 1
                offenders.append(f"{app_crate}/{path.name}:{number}: {named}")

    assert checked >= 1, "the sweep found no sources; it has lost its corpus"
    assert not offenders, (
        "a standalone binary configures gameplay of its own:\n  "
        + "\n  ".join(offenders)
        + "\n⇒ Move it to the experience PLUGIN, which the launcher installs too. "
        "A rule stated in one binary is a rule the other binary does not have, "
        "and the difference is invisible until somebody plays both. If it is "
        "genuinely a HOST concern — the window, the clock, the opening route — "
        "add it to HOST_OWNED here with the reason."
    )
