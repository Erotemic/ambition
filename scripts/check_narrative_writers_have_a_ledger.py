#!/usr/bin/env python3
"""Every `NarrativeInputWriter<M>` needs an installed `NarrativeInputPlugin<M>`.

⛔⛤ **THE SHIPPED DEFECT THIS EXISTS FOR, FOUND IN REVIEW 2026-09-18.**
`cmd_watch_cut_rope_video` (`game/ambition_content/src/bosses/yarn.rs`) takes a
`NarrativeInputWriter<SetFlagRequested>` and is bound to the authored Yarn
command `<<watch_cut_rope_video>>`, which the shipped dialogue invokes. No
`NarrativeInputPlugin<SetFlagRequested>` was installed anywhere in the tree.

`NarrativeInputWriter`'s `ledger` field is a plain
`ResMut<'w, NarrativeInputLedger<M>>` — not an `Option`, deliberately, while
the `tick` field beside it IS an `Option` and says why. So a payload with no
plugin cannot resolve its parameters, and the authored command recorded
nothing.

⚠ **THE MESSAGE CHANNEL WAS NEVER THE MISSING HALF, WHICH IS WHY IT HID FOR SO
LONG.** `SetFlagRequested` is registered with `add_message` by the engine's own
sim resources, so every "is this message registered?" reading answered YES. The
LEDGER resource is created only by `NarrativeInputPlugin`, and nothing created
it. Two halves, one of them satisfied, and the satisfied one is the one a
reader checks.

⇒ **AND THE Q136 CENSUS WAS TAUGHT TO LOOK AWAY.** `NarrativeInputWriter` is
deliberately not counted as a host `MessageWriter` crossing by
`check_host_produced_sim_consumed_requests.py`, on the grounds that the ledger
is the safe ingress. That is right about the MECHANISM and says nothing about
whether the mechanism was installed, so the exemption needs this pairing beside
it or it is an exemption with an unchecked premise.

⚠ **PROSE IS NOT A USE.** Comments are stripped before scanning, by the same
owner the other source scans use. The comment this repair left beside the fixed
registration names the writer in text, and without stripping it read as a
second call site.

⚠ **MATCHED ON THE LEAF TYPE NAME, AND THE PREMISE OF THAT MATCH IS CHECKED
RATHER THAN THE SPELLINGS.** It has to be a leaf match: the writer says
`NarrativeInputWriter<SetFlagRequested>` from a `use`, and the plugin says
`NarrativeInputPlugin::<ambition_combat::SetFlagRequested>`. Leaf is the only
comparison both spellings support. What makes that sound is that the leaf names
exactly ONE type in this workspace, so every rule below pairs a leaf only after
checking there is exactly one `struct`/`enum` declaring it.

⛔⛤ **AND COMPARING MODULE PATHS INSTEAD IS WRONG, WHICH THIS CHECK LEARNED BY
DOING IT (replaced 2026-09-19 after review).** The first rule here compared the
QUALIFIED spellings of a leaf and reported a collision when they disagreed. It
has a hole and a false positive, and they are the same fact seen twice:

  - **The hole.** A bare `NarrativeInputWriter<Foo>` carries no module path, so
    it was excluded from the comparison entirely and would pair happily with a
    plugin for a DIFFERENT qualified `Foo`. Resolving the bare name through the
    file's `use` map closes that.
  - **The false positive it then causes.** Resolved that way,
    `SpawnActorRequest` reads `ambition_platformer2d::actor` at the writer and
    `ambition_platformer2d_actor_spawn` at the plugin. Two paths, ONE type — the
    facade re-exports it. Three of the ten payloads disagree on path this way
    (the others are `super::`-relative), so the strict rule reds on re-exports.

⇒ A re-export multiplies PATHS and not DEFINITIONS, so counting declarations
answers the question the paths cannot. Measured 2026-09-19: all ten payload
leaves have exactly one declaration in shipped source.

⚠ **WHAT THIS STILL CANNOT SEE.** A leaf declared OUTSIDE this workspace is
invisible to a declaration count, so a bare spelling resolving to an external
type that shares a leaf with a workspace type would still pair wrongly. No
payload does today — every one of the ten resolves to a declaration here — and
a leaf with NO declaration is reported rather than passed, which is the shape
that case would arrive in.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts" / "lib"))

from test_paths import file_is_test_only, is_test_path, strip_test_modules  # noqa: E402

sys.path.insert(0, str(REPO / "scripts"))
# ⚠ IMPORTED, NOT RESPELLED. One owner for "what is code and what is prose",
# which handles nested `/* */` and every raw-string form.
from a_rollback_arm_must_refuse_a_frozen_world import code_only  # noqa: E402

#: The generic argument, across line breaks — every shipped spelling wraps.
WRITER = re.compile(r"NarrativeInputWriter\s*<\s*([^,>]+?)\s*(?:,|>)", re.DOTALL)
PLUGIN = re.compile(r"NarrativeInputPlugin\s*::\s*<\s*([^,>]+?)\s*(?:,|>)", re.DOTALL)

#: ⛔ ANTI-VACUITY. Every rule here reports by staying silent, and a regex that
#: stopped matching would pair an empty set against an empty set and pass.
#: MEASURED 2026-09-18: 10 writer payloads, 9 plugin payloads.
MIN_WRITERS = 6
MIN_PLUGINS = 6


def leaf(ty: str) -> str:
    return ty.strip().rsplit("::", 1)[-1].strip()


def declaration_sites(name: str, sources) -> list[str]:
    """Every shipped `struct`/`enum` declaring this leaf name.

    ⚠ A re-export is not a declaration, which is the whole reason this is the
    rule: `ambition_platformer2d::actor::SpawnActorRequest` and
    `ambition_platformer2d_actor_spawn::SpawnActorRequest` are two paths to the
    ONE `struct` counted here.
    """
    decl = re.compile(rf"\b(?:struct|enum)\s+{re.escape(name)}\b")
    return [
        f"{path.relative_to(REPO)}:{body[: m.start()].count(chr(10)) + 1}"
        for path, body in sources
        for m in decl.finditer(body)
    ]


def production_sources() -> list[tuple[pathlib.Path, str]]:
    """Shipped Rust only: test files dropped, `#[cfg(test)]` modules stripped.

    ⚠ A writer inside a test builds its own `App` and installs what it needs
    there; requiring the shipped composition to carry it would be wrong.
    """
    out = []
    for rel in (
        REPO.joinpath(d) for d in ("crates", "game")
    ):
        for path in sorted(rel.rglob("*.rs")):
            raw = path.read_text(encoding="utf-8", errors="ignore")
            if is_test_path(path) or file_is_test_only(raw):
                continue
            # ⛔⛤ COMMENTS STRIPPED, AND THIS GUARD'S OWN FIRST POISON RUN IS
            # WHY. The comment added beside the missing registration names
            # `NarrativeInputWriter<SetFlagRequested>` in prose, and the scan
            # reported it as a second USE SITE — so a page explaining a defect
            # would have become a requirement to fix it again.
            out.append((path, code_only(strip_test_modules(raw))))
    return out


def scan(sources) -> tuple[dict[str, list], dict[str, list]]:
    writers: dict[str, list] = {}
    plugins: dict[str, list] = {}
    for path, body in sources:
        for pattern, sink in ((WRITER, writers), (PLUGIN, plugins)):
            for match in pattern.finditer(body):
                ty = match.group(1).strip()
                # The declaration and its impl block are not uses.
                if ty.startswith("'") or ty in {"M", "'w, M: Message + Clone"}:
                    continue
                line = body[: match.start()].count("\n") + 1
                sink.setdefault(leaf(ty), []).append(
                    f"{path.relative_to(REPO)}:{line}"
                )
    return writers, plugins


def main() -> int:
    sources = production_sources()
    writers, plugins = scan(sources)

    if len(writers) < MIN_WRITERS or len(plugins) < MIN_PLUGINS:
        print(
            f"⛔⛔ parsed {len(writers)} writer payload(s) and {len(plugins)} plugin "
            f"payload(s), below the floors of {MIN_WRITERS}/{MIN_PLUGINS}. That is a "
            "claim about this scan, not about the tree."
        )
        return 1

    bad = []
    for name, uses in sorted(writers.items()):
        if name not in plugins:
            where = ", ".join(uses)
            bad.append(
                f"`NarrativeInputWriter<{name}>` is used at {where} and no "
                f"`NarrativeInputPlugin<{name}>` is installed anywhere, so its "
                "`ResMut<NarrativeInputLedger<_>>` cannot resolve and the write is lost"
            )

    # ⛔ THE LEAF MATCH'S PREMISE, CHECKED RATHER THAN ASSUMED. Pairing by leaf
    # is sound exactly while the leaf names one type; a second declaration makes
    # every pairing above a coincidence, and none makes it uncheckable.
    for name in sorted(set(writers) & set(plugins)):
        decls = declaration_sites(name, sources)
        if len(decls) > 1:
            bad.append(
                f"`{name}` is declared {len(decls)} times in shipped source "
                f"({', '.join(decls)}), so pairing a writer to a plugin by leaf "
                "name is a coincidence rather than a match"
            )
        elif not decls:
            bad.append(
                f"`{name}` is paired by leaf name but no shipped `struct`/`enum` "
                "declares it, so the premise of that pairing cannot be checked here"
            )

    if bad:
        print("narrative writers without a ledger:")
        for line in bad:
            print(f"  {line}")
        return 1

    unused = sorted(set(plugins) - set(writers))
    print(
        f"ok: {len(writers)} narrative writer payload(s) in shipped source, each with an "
        f"installed NarrativeInputPlugin ({len(plugins)} installed)"
    )
    if unused:
        print(f"  installed with no shipped writer (not a defect): {', '.join(unused)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
