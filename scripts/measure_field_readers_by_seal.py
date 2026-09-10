#!/usr/bin/env python3
"""Enumerate the CROSS-CRATE readers of a struct's fields by SEALING them.

⭐⭐ **THE ONE INSTRUMENT WITH NO BLIND SPOTS, AND THAT IS THE ONLY REASON TO
SPEND A BUILD ON IT.** A text scan of `.field_name` cannot answer this: the field
names that matter are `id`, `body`, `kit`, `mount`, `vitals` — words that appear
on every other type in the tree — and a scan also cannot see a read through a
codec, a `Reflect` path, a macro, or a helper three calls down. Sealing asks the
compiler instead, and the compiler cannot miss a read it has to reject.

⇒ Rewrite `pub <field>:` to `pub(crate) <field>:` inside ONE named struct, run
`cargo check --workspace --all-targets`, collect every `E0616` (private field),
group them by field, and put the file back. The result is the exact set of
readers OUTSIDE the owning crate, per field — which is what a "can this type be
split" question actually needs.

⛔⛔ **IT IS A MEASUREMENT, NOT A CHANGE.** The seal is reverted before this
script exits and the file's hash is checked against the one taken before. Making
a field private is an API decision; whether the seal is worth KEEPING is a
separate proposal that a census does not get to make on its own.

⚠ **WHAT IT STILL CANNOT SEE: SAME-CRATE READERS.** `pub(crate)` keeps the field
visible inside its own crate, so a reader in the defining crate compiles fine and
never appears. That is deliberate — the question these packets ask is which
FOREIGN consumers depend on a field — but it means a field reported with zero
readers is "zero outside its crate", never "unused". Seal to private (`--private`)
if you need the stronger claim, and expect the owning crate's own code to light
up.

⚠ AND A FIELD BEHIND A FEATURE THIS BUILD DOES NOT ENABLE IS NOT CHECKED. The
run uses `--workspace --all-targets`; add `--features` if a consumer of yours is
gated.

    python3 scripts/measure_field_readers_by_seal.py PreparedCharacterDefinition
    python3 scripts/measure_field_readers_by_seal.py GroundItem --private
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ROOTS = ("crates", "game", "tools")
CARGO = os.path.expanduser("~/.cargo/bin/cargo")


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def find_struct(name: str) -> tuple[pathlib.Path, int, int]:
    """The file and the byte span of `struct <name> { .. }`'s body."""
    pattern = re.compile(
        rf"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?struct\s+{re.escape(name)}\b[^{{;]*\{{",
        re.MULTILINE,
    )
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            text = path.read_text(encoding="utf-8", errors="ignore")
            match = pattern.search(text)
            if not match:
                continue
            depth, j = 0, match.end() - 1
            while j < len(text):
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            return path, match.end(), j
    raise SystemExit(f"no struct named {name!r} found under {ROOTS}")


def mark_deprecated(text: str, start: int, end: int) -> tuple[str, list[str]]:
    """Tag every `pub` field of one struct `#[deprecated]`.

    ⭐⭐ **THE SEAL ONLY EVER REACHED ONE RING, AND DEPRECATION REACHES ALL OF
    THEM.** Making a field private is an ERROR at every use site, so the nearest
    dependent crate fails to compile and everything downstream of it is never
    built — measured on `GroundItem`: 8 sites, all in `ambition_abilities`, while
    the writer census had already named three other crates constructing the type.
    `--keep-going` recovers crates INDEPENDENT of the failure and there were
    none. The limit is structural, not a flag.

    ⇒ `#[deprecated]` is a WARNING. Compilation continues through the whole
    workspace and every use site reports, in one pass, including SAME-CRATE ones
    — which also removes the `pub(crate)` blind spot the seal could not close.

    ⚠ A consumer carrying `#[allow(deprecated)]` is still invisible, and a
    `derive` that touches the fields warns inside the owning crate.
    """
    body = text[start:end]
    fields = re.findall(r"^([ \t]*)pub\s+([a-z_][A-Za-z_0-9]*)\s*:", body, re.MULTILINE)
    tagged = re.sub(
        r"^([ \t]*)(pub\s+[a-z_][A-Za-z_0-9]*\s*:)",
        lambda m: f'{m.group(1)}#[deprecated(note = "census seal")]\n{m.group(1)}{m.group(2)}',
        body,
        flags=re.MULTILINE,
    )
    return text[:start] + tagged + text[end:], [name for _, name in fields]


def seal(text: str, start: int, end: int, visibility: str) -> tuple[str, list[str]]:
    body = text[start:end]
    fields = re.findall(r"^([ \t]*)pub\s+([a-z_][A-Za-z_0-9]*)\s*:", body, re.MULTILINE)
    sealed = re.sub(
        r"^([ \t]*)pub(\s+[a-z_][A-Za-z_0-9]*\s*:)",
        rf"\1{visibility}\2",
        body,
        flags=re.MULTILINE,
    )
    return text[:start] + sealed + text[end:], [name for _, name in fields]


def cargo_check() -> list[dict]:
    proc = subprocess.run(
        # ⛔⛔ `--keep-going` OR THE ANSWER IS ONE CRATE DEEP. Without it cargo
        # stops at the first crate that fails, and a sealed field fails the
        # NEAREST dependent — so the first run of this script reported 8 sites,
        # all in `ambition_abilities`, while the writer census had already named
        # three other crates constructing the type. They were never compiled.
        [CARGO, "check", "--workspace", "--all-targets", "--keep-going",
         "--message-format=json"],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    out = []
    for line in proc.stdout.split("\n"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("struct_name")
    parser.add_argument(
        "--private",
        action="store_true",
        help="VISIBILITY seal (errors, one dependent ring only) instead of the default deprecation seal",
    )
    parser.add_argument(
        "--visibility-seal",
        action="store_true",
        help="VISIBILITY seal to pub(crate) -- errors, and reaches only the nearest dependent ring",
    )
    args = parser.parse_args()

    path, start, end = find_struct(args.struct_name)
    rel = path.relative_to(REPO)

    # ⛔⛔ A TRIP-WIRE, BECAUSE THIS SCRIPT'S SUBJECT IS A REAL SOURCE FILE. A
    # guard test whose subject mutates the tree needs one: a crash between the
    # seal and the restore would leave somebody else's checkout edited, and in a
    # tree shared with peers that is somebody else's afternoon.
    dirty = subprocess.run(
        ["git", "status", "--porcelain", "--", str(rel)],
        cwd=REPO, capture_output=True, text=True,
    ).stdout.strip()
    if dirty:
        raise SystemExit(
            f"⛔ {rel} has uncommitted changes. This script edits and restores it; "
            "refusing to run so a restore cannot silently discard your work."
        )

    original = path.read_text(encoding="utf-8")
    before = digest(path)
    by_visibility = args.private or args.visibility_seal
    if by_visibility:
        sealed_text, fields = seal(original, start, end, "" if args.private else "pub(crate)")
        how = "private" if args.private else "pub(crate)"
    else:
        sealed_text, fields = mark_deprecated(original, start, end)
        how = "deprecated"
    if not fields:
        raise SystemExit(f"{args.struct_name} has no `pub` fields to seal")

    print(f"sealing {len(fields)} field(s) of {args.struct_name} in {rel} by {how} ...")
    tmp = path.with_suffix(path.suffix + ".seal.tmp")
    tmp.write_text(sealed_text, encoding="utf-8")
    os.replace(tmp, path)
    try:
        messages = cargo_check()
    finally:
        tmp.write_text(original, encoding="utf-8")
        os.replace(tmp, path)
        after = digest(path)
        if after != before:
            raise SystemExit(
                f"⛔⛔ {rel} DID NOT RESTORE ({before[:12]} -> {after[:12]}). "
                "Fix the tree by hand before doing anything else."
            )
        print(f"restored {rel} (sha256 {after[:12]} matches)")

    # ⛔⛔ THREE CODES, AND THE FIRST RUN COLLECTED ONE. E0616 is a private field
    # READ (`x.spec`); E0603 a private item reached by path; **E0451 is a
    # private field in a STRUCT LITERAL** — `GroundItem { spec, pos, .. }` — and
    # leaving it out reported 8 sites, all reads, while the writer census had
    # already named `actor_monolith`, `ambition_demo_smash` and
    # `ambition_content` CONSTRUCTING the type. A seal that collects only reads
    # answers a narrower question than the one it prints.
    # ⛔⛔ AN INSTRUMENT THAT SWALLOWS A COMPILE ERROR REPORTS ITS OWN FINDING.
    # The first deprecation run emitted `note = \"census seal\"` -- literal
    # backslashes, from an over-escaped replacement -- so the OWNING crate failed
    # to parse, nothing downstream was built, no warnings existed, and the report
    # printed a confident `0` for all four fields. A null result is a claim about
    # the instrument until the instrument proves it compiled.
    broke_owner = any(
        isinstance(msg.get("message"), dict)
        and msg["message"].get("level") == "error"
        and any(
            span.get("file_name", "").endswith(str(rel))
            for span in msg["message"].get("spans", [])
        )
        for msg in messages
    )
    if broke_owner and not by_visibility:
        raise SystemExit(
            f"⛔⛔ the deprecation seal made {rel} FAIL TO COMPILE, so no use site "
            "could report and any count below would be a finding about this "
            "script. Fix the rewrite before trusting a number."
        )

    readers: dict[str, set[tuple[str, int]]] = collections.defaultdict(set)
    for message in messages:
        payload = message.get("message")
        if not isinstance(payload, dict):
            continue
        code = (payload.get("code") or {}).get("code")
        text = payload.get("message", "")
        if by_visibility:
            if code not in ("E0616", "E0603", "E0451"):
                continue
        elif "deprecated" not in text:
            continue
        # ⛔⛔ THE MESSAGE NAMES THE FIELD QUALIFIED. rustc says "use of deprecated
        # field `GroundItem::spec`", not "`spec`", so matching on `` `spec` ``
        # attributed every row to nothing and the report printed 0 for all four
        # fields — a NULL RESULT that was a finding about this parser, not about
        # the tree. Rows that still fail to attribute are printed under `?`
        # rather than dropped, because a lookup that silently defaults reports a
        # confident wrong answer.
        field = next(
            (f for f in fields if re.search(rf"\b{re.escape(f)}\b", text)), None
        )
        for span in payload.get("spans", []):
            if not span.get("is_primary"):
                continue
            name = field or next((f for f in fields if f == span.get("text", [{}])[0].get("text", "").strip()), None)
            readers[name or "?"].add((span["file_name"], span["line_start"]))

    print(f"\n⛔ USE SITES OF {args.struct_name}'S FIELDS ({how} seal)\n")
    total = 0
    for name in fields:
        sites = sorted(readers.get(name, set()))
        total += len(sites)
        mark = "   " if sites else " ·"
        print(f"  {mark} {name:28s} {len(sites):3d}")
        for where, line in sites:
            print(f"          {where}:{line}")
    stray = sorted(readers.get("?", set()))
    if stray:
        print(f"   ⛔ ? (unattributed)           {len(stray):3d}")
        for where, line in stray:
            print(f"          {where}:{line}")
        total += len(stray)
    print(f"\n   {total} reader site(s) across {len(fields)} field(s)")
    print("\n⛔ WHAT THIS CANNOT SEE")
    if not by_visibility:
        print("   * A consumer carrying `#[allow(deprecated)]`.")
        print("   * A `derive` on the struct warns inside the OWNING crate; those rows")
        print("     are the owner's own code, not a foreign dependency.")
    if by_visibility and not args.private:
        print("   * SAME-CRATE readers: `pub(crate)` keeps the field visible inside its")
        print("     own crate. A field with 0 here is 'unused OUTSIDE', never 'unused'.")
    print("   * A consumer behind a feature this build does not enable.")
    print("   * ⛔⛔ ONLY THE NEAREST DEPENDENT RING, AND THIS IS STRUCTURAL. A crate")
    print("     that depends on one the seal broke cannot be compiled at all, so its")
    print("     own readers never appear. `--keep-going` recovers every crate that is")
    print("     INDEPENDENT of the failures; it cannot recover one downstream of them.")
    print("     ⇒ This is a complete enumeration of the ring it reaches, NOT of the")
    print("     reader set. To go deeper, fix the ring and seal again.")
    print("   * A workspace already red for an unrelated reason makes it an undercount.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
