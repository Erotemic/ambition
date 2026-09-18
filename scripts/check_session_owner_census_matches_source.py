#!/usr/bin/env python3
"""C03's session-owner census must still be the census SOURCE has.

⛔⛤ **IT DRIFTED BY FOUR AND NOBODY NOTICED UNTIL THE CAMPAIGN BECAME
STARTABLE.** `consolidation-plan.md` opened C03 with *"Source explicitly groups
32 App resources as gameplay-session or activated-generation state"* and named
`SessionScopedResources` (25). MEASURED 2026-09-16, field by field, it is 29 —
`StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks` and
`SessionMatchOrdinal` became members while the campaign waited on its gates. The
drift is not neglect; it is the campaign's own SUBJECT moving, which is what any
census pinned to a commit does. ⇒ A campaign whose starting census is four rows
stale starts by consolidating a set it has not enumerated.

⭐ **SO THE PLAN CARRIES A MACHINE-READABLE COPY AND THIS COMPARES IT TO
SOURCE.** The prose stays for readers; the HTML comment is the one the guard
reads. That is deliberately a SECOND copy of the number — the point is not to
avoid a second copy, which prose already was, but to make the second copy
CHECKABLE.

⚠ **IT COUNTS RESOURCES, NOT FIELDS, and the distinction is what the first
parser got wrong.** `SessionScopedResources` and `SessionOwnedCheckpointState`
are `SystemParam` bundles whose every field is one `ResMut<'w, T>` — one resource
each. `SessionMechanics` is ONE resource that happens to have six fields, and
counting its fields reads the total as 41 instead of 36.

⛔⛤ **RULE 3 — AND A COUNT IS NOT A CHECK ON A LIST.** The first two rules agree
that `SessionOwnedCheckpointState` has SIX members and would keep agreeing if all
six were replaced. The thing C03 step 3 actually needs before it moves storage is
the ROLLBACK PARTITION: which members are rollback-registered, under which key,
and which are deliberately host-side. Source's own doc block said *"Four of these
are canonical ROLLBACK state"*; MEASURED 2026-09-16 in `rollback_registration.rs`
it is FIVE, and the four was a count of the BULLETS below it — one of which pairs
a registered value with an unregistered one. ⇒ A prose list that happens to be
grouped differently from the mechanism reads as a count of the mechanism.

⭐ **THE EXEMPTION IS DERIVED, NOT HAND-LISTED.** A member may be unregistered
only if the doc block DECLARES it so, in the words `DELIBERATELY NOT REGISTERED`,
beside its name. An amnesty list inside this file would be the way to hide what
it exempts; making the declaration live in source means the code review that adds
a member is where the decision is recorded, and this guard only checks the two
agree. It fails BOTH ways: an unregistered member with no declaration, and a
declared-host-side member that is in fact registered.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
PLAN = REPO / "docs/planning/consolidation/consolidation-plan.md"
MARKER = re.compile(r"<!--\s*session-owner-census:\s*(.+?)\s*-->")

#: group name -> (file, how to count it)
BUNDLES = {
    "SessionScopedResources": "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    "SessionOwnedCheckpointState": "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
}
#: ⚠ NOT a bundle: one resource with six fields. Counting its fields is the
#: mistake this guard's docstring records, so it is asserted to EXIST and is
#: never counted by field.
SINGLETON = (
    "SessionMechanics",
    "crates/ambition_platformer2d_actor_monolith/src/session/mechanics.rs",
)

RESMUT_FIELD = re.compile(r"^\s{4}(?:pub\s+)?\w+:\s*ResMut<'w,\s*(.+?)>,\s*$", re.M)

CHECKPOINT = REPO / BUNDLES["SessionOwnedCheckpointState"]
#: ⛔⛔ THE WHOLE WORKSPACE, NOT THE CRATE THAT OWNS THE STRUCT. A member could
#: be registered from anywhere, and "it is not registered in the file I looked
#: in" is a claim about the query, not about the member. MEASURED 2026-09-16: no
#: checkpoint member registers outside the monolith's `rollback_registration.rs`.
#: The scan costs ~0.3 s.
REGISTRATION_ROOTS = ("crates", "game")

#: ⛔⛤ AND THE METHOD LIST IS DERIVED, BECAUSE THE FIRST VERSION NAMED EXACTLY
#: ONE OF TEN. It asked only about `rollback_resource_clone_checksum` while
#: reporting "NO rollback registration" — a verdict its query could not support.
#: `RollbackRegistrar` declares TEN `rollback_resource_*` methods, and a value
#: registered through any of them is registered. The verdict for the checkpoint
#: family did not change (all five use `clone_checksum`), but it was right by
#: luck: `PendingLifecycleCommit`, one resource over, is documented as
#: rollback-registered and does not appear under that one name.
#:
#: ⚠ Every one of them takes `(owner, name, ...)` as its first two arguments, so
#: one call-site shape covers all ten.
REGISTRAR_TRAIT = REPO / "crates/ambition_platformer2d_core/src/snapshot.rs"
RESOURCE_METHOD = re.compile(r"fn (rollback_resource_\w+)<T>")


COMMENT = re.compile(r"^\s*(?://|///|//!).*$", re.M)


def code_only(body: str) -> str:
    """Source with line comments removed.

    ⛔⛤ **REGION FIRST, NOT PROSE-RECOGNITION.** `teardown.rs` explains a
    resource by naming its registration in a doc comment —
    *"`rollback_resource_canonical::<ProjectileSeqCounter>`), so its value is
    inside the state checksum"* — and that mention is a raw occurrence that is
    not a call. Trying to recognise the prose is the rule backwards; deleting the
    comment REGION first leaves only code to classify.
    """
    return COMMENT.sub("", body)


#: ⚠ `rollback_resource_map_entities` registers ENTITY REMAPPING for a resource,
#: not the resource's state, and a value legitimately has BOTH — four do here
#: (`PossessionState`, `EncounterRegistry`, `ActiveConversation`,
#: `PendingPlayerHitEvents`), under `resource.x` and `map.resource.x`. Reading the
#: second as a competing state registration produced four false reds the moment
#: the method list widened. Two registrations of different KINDS are not two
#: authorities.
MAP_METHOD = "map_entities"


def registration_methods() -> list[str]:
    names = sorted(
        set(RESOURCE_METHOD.findall(REGISTRAR_TRAIT.read_text(encoding="utf-8")))
    )
    if len(names) < 5:
        raise SystemExit(
            f"⛔⛔ only {len(names)} `rollback_resource_*` method(s) parsed from "
            f"{REGISTRAR_TRAIT.name}. A guard that derives its query from a trait "
            "must refuse when the trait stops parsing, not narrow silently."
        )
    return names
#: A registration CALL SITE: the generic names a concrete type, the owner is an
#: identifier OR a string literal, and the next argument is the rollback KEY.
#: ⚠ Both owner spellings occur in this tree (`OWNER`, `GATE_PORTAL_ROLLBACK_OWNER`,
#: `"test"`, `"ambition_demo_sanic"`), and the first version of this pattern
#: accepted only the identifier form, so it silently parsed 17 of 23.
def call_site_pattern(methods: list[str]) -> re.Pattern[str]:
    return re.compile(
        r"(?:" + "|".join(methods) + r")::<\s*([\w:]+)\s*>\s*\(\s*"
        r'(?:\w+|"[^"]*")\s*,\s*"([^"]+)"'
    )
#: The trait's own forwarding definition — `::<T>(owner, name, detail, checksum)`,
#: every argument an identifier and the generic a type PARAMETER. It is plumbing,
#: not a registration, and it must be ACCOUNTED FOR rather than silently dropped:
#: a scan that just ignores what it cannot parse reports its own blind spot as a
#: clean tree.
def forwarding_pattern(methods: list[str]) -> re.Pattern[str]:
    return re.compile(
        r"(?:" + "|".join(methods) + r")::<\s*\w+\s*>\s*\(\s*\w+\s*,\s*\w+\s*[,)]"
    )
#: The words source must use to claim a member is host-side. See RULE 3.
HOST_SIDE_PHRASE = "DELIBERATELY NOT REGISTERED"


def workspace_registrations() -> tuple[dict[str, str], list[str]]:
    """Every `(type, rollback key)` pair in the workspace, and any accounting gap.

    ⛔ The return is a PAIR because the accounting is the anti-vacuity check: a
    member reading as unregistered is only evidence if every raw occurrence was
    classified.
    """
    methods = registration_methods()
    state_methods = [m for m in methods if MAP_METHOD not in m]
    raw_calls = [f"{method}::<" for method in methods]
    call_site = call_site_pattern(state_methods)
    map_site = call_site_pattern([m for m in methods if MAP_METHOD in m])
    forwarding_re = forwarding_pattern(methods)

    pairs: list[tuple[str, str]] = []
    raw = forwarding = maps = 0
    for root in REGISTRATION_ROOTS:
        for path in sorted((REPO / root).rglob("*.rs")):
            body = code_only(path.read_text(encoding="utf-8", errors="replace"))
            hits = sum(body.count(call) for call in raw_calls)
            if not hits:
                continue
            raw += hits
            forwarding += len(forwarding_re.findall(body))
            maps += len(map_site.findall(body))
            pairs.extend(
                (generic.rsplit("::", 1)[-1], key)
                for generic, key in call_site.findall(body)
            )

    findings = []
    if len(pairs) + forwarding + maps != raw:
        findings.append(
            f"  RULE 3 classified {len(pairs)} state call site(s) + {maps} "
            f"entity-mapping call site(s) + {forwarding} forwarding definition(s) "
            f"out of {raw} raw occurrence(s) across "
            f"{len(methods)} `rollback_resource_*` method(s). "
            f"{raw - len(pairs) - forwarding - maps} are unaccounted for, so a member could "
            "read as unregistered because of THIS SCANNER."
        )
    registrations: dict[str, str] = {}
    for name, key in pairs:
        if name in registrations and registrations[name] != key:
            findings.append(
                f"  two different rollback keys register a type spelled {name}: "
                f"`{registrations[name]}` and `{key}`. RULE 3 keys by the type's "
                "SHORT name, which is no longer an identity here."
            )
        registrations[name] = key
    return registrations, findings


def checkpoint_partition() -> list[str]:
    """RULE 3: every member registers, or source declares why it does not."""
    text = CHECKPOINT.read_text(encoding="utf-8")
    start = text.index("pub struct SessionOwnedCheckpointState")
    members = RESMUT_FIELD.findall(text[start : text.index("\n}\n", start)])

    registrations, findings = workspace_registrations()
    if findings:
        return findings
    # ⛔ ANTI-VACUITY on the other side: an unparsed member list exempts every
    # member at once.
    if len(members) < 3:
        return [
            f"  RULE 3 parsed {len(members)} member(s) of "
            "SessionOwnedCheckpointState; the field scan is broken"
        ]

    #: The doc block above the struct is where a host-side claim must live.
    doc = text[max(0, start - 4000) : start]
    for member in members:
        member = member.rsplit("::", 1)[-1]
        declared_host_side = any(
            member in line and HOST_SIDE_PHRASE in window
            for line, window in _doc_windows(doc)
        )
        key = registrations.get(member)
        if key and declared_host_side:
            findings.append(
                f"  {member} is registered as `{key}` AND the doc block calls it "
                f"{HOST_SIDE_PHRASE}. Source contradicts itself."
            )
        elif not key and not declared_host_side:
            findings.append(
                f"  {member} is a session-owned checkpoint value with NO rollback "
                "registration and no declaration that its absence is deliberate.\n"
                "     Register it, or say why not beside it — C03 step 3 cannot "
                "record a boundary nobody wrote down."
            )
        elif key and key not in doc:
            findings.append(
                f"  {member} registers as `{key}`, which the doc block above the "
                "struct does not name. The partition a reader sees is not the one "
                "the registrar has."
            )
    return findings


def _doc_windows(doc: str) -> list[tuple[str, str]]:
    """Each doc line paired with the paragraph it sits in.

    ⚠ A host-side claim is a SENTENCE, and a sentence wraps across `///` lines,
    so a per-line test would only ever see the phrase beside the name when the
    author happened to fit both on one line.
    """
    out = []
    for para in doc.split("///\n"):
        for line in para.split("\n"):
            out.append((line, para))
    return out


CENSUS = REPO / "docs/planning/consolidation/architecture-census.md"
#: A backtick-fenced run of `Name, Name, Name` on one line — how the census
#: spells each bundle's membership.
NAME_LIST = re.compile(r"`([A-Z]\w+(?:,\s*[A-Z]\w+){4,})`")


def member_lists() -> list[str]:
    """RULE 4: the census's NAME LISTS must equal source's members, not just count.

    ⛔⛤ **A COUNT IS NOT A CHECK ON A LIST, AND THIS DOCUMENT PROVED IT TWICE IN
    ONE DAY.** §3 carried a 25-name list beside a stale 25; correcting the count
    alone would have left a list missing four members and passed every rule this
    guard had. And once the count is right, a RENAMED member keeps the count at 29
    forever.

    ⚠ The rule is EQUALITY of sets, and it names what is missing and what is
    extra, because "the list is wrong" sends a reader to re-derive 29 rows.
    """
    text = CENSUS.read_text(encoding="utf-8")
    findings = []
    checked = 0
    for name, rel in BUNDLES.items():
        source = set(source_members(rel, name))
        if len(source) < 3:
            findings.append(
                f"  RULE 4 read {len(source)} member(s) of {name} from source; "
                "the field scan is broken, not the census"
            )
            continue
        for listed in NAME_LIST.findall(text):
            members = {part.strip() for part in listed.split(",")}
            # A list is THIS bundle's when it overlaps it more than half — the
            # census spells no bundle name on the list's own line.
            if len(members & source) * 2 <= len(source):
                continue
            checked += 1
            missing = sorted(source - members)
            extra = sorted(members - source)
            if missing or extra:
                findings.append(
                    f"  {name}'s name list in {CENSUS.name} does not match source: "
                    + (f"missing {', '.join(missing)}" if missing else "")
                    + ("; " if missing and extra else "")
                    + (f"not in source: {', '.join(extra)}" if extra else "")
                )
    # ⛔ ANTI-VACUITY. A census whose lists stopped being backtick-fenced matches
    # nothing and this rule certifies a document it never read.
    if not findings and checked < len(BUNDLES):
        findings.append(
            f"  RULE 4 found {checked} member list(s) in {CENSUS.name} for "
            f"{len(BUNDLES)} bundle(s). A list it cannot find is a list it cannot "
            "check, and this rule must refuse rather than report clean."
        )
    return findings


def source_members(rel: str, name: str) -> list[str]:
    text = (REPO / rel).read_text(encoding="utf-8")
    start = text.index(f"pub struct {name}")
    end = text.index("\n}\n", start)
    return [m.rsplit("::", 1)[-1] for m in RESMUT_FIELD.findall(text[start:end])]


def declared() -> dict[str, int]:
    match = MARKER.search(PLAN.read_text(encoding="utf-8"))
    if not match:
        raise SystemExit(
            "⛔⛔ the plan carries no `session-owner-census` marker. A guard that "
            "cannot find its subject must refuse, not report clean."
        )
    return {
        name: int(value)
        for name, value in (pair.split("=") for pair in match.group(1).split())
    }


def bundle_members(rel: str, name: str) -> int:
    text = (REPO / rel).read_text(encoding="utf-8")
    start = text.index(f"pub struct {name}")
    end = text.index("\n}\n", start)
    return len(RESMUT_FIELD.findall(text[start:end]))


PLANNING = REPO / "docs/planning"
#: The bundle whose count is restated across the planning corpus.
BUNDLE = "SessionScopedResources"
#: ⛔⛤ THE MARKER IS NOT THE ONLY COPY. When this guard was written the plan
#: carried the count in prose AND in a marker, and `architecture-census.md`
#: carried a THIRD copy in a table cell — still 25 after the other two were
#: corrected to 29. A rule that checks one known copy cannot find the copy nobody
#: remembered.
#:
#: ⚠ **AND THE FIRST VERSION OF THIS RULE FIRED ON A CORRECT LINE.** It accepted
#: any number within 80 characters of "App resources" on a line mentioning the
#: bundle, so it flagged *"36 process/App resources are explicitly documented…
#: (SessionScopedResources …)"* — where 36 is the TOTAL across three groupings
#: and is right. A false RED is obeyed faster than a false green is questioned,
#: so this matches only the forms that state the BUNDLE'S OWN count.
COUNT_FORMS = [
    re.compile(rf"SessionScopedResources[^\n]{{0,12}}?\((\*\*)?(\d{{1,3}})"),
    # ⛔⛤ THE BACKTICK IS WHY THIS RULE MISSED A WHOLE STALE SECTION.
    # `architecture-census.md` §3 read "`SessionScopedResources` names **25**
    # process resources" — with the CLOSING BACKTICK between the name and the
    # verb — while the same document's executive map had already been corrected
    # to 36. One document, two numbers, and the rule that exists to find exactly
    # that could not see past one character.
    re.compile(
        r"SessionScopedResources`?\s+(?:names|holds|has)\s+(?:\*\*)?(\d{1,3})"
    ),
    re.compile(r"(?:\*\*)?(\d{1,3})(?:\*\*)?[^|\n]{0,60}?accessed through one SystemParam"),
    # ⛔⛤ **AND THE RULE STILL MISSED SIX RESTATEMENTS, FOUND 2026-09-17 BY
    # READING THE PAGE RATHER THAN TRUSTING THE GREEN.** `consolidation-plan.md`
    # said *"All 29 members have at least one reader"*, *"22 OF
    # `SessionScopedResources`' 29"*, *"29 + 6 exhaustively destructured
    # fields"*, *"22 of the 29 are rollback-registered"*, and
    # `consolidation/README.md` said *"`SessionScopedResources` is 29, not 25"*
    # — every one of them while this guard reported the census matching source at
    # 30. The forms above wanted a bracket, or one of three verbs, or a
    # SystemParam clause.
    #
    # ⇒ Two shapes cover all of them: the count IMMEDIATELY before the name, and
    # the name then a copula. ⚠ The gap in the first is `\s+` and not `.{0,12}`
    # ON PURPOSE: *"22 OF `SessionScopedResources`' 30"* states a NUMERATOR
    # before the name, and a looser gap reads it as the bundle's own count and
    # fires on a correct line. A false red is obeyed faster than a false green is
    # questioned.
    re.compile(r"(\d{1,3})\s+`?SessionScopedResources`?\s+members"),
    re.compile(r"SessionScopedResources`?\s+(?:is|are)\s+(?:\*\*)?(\d{1,3})"),
]

#: The TOTAL across the three groupings — the number C03's prose quotes as its
#: starting population.
#:
#: ⛔⛤ **IT WENT STALE THE SAME WAY, ONE INCREMENT BEHIND, AND NOTHING WATCHED
#: IT.** MEASURED 2026-09-17: the plan said *"groups **36** App resources"*,
#: quoted the census helper as *"semantics: 36"* and warned *"do not begin by
#: moving all 36 values"*, while the marker plus source said 30 + 6 + 1 = 37.
#: The bundle's own count had a rule and the SUM of the three did not.
#:
#: ⚠ **AN ALLOWLIST OF LIVE FORMS, NOT A NUMBER HUNT.** The first version of the
#: bundle rule above fired on a CORRECT total because it accepted any number near
#: "App resources"; this corpus also records what a row USED to say ("C03 starts
#: from 32 session-owned App resources") and correcting that would destroy the
#: evidence. So these are the four spellings in which the total is asserted as
#: CURRENT, and a fifth spelling is invisible until someone adds it here.
TOTAL_FORMS = [
    re.compile(r"groups \*\*(\d{1,3})\*\* App resources"),
    re.compile(r"session/generation semantics: (\d{1,3})"),
    re.compile(r"moving all (\d{1,3}) values"),
    re.compile(r"\((\d{1,3}) App resources\)"),
    # ⛔⛤ **THE FIFTH AND SIXTH SPELLINGS, AND THE COMMENT ABOVE PREDICTED THEM
    # — 2026-09-18.** `architecture-census.md` stated the total twice, in forms
    # none of the four above matched: its executive map opened *"**36**
    # process/App resources are explicitly documented by source as session- or
    # generation-owned"* and section 3 closed *"The unique total is **36**"* —
    # both directly above the three lists, which summed to 37. This guard
    # PASSED, because the bundle's own count and all three NAME LISTS were
    # right; only the prose sum was a member behind. ⇒ Two spellings added.
    # ⚠ Both anchor on words the sentence cannot lose without being rewritten
    # ("process/App resources", "unique total"), so neither can drift into
    # matching a numerator the way the bundle rule once did.
    re.compile(r"\*\*(\d{1,3})\*\* process/App resources are"),
    re.compile(r"unique total is \*\*(\d{1,3})\*\*"),
]


#: ⛔⛤ THE SWEEP READ ONLY MARKDOWN, AND THE LEDGER IS JSON. `consolidation-ledger.json`
#: held the stale count in THREE fields — `current_truth`, `evidence[0].claim` and
#: `representation` — for the same day the census prose did, invisible to a rule
#: whose glob was `*.md`. ⇒ Ask how many RECORDERS a fact has before deciding a
#: rule's population: this one had four, and the count was corrected in two of them.
#:
#: ⚠ `static_measurement_snapshot` in that file is DELIBERATELY not swept. It
#: names its own `source_commit` and is a dated measurement, not a live claim —
#: correcting it would destroy the evidence it exists to be.
SWEPT_SUFFIXES = ("*.md", "*.json")


def stray_counts(real: int) -> list[str]:
    """Every line stating the BUNDLE'S OWN count as something other than `real`."""
    out = []
    paths = sorted(
        path for suffix in SWEPT_SUFFIXES for path in PLANNING.rglob(suffix)
    )
    for path in paths:
        for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            if BUNDLE not in line:
                continue
            for form in COUNT_FORMS:
                for match in form.finditer(line):
                    value = int(match.groups()[-1])
                    if value != real:
                        out.append(
                            f"  {path.relative_to(REPO)}:{n} states the bundle holds "
                            f"{value}; source has {real}\n     {line.strip()[:100]}"
                        )
    return out


def stray_totals(real: int) -> list[str]:
    """Every LIVE statement of the three-grouping total that is not `real`."""
    out, seen = [], 0
    paths = sorted(path for suffix in SWEPT_SUFFIXES for path in PLANNING.rglob(suffix))
    for path in paths:
        for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            for form in TOTAL_FORMS:
                for match in form.finditer(line):
                    seen += 1
                    value = int(match.group(1))
                    if value != real:
                        out.append(
                            f"  {path.relative_to(REPO)}:{n} states the session-owner "
                            f"TOTAL as {value}; the marker plus source give {real}\n"
                            f"     {line.strip()[:100]}"
                        )
    # ⛔ ANTI-VACUITY. Every form here is a sentence someone can rephrase, and a
    # rule that matches nothing agrees with everything.
    if seen < 2:
        out.append(
            f"  only {seen} live statement(s) of the session-owner total parsed out of "
            f"{len(paths)} planning file(s). The prose has been rephrased and "
            "`TOTAL_FORMS` no longer describes it, so this rule is not watching "
            "anything."
        )
    return out


def main() -> int:
    stated = declared()
    findings = []

    for name, rel in BUNDLES.items():
        if name not in stated:
            findings.append(f"  the marker does not state a count for {name}")
            continue
        real = bundle_members(rel, name)
        # ⛔ ANTI-VACUITY. A struct whose fields stopped matching the pattern
        # reads as zero, and zero would quietly "disagree" forever or, if the
        # marker were ever 0, agree with nothing.
        if real < 3:
            findings.append(
                f"  {name} parsed as {real} `ResMut` field(s) in {rel}; the "
                "scan is broken, not the census"
            )
            continue
        if real != stated[name]:
            findings.append(
                f"  {name}: the plan says {stated[name]}, {rel} has {real}"
            )

    real_bundle = bundle_members(BUNDLES[BUNDLE], BUNDLE)
    findings.extend(stray_counts(real_bundle))
    findings.extend(stray_totals(sum(stated.values())))
    findings.extend(checkpoint_partition())
    findings.extend(member_lists())

    name, rel = SINGLETON
    if f"pub struct {name}" not in (REPO / rel).read_text(encoding="utf-8"):
        findings.append(f"  {name} is not declared in {rel} any more")
    elif stated.get(name) != 1:
        findings.append(
            f"  {name} is ONE resource; the plan says {stated.get(name)}. "
            "Counting its FIELDS is the error this guard exists to prevent."
        )

    if findings:
        print("⛔ C03's session-owner census no longer matches source:\n")
        print("\n".join(findings))
        print(
            "\n⇒ RE-DERIVE THE CENSUS AND UPDATE BOTH THE MARKER AND THE PROSE.\n"
            "  A campaign whose starting census is stale starts by consolidating a\n"
            "  set it has not enumerated — and the drift is usually the campaign's\n"
            "  own subject moving while it waited on its gates."
        )
        return 1

    total = sum(stated.values())
    print(
        f"C03's session-owner census matches source: "
        + ", ".join(f"{k}={v}" for k, v in sorted(stated.items()))
        + f" ({total} App resources)"
    )
    print(
        "  and RULE 3: every SessionOwnedCheckpointState member either registers "
        "for rollback under a key the doc block names, or source declares its "
        "absence deliberate."
    )
    print(
        "  and RULE 4: the census's member NAME LISTS equal source's members, "
        "not merely their count."
    )
    print(
        f"  and RULE 5: every live restatement of the bundle's count and of the "
        f"{total}-resource total, across the planning corpus, agrees with source."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
