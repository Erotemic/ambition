#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "tree-sitter>=0.25,<0.26",
#   "tree-sitter-rust>=0.24,<0.25",
# ]
# ///
"""Build a static inventory of non-ECS Rust code in ambition_actors.

This complements tools/ecs_inventory.py. The ECS inventory answers "what is
already Bevy ECS?" This script answers "what game-facing code is not directly
an ECS Component/Bundle/Resource/Message/Event yet, and which pieces look worth
reviewing for ECS migration?"

It is intentionally heuristic. It uses tree-sitter-rust for syntax structure,
then reports explainable evidence instead of pretending to be rustc.

The report focuses on:
  * Non-ECS structs/enums/type aliases/traits, after subtracting direct Bevy ECS
    derives and architecture marker derives.
  * ECS-adjacent non-ECS types referenced by components, resources, messages, or
    Bevy system signatures/bodies.
  * Collection fields that may be hiding entity-like runtime state inside a
    Resource or plain model.
  * Runtime-state, content-data, save-data, UI/presentation, platform/backend,
    and utility classifications.
  * Migration-priority scoring with reasons and suggested targets.

Run directly with an inline-metadata aware launcher, for example:

    uv run tools/non_ecs_inventory.py
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
except ImportError:  # pragma: no cover - exercised only without deps installed.
    print(
        "error: missing tree-sitter dependencies. Try: uv run tools/non_ecs_inventory.py",
        file=sys.stderr,
    )
    raise


ECS_DERIVES = {"Component", "Bundle", "Resource", "Message", "Event"}
ARCHITECTURE_DERIVES = {"SystemParam", "SystemSet", "States", "SubStates"}
BEVY_MARKER_DERIVES = ECS_DERIVES | ARCHITECTURE_DERIVES
DEFAULT_EXCLUDED_DIR_NAMES = {"target", ".git"}
DEFAULT_EXCLUDED_PATH_PARTS = {"tests"}

ITEM_NODE_TYPES = {"struct_item", "enum_item", "union_item", "type_item", "trait_item"}
TYPE_ITEM_NODE_TYPES = {"struct_item", "enum_item", "union_item", "type_item"}
IDENTIFIER_NODE_TYPES = {"identifier", "type_identifier", "field_identifier"}
SCOPED_IDENTIFIER_NODE_TYPES = {"scoped_identifier", "scoped_type_identifier"}

DERIVE_ATTR_RE = re.compile(r"#\s*\[\s*derive\s*\((.*?)\)\s*\]", flags=re.DOTALL)
QUALIFIED_IDENT_RE = re.compile(
    r"(?:(?:crate|super|self)::)?[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*"
)

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

COLLECTION_TYPES = {
    "Vec",
    "VecDeque",
    "HashMap",
    "BTreeMap",
    "IndexMap",
    "HashSet",
    "BTreeSet",
    "SlotMap",
    "SecondaryMap",
    "SparseSecondaryMap",
    "SmallVec",
    "ArrayVec",
}
MAP_LIKE_COLLECTIONS = {
    "HashMap",
    "BTreeMap",
    "IndexMap",
    "SlotMap",
    "SecondaryMap",
    "SparseSecondaryMap",
}
SET_LIKE_COLLECTIONS = {"HashSet", "BTreeSet"}

LOW_SIGNAL_PATH_PARTS = {
    "assets",
    "audio",
    "dev",
    "host",
    "input",
    "music",
    "presentation",
    "rendering",
    "ui",
}
HIGH_SIGNAL_PATH_PARTS = {
    "app",
    "brain",
    "content",
    "encounter",
    "engine_core",
    "features",
    "input",
    "inventory",
    "persistence",
    "player",
    "runtime",
    "time",
    "world",
}

RUNTIME_NAME_RE = re.compile(
    r"(Runtime|State|Cache|Queue|Buffer|Index|Registry|Tracker|Controller|Director|Runner|Session|Context)$"
)
DOMAIN_NAME_RE = re.compile(
    r"(Spec|Data|Config|Definition|Catalog|Profile|Tuning|Template|Entry|Model)$"
)
ENTITY_NAME_RE = re.compile(
    r"(Actor|Player|Enemy|Boss|Npc|Feature|Projectile|Room|Encounter|Quest|Inventory|Hitbox|Platform|Portal)"
)
MUTABLE_FIELD_RE = re.compile(
    r"(state|status|timer|cooldown|health|mana|position|velocity|accel|intent|target|phase|queue|pending|active|current|previous|runtime|cache|dirty|flags?)",
    flags=re.IGNORECASE,
)
ID_FIELD_RE = re.compile(r"(^id$|_id$|key$|handle$|entity$|slot$)", flags=re.IGNORECASE)
SAVE_PATH_RE = re.compile(r"/(persistence|save|settings)(/|$|\.rs$)")
VALUE_OBJECT_NAME_RE = re.compile(
    r"^(Aabb|Rect|Rectangle|Bounds|Point|Line|Segment|Range|Interval|GridPos|TilePos|PixelRect|NormPoint|FrameRect)$"
)
UI_PATH_RE = re.compile(r"/(ui|menu|hud|overlay|presentation|rendering)(/|$)")
PLATFORM_PATH_RE = re.compile(r"/(host|platform|audio|assets|music)(/|$)")

STOPWORD_TYPE_NAMES = {
    "Self",
    "Option",
    "Result",
    "Vec",
    "VecDeque",
    "HashMap",
    "BTreeMap",
    "IndexMap",
    "HashSet",
    "BTreeSet",
    "Box",
    "Arc",
    "Rc",
    "Cow",
    "String",
    "str",
    "bool",
    "usize",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "f32",
    "f64",
    "Entity",
    "Commands",
    "Query",
    "Res",
    "ResMut",
    "Local",
    "Assets",
    "Handle",
    "Name",
    "Transform",
    "GlobalTransform",
    "Vec2",
    "Vec3",
    "IVec2",
    "UVec2",
    "Quat",
    "Color",
    "Timer",
    "Duration",
    "PhantomData",
    "Default",
    "Debug",
    "Clone",
    "Copy",
    "Serialize",
    "Deserialize",
}


@dataclasses.dataclass(frozen=True)
class FieldRecord:
    name: str
    type: str
    file: str
    line: int
    visibility: str = ""
    referenced_types: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class ItemRecord:
    name: str
    kind: str
    file: str
    line: int
    visibility: str
    derives: list[str]
    attrs: list[str]
    fields: list[FieldRecord]
    variants: list[str]
    referenced_types: list[str]
    ecs_kind: str = ""
    architecture_kind: str = ""
    category: str = "unknown"
    suggested_target: str = "Unknown"
    migration_score: int = 0
    migration_priority: str = "low"
    reasons: list[str] = dataclasses.field(default_factory=list)
    methods: list[str] = dataclasses.field(default_factory=list)
    trait_impls: list[str] = dataclasses.field(default_factory=list)
    used_by_ecs_fields: list[str] = dataclasses.field(default_factory=list)
    used_by_systems: list[str] = dataclasses.field(default_factory=list)
    used_by_functions: list[str] = dataclasses.field(default_factory=list)
    collection_payload_for: list[str] = dataclasses.field(default_factory=list)


@dataclasses.dataclass(frozen=True)
class FunctionRecord:
    name: str
    file: str
    line: int
    owner: str
    visibility: str
    params: str
    return_type: str
    referenced_types: list[str]
    body_referenced_types: list[str]
    ecs_access: dict[str, object]
    non_ecs_types: list[str]
    is_system_like: bool


@dataclasses.dataclass(frozen=True)
class CollectionRecord:
    owner: str
    owner_kind: str
    owner_ecs_kind: str
    field: str
    file: str
    line: int
    container: str
    field_type: str
    payload_type: str
    payload_category: str
    suggested_target: str
    priority: str
    reasons: list[str]


@dataclasses.dataclass(frozen=True)
class ModuleSummaryRecord:
    module: str
    non_ecs_items: int = 0
    high_priority: int = 0
    medium_priority: int = 0
    runtime_state: int = 0
    content_data: int = 0
    save_data: int = 0
    ui_presentation: int = 0
    platform_backend: int = 0
    utility: int = 0
    ecs_adjacent: int = 0
    hidden_collections: int = 0


@dataclasses.dataclass(frozen=True)
class ParsedRustFile:
    path: pathlib.Path
    text: str
    source: bytes
    root: object


# ---------------------------------------------------------------------------
# tree-sitter compatibility helpers


def rust_language() -> Language:
    raw_language = tsrust.language()
    if isinstance(raw_language, Language):
        return raw_language
    return Language(raw_language)


RUST_LANGUAGE = rust_language()


def make_parser() -> Parser:
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


def iter_rs_files(
    crate_root: pathlib.Path, include_tests: bool
) -> Iterator[pathlib.Path]:
    src_root = crate_root / "src"
    for path in sorted(src_root.rglob("*.rs")):
        parts = set(path.parts)
        if any(part in DEFAULT_EXCLUDED_DIR_NAMES for part in parts):
            continue
        if not include_tests and DEFAULT_EXCLUDED_PATH_PARTS.intersection(parts):
            continue
        if not include_tests and path.name in {"tests.rs", "test.rs"}:
            continue
        if not include_tests and path.name.endswith("_tests.rs"):
            continue
        if not include_tests and "/bin/" in path.as_posix():
            continue
        yield path


def iter_parsed_rs_files(
    crate_root: pathlib.Path, include_tests: bool
) -> Iterator[ParsedRustFile]:
    for path in iter_rs_files(crate_root, include_tests):
        yield parse_rust_file(path)


# ---------------------------------------------------------------------------
# Text helpers


def compact(text: str, max_len: int = 180) -> str:
    text = re.sub(r"\s+", " ", text.strip())
    return text if len(text) <= max_len else text[: max_len - 3] + "..."


def unique_ordered(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for value in values:
        value = value.strip()
        if value and value not in seen:
            out.append(value)
            seen.add(value)
    return out


def direct_visibility(source: bytes, node: object) -> str:
    for child in named_children(node):
        if child.type == "visibility_modifier":
            return compact(node_text(source, child), 80)
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


def normalize_identifier_text(text: str) -> str:
    text = text.strip().removeprefix("r#")
    if text.endswith("!"):
        text = text[:-1]
    return text


def identifier_last(text: str) -> str:
    text = normalize_identifier_text(text)
    text = text.split("<", 1)[0]
    text = text.split(".")[-1]
    return text.split("::")[-1]


def name_child_text(source: bytes, node: object) -> str:
    name = child_by_field_name(node, "name")
    if name is not None:
        return normalize_identifier_text(node_text(source, name))
    for child in named_children(node):
        if child.type in {"identifier", "type_identifier"}:
            return normalize_identifier_text(node_text(source, child))
    return ""


def item_kind(node_type: str) -> str:
    return node_type.removesuffix("_item")


def collect_type_names(source: bytes, root: object) -> list[str]:
    names: list[str] = []
    for node in iter_named_descendants(root):
        if node.type in {"type_identifier", "identifier"}:
            name = identifier_last(node_text(source, node))
            if name and name not in STOPWORD_TYPE_NAMES:
                names.append(name)
        elif node.type in SCOPED_IDENTIFIER_NODE_TYPES:
            name = identifier_last(node_text(source, node))
            if name and name not in STOPWORD_TYPE_NAMES:
                names.append(name)
    return unique_ordered(names)


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
    text = type_text.strip()
    text = re.sub(r"\b(?:crate|super|self)::", "", text)
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


def has_system_param(params: str) -> bool:
    return any(
        re.search(rf"\b{re.escape(name)}\b", params) for name in SYSTEM_PARAM_NAMES
    )


# ---------------------------------------------------------------------------
# Rust AST extraction


def field_type_node(field_node: object) -> object | None:
    explicit = child_by_field_name(field_node, "type")
    if explicit is not None:
        return explicit
    named = named_children(field_node)
    if not named:
        return None
    # field_declaration usually has name then type; tuple fields often only have type.
    candidates = [
        child
        for child in named
        if child.type not in {"visibility_modifier", "field_identifier", "identifier"}
    ]
    return candidates[-1] if candidates else named[-1]


def collect_fields(source: bytes, item_node: object, file: str) -> list[FieldRecord]:
    fields: list[FieldRecord] = []
    for node in iter_named_descendants(item_node):
        if node.type not in {"field_declaration", "ordered_field_declaration"}:
            continue
        name_node = child_by_field_name(node, "name")
        name = node_text(source, name_node).strip() if name_node is not None else ""
        if not name:
            for child in named_children(node):
                if child.type == "field_identifier":
                    name = node_text(source, child).strip()
                    break
        type_node = field_type_node(node)
        type_text = (
            compact_type(node_text(source, type_node), 180)
            if type_node is not None
            else compact(node_text(source, node), 180)
        )
        referenced_types = (
            collect_type_names(source, type_node)
            if type_node is not None
            else collect_type_names(source, node)
        )
        fields.append(
            FieldRecord(
                name=name or f"field_{len(fields)}",
                type=type_text,
                file=file,
                line=node_line(node),
                visibility=direct_visibility(source, node),
                referenced_types=referenced_types,
            )
        )
    return fields


def collect_variants(source: bytes, item_node: object) -> list[str]:
    variants: list[str] = []
    for node in iter_named_descendants(item_node):
        if node.type == "enum_variant":
            name = name_child_text(source, node)
            if name:
                variants.append(name)
    return variants


def function_signature_parts(source: bytes, node: object) -> tuple[str, str, str]:
    params_node = child_by_field_name(node, "parameters")
    if params_node is None:
        for child in named_children(node):
            if child.type == "parameters":
                params_node = child
                break
    params = node_text(source, params_node) if params_node is not None else ""

    body_node = None
    for child in named_children(node):
        if child.type == "block":
            body_node = child
            break
    sig_end = body_node.start_byte if body_node is not None else node.end_byte
    sig_text = source[node.start_byte : sig_end].decode("utf-8")
    ret = ""
    if "->" in sig_text:
        ret = compact_type(sig_text.split("->", 1)[1], 120)
    return compact(sig_text, 220), compact(params, 220), ret


def function_body_node(node: object) -> object | None:
    for child in named_children(node):
        if child.type == "block":
            return child
    return None


def enclosing_impl_node(node: object) -> object | None:
    current = node.parent
    while current is not None:
        if current.type == "impl_item":
            return current
        if current.type in ITEM_NODE_TYPES:
            return None
        current = current.parent
    return None


def impl_header_text(source: bytes, node: object) -> str:
    for child in children(node):
        if child.type == "declaration_list":
            return source[node.start_byte : child.start_byte].decode("utf-8").strip()
    text = node_text(source, node)
    return text.split("{", 1)[0].strip()


def parse_impl_header(header: str) -> tuple[str, str]:
    # Return (target_type, trait_name). This is deliberately conservative and
    # only needs enough precision for inventory grouping.
    header = re.sub(r"\s+", " ", header.strip())
    if " for " in header:
        before, after = header.rsplit(" for ", 1)
        trait_names = QUALIFIED_IDENT_RE.findall(before)
        target_names = QUALIFIED_IDENT_RE.findall(after)
        trait = identifier_last(trait_names[-1]) if trait_names else ""
        target = identifier_last(target_names[0]) if target_names else ""
        return target, trait
    names = QUALIFIED_IDENT_RE.findall(header)
    names = [name for name in names if identifier_last(name) not in {"impl", "where"}]
    target = identifier_last(names[0]) if names else ""
    return target, ""


def collect_raw_items(
    parsed_files: Sequence[ParsedRustFile], repo_root: pathlib.Path, include_tests: bool
) -> tuple[dict[str, ItemRecord], dict[str, list[str]], dict[str, list[str]]]:
    items: dict[str, ItemRecord] = {}
    methods_by_type: dict[str, list[str]] = defaultdict(list)
    traits_by_type: dict[str, list[str]] = defaultdict(list)

    for parsed in parsed_files:
        file = repo_rel(parsed.path, repo_root)
        source = parsed.source
        for node in iter_named_descendants(parsed.root):
            if not include_tests and is_in_cfg_test_context(source, node):
                continue
            if node.type in ITEM_NODE_TYPES:
                name = name_child_text(source, node)
                if not name:
                    continue
                attrs = attrs_immediately_before(source, node)
                derives = sorted(set(derive_names(attrs)))
                ecs_kind = sorted(set(derives).intersection(ECS_DERIVES))
                arch_kind = sorted(set(derives).intersection(ARCHITECTURE_DERIVES))
                fields = (
                    collect_fields(source, node, file)
                    if node.type in {"struct_item", "union_item"}
                    else []
                )
                variants = (
                    collect_variants(source, node) if node.type == "enum_item" else []
                )
                ref_types = collect_type_names(source, node)
                record = ItemRecord(
                    name=name,
                    kind=item_kind(node.type),
                    file=file,
                    line=node_line(node),
                    visibility=direct_visibility(source, node),
                    derives=derives,
                    attrs=[compact(attr, 140) for attr in attrs],
                    fields=fields,
                    variants=variants,
                    referenced_types=ref_types,
                    ecs_kind=ecs_kind[0] if ecs_kind else "",
                    architecture_kind=arch_kind[0] if arch_kind else "",
                )
                # If duplicate local names exist, keep the first in the name map but
                # encode later duplicates with file-qualified suffix for completeness.
                key = name if name not in items else f"{name}@{file}:{record.line}"
                items[key] = record
            elif node.type == "impl_item":
                header = impl_header_text(source, node)
                target, trait = parse_impl_header(header)
                if not target:
                    continue
                if trait:
                    traits_by_type[target].append(trait)
                for child in iter_named_descendants(node):
                    if child.type == "function_item":
                        method_name = name_child_text(source, child)
                        if method_name:
                            methods_by_type[target].append(method_name)
    return items, methods_by_type, traits_by_type


def collect_functions(
    parsed_files: Sequence[ParsedRustFile],
    repo_root: pathlib.Path,
    include_tests: bool,
    item_names: set[str],
) -> list[FunctionRecord]:
    functions: list[FunctionRecord] = []
    for parsed in parsed_files:
        file = repo_rel(parsed.path, repo_root)
        source = parsed.source
        for node in iter_named_descendants(parsed.root):
            if node.type != "function_item":
                continue
            if not include_tests and is_in_cfg_test_context(source, node):
                continue
            name = name_child_text(source, node)
            if not name:
                continue
            _, params, ret = function_signature_parts(source, node)
            sig_types = unique_ordered(
                [t for t in collect_type_names(source, node) if t in item_names]
            )
            body = function_body_node(node)
            body_types = (
                unique_ordered(
                    [t for t in collect_type_names(source, body) if t in item_names]
                )
                if body is not None
                else []
            )
            owner = ""
            impl_node = enclosing_impl_node(node)
            if impl_node is not None:
                owner, _trait = parse_impl_header(impl_header_text(source, impl_node))
            ecs_access = analyze_system_params(params)
            is_system_like = has_system_param(params)
            functions.append(
                FunctionRecord(
                    name=name,
                    file=file,
                    line=node_line(node),
                    owner=owner,
                    visibility=direct_visibility(source, node),
                    params=params,
                    return_type=ret,
                    referenced_types=sig_types,
                    body_referenced_types=body_types,
                    ecs_access=ecs_access,
                    non_ecs_types=[],
                    is_system_like=is_system_like,
                )
            )
    return functions


# ---------------------------------------------------------------------------
# Classification and scoring


def module_name(file: str) -> str:
    marker = "crates/ambition_actors/src/"
    if marker in file:
        rel = file.split(marker, 1)[1]
    else:
        rel = file
    parts = rel.split("/")
    if len(parts) == 1:
        return parts[0].removesuffix(".rs")
    return "/".join(parts[:2])


def path_parts(file: str) -> set[str]:
    return set(file.replace("\\", "/").split("/"))


def field_names_text(fields: Sequence[FieldRecord]) -> str:
    return " ".join(field.name for field in fields)


def field_types_text(fields: Sequence[FieldRecord]) -> str:
    return " ".join(field.type for field in fields)


def classify_category(item: ItemRecord) -> tuple[str, list[str]]:
    reasons: list[str] = []
    file = "/" + item.file
    parts = path_parts(item.file)
    field_names = field_names_text(item.fields)
    field_types = field_types_text(item.fields)
    derives = set(item.derives)

    if item.kind == "trait":
        return "trait_or_interface", ["trait definition"]
    if VALUE_OBJECT_NAME_RE.search(item.name) or "/engine_core/geometry" in file:
        reasons.append("small reusable value/object geometry type")
        return "value_object", reasons
    if (
        SAVE_PATH_RE.search(file)
        or "Save" in item.name
        or item.name.startswith("Persisted")
        or "Settings" in item.name
    ):
        reasons.append("save/settings path or name")
        return "save_data", reasons
    if UI_PATH_RE.search(file) or any(
        term in item.name
        for term in [
            "Ui",
            "UI",
            "Visual",
            "Overlay",
            "Hud",
            "Menu",
            "Panel",
            "Button",
            "Text",
        ]
    ):
        reasons.append("UI/presentation path or name")
        return "ui_presentation", reasons
    if PLATFORM_PATH_RE.search(file):
        reasons.append("platform/asset/audio/music path")
        return "platform_backend", reasons
    if (
        "Asset" in derives
        or "TypePath" in derives
        or "Serialize" in derives
        or "Deserialize" in derives
    ):
        if DOMAIN_NAME_RE.search(item.name) or any(
            part in parts for part in {"content", "data", "catalog"}
        ):
            reasons.append("serializable or asset-like content model")
            return "content_data", reasons
    if DOMAIN_NAME_RE.search(item.name):
        reasons.append("domain/data model name")
        return "domain_model", reasons
    if RUNTIME_NAME_RE.search(item.name):
        reasons.append("runtime-state-like name")
        return "runtime_state", reasons
    if ENTITY_NAME_RE.search(item.name) and (
        MUTABLE_FIELD_RE.search(field_names) or MUTABLE_FIELD_RE.search(field_types)
    ):
        reasons.append("entity-like name with mutable/gameplay fields")
        return "runtime_state", reasons
    if any(part in parts for part in LOW_SIGNAL_PATH_PARTS):
        reasons.append("low migration-signal subsystem path")
        return "utility_or_backend", reasons
    if item.kind == "enum" and len(item.variants) and not item.fields:
        reasons.append("plain enum/state taxonomy")
        return "domain_model", reasons
    return "plain_model", reasons


def suggested_target_for_category(category: str, item: ItemRecord) -> str:
    if category == "runtime_state":
        if "Queue" in item.name or "Request" in item.name:
            return "Resource or Message"
        if ENTITY_NAME_RE.search(item.name):
            return "Component or Resource"
        return "Resource"
    if category in {"content_data", "domain_model", "save_data"}:
        return "Keep as data model or Asset"
    if category == "ui_presentation":
        return "Keep or split into UI Components"
    if category == "value_object":
        return "Keep as value object"
    if category == "platform_backend":
        return "Keep non-ECS backend"
    if category == "trait_or_interface":
        return "Keep as trait/interface"
    if category == "utility_or_backend":
        return "Probably keep non-ECS"
    return "Unknown"


def collection_payloads(field_type: str) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    for container in COLLECTION_TYPES:
        for inner in generic_inners(field_type, container):
            parts = split_top_level_commas(inner)
            parts = [part for part in parts if not part.strip().startswith("'")]
            if not parts:
                continue
            if container in MAP_LIKE_COLLECTIONS and len(parts) >= 2:
                payload = compact_type(parts[-1], 120)
            elif container in SET_LIKE_COLLECTIONS:
                payload = compact_type(parts[0], 120)
            else:
                payload = compact_type(parts[0], 120)
            found.append((container, payload))
    return found


def payload_item_name(payload: str, non_ecs_names: set[str]) -> str:
    candidates = [
        identifier_last(match.group(0))
        for match in QUALIFIED_IDENT_RE.finditer(payload)
    ]
    for candidate in reversed(candidates):
        if candidate in non_ecs_names:
            return candidate
    return ""


def priority_from_score(score: int) -> str:
    if score >= 8:
        return "high"
    if score >= 4:
        return "medium"
    return "low"


def score_item(
    item: ItemRecord,
    category: str,
    base_reasons: Sequence[str],
    used_by_ecs_fields: Sequence[str],
    used_by_systems: Sequence[str],
    used_by_functions: Sequence[str],
    collection_payload_for: Sequence[str],
    methods: Sequence[str],
) -> tuple[int, str, list[str]]:
    score = 0
    reasons = list(base_reasons)
    parts = path_parts(item.file)
    field_names = field_names_text(item.fields)
    field_types = field_types_text(item.fields)

    if category == "runtime_state":
        score += 3
        reasons.append("runtime state category")
    if category in {
        "content_data",
        "save_data",
        "domain_model",
        "platform_backend",
        "ui_presentation",
        "value_object",
    }:
        score -= 3
        reasons.append(f"{category} is often intentionally non-ECS")
    if used_by_ecs_fields:
        score += 3
        reasons.append("referenced by ECS item fields")
    if used_by_systems:
        score += 2
        reasons.append("referenced by system-like functions")
    if collection_payload_for:
        if category == "runtime_state":
            score += 4
        elif category in {"plain_model"}:
            score += 2
        elif category in {"domain_model", "content_data", "save_data"}:
            score += 1
        reasons.append("stored in collection field that may hide entity-like state")
    if MUTABLE_FIELD_RE.search(field_names) or MUTABLE_FIELD_RE.search(field_types):
        score += 2
        reasons.append("mutable/gameplay field names or types")
    if ENTITY_NAME_RE.search(item.name):
        score += 1
        reasons.append("entity/gameplay noun in name")
    if category == "value_object":
        score -= 3
    if len(methods) >= 10:
        score += 2
        reasons.append("many inherent methods")
    elif len(methods) >= 5:
        score += 1
        reasons.append("several inherent methods")
    if any(part in parts for part in HIGH_SIGNAL_PATH_PARTS):
        score += 1
        reasons.append("gameplay/simulation subsystem path")
    if any(part in parts for part in LOW_SIGNAL_PATH_PARTS):
        score -= 2
        reasons.append("presentation/platform/dev subsystem path")
    if item.kind == "trait":
        score -= 3
    if (
        item.kind == "enum"
        and len(item.variants)
        and not item.fields
        and category != "runtime_state"
    ):
        score -= 1
    if item.name.endswith("Error") or item.name.endswith("Plugin"):
        score -= 3
        reasons.append("error/plugin glue shape")

    score = max(0, score)
    return score, priority_from_score(score), unique_ordered(reasons)


def build_collection_records(
    all_items: dict[str, ItemRecord],
    non_ecs_names: set[str],
    classified: dict[str, ItemRecord],
) -> list[CollectionRecord]:
    records: list[CollectionRecord] = []
    # Use first non-qualified record for owner lookup by name.
    by_name = {item.name: item for item in all_items.values() if item.name not in {}}
    for owner in all_items.values():
        for field in owner.fields:
            for container, payload in collection_payloads(field.type):
                payload_name = payload_item_name(payload, non_ecs_names)
                if not payload_name:
                    continue
                payload_item = classified.get(payload_name)
                payload_category = payload_item.category if payload_item else "unknown"
                reasons = [
                    f"{owner.name}.{field.name} stores {container}<{payload_name}>"
                ]
                if owner.ecs_kind == "Resource":
                    reasons.append("collection lives inside an ECS Resource")
                if payload_item and payload_item.category == "runtime_state":
                    reasons.append("payload looks like runtime state")
                if ID_FIELD_RE.search(field.name):
                    reasons.append("field name is id/key/handle-like")
                if payload_item and ENTITY_NAME_RE.search(payload_item.name):
                    reasons.append("payload has entity/gameplay noun in name")

                if (
                    owner.ecs_kind == "Resource"
                    and payload_item
                    and payload_item.category == "runtime_state"
                ):
                    suggested = "Component-backed entities or Resource index"
                    priority = "high"
                elif owner.ecs_kind == "Resource":
                    suggested = "Review Resource collection boundary"
                    priority = "medium"
                elif payload_item and payload_item.category == "runtime_state":
                    suggested = "Component or Resource"
                    priority = "medium"
                else:
                    suggested = "Likely keep as data collection"
                    priority = "low"
                records.append(
                    CollectionRecord(
                        owner=owner.name,
                        owner_kind=owner.kind,
                        owner_ecs_kind=owner.ecs_kind,
                        field=field.name,
                        file=field.file,
                        line=field.line,
                        container=container,
                        field_type=field.type,
                        payload_type=payload_name,
                        payload_category=payload_category,
                        suggested_target=suggested,
                        priority=priority,
                        reasons=unique_ordered(reasons),
                    )
                )
    return records


def classify_and_link(
    all_items: dict[str, ItemRecord],
    functions: Sequence[FunctionRecord],
    methods_by_type: dict[str, list[str]],
    traits_by_type: dict[str, list[str]],
) -> tuple[
    list[ItemRecord],
    list[FunctionRecord],
    list[CollectionRecord],
    list[ModuleSummaryRecord],
]:
    # Direct ECS and architecture marker types are not the subject of this script.
    non_ecs_keys = [
        key
        for key, item in all_items.items()
        if not item.ecs_kind
        and not item.architecture_kind
        and not item.name.endswith("Plugin")
    ]
    non_ecs_names = {all_items[key].name for key in non_ecs_keys}

    used_by_ecs_fields: dict[str, list[str]] = defaultdict(list)
    used_by_functions: dict[str, list[str]] = defaultdict(list)
    used_by_systems: dict[str, list[str]] = defaultdict(list)

    # References from ECS item fields into non-ECS types.
    for owner in all_items.values():
        if not owner.ecs_kind:
            continue
        for field in owner.fields:
            for ref in field.referenced_types:
                if ref in non_ecs_names:
                    used_by_ecs_fields[ref].append(
                        f"{owner.ecs_kind} {owner.name}.{field.name} ({field.file}:{field.line})"
                    )

    # References from function signatures and bodies.
    updated_functions: list[FunctionRecord] = []
    for fn in functions:
        refs = unique_ordered(
            [
                t
                for t in (fn.referenced_types + fn.body_referenced_types)
                if t in non_ecs_names
            ]
        )
        for ref in refs:
            context = (
                f"{fn.owner + '::' if fn.owner else ''}{fn.name} ({fn.file}:{fn.line})"
            )
            used_by_functions[ref].append(context)
            if fn.is_system_like:
                used_by_systems[ref].append(context)
        updated_functions.append(dataclasses.replace(fn, non_ecs_types=refs))

    # Build preliminary classified records without collection links.
    preliminary: dict[str, ItemRecord] = {}
    for key in non_ecs_keys:
        item = all_items[key]
        category, cat_reasons = classify_category(item)
        suggested = suggested_target_for_category(category, item)
        methods = unique_ordered(methods_by_type.get(item.name, []))
        traits = unique_ordered(traits_by_type.get(item.name, []))
        temp = dataclasses.replace(
            item,
            category=category,
            suggested_target=suggested,
            methods=methods,
            trait_impls=traits,
            used_by_ecs_fields=unique_ordered(used_by_ecs_fields.get(item.name, []))[
                :30
            ],
            used_by_systems=unique_ordered(used_by_systems.get(item.name, []))[:30],
            used_by_functions=unique_ordered(used_by_functions.get(item.name, []))[:30],
            reasons=cat_reasons,
        )
        preliminary[item.name] = temp

    collection_records = build_collection_records(all_items, non_ecs_names, preliminary)
    collection_payload_for: dict[str, list[str]] = defaultdict(list)
    for rec in collection_records:
        collection_payload_for[rec.payload_type].append(
            f"{rec.owner}.{rec.field} ({rec.file}:{rec.line})"
        )

    final_items: list[ItemRecord] = []
    for item in preliminary.values():
        score, priority, reasons = score_item(
            item,
            item.category,
            item.reasons,
            item.used_by_ecs_fields,
            item.used_by_systems,
            item.used_by_functions,
            collection_payload_for.get(item.name, []),
            item.methods,
        )
        suggested = item.suggested_target
        if priority == "high" and suggested == "Unknown":
            suggested = "Review for Component/Resource migration"
        final_items.append(
            dataclasses.replace(
                item,
                migration_score=score,
                migration_priority=priority,
                reasons=reasons,
                suggested_target=suggested,
                collection_payload_for=unique_ordered(
                    collection_payload_for.get(item.name, [])
                )[:30],
            )
        )

    final_items.sort(
        key=lambda row: (-row.migration_score, row.file, row.line, row.name)
    )
    module_summaries = build_module_summaries(final_items, collection_records)
    return final_items, updated_functions, collection_records, module_summaries


def build_module_summaries(
    items: Sequence[ItemRecord], collections: Sequence[CollectionRecord]
) -> list[ModuleSummaryRecord]:
    stats: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for item in items:
        mod = module_name(item.file)
        stats[mod]["non_ecs_items"] += 1
        if item.migration_priority == "high":
            stats[mod]["high_priority"] += 1
        elif item.migration_priority == "medium":
            stats[mod]["medium_priority"] += 1
        if item.category in stats[mod]:
            stats[mod][item.category] += 1
        else:
            stats[mod][item.category] += 1
        if item.used_by_ecs_fields or item.used_by_systems:
            stats[mod]["ecs_adjacent"] += 1
    for collection in collections:
        stats[module_name(collection.file)]["hidden_collections"] += 1
    records: list[ModuleSummaryRecord] = []
    for mod, values in sorted(stats.items()):
        records.append(
            ModuleSummaryRecord(
                module=mod,
                non_ecs_items=values["non_ecs_items"],
                high_priority=values["high_priority"],
                medium_priority=values["medium_priority"],
                runtime_state=values["runtime_state"],
                content_data=values["content_data"]
                + values["domain_model"]
                + values["plain_model"],
                save_data=values["save_data"],
                ui_presentation=values["ui_presentation"],
                platform_backend=values["platform_backend"]
                + values["utility_or_backend"],
                utility=values["trait_or_interface"],
                ecs_adjacent=values["ecs_adjacent"],
                hidden_collections=values["hidden_collections"],
            )
        )
    records.sort(
        key=lambda row: (
            -row.high_priority,
            -row.medium_priority,
            -row.non_ecs_items,
            row.module,
        )
    )
    return records


# ---------------------------------------------------------------------------
# Output


def asdict_list(records: Iterable[object]) -> list[dict]:
    return [dataclasses.asdict(record) for record in records]


def write_table(
    out: list[str], headers: Sequence[str], rows: Sequence[Sequence[str]]
) -> None:
    out.append("| " + " | ".join(headers) + " |")
    out.append("| " + " | ".join("---" for _ in headers) + " |")
    for row in rows:
        out.append(
            "| " + " | ".join(str(cell).replace("|", "\\|") for cell in row) + " |"
        )
    out.append("")


def location(item: ItemRecord) -> str:
    return f"`{item.file}:{item.line}`"


def write_markdown(inventory: dict, path: pathlib.Path) -> None:
    items = [
        ItemRecord(**{**row, "fields": [FieldRecord(**f) for f in row["fields"]]})
        for row in inventory["non_ecs_items"]
    ]
    collections = [
        CollectionRecord(**row) for row in inventory["collections_hiding_state"]
    ]
    modules = [ModuleSummaryRecord(**row) for row in inventory["module_summaries"]]
    functions = [FunctionRecord(**row) for row in inventory["functions_using_non_ecs"]]

    high = [item for item in items if item.migration_priority == "high"]
    medium = [item for item in items if item.migration_priority == "medium"]
    ecs_adjacent = [
        item for item in items if item.used_by_ecs_fields or item.used_by_systems
    ]
    runtime = [item for item in items if item.category == "runtime_state"]
    content = [
        item
        for item in items
        if item.category
        in {"content_data", "domain_model", "save_data", "value_object"}
    ]
    keepish = [
        item
        for item in items
        if item.category
        in {
            "platform_backend",
            "ui_presentation",
            "utility_or_backend",
            "trait_or_interface",
        }
    ]

    out: list[str] = []
    out.append("# Ambition Sandbox non-ECS inventory")
    out.append("")
    out.append(f"Generated from `{inventory['crate_root']}`.")
    out.append("")
    out.append(
        "This report complements the ECS inventory. It lists Rust items that do not directly derive Bevy ECS marker traits, then highlights which ones are close to ECS boundaries or may be migration candidates."
    )
    out.append("")

    out.append("## Executive summary")
    out.append("")
    for key, value in inventory["counts"].items():
        out.append(f"- {key.replace('_', ' ').title()}: {value}")
    out.append("")
    out.append(
        "Migration scores are heuristic. Treat high-priority rows as review leads, not automatic refactor instructions."
    )
    out.append("")

    out.append("## Top migration candidates")
    out.append("")
    top = high[:40] if high else medium[:40]
    if not top:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for item in top[:40]:
            rows.append(
                [
                    f"`{item.name}`",
                    item.kind,
                    item.category,
                    str(item.migration_score),
                    item.suggested_target,
                    f"`{item.file}:{item.line}`",
                    "; ".join(item.reasons[:4]),
                ]
            )
        write_table(
            out,
            [
                "Item",
                "Kind",
                "Category",
                "Score",
                "Suggested target",
                "Location",
                "Why",
            ],
            rows,
        )

    out.append("## Collections that may be hiding ECS entities or runtime state")
    out.append("")
    if not collections:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for rec in sorted(
            collections,
            key=lambda r: (
                {"high": 0, "medium": 1, "low": 2}.get(r.priority, 3),
                r.file,
                r.line,
            ),
        )[:80]:
            owner = f"`{rec.owner}.{rec.field}`"
            if rec.owner_ecs_kind:
                owner += f" ({rec.owner_ecs_kind})"
            rows.append(
                [
                    owner,
                    rec.container,
                    f"`{rec.payload_type}`",
                    rec.payload_category,
                    rec.priority,
                    rec.suggested_target,
                    f"`{rec.file}:{rec.line}`",
                ]
            )
        write_table(
            out,
            [
                "Collection field",
                "Container",
                "Payload",
                "Payload category",
                "Priority",
                "Suggestion",
                "Location",
            ],
            rows,
        )

    out.append("## ECS-adjacent non-ECS types")
    out.append("")
    out.append(
        "These types are not ECS items themselves, but they are referenced by ECS item fields or system-like functions."
    )
    out.append("")
    if not ecs_adjacent:
        out.append("- None found.")
        out.append("")
    else:
        for item in sorted(
            ecs_adjacent, key=lambda row: (-row.migration_score, row.file, row.line)
        )[:80]:
            out.append(
                f"- `{item.name}` ({item.category}, score {item.migration_score}) at `{item.file}:{item.line}`"
            )
            if item.used_by_ecs_fields:
                out.append("  - ECS fields:")
                for ref in item.used_by_ecs_fields[:6]:
                    out.append(f"    - {ref}")
            if item.used_by_systems:
                out.append("  - system-like functions:")
                for ref in item.used_by_systems[:6]:
                    out.append(f"    - {ref}")
            if item.collection_payload_for:
                out.append("  - collection payload for:")
                for ref in item.collection_payload_for[:6]:
                    out.append(f"    - {ref}")
        out.append("")

    out.append("## Runtime-state-shaped non-ECS items")
    out.append("")
    if not runtime:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for item in sorted(
            runtime, key=lambda row: (-row.migration_score, row.file, row.line)
        )[:100]:
            rows.append(
                [
                    f"`{item.name}`",
                    item.migration_priority,
                    str(item.migration_score),
                    item.suggested_target,
                    str(len(item.fields)),
                    str(len(item.methods)),
                    f"`{item.file}:{item.line}`",
                ]
            )
        write_table(
            out,
            [
                "Item",
                "Priority",
                "Score",
                "Suggested target",
                "Fields",
                "Methods",
                "Location",
            ],
            rows,
        )

    out.append("## Content, save, and domain data likely to stay non-ECS")
    out.append("")
    if not content:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for item in sorted(content, key=lambda row: (row.category, row.file, row.line))[
            :120
        ]:
            rows.append(
                [
                    f"`{item.name}`",
                    item.category,
                    item.suggested_target,
                    f"`{item.file}:{item.line}`",
                ]
            )
        write_table(out, ["Item", "Category", "Suggestion", "Location"], rows)

    out.append("## Presentation/platform/utility items likely to stay non-ECS")
    out.append("")
    if not keepish:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for item in sorted(keepish, key=lambda row: (row.category, row.file, row.line))[
            :120
        ]:
            rows.append(
                [
                    f"`{item.name}`",
                    item.category,
                    item.suggested_target,
                    f"`{item.file}:{item.line}`",
                ]
            )
        write_table(out, ["Item", "Category", "Suggestion", "Location"], rows)

    out.append("## Modules with the most migration leads")
    out.append("")
    if not modules:
        out.append("- None found.")
        out.append("")
    else:
        rows = []
        for row in modules[:80]:
            rows.append(
                [
                    f"`{row.module}`",
                    str(row.non_ecs_items),
                    str(row.high_priority),
                    str(row.medium_priority),
                    str(row.runtime_state),
                    str(row.ecs_adjacent),
                    str(row.hidden_collections),
                ]
            )
        write_table(
            out,
            [
                "Module",
                "Items",
                "High",
                "Medium",
                "Runtime",
                "ECS-adjacent",
                "Collections",
            ],
            rows,
        )

    out.append("## System-like functions touching non-ECS types")
    out.append("")
    system_functions = [
        fn for fn in functions if fn.is_system_like and fn.non_ecs_types
    ]
    if not system_functions:
        out.append("- None found.")
        out.append("")
    else:
        for fn in sorted(system_functions, key=lambda row: (row.file, row.line))[:160]:
            name = f"{fn.owner}::{fn.name}" if fn.owner else fn.name
            out.append(f"- `{name}` at `{fn.file}:{fn.line}`")
            out.append(
                f"  - non-ECS types: {', '.join(f'`{t}`' for t in fn.non_ecs_types[:12])}"
            )
            reads = fn.ecs_access.get("resources_read") or []
            writes = fn.ecs_access.get("resources_written") or []
            queries = fn.ecs_access.get("queries") or []
            if reads or writes or queries:
                parts = []
                if writes:
                    parts.append("writes " + ", ".join(f"`{x}`" for x in writes[:6]))
                if reads:
                    parts.append("reads " + ", ".join(f"`{x}`" for x in reads[:6]))
                if queries:
                    parts.append("queries " + ", ".join(f"`{x}`" for x in queries[:3]))
                out.append(f"  - ECS access: {'; '.join(parts)}")
        out.append("")

    out.append("## Full non-ECS item index")
    out.append("")
    by_module: dict[str, list[ItemRecord]] = defaultdict(list)
    for item in sorted(items, key=lambda row: (row.file, row.line, row.name)):
        by_module[module_name(item.file)].append(item)
    for mod in sorted(by_module):
        out.append(f"### `{mod}`")
        out.append("")
        for item in by_module[mod]:
            derives = f" derives {', '.join(item.derives)}" if item.derives else ""
            out.append(
                f"- `{item.name}` ({item.kind}, {item.category}, {item.migration_priority}/{item.migration_score}{derives}) at `{item.file}:{item.line}`"
            )
        out.append("")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(out) + "\n", encoding="utf-8")


def build_inventory(
    repo_root: pathlib.Path, crate_root: pathlib.Path, include_tests: bool
) -> dict:
    parsed_files = list(iter_parsed_rs_files(crate_root, include_tests))
    all_items, methods_by_type, traits_by_type = collect_raw_items(
        parsed_files, repo_root, include_tests
    )
    item_names = {item.name for item in all_items.values()}
    functions = collect_functions(parsed_files, repo_root, include_tests, item_names)
    non_ecs_items, linked_functions, collections, module_summaries = classify_and_link(
        all_items, functions, methods_by_type, traits_by_type
    )

    direct_ecs_items = [item for item in all_items.values() if item.ecs_kind]
    arch_items = [item for item in all_items.values() if item.architecture_kind]
    high = [item for item in non_ecs_items if item.migration_priority == "high"]
    medium = [item for item in non_ecs_items if item.migration_priority == "medium"]
    ecs_adjacent = [
        item
        for item in non_ecs_items
        if item.used_by_ecs_fields or item.used_by_systems
    ]
    functions_using_non_ecs = [fn for fn in linked_functions if fn.non_ecs_types]

    inventory = {
        "schema_version": 1,
        "repo_root": ".",
        "crate_root": repo_rel(crate_root, repo_root),
        "include_tests": include_tests,
        "counts": {
            "rust_files": len(parsed_files),
            "all_items": len(all_items),
            "direct_ecs_items_excluded": len(direct_ecs_items),
            "architecture_marker_items_excluded": len(arch_items),
            "non_ecs_items": len(non_ecs_items),
            "high_priority_candidates": len(high),
            "medium_priority_candidates": len(medium),
            "ecs_adjacent_non_ecs_items": len(ecs_adjacent),
            "collections_hiding_state": len(collections),
            "functions_using_non_ecs": len(functions_using_non_ecs),
            "system_like_functions_using_non_ecs": len(
                [fn for fn in functions_using_non_ecs if fn.is_system_like]
            ),
        },
        "non_ecs_items": asdict_list(non_ecs_items),
        "collections_hiding_state": asdict_list(collections),
        "functions_using_non_ecs": asdict_list(functions_using_non_ecs),
        "module_summaries": asdict_list(module_summaries),
        "excluded_ecs_items": [
            {
                "name": item.name,
                "kind": item.kind,
                "ecs_kind": item.ecs_kind,
                "file": item.file,
                "line": item.line,
            }
            for item in sorted(
                direct_ecs_items, key=lambda row: (row.file, row.line, row.name)
            )
        ],
    }
    return inventory


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument(
        "--crate", type=pathlib.Path, default=pathlib.Path("crates/ambition_actors")
    )
    parser.add_argument(
        "--json",
        type=pathlib.Path,
        default=pathlib.Path("target/ambition_non_ecs_inventory.json"),
    )
    parser.add_argument(
        "--markdown",
        type=pathlib.Path,
        default=pathlib.Path("target/ambition_non_ecs_inventory.md"),
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
