#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "networkx>=3.2",
#   "tree-sitter>=0.25,<0.26",
#   "tree-sitter-rust>=0.24,<0.25",
# ]
# ///
"""Build a navigable module-structure graph for the Ambition workspace.

This crawls the cargo workspace and builds a NetworkX node for every Rust
*module*, where a module is a directory or a standalone `.rs` file. The Rust
"module file" idiom is collapsed: `foo/mod.rs` and a sibling `foo.rs` next to a
`foo/` directory are *absorbed* into the single `foo` module node (they are the
module's defining file, not separate modules).

Each node carries the module's `//!` inner doc comment, its crate, its file
path, and a line count.

Three edge layers are produced over a shared node set:

  * ``filesystem`` -- module containment (parent module -> child module). This
    is the canonical module tree and defines the nodes.
  * ``crate``      -- crate -> crate dependencies among workspace members,
    sourced from ``cargo metadata`` (authoritative), tagged by dep kind
    (normal/dev/build).
  * ``imports``    -- module -> module edges resolved from `use` declarations
    (``crate::``/``self``/``super``/workspace-crate paths). External/std paths
    are skipped. Each edge is weighted by how many `use`s contributed to it.

Outputs:
  * ``nx.write_network_text`` summaries to stdout (--print, default on).
  * A combined JSON (nodes + per-layer adjacency) consumed by the HTML viewer.
  * A self-contained interactive HTML explorer (vis-network via CDN) with two
    views over each layer:
      - a collapsible **tree/forest** (the default): a transitive-reduced,
        ``write_network_text``-style skeleton you churn through linearly.
        Already-seen nodes appear as clickable back-reference stubs (``↪``).
        Expand either direction -- ``dependencies`` (what a node needs, top
        down from the apps) or ``dependants`` (what needs it, up from the
        leaves). Crate deps reduce to a single root (``ambition_app``).
      - the force/hierarchical **graph** for when you want the gnarled ball.
    Both share a crate filter, fuzzy search, a "root tree here" action, a sort
    control (name / lines of code / num dependencies / num dependants, with an
    ascending/descending toggle; folder LOC is the filesystem-subtree sum), a
    drag-to-resize side panel, and a per-node detail panel showing the docstring
    plus dependencies, dependants, filesystem children, and (for crates)
    crate-level deps — in every view, with folders/files/crates distinguished.
    Transitive reduction uses ``nx.transitive_reduction`` on DAG layers and an
    SCC-condensation for the cyclic imports layer.
  * Optional per-layer GraphML (--graphml) for Gephi/yEd.

Why a custom tool? `cargo-modules` (module tree/deps) and `cargo-depgraph`
(crate deps) exist and are great for one-shot Graphviz dumps, but neither gives
a queryable graph keyed by module that also carries docstrings and a browsable
HTML index. This fills that gap and reuses `cargo metadata` for the crate layer.

Run it directly (uv resolves deps from the inline metadata):

    scripts/module_graph.py                 # writes to .agent/reports/module-graph/
    scripts/module_graph.py --crate ambition_sandbox
    scripts/module_graph.py --no-print --graphml
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import networkx as nx
import tree_sitter_rust
from tree_sitter import Language, Parser

RUST = Language(tree_sitter_rust.language())


# --------------------------------------------------------------------------- #
# Workspace discovery via cargo metadata
# --------------------------------------------------------------------------- #


@dataclass
class CrateInfo:
    name: str
    manifest_path: Path
    src_root: Path  # the lib.rs / main.rs that roots the module tree
    src_dir: Path  # directory that holds the crate's submodules
    deps: list[tuple[str, str]] = field(default_factory=list)  # (dep_name, kind)


def _cargo_bin() -> str:
    # Honor the documented ~/.cargo/bin location without forcing PATH edits.
    candidate = Path.home() / ".cargo" / "bin" / "cargo"
    return str(candidate) if candidate.exists() else "cargo"


def load_workspace(root: Path) -> dict[str, CrateInfo]:
    """Return workspace crates keyed by name, with internal deps resolved."""
    out = subprocess.run(
        [_cargo_bin(), "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    )
    meta = json.loads(out.stdout)
    members = set(meta.get("workspace_members", []))
    crates: dict[str, CrateInfo] = {}
    pkg_by_id = {p["id"]: p for p in meta["packages"]}

    for pkg_id in members:
        pkg = pkg_by_id.get(pkg_id)
        if pkg is None:
            continue
        name = pkg["name"]
        # Prefer the lib target as the module-tree root; fall back to a bin.
        lib_src = None
        bin_src = None
        for tgt in pkg["targets"]:
            kinds = tgt.get("kind", [])
            if any(k in ("lib", "rlib", "cdylib", "staticlib") for k in kinds):
                lib_src = Path(tgt["src_path"])
            elif "bin" in kinds and bin_src is None:
                bin_src = Path(tgt["src_path"])
        src_root = lib_src or bin_src
        if src_root is None:
            continue
        crates[name] = CrateInfo(
            name=name,
            manifest_path=Path(pkg["manifest_path"]),
            src_root=src_root,
            src_dir=src_root.parent,
        )

    member_names = set(crates)
    # Resolve internal dependency edges (kind: normal/dev/build).
    for pkg_id in members:
        pkg = pkg_by_id.get(pkg_id)
        if pkg is None:
            continue
        name = pkg["name"]
        if name not in crates:
            continue
        seen: set[tuple[str, str]] = set()
        for dep in pkg.get("dependencies", []):
            dep_name = dep["name"]
            if dep_name not in member_names or dep_name == name:
                continue
            kind = dep.get("kind") or "normal"
            key = (dep_name, kind)
            if key not in seen:
                seen.add(key)
                crates[name].deps.append(key)
    return crates


# --------------------------------------------------------------------------- #
# Docstring extraction
# --------------------------------------------------------------------------- #


def extract_module_doc(path: Path) -> str:
    """Pull the leading `//!` / `/*! */` module doc block from a file."""
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""
    lines: list[str] = []
    in_block = False
    for raw in text.splitlines():
        s = raw.strip()
        if in_block:
            end = s.endswith("*/")
            body = s[:-2] if end else s
            body = body.lstrip("*").strip()
            lines.append(body)
            if end:
                break
            continue
        if s.startswith("//!"):
            lines.append(s[3:].strip())
        elif s.startswith("/*!"):
            rest = s[3:]
            if rest.endswith("*/"):
                lines.append(rest[:-2].strip())
                break
            lines.append(rest.strip())
            in_block = True
        elif s == "" or s.startswith("#![") or s.startswith("//"):
            # Blank lines, inner attributes, and stray non-doc comments may
            # precede or interleave the module doc; keep scanning.
            if s == "" and lines:
                lines.append("")
            continue
        else:
            # First real item -> module doc (if any) is finished.
            break
    return "\n".join(lines).strip()


def count_lines(path: Path) -> int:
    try:
        with path.open("rb") as fh:
            return sum(1 for _ in fh)
    except OSError:
        return 0


# --------------------------------------------------------------------------- #
# Module tree (filesystem containment)
# --------------------------------------------------------------------------- #


@dataclass
class ModuleNode:
    node_id: str  # e.g. "ambition_sandbox::player::input"
    kind: str  # "crate" | "module"
    crate: str
    file: Path | None
    rel_path: str
    doc: str
    loc: int


def _add_node(
    nodes: dict[str, ModuleNode],
    node_id: str,
    kind: str,
    crate: str,
    file: Path | None,
    root: Path,
) -> None:
    if node_id in nodes:
        return
    rel = ""
    doc = ""
    loc = 0
    if file is not None:
        try:
            rel = str(file.relative_to(root))
        except ValueError:
            rel = str(file)
        doc = extract_module_doc(file)
        loc = count_lines(file)
    nodes[node_id] = ModuleNode(node_id, kind, crate, file, rel, doc, loc)


def walk_modules(
    crate: CrateInfo,
    root: Path,
) -> tuple[dict[str, ModuleNode], list[tuple[str, str]]]:
    """Return (nodes, containment_edges) for one crate's module tree."""
    nodes: dict[str, ModuleNode] = {}
    edges: list[tuple[str, str]] = []
    _add_node(nodes, crate.name, "crate", crate.name, crate.src_root, root)

    crate_root_names = {crate.src_root.name, "lib.rs", "main.rs", "mod.rs"}

    def recurse(dirpath: Path, parent_id: str, prefix: str, is_crate_root: bool):
        try:
            entries = list(os.scandir(dirpath))
        except OSError:
            return
        dirs = sorted((e for e in entries if e.is_dir()), key=lambda e: e.name)
        rs_files = sorted(
            (e for e in entries if e.is_file() and e.name.endswith(".rs")),
            key=lambda e: e.name,
        )
        file_by_name = {e.name: Path(e.path) for e in rs_files}
        consumed: set[str] = set()

        for d in dirs:
            modname = d.name
            node_id = f"{prefix}::{modname}"
            mod_rs = Path(d.path) / "mod.rs"
            sibling = f"{modname}.rs"
            if mod_rs.exists():
                modfile: Path | None = mod_rs
            elif sibling in file_by_name:
                modfile = file_by_name[sibling]
                consumed.add(sibling)
            else:
                modfile = None  # dir with no module file (rare); still a node
            _add_node(nodes, node_id, "module", prefix.split("::")[0], modfile, root)
            edges.append((parent_id, node_id))
            recurse(Path(d.path), node_id, node_id, False)

        for name, path in file_by_name.items():
            if name == "mod.rs":
                continue
            if is_crate_root and name in ("lib.rs", "main.rs"):
                continue
            if name in consumed:
                continue
            modname = name[:-3]
            node_id = f"{prefix}::{modname}"
            if node_id in nodes:  # already created as a dir-backed module
                continue
            _add_node(nodes, node_id, "module", prefix.split("::")[0], path, root)
            edges.append((parent_id, node_id))

    recurse(crate.src_dir, crate.name, crate.name, True)
    return nodes, edges


# --------------------------------------------------------------------------- #
# `use` extraction + resolution (imports layer)
# --------------------------------------------------------------------------- #


def _text(node, src: bytes) -> str:
    return src[node.start_byte : node.end_byte].decode("utf-8", "replace")


def _path_tokens(node, src: bytes) -> list[str]:
    t = node.type
    if t in ("identifier", "crate", "self", "super", "metavariable", "primitive_type"):
        return [_text(node, src)]
    if t == "scoped_identifier":
        left = node.child_by_field_name("path")
        right = node.child_by_field_name("name")
        toks: list[str] = []
        if left is not None:
            toks += _path_tokens(left, src)
        if right is not None:
            toks += _path_tokens(right, src)
        return toks
    return [_text(node, src)]


def _expand_use(node, prefix: list[str], src: bytes, out: list[list[str]]) -> None:
    t = node.type
    if t == "use_list":
        for c in node.named_children:
            _expand_use(c, prefix, src, out)
    elif t == "scoped_use_list":
        path = node.child_by_field_name("path")
        base = prefix + (_path_tokens(path, src) if path is not None else [])
        lst = node.child_by_field_name("list")
        if lst is not None:
            for c in lst.named_children:
                _expand_use(c, base, src, out)
    elif t == "use_as_clause":
        path = node.child_by_field_name("path")
        if path is not None:
            _expand_use(path, prefix, src, out)
    elif t == "use_wildcard":
        kids = list(node.named_children)
        if kids:
            out.append(prefix + _path_tokens(kids[0], src) + ["*"])
        else:
            out.append(prefix + ["*"])
    elif t in ("scoped_identifier", "identifier", "crate", "self", "super"):
        out.append(prefix + _path_tokens(node, src))
    else:
        for c in node.named_children:
            _expand_use(c, prefix, src, out)


def extract_uses(path: Path, parser: Parser) -> list[list[str]]:
    try:
        src = path.read_bytes()
    except OSError:
        return []
    tree = parser.parse(src)
    out: list[list[str]] = []

    def visit(node):
        if node.type == "use_declaration":
            arg = node.child_by_field_name("argument")
            if arg is not None:
                _expand_use(arg, [], src, out)
            return
        for c in node.children:
            visit(c)

    visit(tree.root_node)
    return out


def resolve_use(
    tokens: list[str],
    cur_crate: str,
    cur_module: str,
    module_ids: set[str],
    crate_names: set[str],
) -> str | None:
    tokens = [t for t in tokens if t != "*"]
    if not tokens:
        return None
    first = tokens[0]
    if first == "crate":
        base = [cur_crate] + tokens[1:]
    elif first == "self":
        base = cur_module.split("::") + tokens[1:]
    elif first == "super":
        parts = cur_module.split("::")
        i = 0
        while i < len(tokens) and tokens[i] == "super":
            if len(parts) > 1:
                parts = parts[:-1]
            i += 1
        base = parts + tokens[i:]
    elif first in crate_names:
        base = tokens[:]
    else:
        return None  # std / external / unknown -> no node
    for n in range(len(base), 0, -1):
        cand = "::".join(base[:n])
        if cand in module_ids:
            return cand
    return base[0] if base and base[0] in crate_names else None


# --------------------------------------------------------------------------- #
# Graph assembly
# --------------------------------------------------------------------------- #


def reduce_graph(g: nx.DiGraph) -> list[tuple[str, str]]:
    """Transitive reduction as an edge list.

    For a DAG (the crate layer always is — cargo forbids cyclic deps), this is
    the unique minimal edge set with the same reachability. For a cyclic graph
    (the imports layer can be), we condense strongly-connected components into a
    DAG, reduce that, lift each surviving inter-component edge back to a single
    representative original edge, and chain each component's members so the
    cycle stays reachable. The result is a tree-friendly skeleton; the full
    adjacency is still available in the ``layers`` JSON for the graph view.
    """
    if g.number_of_edges() == 0:
        return []
    if nx.is_directed_acyclic_graph(g):
        return list(nx.transitive_reduction(g).edges())

    cond = nx.condensation(g)  # DAG; each node has a 'members' set attribute
    reduced_cond = nx.transitive_reduction(cond)
    members = nx.get_node_attributes(cond, "members")
    edges: list[tuple[str, str]] = []
    for cu, cv in reduced_cond.edges():
        rep = next(
            ((u, v) for u in sorted(members[cu]) for v in sorted(members[cv]) if g.has_edge(u, v)),
            None,
        )
        if rep is not None:
            edges.append(rep)
    for mem in members.values():
        chain = sorted(mem)
        for a, b in zip(chain, chain[1:]):
            edges.append((a, b))
    return edges


def build_graphs(root: Path, crate_filter: str | None):
    crates = load_workspace(root)
    if crate_filter:
        if crate_filter not in crates:
            sys.exit(f"error: crate '{crate_filter}' not in workspace {sorted(crates)}")
        crates = {crate_filter: crates[crate_filter]}

    all_nodes: dict[str, ModuleNode] = {}
    fs_edges: list[tuple[str, str]] = []
    file_to_module: dict[Path, str] = {}

    for crate in crates.values():
        nodes, edges = walk_modules(crate, root)
        all_nodes.update(nodes)
        fs_edges.extend(edges)
        for nid, node in nodes.items():
            if node.file is not None:
                file_to_module[node.file] = nid

    module_ids = set(all_nodes)
    crate_names = set(crates)

    # Imports layer.
    parser = Parser(RUST)
    import_weights: dict[tuple[str, str], int] = {}
    for nid, node in all_nodes.items():
        if node.file is None:
            continue
        cur_crate = node.crate
        for tokens in extract_uses(node.file, parser):
            target = resolve_use(tokens, cur_crate, nid, module_ids, crate_names)
            if target is None or target == nid:
                continue
            import_weights[(nid, target)] = import_weights.get((nid, target), 0) + 1

    # Crate layer.
    crate_edges: list[tuple[str, str, str]] = []
    for crate in crates.values():
        for dep_name, kind in crate.deps:
            if dep_name in crate_names:
                crate_edges.append((crate.name, dep_name, kind))

    # --- Build NetworkX graphs --------------------------------------------- #
    def base_graph() -> nx.DiGraph:
        g = nx.DiGraph()
        for nid, node in all_nodes.items():
            summary = node.doc.splitlines()[0] if node.doc else ""
            g.add_node(
                nid,
                kind=node.kind,
                crate=node.crate,
                path=node.rel_path,
                loc=node.loc,
                summary=summary,
                doc=node.doc,
            )
        return g

    g_fs = base_graph()
    g_fs.add_edges_from(fs_edges, layer="filesystem")

    g_imports = base_graph()
    for (s, d), w in import_weights.items():
        g_imports.add_edge(s, d, layer="imports", weight=w)

    g_crate = nx.DiGraph()
    for name in crate_names:
        node = all_nodes.get(name)
        g_crate.add_node(
            name,
            kind="crate",
            crate=name,
            path=node.rel_path if node else "",
            loc=node.loc if node else 0,
            summary=(node.doc.splitlines()[0] if node and node.doc else ""),
            doc=node.doc if node else "",
        )
    for s, d, kind in crate_edges:
        g_crate.add_edge(s, d, layer="crate", kind=kind)

    return all_nodes, g_fs, g_imports, g_crate, import_weights, crate_edges, crate_names


# --------------------------------------------------------------------------- #
# Outputs
# --------------------------------------------------------------------------- #


def write_json(
    path: Path,
    all_nodes: dict[str, ModuleNode],
    fs_edges,
    import_weights,
    crate_edges,
    crate_names: set[str],
    reduced: dict[str, list[tuple[str, str]]],
) -> None:
    # Filesystem-subtree aggregates: a folder's LOC is its own module file plus
    # every descendant. `is_folder` distinguishes directory modules from leaves.
    fs_children: dict[str, list[str]] = {}
    for s, d in fs_edges:
        fs_children.setdefault(s, []).append(d)
    subtree_loc: dict[str, int] = {}

    def _subtree_loc(nid: str) -> int:
        if nid in subtree_loc:
            return subtree_loc[nid]
        node = all_nodes.get(nid)
        total = node.loc if node else 0
        for child in fs_children.get(nid, []):
            total += _subtree_loc(child)
        subtree_loc[nid] = total
        return total

    for nid in all_nodes:
        _subtree_loc(nid)

    nodes_json = [
        {
            "id": n.node_id,
            "label": n.node_id.split("::")[-1],
            "kind": n.kind,
            "crate": n.crate,
            "path": n.rel_path,
            "loc": n.loc,
            "subtree_loc": subtree_loc.get(n.node_id, n.loc),
            "is_folder": bool(fs_children.get(n.node_id)),
            "summary": n.doc.splitlines()[0] if n.doc else "",
            "doc": n.doc,
        }
        for n in all_nodes.values()
    ]
    data = {
        "nodes": nodes_json,
        "crates": sorted(crate_names),
        "layers": {
            "filesystem": [[s, d] for s, d in fs_edges],
            "imports": [[s, d, w] for (s, d), w in import_weights.items()],
            "crate": [[s, d, k] for s, d, k in crate_edges],
        },
        # Transitive-reduced edge lists used by the collapsible tree view.
        "layers_reduced": {
            layer: [[s, d] for s, d in edges] for layer, edges in reduced.items()
        },
    }
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def write_graphml(out_dir: Path, g_fs, g_imports, g_crate) -> None:
    def clean(g: nx.DiGraph) -> nx.DiGraph:
        h = g.copy()
        for _, attrs in h.nodes(data=True):
            for k, v in list(attrs.items()):
                if v is None:
                    attrs[k] = ""
        return h

    nx.write_graphml(clean(g_fs), out_dir / "module_graph.filesystem.graphml")
    nx.write_graphml(clean(g_imports), out_dir / "module_graph.imports.graphml")
    nx.write_graphml(clean(g_crate), out_dir / "module_graph.crate.graphml")


def print_network_text(g_fs, g_imports, g_crate) -> None:
    print("\n" + "=" * 78)
    print("FILESYSTEM (module containment tree)")
    print("=" * 78)
    nx.write_network_text(g_fs, with_labels=True)

    print("\n" + "=" * 78)
    print("CRATE DEPENDENCIES (workspace-internal)")
    print("=" * 78)
    nx.write_network_text(g_crate, with_labels=True)

    print("\n" + "=" * 78)
    print(f"IMPORTS (module -> module via `use`; {g_imports.number_of_edges()} edges)")
    print("=" * 78)
    # The import graph is cyclic and dense; network_text on the whole thing is
    # noisy, so summarize the most-depended-on modules instead.
    indeg = sorted(g_imports.in_degree(), key=lambda kv: kv[1], reverse=True)
    print("Top imported-by (fan-in) modules:")
    for nid, deg in indeg[:25]:
        if deg == 0:
            break
        print(f"  {deg:4d}  {nid}")


def build_html(json_path: Path, html_path: Path) -> None:
    data = json_path.read_text(encoding="utf-8")
    # The JSON is embedded inside a <script> block; neutralize any "</script>"
    # (or stray "<!--") that a docstring might contain so it can't end the tag.
    data = data.replace("</", "<\\/")
    html = HTML_TEMPLATE.replace("/*__DATA__*/", data)
    html_path.write_text(html, encoding="utf-8")


HTML_TEMPLATE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Ambition module explorer</title>
<script src="https://unpkg.com/vis-network/standalone/umd/vis-network.min.js"></script>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body { margin: 0; font: 13px/1.5 system-ui, sans-serif; background: #14161a; color: #d7dae0; }
  #bar { display: flex; gap: 12px; align-items: center; flex-wrap: wrap;
         padding: 8px 12px; background: #1c1f26; border-bottom: 1px solid #2a2e37; }
  #bar label { font-size: 12px; color: #9aa1ac; }
  select, input { background: #0f1115; color: #d7dae0; border: 1px solid #2a2e37;
                  border-radius: 5px; padding: 4px 6px; font: inherit; }
  #wrap { display: flex; height: calc(100vh - 49px); }
  #net { flex: 1; min-width: 0; }
  #gutter { width: 6px; flex: none; cursor: col-resize; background: #2a2e37; }
  #gutter:hover, #gutter.drag { background: #3d4a6b; }
  #side { width: 380px; flex: none; overflow: auto; padding: 14px; background: #1a1d23;
          border-left: 1px solid #2a2e37; }
  .pill.kind-folder { background: #3a466b; color: #cdd7f5; }
  .pill.kind-file { background: #2f3a33; color: #c7e0cd; }
  .pill.kind-crate { background: #5a3f6b; color: #e6d2f5; }
  #side h2 { font-size: 15px; margin: 0 0 2px; word-break: break-all; }
  #side .meta { color: #9aa1ac; font-size: 12px; margin-bottom: 10px; }
  #side pre { white-space: pre-wrap; word-break: break-word; background: #0f1115;
              border: 1px solid #262a33; border-radius: 6px; padding: 8px;
              font: 12px/1.45 ui-monospace, monospace; color: #c5cad3; }
  .seclabel { color: #7f8794; text-transform: uppercase; letter-spacing: .05em;
              font-size: 11px; margin: 14px 0 4px; }
  .nbr { cursor: pointer; padding: 2px 4px; border-radius: 4px; word-break: break-all; }
  .nbr:hover { background: #262a33; }
  .pill { display: inline-block; padding: 1px 7px; border-radius: 999px;
          font-size: 11px; background: #2a2e37; color: #b9c0cc; }
  #stats { color: #7f8794; font-size: 12px; margin-left: auto; }
  #results { position: absolute; z-index: 9; background: #0f1115; border: 1px solid #2a2e37;
             max-height: 260px; overflow: auto; width: 320px; display: none; border-radius: 6px; }
  #results div { padding: 4px 8px; cursor: pointer; }
  #results div:hover { background: #262a33; }
  button { background: #232733; color: #d7dae0; border: 1px solid #2a2e37;
           border-radius: 5px; padding: 4px 8px; font: inherit; cursor: pointer; }
  button:hover { background: #2c313d; }
  #tree { flex: 1; min-width: 0; overflow: auto; padding: 8px 4px;
          font: 12.5px/1.5 ui-monospace, SFMono-Regular, monospace; }
  .trow { display: flex; align-items: center; gap: 4px; padding: 1px 6px;
          white-space: nowrap; cursor: pointer; border-radius: 4px; user-select: none; }
  .trow:hover { background: #20242d; }
  .trow.sel { background: #2d3550; }
  .trow.stub { color: #6b7280; font-style: italic; }
  .tw { display: inline-block; width: 15px; text-align: center; flex: none; }
  /* Expandable nodes get a bright filled caret; leaves get a small dim dot. */
  .trow.has-children > .tw { color: #8ab4f8; font-size: 11px; }
  .trow.has-children > .tw:hover { color: #c8def8; }
  .trow.leaf > .tw { color: #444a55; font-size: 8px; }
  .trow.stub > .tw { color: #6b7280; }
  .trow.has-children > .tlabel { font-weight: 600; }
  .trow.leaf > .tlabel { color: #aeb4be; }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .trow .dim { color: #6b7280; }
  .trow.crate-row { font-weight: 600; }
  .flash { animation: flash 1s ease-out; }
  @keyframes flash { from { background: #3d4a6b; } to { background: transparent; } }
  .hidden { display: none !important; }
</style>
</head>
<body>
<div id="bar">
  <label>View
    <select id="view">
      <option value="tree">tree</option>
      <option value="graph">graph</option>
    </select>
  </label>
  <label>Layer
    <select id="layer">
      <option value="filesystem">filesystem (containment)</option>
      <option value="crate">crate deps</option>
      <option value="imports">imports (use)</option>
    </select>
  </label>
  <label class="tree-only">Expand
    <select id="direction">
      <option value="dependencies">dependencies (what it needs) ↓</option>
      <option value="dependants">dependants (what needs it) ↓</option>
    </select>
  </label>
  <label class="tree-only" title="Transitive reduction: drop edges implied by a longer path; cycles condensed into SCC chains">
    <input type="checkbox" id="reduced" checked /> reduce
  </label>
  <label>Sort
    <select id="sort">
      <option value="name">name</option>
      <option value="loc">lines of code</option>
      <option value="dependencies">num dependencies</option>
      <option value="dependants">num dependants</option>
    </select>
  </label>
  <button id="sortDir" title="toggle ascending / descending">▼ desc</button>
  <label>Crate
    <select id="crate"><option value="">all</option></select>
  </label>
  <label style="position:relative">Search
    <input id="search" placeholder="module name..." autocomplete="off" />
    <div id="results"></div>
  </label>
  <button id="expandAll" class="tree-only">expand all</button>
  <button id="collapseAll" class="tree-only">collapse all</button>
  <span id="stats"></span>
</div>
<div id="wrap">
  <div id="net" class="hidden"></div>
  <div id="tree"></div>
  <div id="gutter" title="drag to resize"></div>
  <div id="side"><div class="meta">Click a node to inspect it.</div></div>
</div>
<script>
const DATA = /*__DATA__*/;

const crateList = DATA.crates;
const nodeById = new Map(DATA.nodes.map(n => [n.id, n]));

// Stable color per crate.
const palette = ["#e06c75","#61afef","#98c379","#e5c07b","#c678dd","#56b6c2",
  "#d19a66","#7fbf7f","#ff9e64","#9aa1ac","#bb9af7","#73daca","#f7768e","#7dcfff"];
const crateColor = {};
crateList.forEach((c,i) => crateColor[c] = palette[i % palette.length]);

const layerSel = document.getElementById("layer");
const crateSel = document.getElementById("crate");
const searchEl = document.getElementById("search");
const resultsEl = document.getElementById("results");
const statsEl = document.getElementById("stats");
crateList.forEach(c => { const o=document.createElement("option"); o.value=c; o.textContent=c; crateSel.appendChild(o); });

const viewSel = document.getElementById("view");
const dirSel = document.getElementById("direction");
const reducedEl = document.getElementById("reduced");
const sortSel = document.getElementById("sort");
const sortDirBtn = document.getElementById("sortDir");
const treeEl = document.getElementById("tree");
const netEl = document.getElementById("net");
const sideEl = document.getElementById("side");
const gutterEl = document.getElementById("gutter");
let sortAsc = false;   // numeric sorts default to descending; name flips to asc

let network = null;
let rootOverride = null;       // when set, the tree is a single subtree from here
let selectedId = null;
let selRow = null;
let currentFirstDom = new Map(); // node id -> dom id of its first (expanded) occurrence

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");
const jsq = (s) => String(s).replace(/\\/g, "\\\\").replace(/'/g, "\\'");
const shortLabel = (id) => { const p = id.split("::"); return p.length > 1 ? p[p.length-1] : id; };

// --------------------------------------------------------------------------- //
// Graph view (vis-network force / hierarchical layout)
// --------------------------------------------------------------------------- //

function edgesFor(layer) {
  if (layer === "filesystem") return DATA.layers.filesystem.map(([s,d]) => ({from:s,to:d}));
  if (layer === "imports") return DATA.layers.imports.map(([s,d,w]) => ({from:s,to:d,value:w,title:w+" use(s)"}));
  return DATA.layers.crate.map(([s,d,k]) => ({from:s,to:d,label:k==="normal"?"":k,
      dashes:k!=="normal", title:k}));
}

function renderGraph() {
  const layer = layerSel.value;
  const crate = crateSel.value;
  const rawEdges = edgesFor(layer);
  const participating = new Set();
  rawEdges.forEach(e => { participating.add(e.from); participating.add(e.to); });

  let nodes = DATA.nodes.filter(n => {
    if (layer === "crate" && n.kind !== "crate") return false;
    if (crate && n.crate !== crate) return false;
    return true;
  });
  const keep = new Set(nodes.map(n => n.id));
  let edges = rawEdges.filter(e => keep.has(e.from) && keep.has(e.to));

  const visNodes = nodes.map(n => ({
    id: n.id, label: n.label, title: n.summary || n.id,
    color: { background: crateColor[n.crate] || "#888",
             border: n.kind==="crate" ? "#fff" : "#0008" },
    shape: n.kind==="crate" ? "box" : "dot",
    value: Math.max(4, Math.sqrt(n.loc || 1)),
    font: { color: "#e6e9ee", size: 13 },
  }));

  const hierarchical = layer === "filesystem";
  const options = {
    nodes: { borderWidth: 1, scaling: { min: 6, max: 40 } },
    edges: { arrows: { to: { enabled: true, scaleFactor: 0.5 } },
             color: { color: "#3a3f4b", highlight: "#9aa1ac" }, smooth: hierarchical },
    physics: hierarchical ? false : { stabilization: { iterations: 180 },
             barnesHut: { gravitationalConstant: -6000, springLength: 110 } },
    layout: hierarchical ? { hierarchical: { direction: "LR", sortMethod: "directed",
             levelSeparation: 180, nodeSpacing: 28 } } : {},
    interaction: { hover: true, tooltipDelay: 120 },
  };
  network = new vis.Network(netEl,
      { nodes: new vis.DataSet(visNodes), edges: new vis.DataSet(edges) }, options);
  network.on("selectNode", p => showNode(p.nodes[0]));
  statsEl.textContent = `${visNodes.length} nodes · ${edges.length} edges · ${layer}`;
}

// --------------------------------------------------------------------------- //
// Tree view: transitive-reduced forest, expandable, with back-reference stubs
// --------------------------------------------------------------------------- //

function rawPairs(layer) {
  if (layer === "filesystem") return DATA.layers.filesystem;
  if (layer === "imports") return DATA.layers.imports.map(e => [e[0], e[1]]);
  return DATA.layers.crate.map(e => [e[0], e[1]]);
}

// Per-layer degree of every node, cached. out = dependencies / fs children,
// inc = dependants / fs parents. Distinct neighbors only (dedupes crate dev+normal).
const _metricCache = {};
function metrics(layer) {
  if (_metricCache[layer]) return _metricCache[layer];
  const out = new Map(), inc = new Map();
  const add = (m, k, v) => { (m.get(k) || m.set(k, new Set()).get(k)).add(v); };
  for (const [s, d] of rawPairs(layer)) { add(out, s, d); add(inc, d, s); }
  return (_metricCache[layer] = { out, inc });
}
const _loc = (id) => (nodeById.get(id) || {}).loc || 0;
// Folder LOC is the filesystem-subtree sum (own module file + all descendants).
const _subloc = (id) => { const n = nodeById.get(id) || {}; return n.subtree_loc != null ? n.subtree_loc : (n.loc || 0); };
const _deg = (m, id) => { const s = m.get(id); return s ? s.size : 0; };

// Comparator over node ids for the chosen sort key + direction (sortAsc).
// Degrees come from `layer`; "loc" uses the subtree aggregate.
function makeCmp(sortKey, layer) {
  const m = metrics(layer);
  const nameAsc = (a, b) => a.localeCompare(b);
  if (sortKey === "name") return sortAsc ? nameAsc : (a, b) => nameAsc(b, a);
  const val = sortKey === "loc" ? _subloc
            : sortKey === "dependencies" ? (id) => _deg(m.out, id)
            : (id) => _deg(m.inc, id);
  const dir = sortAsc ? 1 : -1;
  return (a, b) => dir * (val(a) - val(b)) || nameAsc(a, b);
}
const sortIds = (ids, layer) => ids.slice().sort(makeCmp(sortSel.value, layer));

// Build a child-adjacency for the chosen direction.
//   dependencies: A -> (things A depends on / its fs children)
//   dependants:   A -> (things that depend on A / its fs parent)
function buildAdj(layer, direction, reduced) {
  const src = reduced ? DATA.layers_reduced[layer] : rawPairs(layer);
  const adj = new Map(), indeg = new Map(), hasEdge = new Set();
  for (const pair of src) {
    let s = pair[0], d = pair[1];
    if (direction === "dependants") { const t = s; s = d; d = t; }
    if (!adj.has(s)) adj.set(s, []);
    adj.get(s).push(d);
    indeg.set(d, (indeg.get(d) || 0) + 1);
    hasEdge.add(pair[0]); hasEdge.add(pair[1]);
  }
  return { adj, indeg, hasEdge };
}

function buildForest() {
  const layer = layerSel.value, direction = dirSel.value, reduced = reducedEl.checked;
  const crate = crateSel.value;
  const { adj, indeg, hasEdge } = buildAdj(layer, direction, reduced);
  const cmp = makeCmp(sortSel.value, layer);

  const universe = new Set();
  if (layer === "crate") DATA.crates.forEach(c => universe.add(c));
  else DATA.nodes.forEach(n => { if (!crate || n.crate === crate) universe.add(n.id); });
  const inU = (id) => universe.has(id);

  let roots;
  if (rootOverride && universe.has(rootOverride)) {
    roots = [rootOverride];
  } else {
    // A root is a top node (nothing points to it). Keep nodes that take part in
    // an edge, plus every crate node so single-file crates stay visible.
    roots = [...universe].filter(id => {
      if (indeg.get(id) > 0) return false;
      const nn = nodeById.get(id);
      return hasEdge.has(id) || (nn && nn.kind === "crate");
    });
    roots.sort(cmp);
  }

  const visited = new Set(), firstDom = new Map();
  let counter = 0;
  function expand(id) {
    const domId = "t" + (counter++);
    if (visited.has(id)) return { id, domId, stub: true, ref: firstDom.get(id), children: [] };
    visited.add(id); firstDom.set(id, domId);
    const kids = (adj.get(id) || []).filter(inU);
    kids.sort(cmp);
    return { id, domId, stub: false, children: kids.map(expand) };
  }
  let forest = roots.map(expand);
  // Cycle islands with no in-edge-free entry point: surface them as extra roots.
  for (const id of universe) {
    if (hasEdge.has(id) && !visited.has(id)) forest.push(expand(id));
  }
  return { forest, firstDom };
}

function renderTree() {
  const { forest, firstDom } = buildForest();
  currentFirstDom = firstDom;
  treeEl.innerHTML = "";
  selRow = null;
  const frag = document.createDocumentFragment();
  let rows = 0, stubs = 0;

  function make(node, depth) {
    const n = nodeById.get(node.id) || {};
    const wrap = document.createElement("div");
    const row = document.createElement("div");
    row.className = "trow" + (node.stub ? " stub" : "") + (n.kind === "crate" ? " crate-row" : "");
    row.id = node.domId;
    row.style.paddingLeft = (depth * 14 + 4) + "px";
    rows++;

    if (node.stub) {
      stubs++;
      row.innerHTML = `<span class="tw">↪</span>` +
        `<span class="dot" style="background:${crateColor[n.crate] || "#888"}"></span>` +
        `<span class="tlabel">${esc(shortLabel(node.id))}</span><span class="dim"> ↑ shown above</span>`;
      row.title = node.id + "  (already expanded above — click to jump there)";
      row.onclick = (e) => { e.stopPropagation(); select(row, node.id); jumpToTreeRow(node.ref); };
      wrap.appendChild(row);
      return wrap;
    }

    const hasKids = node.children.length > 0;
    row.classList.add(hasKids ? "has-children" : "leaf");
    const sloc = _subloc(node.id);  // subtree LOC for folders, own LOC for leaves
    row.innerHTML = `<span class="tw">${hasKids ? "▶" : "◦"}</span>` +
      `<span class="dot" style="background:${crateColor[n.crate] || "#888"}"></span>` +
      `<span class="tlabel">${esc(shortLabel(node.id))}</span>` +
      `<span class="dim">${sloc ? " " + sloc : ""}${hasKids ? " (" + node.children.length + ")" : ""}</span>`;
    row.title = node.id + (n.summary ? "  —  " + n.summary : "");
    wrap.appendChild(row);

    if (hasKids) {
      const kids = document.createElement("div");
      kids.className = "tkids hidden";
      node.children.forEach(c => kids.appendChild(make(c, depth + 1)));
      wrap.appendChild(kids);
      const tw = row.querySelector(".tw");
      const toggle = () => {
        const open = kids.classList.toggle("hidden") === false;
        tw.textContent = open ? "▼" : "▶";
      };
      // Arrow toggles; single click on the row selects; double click toggles.
      tw.style.cursor = "pointer";
      tw.addEventListener("click", (e) => { e.stopPropagation(); toggle(); });
      row.addEventListener("click", () => select(row, node.id));
      row.addEventListener("dblclick", (e) => { e.preventDefault(); toggle(); });
    } else {
      row.addEventListener("click", () => select(row, node.id));
    }
    return wrap;
  }

  forest.forEach(node => frag.appendChild(make(node, 0)));
  treeEl.appendChild(frag);
  const note = rootOverride ? ` · rooted @ ${shortLabel(rootOverride)}` : "";
  statsEl.textContent = `${rows} rows · ${stubs} back-refs · ${layerSel.value} · ${dirSel.value}${note}`;
}

function select(row, id) {
  if (selRow) selRow.classList.remove("sel");
  selRow = row; if (row) row.classList.add("sel");
  selectedId = id; showNode(id);
}

function openAncestors(el) {
  let p = el.parentElement;
  while (p && p !== treeEl) {
    if (p.classList && p.classList.contains("tkids") && p.classList.contains("hidden")) {
      p.classList.remove("hidden");
      const tw = p.previousElementSibling && p.previousElementSibling.querySelector(".tw");
      if (tw) tw.textContent = "▼";
    }
    p = p.parentElement;
  }
}

function jumpToTreeRow(domId) {
  const el = document.getElementById(domId);
  if (!el) return;
  openAncestors(el);
  el.scrollIntoView({ block: "center", behavior: "smooth" });
  el.classList.remove("flash"); void el.offsetWidth; el.classList.add("flash");
  if (selRow) selRow.classList.remove("sel"); selRow = el; el.classList.add("sel");
}

// --------------------------------------------------------------------------- //
// Shared detail panel + navigation
// --------------------------------------------------------------------------- //

function neighborList(layer, id) {
  const out = { in: [], out: [] };
  const push = (arr, x) => { if (!arr.includes(x)) arr.push(x); };
  const pairs = layer === "filesystem" ? DATA.layers.filesystem
              : layer === "imports" ? DATA.layers.imports : DATA.layers.crate;
  pairs.forEach(e => { const s=e[0], d=e[1];
    if (s===id) push(out.out,d); if (d===id) push(out.in,s); });
  return out;
}

function showNode(id) {
  const n = nodeById.get(id);
  const side = document.getElementById("side");
  if (!n) { side.innerHTML = '<div class="meta">No data for '+esc(id)+'</div>'; return; }
  // Show every relationship regardless of the active view/layer.
  const imp = neighborList("imports", id);
  const fs = neighborList("filesystem", id);
  const crateNb = n.kind === "crate" ? neighborList("crate", id) : null;
  const link = (x) => `<div class="nbr" onclick="goTo('${jsq(x)}')">${esc(x)}</div>`;
  const section = (label, arr, layer) =>
    `<div class="seclabel">${label} (${arr.length})</div>` +
    (sortIds(arr, layer).map(link).join("") || '<div class="meta">—</div>');
  const isFolder = !!n.is_folder;
  const kindLabel = n.kind === "crate" ? "crate" : (isFolder ? "folder" : "file");
  const kindClass = n.kind === "crate" ? "kind-crate" : (isFolder ? "kind-folder" : "kind-file");
  const aggregate = n.kind === "crate" || isFolder;
  const locPills = aggregate
    ? `<span class="pill" title="filesystem-subtree total">Σ ${_subloc(id)} loc</span>` +
      `<span class="pill" title="this module's own file (e.g. mod.rs)">${n.loc} own</span>`
    : `<span class="pill">${n.loc} loc</span>`;
  side.innerHTML = `
    <h2>${esc(n.label)}</h2>
    <div class="meta">${esc(n.id)}</div>
    <div><span class="pill ${kindClass}">${kindLabel}</span> <span class="pill">${esc(n.crate)}</span>
         ${locPills}</div>
    <div style="margin:8px 0"><button onclick="rootTreeHere('${jsq(id)}')">⌖ root tree here</button></div>
    <div class="seclabel">path</div><div class="meta">${esc(n.path) || "—"}</div>
    <div class="seclabel">docstring</div>
    <pre>${esc(n.doc || "(no //! doc)")}</pre>
    ${section("dependencies — uses (imports →)", imp.out, "imports")}
    ${section("dependants — used by (imports ←)", imp.in, "imports")}
    ${section("filesystem children", fs.out, "filesystem")}
    ${crateNb ? section("crate deps →", crateNb.out, "crate") + section("crate dependants ←", crateNb.in, "crate") : ""}
  `;
}

function focusGraphNode(id) {
  if (!network) return;
  try { network.selectNodes([id]); network.focus(id, { scale: 1.1, animation: true }); } catch(e){}
}

// Navigate to a node in whatever view is active.
window.goTo = (id) => {
  if (viewSel.value === "tree") {
    const dom = currentFirstDom.get(id);
    if (dom) { jumpToTreeRow(dom); showNode(id); }
    else { rootTreeHere(id); }      // not in current forest (filtered/unreachable) -> reroot
  } else {
    focusGraphNode(id); showNode(id);
  }
};

window.rootTreeHere = (id) => {
  const n = nodeById.get(id);
  viewSel.value = "tree";
  rootOverride = id;
  if (n && crateSel.value && n.crate !== crateSel.value && n.kind !== "crate") crateSel.value = "";
  update();
  const dom = currentFirstDom.get(id);
  if (dom) jumpToTreeRow(dom);
  showNode(id);
};

// Search-as-you-type over every node id.
searchEl.addEventListener("input", () => {
  const q = searchEl.value.toLowerCase().trim();
  if (!q) { resultsEl.style.display = "none"; return; }
  const hits = DATA.nodes.filter(n => n.id.toLowerCase().includes(q)).slice(0, 40);
  resultsEl.innerHTML = hits.map(n =>
     `<div onclick="pick('${jsq(n.id)}')">${esc(n.id)}</div>`).join("");
  resultsEl.style.display = hits.length ? "block" : "none";
});
window.pick = (id) => {
  resultsEl.style.display = "none"; searchEl.value = "";
  const n = nodeById.get(id);
  if (n && crateSel.value && n.crate !== crateSel.value && n.kind !== "crate") crateSel.value = "";
  update();
  setTimeout(() => goTo(id), 50);
};

// --------------------------------------------------------------------------- //
// View dispatch + wiring
// --------------------------------------------------------------------------- //

function update() {
  const tree = viewSel.value === "tree";
  treeEl.classList.toggle("hidden", !tree);
  netEl.classList.toggle("hidden", tree);
  document.querySelectorAll(".tree-only").forEach(e => { e.style.display = tree ? "" : "none"; });
  if (tree) renderTree(); else renderGraph();
}

viewSel.addEventListener("change", update);
layerSel.addEventListener("change", () => { rootOverride = null; update(); });
dirSel.addEventListener("change", () => { rootOverride = null; update(); });
reducedEl.addEventListener("change", update);
function refreshSort() {
  sortDirBtn.textContent = sortAsc ? "▲ asc" : "▼ desc";
  update();
  if (selectedId) showNode(selectedId);
}
sortSel.addEventListener("change", () => {
  sortAsc = sortSel.value === "name";   // names read best A→Z; metrics biggest-first
  refreshSort();
});
sortDirBtn.addEventListener("click", () => { sortAsc = !sortAsc; refreshSort(); });
crateSel.addEventListener("change", () => { rootOverride = null; update(); });

// Drag the gutter to resize the side panel.
let dragging = false;
gutterEl.addEventListener("mousedown", (e) => {
  dragging = true; gutterEl.classList.add("drag");
  document.body.style.userSelect = "none"; e.preventDefault();
});
window.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  const w = window.innerWidth - e.clientX;
  sideEl.style.width = Math.max(220, Math.min(window.innerWidth - 200, w)) + "px";
});
window.addEventListener("mouseup", () => {
  if (!dragging) return;
  dragging = false; gutterEl.classList.remove("drag");
  document.body.style.userSelect = "";
  if (network) network.redraw();
});
document.getElementById("expandAll").onclick = () => {
  treeEl.querySelectorAll(".tkids").forEach(k => k.classList.remove("hidden"));
  treeEl.querySelectorAll(".trow .tw").forEach(tw => { if (tw.textContent === "▶") tw.textContent = "▼"; });
};
document.getElementById("collapseAll").onclick = () => {
  treeEl.querySelectorAll(".tkids").forEach(k => k.classList.add("hidden"));
  treeEl.querySelectorAll(".trow .tw").forEach(tw => { if (tw.textContent === "▼") tw.textContent = "▶"; });
};

// Initialize sort direction to match the default key, then render (collapsed).
sortAsc = sortSel.value === "name";
sortDirBtn.textContent = sortAsc ? "▲ asc" : "▼ desc";
update();
</script>
</body>
</html>
"""


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #


def repo_root() -> Path:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
        return Path(out.stdout.strip())
    except Exception:
        return Path.cwd()


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--root", type=Path, default=None, help="workspace root (default: git toplevel)")
    p.add_argument("--out-dir", type=Path, default=None,
                   help="output dir (default: <root>/.agent/reports/module-graph)")
    p.add_argument("--crate", default=None, help="limit to a single workspace crate")
    p.add_argument("--graphml", action="store_true", help="also write per-layer GraphML")
    p.add_argument("--no-print", dest="do_print", action="store_false",
                   help="skip nx.write_network_text stdout summary")
    p.add_argument("--no-html", dest="do_html", action="store_false", help="skip HTML explorer")
    args = p.parse_args(argv)

    root = (args.root or repo_root()).resolve()
    out_dir = (args.out_dir or (root / ".agent" / "reports" / "module-graph")).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    (all_nodes, g_fs, g_imports, g_crate, import_weights, crate_edges, crate_names) = build_graphs(
        root, args.crate
    )

    fs_edges = list(g_fs.edges())
    reduced = {
        "filesystem": reduce_graph(g_fs),
        "imports": reduce_graph(g_imports),
        "crate": reduce_graph(g_crate),
    }
    json_path = out_dir / "module_graph.json"
    write_json(json_path, all_nodes, fs_edges, import_weights, crate_edges, crate_names, reduced)

    if args.graphml:
        write_graphml(out_dir, g_fs, g_imports, g_crate)

    if args.do_html:
        html_path = out_dir / "module_explorer.html"
        build_html(json_path, html_path)

    if args.do_print:
        print_network_text(g_fs, g_imports, g_crate)

    print("\n" + "-" * 78)
    print(f"crates:  {len(crate_names)}")
    print(f"modules: {len(all_nodes)}")
    print(f"edges:   filesystem={g_fs.number_of_edges()}  "
          f"imports={g_imports.number_of_edges()}  crate={g_crate.number_of_edges()}")
    print(f"json:    {json_path}")
    if args.graphml:
        print(f"graphml: {out_dir}/module_graph.*.graphml")
    if args.do_html:
        print(f"html:    {out_dir / 'module_explorer.html'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
