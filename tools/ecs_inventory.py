#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "tree-sitter>=0.25,<0.26",
#   "tree-sitter-rust>=0.24,<0.25",
# ]
# ///
"""Build a static ECS inventory for the ambition_sandbox crate.

The inventory is intended to be good enough for refactor planning and CI diffs,
not a replacement for rustc. It uses tree-sitter-rust for Rust syntax structure
and keeps Bevy-specific classification deliberately heuristic.

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

Run directly with an inline-metadata aware launcher, for example:

    uv run tools/ecs_inventory.py
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

try:
    import tree_sitter_rust as tsrust
    from tree_sitter import Language, Parser
except ImportError as ex:  # pragma: no cover - exercised only without deps installed.
    print(
        "error: missing tree-sitter dependencies. Try: uv run tools/ecs_inventory.py",
        file=sys.stderr,
    )
    raise


ECS_DERIVES = {"Component", "Bundle", "Resource", "Message", "Event"}
DEFAULT_EXCLUDED_DIR_NAMES = {"target", ".git"}
DEFAULT_EXCLUDED_PATH_PARTS = {"tests"}

SYSTEM_PARAM_NAMES = {
    "Commands",
    "Query",
    "Res",
    "ResMut",
    "EventReader",
    "EventWriter",
    "MessageReader",
    "MessageWriter",
    "Local",
    "RemovedComponents",
    "Assets",
    "Single",
    "ParamSet",
    "NonSend",
    "NonSendMut",
    "Deferred",
    "EventMutator",
    "SystemState",
    "In",
    "StaticSystemParam",
}

ITEM_NODE_TYPES = {"struct_item", "enum_item", "union_item"}
IDENTIFIER_NODE_TYPES = {"identifier", "type_identifier", "field_identifier"}
SCOPED_IDENTIFIER_NODE_TYPES = {"scoped_identifier", "scoped_type_identifier"}

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
    *REGISTRATION_NAMES,
}

QUALIFIED_IDENT_RE = re.compile(
    r"(?:(?:crate|super|self)::)?[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*"
)
DERIVE_ATTR_RE = re.compile(r"#\s*\[\s*derive\s*\((.*?)\)\s*\]", flags=re.DOTALL)
PLUGIN_IMPL_RE = re.compile(
    r"\bimpl\b(?:\s*<[^{};]*>)?\s+"
    r"(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*Plugin)\s+for\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
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


@dataclasses.dataclass(frozen=True)
class ParsedRustFile:
    path: pathlib.Path
    text: str
    source: bytes
    root: object


def rust_language() -> Language:
    """Return the tree-sitter Rust language, tolerant of binding variations."""
    raw_language = tsrust.language()
    if isinstance(raw_language, Language):
        return raw_language
    return Language(raw_language)


RUST_LANGUAGE = rust_language()


def make_parser() -> Parser:
    """Construct a parser across recent py-tree-sitter APIs."""
    try:
        return Parser(RUST_LANGUAGE)
    except TypeError:
        parser = Parser()
        parser.set_language(RUST_LANGUAGE)
        return parser


PARSER = make_parser()


def repo_rel(path: pathlib.Path, repo_root: pathlib.Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def parse_rust_file(path: pathlib.Path) -> ParsedRustFile:
    source = path.read_bytes()
    tree = PARSER.parse(source)
    text = source.decode("utf-8")
    return ParsedRustFile(path=path, text=text, source=source, root=tree.root_node)


def node_text(source: bytes, node: object) -> str:
    return source[node.start_byte : node.end_byte].decode("utf-8")


def node_line(node: object) -> int:
    point = node.start_point
    if hasattr(point, "row"):
        return point.row + 1
    return point[0] + 1


def child_count(node: object) -> int:
    return node.child_count


def named_child_count(node: object) -> int:
    return node.named_child_count


def children(node: object) -> list[object]:
    return [node.child(i) for i in range(child_count(node))]


def named_children(node: object) -> list[object]:
    return [node.named_child(i) for i in range(named_child_count(node))]


def child_by_field_name(node: object, name: str) -> object | None:
    try:
        return node.child_by_field_name(name)
    except Exception:
        return None


def same_node(left: object, right: object) -> bool:
    return (
        left.type == right.type
        and left.start_byte == right.start_byte
        and left.end_byte == right.end_byte
    )


def iter_named_descendants(node: object) -> Iterator[object]:
    yield node
    for child in named_children(node):
        yield from iter_named_descendants(child)


def iter_parsed_rs_files(crate_root: pathlib.Path, include_tests: bool) -> Iterator[ParsedRustFile]:
    for path in iter_rs_files(crate_root, include_tests):
        yield parse_rust_file(path)


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


def compact(text: str, max_len: int = 220) -> str:
    text = re.sub(r"\s+", " ", text.strip())
    return text if len(text) <= max_len else text[: max_len - 3] + "..."


def direct_visibility(source: bytes, node: object) -> str:
    for child in named_children(node):
        if child.type == "visibility_modifier":
            return node_text(source, child).strip()
    return ""


def attrs_immediately_before(source: bytes, node: object) -> list[str]:
    parent = node.parent
    if parent is None:
        return []
    siblings = named_children(parent)
    node_index = None
    for index, sibling in enumerate(siblings):
        if same_node(sibling, node):
            node_index = index
            break
    if node_index is None:
        return []

    attrs: list[str] = []
    index = node_index - 1
    while index >= 0 and siblings[index].type == "attribute_item":
        attrs.append(node_text(source, siblings[index]))
        index -= 1
    attrs.reverse()
    return attrs


def attr_is_cfg_test(attr: str) -> bool:
    dense = "".join(attr.split())
    return "cfg(test)" in dense or "cfg(any(test" in dense


def is_in_cfg_test_context(source: bytes, node: object) -> bool:
    current = node
    while current is not None:
        if any(attr_is_cfg_test(attr) for attr in attrs_immediately_before(source, current)):
            return True
        current = current.parent
    return False


def derive_names(attrs: Sequence[str]) -> list[str]:
    names: list[str] = []
    for attr in attrs:
        match = DERIVE_ATTR_RE.search(attr)
        if not match:
            continue
        for part in match.group(1).split(","):
            name = part.strip().split("::")[-1]
            if name:
                names.append(name)
    return names


def name_child_text(source: bytes, node: object) -> str:
    name = child_by_field_name(node, "name")
    if name is not None:
        return node_text(source, name).strip()
    for child in named_children(node):
        if child.type in {"identifier", "type_identifier"}:
            return node_text(source, child).strip()
    return ""


def item_kind(node_type: str) -> str:
    return node_type.removesuffix("_item")


def normalize_identifier_text(text: str) -> str:
    text = text.strip()
    text = text.removeprefix("r#")
    if text.endswith("!"):
        text = text[:-1]
    return text


def useful_identifier(ident: str) -> bool:
    ident = normalize_identifier_text(ident)
    if not ident:
        return False
    last = ident.split("::")[-1]
    last = last.split("<", 1)[0]
    last = last.removeprefix("r#")
    if ident in STOPWORD_IDENTIFIERS or last in STOPWORD_IDENTIFIERS:
        return False
    if ident in REGISTRATION_NAMES or last in REGISTRATION_NAMES:
        return False
    return bool(re.match(r"^[a-z_][a-z0-9_]*$", last) or re.match(r"^[A-Z][A-Za-z0-9_]*$", last))


def collect_identifiers(source: bytes, roots: Iterable[object]) -> list[str]:
    identifiers: list[str] = []
    seen: set[str] = set()
    stack = list(roots)
    while stack:
        node = stack.pop()
        if node.type in SCOPED_IDENTIFIER_NODE_TYPES:
            ident = normalize_identifier_text(node_text(source, node))
            if useful_identifier(ident) and ident not in seen:
                identifiers.append(ident)
                seen.add(ident)
            # Do not also emit every path segment as a separate identifier.
            continue
        if node.type in IDENTIFIER_NODE_TYPES:
            ident = normalize_identifier_text(node_text(source, node))
            if useful_identifier(ident) and ident not in seen:
                identifiers.append(ident)
                seen.add(ident)
        stack.extend(reversed(named_children(node)))
    return identifiers


def collect_type_names(source: bytes, root: object) -> set[str]:
    names: set[str] = set()
    for node in iter_named_descendants(root):
        if node.type in {"type_identifier", "identifier"}:
            names.add(normalize_identifier_text(node_text(source, node)))
        elif node.type in SCOPED_IDENTIFIER_NODE_TYPES:
            text = normalize_identifier_text(node_text(source, node))
            if text:
                names.add(text.split("::")[-1])
    return names


def argument_list_node(call_node: object) -> object | None:
    args = child_by_field_name(call_node, "arguments")
    if args is not None:
        return args
    for child in named_children(call_node):
        if child.type == "arguments":
            return child
    return None


def argument_nodes(args_node: object | None) -> list[object]:
    if args_node is None:
        return []
    return named_children(args_node)


def call_function_node(call_node: object) -> object | None:
    fn = child_by_field_name(call_node, "function")
    if fn is not None:
        return fn
    for child in named_children(call_node):
        if child.type != "arguments":
            return child
    return None


def field_name_from_field_expression(source: bytes, node: object) -> str:
    field = child_by_field_name(node, "field")
    if field is not None:
        return normalize_identifier_text(node_text(source, field))
    for child in reversed(named_children(node)):
        if child.type == "field_identifier":
            return normalize_identifier_text(node_text(source, child))
    return ""


def name_from_generic_function(source: bytes, node: object) -> str:
    fn = child_by_field_name(node, "function")
    if fn is not None:
        return called_function_name_from_node(source, fn)
    for child in named_children(node):
        if child.type != "type_arguments":
            return called_function_name_from_node(source, child)
    return ""


def called_function_name_from_node(source: bytes, node: object) -> str:
    if node.type == "field_expression":
        return field_name_from_field_expression(source, node)
    if node.type == "generic_function":
        return name_from_generic_function(source, node)
    if node.type in {"identifier", "field_identifier", "type_identifier"}:
        return normalize_identifier_text(node_text(source, node))
    if node.type in SCOPED_IDENTIFIER_NODE_TYPES:
        return normalize_identifier_text(node_text(source, node)).split("::")[-1]
    for child in reversed(named_children(node)):
        name = called_function_name_from_node(source, child)
        if name:
            return name
    return ""


def called_function_name(source: bytes, call_node: object) -> str:
    fn = call_function_node(call_node)
    return called_function_name_from_node(source, fn) if fn is not None else ""


def called_function_path_from_node(source: bytes, node: object) -> str:
    if node.type == "generic_function":
        fn = child_by_field_name(node, "function")
        if fn is not None:
            return called_function_path_from_node(source, fn)
    if node.type in SCOPED_IDENTIFIER_NODE_TYPES:
        return normalize_identifier_text(node_text(source, node))
    if node.type in {"identifier", "field_identifier", "type_identifier"}:
        return normalize_identifier_text(node_text(source, node))
    if node.type == "field_expression":
        field = field_name_from_field_expression(source, node)
        receiver = child_by_field_name(node, "value")
        if receiver is None:
            receiver = child_by_field_name(node, "argument")
        if receiver is not None:
            receiver_text = compact(node_text(source, receiver), 120)
            return f"{receiver_text}.{field}" if field else receiver_text
        return field
    for child in reversed(named_children(node)):
        path = called_function_path_from_node(source, child)
        if path:
            return path
    return ""


def called_function_path(source: bytes, call_node: object) -> str:
    fn = call_function_node(call_node)
    return called_function_path_from_node(source, fn) if fn is not None else ""


def argument_range_text(source: bytes, args: Sequence[object]) -> str:
    if not args:
        return ""
    return source[args[0].start_byte : args[-1].end_byte].decode("utf-8")


def collect_items(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[ItemRecord]:
    records: list[ItemRecord] = []
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type not in ITEM_NODE_TYPES:
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            attrs = attrs_immediately_before(parsed.source, node)
            derives = sorted(set(derive_names(attrs)).intersection(ECS_DERIVES))
            if not derives:
                continue
            records.append(
                ItemRecord(
                    name=name_child_text(parsed.source, node),
                    kind=item_kind(node.type),
                    derives=derives,
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    visibility=direct_visibility(parsed.source, node),
                )
            )
    return records


def impl_header_text(source: bytes, node: object) -> str:
    body = child_by_field_name(node, "body")
    if body is not None:
        end = body.start_byte
    else:
        brace = source.find(b"{", node.start_byte, node.end_byte)
        end = brace if brace != -1 else node.end_byte
    return source[node.start_byte : end].decode("utf-8")


def collect_plugins(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[PluginRecord]:
    records: list[PluginRecord] = []
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type != "impl_item":
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            match = PLUGIN_IMPL_RE.search(impl_header_text(parsed.source, node))
            if match is None:
                continue
            records.append(
                PluginRecord(
                    name=match.group("name"),
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                )
            )
    return records


def collect_system_like_functions(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[FunctionRecord]:
    records: list[FunctionRecord] = []
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type != "function_item":
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            params_node = child_by_field_name(node, "parameters")
            if params_node is None:
                continue
            if not (collect_type_names(parsed.source, params_node) & SYSTEM_PARAM_NAMES):
                continue
            records.append(
                FunctionRecord(
                    name=name_child_text(parsed.source, node),
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    public=bool(direct_visibility(parsed.source, node)),
                    params=compact(node_text(parsed.source, params_node).strip()[1:-1], 240),
                )
            )
    return records


def collect_registrations(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[RegistrationRecord]:
    records: list[RegistrationRecord] = []
    registration_names = set(REGISTRATION_NAMES)
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type != "call_expression":
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            name = called_function_name(parsed.source, node)
            if name not in registration_names:
                continue
            args_node = argument_list_node(node)
            args = argument_nodes(args_node)
            first_arg = node_text(parsed.source, args[0]) if args else ""
            function_node = call_function_node(node)
            if name == "add_systems":
                rest_args = args[1:]
                expression = argument_range_text(parsed.source, rest_args)
                identifier_roots = rest_args
            else:
                expression_parts = []
                if function_node is not None:
                    expression_parts.append(node_text(parsed.source, function_node))
                if args_node is not None:
                    expression_parts.append(node_text(parsed.source, args_node))
                expression = "".join(expression_parts)
                identifier_roots = []
                if function_node is not None:
                    identifier_roots.append(function_node)
                identifier_roots.extend(args)
            records.append(
                RegistrationRecord(
                    kind=name,
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    schedule_or_arg=compact(first_arg, 160),
                    expression=compact(expression, 360),
                    identifiers=collect_identifiers(parsed.source, identifier_roots),
                )
            )
    return records


def find_name_labels(source: bytes, expression_root: object) -> list[str]:
    labels: list[str] = []
    for node in iter_named_descendants(expression_root):
        if node.type != "call_expression":
            continue
        if called_function_path(source, node) != "Name::new":
            continue
        args = argument_nodes(argument_list_node(node))
        if args:
            labels.append(compact(node_text(source, args[0]), 120))
    return labels


def collect_spawns(crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool) -> list[SpawnRecord]:
    records: list[SpawnRecord] = []
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type != "call_expression":
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            name = called_function_name(parsed.source, node)
            if name not in {"spawn", "spawn_empty"}:
                continue
            args = argument_nodes(argument_list_node(node))
            expression = argument_range_text(parsed.source, args)
            records.append(
                SpawnRecord(
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    expression=compact(expression, 360),
                    identifiers=collect_identifiers(parsed.source, args),
                    name_labels=find_name_labels(parsed.source, node),
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
            out.append(f"- `{row.file}:{row.line}` - `{row.kind}` on/with `{row.schedule_or_arg or '<none>'}`")
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
