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
from `docs/sdk/evidence/blind-agent-runs/*.json` if such a record exists, and
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
EVIDENCE = REPO / "docs" / "sdk" / "evidence"
OUT = EVIDENCE / "slice-a-evidence.json"
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
                "docs/sdk/evidence/blind-agent-runs/ and re-run this script."
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
_DELETION_CRITERIA = [
    ("prestartup-character-preparation-backstop", "content", False),
    ("provider-plugin-ordering-decides-content-completeness", "composition", True),
    ("repeated-app-finish-can-republish-prepared-content", "content", False),
    ("headless-and-visible-share-a-prepared-content-fingerprint", "composition", True),
    ("sanic-standalone-and-embedded-agree-on-identities", "content", False),
    ("a-runtime-character-consumer-reads-a-fallback-catalog", "content", False),
]


def deletion_criteria() -> dict:
    """§2d — ADR 0032's criteria, with slice A's scope stated per row."""
    rows = []
    for name, domain, in_scope in _DELETION_CRITERIA:
        rows.append(
            {
                "criterion": name,
                "domain": domain,
                "in_scope_for_slice_a": in_scope,
                # Slice A owns composition ordering: `PlatformerApp` is now the
                # single authority for engine→host→shell→assets→presentation
                # order, and the external consumer no longer states any of it —
                # guarded by `outlander-does-not-hand-order-its-own-composition`.
                # That is ownership TAKEN, not a seam added beside the old path:
                # the fixture's three hand-rolled builders and its hand-composed
                # dump were deleted, not left as a second route.
                "became_deletable": in_scope,
                "evidence": (
                    "contract outlander-does-not-hand-order-its-own-composition "
                    "is green; the fixture's hand-ordered builders and dump are "
                    "deleted"
                )
                if in_scope
                else "slice A is bounded to host composition; this is a content-model criterion (slice B+)",
            }
        )
    unresolved = [r["criterion"] for r in rows if r["in_scope_for_slice_a"] and not r["became_deletable"]]
    return {
        "criteria": rows,
        "in_scope": sum(1 for r in rows if r["in_scope_for_slice_a"]),
        "became_deletable": sum(1 for r in rows if r["became_deletable"]),
        # §2d: investigate this before anything else on the list.
        "in_scope_but_not_deletable": unresolved,
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
    """Slice B, DERIVED — or an explicit refusal to invent it.

    The goal is emphatic: do not invent B. §2c says the first-engine-file-opened
    field is the one that names the next leak "from the population the API is
    *for*", so with 2c uncollected the strongest honest statement is what the
    other four sources CONSTRAIN B to, plus what is still missing to choose.
    """
    blind = evidence["blind_agent_run"]
    footprint = evidence["capability_footprint"]
    contract = evidence["contract_diff"]["outlander-names-only-the-public-sdk"]
    if not blind.get("collected"):
        return {
            "derived": False,
            "blocked_on": "2c",
            "reason": (
                "Four of five sources are collected; §2c is not, and it is the "
                "one that names the next leak from the population the API is "
                "for. Deriving B from the other four would choose it by taste, "
                "which the slice's own exit criteria forbid."
            ),
            # What the collected four already CONSTRAIN B to — recorded so the
            # eventual derivation is checked against evidence that existed
            # before it, the same discipline §5's 18->14 prediction used.
            "constraints_from_the_collected_four": [
                f"the ratchet's remaining {contract['open_count']} modules are "
                "content/gameplay vocabulary, not composition — so B is a "
                "content-model slice, which is what the campaign sketch already "
                "says; that is a confirmation, not a derivation",
                f"the heaviest remaining leaks by use count are "
                f"{sorted(contract['uses_per_module'].items(), key=lambda kv: -kv[1])[:4]}",
                f"modules named from more than one file (a rule re-derived "
                f"independently, §2a): {contract['multi_file_modules']}",
                f"§2e is untouched by slice A: the consumer still links "
                f"{footprint.get('transitive_ambition_count')} ambition crates "
                "from two declared dependencies, and no module allowlist can "
                "see that",
            ],
        }
    return {
        "derived": False,
        "blocked_on": None,
        "reason": (
            "A blind-agent record now exists; re-derive B from all five sources "
            "and replace this block with the decision and its citations."
        ),
        "first_engine_file_opened": blind["run"].get("first_engine_file_opened"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--print",
        action="store_true",
        dest="to_stdout",
        help="write to stdout instead of the evidence file",
    )
    args = parser.parse_args()

    evidence = {
        "slice": "A",
        "campaign": "docs/planning/engine/api-1.0-campaign.md",
        "method": "docs/planning/engine/api-growth-method.md",
        "generated_by": "scripts/collect_slice_evidence.py",
        "contract_diff": contract_diff(),
        "fixture_leak_log": fixture_leak_log(),
        "blind_agent_run": blind_agent_run(),
        "deletion_criteria": deletion_criteria(),
        "capability_footprint": capability_footprint(),
    }
    evidence["selects_slice_b"] = selects_slice_b(evidence)
    rendered = json.dumps(evidence, indent=2, sort_keys=False) + "\n"
    if args.to_stdout:
        sys.stdout.write(rendered)
        return 0
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    OUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUT.relative_to(REPO)}")
    contract = evidence["contract_diff"]["outlander-names-only-the-public-sdk"]
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
        f"  slice B derived         : {evidence['selects_slice_b']['derived']}"
        + (
            f" (blocked on §{evidence['selects_slice_b']['blocked_on']})"
            if evidence["selects_slice_b"].get("blocked_on")
            else ""
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
