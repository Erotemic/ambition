#!/usr/bin/env python3
"""Model the cost of Ambition's test suite: compile/link burden and volume.

This answers "how intense is our test suite in terms of extra compile time and
runtime" with a fast, deterministic *static cost model*, plus an optional mode
that measures real `cargo` timings.

Why a cost model instead of just counting tests:

  In Rust the dominant test compile cost is *linking*, not source volume. Every
  top-level `tests/*.rs` file is its own crate -> its own linked binary against
  the full dependency graph (Bevy is heavy). Each library crate with inline
  `#[cfg(test)]` code also produces one extra "lib-test" binary. So a full
  `cargo test` performs roughly:

      (top-level integration test files) + (lib crates with inline tests)

  separate link steps on top of the normal build. Consolidating many small
  `tests/*.rs` files in a heavy crate into ONE aggregator target (a single
  `tests/main.rs` with `mod foo;` submodules) collapses N links into 1 with no
  change to what is tested -- usually the single biggest available win.

This is reconnaissance, not coverage: for executable line/branch coverage use
`cargo llvm-cov`; for per-file line/test tallies use tools/test_coverage_report.sh.

Usage:
  scripts/test_suite_cost.py                 # static cost model, human report
  scripts/test_suite_cost.py --json          # same, machine-readable
  scripts/test_suite_cost.py --measure       # ALSO run `cargo test --no-run`
                                             #   (slow: full test compile+link)
  scripts/test_suite_cost.py --measure -p ambition_app   # measure one crate

The static model reads only source; it never invokes cargo unless --measure is
given. Run it from anywhere; the workspace root is located from this file.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass, field, asdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# A test target is "heavy" (slow to link) if it pulls in bevy, directly or via
# the crate under test. We approximate by scanning the test source for `bevy`.
HEAVY_MARKER = re.compile(r"\bbevy\b")
TEST_FN = re.compile(r"#\[\s*(?:tokio::)?test\s*\]|#\[\s*test\s*\]")
CFG_TEST = re.compile(r"#\[\s*cfg\s*\(\s*test\s*\)\s*\]")


def workspace_members(root: Path) -> list[Path]:
    """Return absolute paths of every workspace member declared in Cargo.toml."""
    text = (root / "Cargo.toml").read_text(encoding="utf-8", errors="replace")
    m = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    if not m:
        return []
    members = []
    for line in m.group(1).splitlines():
        line = line.strip().strip(",").strip()
        if not line or line.startswith("#"):
            continue
        members.append(root / line.strip('"'))
    return members


def count_lines(path: Path) -> int:
    try:
        return len(path.read_text(encoding="utf-8", errors="replace").splitlines())
    except OSError:
        return 0


def inline_test_loc(path: Path) -> tuple[int, int]:
    """(#cfg(test) blocks, approx LOC inside them) via brace matching."""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return 0, 0
    blocks = loc = 0
    i = 0
    n = len(lines)
    while i < n:
        if CFG_TEST.search(lines[i]):
            j = i
            depth = 0
            started = False
            while j < n:
                depth += lines[j].count("{") - lines[j].count("}")
                if "{" in lines[j]:
                    started = True
                if started and depth <= 0:
                    break
                j += 1
            blocks += 1
            loc += j - i + 1
            i = j + 1
        else:
            i += 1
    return blocks, loc


def count_test_fns(path: Path) -> int:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 0
    return len(TEST_FN.findall(text))


@dataclass
class CrateCost:
    name: str
    rel_path: str
    # Compile/link drivers:
    integration_targets: int = 0     # top-level tests/*.rs -> separate binaries
    has_lib_tests: bool = False      # inline #[cfg(test)] -> one lib-test binary
    heavy_targets: int = 0           # integration targets that reference bevy
    # Volume:
    integration_files: int = 0       # incl. nested submodules
    integration_loc: int = 0
    inline_test_loc: int = 0
    test_fns: int = 0
    integration_target_names: list[str] = field(default_factory=list)

    @property
    def link_targets(self) -> int:
        return self.integration_targets + (1 if self.has_lib_tests else 0)

    @property
    def consolidatable(self) -> int:
        """Extra link steps removable by merging tests/*.rs into one aggregator."""
        return max(0, self.integration_targets - 1)


def analyze_crate(crate_dir: Path, root: Path) -> CrateCost:
    name = crate_dir.name
    cost = CrateCost(name=name, rel_path=str(crate_dir.relative_to(root)))

    # Inline unit tests across the crate's src/ (one lib-test binary if any).
    src = crate_dir / "src"
    if src.is_dir():
        for rs in src.rglob("*.rs"):
            blocks, loc = inline_test_loc(rs)
            if blocks:
                cost.has_lib_tests = True
                cost.inline_test_loc += loc
            cost.test_fns += count_test_fns(rs)

    # Integration tests. Cargo links one binary per top-level `tests/*.rs` ONLY
    # when auto-discovery is on. A crate with `autotests = false` (e.g.
    # `ambition_app`) builds just its explicit `[[test]]` targets; the other
    # `tests/*.rs` files are `mod` submodules of one aggregate binary, not
    # separate link steps. Read the manifest so the model reflects Cargo reality.
    manifest: dict = {}
    cargo_toml = crate_dir / "Cargo.toml"
    if cargo_toml.exists():
        try:
            manifest = tomllib.loads(
                cargo_toml.read_text(encoding="utf-8", errors="replace"))
        except tomllib.TOMLDecodeError:
            manifest = {}
    autotests = manifest.get("package", {}).get("autotests", True)
    explicit_tests = manifest.get("test", [])
    if isinstance(explicit_tests, dict):  # a single `[test]` table
        explicit_tests = [explicit_tests]

    tests = crate_dir / "tests"
    if tests.is_dir():
        # Volume metrics count EVERY source file (aggregate submodules included):
        # consolidating targets removes link steps, not test code.
        for rs in tests.rglob("*.rs"):
            cost.integration_files += 1
            cost.integration_loc += count_lines(rs)
            cost.test_fns += count_test_fns(rs)

        # Real integration binaries = auto-discovered top-level files (if enabled)
        # plus explicitly-declared `[[test]]` targets.
        names: list[str] = []
        if autotests:
            names = [rs.stem for rs in sorted(tests.glob("*.rs"))]
        for t in explicit_tests:
            nm = t.get("name")
            if not nm and t.get("path"):
                nm = Path(t["path"]).stem
            if nm and nm not in names:
                names.append(nm)
        cost.integration_targets = len(names)
        cost.integration_target_names = names

        # Heaviness (bevy-linking = slow to link). One aggregate binary links its
        # whole tree together, so it is a single heavy target if any file pulls in
        # bevy; with auto-discovery, attribute per top-level target as before.
        if not autotests:
            tree_txt = "".join(
                rs.read_text(encoding="utf-8", errors="replace")
                for rs in tests.rglob("*.rs"))
            cost.heavy_targets = 1 if (names and HEAVY_MARKER.search(tree_txt)) else 0
        else:
            for rs in sorted(tests.glob("*.rs")):
                tree_txt = rs.read_text(encoding="utf-8", errors="replace")
                sub = tests / rs.stem
                if sub.is_dir():
                    for s in sub.rglob("*.rs"):
                        tree_txt += s.read_text(encoding="utf-8", errors="replace")
                if HEAVY_MARKER.search(tree_txt):
                    cost.heavy_targets += 1
    return cost


def static_model(root: Path) -> dict:
    crates = [analyze_crate(m, root) for m in workspace_members(root) if m.is_dir()]
    crates.sort(key=lambda c: (c.consolidatable, c.link_targets), reverse=True)
    totals = {
        "crates": len(crates),
        "integration_targets": sum(c.integration_targets for c in crates),
        "lib_test_targets": sum(1 for c in crates if c.has_lib_tests),
        "link_targets": sum(c.link_targets for c in crates),
        "heavy_targets": sum(c.heavy_targets for c in crates),
        "consolidatable_links": sum(c.consolidatable for c in crates),
        "integration_files": sum(c.integration_files for c in crates),
        "integration_loc": sum(c.integration_loc for c in crates),
        "inline_test_loc": sum(c.inline_test_loc for c in crates),
        "test_loc": sum(c.integration_loc + c.inline_test_loc for c in crates),
        "test_fns": sum(c.test_fns for c in crates),
    }
    return {"totals": totals, "crates": [asdict(c) for c in crates]}


def human_report(model: dict) -> str:
    t = model["totals"]
    out = []
    out.append("=" * 68)
    out.append("TEST SUITE COST MODEL (static)")
    out.append("=" * 68)
    out.append(f"  workspace crates ............. {t['crates']}")
    out.append(f"  test functions ............... {t['test_fns']}")
    out.append(f"  test LOC (inline + integ) .... {t['test_loc']:,} "
               f"({t['inline_test_loc']:,} inline + {t['integration_loc']:,} integ)")
    out.append("")
    out.append("  COMPILE/LINK DRIVERS (a full `cargo test` links each):")
    out.append(f"    integration test binaries .. {t['integration_targets']}")
    out.append(f"    lib-test binaries .......... {t['lib_test_targets']}")
    out.append(f"    -> total extra link steps .. {t['link_targets']}")
    out.append(f"       of which bevy-heavy ..... {t['heavy_targets']}")
    out.append(f"    consolidatable link steps .. {t['consolidatable_links']} "
               f"(mergeable tests/*.rs -> aggregators)")
    out.append("")
    out.append("  Crates with integration test binaries "
               "(the consolidation targets):")
    out.append(f"    {'crate':<32}{'integ':>6}{'heavy':>6}{'lib':>4}{'save':>6}")
    shown = 0
    for c in model["crates"]:
        if c["integration_targets"] == 0:
            continue
        shown += 1
        save = max(0, c["integration_targets"] - 1)
        out.append(f"    {c['name']:<32}{c['integration_targets']:>6}"
                   f"{c['heavy_targets']:>6}{'yes' if c['has_lib_tests'] else '-':>4}"
                   f"{save:>6}")
    lib_only = sum(1 for c in model["crates"]
                   if c["has_lib_tests"] and c["integration_targets"] == 0)
    out.append(f"    (+ {lib_only} more crates with only inline lib tests: "
               f"1 unavoidable link each)")
    out.append("")
    out.append("  Interpretation: link steps dominate test compile time. Merging")
    out.append("  a heavy crate's N top-level tests/*.rs into ONE tests/main.rs")
    out.append("  (with `mod foo;` submodules) removes N-1 links, no test loss.")
    out.append("=" * 68)
    return "\n".join(out)


def measure(root: Path, package: str | None) -> dict:
    """Run `cargo test --no-run` and time the wall clock. Slow; opt-in."""
    cargo = os.path.expanduser("~/.cargo/bin/cargo")
    if not os.path.exists(cargo):
        cargo = "cargo"
    cmd = [cargo, "test", "--no-run"]
    if package:
        cmd += ["-p", package]
    else:
        cmd += ["--workspace"]
    print(f"[measure] running: {' '.join(cmd)}", file=sys.stderr)
    start = time.monotonic()
    proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    elapsed = time.monotonic() - start
    # count compiled test binaries from cargo's stderr ("Compiling"/"Executable")
    execs = len(re.findall(r"Executable .*\(target", proc.stderr))
    return {
        "cmd": " ".join(cmd),
        "wall_seconds": round(elapsed, 1),
        "exit_code": proc.returncode,
        "test_executables_built": execs,
        "ok": proc.returncode == 0,
        "stderr_tail": proc.stderr.strip().splitlines()[-15:],
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--json", action="store_true", help="machine-readable output")
    ap.add_argument("--measure", action="store_true",
                    help="ALSO run `cargo test --no-run` and time it (slow)")
    ap.add_argument("-p", "--package", default=None,
                    help="with --measure, restrict to one crate")
    args = ap.parse_args()

    model = static_model(REPO_ROOT)
    if args.measure:
        model["measured"] = measure(REPO_ROOT, args.package)

    if args.json:
        print(json.dumps(model, indent=2))
    else:
        print(human_report(model))
        if "measured" in model:
            mm = model["measured"]
            print("\nMEASURED (cargo test --no-run):")
            print(f"  {mm['cmd']}")
            print(f"  wall time ............ {mm['wall_seconds']}s "
                  f"({'ok' if mm['ok'] else 'FAILED rc=%d' % mm['exit_code']})")
            print(f"  test binaries built .. {mm['test_executables_built']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
