#!/usr/bin/env python3
"""Explore Ambition world concepts with reproducible numerical comparisons.

This is an offline authoring tool. It does not edit the game or its world canon.
"""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import csv
from dataclasses import dataclass
from html import escape as escape_xml
import json
from pathlib import Path
import sys
from typing import Any

import numpy as np

TOOL_DIR = Path(__file__).resolve().parent
DEFAULT_CATALOG = TOOL_DIR / "data" / "catalog.json"
DEFAULT_SCENARIOS = TOOL_DIR / "data" / "world_scenarios.json"
EXCLUDED_FAMILIES = frozenset({"Unassigned", "World topology"})
SCOPES: dict[str, frozenset[str]] = {
    "original": frozenset({"original"}),
    "core": frozenset({"original", "built", "conversation_user"}),
    "expanded": frozenset({"original", "built", "conversation_user", "later"}),
    "all": frozenset({
        "original", "built", "conversation_user", "later", "llm",
        "conversation_proposed", "hypothesis",
    }),
}
SIGN_ANCHORS = ("creative", "formal", "organic", "kinetic")


class InputError(ValueError):
    """The catalog or a scenario does not meet the input contract."""


def read_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise InputError(f"Cannot read JSON from {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise InputError(f"Expected an object in {path}")
    return data


def load_catalog(path: Path = DEFAULT_CATALOG, supplements: tuple[Path, ...] = ()) -> dict[str, Any]:
    catalog = read_json(path)
    if catalog.get("schema_version") != 1:
        raise InputError("Catalog schema_version must be 1")
    records = catalog.get("records")
    definitions = catalog.get("features")
    if not isinstance(records, list) or not isinstance(definitions, list):
        raise InputError("Catalog must have features and records arrays")
    feature_ids = [f.get("id") for f in definitions if isinstance(f, dict)]
    if not feature_ids or len(feature_ids) != len(definitions) or len(set(feature_ids)) != len(feature_ids):
        raise InputError("Feature definitions require distinct ids")
    if any(not isinstance(x, str) or not x for x in feature_ids):
        raise InputError("Every feature id must be a nonempty string")
    known_origins = catalog.get("origin_notes", {})
    for path_extra in supplements:
        extension = read_json(path_extra)
        if extension.get("schema_version") != 1 or not isinstance(extension.get("records"), list):
            raise InputError(f"Supplement {path_extra} needs schema_version=1 and a records array")
        records = [*records, *extension["records"]]
    catalog = {**catalog, "records": records}
    ids: set[str] = set()
    for n, record in enumerate(records, 1):
        if not isinstance(record, dict):
            raise InputError(f"Record {n} must be an object")
        for field in ("id", "name", "family", "kind", "origin"):
            if not isinstance(record.get(field), str) or not record[field].strip():
                raise InputError(f"Record {n}: missing string field {field}")
        rid = record["id"]
        if rid in ids:
            raise InputError(f"Duplicate concept id: {rid}")
        ids.add(rid)
        if record["origin"] not in known_origins:
            raise InputError(f"{rid}: unknown origin {record['origin']}")
        tags = record.get("tags")
        if (not isinstance(tags, list) or any(not isinstance(t, str) for t in tags)
                or len(tags) != len(set(tags))):
            raise InputError(f"{rid}: tags must be a list of unique strings")
        unknown = set(tags) - set(feature_ids)
        if unknown:
            raise InputError(f"{rid}: unknown features {sorted(unknown)}")
        sources = record.get("source_scope")
        if not isinstance(sources, list) or not sources or any(not isinstance(s, str) or not s for s in sources):
            raise InputError(f"{rid}: source_scope must list evidence leads")
    return catalog


def load_scenarios(path: Path, catalog: dict[str, Any]) -> dict[str, Any]:
    data = read_json(path)
    if data.get("schema_version") != 1 or not isinstance(data.get("scenarios"), list):
        raise InputError("Scenario schema_version must be 1 and scenarios must be an array")
    family_ids = {r["family"] for r in catalog["records"]}
    scenario_ids: set[str] = set()
    protected = set(data.get("protected_world_ids", ()))
    if "math" not in protected:
        raise InputError("Math must remain protected in the comparison scenarios")
    for scenario in data["scenarios"]:
        sid = scenario.get("id")
        if not isinstance(sid, str) or not sid or sid in scenario_ids:
            raise InputError("Scenarios require distinct ids")
        scenario_ids.add(sid)
        world_ids: set[str] = set()
        for world in scenario.get("worlds", []):
            wid = world.get("id")
            if not isinstance(wid, str) or not wid or wid in world_ids:
                raise InputError(f"Scenario {sid}: world ids must be distinct")
            world_ids.add(wid)
            if (not isinstance(world.get("physical_setting"), str)
                    or not world["physical_setting"].strip()
                    or not isinstance(world.get("label"), str) or not world["label"].strip()):
                raise InputError(f"Scenario {sid}/{wid}: give a physical setting and label")
            families = world.get("families")
            if (not isinstance(families, list) or any(not isinstance(f, str) for f in families)
                    or len(families) != len(set(families))):
                raise InputError(f"Scenario {sid}/{wid}: families must be a unique string list")
            if (unknown := set(families) - family_ids):
                raise InputError(f"Scenario {sid}/{wid}: unknown families {sorted(unknown)}")
            if "philosophy" in wid.lower() or "between" in wid.lower():
                raise InputError(f"Scenario {sid}: do not invent a philosophy or between-world dimension")
            if wid == "math" and families != ["Formal mathematics"]:
                raise InputError(f"Scenario {sid}: protected Math must keep Formal mathematics only")
        if not protected.issubset(world_ids):
            raise InputError(f"Scenario {sid}: lacks protected world ids {sorted(protected - world_ids)}")
    return data


def feature_names(catalog: dict[str, Any]) -> list[str]:
    return [f["id"] for f in catalog["features"]]


def selected_records(catalog: dict[str, Any], scope: str) -> list[dict[str, Any]]:
    allowed = SCOPES[scope]
    return [
        record for record in catalog["records"]
        if record["origin"] in allowed
        and record["family"] not in EXCLUDED_FAMILIES
        and record["kind"] != "setting structure"
    ]


@dataclass(frozen=True)
class FamilyData:
    names: tuple[str, ...]
    matrix: np.ndarray
    sizes: dict[str, int]
    examples: dict[str, tuple[str, ...]]
    features: tuple[str, ...]

    def index(self, name: str) -> int:
        try:
            return self.names.index(name)
        except ValueError as exc:
            raise InputError(f"Unknown active family {name!r}; choose one of {', '.join(self.names)}") from exc


def family_data(catalog: dict[str, Any], scope: str) -> FamilyData:
    features = tuple(feature_names(catalog))
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in selected_records(catalog, scope):
        groups[record["family"]].append(record)
    if len(groups) < 2:
        raise InputError(f"Scope {scope!r} has fewer than two usable concept families")
    names = tuple(sorted(groups))
    mat = np.array([
        [sum(f in r["tags"] for r in groups[family]) / len(groups[family]) for f in features]
        for family in names
    ], dtype=float)
    return FamilyData(
        names=names,
        matrix=mat,
        sizes={family: len(groups[family]) for family in names},
        examples={family: tuple(r["name"] for r in groups[family][:5]) for family in names},
        features=features,
    )


def pca(data: FamilyData) -> dict[str, Any]:
    """Center the family profiles and compute a full deterministic SVD."""
    X = data.matrix
    mean = X.mean(axis=0)
    centered = X - mean
    _u, singular, vh = np.linalg.svd(centered, full_matrices=False)
    scores = centered @ vh.T
    # SVD signs are arbitrary. Set a stable, readable sign when possible.
    for j, anchor in enumerate(SIGN_ANCHORS[:len(vh)]):
        if anchor in data.features and vh[j, data.features.index(anchor)] < 0:
            vh[j] *= -1
            scores[:, j] *= -1
    eigen = singular ** 2
    ratio = eigen / eigen.sum() if eigen.sum() else np.zeros_like(eigen)
    return {
        "mean": mean,
        "loadings": vh,
        "scores": scores,
        "variance": ratio,
        "cumulative": np.cumsum(ratio),
    }


def bootstrap_pca(data: FamilyData, *, repeats: int, seed: int) -> dict[str, Any]:
    """Measure sensitivity to family sampling, not statistical confidence."""
    if repeats < 0:
        raise InputError("Bootstrap repeats cannot be negative")
    if not repeats:
        return {"repeats": 0, "note": "No resampling requested"}
    ref = pca(data)
    rng = np.random.default_rng(seed)
    ratios: list[list[float]] = []
    alignments: list[list[float]] = []
    top = min(3, data.matrix.shape[1])
    for _ in range(repeats):
        indexes = rng.integers(0, len(data.names), len(data.names))
        sub = FamilyData(data.names, data.matrix[indexes], data.sizes, data.examples, data.features)
        result = pca(sub)
        ratios.append(result["variance"][:top].tolist())
        # Any reordered component in the leading subspace can match the reference.
        alignments.append([
            float(max(abs(ref["loadings"][j] @ result["loadings"][k]) for k in range(top)))
            for j in range(top)
        ])
    return {
        "repeats": repeats,
        "seed": seed,
        "pc_variance_10_50_90_percentiles": np.percentile(ratios, (10, 50, 90), axis=0).T.tolist(),
        "max_top3_loading_alignment_median": np.median(alignments, axis=0).tolist(),
        "note": "Resampling of hand-coded families. Do not interpret as inferential confidence limits.",
    }


def nmf(data: FamilyData, *, factors: int = 7, seed: int = 17, steps: int = 1500) -> dict[str, Any]:
    """Factor the nonnegative profiles with multiplicative updates."""
    if factors < 1 or factors > min(data.matrix.shape):
        raise InputError(f"NMF factors must be between 1 and {min(data.matrix.shape)}")
    X = data.matrix
    rng = np.random.default_rng(seed)
    W = rng.uniform(0.1, 1.0, size=(len(data.names), factors))
    H = rng.uniform(0.1, 1.0, size=(factors, len(data.features)))
    tiny = 1e-12
    for _ in range(steps):
        H *= (W.T @ X) / (W.T @ W @ H + tiny)
        W *= (X @ H.T) / (W @ H @ H.T + tiny)
    scale = H.sum(axis=1)
    safe_scale = np.where(scale > tiny, scale, 1.0)
    H = H / safe_scale[:, None]
    W = W * safe_scale[None, :]
    residual = float(np.linalg.norm(X - W @ H))
    return {
        "factors": factors,
        "seed": seed,
        "steps": steps,
        "W": W,
        "H": H,
        "residual_frobenius": residual,
        "relative_residual": residual / max(float(np.linalg.norm(X)), tiny),
    }


def _cosine(a: np.ndarray, b: np.ndarray) -> float:
    norm = float(np.linalg.norm(a) * np.linalg.norm(b))
    if norm == 0:
        return 0.0
    return float(np.clip(np.dot(a, b) / norm, -1, 1))


def nearest(data: FamilyData, family: str, *, limit: int = 8) -> list[dict[str, Any]]:
    ix = data.index(family)
    query = data.matrix[ix]
    results = [
        {"family": name, "similarity": _cosine(query, data.matrix[j]), "example": data.examples[name][0]}
        for j, name in enumerate(data.names) if j != ix
    ]
    return sorted(results, key=lambda x: (-x["similarity"], x["family"]))[:limit]


def fusions(data: FamilyData, *, anchor: str | None = None, limit: int = 12) -> list[dict[str, Any]]:
    """Rank family combinations that share a base but also add distinct motifs."""
    if anchor is not None:
        data.index(anchor)
    proposals: list[dict[str, Any]] = []
    for i, first in enumerate(data.names):
        for j in range(i + 1, len(data.names)):
            second = data.names[j]
            if anchor is not None and anchor not in (first, second):
                continue
            a, b = data.matrix[i], data.matrix[j]
            common = np.minimum(a, b)
            union = np.maximum(a, b)
            shared = float(common.sum())
            distinct_a = float(np.maximum(a - b, 0).sum())
            distinct_b = float(np.maximum(b - a, 0).sum())
            ratio = shared / max(float(union.sum()), 1e-12)
            # An even blend is not a recommendation. It is an exploration priority.
            balance = min(distinct_a, distinct_b) / max(distinct_a, distinct_b, 1e-12)
            mix_score = 4 * ratio * (1 - ratio) * balance
            shared_features = sorted(zip(data.features, common), key=lambda x: (-x[1], x[0]))
            a_features = sorted(zip(data.features, np.maximum(a - b, 0)), key=lambda x: (-x[1], x[0]))
            b_features = sorted(zip(data.features, np.maximum(b - a, 0)), key=lambda x: (-x[1], x[0]))
            proposals.append({
                "first": first,
                "second": second,
                "score": float(mix_score),
                "similarity": _cosine(a, b),
                "shared": [n for n, v in shared_features if v > 1e-9][:3],
                "first_adds": [n for n, v in a_features if v > 1e-9][:3],
                "second_adds": [n for n, v in b_features if v > 1e-9][:3],
                "examples": [data.examples[first][0], data.examples[second][0]],
            })
    return sorted(proposals, key=lambda x: (-x["score"], x["first"], x["second"]))[:limit]


def scenario_metrics(data: FamilyData, scenario: dict[str, Any]) -> dict[str, Any]:
    profiles = {name: data.matrix[i] for i, name in enumerate(data.names)}
    assigned: Counter[str] = Counter()
    worlds: list[dict[str, Any]] = []
    centroids = []
    all_within = []
    for world in scenario["worlds"]:
        active = [fam for fam in world["families"] if fam in profiles]
        for family in active:
            assigned[family] += 1
        vectors = np.array([profiles[f] for f in active]) if active else np.empty((0, len(data.features)))
        cohesion_pairs = [
            _cosine(vectors[i], vectors[j])
            for i in range(len(vectors)) for j in range(i + 1, len(vectors))
        ]
        all_within.extend(cohesion_pairs)
        if len(vectors):
            centroids.append((world["id"], vectors.mean(axis=0)))
        worlds.append({
            "id": world["id"], "label": world["label"], "n_families": len(active),
            "active_families": active,
            "within_cosine": float(np.mean(cohesion_pairs)) if cohesion_pairs else None,
        })
    between = [
        1 - _cosine(a, b)
        for i, (_, a) in enumerate(centroids) for _, b in centroids[i + 1:]
    ]
    included = set(assigned)
    unassigned = sorted(set(profiles) - included)
    return {
        "id": scenario["id"],
        "world_count": len(scenario["worlds"]),
        "family_coverage": len(included) / len(profiles),
        "families_covered": len(included),
        "families_total": len(profiles),
        "unassigned_families": unassigned,
        "shared_families": sorted(name for name, count in assigned.items() if count > 1),
        "mean_within_cosine": float(np.mean(all_within)) if all_within else None,
        "mean_between_cosine_distance": float(np.mean(between)) if between else None,
        "worlds": worlds,
    }


def placements(data: FamilyData, scenario: dict[str, Any], *, world_id: str | None = None,
               limit: int = 12, only_unassigned: bool = False) -> list[dict[str, Any]]:
    """Find possible theme transfers or additions. These do not change world assignments."""
    profiles = {name: data.matrix[i] for i, name in enumerate(data.names)}
    assigned: dict[str, list[str]] = defaultdict(list)
    for world in scenario["worlds"]:
        for family in world["families"]:
            assigned[family].append(world["id"])
    items = []
    valid_ids = {w["id"] for w in scenario["worlds"]}
    if world_id is not None and world_id not in valid_ids:
        raise InputError(f"Unknown world id {world_id!r}; choose {sorted(valid_ids)}")
    for world in scenario["worlds"]:
        if world_id is not None and world_id != world["id"]:
            continue
        if world["id"] == "math":
            # Preserve Math as a distinct world of formal mathematics.
            continue
        included = [profiles[f] for f in world["families"] if f in profiles]
        if not included:
            continue
        centroid = np.mean(included, axis=0)
        for family, vec in profiles.items():
            if family in world["families"] or family == "Formal mathematics":
                continue
            from_worlds = assigned.get(family, [])
            if only_unassigned and from_worlds:
                continue
            items.append({
                "world": world["label"], "world_id": world["id"], "family": family,
                "similarity": _cosine(centroid, vec),
                "assigned_elsewhere": from_worlds,
            })
    return sorted(items, key=lambda x: (-x["similarity"], x["world"], x["family"]))[:limit]


def summarize_pca(data: FamilyData, result: dict[str, Any], *, count: int = 4) -> list[dict[str, Any]]:
    rows = []
    for pc in range(min(count, len(result["variance"]))):
        vec = result["loadings"][pc]
        pos = sorted(zip(data.features, vec), key=lambda x: (-x[1], x[0]))
        neg = sorted(zip(data.features, vec), key=lambda x: (x[1], x[0]))
        rows.append({
            "axis": f"PC{pc + 1}", "explained_fraction": float(result["variance"][pc]),
            "positive": [(name, float(v)) for name, v in pos[:4]],
            "negative": [(name, float(v)) for name, v in neg[:4]],
        })
    return rows


def scatter_svg(data: FamilyData, result: dict[str, Any]) -> str:
    """Draw an SVG plot without requiring a graphics backend."""
    width, height = 1280, 900
    left, right, top, bottom = 110, 120, 85, 95
    points = result["scores"][:, :2]
    xmin, ymin = np.min(points, axis=0)
    xmax, ymax = np.max(points, axis=0)
    dx = max(float(xmax - xmin), 1e-9)
    dy = max(float(ymax - ymin), 1e-9)
    padx, pady = 0.12 * dx, 0.12 * dy
    xmin -= padx; xmax += padx; ymin -= pady; ymax += pady
    def xpix(val: float) -> float:
        return left + (val - xmin) * (width - left - right) / (xmax - xmin)
    def ypix(val: float) -> float:
        return height - bottom - (val - ymin) * (height - top - bottom) / (ymax - ymin)
    s = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" role="img" aria-label="Principal component projection of hand-coded Ambition themes">',
        '<rect width="100%" height="100%" fill="#fbfaf9"/>',
        '<style>text {font-family: DejaVu Sans, Arial, sans-serif;} .label {font-size: 13px; fill:#263344;} .axis {font-size:17px; fill:#2e3e52;} .caption {font-size:15px;fill:#5a6674;}</style>',
        '<text x="54" y="47" font-size="27" font-weight="700" fill="#243449">Ambition concept families · PCA</text>',
        '<text x="54" y="73" class="caption">Distances represent manually coded thematic profiles, not actual travel connections.</text>',
    ]
    if xmin < 0 < xmax:
        zero_x=xpix(0)
        s.append(f'<line x1="{zero_x:.1f}" x2="{zero_x:.1f}" y1="{top}" y2="{height-bottom}" stroke="#b1bdc7" stroke-dasharray="5,5"/>')
    if ymin < 0 < ymax:
        zero_y=ypix(0)
        s.append(f'<line x1="{left}" x2="{width-right}" y1="{zero_y:.1f}" y2="{zero_y:.1f}" stroke="#b1bdc7" stroke-dasharray="5,5"/>')
    s.append(f'<rect x="{left}" y="{top}" width="{width-left-right}" height="{height-top-bottom}" fill="none" stroke="#8996a3"/>')
    # Use a simple collision guard. Full names remain available in SVG tooltips.
    used: list[tuple[float, float, float, float]]=[]
    palette=("#225e91", "#9a4973", "#14766a", "#9b6419")
    for i, name in enumerate(data.names):
        px,py=xpix(float(points[i,0])), ypix(float(points[i,1]))
        color=palette[i%len(palette)]
        s.append(f'<circle cx="{px:.1f}" cy="{py:.1f}" r="6.0" fill="{color}" stroke="white" stroke-width="1.5"><title>{escape_xml(name)}: {data.sizes[name]} concepts</title></circle>')
        # Pick a free one-line label position. If all candidates collide, the tooltip is still present.
        short = name if len(name)<=25 else name[:23]+'…'
        approx_w = len(short)*7.3
        options=((10,-10),(10,15),(-approx_w-10,-10),(-approx_w-10,15),(10,-25),(-approx_w-10,30))
        for ox,oy in options:
            xx,yy=px+ox,py+oy
            rect=(xx-2,yy-14,xx+approx_w+2,yy+3)
            if rect[0]<left or rect[2]>width-right or rect[1]<top or rect[3]>height-bottom:
                continue
            if any(not (rect[2]<r[0] or rect[0]>r[2] or rect[3]<r[1] or rect[1]>r[3]) for r in used):
                continue
            used.append(rect)
            s.append(f'<text x="{xx:.1f}" y="{yy:.1f}" class="label">{escape_xml(short)}</text>')
            break
    vx=float(result["variance"][0])*100
    vy=float(result["variance"][1])*100
    s.extend([
        f'<text x="{(left+width-right)/2:.1f}" y="{height-36}" text-anchor="middle" class="axis">PC1 · {vx:.1f}% of coded variance</text>',
        f'<text transform="translate(34 {(top+height-bottom)/2:.1f}) rotate(-90)" text-anchor="middle" class="axis">PC2 · {vy:.1f}% of coded variance</text>',
        '</svg>',
    ])
    return '\n'.join(s)+'\n'


def _percent_or_na(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.3f}"


def compare_markdown(metrics: list[dict[str, Any]]) -> str:
    s=["# Candidate world partitions", "", "These scores describe a theme coding, not geography, gameplay quality, or canon.", "",
       "| Scenario | Worlds | Coverage | Within-world cosine | Between-world distance | Shared families | Unassigned |",
       "|---|---:|---:|---:|---:|---:|---:|"]
    for row in metrics:
        s.append(f"| {row['id']} | {row['world_count']} | {row['family_coverage']:.1%} | "
                 f"{_percent_or_na(row['mean_within_cosine'])} | {_percent_or_na(row['mean_between_cosine_distance'])} | "
                 f"{len(row['shared_families'])} | {len(row['unassigned_families'])} |")
    s.extend(["", "Higher cohesion and higher separation can both be useful, but a high value does not establish a good world design.",
              "An assignment shared by two worlds can represent exchange or overlapping cultures. It is not always a defect.", ""])
    for row in metrics:
        s.extend([f"## {row['id']}","",f"Unassigned model families: {', '.join(row['unassigned_families']) or '(none)'}.", "",
                  "| World | Families | Within-world cosine |", "|---|---:|---:|"])
        for world in row["worlds"]:
            s.append(f"| {world['label']} | {world['n_families']} | {_percent_or_na(world['within_cosine'])} |")
        s.append("")
    return "\n".join(s)


def fusion_markdown(rows: list[dict[str, Any]]) -> str:
    s=["# Experimental family mixtures", "", "These are combination prompts. The score measures an overlap/distinctness heuristic only.", ""]
    for i, r in enumerate(rows,1):
        s.extend([f"## {i}. {r['first']} + {r['second']} (score {r['score']:.3f})", "",
                  f"Shared themes: {', '.join(r['shared']) or '(none)'}.",
                  f"First adds: {', '.join(r['first_adds']) or '(none)'}.",
                  f"Second adds: {', '.join(r['second_adds']) or '(none)'}.",
                  f"Source examples: {r['examples'][0]}; {r['examples'][1]}.",
                  "Question: What physical location, daily life, and traversal mechanic would make this combination concrete?", ""])
    return "\n".join(s)


def placement_markdown(scenario_id: str, rows: list[dict[str, Any]]) -> str:
    lines = [f"# Possible theme transfers in {scenario_id}", "",
             "These placements are proximity prompts. They are not automatic world changes.", "",
             "| Similarity | Theme family | Candidate world | Existing assignments |",
             "|---:|---|---|---|"]
    for row in rows:
        existing = ", ".join(row["assigned_elsewhere"]) or "unassigned"
        lines.append(f"| {row['similarity']:.3f} | {row['family']} | {row['world']} | {existing} |")
    lines.extend(["", "A good transfer still needs a concrete geography, inhabited places, and a gameplay reason.", ""])
    return "\n".join(lines)


def scope_sensitivity(catalog: dict[str, Any], reference: dict[str, Any]) -> dict[str, Any]:
    """Compare the first three directions under different source filters."""
    ref = reference["loadings"]
    result: dict[str, Any] = {}
    top = min(3, len(ref))
    for scope in ("original", "core", "expanded", "all"):
        data = family_data(catalog, scope)
        fit = pca(data)
        comparison = [
            float(max(abs(ref[i] @ fit["loadings"][j]) for j in range(top)))
            for i in range(top)
        ]
        result[scope] = {
            "families": len(data.names),
            "concepts": sum(data.sizes.values()),
            "variance_ratio_top3": fit["variance"][:top].tolist(),
            "loading_alignment_to_selected_scope_top3": comparison,
        }
    return result


def report_markdown(data: FamilyData, results: dict[str, Any], *, scope: str, scenario: str,
                    origin_counts: Counter[str], boot: dict[str, Any], sensitivity: dict[str, Any]) -> str:
    pcs = summarize_pca(data, results["pca"])
    s=["# Ambition world concept analysis", "", "**State:** exploratory authoring report, not approved world topology.", "",
       f"Scope: `{scope}`. {len(data.names)} families, {sum(data.sizes.values())} included concepts.",
       f"Origin counts in the full catalog: {', '.join(f'{k}={v}' for k,v in sorted(origin_counts.items()))}.", "",
       "## PCA", "",
       "PCA uses centered, equally weighted family feature profiles. It does not derive a canonical world count.", "",
       "| Axis | Variance | Positive loadings | Negative loadings |", "|---|---:|---|---|"]
    for r in pcs:
        s.append(f"| {r['axis']} | {r['explained_fraction']:.1%} | "
                 f"{', '.join(n for n,v in r['positive'] if v>0) or '(none)'} | "
                 f"{', '.join(n for n,v in r['negative'] if v<0) or '(none)'} |")
    s.extend(["", "## Nonnegative factorization", "",
              f"The NMF fit has {results['nmf']['factors']} factors and relative reconstruction residual "
              f"{results['nmf']['relative_residual']:.3f}. Factors can rotate or change with the seed and rank.", "",
              "| Factor | Strong features | High-weight families |", "|---|---|---|"])
    W,H=results["nmf"]["W"],results["nmf"]["H"]
    for j in range(H.shape[0]):
        f_order=np.argsort(-H[j])[:4]
        w_order=np.argsort(-W[:,j])[:4]
        s.append(f"| F{j+1} | {', '.join(data.features[k] for k in f_order)} | "
                 f"{', '.join(data.names[k] for k in w_order)} |")
    s.extend(["", "## Family bootstrap sensitivity", "",
              f"{boot['repeats']} seeded family resamples. Resampling is descriptive only and does not fix subjective annotation.", ""])
    if boot["repeats"]:
        for j,(lo,md,hi) in enumerate(boot["pc_variance_10_50_90_percentiles"],1):
            s.append(f"- PC{j} explained variance, resample P10/P50/P90: {lo:.1%} / {md:.1%} / {hi:.1%}.")
        s.append("- Median maximum top-three loading alignments: " + ", ".join(f"{v:.3f}" for v in boot["max_top3_loading_alignment_median"]) + ".")
    s.extend(["", "## Sensitivity to source categories", "",
              "This compares directions in the selected scope to directions in each other source filter.",
              "A high alignment means similar loading directions; it is not an endorsement score.", "",
              "| Scope | Concepts | Families | Top-three loading alignments |",
              "|---|---:|---:|---|"])
    for name, check in sensitivity.items():
        align = ", ".join(f"{v:.3f}" for v in check["loading_alignment_to_selected_scope_top3"])
        s.append(f"| {name} | {check['concepts']} | {check['families']} | {align} |")
    s.extend(["", "## Candidate dimensions", "", f"Scenario for placement suggestions: `{scenario}`.",
              "", "See `world_scenarios.md` for descriptive comparisons. The program never alters gate addresses or world definitions.",
              "", "## Limits", "",
              "- The feature labels are manual judgements. Their correlations depend on the tags and chosen families.",
              "- Concepts that share a family have equal weight inside that family. Each family has equal weight in PCA.",
              "- The full catalog includes unendorsed assistant proposals. Use `core` or `expanded` to exclude those.",
              "- Source scopes in the catalog are research leads, not verified quotations for each item.",
              "- No score measures physical geography, level design, story quality, or whether a dimension exists.",
              "- The protected Math world keeps its own identity. No new philosophy or between-world destination is implied.", ""])
    return "\n".join(s)


def write_analysis(path: Path, data: FamilyData, catalog: dict[str, Any], scenarios: dict[str, Any], *,
                   scope: str, scenario_id: str, factors: int, seed: int, bootstrap: int) -> list[Path]:
    chosen = next((s for s in scenarios["scenarios"] if s["id"] == scenario_id), None)
    if chosen is None:
        raise InputError(f"Unknown scenario {scenario_id!r}")
    components = pca(data)
    fac = nmf(data, factors=factors, seed=seed)
    boot = bootstrap_pca(data, repeats=bootstrap, seed=seed)
    evaluations = [scenario_metrics(data, s) for s in scenarios["scenarios"]]
    mixes = fusions(data, limit=15)
    suggested = placements(data, chosen, limit=20)
    sensitivity = scope_sensitivity(catalog, components)
    payload = {
        "schema_version": 1,
        "scope": scope,
        "scenario": scenario_id,
        "method": "Hand-coded binary tags -> family mean profiles -> centered SVD and seeded multiplicative-update NMF",
        "family_names": data.names,
        "feature_names": data.features,
        "family_sizes": data.sizes,
        "pca": {"variance_ratio": components["variance"].tolist(),
                "loadings": components["loadings"].tolist(),
                "scores": components["scores"].tolist()},
        "nmf": {"factors": factors, "seed": seed, "relative_residual": fac["relative_residual"],
                "factor_feature_weights": fac["H"].tolist(), "family_factor_weights": fac["W"].tolist()},
        "bootstrap": boot,
        "scope_sensitivity": sensitivity,
        "scenario_metrics": evaluations,
        "fusion_ideas": mixes,
        "candidate_placements": suggested,
    }
    path.mkdir(parents=True, exist_ok=True)
    results = {"pca":components,"nmf":fac}
    contents = {
        "analysis.json": json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        "report.md": report_markdown(data, results, scope=scope, scenario=scenario_id,
                                     origin_counts=Counter(r["origin"] for r in catalog["records"]),
                                     boot=boot,sensitivity=sensitivity),
        "world_scenarios.md": compare_markdown(evaluations),
        "fusions.md": fusion_markdown(mixes),
        "placements.md": placement_markdown(scenario_id, suggested),
        "pca.svg": scatter_svg(data, components),
    }
    outputs=[]
    for name,content in contents.items():
        target=path / name
        target.write_text(content,encoding="utf-8")
        outputs.append(target)
    target=path / "family_coords.csv"
    with target.open("w", newline="", encoding="utf-8") as stream:
        writer=csv.writer(stream)
        writer.writerow(["family","concept_count","pc1","pc2","pc3",*data.features])
        for i,name in enumerate(data.names):
            writer.writerow([name,data.sizes[name],
                             *(f"{components['scores'][i,j]:.6f}" for j in range(3)),
                             *(f"{x:.6f}" for x in data.matrix[i])])
    outputs.append(target)
    return outputs


def main(argv: list[str] | None = None) -> int:
    parser=argparse.ArgumentParser(description=__doc__,formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument("--catalog",type=Path,default=DEFAULT_CATALOG)
    parser.add_argument("--scenarios",type=Path,default=DEFAULT_SCENARIOS)
    parser.add_argument("--supplement",type=Path,action="append",default=[],help="Extra JSON records, without changing the base catalog")
    parser.add_argument("--scope",choices=sorted(SCOPES),default="core")
    cmd=parser.add_subparsers(dest="cmd",required=True)
    cmd.add_parser("validate",help="Check catalogs and scenarios")
    ana=cmd.add_parser("analyze",help="Write a complete offline analysis report")
    ana.add_argument("--out-dir",type=Path,required=True)
    ana.add_argument("--scenario",default="consolidated")
    ana.add_argument("--factors",type=int,default=7)
    ana.add_argument("--seed",type=int,default=17)
    ana.add_argument("--bootstrap",type=int,default=80)
    compare=cmd.add_parser("compare",help="Compare proposed dimension groupings")
    compare.add_argument("--scenario",default=None,help="Optional scenario id")
    near=cmd.add_parser("neighbors",help="Find nearby theme families")
    near.add_argument("family")
    near.add_argument("--limit",type=int,default=8)
    fuse=cmd.add_parser("fuse",help="Find complementary families for a new world or district")
    fuse.add_argument("--anchor",default=None,help="Only mixtures containing this family")
    fuse.add_argument("--limit",type=int,default=12)
    place=cmd.add_parser("placements",help="Explore theme transfers into existing concrete worlds")
    place.add_argument("--scenario",default="consolidated")
    place.add_argument("--world",default=None)
    place.add_argument("--limit",type=int,default=15)
    place.add_argument("--only-unassigned",action="store_true",help="Do not include families already placed in another world")
    args=parser.parse_args(argv)
    try:
        catalog=load_catalog(args.catalog,tuple(args.supplement))
        scenarios=load_scenarios(args.scenarios,catalog)
        if args.cmd == "validate":
            print(f"OK: {len(catalog['records'])} concepts, {len(catalog['features'])} features, {len(scenarios['scenarios'])} scenarios")
            return 0
        data=family_data(catalog,args.scope)
        if args.cmd=="analyze":
            paths=write_analysis(args.out_dir,data,catalog,scenarios,scope=args.scope,
                                 scenario_id=args.scenario,factors=args.factors,seed=args.seed,bootstrap=args.bootstrap)
            print(f"Analyzed {sum(data.sizes.values())} concepts in {len(data.names)} families ({args.scope}).")
            print("Wrote:\n"+"\n".join(str(p) for p in paths))
        elif args.cmd=="compare":
            cases=[s for s in scenarios["scenarios"] if args.scenario in (None,s["id"])]
            if not cases:
                raise InputError(f"Unknown scenario {args.scenario!r}")
            print(compare_markdown([scenario_metrics(data,s) for s in cases]))
        elif args.cmd=="neighbors":
            print(f"Nearest family profiles to {args.family} ({args.scope}):")
            for row in nearest(data,args.family,limit=args.limit):
                print(f"  {row['similarity']:.3f}  {row['family']}  [e.g. {row['example']}]")
        elif args.cmd=="fuse":
            print(fusion_markdown(fusions(data,anchor=args.anchor,limit=args.limit)))
        elif args.cmd=="placements":
            case=next((s for s in scenarios["scenarios"] if s["id"]==args.scenario),None)
            if case is None:
                raise InputError(f"Unknown scenario {args.scenario!r}")
            for row in placements(data,case,world_id=args.world,limit=args.limit,only_unassigned=args.only_unassigned):
                from_elsewhere=",".join(row["assigned_elsewhere"]) or "unassigned"
                print(f"{row['similarity']:.3f}  {row['family']} -> {row['world']}  [currently: {from_elsewhere}]")
    except InputError as exc:
        parser.exit(2,f"error: {exc}\n")
    return 0


if __name__=="__main__":
    sys.exit(main())
