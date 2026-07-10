#!/usr/bin/env python3
"""Generate and CHECK each crate's `MODULES.md` — the D-B navigability standard.

`docs/planning/engine/decomposition.md` Phase D-B says navigability below the
crate line is won by three things: every module under ~1.5k lines with a header
stating its ONE concern; the `features/` hub globs dissolved; and **a `MODULES.md`
map at each crate root**. The first two hold. This is the third, and it is the
last unmet piece of the standard the 2026-07-10 ledger ruling leans on when it
says no further crate split is owed.

The map is generated from the code, not written beside it, so it cannot rot: each
row is a crate-root module and the FIRST sentence of that module's own `//!` doc.
A module with no `//!` header is a finding, not an omission — the same standard
already requires one.

    python scripts/modules_md.py            # check (exit 1 on drift)
    python scripts/modules_md.py --write    # regenerate every MODULES.md

Hand-written prose goes BELOW the managed block and survives regeneration.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

BEGIN = "<!-- BEGIN generated module map (scripts/modules_md.py) -->"
END = "<!-- END generated module map -->"

# `pub mod x;` / `mod x;` / `pub(crate) mod x;`, optionally `#[cfg(...)]`-gated.
MOD_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_][a-z0-9_]*)\s*;")


def declared_modules(text: str) -> list[str]:
    """Crate-root modules, minus the `#[cfg(test)]`-gated ones.

    A test module is not part of the crate's concern map, and requiring it to
    carry a `//!` header would be noise rather than navigability.
    """
    names: list[str] = []
    cfg_test = False
    for raw in text.splitlines():
        stripped = raw.strip()
        if stripped.startswith("#[cfg(test)]"):
            cfg_test = True
            continue
        m = MOD_RE.match(raw)
        if m:
            if not cfg_test:
                names.append(m.group(1))
            cfg_test = False
            continue
        if stripped and not stripped.startswith(("#[", "//")):
            cfg_test = False
    return names


def crate_roots() -> list[Path]:
    """Every workspace crate with a `src/lib.rs`, in a stable order."""
    roots = []
    for parent in ("crates", "game"):
        for d in sorted((REPO / parent).iterdir()):
            if (d / "src" / "lib.rs").is_file():
                roots.append(d)
    return roots


def module_file(src: Path, name: str) -> Path | None:
    for cand in (src / f"{name}.rs", src / name / "mod.rs"):
        if cand.is_file():
            return cand
    return None


def first_doc_sentence(path: Path) -> str:
    """The first sentence of a module's `//!` header, flattened to one line."""
    lines: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        stripped = raw.strip()
        if stripped.startswith("//!"):
            lines.append(stripped[3:].strip())
            continue
        if not lines:
            if stripped.startswith("#!["):  # inner attributes precede the doc
                continue
            if stripped == "":
                continue
        break
    if not lines:
        return ""
    # Join the leading paragraph, then cut at the first sentence end.
    para: list[str] = []
    for line in lines:
        if line == "":
            break
        para.append(line)
    text = " ".join(para).strip()
    # `Concern. Detail.` -> `Concern.`  (don't cut on `e.g.` / abbreviations)
    m = re.search(r"(?<![A-Z])\.\s", text)
    if m:
        text = text[: m.start() + 1]
    return re.sub(r"\s+", " ", text).strip()


def crate_summary(lib: Path) -> str:
    return first_doc_sentence(lib)


def build_map(crate: Path) -> tuple[str, list[str]]:
    """Return the managed markdown block, plus any findings."""
    src = crate / "src"
    lib = src / "lib.rs"
    text = lib.read_text(encoding="utf-8")
    names = sorted(set(declared_modules(text)))

    findings: list[str] = []
    rows: list[str] = []
    for name in names:
        path = module_file(src, name)
        if path is None:
            # An inline `mod x { .. }` in lib.rs, or a `#[path]` override.
            continue
        concern = first_doc_sentence(path)
        if not concern:
            findings.append(
                f"{crate.name}: module `{name}` has no `//!` header stating its ONE concern "
                f"({path.relative_to(REPO)})"
            )
            concern = "_(no `//!` header — see D-B navigability standard)_"
        rel = path.relative_to(crate).as_posix()
        rows.append(f"| [`{name}`]({rel}) | {concern} |")

    body = [
        BEGIN,
        "",
        f"**{crate.name}** — {crate_summary(lib) or '_(no crate `//!` header)_'}",
        "",
        "| Module | Its ONE concern (from the module's own `//!` header) |",
        "|---|---|",
        *rows,
        "",
        f"_{len(rows)} crate-root modules. Regenerate: `python scripts/modules_md.py --write`._",
        "",
        END,
    ]
    return "\n".join(body), findings


def render(crate: Path, existing: str | None) -> tuple[str, list[str]]:
    block, findings = build_map(crate)
    header = f"# `{crate.name}` — module map\n\n"
    tail = (
        "\n\n## Notes\n\n"
        "_Hand-written notes live here and survive regeneration: the crate's "
        "authoritative state, its seams, and anything the module headers cannot say._\n"
    )
    if existing and BEGIN in existing and END in existing:
        pre = existing.split(BEGIN)[0]
        post = existing.split(END, 1)[1]
        return pre + block + post, findings
    return header + block + tail, findings


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--write", action="store_true", help="regenerate every MODULES.md")
    args = ap.parse_args()

    drift: list[str] = []
    findings: list[str] = []
    for crate in crate_roots():
        target = crate / "MODULES.md"
        existing = target.read_text(encoding="utf-8") if target.is_file() else None
        rendered, crate_findings = render(crate, existing)
        findings.extend(crate_findings)
        if args.write:
            target.write_text(rendered, encoding="utf-8")
        elif existing is None:
            drift.append(f"{crate.relative_to(REPO)}/MODULES.md is missing")
        elif existing != rendered:
            drift.append(f"{crate.relative_to(REPO)}/MODULES.md is stale")

    if findings:
        print(
            f"{len(findings)} module(s) without a `//!` concern header:",
            file=sys.stderr,
        )
        for f in findings:
            print(f"  {f}", file=sys.stderr)

    if args.write:
        print(f"wrote MODULES.md for {len(crate_roots())} crates")
        return 0

    if drift:
        print(f"\n{len(drift)} MODULES.md file(s) out of date:", file=sys.stderr)
        for d in drift:
            print(f"  {d}", file=sys.stderr)
        print(
            "\nRegenerate with: python scripts/modules_md.py --write", file=sys.stderr
        )
        return 1

    print(f"MODULES.md up to date for {len(crate_roots())} crates")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
