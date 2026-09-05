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

import collections
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
#: toggles, per-frame input, and performance budgets.
#:
#: ⛔⛔ THIS USED TO ALSO EXCLUDE `Tuning` AND `AbilitySet`, AND THAT WAS WRONG --
#: it removed 34 of the modes this census exists to count, including the whole
#: capability vocabulary. MEASURED 2026-09-05 by classifying the excluded rows
#: instead of trusting the rule: `AbilitySet` is 28 LIVE authored capability
#: toggles (`jump`, `wall_climb`, `blink_through_hard_walls` -- the progression
#: vocabulary itself), and `BossMacroTuning.suppress_attacks_while_moving` is
#: authored content too. The word "Tuning" in a type name says nothing about
#: whether content authors it.
#:
#: ⚠ THE EXCLUSION WAS INVISIBLE, WHICH IS WHY IT SURVIVED. The census printed
#: its classifications and not its removals, so the 34 missing rows could only be
#: found by asking what the filter dropped. `EXCLUDED` now reports that, and the
#: rule kept here is the narrow one: a thing a PLAYER or a DEVELOPER sets, not a
#: thing an AUTHOR sets.
NOT_AUTHORED = re.compile(r"Settings|DeveloperTools|ControlFrame|Budget")

STRUCT = re.compile(
    r"#\[derive\([^)]*Deserialize[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*pub struct (\w+)\s*\{(.*?)\n\}",
    re.S,
)
BOOL_FIELD = re.compile(r"\n\s+pub (\w+):\s*bool\b")


def git(*args: str) -> list[str]:
    return subprocess.run(
        ["git", *args], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()


#: ⭐ WHAT THIS CENSUS THREW AWAY, counted so a reader can see it.
#:
#: ⛔⛔ THIS SCRIPT PUBLISHED A WRONG NUMBER TWICE, and both times the defect was
#: the POPULATION rather than the classification: a corpus missing the
#: `ambition_characters` authoring helpers invented two dormant modes, and a
#: polarity fault counted 16 always-on modes as dormant. A census whose subject
#: is the output of several silent filters cannot be audited by reading its
#: output, because the rows it dropped are exactly the rows not printed.
#:
#: ⇒ Every `continue` that removes a candidate increments a named bucket, and
#: the report prints them. This does not change a single classification -- it
#: makes the SHAPE of the corpus visible, so "is that number too small?" is a
#: question a reader can actually ask.
EXCLUDED: dict[str, int] = collections.defaultdict(int)


def _is_test(path: str) -> bool:
    """⛔⛔ THE EVIDENCE AND THE THING BEING PROVED MUST NOT SHARE A CORPUS.

    A test that sets a field proves nothing about authored content, and a test is
    often the LAST place a name survives after the real customer goes. ⇒ a field
    set only in `*_tests.rs` is DORMANT, and a corpus containing tests calls it
    LIVE — the exact inversion this census exists to avoid.

    ⚠ This bit only after the corpus widened to `ambition_characters` for the
    authoring helpers: that crate carries 60 test files which set gate fields
    constantly. The narrow `game/` corpus had almost none, so the bug was latent
    until the fix for a DIFFERENT false positive introduced it.
    """
    name = path.rsplit("/", 1)[-1]
    return (
        name == "tests.rs"
        or name.endswith("_tests.rs")
        or name == "test_support.rs"
        or "/tests/" in path
    )


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
        # ⛔ AUTHORED DATA IS AUTHORED WHEREVER IT SITS. This required
        # `game/`, and `platformer_defaults.ron` -- which authors
        # `AbilitySet.reset` -- lives under `crates/…/assets/`. A .ron in an
        # `assets/` directory is content by construction; the crate it is
        # packaged under is a build fact, not an authorship one.
        if (
            f.endswith((".ron", ".ldtk", ".yarn"))
            and (f.startswith("game/") or "/assets/" in f)
        )
        or (f.endswith(".rs") and f.startswith(roots) and not _is_test(f))
    ]
    text = "\n".join(
        (REPO / f).read_text(encoding="utf-8", errors="replace") for f in files
    )
    return text, len(files)


def set_true_in_production(field: str) -> str | None:
    """Where non-test PRODUCTION code sets `field` true, outside the authored corpus.

    ⛔⛔ THE HELPER CORPUS IS A HAND-LISTED GUESS AND IT WENT STALE TWICE. It named
    `crates/ambition_characters` because authoring helpers live there -- and then
    `QuestSpec::starting_at_boot()` shipped in `ambition_persistence`, setting
    `auto_start` for seven quests, and this census called the field DORMANT. The
    list was not wrong, it was INCOMPLETE, which is the failure mode every
    hand-kept list has.

    ⭐ SO STOP COMPLETING IT. A dormant verdict now asks the whole production tree
    whether anything sets the field, and reports WHERE. That turns a false
    "nobody wants this" into a true "no CONTENT sets it; this does" without the
    census having to know in advance where authoring helpers live.

    ⚠ Deliberately NOT folded into LIVE. "Content authored it" and "engine code
    sets it" are different facts and the difference is the whole point of the
    census -- a boss literal in `spawn_actors.rs` is not an authored customer.
    The row stays out of DORMANT and gets its own bucket, because a reader needs
    to see the location to judge which one it is.
    """
    # ⚠ NOT `git()`: it uses `check=True`, and `git grep` exits 1 for NO MATCH.
    # No-match is the common, expected answer here, not an error.
    done = subprocess.run(
        ["git", "grep", "-l", "-E",
         rf"\b{re.escape(field)}\s*[:=]\s*true\b", "--", "crates", "game"],
        cwd=REPO, capture_output=True, text=True,
    )
    if done.returncode not in (0, 1):
        raise SystemExit(f"git grep failed for {field}: {done.stderr.strip()}")
    live = [f for f in done.stdout.split() if not _is_test(f)]
    return live[0] if live else None


def set_from_expression(field: str) -> str | None:
    """Where production code assigns `field` from something that is not a literal.

    ⛔⛔ THE THIRD TIME THE PATTERN WAS THE POPULATION BUG. `Breakable.pogo_refresh`
    read UNNAMED -- never mentioned anywhere -- while SEVEN production sites read
    it and `ldtk/surfaces.rs` sets it with `breakable.pogo_refresh =
    pogo_orb_combo`. The census matched `field [:=] true`, so a field whose
    authored road runs through a VARIABLE was invisible to it, exactly as a field
    set by a helper in an unlisted crate had been.

    ⭐ A field assigned from an expression is a THIRD state, not a missing one:
    the authoring lives in whatever computes that expression -- here an LDtk
    entity identifier -- and the census cannot follow it. Saying "derived, look
    here" is the honest answer; saying "nobody authors this" was a false one.

    ⚠ `: false` and `: true` are excluded so this only catches the non-literal
    case; the literal roads already have their own buckets.
    """
    # ⛔⛔ FLAGS BEFORE THE PATTERN, and NEVER swallow a bad status. Written as
    # `["-E", pattern, "-P"]` git read `-P` as a REVISION and died with `fatal:
    # unable to resolve revision: -P` -- status 128, which an
    # `if returncode not in (0, 1): return None` turned into "no match" for every
    # field on the list. The bucket printed 0 and looked like a clean result.
    # A scanner that swallows a read error reports its own failure as a finding.
    done = subprocess.run(
        ["git", "grep", "-lP",
         rf"\b{re.escape(field)}\s*=\s*(?!true|false)[A-Za-z_]",
         "--", "crates", "game"],
        cwd=REPO, capture_output=True, text=True,
    )
    if done.returncode not in (0, 1):
        raise SystemExit(
            f"git grep failed for {field} (status {done.returncode}): "
            f"{done.stderr.strip()}"
        )
    hits = [f for f in done.stdout.split() if not _is_test(f)]
    if not hits:
        return None

    # ⛔⛔ A SAME-NAME THREAD IS NOT AUTHORING. `spawn_static.rs` writes
    # `collected: authored.payload.collected` -- that carries the spec's value to
    # the component and DECIDES nothing. Counting it as "derived, the authoring
    # is elsewhere" would quietly retire a genuinely unauthored field into a
    # bucket that sounds resolved, which is the same error as calling it live.
    #
    # ⇒ A row is DERIVED only if some assignment's right-hand side is something
    # OTHER than the field's own name. `pogo_refresh = pogo_orb_combo` qualifies;
    # `collected: authored.payload.collected` does not.
    shown = subprocess.run(
        ["git", "grep", "-hP", rf"\b{re.escape(field)}\s*=\s*(?!true|false)[A-Za-z_]",
         "--", "crates", "game"],
        cwd=REPO, capture_output=True, text=True,
    )
    for line in shown.stdout.splitlines():
        rhs = line.split("=", 1)[1] if "=" in line else ""
        tail = re.match(r"\s*([A-Za-z0-9_.]+)", rhs)
        if tail and not tail.group(1).split(".")[-1] == field:
            return hits[0]
    return None


def modes() -> list[tuple[str, str, str, bool]]:
    """`(struct, field, file, defaults_true)` for every authored bool mode.

    ⛔ `defaults_true` scans EVERY `impl` block on the type in its own file, not
    just `impl Default` — `Chest::new` sets `persistent: true` in a constructor,
    and a Default-only scan finds 1 of the 16 that actually default true.
    """
    out = []
    for f in git("grep", "-l", "Deserialize", "--", "crates"):
        if "/tests" in f or f.endswith("tests.rs"):
            EXCLUDED["test files"] += 1
            continue
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        impls: dict[str, str] = {}
        for im in re.finditer(r"\nimpl(?:\s+Default\s+for)?\s+(\w+)\b", text):
            impls[im.group(1)] = impls.get(im.group(1), "") + text[im.start() : im.start() + 3000]
        for m in STRUCT.finditer(text):
            name, body = m.group(1), m.group(2)
            if NOT_AUTHORED.search(name):
                if BOOL_FIELD.search(body):
                    EXCLUDED["structs ruled not-authored"] += 1
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
                EXCLUDED["enums ruled not-authored"] += 1
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
    live, dormant, unnamed, always_on, engine_set, derived = [], [], [], [], [], []
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

    # ⛔⛔ THESE ANNOTATE, THEY DO NOT RECLASSIFY, and the difference is the whole
    # lesson. Both scans match a field by NAME across the tree, and a bool field
    # name is not unique: `PropSpec.flip_y` collided with Bevy's `Sprite.flip_y`,
    # and `Pickup.collected` with `let collected = SINK.with(..)` in an unrelated
    # crate. Reclassifying on those hits SILENTLY RETIRED `PickupSpec.collected`
    # -- a field this repo has a written finding about -- out of the dormant list
    # and into a bucket whose name reads "resolved".
    #
    # ⭐ A filter whose mistakes all point at "nothing to see here" is worse than
    # no filter, because its errors are invisible by construction. So a row keeps
    # its verdict and CARRIES A POINTER for a person to check. Over-reporting
    # candidates is the safe direction for a census whose output is a list for a
    # human; under-reporting deletes findings.
    for bucket in (dormant, unnamed):
        for row in bucket:
            site = set_true_in_production(row[1])
            if site is not None:
                engine_set.append((row[0], row[1], site))
                continue
            site = set_from_expression(row[1])
            if site is not None:
                derived.append((row[0], row[1], site))

    print(f"AUTHORED BOOLEAN PARAMETER MODES  (at {head}, corpus {n_files} files)\n")
    print(f"  technique/placement bool modes      {len(fields)}")
    print(f"  LIVE      — set true in content     {len(live)}")
    print(f"  ALWAYS ON — defaults true, unset    {len(always_on)}")
    print(f"  DORMANT   — default false, never on {len(dormant)}")
    print(f"  UNNAMED   — never mentioned         {len(unnamed)}")
    print(
        f"\n  ⚠ {len(engine_set) + len(derived)} of those rows have their NAME "
        "written elsewhere in the tree.\n     Listed below as pointers, NOT as "
        "verdicts -- a bool field name is not unique,\n     so each one needs a "
        "person to say whether it is the same field."
    )
    if EXCLUDED:
        print("\n  EXCLUDED FROM THE SUBJECT CORPUS (not classified above):")
        for reason, count in sorted(EXCLUDED.items()):
            print(f"    {count:4d}  {reason}")
        print(
            "    ⚠ These are the rows this census cannot speak for. A number "
            "here that looks\n       too large is a reason to re-read the rule "
            "that removed them."
        )
    for label, rows in (("DORMANT", dormant), ("UNNAMED", unnamed), ("NAME ALSO SET TRUE ELSEWHERE", engine_set),
                        ("NAME ALSO ASSIGNED ELSEWHERE", derived)):
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
