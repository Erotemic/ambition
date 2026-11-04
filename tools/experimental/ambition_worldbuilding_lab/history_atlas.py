#!/usr/bin/env python3
"""The History atlas: an ideal graph of world history, to see what the
History dimension is missing.

    uv run --project tools/experimental/ambition_worldbuilding_lab \\
        python tools/experimental/ambition_worldbuilding_lab/history_atlas.py <command>

Commands:
    check                 the data is whole, and the built part agrees with
                          the game (the rooms of make_specs.py, the ways of
                          history.ldtk)
    report [--out F]      the coverage report (Markdown)
    plot --out F.png      the graph, time across and the clusters down
    where <country|people> "where's my history?" for one player

The data is `data/history_atlas/*.yaml` (see each file's header). The graph
is an aid to planning, not canon: it changes no game file.

Jon, 2026-11-04: "Imagine different players around the world playing and
then asking themselves: where's my history? Do we have an answer? Let's
make sure there is an answer and it is a good one."
"""
from __future__ import annotations

import argparse
import itertools
import json
import math
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import networkx as nx
import yaml

HERE = Path(__file__).resolve().parent
DATA = HERE / "data" / "history_atlas"
REPO = HERE.parents[2]
WORLD = REPO / "game" / "ambition_map_assets" / "ambition_content" / "worlds" / "history.ldtk"
SPECS = REPO / "game" / "ambition_map_assets" / "ambition_content" / "generators" / "history"

STATUSES = ("built", "planned", "candidate")
MODES = ("grounded", "parody", "sober", "archive")
KINDS = ("succession", "contact", "diffusion", "comparison", "atlas_shortcut")


# ---- Loading -----------------------------------------------------------------


def load(data: Path = DATA) -> dict:
    """Read the atlas. Returns a dict of the parts; `check` says what is wrong."""
    frame = yaml.safe_load((data / "frame.yaml").read_text())
    countries = {}
    for group in yaml.safe_load((data / "countries.yaml").read_text()).values():
        for row in group:
            code, name, region, pop = row[:4]
            extra = row[4] if len(row) > 4 else {}
            countries[code] = {"code": code, "name": name, "region": region, "pop": float(pop),
                               "member": extra.get("member", True)}
    peoples = {pid: {"id": pid, "name": name, "countries": cs}
               for pid, name, cs in yaml.safe_load((data / "peoples.yaml").read_text())["peoples"]}
    topics = {tid: {"id": tid, "name": name, "kind": kind}
              for tid, name, kind in yaml.safe_load((data / "topics.yaml").read_text())["topics"]}
    nodes = {}
    duplicate = []
    for n in yaml.safe_load((data / "nodes.yaml").read_text())["nodes"]:
        # A title that is a year ("1348") reads as a number.
        n["title"] = str(n["title"])
        n.setdefault("mode", "grounded")
        n.setdefault("shape", "room")
        n.setdefault("peoples", [])
        n.setdefault("topics", [])
        n.setdefault("countries", [])
        if n["id"] in nodes:
            duplicate.append(n["id"])
        nodes[n["id"]] = n
    edges_doc = yaml.safe_load((data / "edges.yaml").read_text())
    edges = []
    for group in ("built", "links"):
        for row in edges_doc.get(group, []):
            a, b, kind = row[:3]
            edges.append({"a": a, "b": b, "kind": kind, "note": row[3] if len(row) > 3 else "", "built": group == "built"})
    region_cluster = {r: c for c, spec in frame["clusters"].items() for r in spec["regions"]}
    return {"frame": frame, "countries": countries, "peoples": peoples, "topics": topics, "nodes": nodes,
            "edges": edges, "region_cluster": region_cluster, "duplicate_ids": duplicate}


def horizon_of(atlas: dict, node: dict) -> str:
    """The horizon of a node: the one its middle year falls in; where two
    overlap, the later one."""
    if "horizon" in node:
        return node["horizon"]
    a, b = node["years"]
    mid = (a + b) / 2
    found = None
    for h in atlas["frame"]["horizons"]:
        if h["from"] <= mid <= h["to"]:
            found = h["id"]
    return found or atlas["frame"]["horizons"][-1]["id"]


def cluster_of(atlas: dict, node: dict) -> str:
    return atlas["region_cluster"][node["region"]]


def graph(atlas: dict, statuses=STATUSES) -> nx.Graph:
    """The atlas as an undirected graph of the nodes of `statuses`."""
    g = nx.Graph()
    for n in atlas["nodes"].values():
        if n["status"] in statuses:
            g.add_node(n["id"], **n)
    for e in atlas["edges"]:
        if e["a"] in g and e["b"] in g:
            g.add_edge(e["a"], e["b"], kind=e["kind"], note=e["note"], built=e["built"])
    return g


# ---- Checks ------------------------------------------------------------------


def game_rooms() -> set[str]:
    """The room ids `make_specs.py` makes (parsed from its `GRID` and its
    buildings, so that this tool does not import the generator)."""
    text = (SPECS / "make_specs.py").read_text()
    grid = set(re.findall(r'^\s*\([0-9.]+, [0-9.]+\): "(history_[a-z_]+)"', text, re.M))
    consts = dict(re.findall(r'^([A-Z]+) = "(history_[a-z_]+)"', text, re.M))
    buildings = re.search(r"^BUILDINGS = \(([^)]*)\)", text, re.M).group(1)
    return grid | {consts[name.strip()] for name in buildings.split(",") if name.strip()}


def game_ways(world: Path = WORLD) -> set[frozenset]:
    """The pairs of History rooms that a loading zone joins in the world file."""
    data = json.loads(world.read_text())
    ways = set()
    for level in data["levels"]:
        rid = level["identifier"]
        if not rid.startswith("history_"):
            continue
        for layer in level.get("layerInstances") or []:
            for e in layer["entityInstances"]:
                if e["__identifier"] != "LoadingZone":
                    continue
                f = {x["__identifier"]: x["__value"] for x in e["fieldInstances"]}
                target = f.get("target_room") or ""
                if target.startswith("history_") and target != rid:
                    ways.add(frozenset((rid, target)))
    return ways


def check(atlas: dict, *, against_game: bool = True) -> list[str]:
    """Every problem with the data, as sentences. Empty means whole."""
    bad = [f"node id {i} is used twice" for i in atlas["duplicate_ids"]]
    countries, peoples, topics, nodes = atlas["countries"], atlas["peoples"], atlas["topics"], atlas["nodes"]
    regions = atlas["region_cluster"]
    shapes = set(atlas["frame"]["shapes"])
    for c in countries.values():
        if c["region"] not in regions:
            bad.append(f"country {c['code']} names unknown region {c['region']}")
    for p in peoples.values():
        bad += [f"people {p['id']} names unknown country {c}" for c in p["countries"] if c not in countries]
    keys = {"id", "title", "region", "years", "status", "room", "countries", "peoples", "topics", "mode", "shape", "note", "horizon"}
    for n in nodes.values():
        where = f"node {n['id']}"
        bad += [f"{where}: unknown key {k!r} (an unquoted comma?)" for k in n if k not in keys]
        if n["region"] not in regions:
            bad.append(f"{where}: unknown region {n['region']}")
        if n["status"] not in STATUSES:
            bad.append(f"{where}: unknown status {n['status']}")
        if n["mode"] not in MODES:
            bad.append(f"{where}: unknown mode {n['mode']}")
        if n["shape"] not in shapes:
            bad.append(f"{where}: unknown shape {n['shape']}")
        if (n["status"] == "built") != ("room" in n):
            bad.append(f"{where}: a built node has a room, and only a built node")
        a, b = n["years"]
        if a > b:
            bad.append(f"{where}: years run backwards")
        bad += [f"{where}: unknown country {c}" for c in n["countries"] if c not in countries]
        bad += [f"{where}: unknown people {p}" for p in n["peoples"] if p not in peoples]
        bad += [f"{where}: unknown topic {t}" for t in n["topics"] if t not in topics]
        if not n["countries"] and not n["peoples"]:
            bad.append(f"{where}: names no country and no people")
    seen = set()
    for e in atlas["edges"]:
        for end in (e["a"], e["b"]):
            if end not in nodes:
                bad.append(f"edge {e['a']} - {e['b']}: unknown node {end}")
        if e["kind"] not in KINDS:
            bad.append(f"edge {e['a']} - {e['b']}: unknown kind {e['kind']}")
        key = frozenset((e["a"], e["b"]))
        if key in seen:
            bad.append(f"edge {e['a']} - {e['b']} is listed twice")
        seen.add(key)
        if e["built"] and any(nodes.get(x, {}).get("status") != "built" for x in (e["a"], e["b"])):
            bad.append(f"built edge {e['a']} - {e['b']} joins a node that is not built")
    if against_game:
        room_of = {n["room"]: n["id"] for n in nodes.values() if "room" in n}
        rooms = game_rooms()
        bad += [f"the game makes room {r}, and no built node has it" for r in sorted(rooms - set(room_of))]
        bad += [f"built node {room_of[r]} names room {r}, which the game does not make" for r in sorted(set(room_of) - rooms)]
        if WORLD.exists():
            built = {frozenset((nodes[e["a"]]["room"], nodes[e["b"]]["room"])) for e in atlas["edges"]
                     if e["built"] and e["a"] in nodes and e["b"] in nodes and "room" in nodes[e["a"]] and "room" in nodes[e["b"]]}
            ways = {w for w in game_ways() if w <= set(room_of)}
            bad += [f"the game joins {' and '.join(sorted(w))}; edges.yaml `built:` does not" for w in sorted(ways - built, key=sorted)]
            bad += [f"edges.yaml `built:` joins {' and '.join(sorted(w))}; the game does not" for w in sorted(built - ways, key=sorted)]
    return bad


# ---- Coverage ----------------------------------------------------------------


def best_status(found: dict) -> str:
    """The best status with at least one node in `found` (status -> ids)."""
    for s in STATUSES:
        if found.get(s):
            return s
    return "none"


def country_coverage(atlas: dict) -> list[dict]:
    """For each country: its nodes by status, and its best status."""
    by_country = defaultdict(lambda: defaultdict(list))
    for n in atlas["nodes"].values():
        for c in n["countries"]:
            by_country[c][n["status"]].append(n["id"])
    rows = []
    for c in atlas["countries"].values():
        found = by_country[c["code"]]
        rows.append({**c, **{s: sorted(found[s]) for s in STATUSES}, "best": best_status(found)})
    return sorted(rows, key=lambda r: -r["pop"])


def people_coverage(atlas: dict) -> list[dict]:
    by_people = defaultdict(lambda: defaultdict(list))
    for n in atlas["nodes"].values():
        for p in n["peoples"]:
            by_people[p][n["status"]].append(n["id"])
    return [{**p, **{s: sorted(by_people[p["id"]][s]) for s in STATUSES}, "best": best_status(by_people[p["id"]])}
            for p in atlas["peoples"].values()]


def topic_coverage(atlas: dict) -> list[dict]:
    by_topic = defaultdict(lambda: defaultdict(list))
    for n in atlas["nodes"].values():
        for t in n["topics"]:
            by_topic[t][n["status"]].append(n["id"])
    return [{**t, **{s: sorted(by_topic[t["id"]][s]) for s in STATUSES}, "best": best_status(by_topic[t["id"]])}
            for t in atlas["topics"].values()]


def matrix(atlas: dict) -> dict:
    """(cluster, horizon) -> Counter of statuses."""
    out = defaultdict(Counter)
    for n in atlas["nodes"].values():
        out[(cluster_of(atlas, n), horizon_of(atlas, n))][n["status"]] += 1
    return out


# ---- Layout ------------------------------------------------------------------


def cluster_weights(atlas: dict, statuses=("built", "planned")) -> Counter:
    """How many links of the nodes of `statuses` join each pair of clusters.
    A comparison is not a road, so it does not count."""
    g = graph(atlas, statuses)
    w = Counter()
    for a, b, d in g.edges(data=True):
        if d["kind"] == "comparison":
            continue
        ca, cb = cluster_of(atlas, g.nodes[a]), cluster_of(atlas, g.nodes[b])
        if ca != cb:
            w[frozenset((ca, cb))] += 1
    return w


def best_floor_order(clusters: list[str], weights: Counter, fixed_first: str | None = None) -> tuple[list[str], int]:
    """The order of the clusters as floors, one above the next, that puts the
    most link weight between neighbours (a maximum-weight Hamiltonian path,
    by dynamic programming over subsets; fine for a dozen clusters)."""
    n = len(clusters)
    idx = {c: i for i, c in enumerate(clusters)}
    W = [[weights.get(frozenset((a, b)), 0) for b in clusters] for a in clusters]
    best = {}
    starts = [idx[fixed_first]] if fixed_first else range(n)
    for i in starts:
        best[(1 << i, i)] = (0, None)
    for mask in range(1, 1 << n):
        for last in range(n):
            if (mask, last) not in best:
                continue
            score = best[(mask, last)][0]
            for nxt in range(n):
                if mask & (1 << nxt):
                    continue
                key = (mask | (1 << nxt), nxt)
                cand = score + W[last][nxt]
                if key not in best or cand > best[key][0]:
                    best[key] = (cand, last)
    full = (1 << n) - 1
    score, last = max((best[(full, i)][0], i) for i in range(n) if (full, i) in best)
    order, mask = [], full
    while last is not None:
        order.append(clusters[last])
        prev = best[(mask, last)][1]
        mask &= ~(1 << last)
        last = prev
    return order[::-1], score


# ---- Report ------------------------------------------------------------------

MARK = {"built": "BUILT", "planned": "planned", "candidate": "candidate", "none": "**NONE**"}


def _names(atlas, ids, limit=6):
    names = [atlas["nodes"][i]["title"] for i in ids]
    more = len(names) - limit
    return "; ".join(names[:limit]) + (f"; and {more} more" if more > 0 else "")


def report(atlas: dict) -> str:
    nodes, edges = atlas["nodes"], atlas["edges"]
    status_count = Counter(n["status"] for n in nodes.values())
    kind_count = Counter(e["kind"] for e in edges)
    countries = country_coverage(atlas)
    total_pop = sum(c["pop"] for c in countries)
    out = ["# The History atlas: coverage report", "",
           "Generated by `tools/experimental/ambition_worldbuilding_lab/history_atlas.py report`. Do not edit by hand:",
           "edit `data/history_atlas/*.yaml` and generate it again. The atlas is a plan, not canon.", "",
           "## Summary", "",
           f"- Nodes: {len(nodes)} ({status_count['built']} built, {status_count['planned']} planned, {status_count['candidate']} candidate).",
           f"- Links: {len(edges)} ({', '.join(f'{kind_count[k]} {k}' for k in KINDS)}).",
           f"- Countries and territories: {len(countries)}; peoples: {len(atlas['peoples'])}; topics: {len(atlas['topics'])}.", ""]

    # Where's my history?
    def pop_share(level):
        keep = STATUSES[:STATUSES.index(level) + 1]
        return sum(c["pop"] for c in countries if c["best"] in keep) / total_pop
    out += ["## Where's my history?", "",
            "For a player from each country: the places of the atlas that are their history. The population is a rounded",
            "2024 estimate, used only to weigh how many players ask (`countries.yaml`).", "",
            f"- A built room: {sum(c['best'] == 'built' for c in countries)} countries, {pop_share('built'):.0%} of the world's people.",
            f"- At least a planned room: {sum(c['best'] in ('built', 'planned') for c in countries)} countries, {pop_share('planned'):.0%}.",
            f"- At least a candidate: {sum(c['best'] != 'none' for c in countries)} countries, {pop_share('candidate'):.0%}.",
            f"- No answer at all: {sum(c['best'] == 'none' for c in countries)} countries.", "",
            "| country | people (M) | best | built | planned | candidates |", "| --- | ---: | --- | --- | --- | ---: |"]
    for c in countries:
        out.append(f"| {c['name']} | {c['pop']:g} | {MARK[c['best']]} | {_names(atlas, c['built'], 4)} | {_names(atlas, c['planned'], 4)} | {len(c['candidate'])} |")
    out += ["", "### Peoples", "", "| people | best | built | planned | candidates |", "| --- | --- | --- | --- | ---: |"]
    for p in sorted(people_coverage(atlas), key=lambda p: (STATUSES + ("none",)).index(p["best"]), reverse=True):
        out.append(f"| {p['name']} | {MARK[p['best']]} | {_names(atlas, p['built'], 4)} | {_names(atlas, p['planned'], 4)} | {len(p['candidate'])} |")

    # Topics.
    out += ["", "## Topics", "",
            "An obvious topic with nothing planned is the failure Jon named: \"obvious non-neglected topics that we are neglecting\".", "",
            "| topic | kind | best | built | planned | candidates |", "| --- | --- | --- | --- | --- | ---: |"]
    for t in sorted(topic_coverage(atlas), key=lambda t: ((STATUSES + ("none",)).index(t["best"]) * -1, t["kind"])):
        out.append(f"| {t['name']} | {t['kind']} | {MARK[t['best']]} | {_names(atlas, t['built'], 3)} | {_names(atlas, t['planned'], 3)} | {len(t['candidate'])} |")

    # The matrix.
    horizons = atlas["frame"]["horizons"]
    m = matrix(atlas)
    out += ["", "## Clusters and horizons", "",
            "Each cell: built / planned / candidate. An empty cell is a time and place the atlas has nothing for; it is not a",
            "claim that nobody lived there.", "",
            "| cluster | " + " | ".join(h["id"] for h in horizons) + " |", "| --- |" + " ---: |" * len(horizons)]
    for cid, spec in atlas["frame"]["clusters"].items():
        cells = []
        for h in horizons:
            c = m.get((cid, h["id"]), Counter())
            cells.append(f"{c['built']}/{c['planned']}/{c['candidate']}" if c else "")
        out.append(f"| {spec['name']} | " + " | ".join(cells) + " |")
    out += ["", "Horizons: " + "; ".join(f"{h['id']} {h['name']}" for h in horizons) + "."]

    # The built ways.
    built = [e for e in edges if e["built"]]
    shortcuts = [e for e in built if e["kind"] == "atlas_shortcut"]
    out += ["", "## The ways the game has", "",
            f"{len(built)} ways join the built rooms; {len(shortcuts)} of them are the atlas's fiction, not history. Each must say so",
            "in its room (a sign, Clio), or be moved when the place it should lead to is built:", ""]
    out += [f"- {nodes[e['a']]['title']} and {nodes[e['b']]['title']}" + (f": {e['note']}" if e["note"] else "") for e in shortcuts]
    notes = [n for n in nodes.values() if n["status"] == "built" and n.get("note")]
    out += ["", "What the built rooms must get right (each node's `note`):", ""]
    out += [f"- {n['title']} (`{n['room']}`): {n['note']}" for n in notes]

    # Layout.
    clusters = [c for c in atlas["frame"]["clusters"] if c != "seas"]
    w = cluster_weights(atlas)
    order, score = best_floor_order(clusters, w, fixed_first=None)
    total = sum(v for k, v in w.items() if "seas" not in k)
    out += ["", "## Can it be one map?", "",
            "The clusters as floors of one map: the order that puts the most links (built and planned, not comparisons)",
            "between neighbouring floors. A link between two floors that do not touch needs a corridor room, a ship, a door,",
            "or a gate.", "",
            f"Best order, top to bottom: {' / '.join(atlas['frame']['clusters'][c]['name'] for c in order)}.",
            f"It makes {score} of the {total} links between clusters walkable floor to floor; the rest need a corridor or a gate:", ""]
    adjacent = {frozenset(p) for p in zip(order, order[1:])}
    far = sorted(((v, k) for k, v in w.items() if k not in adjacent and "seas" not in k), key=lambda x: -x[0])
    out += [f"- {' and '.join(atlas['frame']['clusters'][c]['name'] for c in sorted(k))}: {v}" for v, k in far]
    seas = sorted(((v, k) for k, v in w.items() if "seas" in k), key=lambda x: -x[0])
    out += ["", "The seas and networks are corridors by nature (`shape: corridor`); their links to each cluster:", ""]
    out += [f"- {' and '.join(atlas['frame']['clusters'][c]['name'] for c in sorted(k))}: {v}" for v, k in seas]

    # Connectivity.
    g = graph(atlas, ("built", "planned"))
    comps = sorted(nx.connected_components(g), key=len, reverse=True)
    out += ["", "## Is it one world?", "",
            f"The built and planned nodes make {len(comps)} connected part(s) by their links."]
    if len(comps) > 1:
        out += [f"- Apart from the rest: {_names(atlas, sorted(c), 8)}" for c in comps[1:]]
    if "cradle" in g:
        dist = nx.single_source_shortest_path_length(g, "cradle")
        far_nodes = sorted(dist.items(), key=lambda kv: -kv[1])[:8]
        out += ["", "The farthest places from the Cradle, in links: " +
                "; ".join(f"{nodes[i]['title']} ({d})" for i, d in far_nodes) + "."]
    return "\n".join(out) + "\n"


# ---- Plot --------------------------------------------------------------------


def time_x(year: float) -> float:
    """Time across: the log of the years before 2030, so that deep time and
    the last century both have room. Later is to the right."""
    return -math.log10(2030 - year)


def plot(atlas: dict, out: Path) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    clusters = list(atlas["frame"]["clusters"])
    regions = [(c, r) for c in clusters for r in atlas["frame"]["clusters"][c]["regions"]]
    row = {r: i for i, (_, r) in enumerate(regions)}
    pos = {}
    for n in atlas["nodes"].values():
        a, b = n["years"]
        jitter = (hash(n["id"]) % 7 - 3) * 0.08
        pos[n["id"]] = (time_x((a + b) / 2), -row[n["region"]] + jitter)
    fig, ax = plt.subplots(figsize=(22, 26))
    colour = {"built": "#1b7837", "planned": "#2166ac", "candidate": "#b2b2b2"}
    style = {"succession": "-", "contact": "-", "diffusion": "--", "comparison": ":", "atlas_shortcut": "-."}
    for e in atlas["edges"]:
        if e["a"] in pos and e["b"] in pos:
            (x0, y0), (x1, y1) = pos[e["a"]], pos[e["b"]]
            ax.plot([x0, x1], [y0, y1], style[e["kind"]], color="#d6604d" if e["kind"] == "atlas_shortcut" else "#888888",
                    lw=1.4 if e["built"] else 0.6, alpha=0.9 if e["built"] else 0.45, zorder=1)
    for n in atlas["nodes"].values():
        x, y = pos[n["id"]]
        ax.scatter([x], [y], s=70 if n["status"] == "built" else 40, color=colour[n["status"]], zorder=3,
                   edgecolors="black" if n["mode"] == "sober" else "none", linewidths=1.2)
        ax.annotate(n["title"], (x, y), fontsize=6.5, xytext=(3, 2), textcoords="offset points", zorder=4)
    for i, (c, r) in enumerate(regions):
        ax.axhline(-i - 0.5, color="#eeeeee", lw=0.5, zorder=0)
        ax.text(time_x(-330000), -i, r.replace("_", " "), fontsize=7, ha="right", va="center")
    first = 0
    for c in clusters:
        k = len(atlas["frame"]["clusters"][c]["regions"])
        ax.axhspan(-first + 0.5, -first - k + 0.5, color="#f7f7f7" if clusters.index(c) % 2 else "white", zorder=-1)
        ax.text(time_x(2029.5), -first - (k - 1) / 2, atlas["frame"]["clusters"][c]["name"], fontsize=9, ha="left", va="center", weight="bold")
        first += k
    ticks = [-300000, -100000, -50000, -20000, -10000, -5000, -3000, -1000, 1, 500, 1000, 1500, 1800, 1900, 1950, 2000]
    ax.set_xticks([time_x(t) for t in ticks])
    ax.set_xticklabels([f"{-t:,} BCE" if t < 0 else f"{t} CE" for t in ticks], rotation=45, fontsize=8)
    ax.set_yticks([])
    ax.set_xlim(time_x(-340000), time_x(2029.9))
    ax.set_title("The History atlas: green built, blue planned, grey candidate; black ring sober; red dash-dot an atlas shortcut", fontsize=12)
    fig.tight_layout()
    fig.savefig(out, dpi=110)


# ---- Where ---------------------------------------------------------------------


def where(atlas: dict, who: str) -> str:
    key = who.strip().lower()
    for c in country_coverage(atlas):
        if key in (c["code"].lower(), c["name"].lower()):
            lines = [f"{c['name']}: {MARK[c['best']]}"]
            for s in STATUSES:
                lines += [f"  {s}: {atlas['nodes'][i]['title']}" for i in c[s]]
            return "\n".join(lines)
    for p in people_coverage(atlas):
        if key in (p["id"], p["name"].lower()):
            lines = [f"{p['name']}: {MARK[p['best']]}"]
            for s in STATUSES:
                lines += [f"  {s}: {atlas['nodes'][i]['title']}" for i in p[s]]
            return "\n".join(lines)
    return f"no country or people called {who!r}"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="command", required=True)
    sub.add_parser("check")
    rp = sub.add_parser("report")
    rp.add_argument("--out", type=Path)
    pp = sub.add_parser("plot")
    pp.add_argument("--out", type=Path, required=True)
    wp = sub.add_parser("where")
    wp.add_argument("who")
    args = ap.parse_args(argv)
    atlas = load()
    if args.command == "check":
        bad = check(atlas)
        for line in bad:
            print(line)
        print(f"{len(bad)} problem(s)")
        return 1 if bad else 0
    if args.command == "report":
        text = report(atlas)
        if args.out:
            args.out.write_text(text)
        else:
            sys.stdout.write(text)
        return 0
    if args.command == "plot":
        plot(atlas, args.out)
        return 0
    print(where(atlas, args.who))
    return 0


if __name__ == "__main__":
    sys.exit(main())
