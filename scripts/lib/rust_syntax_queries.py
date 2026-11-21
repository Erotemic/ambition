"""Pure Rust syntax queries for source inventory tools."""

from __future__ import annotations

from typing import Iterator


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
