#!/usr/bin/env python3
"""Collect the five evidence sources `api-growth-method.md` §2 makes mandatory.

A slice is not complete until all five are collected, and §2's whole point is
that they are gathered **before** the next slice is chosen — so the choice is
made from evidence rather than from whatever seems interesting.

This is a SCRIPT rather than a hand-written JSON file for one reason: four of the
five sources are measurements of the live tree, and a measurement transcribed by
hand is a number nobody can re-take. Re-running this after any change re-derives
them. `check_absence_contracts.py`'s own baseline comment makes the same argument
about itself.

⚠ **§2c cannot be produced by this script, and deliberately is not faked.** The
blind agent run requires a FRESH agent; §2c says an agent resumed from a session
that touched engine internals "measures its own memory", and that the result is
"falsely green in the direction that feels good". So `blind_agent_run` is read
from `docs/planning/engine/slice-evidence/blind-agent-runs/*.json` if such a record exists, and

⚠ This tree used to live at `docs/sdk/evidence/`. It was MOVED 2026-07-30,
mid-blind-run, because that is the one directory §2c's subject is told to
start from — so the measurement apparatus, including the fixed script naming
*which engine file did it open first* as the field that matters, was sitting
in the room with the agent being measured. An instrument a subject can read
is not an instrument. It is also simply the wrong home: a third party reading
`docs/sdk/` should find the SDK, not our internal scorekeeping.
otherwise reports `collected: false` with the reason. The moment a real run is
recorded there, re-running this completes the evidence file — including
`selects_slice_b`, which §2c says is the field that names the next leak and
therefore cannot be derived without it.

Usage:
    python3 scripts/collect_slice_evidence.py            # write the evidence file
    python3 scripts/collect_slice_evidence.py --print    # to stdout, write nothing
"""

from __future__ import annotations

import argparse
import glob
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_absence_contracts import (  # noqa: E402
    MODULE_ALLOWLISTS,
    allowlist_usage,
    allowlist_violations,
    cargo_binary,
)

REPO = Path(__file__).resolve().parents[1]
# NOT under `docs/sdk/` — that is the surface the §2c subject is told to start
# from, and an instrument the subject can read is not an instrument. Moved
# 2026-07-30; see the module docstring.
EVIDENCE = REPO / "docs" / "planning" / "engine" / "slice-evidence"
# Per-slice, because the induction runs to a terminal condition and every slice
# owes §2 the same five sources. A collector hardcoded to one slice makes the
# NEXT slice's evidence a copy-paste job, and a copy-pasted measurement is the
# taste-based selection §3 exists to prevent.
def evidence_path(slice_id: str) -> Path:
    return EVIDENCE / f"slice-{slice_id.lower()}-evidence.json"


def selection_path(next_slice: str) -> Path:
    return EVIDENCE / f"slice-{next_slice.lower()}-selection.json"


OUT = evidence_path("a")
SELECTION = selection_path("b")
BLIND_RUNS = EVIDENCE / "blind-agent-runs"

# The consumer this slice is measured against. Slice A is BOUNDED to the external
# fixture (campaign §A1's note), so the footprint is measured there and nowhere
# else — widening it would answer a deferred measurement question by accident.
SENTINEL = REPO / "fixtures" / "external_consumer" / "Cargo.toml"

# §A1 recorded this, measured by the instrument rather than transcribed from
# prose. The campaign and ADR 0031 both said NINETEEN while listing eighteen.
RECORDED_BASELINE = 18
# `api-prototype.md` §5's prediction, written down BEFORE A4 ran. Comparing the
# instrument against a RECORDED prediction is the only version of the exercise
# worth anything — 14 against a remembered guess of 12 would have taught nothing.
PREDICTED_AFTER_A4 = 14


def contract_diff() -> dict:
    """§2a — which forbidden paths the consumer still names, and how many times.

    Frequency is the crude cost proxy and §2a says it is usually right: seven
    uses of one module is a bigger leak than one use of another. Module
    granularity is COARSE and reports progress late (six of `asset_manager`'s
    eight uses closed and the module stays), so the per-path counts live here
    beside the module list rather than instead of it.
    """
    rows = {}
    for contract in MODULE_ALLOWLISTS:
        usage = allowlist_usage(contract, REPO)
        new, stale = allowlist_violations(contract, usage)
        still_open = sorted(set(contract["baseline"]) & set(usage))
        counts = {module: len(sites) for module, sites in sorted(usage.items())}
        # A module named from more than one FILE is worth more than its count
        # suggests (§2a): it is a rule re-derived independently.
        files = {
            module: sorted({path for path, _ in sites})
            for module, sites in sorted(usage.items())
        }
        rows[contract["id"]] = {
            "allowed_sdk_modules": sorted(contract["allowed"]),
            "baseline_open": still_open,
            "open_count": len(still_open),
            "recorded_baseline": RECORDED_BASELINE,
            "predicted_after_a4": PREDICTED_AFTER_A4,
            "prediction_confirmed": len(still_open) == PREDICTED_AFTER_A4,
            "retired_by_this_slice": sorted(
                set(str(m) for m in _RETIRED_BY_A4)
            ),
            "uses_per_module": counts,
            "files_per_module": files,
            "multi_file_modules": sorted(m for m, f in files.items() if len(f) > 1),
            "violations": {"new": new, "stale": stale},
        }
    return rows


# Recorded because the diff is a BEFORE/AFTER and the "before" no longer exists
# in the tree. These four are what A4 retired; `api-prototype.md` §5 predicted
# exactly them.
_RETIRED_BY_A4 = ("engine", "game_assets", "presentation", "windowed_host")

_LEAK = re.compile(
    r"(?P<kind>LEAK CLOSED|LEAK \(recorded[^)]*\)|recorded leak #\d+|LEAK)\s*(?P<rest>.*)"
)


def fixture_leak_log() -> dict:
    """§2b — Outlander's own comments, the highest-quality source.

    Each entry is a *sentence about what the consumer had to know*, not a symbol
    count. Collected verbatim (with file and line) rather than summarized: §2b
    says a leak closed without its finding recorded is a lesson that has to be
    relearned, and paraphrasing here would be the same loss one step later.
    """
    entries = []
    root = REPO / "fixtures" / "external_consumer"
    for path in sorted(root.rglob("*.rs")):
        for number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            text = line.strip().lstrip("/!").strip()
            match = _LEAK.search(text)
            if match:
                entries.append(
                    {
                        "file": str(path.relative_to(REPO)),
                        "line": number,
                        "kind": match.group("kind"),
                        "text": text,
                    }
                )
    closed = [e for e in entries if e["kind"].startswith("LEAK CLOSED")]
    return {
        "entries": entries,
        "total": len(entries),
        "closed": len(closed),
        "open_recorded": len(entries) - len(closed),
    }


def blind_agent_run() -> dict:
    """§2c — read from a recorded run, never synthesized.

    See the module docstring. The three required fields are `completed`,
    `first_engine_file_opened` and `elapsed_context`; `first_engine_file_opened`
    is the one §2c says matters, because it names the next leak from the
    population the API is actually for.
    """
    records = sorted(glob.glob(str(BLIND_RUNS / "*.json")))
    if not records:
        return {
            "collected": False,
            "reason": (
                "No blind-agent run recorded. §2c requires a FRESH agent: one "
                "resumed from a session that touched engine internals measures "
                "its own memory, and §2c names that the single easiest way to "
                "get a falsely green result. The session that landed slice A "
                "read the movement kernel, the sim-view seam and the render "
                "cluster, so it is exactly the disqualified population and did "
                "not self-report a baseline. Drop a record in "
                "docs/planning/engine/slice-evidence/blind-agent-runs/ and re-run this script."
            ),
            "required_fields": [
                "completed",
                "first_engine_file_opened",
                "elapsed_context",
            ],
        }
    latest = json.loads(Path(records[-1]).read_text(encoding="utf-8"))
    missing = sorted(
        {"completed", "first_engine_file_opened", "elapsed_context"} - set(latest)
    )
    return {
        "collected": not missing,
        "record": str(Path(records[-1]).relative_to(REPO)),
        "missing_fields": missing,
        "run": latest,
    }


# §2d — ADR 0032's list. Each names a compensating mechanism that should become
# unnecessary, and §2d is emphatic: a criterion that did NOT become deletable is
# the most valuable single signal the method produces, because it means a seam was
# added BESIDE the old mechanism instead of taking ownership from it.
#
# Slice A is bounded to host COMPOSITION, so most of these are content-model
# criteria that slice A never touched. Recording them as "not yet in scope" is
# different from recording them as "still not deletable", and conflating the two
# would manufacture the alarming signal §2d wants to be rare and meaningful.
# ADR 0032's six deletion criteria, each with the QUESTION slice A can actually
# answer about it and the answer slice A produced.
#
# ⚠ This used to be `(name, domain, in_scope)` with `became_deletable = in_scope`.
# That is tautological: an in-scope row could never be false, so
# `in_scope_but_not_deletable` was empty BY CONSTRUCTION — and §2d says that list
# is "the most valuable single signal this method produces". A classifier that
# cannot emit the valuable signal is not a classifier, it is a restatement of its
# own input. Each row now carries its own verdict and its own reason.
_DELETION_CRITERIA = [
    {
        "criterion": "prestartup-character-preparation-backstop",
        "domain": "content",
        "in_scope_for_slice_a": False,
        "became_deletable": False,
        "evidence": (
            "Untouched, and NOT slice A's to touch. The barrier is process-global: "
            "Mary-O, Sanic, the versus fighters and the robot lineage all still "
            "stage through it, so an Outlander-only slice cannot make it "
            "deletable. This is the criterion whose misplacement forced the "
            "campaign's A-D split."
        ),
    },
    {
        "criterion": "provider-plugin-ordering-decides-content-completeness",
        "domain": "content",
        "in_scope_for_slice_a": False,
        "became_deletable": False,
        "evidence": (
            "⚠ RECLASSIFIED 2026-07-30. Previously marked composition/deletable on "
            "the strength of the green hand-ordering contract. That contract is "
            "about HOST composition order; this criterion is about CONTENT "
            "completeness, which is still decided by `Plugin::build` running "
            "`install_outlander_content` and by the finish/PreStartup apparatus "
            "slice A never went near. `define` accumulating into a draft is the "
            "shape that will close it, but the draft holds routes and a room in "
            "slice A, not content."
        ),
    },
    {
        "criterion": "repeated-app-finish-can-republish-prepared-content",
        "domain": "content",
        "in_scope_for_slice_a": False,
        "became_deletable": False,
        "evidence": "content-model criterion; the idempotence flag is untouched (slice B+)",
    },
    {
        "criterion": "headless-and-visible-share-a-prepared-content-fingerprint",
        "domain": "composition",
        "in_scope_for_slice_a": True,
        "became_deletable": False,
        "evidence": (
            "⚠ MOVED AWAY from deletable, and this is the sharpest thing slice A "
            "measured. `PlatformerApp` gained `with_game_assets`, OFF by default "
            "on headless and always on for windowed — so the two faces now "
            "consume different prepared art unless the consumer says otherwise. "
            "That was a deliberate correction (the first draft installed assets "
            "on both faces citing THIS criterion, and the fixture's rollback "
            "parity test caught it: under GGRS the extra asset frames are frames "
            "the sim does not advance, and the two hosts landed twelve `update()` "
            "calls apart). Preparing art is also not free — boot decode was "
            "measured at 627MP/2.5GB. So the knob is right and the criterion is "
            "further away, which is a real finding rather than a regression: the "
            "criterion wants ONE FINGERPRINT, and slice A has no content "
            "fingerprint to share. Slice B owns closing it, and must close it "
            "without collapsing the policy back into the face."
        ),
    },
    {
        "criterion": "sanic-standalone-and-embedded-agree-on-identities",
        "domain": "content",
        "in_scope_for_slice_a": False,
        "became_deletable": False,
        "evidence": "content-model criterion; no module-qualified namespaces exist yet (slice B)",
    },
    {
        "criterion": "a-runtime-character-consumer-reads-a-fallback-catalog",
        "domain": "content",
        "in_scope_for_slice_a": False,
        "became_deletable": False,
        "evidence": "content-model criterion; the fallback catalog is untouched (slice B)",
    },
]


def deletion_criteria() -> dict:
    """§2d — ADR 0032's criteria, each with its own verdict.

    Slice A moved NONE of them, and that is the honest and expected result: all
    six are content or capability criteria, and slice A was bounded to host
    composition. A slice that reported progress here would have been reporting
    that it exceeded its own scope.

    §2d: *"A criterion that did NOT become deletable is the most valuable single
    signal this method produces. It means a seam was added beside the old
    mechanism rather than taking ownership from it."* That reading applies to
    IN-SCOPE rows. The one in-scope row here did not merely fail to move — it
    moved the wrong way, and its `evidence` says why in full.
    """
    rows = [dict(row) for row in _DELETION_CRITERIA]

    # Non-vacuity: the verdict must not be a restatement of the scope, which is
    # the bug this function had. If every in-scope row is deletable and every
    # out-of-scope row is not, the column is carrying no information.
    scope = [r["in_scope_for_slice_a"] for r in rows]
    verdict = [r["became_deletable"] for r in rows]
    tautological = scope == verdict

    unresolved = [
        r["criterion"]
        for r in rows
        if r["in_scope_for_slice_a"] and not r["became_deletable"]
    ]
    return {
        "criteria": rows,
        "in_scope": sum(scope),
        "became_deletable": sum(verdict),
        # §2d: investigate this before anything else on the list.
        "in_scope_but_not_deletable": unresolved,
        "verdict_column_is_tautological": tautological,
    }


def _cargo_json(args: list[str]) -> dict | None:
    try:
        out = subprocess.run(
            [cargo_binary(), *args],
            cwd=str(REPO),
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    return json.loads(out)


# Capabilities a MOVEMENT-ONLY game plainly never asked for. Named explicitly
# rather than derived by a heuristic, because the consumer-matrix row asks a
# specific question — "does a small game link menus, persistence, audio,
# bosses?" — and a fuzzy match would let the answer drift.
_UNASKED_BY_A_MOVEMENT_ONLY_GAME = (
    "menu", "persist", "audio", "sfx", "boss", "ldtk", "cutscene", "dialog",
    "inventory", "portal", "settings", "ui_nav", "touch", "encounter", "items",
    "projectile", "vfx", "render",
)


def sentinel_closures() -> dict:
    """Per-sentinel `ambition_*` closure, from the WORKSPACE GRAPH.

    ⚠ `cargo tree` on an out-of-workspace consumer hangs here — it re-resolves
    682 packages against a git-patched dependency — so the closure is taken from
    the workspace manifest graph instead. Both sentinels declare `ambition` and
    nothing else from this workspace, so the facade's reachable set IS their
    closure.

    It is an UPPER BOUND: the graph ignores feature gating, so a capability that
    would drop out under `default-features = false` still counts here. Stated
    rather than papered over — §4 authorises a carve when a footprint cannot be
    reduced WITHOUT MOVING CODE, and "we never tried features" is not that.
    """
    import sys as _sys

    _sys.path.insert(0, str(Path(__file__).resolve().parent))
    from check_absence_contracts import reachable, workspace_graph

    graph = workspace_graph(REPO)
    closure = sorted(
        {c for c in reachable(graph, "ambition") if c.startswith("ambition")}
        | {"ambition"}
    )
    unasked = [
        c for c in closure if any(k in c for k in _UNASKED_BY_A_MOVEMENT_ONLY_GAME)
    ]
    return {
        "outlander": {
            "manifest": "fixtures/external_consumer/Cargo.toml",
            "declared_dependencies": ["ambition", "bevy"],
            "ambition_crates_linked": len(closure),
        },
        "minimal_game": {
            "manifest": "fixtures/minimal_game/Cargo.toml",
            "declared_dependencies": ["ambition"],
            "ambition_crates_linked": len(closure),
            "linked_but_never_asked_for": unasked,
            "unwanted_count": len(unasked),
            "verdict": (
                "The consumer-matrix question — does a small game link menus, "
                "persistence, audio, bosses? — is answered YES. A movement-only "
                "game with one room, one walker and no combat links "
                f"{len(closure)} Ambition crates, {len(unasked)} of which it "
                "never asked for: menus, persistence, cutscenes, encounters, "
                "inventory UI, portals, projectiles, LDtk, settings menus and "
                "touch input among them."
            ),
            "authorises_a_carve": (
                "NOT YET. §4 authorises an internal carve when a sentinel "
                "consumer's footprint cannot be reduced WITHOUT moving code "
                "between crates. This measurement shows the footprint is large; "
                "it does not show it is irreducible, because feature-gating has "
                "not been attempted. Trying that is the cheap experiment that "
                "either closes this or converts it into the carve argument — and "
                "§4 warns explicitly not to let a single leak authorise a full "
                "decomposition."
            ),
        },
    }


def capability_footprint() -> dict:
    """§2e — the evidence a clean facade can hide.

    "A perfectly semantic API could still force a movement-only game to compile
    and link menus, persistence, audio, LDtk, bosses and every unrelated gameplay
    domain." No consumer names a forbidden path; the footprint is still wrong.

    Measured as the sentinel consumer's TRANSITIVE `ambition_*` closure, because
    that is the quantity a capability system would have to be able to shrink and
    the one a module allowlist says nothing about.
    """
    metadata = _cargo_json(
        [
            "metadata",
            "--manifest-path",
            str(SENTINEL),
            "--format-version",
            "1",
            "--no-deps",
        ]
    )
    full = _cargo_json(
        ["metadata", "--manifest-path", str(SENTINEL), "--format-version", "1"]
    )
    if full is None or metadata is None:
        return {"collected": False, "reason": "cargo metadata unavailable"}
    ambition_crates = sorted(
        p["name"] for p in full["packages"] if p["name"].startswith("ambition")
    )
    # Direct edges per Ambition crate — §2e names `ambition_actors`' 27 direct
    # `ambition_*` dependencies as the shape of the problem, so the number is
    # measured rather than quoted.
    direct = Counter()
    for package in full["packages"]:
        if not package["name"].startswith("ambition"):
            continue
        direct[package["name"]] = sum(
            1 for d in package["dependencies"] if d["name"].startswith("ambition")
        )
    declared = [
        d["name"]
        for p in metadata["packages"]
        for d in p["dependencies"]
        if d["kind"] is None
    ]
    return {
        "collected": True,
        "sentinel": str(SENTINEL.relative_to(REPO)),
        "declared_dependencies": sorted(set(declared)),
        "transitive_ambition_closure": ambition_crates,
        "transitive_ambition_count": len(ambition_crates),
        "direct_ambition_edges_per_crate": dict(direct.most_common()),
        "widest_fan_out": direct.most_common(3),
        "note": (
            "The consumer declares two dependencies (`ambition`, `bevy`) and "
            "links the whole engine closure. A module allowlist cannot see this: "
            "slice A's ratchet went from 18 to 14 while this number did not move "
            "at all, which is precisely the blind spot §2e exists to record."
        ),
    }


def selects_slice_b(evidence: dict) -> dict:
    """§3 — the derivation, READ rather than generated.

    Ranking candidates by cost/closeable/owned is a judgment call. A script that
    produced one would be manufacturing exactly the taste-based selection §3
    exists to prevent, and it would look identical to a real derivation in the
    output file. So the decision is authored in `slice-b-selection.json` and this
    only checks that it exists and that its preconditions held.
    """
    blind = evidence["blind_agent_run"]
    if not blind.get("collected", False):
        return {
            "derived": False,
            "blocked_on": "2c",
            "reason": (
                "Four of five sources are collected; §2c is not, and it is the one "
                "that names the next leak from the population the API is for. "
                "Deriving B from the other four would choose it by taste, which "
                "the slice's own exit criteria forbid."
            ),
        }
    if not SELECTION.exists():
        return {
            "derived": False,
            "blocked_on": None,
            "reason": (
                f"All five sources are collected. Author the §3 derivation in "
                f"{SELECTION.relative_to(REPO)} — rank the candidates by "
                f"cost/closeable/owned, route them per §3b, and size per §3c."
            ),
            "first_engine_file_opened": blind.get("first_engine_file_opened"),
        }
    selection = json.loads(SELECTION.read_text(encoding="utf-8"))
    return {
        "derived": bool(selection.get("derived")),
        "selection": str(SELECTION.relative_to(REPO)),
        "slice_b": selection.get("slice_b", {}).get("name"),
        "one_leak": selection.get("slice_b", {}).get("one_leak"),
        "candidates_ranked": len(selection.get("ranked_candidates", [])),
        "first_engine_file_opened": blind.get("first_engine_file_opened"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--slice",
        default="a",
        help="which slice's evidence to collect (a, b, c, ...). Each slice owes "
        "§2 the same five sources; the selection file it writes toward is the "
        "NEXT slice's.",
    )
    parser.add_argument(
        "--print",
        action="store_true",
        dest="to_stdout",
        help="write to stdout instead of the evidence file",
    )
    args = parser.parse_args()

    slice_id = args.slice.lower()
    next_slice = chr(ord(slice_id) + 1)
    global OUT, SELECTION
    OUT = evidence_path(slice_id)
    SELECTION = selection_path(next_slice)

    evidence = {
        "slice": slice_id.upper(),
        "campaign": "docs/planning/engine/api-1.0-campaign.md",
        "method": "docs/planning/engine/api-growth-method.md",
        "generated_by": "scripts/collect_slice_evidence.py",
        "contract_diff": contract_diff(),
        "fixture_leak_log": fixture_leak_log(),
        "blind_agent_run": blind_agent_run(),
        "deletion_criteria": deletion_criteria(),
        "capability_footprint": capability_footprint(),
    }
    evidence["capability_footprint"]["sentinels"] = sentinel_closures()
    evidence[f"selects_slice_{next_slice}"] = selects_slice_b(evidence)
    rendered = json.dumps(evidence, indent=2, sort_keys=False) + "\n"
    if args.to_stdout:
        sys.stdout.write(rendered)
        return 0
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT.relative_to(REPO)}")
    contract = evidence["contract_diff"]["outlander-names-only-the-public-sdk"]
    for name, row in sorted(evidence["contract_diff"].items()):
        if name != "outlander-names-only-the-public-sdk":
            print(f"  2a {name}: {row['open_count']} open")
    print(
        f"  2a contract diff        : {contract['open_count']} open "
        f"(baseline {contract['recorded_baseline']}, predicted "
        f"{contract['predicted_after_a4']}, confirmed="
        f"{contract['prediction_confirmed']})"
    )
    print(
        f"  2b fixture leak log     : {evidence['fixture_leak_log']['total']} entries, "
        f"{evidence['fixture_leak_log']['closed']} closed"
    )
    print(
        f"  2c blind agent run      : "
        f"{'collected' if evidence['blind_agent_run']['collected'] else 'NOT COLLECTED (needs a fresh agent)'}"
    )
    print(
        f"  2d deletion criteria    : "
        f"{evidence['deletion_criteria']['became_deletable']} of "
        f"{evidence['deletion_criteria']['in_scope']} in-scope became deletable"
    )
    print(
        f"  2e capability footprint : "
        f"{evidence['capability_footprint'].get('transitive_ambition_count')} ambition "
        "crates linked from 2 declared dependencies"
    )
    print(
        f"  slice {next_slice.upper()} derived         : "
        f"{evidence[f'selects_slice_{next_slice}']['derived']}"
        + (
            f" (blocked on §{evidence[f'selects_slice_{next_slice}']['blocked_on']})"
            if evidence[f"selects_slice_{next_slice}"].get("blocked_on")
            else ""
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
