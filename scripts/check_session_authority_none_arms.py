#!/usr/bin/env python3
"""C07: every optional read of a session-owned authority must say what its `None` means.

⭐ **THE QUESTION IS NOT "REMOVE THE `Option<Res<_>>` PARAMETERS".** After C03
the session authorities live in known places (the session root's components, the
checkpoint/horizon baselines, the session mechanics). An `Option` read of one of
them is either a supported composition, a lifecycle moment, a view that has
nothing to show, a refusal, or a *mistake that nobody had reason to look at* — a
`None` arm no composition can reach, which only exists to make a fixture compile
and which hides the day a real composition loses the resource.

This guard makes each one say which. The table is
`docs/planning/consolidation/session-authority-none-arms.md`, one row per
function (or `SystemParam` struct) that reads an authority optionally:

    none-arm: <path>::<item> | <class> | <reason>

CLASSES (closed):

* `reduced-composition` — the reader's plugin does NOT install the authority, so
  a composition that omits the installing plugin is supported and the `None` arm
  is its behavior. (If the SAME plugin installs both, the read is required and the
  row does not belong here: it was converted.)
* `lifecycle-remainder` — absence is a moment in the session's life (before the
  first activation, after teardown, no operation outstanding), not a composition.
* `presentation` — a view system; with nothing to read there is nothing to show.
  `PRESENTATION_PREFIXES` classifies a whole render/view crate, so the render
  family does not need seventy rows that say one thing.
* `refuses` — absence is an invalid composition; the arm logs and refuses rather
  than substituting an empty value.

⛔ **A `None` ARM THAT NO CLASS FITS IS THE FINDING.** Do not stretch a class: convert
the read to required (the probe is to convert it and run the lanes — Bevy's
validation panic names the system), or fix the plugin that should install it.

WHAT THIS CHECKS: every scanned site has a row or a prefix default; no row is stale
(its item still reads the authority optionally); the class is in the vocabulary; the
reason is a sentence; and the population floor holds, so a scan that stops seeing
the tree cannot pass by being empty.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "lib"))

from rust_source import code_only  # noqa: E402
from test_paths import file_is_test_only, is_test_path, strip_test_modules  # noqa: E402

TABLE = "docs/planning/consolidation/session-authority-none-arms.md"

CLASSES = ("reduced-composition", "lifecycle-remainder", "presentation", "refuses")

# The authorities: process resources that C03 / the durable horizon / the session
# lifecycle make a single owner per session. Clocks (`SimTick`) are deliberately
# not here: whether a clock may be absent is the composition-profile question (A9),
# not an ownership one.
AUTHORITIES = frozenset(
    """
    AbandonedCheckpointOperation AcceptedCheckpointRestore ActiveConversation ActiveSessionScope
    AuthoredOccurrences BagSpendsSinceCheckpoint BaseGravity BossDefeatsSinceCheckpoint
    BossEncounterRegistry ClockState ConsumedSinceCheckpoint ControlledSubject CustodyBaseline
    GameplayElapsed GatePortalPhases ImpactHitstop MintedItemBaseline OccurrenceBaseline
    OwnedItemsBaseline PendingLifecycleCommit QuestRegistry RequestedClockScale
    RewardGrantsSinceCheckpoint SessionCheckpointOutcomes SessionMechanics StocksMatchSettled
    WorldTime WorldTimeSchedule
    """.split()
)

PRESENTATION_PREFIXES = (
    "crates/ambition_render/",
    "crates/ambition_sim_view/",
    "game/ambition_app/src/dev/",
)

MIN_SITES = 40  # anti-vacuity: a scan that finds fewer has lost the tree
MIN_REASON = 30

OPT = re.compile(
    r"Option<\s*(?:\w+::)*(?:Res|ResMut|SessionWorldRef|SessionWorldMut)\s*<\s*(?:'\w+\s*,\s*)*(?:\w+::)*(\w+)\s*>"
)
GET = re.compile(r"\.(?:get_resource|get_resource_mut)::<\s*(?:\w+::)*(\w+)\s*>")
SWC = re.compile(r"session_world_component(?:_mut)?::<\s*(?:\w+::)*(\w+)\s*>")
ITEM = re.compile(r"^\s*(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?(?:fn|struct)\s+(\w+)")
ROW = re.compile(r"^none-arm:\s*(\S+?)::(\w+)\s*\|\s*([\w-]+)\s*\|\s*(.+?)\s*$")


def scan(root: pathlib.Path) -> dict[tuple[str, str], set[str]]:
    """(path, enclosing fn/struct) -> the authorities it reads optionally."""
    found: dict[tuple[str, str], set[str]] = {}
    for top in ("crates", "game"):
        for path in sorted((root / top).rglob("*.rs")):
            rel = path.relative_to(root)
            src = path.read_text(encoding="utf-8", errors="replace")
            if is_test_path(rel) or file_is_test_only(src):
                continue
            lines = code_only(strip_test_modules(src)).split("\n")
            for number, line in enumerate(lines):
                names = {m.group(1) for m in OPT.finditer(line)}
                names |= {m.group(1) for m in GET.finditer(line)}
                names |= {m.group(1) for m in SWC.finditer(line)}
                names &= AUTHORITIES
                if not names:
                    continue
                at = number
                while at >= 0 and not ITEM.match(lines[at]):
                    at -= 1
                item = ITEM.match(lines[at]).group(1) if at >= 0 else "?"
                found.setdefault((str(rel), item), set()).update(names)
    return found


def parse_table(text: str) -> tuple[dict[tuple[str, str], tuple[str, str]], list[str]]:
    rows: dict[tuple[str, str], tuple[str, str]] = {}
    problems: list[str] = []
    for line in text.split("\n"):
        if not line.startswith("none-arm:"):
            continue
        m = ROW.match(line)
        if not m:
            problems.append(f"unparseable row: {line[:100]}")
            continue
        path, item, cls, reason = m.groups()
        if (path, item) in rows:
            problems.append(f"duplicate row: {path}::{item}")
        rows[(path, item)] = (cls, reason)
    return rows, problems


def check(found, rows, table_problems=()) -> list[str]:
    problems = list(table_problems)
    for (path, item), (cls, reason) in sorted(rows.items()):
        if cls not in CLASSES:
            problems.append(f"{path}::{item}: class `{cls}` is not one of {CLASSES}")
        if len(reason) < MIN_REASON:
            problems.append(f"{path}::{item}: the reason is not a sentence ({reason!r})")
        if (path, item) not in found:
            problems.append(f"{path}::{item}: STALE row, nothing there reads an authority optionally any more")
    for (path, item), names in sorted(found.items()):
        if (path, item) in rows:
            continue
        if path.startswith(PRESENTATION_PREFIXES):
            continue
        problems.append(
            f"{path}::{item}: reads {sorted(names)} optionally and has no `none-arm:` row — "
            f"say what its None means, or convert it to a required read"
        )
    if len(found) < MIN_SITES:
        problems.append(f"only {len(found)} sites scanned (floor {MIN_SITES}): the scan lost the tree")
    return problems


def main() -> int:
    found = scan(REPO)
    table = REPO / TABLE
    if not table.exists():
        print(f"RED: {TABLE} does not exist")
        return 1
    rows, table_problems = parse_table(table.read_text(encoding="utf-8"))
    problems = check(found, rows, table_problems)
    by_class: dict[str, int] = {}
    for cls, _ in rows.values():
        by_class[cls] = by_class.get(cls, 0) + 1
    presentation_default = sum(
        1 for key in found if key not in rows and key[0].startswith(PRESENTATION_PREFIXES)
    )
    print(
        f"session-authority None arms: {len(found)} sites, {len(rows)} rows "
        f"{dict(sorted(by_class.items()))}, +{presentation_default} presentation by prefix"
    )
    for problem in problems:
        print(f"RED: {problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
