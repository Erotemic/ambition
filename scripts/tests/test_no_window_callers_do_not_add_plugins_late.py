"""A `NoWindow` app is FINISHED before it is returned, so adding a plugin panics.

`build_visible_app_with` runs `app.finish(); app.cleanup()` for
`VisibleRenderMode::NoWindow`, because `App::update()` never calls
`Plugin::finish` and `ImagePlugin` registers the image loader there — without it
every `Handle<Image>` sat in `LoadState::Loading` forever. Keeping that repair
costs one rule: a caller may no longer add plugins to the returned App, and
Bevy 0.19 enforces it with a panic:

    Plugins cannot be added after App::cleanup() or App::finish() has been called.

Three developer binaries did exactly that and died on their third line —
`moveset_takes` (which the moveset inspector shells out to), `shark_ride_probe`,
and `smash_match_profile`'s `--features profile` arm. Reproduced 2026-09-02:
`cargo run --bin shark_ride_probe` exited 101 at `shark_ride_probe.rs:27`.

⛔⛔ **A COMPILE CHECK CANNOT SEE THIS.** Every branch type-checks; the panic is
a runtime state machine. The app integration tests did not catch it either,
because they do not execute developer binaries. So the guard reads source: a
`NoWindow` app's binding must never be handed `add_plugins` afterwards.
Plugins go in the `build_visible_app_with(..., |app| ...)` composition hook,
which runs BEFORE the builder finishes.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)

# `let mut app = ...build_visible_app(` — the binding is what we then follow.
BINDING = re.compile(r"\blet\s+mut\s+([a-z_][a-z_0-9]*)\s*=[^;]*?\bbuild_visible_app\s*\(", re.S)
NO_WINDOW = "VisibleRenderMode::NoWindow"


def rust_sources() -> list[Path]:
    tracked = subprocess.run(
        ["git", "ls-files", "*.rs"], capture_output=True, text=True, cwd=REPO
    ).stdout.split()
    return [REPO / rel for rel in tracked]


def late_plugin_additions() -> list[str]:
    """Every place a NoWindow app is handed `add_plugins` after construction."""
    offences: list[str] = []
    for path in rust_sources():
        try:
            text = path.read_text(errors="ignore")
        except OSError:
            continue
        if "build_visible_app" not in text:
            continue
        for match in BINDING.finditer(text):
            binding = match.group(1)
            # The call's own text decides whether this is the finished mode.
            call_end = text.find(";", match.end())
            call = text[match.start() : call_end if call_end != -1 else len(text)]
            if NO_WINDOW not in call:
                continue
            # Follow the binding only to the end of its function: the next
            # line that starts a new item at column zero.
            rest = text[call_end:]
            stop = re.search(r"\n(?:pub )?(?:fn|struct|impl|mod)\s", rest)
            body = rest[: stop.start()] if stop else rest
            for hit in re.finditer(rf"\b{re.escape(binding)}\s*\.\s*add_plugins\s*\(", body):
                line = text[: call_end + hit.start()].count("\n") + 1
                offences.append(f"{path.relative_to(REPO)}:{line}  {binding}.add_plugins(...)")
    return offences


def test_no_caller_adds_a_plugin_to_a_finished_no_window_app():
    offences = late_plugin_additions()
    assert not offences, (
        "a NoWindow app is finished and cleaned up before it is returned, so "
        "Bevy 0.19 panics on add_plugins — move the plugin into the "
        "build_visible_app_with(..., |app| ...) composition hook:\n  "
        + "\n  ".join(offences)
    )


def test_the_lint_can_see_the_shape_it_forbids():
    """⛔ A LINT THAT MATCHES NOTHING PASSES FOREVER. This is its positive control.

    The three real offenders are fixed, so the repo no longer contains the
    pattern — which means the test above would also pass if the regex were
    broken, the file list were empty, or the binding tracking never fired.
    """
    sample = """
fn main() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.add_plugins(bevy::log::LogPlugin::default());
}
"""
    match = BINDING.search(sample)
    assert match and match.group(1) == "app", "the binding is not being tracked"
    assert NO_WINDOW in sample
    call_end = sample.find(";", match.end())
    assert re.search(r"\bapp\s*\.\s*add_plugins\s*\(", sample[call_end:]), (
        "the lint cannot see an add_plugins that follows the binding"
    )


def test_a_windowed_app_is_left_alone():
    """Only NoWindow finishes early. A Windowed caller may still add plugins."""
    sample = """
fn main() {
    let mut app = build_visible_app(VisibleRenderMode::Windowed, true);
    app.add_plugins(bevy::log::LogPlugin::default());
}
"""
    match = BINDING.search(sample)
    assert match, "premise: the binding still parses"
    call_end = sample.find(";", match.end())
    call = sample[match.start() : call_end]
    assert NO_WINDOW not in call, (
        "a Windowed build is not finished early and must not be flagged — "
        "cli.rs:268 is exactly this shape and is correct"
    )
