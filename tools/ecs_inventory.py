#!/usr/bin/env python3
"""Build a static ECS inventory for the ambition_sandbox crate.

The inventory is intended to be good enough for refactor planning and CI diffs,
not a replacement for rustc. It uses syntax-aware scanning with balanced
parentheses/braces and source locations instead of plain grep.

It reports:
  * Rust item types that derive Bevy ECS traits: Component, Bundle, Resource,
    Message, Event.
  * Plugin impls.
  * add_systems / configure_sets / add_message / add_event registrations.
  * ECS-looking function definitions, based on Bevy system parameter types.
  * Entity archetype evidence from spawn sites, inserted bundles/components,
    and Name::new labels.

The script deliberately keeps raw evidence in JSON so inventory changes can be
reviewed in code review rather than hidden behind a prose summary.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import pathlib
import re
import sys
from collections import defaultdict
from typing import Iterable, Iterator, Sequence


ECS_DERIVES = {"Component", "Bundle", "Resource", "Message", "Event"}
DEFAULT_EXCLUDED_DIR_NAMES = {"target", ".git"}
DEFAULT_EXCLUDED_PATH_PARTS = {"tests"}

SYSTEM_PARAM_RE = re.compile(
    r"\b(Commands|Query<|Res<|ResMut<|EventReader<|EventWriter<|"
    r"MessageReader<|MessageWriter<|Local<|RemovedComponents<|"
    r"Assets<|Single<|ParamSet<|NonSend<|NonSendMut<|Deferred<|"
    r"EventMutator<|SystemState<|In<|StaticSystemParam<)"
)

ITEM_RE = re.compile(
    r"(?P<vis>pub(?:\s*\([^)]*\))?\s+)?(?P<kind>struct|enum|union)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)

FN_RE = re.compile(
    r"(?P<vis>pub(?:\s*\([^)]*\))?\s+)?(?P<qual>unsafe\s+|async\s+|const\s+)*"
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)

PLUGIN_IMPL_RE = re.compile(
    r"impl\s+(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*Plugin)\s+for\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)

REGISTRATION_NAMES = (
    "add_systems",
    "configure_sets",
    "add_message",
    "add_event",
    "init_resource",
    "insert_resource",
    "add_plugins",
)

STOPWORD_IDENTIFIERS = {
    # Common Bevy schedule/set/type names and methods that appear inside
    # add_systems expressions but are not systems.
    "Startup", "PreStartup", "PostStartup", "First", "PreUpdate", "StateTransition",
    "RunFixedMainLoop", "FixedFirst", "FixedPreUpdate", "FixedUpdate",
    "FixedPostUpdate", "FixedLast", "Update", "PostUpdate", "Last",
    "RenderStartup", "Render", "App", "Plugin", "Plugins", "Commands", "Query",
    "Res", "ResMut", "Local", "Entity", "Name", "Transform", "Visibility", "Vec2",
    "Vec3", "Color", "Text", "Sprite", "Camera", "Camera2d", "Bundle", "default",
    "Default", "new", "clone", "load", "insert", "spawn", "id", "into", "from",
    "as_ref", "map", "run_if", "after", "before", "in_set", "chain", "amb",
    "system_set", "not", "or", "and", "resource_exists", "resource_changed",
    "resource_added", "in_state", "on_event", "any_with_component", "distributive_run_if",
}

QUALIFIED_IDENT_RE = re.compile(
    r"(?:(?:crate|super|self)::)?[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*"
)


@dataclasses.dataclass(frozen=True)
class ItemRecord:
    name: str
    kind: str
    derives: list[str]
    file: str
    line: int
    visibility: str = ""


@dataclasses.dataclass(frozen=True)
class FunctionRecord:
    name: str
    file: str
    line: int
    public: bool
    params: str


@dataclasses.dataclass(frozen=True)
class RegistrationRecord:
    kind: str
    file: str
    line: int
    schedule_or_arg: str
    expression: str
    identifiers: list[str]


@dataclasses.dataclass(frozen=True)
class SpawnRecord:
    file: str
    line: int
    expression: str
    identifiers: list[str]
    name_labels: list[str]


@dataclasses.dataclass(frozen=True)
class PluginRecord:
    name: str
    file: str
    line: int


def repo_rel(path: pathlib.Path, repo_root: pathlib.Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def strip_test_tail(text: str) -> str:
    """Drop a trailing cfg(test) module without changing non-test modules."""
    match = re.search(r"^\s*#\[cfg\(test\)\]", text, flags=re.MULTILINE)
    return text[: match.start()] if match else text


def mask_comments_and_strings(text: str) -> str:
    """Replace comments and string/char contents with spaces, preserving offsets.

    This is not a full Rust lexer, but it handles ordinary comments, block
    comments, normal strings, byte strings, raw strings such as r###"..."###,
    char literals, and escaped quotes well enough for source inventory.
    """
    out = list(text)
    i = 0
    n = len(text)
    while i < n:
        two = text[i : i + 2]
        if two == "//":
            j = text.find("\n", i)
            if j == -1:
                j = n
            for k in range(i, j):
                out[k] = " "
            i = j
            continue
        if two == "/*":
            j = text.find("*/", i + 2)
            if j == -1:
                j = n - 2
            for k in range(i, min(j + 2, n)):
                if out[k] != "\n":
                    out[k] = " "
            i = j + 2
            continue
        # Raw strings: r"...", r#"..."#, br#"..."#.
        raw_start = None
        if text.startswith("r", i) or text.startswith("br", i):
            prefix_len = 2 if text.startswith("br", i) else 1
            j = i + prefix_len
            hashes = 0
            while j < n and text[j] == "#":
                hashes += 1
                j += 1
            if j < n and text[j] == '"':
                raw_start = (j, hashes)
        if raw_start is not None:
            quote_index, hashes = raw_start
            end_token = '"' + ("#" * hashes)
            j = text.find(end_token, quote_index + 1)
            if j == -1:
                j = n - len(end_token)
            end = min(j + len(end_token), n)
            for k in range(i, end):
                if out[k] != "\n":
                    out[k] = " "
            i = end
            continue
        if text[i] == '"' or text.startswith('b"', i):
            start = i
            i += 2 if text.startswith('b"', i) else 1
            escaped = False
            while i < n:
                c = text[i]
                if escaped:
                    escaped = False
                elif c == "\\":
                    escaped = True
                elif c == '"':
                    i += 1
                    break
                i += 1
            for k in range(start, min(i, n)):
                if out[k] != "\n":
                    out[k] = " "
            continue
        if text[i] == "'":
            # Mask char literals, but leave lifetimes like 'a alone.
            j = i + 1
            if j < n and text[j].isalpha():
                j += 1
                if j >= n or text[j] != "'":
                    i += 1
                    continue
            escaped = False
            while j < n:
                c = text[j]
                if escaped:
                    escaped = False
                elif c == "\\":
                    escaped = True
                elif c == "'":
                    j += 1
                    break
                elif c == "\n":
                    break
                j += 1
            if j <= n and j > i + 1:
                for k in range(i, min(j, n)):
                    if out[k] != "\n":
                        out[k] = " "
                i = j
                continue
        i += 1
    return "".join(out)


def balance_end(text: str, open_pos: int, open_char: str = "(", close_char: str = ")") -> int | None:
    depth = 0
    for i in range(open_pos, len(text)):
        c = text[i]
        if c == open_char:
            depth += 1
        elif c == close_char:
            depth -= 1
            if depth == 0:
                return i
    return None


def split_top_level_comma(text: str) -> tuple[str, str]:
    depth = 0
    for i, c in enumerate(text):
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == "," and depth == 0:
            return text[:i].strip(), text[i + 1 :].strip()
    return text.strip(), ""


def compact(text: str, max_len: int = 220) -> str:
    text = re.sub(r"\s+", " ", text.strip())
    return text if len(text) <= max_len else text[: max_len - 3] + "..."


def find_identifiers(expression: str) -> list[str]:
    identifiers: list[str] = []
    seen: set[str] = set()
    for match in QUALIFIED_IDENT_RE.finditer(expression):
        ident = match.group(0)
        last = ident.split("::")[-1]
        if ident in STOPWORD_IDENTIFIERS or last in STOPWORD_IDENTIFIERS:
            continue
        # For system functions, snake_case names are most useful, but keep
        # plugin/resource names for add_plugins/init_resource calls.
        if not (re.match(r"^[a-z_][a-z0-9_]*$", last) or re.match(r"^[A-Z][A-Za-z0-9_]*$", last)):
            continue
        if ident not in seen:
            identifiers.append(ident)
            seen.add(ident)
    return identifiers


def find_name_labels(expression: str) -> list[str]:
    labels: list[str] = []
    for match in re.finditer(r"Name::new\s*\(", expression):
        close = balance_end(expression, match.end() - 1)
        if close is None:
            continue
        inner = expression[match.end() : close]
        labels.append(compact(inner, 120))
    return labels


def iter_rs_files(crate_root: pathlib.Path, include_tests: bool) -> Iterator[pathlib.Path]:
    src_root = crate_root / "src"
    for path in sorted(src_root.rglob("*.rs")):
        parts = set(path.parts)
        if any(part in DEFAULT_EXCLUDED_DIR_NAMES for part in parts):
            continue
        if not include_tests and "tests" in parts:
            continue
        if not include_tests and "/bin/" in path.as_posix():
            continue
        yield path


def extract_attrs_before(masked_text: str, item_start: int) -> list[str]:
    """Collect contiguous outer attributes immediately above an item."""
    prefix = masked_text[:item_start]
    lines = prefix.splitlines()
    attrs_reversed: list[str] = []
    pending: list[str] = []
    depth = 0
    # Walk upward over blank/doc-comment/attribute lines. The text is masked, so
    # real comments are spaces; doc comments are not attributes and are skipped.
    for line in reversed(lines):
        stripped = line.strip()
        if not stripped:
            if attrs_reversed or pending:
                break
            continue
        if stripped.startswith("///") or stripped.startswith("//!"):
            continue
        if stripped.endswith("]") or pending:
            pending.append(stripped)
            depth += stripped.count("]") - stripped.count("[")
            if depth >= 0 and any(s.startswith("#[") for s in pending):
                attrs_reversed.append(" ".join(reversed(pending)))
                pending = []
                depth = 0
            continue
        if stripped.startswith("#["):
            attrs_reversed.append(stripped)
            continue
        break
    return list(reversed(attrs_reversed))


def derive_names(attrs: Sequence[str]) -> list[str]:
    names: list[str] = []
    for attr in attrs:
        match = re.search(r"#\s*\[\s*derive\s*\((.*?)\)\s*\]", attr, flags=re.DOTALL)
        if not match:
            continue
        for part in match.group(1).split(","):
            name = part.strip().split("::")[-1]
            if name:
                names.append(name)
    return names


def collect_items(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[ItemRecord]:
    records: list[ItemRecord] = []
    for path in iter_rs_files(crate_root, include_tests):
        text = path.read_text(encoding="utf-8")
        if not include_tests:
            text = strip_test_tail(text)
        masked = mask_comments_and_strings(text)
        for match in ITEM_RE.finditer(masked):
            attrs = extract_attrs_before(masked, match.start())
            derives = sorted(set(derive_names(attrs)).intersection(ECS_DERIVES))
            if not derives:
                continue
            records.append(
                ItemRecord(
                    name=match.group("name"),
                    kind=match.group("kind"),
                    derives=derives,
                    file=repo_rel(path, repo_root),
                    line=text.count("\n", 0, match.start()) + 1,
                    visibility=(match.group("vis") or "").strip(),
                )
            )
    return records


def collect_plugins(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[PluginRecord]:
    records: list[PluginRecord] = []
    for path in iter_rs_files(crate_root, include_tests):
        text = path.read_text(encoding="utf-8")
        if not include_tests:
            text = strip_test_tail(text)
        masked = mask_comments_and_strings(text)
        for match in PLUGIN_IMPL_RE.finditer(masked):
            records.append(
                PluginRecord(
                    name=match.group("name"),
                    file=repo_rel(path, repo_root),
                    line=text.count("\n", 0, match.start()) + 1,
                )
            )
    return records


def function_signature(masked: str, start: int) -> str:
    open_pos = masked.find("(", start)
    if open_pos == -1:
        return ""
    close_pos = balance_end(masked, open_pos)
    if close_pos is None:
        return masked[start : start + 400]
    # Include return type up to opening body if nearby.
    brace_pos = masked.find("{", close_pos)
    semi_pos = masked.find(";", close_pos)
    end_candidates = [p for p in (brace_pos, semi_pos) if p != -1]
    end = min(end_candidates) if end_candidates else close_pos + 1
    return masked[start:end]


def collect_system_like_functions(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[FunctionRecord]:
    records: list[FunctionRecord] = []
    for path in iter_rs_files(crate_root, include_tests):
        text = path.read_text(encoding="utf-8")
        if not include_tests:
            text = strip_test_tail(text)
        masked = mask_comments_and_strings(text)
        for match in FN_RE.finditer(masked):
            sig = function_signature(masked, match.start())
            if not SYSTEM_PARAM_RE.search(sig):
                continue
            params = ""
            open_pos = sig.find("(")
            if open_pos != -1:
                close_pos = balance_end(sig, open_pos)
                if close_pos is not None:
                    params = compact(sig[open_pos + 1 : close_pos], 240)
            records.append(
                FunctionRecord(
                    name=match.group("name"),
                    file=repo_rel(path, repo_root),
                    line=text.count("\n", 0, match.start()) + 1,
                    public=bool(match.group("vis")),
                    params=params,
                )
            )
    return records


def collect_registrations(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[RegistrationRecord]:
    records: list[RegistrationRecord] = []
    name_alt = "|".join(REGISTRATION_NAMES)
    call_re = re.compile(rf"(?:\.|\b)(?P<name>{name_alt})(?P<generic>\s*::\s*<[^;(){{}}]+>)?\s*\(")
    for path in iter_rs_files(crate_root, include_tests):
        text = path.read_text(encoding="utf-8")
        if not include_tests:
            text = strip_test_tail(text)
        masked = mask_comments_and_strings(text)
        for match in call_re.finditer(masked):
            open_pos = masked.find("(", match.start())
            close_pos = balance_end(masked, open_pos)
            if open_pos == -1 or close_pos is None:
                continue
            body = text[open_pos + 1 : close_pos]
            masked_body = masked[open_pos + 1 : close_pos]
            first_arg, rest = split_top_level_comma(masked_body)
            raw_first, raw_rest = split_top_level_comma(body)
            generic = match.group("generic") or ""
            if match.group("name") == "add_systems":
                schedule_or_arg = compact(raw_first, 160)
                expression = compact(raw_rest, 360)
                _, masked_rest = split_top_level_comma(masked_body)
                identifiers = find_identifiers(masked_rest)
            else:
                schedule_or_arg = compact(raw_first, 160)
                expression = compact((generic + body), 360)
                identifiers = find_identifiers(generic + masked_body)
            records.append(
                RegistrationRecord(
                    kind=match.group("name"),
                    file=repo_rel(path, repo_root),
                    line=text.count("\n", 0, match.start()) + 1,
                    schedule_or_arg=schedule_or_arg,
                    expression=expression,
                    identifiers=identifiers,
                )
            )
    return records


def collect_spawns(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[SpawnRecord]:
    records: list[SpawnRecord] = []
    spawn_re = re.compile(r"(?:\.|\b)(spawn|spawn_empty)\s*\(")
    for path in iter_rs_files(crate_root, include_tests):
        text = path.read_text(encoding="utf-8")
        if not include_tests:
            text = strip_test_tail(text)
        masked = mask_comments_and_strings(text)
        for match in spawn_re.finditer(masked):
            open_pos = masked.find("(", match.start())
            close_pos = balance_end(masked, open_pos)
            if open_pos == -1 or close_pos is None:
                continue
            expr = text[open_pos + 1 : close_pos]
            masked_expr = masked[open_pos + 1 : close_pos]
            records.append(
                SpawnRecord(
                    file=repo_rel(path, repo_root),
                    line=text.count("\n", 0, match.start()) + 1,
                    expression=compact(expr, 360),
                    identifiers=find_identifiers(masked_expr),
                    name_labels=find_name_labels(expr),
                )
            )
    return records


def asdict_list(records: Iterable[object]) -> list[dict]:
    return [dataclasses.asdict(record) for record in records]


def group_by_derive(items: Sequence[ItemRecord]) -> dict[str, list[ItemRecord]]:
    grouped: dict[str, list[ItemRecord]] = {derive: [] for derive in sorted(ECS_DERIVES)}
    for item in items:
        for derive in item.derives:
            grouped.setdefault(derive, []).append(item)
    return grouped


def write_markdown(inventory: dict, path: pathlib.Path) -> None:
    items = [ItemRecord(**item) for item in inventory["ecs_items"]]
    grouped_items = group_by_derive(items)
    functions = [FunctionRecord(**row) for row in inventory["system_like_functions"]]
    registrations = [RegistrationRecord(**row) for row in inventory["registrations"]]
    spawns = [SpawnRecord(**row) for row in inventory["spawn_sites"]]
    plugins = [PluginRecord(**row) for row in inventory["plugins"]]

    out: list[str] = []
    out.append("# Ambition Sandbox ECS inventory")
    out.append("")
    out.append(f"Generated from `{inventory['crate_root']}`.")
    out.append("")
    out.append("## Counts")
    for key, value in inventory["counts"].items():
        out.append(f"- {key.replace('_', ' ').title()}: {value}")
    out.append("")

    for derive_name in ("Component", "Bundle", "Resource", "Message", "Event"):
        rows = grouped_items.get(derive_name, [])
        out.append(f"## {derive_name}s")
        if not rows:
            out.append("- None found.")
            out.append("")
            continue
        by_file: dict[str, list[ItemRecord]] = defaultdict(list)
        for row in rows:
            by_file[row.file].append(row)
        for file in sorted(by_file):
            out.append(f"- `{file}`")
            for row in sorted(by_file[file], key=lambda r: r.line):
                other = [d for d in row.derives if d != derive_name]
                suffix = f"; also derives {', '.join(other)}" if other else ""
                out.append(f"  - `{row.name}` ({row.kind}, line {row.line}{suffix})")
        out.append("")

    out.append("## Plugins")
    if not plugins:
        out.append("- None found.")
    else:
        by_file: dict[str, list[PluginRecord]] = defaultdict(list)
        for row in plugins:
            by_file[row.file].append(row)
        for file in sorted(by_file):
            out.append(f"- `{file}`")
            for row in sorted(by_file[file], key=lambda r: r.line):
                out.append(f"  - `{row.name}` (line {row.line})")
    out.append("")

    out.append("## Registrations")
    if not registrations:
        out.append("- None found.")
    else:
        for row in registrations:
            out.append(f"- `{row.file}:{row.line}` — `{row.kind}` on/with `{row.schedule_or_arg or '<none>'}`")
            if row.identifiers:
                for ident in row.identifiers:
                    out.append(f"  - `{ident}`")
            else:
                out.append(f"  - expression: `{row.expression}`")
    out.append("")

    out.append("## System-like function definitions")
    if not functions:
        out.append("- None found.")
    else:
        by_file: dict[str, list[FunctionRecord]] = defaultdict(list)
        for row in functions:
            by_file[row.file].append(row)
        for file in sorted(by_file):
            out.append(f"- `{file}`")
            for row in sorted(by_file[file], key=lambda r: r.line):
                vis = "pub " if row.public else ""
                out.append(f"  - `{vis}{row.name}` (line {row.line})")
    out.append("")

    out.append("## Entity archetype evidence / spawn sites")
    out.append("Static analysis cannot know every runtime entity instance. This section lists spawn sites and the bundle/component/type identifiers found in each spawn expression.")
    if not spawns:
        out.append("- None found.")
    else:
        for row in spawns:
            out.append(f"- `{row.file}:{row.line}`")
            if row.name_labels:
                for label in row.name_labels:
                    out.append(f"  - name label: `{label}`")
            if row.identifiers:
                out.append("  - identifiers:")
                for ident in row.identifiers[:40]:
                    out.append(f"    - `{ident}`")
                if len(row.identifiers) > 40:
                    out.append(f"    - ... {len(row.identifiers) - 40} more")
            else:
                out.append(f"  - expression: `{row.expression}`")
    out.append("")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(out) + "\n", encoding="utf-8")


def build_inventory(repo_root: pathlib.Path, crate_root: pathlib.Path, include_tests: bool) -> dict:
    items = collect_items(crate_root, repo_root, include_tests)
    functions = collect_system_like_functions(crate_root, repo_root, include_tests)
    registrations = collect_registrations(crate_root, repo_root, include_tests)
    spawns = collect_spawns(crate_root, repo_root, include_tests)
    plugins = collect_plugins(crate_root, repo_root, include_tests)
    grouped = group_by_derive(items)
    unique_registration_identifiers = sorted({ident for row in registrations for ident in row.identifiers})
    inventory = {
        "schema_version": 1,
        "repo_root": ".",
        "crate_root": repo_rel(crate_root, repo_root),
        "include_tests": include_tests,
        "counts": {
            "components": len(grouped.get("Component", [])),
            "bundles": len(grouped.get("Bundle", [])),
            "resources": len(grouped.get("Resource", [])),
            "messages": len(grouped.get("Message", [])),
            "events": len(grouped.get("Event", [])),
            "plugins": len(plugins),
            "registrations": len(registrations),
            "unique_registration_identifiers": len(unique_registration_identifiers),
            "system_like_functions": len(functions),
            "spawn_sites": len(spawns),
        },
        "ecs_items": asdict_list(items),
        "plugins": asdict_list(plugins),
        "registrations": asdict_list(registrations),
        "unique_registration_identifiers": unique_registration_identifiers,
        "system_like_functions": asdict_list(functions),
        "spawn_sites": asdict_list(spawns),
    }
    return inventory


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--crate", type=pathlib.Path, default=pathlib.Path("crates/ambition_sandbox"))
    parser.add_argument("--json", type=pathlib.Path, default=pathlib.Path("target/ambition_ecs_inventory.json"))
    parser.add_argument("--markdown", type=pathlib.Path, default=pathlib.Path("target/ambition_ecs_inventory.md"))
    parser.add_argument("--include-tests", action="store_true")
    parser.add_argument("--check-json", type=pathlib.Path, help="Compare generated JSON with an existing inventory file.")
    args = parser.parse_args(argv)

    repo_root = args.repo_root.resolve()
    crate_root = args.crate if args.crate.is_absolute() else repo_root / args.crate
    crate_root = crate_root.resolve()
    if not (crate_root / "src").is_dir():
        print(f"error: crate source directory not found: {crate_root / 'src'}", file=sys.stderr)
        return 2

    inventory = build_inventory(repo_root, crate_root, args.include_tests)

    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(inventory, args.markdown)

    if args.check_json:
        expected = json.loads(args.check_json.read_text(encoding="utf-8"))
        if expected != inventory:
            print(f"inventory differs from {args.check_json}", file=sys.stderr)
            print(f"wrote current inventory to {args.json}", file=sys.stderr)
            return 1

    print(f"wrote {args.json}")
    print(f"wrote {args.markdown}")
    print(json.dumps(inventory["counts"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
