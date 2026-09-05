#!/usr/bin/env python3
"""Which authored BOOLEAN parameter modes has no shipped content ever turned on?

⭐ THE QUESTION, and it is not "does this field exist". A technique parameter can
be fully built, guarded, and rollback-registered while every authored customer
leaves it `false` — the behaviour behind it then never runs in the shipped game
and no test notices, because the tests set it themselves. Two such modes turned
up by accident on 2026-09-05 (`TeleportParams::behind_nearest_foe`, and Sing's
engine earlier the same day), which is what this census is for: a catalog that
only lists what EXISTS cannot find them; one that joins the type against
authored content can.

⛔ IT COUNTS BY EFFECTIVE DEFAULT, and getting that wrong inverts the answer.
A field whose DEFAULT is `true` and which no content sets is not dormant — it is
ALWAYS ON. 16 of 51 modes default true here (via `impl Default` OR a constructor
like `Chest::new`), so a census assuming default-false mislabels a third of the
population. The first version of this script did exactly that.

  live     — default false and set `true` somewhere, or default true (always on)
  dormant  — DEFAULT FALSE and never set true. The expensive case: it looks used
             because the field is named, and a grep for it says "used"
  unnamed  — never mentioned in content at all (dormant unless default true)

⚠ THE AUTHORED CORPUS INCLUDES RUST. This repo authors content in `.ron`,
`.ldtk`, `.yarn` AND in the content crates' own `.rs`. A first version of this
scan globbed only data files, called `behind_nearest_foe` unauthored, and was
wrong about which half of the tree does the authoring.
⛔ AND `git ls-files 'game/x/src/**/*.rs'` DOES NOT MATCH `src/*.rs` — it matched
only subdirectories and silently dropped every top-level content module, which is
where most authoring lives. The filtering is done in Python for that reason.

⚠ NOT A GATE, and it must not become one: content breadth is not gated in this
repository. The number moves whenever content lands — it moved by one while this
script was being written — so a run is a snapshot and prints its commit.

⛔⛔ IT COUNTS LEAVES, NOT BRANCHES, and that blind spot is structural. A whole
REACTION with no authored customer is invisible to a field census unless
something inside it happens to be a bool: `VolumeReaction::Windbox` has no
authored fighter at all, and this script only ever saw its `repeating` flag.
`--variants` adds that axis. ⚠ It is WIDER AND NOISIER — many `Deserialize`
enums are runtime state (`LocomotionState`, `CachePolicy`) rather than authored
vocabulary — so it is a starting list for a person, never a verdict.

⛔⛔ AND THE OBVIOUS SHARPENING DESTROYS THE SIGNAL — measured 2026-09-05, do not
retry it. The tempting filter is "if NO variant of an enum appears in content the
enum is not authored vocabulary, so skip the whole enum". It excludes only 9 of
126 enums, and it LOSES `VolumeReaction::Windbox` — the one hit this axis was
built for and the only one independently confirmed. ⇒ an enum with zero authored
variants is either internal OR an authored vocabulary nobody has used yet, and
the second case is exactly the finding. The filter cannot tell them apart, and
neither can any rule over names; it takes knowing what the enum is for.
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

#: Crates whose `.rs` IS authored content rather than engine code.
#:
#: ⛔⛔ `ambition_characters` IS ON THIS LIST, and leaving it off is why the
#: first version reported two gameplay-critical gates as unauthored. In this
#: repo CONTENT CALLS AUTHORING HELPERS AND THE HELPERS SET THE GATES:
#: `smash_ride::author_summon_ride` writes `spec.gates.forbidden_while_held =
#: true` (the rule that stops a rider recasting his up-B from the saddle), and
#: `smash_repertoire` applies `roots_steering` centrally per slot family because
#: it is a fact about the STANCE rather than about any one move. No content file
#: names either field. ⇒ a corpus restricted to `game/` is blind to every field
#: applied that way, and reports engine RULES as unauthored.
CONTENT_CRATES = (
    "game/ambition_content/",
    "game/ambition_demo_smash/",
    "game/ambition_demo_mary_o/",
    "game/ambition_demo_sanic/",
    "game/ambition_demo_pocket/",
    "game/ambition_demo_twintrack/",
)

#: Deserializable types that are NOT authored content: user settings, dev
#: toggles, per-frame input, and budgets/tuning the engine defaults.
NOT_AUTHORED = re.compile(r"Settings|DeveloperTools|ControlFrame|Budget|Tuning|AbilitySet")

STRUCT = re.compile(
    r"#\[derive\([^)]*Deserialize[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*pub struct (\w+)\s*\{(.*?)\n\}",
    re.S,
)
BOOL_FIELD = re.compile(r"\n\s+pub (\w+):\s*bool\b")


def git(*args: str) -> list[str]:
    return subprocess.run(
        ["git", *args], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()


def authored_corpus(*, with_helpers: bool) -> tuple[str, int]:
    """The authored corpus. ⛔ THE TWO AXES NEED DIFFERENT ONES.

    `with_helpers=True` (FIELD axis) includes `ambition_characters`, because in
    this repo content calls AUTHORING HELPERS and the helpers SET the gates:
    `smash_ride::author_summon_ride` writes `spec.gates.forbidden_while_held =
    true` and no content file names it. Without them the census reports engine
    RULES as unauthored.

    `with_helpers=False` (VARIANT axis) excludes them, because a helper that
    OFFERS a builder is vocabulary, not a customer. `moveset_authoring.rs` names
    `WindboxVolume` six times while no fighter authors a windbox — including it
    makes the one confirmed gap disappear.

    ⇒ APPLIES vs OFFERS is the distinction, and it is not lexical. Two corpora is
    the honest answer; a single one is wrong for one axis whichever way it goes.
    """
    roots = CONTENT_CRATES + (("crates/ambition_characters/",) if with_helpers else ())
    files = [
        f
        for f in git("ls-files")
        if (f.endswith((".ron", ".ldtk", ".yarn")) and f.startswith("game/"))
        or (f.endswith(".rs") and f.startswith(roots))
    ]
    text = "\n".join(
        (REPO / f).read_text(encoding="utf-8", errors="replace") for f in files
    )
    return text, len(files)


def modes() -> list[tuple[str, str, str, bool]]:
    """`(struct, field, file, defaults_true)` for every authored bool mode.

    ⛔ `defaults_true` scans EVERY `impl` block on the type in its own file, not
    just `impl Default` — `Chest::new` sets `persistent: true` in a constructor,
    and a Default-only scan finds 1 of the 16 that actually default true.
    """
    out = []
    for f in git("grep", "-l", "Deserialize", "--", "crates"):
        if "/tests" in f or f.endswith("tests.rs"):
            continue
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        impls: dict[str, str] = {}
        for im in re.finditer(r"\nimpl(?:\s+Default\s+for)?\s+(\w+)\b", text):
            impls[im.group(1)] = impls.get(im.group(1), "") + text[im.start() : im.start() + 3000]
        for m in STRUCT.finditer(text):
            name, body = m.group(1), m.group(2)
            if NOT_AUTHORED.search(name):
                continue
            blob = impls.get(name, "")
            for fm in BOOL_FIELD.finditer(body):
                field = fm.group(1)
                defaults_true = bool(re.search(r"\b" + re.escape(field) + r"\s*:\s*true\b", blob))
                out.append((name, field, f, defaults_true))
    return out


ENUM = re.compile(
    r"#\[derive\([^)]*Deserialize[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*pub enum (\w+)\s*\{(.*?)\n\}",
    re.S,
)
#: `Foo,` / `Foo {` / `Foo(Payload)` — the payload is captured because content
#: often names the TYPE rather than the variant.
VARIANT = re.compile(r"\n\s+([A-Z]\w+)\s*(?:\{|\(\s*(\w+)|,|=)")


def _camel_to_snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def variant_is_authored(variant: str, payload: str | None, corpus: str) -> bool:
    """⛔ THREE SPELLINGS, because content rarely writes the variant name.

    `VolumeReaction::Autolink` is authored — as a field `autolink:` carrying an
    `AutolinkVolume`, and the CamelCase variant name appears nowhere in content.
    A variant-name-only search called it dead, which is a FALSE POSITIVE that
    would have sent someone to author a thing that exists.
    """
    for spelling in (variant, _camel_to_snake(variant), payload or ""):
        if spelling and re.search(r"\b" + re.escape(spelling) + r"\b", corpus):
            return True
    return False


def variants() -> list[tuple[str, str, str, str | None]]:
    """`(enum, variant, file)` for authored enums — the BRANCH axis.

    ⚠ Deliberately not filtered as hard as the field axis. Curating it needs
    judgement about which `Deserialize` enums are authored vocabulary, so this
    prints a list for a person rather than a count for a report.
    """
    out = []
    for f in git("grep", "-l", "Deserialize", "--", "crates"):
        if "/tests" in f or f.endswith("tests.rs"):
            continue
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        for m in ENUM.finditer(text):
            name, body = m.group(1), m.group(2)
            if NOT_AUTHORED.search(name) or name.endswith(("Error", "Kind")):
                continue
            for vm in VARIANT.finditer(body):
                out.append((name, vm.group(1), f, vm.group(2)))
    return out


def main() -> int:
    corpus, n_files = authored_corpus(with_helpers=True)
    # ⛔ ANTI-VACUITY: a corpus that missed the content crates would report
    # almost everything as unnamed and read as a catastrophe. This field is
    # authored in Rust, so its absence means the corpus is broken, not the tree.
    # ⛔⛔ THE CALIBRATION PAIR IS ASYMMETRIC ON PURPOSE, and only one half can
    # be an assertion.
    #
    #   known FALSE dormant  — `forbidden_while_held` IS set, by an authoring
    #     helper. STABLE: a false positive is a defect in this census and stays
    #     fixed once fixed. This is the assertion below.
    #   known TRUE dormant   — PERISHABLE, and cannot be asserted on. A true
    #     dormant is by construction a thing somebody is about to author, so the
    #     moment the list is useful the point FLIPS. Three have flipped while
    #     this script was being written: `behind_nearest_foe` (authored by the
    #     Author's counter), `VolumeReaction::Windbox` (the Officer's neutral
    #     special, the first on the roster), and `close_on_transit` (implemented
    #     rather than authored). Each flip is the instrument working.
    # ⛔ A HELPER-ONLY MARKER. `forbidden_while_held` is NAMED in `game/` too, so
    # its presence proves nothing; the ASSIGNMENT only exists in the helper.
    if "spec.gates.forbidden_while_held = true" not in corpus:
        print(
            "authored corpus has lost the authoring helpers in "
            "`ambition_characters` — every gate they apply will be reported as "
            "unauthored, including engine RULES like the shark-ride recast lock.",
            file=sys.stderr,
        )
        return 1
    if "behind_nearest_foe" not in corpus:
        print(
            "authored corpus does not contain a field known to be authored in Rust "
            "(`behind_nearest_foe`) — the file selection is broken, and every "
            "'unnamed' below would be an artefact.",
            file=sys.stderr,
        )
        return 1

    head = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], cwd=REPO, capture_output=True, text=True
    ).stdout.strip()
    fields = modes()
    live, dormant, unnamed, always_on = [], [], [], []
    for name, field, where, defaults_true in fields:
        set_true = re.search(re.escape(field) + r"\s*[:=]\s*true\b", corpus)
        named = re.search(r"\b" + re.escape(field) + r"\b", corpus)
        if defaults_true and not set_true:
            # ⛔ NOT DORMANT — the behaviour is on everywhere by default.
            always_on.append((name, field, where))
        elif set_true:
            live.append((name, field, where))
        elif named:
            dormant.append((name, field, where))
        else:
            unnamed.append((name, field, where))

    print(f"AUTHORED BOOLEAN PARAMETER MODES  (at {head}, corpus {n_files} files)\n")
    print(f"  technique/placement bool modes      {len(fields)}")
    print(f"  LIVE      — set true in content     {len(live)}")
    print(f"  ALWAYS ON — defaults true, unset    {len(always_on)}")
    print(f"  DORMANT   — default false, never on {len(dormant)}")
    print(f"  UNNAMED   — never mentioned         {len(unnamed)}")
    for label, rows in (("DORMANT", dormant), ("UNNAMED", unnamed)):
        if not rows:
            continue
        print(f"\n{label}:")
        for name, field, where in sorted(rows):
            print(f"  {name}.{field}\n      {where}")
    print(
        "\nⓘ A dormant mode is not a defect. It is a built behaviour with no "
        "authored customer — decide whether to author one or to delete it."
    )

    if "--variants" in sys.argv:
        rows = variants()
        # ⛔ THE VARIANT AXIS USES THE NARROWER CORPUS — see `authored_corpus`.
        corpus, _ = authored_corpus(with_helpers=False)
        cold = [
            (e, v, f)
            for e, v, f, payload in rows
            if not variant_is_authored(v, payload, corpus)
        ]
        print(f"\n\nAUTHORED ENUM VARIANTS (the BRANCH axis)\n")
        print(f"  variants examined                   {len(rows)}")
        print(f"  never named in authored content     {len(cold)}")
        print(
            "\n⚠ WIDER AND NOISIER THAN THE FIELD AXIS. Many `Deserialize` enums are\n"
            "   runtime state rather than authored vocabulary, so this is a starting\n"
            "   list for a person, not a verdict. It exists because a field census\n"
            "   counts LEAVES: a whole reaction with no authored customer is invisible\n"
            "   to it unless something inside it happens to be a bool.\n"
        )
        for e, v, f in sorted(cold):
            print(f"  {e}::{v}\n      {f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
