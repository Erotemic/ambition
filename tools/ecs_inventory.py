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
  * Bevy architecture marker types such as SystemSet and States.
  * Plugin impls and the registrations made inside each plugin build context.
  * add_systems / configure_sets / add_message / add_event registrations, with
    schedule/set/run-condition/system breakdowns where static analysis can infer
    them.
  * ECS-looking function definitions, based on Bevy system parameter types, plus
    resource/query/message access summaries.
  * Message bus and resource access summaries for architectural review.
  * Entity archetype evidence from spawn sites, inserted bundles/components,
    and Name::new labels.
  * Non-ECS Rust data/model items with heuristic migration-candidate scoring.

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
ARCHITECTURE_DERIVES = {"SystemSet", "States", "SubStates", "SystemParam"}
STATEFUL_NAME_RE = re.compile(
    r"(Runtime|State|Config|Spec|Data|Profile|Archetype|Behavior|Cluster|"
    r"Registry|Catalog|Index|Set|Queue|Request|Context|Controller|Model|"
    r"Room|Actor|Player|Enemy|Boss|Inventory|Quest|Encounter|Save|World)"
)
LOW_SIGNAL_PATH_PARTS = {
    "assets",
    "audio",
    "dev",
    "host",
    "music",
    "presentation",
    "rendering",
    "ui",
    "tests",
    "test",
}
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

SCHEDULE_NAMES = {
    "Startup",
    "PreStartup",
    "PostStartup",
    "First",
    "PreUpdate",
    "StateTransition",
    "RunFixedMainLoop",
    "FixedFirst",
    "FixedPreUpdate",
    "FixedUpdate",
    "FixedPostUpdate",
    "FixedLast",
    "Update",
    "PostUpdate",
    "Last",
    "RenderStartup",
    "Render",
}
RUN_CONDITION_METHODS = {"run_if", "distributive_run_if"}
SET_MODIFIER_METHODS = {"in_set"}
ORDERING_METHODS = {"after", "before", "chain", "amb"}

STOPWORD_IDENTIFIERS = {
    # Common Bevy schedule/set/type names and methods that appear inside
    # add_systems expressions but are not systems.
    "Startup",
    "PreStartup",
    "PostStartup",
    "First",
    "PreUpdate",
    "StateTransition",
    "RunFixedMainLoop",
    "FixedFirst",
    "FixedPreUpdate",
    "FixedUpdate",
    "FixedPostUpdate",
    "FixedLast",
    "Update",
    "PostUpdate",
    "Last",
    "RenderStartup",
    "Render",
    "App",
    "Plugin",
    "Plugins",
    "Commands",
    "Query",
    "Res",
    "ResMut",
    "Local",
    "Entity",
    "Name",
    "Transform",
    "Visibility",
    "Vec2",
    "Vec3",
    "Color",
    "Text",
    "Sprite",
    "Camera",
    "Camera2d",
    "Bundle",
    "default",
    "Default",
    "new",
    "clone",
    "load",
    "insert",
    "spawn",
    "id",
    "into",
    "from",
    "as_ref",
    "map",
    "run_if",
    "after",
    "before",
    "in_set",
    "chain",
    "amb",
    "system_set",
    "not",
    "or",
    "and",
    "resource_exists",
    "resource_changed",
    "resource_added",
    "in_state",
    "on_event",
    "any_with_component",
    "distributive_run_if",
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
    commands: bool = False
    queries: list[str] = dataclasses.field(default_factory=list)
    resources_read: list[str] = dataclasses.field(default_factory=list)
    resources_written: list[str] = dataclasses.field(default_factory=list)
    messages_read: list[str] = dataclasses.field(default_factory=list)
    messages_written: list[str] = dataclasses.field(default_factory=list)
    locals: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class RegistrationRecord:
    kind: str
    file: str
    line: int
    schedule_or_arg: str
    expression: str
    identifiers: list[str]
    context: str = ""
    systems: list[str] = dataclasses.field(default_factory=list)
    run_conditions: list[str] = dataclasses.field(default_factory=list)
    sets: list[str] = dataclasses.field(default_factory=list)
    ordering: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class SpawnRecord:
    file: str
    line: int
    expression: str
    identifiers: list[str]
    name_labels: list[str]
    matched_ecs_items: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class PluginRecord:
    name: str
    file: str
    line: int


@dataclasses.dataclass(frozen=True)
class PlainItemRecord:
    name: str
    kind: str
    derives: list[str]
    file: str
    line: int
    visibility: str = ""
    category: str = "plain"
    migration_score: int = 0
    migration_priority: str = "low"
    reasons: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class ModuleSummaryRecord:
    module: str
    ecs_items: int = 0
    components: int = 0
    resources: int = 0
    messages: int = 0
    plugins: int = 0
    registered_systems: int = 0
    system_like_functions: int = 0
    spawn_sites: int = 0
    non_ecs_items: int = 0
    migration_candidates: int = 0


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


def iter_parsed_rs_files(
    crate_root: pathlib.Path, include_tests: bool
) -> Iterator[ParsedRustFile]:
    for path in iter_rs_files(crate_root, include_tests):
        yield parse_rust_file(path)


def iter_rs_files(
    crate_root: pathlib.Path, include_tests: bool
) -> Iterator[pathlib.Path]:
    src_root = crate_root / "src"
    for path in sorted(src_root.rglob("*.rs")):
        parts = set(path.parts)
        if any(part in DEFAULT_EXCLUDED_DIR_NAMES for part in parts):
            continue
        if not include_tests and "tests" in parts:
            continue
        if not include_tests and path.name in {"tests.rs", "test.rs"}:
            continue
        if not include_tests and path.name.endswith("_tests.rs"):
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
    return (
        "cfg(test)" in dense
        or "cfg(any(test" in dense
        or dense in {"#[test]", "#[tokio::test]"}
        or dense.startswith("#[test]")
    )


def is_in_cfg_test_context(source: bytes, node: object) -> bool:
    current = node
    while current is not None:
        if any(
            attr_is_cfg_test(attr) for attr in attrs_immediately_before(source, current)
        ):
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
    return bool(
        re.match(r"^[a-z_][a-z0-9_]*$", last) or re.match(r"^[A-Z][A-Za-z0-9_]*$", last)
    )


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


def unique_ordered(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for value in values:
        if value and value not in seen:
            out.append(value)
            seen.add(value)
    return out


def split_top_level_commas(text: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth_angle = depth_paren = depth_bracket = depth_brace = 0
    for index, char in enumerate(text):
        if char == "<":
            depth_angle += 1
        elif char == ">" and depth_angle:
            depth_angle -= 1
        elif char == "(":
            depth_paren += 1
        elif char == ")" and depth_paren:
            depth_paren -= 1
        elif char == "[":
            depth_bracket += 1
        elif char == "]" and depth_bracket:
            depth_bracket -= 1
        elif char == "{":
            depth_brace += 1
        elif char == "}" and depth_brace:
            depth_brace -= 1
        elif char == "," and not (
            depth_angle or depth_paren or depth_bracket or depth_brace
        ):
            part = text[start:index].strip()
            if part:
                parts.append(part)
            start = index + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def balance_angle_end(text: str, open_pos: int) -> int | None:
    depth = 0
    for index in range(open_pos, len(text)):
        char = text[index]
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
            if depth == 0:
                return index
    return None


def generic_inners(text: str, name: str) -> list[str]:
    inners: list[str] = []
    for match in re.finditer(rf"\b{re.escape(name)}\s*<", text):
        open_pos = match.end() - 1
        close_pos = balance_angle_end(text, open_pos)
        if close_pos is not None:
            inners.append(text[open_pos + 1 : close_pos].strip())
    return inners


def compact_type(type_text: str, max_len: int = 120) -> str:
    text = re.sub(r"\b(?:crate|super|self)::", "", type_text.strip())
    text = re.sub(
        r"\b(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)", r"\1", text
    )
    text = re.sub(r"\s+", " ", text)
    return compact(text, max_len)


def generic_payload_type(inner: str) -> str:
    parts = split_top_level_commas(inner)
    parts = [part for part in parts if not part.strip().startswith("'")]
    if not parts:
        return compact_type(inner, 100)
    return compact_type(parts[-1], 100)


def analyze_system_params(params: str) -> dict[str, object]:
    resources_read = [
        generic_payload_type(inner) for inner in generic_inners(params, "Res")
    ]
    resources_written = [
        generic_payload_type(inner) for inner in generic_inners(params, "ResMut")
    ]
    resources_read.extend(
        f"Assets<{generic_payload_type(inner)}>"
        for inner in generic_inners(params, "Assets")
    )
    messages_read = [
        generic_payload_type(inner) for inner in generic_inners(params, "MessageReader")
    ]
    messages_read.extend(
        generic_payload_type(inner) for inner in generic_inners(params, "EventReader")
    )
    messages_written = [
        generic_payload_type(inner) for inner in generic_inners(params, "MessageWriter")
    ]
    messages_written.extend(
        generic_payload_type(inner) for inner in generic_inners(params, "EventWriter")
    )
    locals_ = [generic_payload_type(inner) for inner in generic_inners(params, "Local")]
    queries = [compact(inner, 160) for inner in generic_inners(params, "Query")]
    return {
        "commands": bool(re.search(r"\bCommands\b", params)),
        "queries": unique_ordered(queries),
        "resources_read": unique_ordered(resources_read),
        "resources_written": unique_ordered(resources_written),
        "messages_read": unique_ordered(messages_read),
        "messages_written": unique_ordered(messages_written),
        "locals": unique_ordered(locals_),
    }


def identifiers_for_method_calls(
    source: bytes, roots: Iterable[object], method_names: set[str]
) -> list[str]:
    out: list[str] = []
    for root in roots:
        for node in iter_named_descendants(root):
            if node.type != "call_expression":
                continue
            if called_function_name(source, node) not in method_names:
                continue
            out.extend(
                collect_identifiers(source, argument_nodes(argument_list_node(node)))
            )
    return unique_ordered(out)


def identifier_last(ident: str) -> str:
    base = ident.split("<", 1)[0]
    return base.split("::")[-1].split(".")[-1]


def looks_like_system_identifier(ident: str) -> bool:
    last = identifier_last(ident)
    return (
        bool(re.match(r"^[a-z_][a-z0-9_]*$", last)) and last not in STOPWORD_IDENTIFIERS
    )


def looks_like_set_identifier(ident: str) -> bool:
    last = identifier_last(ident)
    return "Set::" in ident or last.endswith("Set") or last in SCHEDULE_NAMES


def registration_breakdown(
    source: bytes, name: str, args: Sequence[object], function_node: object | None
) -> dict[str, list[str]]:
    roots: list[object]
    if name == "add_systems":
        roots = list(args[1:])
    else:
        roots = []
        if function_node is not None:
            roots.append(function_node)
        roots.extend(args)

    all_identifiers = collect_identifiers(source, roots)
    run_conditions = identifiers_for_method_calls(source, roots, RUN_CONDITION_METHODS)
    set_modifiers = identifiers_for_method_calls(source, roots, SET_MODIFIER_METHODS)
    ordering = identifiers_for_method_calls(source, roots, ORDERING_METHODS)
    sets = unique_ordered(
        [ident for ident in all_identifiers if looks_like_set_identifier(ident)]
        + [ident for ident in set_modifiers if looks_like_set_identifier(ident)]
    )

    systems: list[str] = []
    if name == "add_systems":
        excluded = set(run_conditions) | set(sets) | set(ordering)
        systems = [
            ident
            for ident in all_identifiers
            if ident not in excluded and looks_like_system_identifier(ident)
        ]

    return {
        "systems": unique_ordered(systems),
        "run_conditions": unique_ordered(run_conditions),
        "sets": sets,
        "ordering": unique_ordered(ordering),
    }


def plugin_name_from_impl(source: bytes, node: object) -> str:
    match = PLUGIN_IMPL_RE.search(impl_header_text(source, node))
    return match.group("name") if match is not None else ""


def enclosing_context(source: bytes, node: object) -> str:
    function_name = ""
    current = node.parent
    while current is not None:
        if current.type == "function_item" and not function_name:
            function_name = name_child_text(source, current)
        elif current.type == "impl_item":
            plugin_name = plugin_name_from_impl(source, current)
            if plugin_name and function_name:
                return f"{plugin_name}::{function_name}"
            if plugin_name:
                return plugin_name
        current = current.parent
    return function_name


def module_bucket(file: str) -> str:
    prefix = "crates/ambition_sandbox/src/"
    rel = file[len(prefix) :] if file.startswith(prefix) else file
    rel = rel.removesuffix(".rs")
    parts = rel.split("/")
    if not parts:
        return rel
    if parts[0] == "content" and len(parts) > 1 and parts[1] == "features":
        if len(parts) > 2 and parts[2] == "ecs":
            return "content/features/ecs"
        return "content/features"
    if parts[0] == "presentation" and len(parts) > 1:
        return "presentation/" + parts[1]
    if parts[0] == "world" and len(parts) > 1:
        return "world/" + parts[1]
    if parts[0] == "player" and len(parts) > 1 and parts[1] == "affordances":
        return "player/affordances"
    return parts[0]


def classify_plain_item(
    file: str, name: str, derives: Sequence[str], visibility: str
) -> tuple[str, int, str, list[str]]:
    rel_parts = set(file.split("/"))
    reasons: list[str] = []
    score = 0
    category = "support"

    if (
        "content" in rel_parts
        or "engine_core" in rel_parts
        or "world" in rel_parts
        or "player" in rel_parts
    ):
        score += 2
        category = "domain_model"
        reasons.append("game/domain path")
    if "features" in rel_parts and "ecs" not in rel_parts:
        score += 3
        category = "legacy_feature_model"
        reasons.append("feature model outside content/features/ecs")
    if STATEFUL_NAME_RE.search(name):
        score += 2
        reasons.append("stateful/domain-style name")
    if set(derives) & {"Serialize", "Deserialize"}:
        score += 1
        reasons.append("serialized data")
    if set(derives) & {"Default", "Clone", "Copy"}:
        score += 1
        reasons.append("value-like runtime data")
    if visibility.startswith("pub"):
        score += 1
        reasons.append("public API")
    if rel_parts & LOW_SIGNAL_PATH_PARTS:
        score -= 2
        reasons.append("likely presentation/tooling/support path")
    if set(derives) & ARCHITECTURE_DERIVES:
        category = "bevy_architecture_marker"
        score = max(score, 1)
        reasons.append("Bevy architecture derive")

    if score >= 5:
        priority = "high"
    elif score >= 3:
        priority = "medium"
    else:
        priority = "low"
    return category, score, priority, reasons


def collect_plain_items(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> tuple[list[PlainItemRecord], list[PlainItemRecord]]:
    plain: list[PlainItemRecord] = []
    architecture: list[PlainItemRecord] = []
    for parsed in iter_parsed_rs_files(crate_root, include_tests):
        for node in iter_named_descendants(parsed.root):
            if node.type not in ITEM_NODE_TYPES:
                continue
            if not include_tests and is_in_cfg_test_context(parsed.source, node):
                continue
            attrs = attrs_immediately_before(parsed.source, node)
            derives = sorted(set(derive_names(attrs)))
            if set(derives) & ECS_DERIVES:
                continue
            file = repo_rel(parsed.path, repo_root)
            name = name_child_text(parsed.source, node)
            visibility = direct_visibility(parsed.source, node)
            category, score, priority, reasons = classify_plain_item(
                file, name, derives, visibility
            )
            record = PlainItemRecord(
                name=name,
                kind=item_kind(node.type),
                derives=derives,
                file=file,
                line=node_line(node),
                visibility=visibility,
                category=category,
                migration_score=score,
                migration_priority=priority,
                reasons=reasons,
            )
            if set(derives) & ARCHITECTURE_DERIVES:
                architecture.append(record)
            else:
                plain.append(record)
    return plain, architecture


def collect_items(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> list[ItemRecord]:
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


def collect_plugins(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> list[PluginRecord]:
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


def collect_system_like_functions(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> list[FunctionRecord]:
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
            if not (
                collect_type_names(parsed.source, params_node) & SYSTEM_PARAM_NAMES
            ):
                continue
            params_text = node_text(parsed.source, params_node).strip()[1:-1]
            access = analyze_system_params(params_text)
            records.append(
                FunctionRecord(
                    name=name_child_text(parsed.source, node),
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    public=bool(direct_visibility(parsed.source, node)),
                    params=compact(params_text, 240),
                    commands=bool(access["commands"]),
                    queries=access["queries"],
                    resources_read=access["resources_read"],
                    resources_written=access["resources_written"],
                    messages_read=access["messages_read"],
                    messages_written=access["messages_written"],
                    locals=access["locals"],
                )
            )
    return records


def collect_registrations(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> list[RegistrationRecord]:
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
            details = registration_breakdown(parsed.source, name, args, function_node)
            records.append(
                RegistrationRecord(
                    kind=name,
                    file=repo_rel(parsed.path, repo_root),
                    line=node_line(node),
                    schedule_or_arg=compact(first_arg, 160),
                    expression=compact(expression, 360),
                    identifiers=collect_identifiers(parsed.source, identifier_roots),
                    context=enclosing_context(parsed.source, node),
                    systems=details["systems"],
                    run_conditions=details["run_conditions"],
                    sets=details["sets"],
                    ordering=details["ordering"],
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


def collect_spawns(
    crate_root: pathlib.Path, repo_root: pathlib.Path, include_tests: bool
) -> list[SpawnRecord]:
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
    grouped: dict[str, list[ItemRecord]] = {
        derive: [] for derive in sorted(ECS_DERIVES)
    }
    for item in items:
        for derive in item.derives:
            grouped.setdefault(derive, []).append(item)
    return grouped


def priority_sort_key(row: PlainItemRecord) -> tuple[int, str, str, int]:
    priority_rank = {"high": 0, "medium": 1, "low": 2}.get(row.migration_priority, 3)
    return (priority_rank, row.file, row.name, row.line)


def summarize_modules(
    items: Sequence[ItemRecord],
    functions: Sequence[FunctionRecord],
    registrations: Sequence[RegistrationRecord],
    spawns: Sequence[SpawnRecord],
    plugins: Sequence[PluginRecord],
    plain_items: Sequence[PlainItemRecord],
) -> list[ModuleSummaryRecord]:
    accum: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for item in items:
        module = module_bucket(item.file)
        accum[module]["ecs_items"] += 1
        for derive in item.derives:
            if derive == "Component":
                accum[module]["components"] += 1
            elif derive == "Resource":
                accum[module]["resources"] += 1
            elif derive in {"Message", "Event"}:
                accum[module]["messages"] += 1
    for function in functions:
        accum[module_bucket(function.file)]["system_like_functions"] += 1
    for registration in registrations:
        accum[module_bucket(registration.file)]["registered_systems"] += len(
            registration.systems
        )
    for spawn in spawns:
        accum[module_bucket(spawn.file)]["spawn_sites"] += 1
    for plugin in plugins:
        accum[module_bucket(plugin.file)]["plugins"] += 1
    for row in plain_items:
        module = module_bucket(row.file)
        accum[module]["non_ecs_items"] += 1
        if row.migration_priority in {"high", "medium"}:
            accum[module]["migration_candidates"] += 1
    return [
        ModuleSummaryRecord(module=module, **counts)
        for module, counts in sorted(accum.items())
    ]


def build_message_bus(
    functions: Sequence[FunctionRecord], registrations: Sequence[RegistrationRecord]
) -> dict[str, dict[str, list[str]]]:
    bus: dict[str, dict[str, list[str]]] = defaultdict(
        lambda: {"registered_at": [], "read_by": [], "written_by": []}
    )
    for registration in registrations:
        if registration.kind not in {"add_message", "add_event"}:
            continue
        for ident in registration.identifiers:
            last = identifier_last(ident)
            if last not in {"app"} and re.match(r"^[A-Z]", last):
                bus[last]["registered_at"].append(
                    f"{registration.file}:{registration.line}"
                )
    for function in functions:
        fn_ref = f"{function.name} ({function.file}:{function.line})"
        for msg in function.messages_read:
            bus[compact_type(msg)]["read_by"].append(fn_ref)
        for msg in function.messages_written:
            bus[compact_type(msg)]["written_by"].append(fn_ref)
    return {
        key: {subkey: unique_ordered(values) for subkey, values in value.items()}
        for key, value in sorted(bus.items())
    }


def build_resource_access(
    functions: Sequence[FunctionRecord],
) -> dict[str, dict[str, list[str]]]:
    access: dict[str, dict[str, list[str]]] = defaultdict(
        lambda: {"read_by": [], "written_by": []}
    )
    for function in functions:
        fn_ref = f"{function.name} ({function.file}:{function.line})"
        for resource in function.resources_read:
            access[compact_type(resource)]["read_by"].append(fn_ref)
        for resource in function.resources_written:
            access[compact_type(resource)]["written_by"].append(fn_ref)
    return {
        key: {subkey: unique_ordered(values) for subkey, values in value.items()}
        for key, value in sorted(access.items())
    }


def append_module_summary(
    out: list[str], summaries: Sequence[ModuleSummaryRecord]
) -> None:
    out.append("## Architecture summary by module")
    out.append("")
    out.append(
        "| Module | ECS items | Components | Resources | Messages | Plugins | Registered systems | System-like fns | Spawns | Non-ECS items | Migration candidates |"
    )
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for row in sorted(
        summaries,
        key=lambda r: (
            -r.registered_systems - r.ecs_items - r.migration_candidates,
            r.module,
        ),
    ):
        out.append(
            f"| `{row.module}` | {row.ecs_items} | {row.components} | {row.resources} | "
            f"{row.messages} | {row.plugins} | {row.registered_systems} | "
            f"{row.system_like_functions} | {row.spawn_sites} | {row.non_ecs_items} | "
            f"{row.migration_candidates} |"
        )
    out.append("")


def append_schedule_map(
    out: list[str], registrations: Sequence[RegistrationRecord]
) -> None:
    out.append("## Schedule and system-set map")
    out.append("")
    out.append(
        "This section is derived from `add_systems` calls. Run conditions and ordering modifiers are separated from likely system functions when tree-sitter can identify the call structure."
    )
    out.append("")
    by_schedule_set: dict[tuple[str, str], list[RegistrationRecord]] = defaultdict(list)
    for row in registrations:
        if row.kind != "add_systems":
            continue
        schedule = row.schedule_or_arg or "<unspecified>"
        sets = row.sets or ["<no explicit set>"]
        for set_name in sets:
            by_schedule_set[(schedule, set_name)].append(row)
    if not by_schedule_set:
        out.append("- None found.")
        out.append("")
        return
    for (schedule, set_name), rows in sorted(by_schedule_set.items()):
        system_count = sum(len(row.systems) for row in rows)
        out.append(f"### `{schedule}` / `{set_name}` ({system_count} systems)")
        for row in rows:
            context = f" in `{row.context}`" if row.context else ""
            out.append(f"- `{row.file}:{row.line}`{context}")
            if row.systems:
                out.append(
                    "  - systems: " + ", ".join(f"`{system}`" for system in row.systems)
                )
            if row.run_conditions:
                out.append(
                    "  - run if: "
                    + ", ".join(f"`{cond}`" for cond in row.run_conditions)
                )
            if row.ordering:
                out.append(
                    "  - ordering: " + ", ".join(f"`{item}`" for item in row.ordering)
                )
            if not row.systems and row.identifiers:
                out.append(
                    "  - identifiers: "
                    + ", ".join(f"`{ident}`" for ident in row.identifiers[:20])
                )
        out.append("")


def append_message_bus(
    out: list[str], message_bus: dict[str, dict[str, list[str]]]
) -> None:
    out.append("## Message bus")
    out.append("")
    if not message_bus:
        out.append("- None found.")
        out.append("")
        return
    out.append("| Message | Registered | Producers | Consumers |")
    out.append("|---|---:|---:|---:|")
    for message, info in sorted(
        message_bus.items(),
        key=lambda kv: (-(len(kv[1]["read_by"]) + len(kv[1]["written_by"])), kv[0]),
    ):
        out.append(
            f"| `{message}` | {len(info['registered_at'])} | {len(info['written_by'])} | {len(info['read_by'])} |"
        )
    out.append("")
    for message, info in sorted(message_bus.items()):
        out.append(f"### `{message}`")
        if info["registered_at"]:
            out.append(
                "- registered at: "
                + ", ".join(f"`{x}`" for x in info["registered_at"][:8])
            )
        if info["written_by"]:
            out.append("- written by:")
            for writer in info["written_by"][:12]:
                out.append(f"  - `{writer}`")
            if len(info["written_by"]) > 12:
                out.append(f"  - ... {len(info['written_by']) - 12} more")
        if info["read_by"]:
            out.append("- read by:")
            for reader in info["read_by"][:12]:
                out.append(f"  - `{reader}`")
            if len(info["read_by"]) > 12:
                out.append(f"  - ... {len(info['read_by']) - 12} more")
        out.append("")


def append_resource_access(
    out: list[str], resource_access: dict[str, dict[str, list[str]]]
) -> None:
    out.append("## Resource access hotspots")
    out.append("")
    if not resource_access:
        out.append("- None found.")
        out.append("")
        return
    rows = sorted(
        resource_access.items(),
        key=lambda kv: (-(len(kv[1]["written_by"])), -(len(kv[1]["read_by"])), kv[0]),
    )
    out.append("| Resource | Mut writers | Readers |")
    out.append("|---|---:|---:|")
    for resource, info in rows[:40]:
        out.append(
            f"| `{resource}` | {len(info['written_by'])} | {len(info['read_by'])} |"
        )
    out.append("")
    out.append("### Mutable resource writers")
    for resource, info in rows[:30]:
        if not info["written_by"]:
            continue
        out.append(f"- `{resource}`")
        for writer in info["written_by"][:10]:
            out.append(f"  - `{writer}`")
        if len(info["written_by"]) > 10:
            out.append(f"  - ... {len(info['written_by']) - 10} more")
    out.append("")


def append_non_ecs_inventory(
    out: list[str],
    plain_items: Sequence[PlainItemRecord],
    architecture_items: Sequence[PlainItemRecord],
) -> None:
    out.append("## Non-ECS Rust data/model inventory")
    out.append("")
    out.append(
        "These are Rust structs/enums/unions that do not derive Component, Bundle, Resource, Message, or Event. The priority is heuristic: it is meant to highlight likely legacy/domain state that may deserve an ECS migration review, not to make a semantic claim."
    )
    out.append("")
    candidates = [
        row for row in plain_items if row.migration_priority in {"high", "medium"}
    ]
    out.append(f"- Total non-ECS items: {len(plain_items)}")
    out.append(f"- High/medium migration candidates: {len(candidates)}")
    out.append(f"- Bevy architecture marker types: {len(architecture_items)}")
    out.append("")

    if architecture_items:
        out.append("### Bevy architecture marker types")
        for row in sorted(architecture_items, key=lambda r: (r.file, r.line)):
            derives = f"; derives {', '.join(row.derives)}" if row.derives else ""
            out.append(f"- `{row.name}` ({row.kind}, `{row.file}:{row.line}`{derives})")
        out.append("")

    out.append("### High/medium migration candidates")
    if not candidates:
        out.append("- None found.")
    else:
        out.append("| Priority | Score | Item | Kind | Location | Reasons |")
        out.append("|---|---:|---|---|---|---|")
        for row in sorted(candidates, key=priority_sort_key)[:120]:
            reasons = "; ".join(row.reasons)
            out.append(
                f"| {row.migration_priority} | {row.migration_score} | `{row.name}` | {row.kind} | `{row.file}:{row.line}` | {reasons} |"
            )
        if len(candidates) > 120:
            out.append("")
            out.append(
                f"_... {len(candidates) - 120} additional candidates are present in JSON._"
            )
    out.append("")

    out.append("### Complete non-ECS item index by module")
    by_module: dict[str, list[PlainItemRecord]] = defaultdict(list)
    for row in plain_items:
        by_module[module_bucket(row.file)].append(row)
    for module in sorted(by_module):
        rows = sorted(by_module[module], key=lambda r: (r.file, r.line, r.name))
        out.append(f"#### `{module}` ({len(rows)} items)")
        for row in rows:
            derives = f"; derives {', '.join(row.derives)}" if row.derives else ""
            out.append(
                f"- `{row.name}` ({row.kind}, `{row.file}:{row.line}`, priority {row.migration_priority}, score {row.migration_score}{derives})"
            )
        out.append("")


def write_markdown(inventory: dict, path: pathlib.Path) -> None:
    items = [ItemRecord(**item) for item in inventory["ecs_items"]]
    grouped_items = group_by_derive(items)
    functions = [FunctionRecord(**row) for row in inventory["system_like_functions"]]
    registrations = [RegistrationRecord(**row) for row in inventory["registrations"]]
    spawns = [SpawnRecord(**row) for row in inventory["spawn_sites"]]
    plugins = [PluginRecord(**row) for row in inventory["plugins"]]
    plain_items = [PlainItemRecord(**row) for row in inventory.get("non_ecs_items", [])]
    architecture_items = [
        PlainItemRecord(**row) for row in inventory.get("architecture_items", [])
    ]
    module_summaries = [
        ModuleSummaryRecord(**row) for row in inventory.get("module_summaries", [])
    ]
    message_bus = inventory.get("message_bus", {})
    resource_access = inventory.get("resource_access", {})

    out: list[str] = []
    out.append("# Ambition Sandbox ECS inventory")
    out.append("")
    out.append(f"Generated from `{inventory['crate_root']}`.")
    out.append("")
    out.append("## Counts")
    for key, value in inventory["counts"].items():
        out.append(f"- {key.replace('_', ' ').title()}: {value}")
    out.append("")

    if module_summaries:
        append_module_summary(out, module_summaries)
    append_schedule_map(out, registrations)
    append_message_bus(out, message_bus)
    append_resource_access(out, resource_access)
    append_non_ecs_inventory(out, plain_items, architecture_items)

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
            context = f" in `{row.context}`" if row.context else ""
            out.append(
                f"- `{row.file}:{row.line}`{context} - `{row.kind}` on/with `{row.schedule_or_arg or '<none>'}`"
            )
            if row.systems:
                out.append(
                    "  - systems: " + ", ".join(f"`{system}`" for system in row.systems)
                )
            if row.sets:
                out.append(
                    "  - sets: " + ", ".join(f"`{set_name}`" for set_name in row.sets)
                )
            if row.run_conditions:
                out.append(
                    "  - run if: "
                    + ", ".join(f"`{cond}`" for cond in row.run_conditions)
                )
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
                details = []
                if row.commands:
                    details.append("Commands")
                if row.resources_written:
                    details.append("writes " + ", ".join(row.resources_written[:6]))
                if row.resources_read:
                    details.append("reads " + ", ".join(row.resources_read[:6]))
                if row.messages_written:
                    details.append(
                        "writes messages " + ", ".join(row.messages_written[:6])
                    )
                if row.messages_read:
                    details.append("reads messages " + ", ".join(row.messages_read[:6]))
                if row.queries:
                    details.append(f"{len(row.queries)} queries")
                if details:
                    out.append("    - " + "; ".join(details))
    out.append("")

    out.append("## Entity archetype evidence / spawn sites")
    out.append(
        "Static analysis cannot know every runtime entity instance. This section lists spawn sites and the bundle/component/type identifiers found in each spawn expression."
    )
    if not spawns:
        out.append("- None found.")
    else:
        for row in spawns:
            out.append(f"- `{row.file}:{row.line}`")
            if row.name_labels:
                for label in row.name_labels:
                    out.append(f"  - name label: `{label}`")
            if row.matched_ecs_items:
                out.append(
                    "  - matched ECS items: "
                    + ", ".join(f"`{item}`" for item in row.matched_ecs_items[:30])
                )
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


def build_inventory(
    repo_root: pathlib.Path, crate_root: pathlib.Path, include_tests: bool
) -> dict:
    items = collect_items(crate_root, repo_root, include_tests)
    functions = collect_system_like_functions(crate_root, repo_root, include_tests)
    registrations = collect_registrations(crate_root, repo_root, include_tests)
    spawns = collect_spawns(crate_root, repo_root, include_tests)
    plugins = collect_plugins(crate_root, repo_root, include_tests)
    plain_items, architecture_items = collect_plain_items(
        crate_root, repo_root, include_tests
    )
    grouped = group_by_derive(items)
    ecs_item_names = {item.name for item in items}
    spawns = [
        dataclasses.replace(
            row,
            matched_ecs_items=unique_ordered(
                ident
                for ident in row.identifiers
                if identifier_last(ident) in ecs_item_names
            ),
        )
        for row in spawns
    ]
    unique_registration_identifiers = sorted(
        {ident for row in registrations for ident in row.identifiers}
    )
    module_summaries = summarize_modules(
        items, functions, registrations, spawns, plugins, plain_items
    )
    message_bus = build_message_bus(functions, registrations)
    resource_access = build_resource_access(functions)
    registered_system_names = sorted(
        {identifier_last(system) for row in registrations for system in row.systems}
    )
    inventory = {
        "schema_version": 2,
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
            "registered_systems": len(registered_system_names),
            "module_summaries": len(module_summaries),
            "non_ecs_items": len(plain_items),
            "migration_candidates_high": sum(
                1 for row in plain_items if row.migration_priority == "high"
            ),
            "migration_candidates_medium": sum(
                1 for row in plain_items if row.migration_priority == "medium"
            ),
            "architecture_items": len(architecture_items),
            "message_channels": len(message_bus),
            "resource_access_entries": len(resource_access),
        },
        "ecs_items": asdict_list(items),
        "plugins": asdict_list(plugins),
        "registrations": asdict_list(registrations),
        "unique_registration_identifiers": unique_registration_identifiers,
        "system_like_functions": asdict_list(functions),
        "spawn_sites": asdict_list(spawns),
        "registered_systems": registered_system_names,
        "module_summaries": asdict_list(module_summaries),
        "message_bus": message_bus,
        "resource_access": resource_access,
        "non_ecs_items": asdict_list(plain_items),
        "architecture_items": asdict_list(architecture_items),
    }
    return inventory


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument(
        "--crate", type=pathlib.Path, default=pathlib.Path("crates/ambition_sandbox")
    )
    parser.add_argument(
        "--json",
        type=pathlib.Path,
        default=pathlib.Path("target/ambition_ecs_inventory.json"),
    )
    parser.add_argument(
        "--markdown",
        type=pathlib.Path,
        default=pathlib.Path("target/ambition_ecs_inventory.md"),
    )
    parser.add_argument("--include-tests", action="store_true")
    parser.add_argument(
        "--check-json",
        type=pathlib.Path,
        help="Compare generated JSON with an existing inventory file.",
    )
    args = parser.parse_args(argv)

    repo_root = args.repo_root.resolve()
    crate_root = args.crate if args.crate.is_absolute() else repo_root / args.crate
    crate_root = crate_root.resolve()
    if not (crate_root / "src").is_dir():
        print(
            f"error: crate source directory not found: {crate_root / 'src'}",
            file=sys.stderr,
        )
        return 2

    inventory = build_inventory(repo_root, crate_root, args.include_tests)

    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(
        json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
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
