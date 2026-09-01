"""The bundle pruner must never delete a file the ledger or summary opens.

⛔⛔ **`profiles/` IS GITIGNORED, SO A WRONG DELETION IS UNRECOVERABLE.** The
pruner exists because that directory reached 127 GB — a single
`tracy_zone_instances.csv` was 85 GB, in a bundle never ingested at all — and
because bundles that get deleted outright take their numbers with them: adding
`sim_phases_ms` on 2026-09-01 backfilled 13 of 26 ledger rows and could not
touch the other 12.

So the keep-list has to be checked against what the two consumers actually open,
not against what someone remembered. This test reads the filenames straight out
of `profile_bundle_to_history.py` and `profile_bundle_summary.py` and asserts the
pruner keeps every one.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import prune_profile_bundles as pruner  # noqa: E402

CONSUMERS = (
    REPO / "scripts/lib/profile_bundle_to_history.py",
    REPO / "scripts/lib/profile_bundle_summary.py",
)


def filenames_the_consumers_open() -> set[str]:
    names: set[str] = set()
    for path in CONSUMERS:
        for match in re.finditer(
            r'"([a-z0-9_\-]+\.(?:csv|txt|json|md|status|log))"', path.read_text()
        ):
            names.add(match.group(1))
    return names


def test_every_file_the_ledger_reads_survives_pruning():
    opened = filenames_the_consumers_open()
    assert len(opened) > 15, (
        f"only found {len(opened)} filenames in the consumers — if they stopped "
        "naming files as literals, this guard stopped guarding"
    )
    doomed = sorted(n for n in opened if pruner.classify(Path("bundle") / n))
    assert not doomed, (
        f"the pruner would delete files the ledger or summary opens: {doomed}. "
        "profiles/ is gitignored, so that data is gone for good."
    )


def test_the_big_raw_captures_are_still_dropped():
    """⛔ PREMISE GUARD. A keep-list that keeps everything passes the test above
    and frees nothing — which is how the directory reached 127 GB."""
    for name in (
        "tracy_zone_instances.csv",  # 85 GB in one run
        "tracy.trace",
        "perf.data",
        "perf.data.1",
        "bundle.tar.gz",
    ):
        assert pruner.classify(Path("bundle") / name), f"{name} should be pruned"


def test_the_aggregate_tracy_export_is_kept_not_the_per_instance_one():
    """The two differ by one word and by five orders of magnitude."""
    assert not pruner.classify(Path("b/tracy_zones.csv")), "the summary reads this"
    assert pruner.classify(Path("b/tracy_zone_instances.csv")), "and not this one"
