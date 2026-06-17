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
  * A self-contained interactive HTML explorer (vis-network via CDN) with a
    layer toggle, crate filter, search, and a per-node detail panel showing the
    docstring and neighbors in every layer.
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
) -> None:
    nodes_json = [
        {
            "id": n.node_id,
            "label": n.node_id.split("::")[-1],
            "kind": n.kind,
            "crate": n.crate,
            "path": n.rel_path,
            "loc": n.loc,
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
  #side { width: 380px; overflow: auto; padding: 14px; background: #1a1d23;
          border-left: 1px solid #2a2e37; }
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
</style>
</head>
<body>
<div id="bar">
  <label>Layer
    <select id="layer">
      <option value="filesystem">filesystem (containment)</option>
      <option value="imports">imports (use)</option>
      <option value="crate">crate deps</option>
    </select>
  </label>
  <label>Crate
    <select id="crate"><option value="">all</option></select>
  </label>
  <label style="position:relative">Search
    <input id="search" placeholder="module name..." autocomplete="off" />
    <div id="results"></div>
  </label>
  <span id="stats"></span>
</div>
<div id="wrap">
  <div id="net"></div>
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

let network = null;

function edgesFor(layer) {
  if (layer === "filesystem") return DATA.layers.filesystem.map(([s,d]) => ({from:s,to:d}));
  if (layer === "imports") return DATA.layers.imports.map(([s,d,w]) => ({from:s,to:d,value:w,title:w+" use(s)"}));
  return DATA.layers.crate.map(([s,d,k]) => ({from:s,to:d,label:k==="normal"?"":k,
      dashes:k!=="normal", title:k}));
}

function render() {
  const layer = layerSel.value;
  const crate = crateSel.value;
  const rawEdges = edgesFor(layer);

  // Which nodes participate in this layer?
  const participating = new Set();
  rawEdges.forEach(e => { participating.add(e.from); participating.add(e.to); });

  let nodes = DATA.nodes.filter(n => {
    if (layer === "crate" && n.kind !== "crate") return false;
    if (crate && n.crate !== crate) return false;
    if (layer !== "crate" && !participating.has(n.id) && (n.kind!=="crate")) {
      // keep crate roots even if isolated, drop dangling leaves with no edge
    }
    return true;
  });
  const keep = new Set(nodes.map(n => n.id));
  let edges = rawEdges.filter(e => keep.has(e.from) && keep.has(e.to));

  const visNodes = nodes.map(n => ({
    id: n.id,
    label: n.label,
    title: n.summary || n.id,
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

  network = new vis.Network(document.getElementById("net"),
      { nodes: new vis.DataSet(visNodes), edges: new vis.DataSet(edges) }, options);
  network.on("selectNode", p => showNode(p.nodes[0]));
  statsEl.textContent = `${visNodes.length} nodes · ${edges.length} edges · ${layer}`;
}

function neighborList(layer, id) {
  const out = { in: [], out: [] };
  const push = (arr, x) => { if (!arr.includes(x)) arr.push(x); };
  if (layer === "filesystem") DATA.layers.filesystem.forEach(([s,d]) => {
    if (s===id) push(out.out,d); if (d===id) push(out.in,s); });
  else if (layer === "imports") DATA.layers.imports.forEach(([s,d]) => {
    if (s===id) push(out.out,d); if (d===id) push(out.in,s); });
  else DATA.layers.crate.forEach(([s,d]) => {
    if (s===id) push(out.out,d); if (d===id) push(out.in,s); });
  return out;
}

function showNode(id) {
  const n = nodeById.get(id);
  const side = document.getElementById("side");
  if (!n) { side.innerHTML = '<div class="meta">No data for '+id+'</div>'; return; }
  const layer = layerSel.value;
  const nb = neighborList(layer, id);
  const link = (x) => `<div class="nbr" onclick="focusNode('${x.replace(/'/g,"\\'")}')">${x}</div>`;
  side.innerHTML = `
    <h2>${n.label}</h2>
    <div class="meta">${n.id}</div>
    <div><span class="pill">${n.kind}</span> <span class="pill">${n.crate}</span>
         <span class="pill">${n.loc} loc</span></div>
    <div class="seclabel">path</div><div class="meta">${n.path || "—"}</div>
    <div class="seclabel">docstring</div>
    <pre>${(n.doc || "(no //! doc)").replace(/</g,"&lt;")}</pre>
    <div class="seclabel">${layer}: imported-by / parents (${nb.in.length})</div>
    ${nb.in.map(link).join("") || '<div class="meta">—</div>'}
    <div class="seclabel">${layer}: imports / children (${nb.out.length})</div>
    ${nb.out.map(link).join("") || '<div class="meta">—</div>'}
  `;
}

function focusNode(id) {
  if (!network) return;
  try { network.selectNodes([id]); network.focus(id, { scale: 1.1, animation: true }); } catch(e){}
  showNode(id);
}
window.focusNode = focusNode;

// Search-as-you-type over all nodes (jumps across layers if needed).
searchEl.addEventListener("input", () => {
  const q = searchEl.value.toLowerCase().trim();
  if (!q) { resultsEl.style.display="none"; return; }
  const hits = DATA.nodes.filter(n => n.id.toLowerCase().includes(q)).slice(0, 40);
  resultsEl.innerHTML = hits.map(n =>
     `<div onclick="pick('${n.id.replace(/'/g,"\\'")}')">${n.id}</div>`).join("");
  resultsEl.style.display = hits.length ? "block" : "none";
});
window.pick = (id) => {
  resultsEl.style.display = "none"; searchEl.value = "";
  const n = nodeById.get(id);
  // If the node isn't in the current layer's filtered view, relax the crate filter.
  if (n && crateSel.value && n.crate !== crateSel.value) crateSel.value = "";
  render(); setTimeout(() => focusNode(id), 60);
};

layerSel.addEventListener("change", render);
crateSel.addEventListener("change", render);
render();
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
    json_path = out_dir / "module_graph.json"
    write_json(json_path, all_nodes, fs_edges, import_weights, crate_edges, crate_names)

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
