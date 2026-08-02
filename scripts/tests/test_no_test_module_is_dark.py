"""No feature-gated test is un-run by `run_tests.py`'s own exemptions.

## What actually runs

Two mechanisms cover feature-gated tests, and both have to be modelled or this
check invents findings:

1. `cargo test --workspace` builds every member with its DEFAULT features and
   unifies. `cargo metadata`'s resolve graph is cargo's own answer for that set.
2. `run_tests.py` then runs, per crate, ONE job with every non-default feature
   turned on at once — so a feature outside the default closure is still covered.

⚠ **the first two drafts of this guard modelled only one of those and reported
five dark modules; all five were fine.** Draft one hand-rolled the feature
closure from the manifests and was wrong about three crates cargo resolves ON.
Draft two asked cargo, and was still wrong about `ambition_causal/bevy` and
`ambition_platformer2d_actor_monolith/causal` because it had never read the
runner. A guard that models a system instead of asking it produces confident
false findings, which cost more than the gap they were looking for.

## What is actually at risk

Mechanism 2 has two documented holes, and each is stated in `run_tests.py` as a
claim about the repository rather than enforced against it:

* `SKIP_FEATURE_JOB` — crates whose feature job is skipped because it would
  recompile the whole Bevy graph "for zero added coverage". Its own comment
  says: *"RULE: adding a `#[cfg(feature = ...)]` test to a skipped crate must
  remove the skip in the same commit — a stale entry here silently un-runs
  tests."*
* `DENY_EXACT` / `DENY_PREFIX` — features excluded from the feature job (they
  abort the binary, swap the render path, or target another platform). Each
  entry's comment asserts *"nothing gates a test on it, so denying it loses no
  coverage."*

Both are true today. Neither is checked, and both are exactly the kind of claim
that stops being true without anyone noticing — the test still exists, still
looks green in the file, and is simply never compiled. That is worse than a
missing test, because the count makes it look executed.

⚠ this deliberately does NOT check "is this feature in the default closure".
That question is answered by mechanism 2 for every crate that is not exempt, and
asking it anyway is what produced the false findings above.
"""

from __future__ import annotations

import ast
import json
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUNNER = REPO / "scripts" / "run_tests.py"

# `#[cfg(feature = "x")]` immediately above a `mod y;` declaration.
GATED_MOD = re.compile(
    r'#\[cfg\(\s*feature\s*=\s*"(?P<feature>[^"]+)"\s*\)\]\s*'
    r'(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<module>\w+)\s*;'
)
# `#[cfg(feature = "x")]` above an INLINE `mod y { ... }` or a `#[test]` fn.
GATED_INLINE = re.compile(
    r'#\[cfg\(\s*feature\s*=\s*"(?P<feature>[^"]+)"\s*\)\]\s*'
    r'(?:#\[cfg\(test\)\]\s*)?'
    r'(?:(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{|#\[test\])'
)


def _runner_constant(name: str) -> set[str] | tuple[str, ...]:
    """Read a literal set/tuple out of `run_tests.py` without importing it.

    Importing would execute the runner's module level; parsing keeps this a
    read. ⚠ it also means a RENAME of either constant makes this guard silently
    watch nothing, so both lookups assert they found something.
    """
    tree = ast.parse(RUNNER.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id == name
            for target in node.targets
        ):
            return ast.literal_eval(node.value)
    raise AssertionError(
        f"{name} is gone from run_tests.py, so this guard is watching nothing. "
        "It exists to enforce that constant's own documented rule; find where "
        "the exemption moved to and point it there."
    )


def _workspace_packages() -> list[dict]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return json.loads(out)["packages"]


def _default_features(pkg: dict) -> set[str]:
    """The package's own default closure (within-package edges only)."""
    features, out = pkg["features"], set()

    def walk(name: str) -> None:
        if name in out or name not in features:
            return
        out.add(name)
        for entry in features[name]:
            if "/" not in entry and not entry.startswith("dep:"):
                walk(entry)

    walk("default")
    return out


def _gated_test_counts(pkg: dict) -> list[tuple[str, str, int]]:
    """`(feature, where, test_count)` for every non-default feature gating tests."""
    root = Path(pkg["manifest_path"]).parent
    src = root / "src"
    if not src.is_dir():
        return []
    default = _default_features(pkg)
    found: list[tuple[str, str, int]] = []

    for rust in sorted(src.rglob("*.rs")):
        source = rust.read_text(encoding="utf-8", errors="ignore")

        for match in GATED_MOD.finditer(source):
            feature, module = match["feature"], match["module"]
            if feature in default:
                continue
            candidates = [
                rust.parent / f"{module}.rs",
                rust.parent / module / "mod.rs",
                rust.parent / rust.stem / f"{module}.rs",
                rust.parent / rust.stem / module / "mod.rs",
            ]
            target = next((path for path in candidates if path.is_file()), None)
            if target is None:
                continue
            tests = target.read_text(encoding="utf-8", errors="ignore").count("#[test]")
            if tests:
                found.append((feature, str(target.relative_to(REPO)), tests))

        for match in GATED_INLINE.finditer(source):
            feature = match["feature"]
            if feature in default:
                continue
            # Everything from the gate to the end of the enclosing item; a count
            # of 1 is enough to make the point, so precision past that is noise.
            tail = source[match.start() : match.start() + 4000]
            if "#[test]" in tail:
                found.append((feature, str(rust.relative_to(REPO)), tail.count("#[test]")))

    return found


def _is_denied(feature: str, deny_exact: set[str], deny_prefix: tuple[str, ...]) -> bool:
    return feature in deny_exact or feature.startswith(tuple(deny_prefix))


def _unrun() -> list[str]:
    skipped = set(_runner_constant("SKIP_FEATURE_JOB"))
    deny_exact = set(_runner_constant("DENY_EXACT"))
    deny_prefix = tuple(_runner_constant("DENY_PREFIX"))
    assert skipped and deny_exact and deny_prefix, "an exemption list came back empty"

    findings: list[str] = []
    for pkg in _workspace_packages():
        name = pkg["name"]
        # One line per (file, feature). Inline gates are matched with an
        # overlapping window, so the same site can be seen more than once with
        # different counts; the largest is the informative one.
        worst: dict[tuple[str, str], int] = {}
        for feature, where, tests in _gated_test_counts(pkg):
            key = (where, feature)
            worst[key] = max(worst.get(key, 0), tests)
        for (where, feature), tests in sorted(worst.items()):
            if name in skipped:
                findings.append(
                    f"{where}: {tests} test(s) behind `{name}/{feature}`, and "
                    f"`{name}` is in SKIP_FEATURE_JOB — so no job ever builds them"
                )
            elif _is_denied(feature, deny_exact, deny_prefix):
                findings.append(
                    f"{where}: {tests} test(s) behind `{name}/{feature}`, which is "
                    "DENIED from the feature job — so no job ever builds them"
                )
    return sorted(findings)


def test_no_exempted_feature_hides_tests_from_every_job():
    unrun = _unrun()
    assert not unrun, (
        "these tests are compiled by no job in the suite — the crate's feature "
        "job is skipped, or the feature that gates them is denied from it:\n  "
        + "\n  ".join(unrun)
        + "\n\nRemove the crate from SKIP_FEATURE_JOB (its own comment requires "
        "this in the same commit as the new test), or un-deny the feature, or "
        "add a dedicated `run_tests.py` job. A test nothing compiles is worse "
        "than a missing one, because the count makes it look executed."
    )
