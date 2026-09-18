#!/usr/bin/env python3
"""Every declared `[dev-dependencies]` entry is named by that crate's own code.

⛔⛤ **THE UNUSED-DEPENDENCY CENSUS CLOSED WITHOUT ASKING THIS.** Its detector is
a `--lib` run and a dev-dependency never enters one, which
`docs/planning/triage/unused-dependency-census.md` states in its own words —
and the page still reported 64 crates as needing "no further work". Asking the
question properly on 2026-09-18 found SEVEN stranded entries across four
crates, one of them carrying a comment that described a use which did not
exist. This exists so the next one cannot arrive unnoticed.

⚠ **WHY THIS IS TEXTUAL AND NOT THE COMPILER.** `-W unused_crate_dependencies`
is the authoritative answer and is unaffordable: `RUSTFLAGS` is part of cargo's
fingerprint, so switching it on rebuilds the workspace — the failure
`scripts/check_no_warnings.py` refuses to cause, having filled this target
directory three times. So this reports CANDIDATES, and the settling instrument
stays delete-and-build.

⛔ **TWO WAYS THIS MEASUREMENT LIES, both met while writing it:**

1. **A manifest is not split on `"[dev-dependencies]"`.** That string also
   matches a MENTION of the table inside a comment, and the first version of
   this sweep did exactly that: it took the text after a comment and reported
   five dependencies out of the wrong crate's `[dependencies]` block. Parsed
   with `tomllib`, which cannot make that mistake.

2. **A substring is not a reference.** A loose search for `insta` returns 562
   hits in `game/ambition_app` and 443 in the actor monolith, every one of them
   inside the word `install`. Only a `name::` or `name!` form counts.
"""

from __future__ import annotations

import pathlib
import re
import sys
import tomllib

REPO = pathlib.Path(__file__).resolve().parents[1]

#: Source roots a dev-dependency may legitimately be named from. `src/` is here
#: because `#[cfg(test)]` modules live inside it, which is most of this
#: repository's test code.
SOURCE_DIRS = ("src", "tests", "benches", "examples")

#: A crate that genuinely declares a dev-dependency nothing NAMES, with the
#: reason. Empty, and it should stay that way: the delete-and-build test settles
#: this question, so an entry here is a claim the compiler disagreed with.
KEPT_UNNAMED: dict[tuple[str, str], str] = {}

#: ⛔ ANTI-VACUITY. A parser that silently stops finding manifests reports a
#: clean sweep. Measured 2026-09-18: 18 crates declare dev-dependencies (content_builder
#: lost its table when its one stranded entry went).
MIN_CRATES_WITH_DEV_DEPS = 15


def manifests() -> list[pathlib.Path]:
    return [
        p
        for p in sorted(REPO.rglob("Cargo.toml"))
        if "target" not in p.parts and ".git" not in p.parts
    ]


def dev_dependencies(doc: dict) -> dict[str, object]:
    """The plain table plus every `[target.'cfg(..)'.dev-dependencies]`."""
    devs = dict(doc.get("dev-dependencies") or {})
    for target in (doc.get("target") or {}).values():
        devs.update(target.get("dev-dependencies") or {})
    return devs


def source_text(crate_dir: pathlib.Path) -> str:
    chunks = []
    for sub in SOURCE_DIRS:
        root = crate_dir / sub
        if root.is_dir():
            for f in root.rglob("*.rs"):
                chunks.append(f.read_text(errors="replace"))
    return "\n".join(chunks)


def is_named(dep: str, spec: object, blob: str) -> bool:
    """`dep::` or `dep!` somewhere in the crate's own source.

    A renamed dependency (`foo = { package = "bar" }`) is reached in code by the
    KEY, not by the package name, so both spellings count.
    """
    idents = {dep.replace("-", "_")}
    if isinstance(spec, dict) and "package" in spec:
        idents.add(str(spec["package"]).replace("-", "_"))
    alternatives = "|".join(re.escape(i) for i in sorted(idents))
    return re.search(rf"(^|[^A-Za-z0-9_])({alternatives})\s*(::|!)", blob) is not None


def main() -> int:
    found = manifests()
    crates_with_dev_deps = 0
    stranded: list[str] = []

    for manifest in found:
        try:
            doc = tomllib.loads(manifest.read_text(errors="replace"))
        except tomllib.TOMLDecodeError as exc:
            print(f"⛔ {manifest.relative_to(REPO)} does not parse as TOML: {exc}")
            return 1
        if "package" not in doc:
            continue
        devs = dev_dependencies(doc)
        if not devs:
            continue
        crates_with_dev_deps += 1
        crate_dir = manifest.parent
        rel = str(crate_dir.relative_to(REPO))
        blob = source_text(crate_dir)
        for dep, spec in sorted(devs.items()):
            if is_named(dep, spec, blob):
                continue
            if (rel, dep) in KEPT_UNNAMED:
                continue
            stranded.append(f"  {rel}: `{dep}` is declared and never named")

    if crates_with_dev_deps < MIN_CRATES_WITH_DEV_DEPS:
        print(
            f"⛔⛔ only {crates_with_dev_deps} crate(s) with dev-dependencies found "
            f"(expected {MIN_CRATES_WITH_DEV_DEPS}+). That is a claim about this "
            "parser, not about the tree — a clean sweep here would be vacuous."
        )
        return 1

    if stranded:
        print(
            f"{len(stranded)} dev-dependency declaration(s) nothing names:\n"
            + "\n".join(stranded)
            + "\n\nEach is a CANDIDATE, not a verdict — this sweep is textual. "
            "Settle it the way the census does: delete the line and run "
            "`cargo check -p <crate> --all-targets`. If it compiles, the line was "
            "dead weight a shipped profile still had to resolve.\n"
            "⚠ Check the comment above the line too. One of the seven found on "
            "2026-09-18 described a use that did not exist."
        )
        return 1

    print(
        f"ok: every dev-dependency is named by its own crate "
        f"({crates_with_dev_deps} crate(s) with a `[dev-dependencies]` table, "
        f"{len(found)} manifest(s) scanned)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
