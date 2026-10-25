"""The assets related to each asset, so that the page can go from one to the
next: the other forms of one character, the other incarnations of its
lineage, and the other art made the same way.

Each relation is derived from evidence the index already holds (the
lineages, the catalog character ids, the generator modules, the
character-generator specs, the names). It is a way to navigate, never an
authority: a name match says only that the names match.
"""

from __future__ import annotations

import re
from collections import defaultdict
from pathlib import Path
from typing import Any

#: A tag on more assets than this is a type, not a relation: the gallery's
#: facets reach it.
MAX_GROUP = 48

_VERSION = re.compile(r"_v\d+$")


def stem(name: str) -> str:
    """The name without a version suffix: `alice_v2` -> `alice`."""
    return _VERSION.sub("", name)


def _named_alike(a: str, b: str) -> bool:
    """True when one name is the other's form: the same stem, or one stem
    and an underscore start the other (`alice` and `alice_strike_fx`)."""
    sa, sb = stem(a), stem(b)
    if len(min(sa, sb, key=len)) < 3:
        return False
    return sa == sb or sb.startswith(sa + "_") or sa.startswith(sb + "_")


def chargen_traits(repo: Path, data_files: list[str]) -> dict[str, str]:
    """The family, base body and species of a character-generator spec."""
    import yaml  # noqa: PLC0415

    for rel in data_files:
        if not rel.endswith(".chargen.yaml"):
            continue
        try:
            spec = yaml.safe_load((repo / rel).read_text(encoding="utf-8")) or {}
        except (OSError, yaml.YAMLError):
            return {}
        head = spec.get("head") if isinstance(spec.get("head"), dict) else {}
        traits = {"family": spec.get("family") or "standard", "species": head.get("species") or "human"}
        if spec.get("base"):
            traits["base"] = spec["base"]
        return traits
    return {}


def relate(rows: dict[str, dict], generators: dict[str, dict], repo: Path) -> None:
    """Set `related` on each asset row: a list of groups, each
    `{kind, title, assets}`, nearest relation first. Uses the rows'
    `lineages` when the design history put them there."""
    by_target = {g.split(":", 1)[1]: aid for aid, r in rows.items() if (g := r.get("generator") or "").startswith("target:")}
    groups: dict[str, dict[tuple, set]] = defaultdict(lambda: defaultdict(set))

    # A target whose record lists no data files still has its spec beside it
    # (`data/characters/alice_v2/alice_v2.chargen.yaml`).
    renderer = repo / "tools" / "ambition_sprite2d_renderer"
    specs = {p.name[: -len(".chargen.yaml")]: p.relative_to(repo).as_posix()
             for p in sorted(renderer.glob("ambition_sprite2d_renderer/data/**/*.chargen.yaml"))}
    traits: dict[str, dict[str, str]] = {}
    for aid, row in rows.items():
        gen = generators.get(row.get("generator") or "") or {}
        data = list(gen.get("data") or [])
        if gen.get("family") == "sprite_target" and gen.get("id") in specs:
            data.append(specs[gen["id"]])
        if row.get("character_id"):
            groups["character"][(row["character_id"],)].add(aid)
        for lineage in row.get("lineages") or []:
            key = (lineage["lineage"], lineage["name"])
            groups["lineage"][key].add(aid)
            groups["lineage"][key].update(by_target[t] for t in lineage.get("targets", []) if t in by_target)
        for tag in row.get("tags") or []:
            groups["tag"][(tag,)].add(aid)
        if gen.get("module"):
            groups["module"][(gen["module"], gen.get("module_source") or "")].add(aid)
        traits[aid] = chargen_traits(repo, data)
        for trait, value in traits[aid].items():
            groups[f"chargen_{trait}"][(value,)].add(aid)

    names = [(aid, row["name"]) for aid, row in rows.items() if row.get("name")]
    alike: dict[str, set] = defaultdict(set)
    for i, (a, na) in enumerate(names):
        for b, nb in names[i + 1:]:
            if _named_alike(na, nb):
                alike[a].add(b)
                alike[b].add(a)

    titles = {
        "lineage": lambda k: f"Lineage: {k[1]}",
        "character": lambda k: f"Game character {k[0]}",
        "module": lambda k: f"Made by {Path(k[1]).name or k[0].rsplit('.', 1)[-1]}",
        "chargen_family": lambda k: f"Chargen family {k[0]}",
        "chargen_species": lambda k: f"Chargen species {k[0]}",
        "chargen_base": lambda k: f"Chargen base body {k[0]}",
        "tag": lambda k: f"Tagged {k[0]}",
    }
    order = ["lineage", "character", "named", "module", "chargen_family", "chargen_species", "chargen_base", "tag"]
    member: dict[str, list] = defaultdict(list)
    for kind, by_key in groups.items():
        for key, ids in by_key.items():
            if len(ids) < 2:
                continue
            for aid in ids:
                member[aid].append((kind, key, ids))
    for aid, row in rows.items():
        found = []
        for kind, key, ids in member.get(aid, []):
            if (kind == "chargen_species" and key == ("human",)) or (kind == "tag" and len(ids) > MAX_GROUP):
                continue  # every person, every prop: a type, not a relation (the gallery's facets)
            found.append({"kind": kind, "title": titles[kind](key), "assets": _ordered(ids - {aid}, rows)})
        if alike.get(aid):
            found.append({"kind": "named", "title": "Named alike", "assets": _ordered(alike[aid], rows)})
        found.sort(key=lambda g: (order.index(g["kind"]), g["title"]))
        # Two relations with the same members are one row with both titles.
        merged: list[dict] = []
        for g in found:
            same = next((m for m in merged if m["assets"] == g["assets"]), None)
            if same:
                same["title"] += " · " + g["title"]
            else:
                merged.append(g)
        found = merged
        row["related"] = found
        if traits.get(aid):
            row["chargen"] = traits[aid]


def _ordered(ids: set, rows: dict[str, dict]) -> list[str]:
    return sorted(ids, key=lambda x: (rows[x].get("display") or rows[x].get("name") or x).lower())
