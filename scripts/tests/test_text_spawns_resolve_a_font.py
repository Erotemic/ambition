"""Menu text spawns must resolve an explicit font handle.

Bevy supplies a default `TextFont` when none is provided, and that fallback does
not cover every glyph used by Ambition menus. These tests stay scoped to the menu
renderer, where the failure is established, and accept any spawn path that puts
a real font handle in scope. Composition with no loaded fonts may still use the
engine fallback."""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# The renderer whose tofu is measured. Both files draw player-facing strings a
# game supplied (page text, row labels, tab labels).
MENU_RENDER_PATHS = (
    Path("crates/ambition_menu/src/render/bevy_ui/spawn.rs"),
    Path("crates/ambition_menu/src/render/bevy_ui/mod.rs"),
)

SPAWN_CALL = re.compile(r"\bspawn\s*\(")
TEXT_CTOR = re.compile(r"\bText(2d)?\s*::\s*(new|default)\b")
# `font_size` must NOT count as a resolved font, and neither must the type name `TextFont` — those
# are the two spellings present at every offending site. Word boundaries exclude both: `_` is a word
# character, so `\bfont\b` cannot match inside `font_size`, and there is no boundary before the
# `font` in `TextFont`.
FONT_RESOLVED = re.compile(
    r"""
      font \s* :            # `TextFont { font: <handle>, .. }`
    | \b font \b            # a resolved handle passed by name
    | \b \w*_font \b        # `card_font`, `bubble_font`
    | \b \w* font \w* \s* \(  # `nameplate_font(..)`, `fonts.text_font(..)`
    """,
    re.IGNORECASE | re.VERBOSE,
)


def _balanced(source: str, open_paren: int) -> str:
    """The source between `(` at `open_paren` and its matching `)`."""
    depth = 0
    in_str = False
    escaped = False
    for i in range(open_paren, len(source)):
        ch = source[i]
        if in_str:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return source[open_paren + 1 : i]
    return source[open_paren + 1 :]


def _test_spans(source: str) -> list[tuple[int, int]]:
    """Byte spans of inline `#[cfg(test)] mod ...` blocks."""
    spans: list[tuple[int, int]] = []
    for match in re.finditer(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{", source):
        depth = 0
        for i in range(match.end() - 1, len(source)):
            if source[i] == "{":
                depth += 1
            elif source[i] == "}":
                depth -= 1
                if depth == 0:
                    spans.append((match.start(), i))
                    break
    return spans


def _unfonted_menu_text_spawns() -> list[str]:
    offenders: list[str] = []
    for rel in MENU_RENDER_PATHS:
        path = REPO / rel
        assert path.is_file(), (
            f"{rel} no longer exists, so this guard is watching nothing. It was "
            "written against the menu's Bevy-UI renderer; if that moved, point "
            "MENU_RENDER_PATHS at wherever it lives now."
        )
        source = path.read_text(encoding="utf-8")
        spans = _test_spans(source)
        for call in SPAWN_CALL.finditer(source):
            open_paren = call.end() - 1
            if any(start <= open_paren <= end for start, end in spans):
                continue
            args = _balanced(source, open_paren)
            if not TEXT_CTOR.search(args) or FONT_RESOLVED.search(args):
                continue
            line = source[:open_paren].count("\n") + 1
            offenders.append(f"{rel}:{line}  spawn(( {' '.join(args.split())[:72]} ))")
    return sorted(offenders)


def test_no_menu_text_falls_back_to_the_default_font_handle():
    offenders = _unfonted_menu_text_spawns()
    assert not offenders, (
        "these menu spawns hand Bevy a `Text` with no font, so it inserts the "
        "built-in `FiraMono-subset.ttf` — the handle that drew a hollow box for "
        "`·` in every menu in every game:\n  "
        + "\n  ".join(offenders)
        + "\n\nPass the handle the host resolved: `TextFont { font: "
        "font.cloned().unwrap_or_default(), ..default() }`, where `font` is the "
        "`MenuFont` threaded through `spawn_bevy_ui_menu_with_font`."
    )
