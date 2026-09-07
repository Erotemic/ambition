#!/usr/bin/env python3
"""Split "foreign system installations" into REDUCIBLE and IRREDUCIBLE.

⭐ WHY THIS EXISTS. `measure_foreign_system_ordering.py` reports one number for
"INSTALLING a foreign system (the broader question)". Read alone it invites the
conclusion that every one of them is a carve waiting to happen. It is not, and the
distinction is structural rather than a matter of effort:

  REDUCIBLE   the block names systems/sets from exactly ONE capability (plus shared
              vocabulary every crate already depends on). That capability can install
              itself, and the composition names one function instead of N paths.

  IRREDUCIBLE the composition is the only place that can express it, for any of FOUR
              reasons — each one measured against a real block rather than imagined:
                · it names two capabilities that do not depend on each other
                  (`projectile_visuals` orders `ambition_render` systems `.after` a
                  `ambition_platformer2d_runtime` set; render does not depend on runtime);
                · the owner cannot NAME its anchors, shared vocabulary included
                  (`ambition_characters` uses `GameplaySimulationRoot` and does not depend
                  on `shared_tangle`);
                · it installs the HOST'S OWN system and merely orders it against a foreign
                  set (`crate::portal::tag_portal_camera_continuity_camera`);
                · it PLACES a capability's published set inside the host's phase
                  vocabulary (`EncounterLifecycleSet` between two `ProgressionSet`
                  phases; mount's four stages `.in_set(CombatSet::Settle)`). The
                  capability orders its own stages; only the composition knows where the
                  whole thing sits.

⛔⛔ AND THE OBVIOUS IMPLEMENTATION IS WRONG, which is why this is a script and not a
grep. Classifying a block by the `crate::path::` prefixes written in it MISSES every
foreign system imported with `use` and named bare — a scanner doing that called two
blocks "pure input" while they contained the host's own
`sync_primary_recipe_from_settings` and `declare_gameplay_input_context`. This resolves
the file's `use` statements first and attributes bare names through them.

⚠ WHAT "REDUCIBLE" REQUIRES, both halves — the second was missing at first and made the
count wrong in the safe-looking direction:

  1. every system the block INSTALLS belongs to one capability (ordering combinators are
     stripped first, because a block can install the HOST'S OWN system and merely order
     it against a foreign set — that is the composition's job and cannot move);
  2. that capability can NAME everything the block orders against, i.e. its manifest
     depends on each anchor's crate. Measured counter-example: `projectile_visuals`
     installs `ambition_render` systems `.after` an `ambition_platformer2d_runtime` set
     and render does NOT depend on runtime, so carving it would invert a dependency.

✔ NAMING shared vocabulary is free; INSTALLING a shared crate's SYSTEMS is not, and the
two are now separated. A block may name `shared_tangle`'s sets and run-conditions freely
— every capability depends on it — but one that REGISTERS its systems is installing
another crate's code, which the owner cannot take with it. The camera block
(`camera_ease::tick_camera_shake` + `tick_finish_zoom` beside render's `camera_follow`)
read as single-capability until that distinction existed.

✔ **RE-EXPORTS ARE RESOLVED** (see [`reexport_map`]), and doing so moved the totals from
24/20 to **18/26** — six blocks that looked carveable belong to a crate the host only
mentions through a facade. `ambition_platformer2d_runtime::host_input` is the case that
forced it: a re-export of systems defined in `ambition_platformer2d_actor_monolith`,
which is another session's lane, so the mislabel was a lane violation waiting to happen
rather than a cosmetic one.

⚠ Cross-check a target against `scripts/measure_foreign_system_ordering.py` anyway; its
`defining_crate()` is the more thorough resolver and its author reports it dropped 31
false rows when added.

⚠ IT IS A REPORT, NOT A GATE. The reducible count is an upper bound on easy carves, not
a promise: a block can be single-capability and still be entangled by a `.chain()` that
crosses a lane boundary (measured: `CombatSet::Playback` chains eleven
`ambition_combat` systems with one `actor_monolith` system). Those show as REDUCIBLE
here and are not.

⛔ A FIFTH IRREDUCIBILITY REASON THE SCRIPT CANNOT SEE, found 2026-09-07 by
carving one of its own REDUCIBLE rows: THE GUARD AROUND THE BLOCK.
`host/src/lib.rs:144` reads as one capability plus shared vocabulary and is
irreducible anyway, because it sits inside

    if app.sim_is_fixed_tick() { let sim = app.sim_schedule(); ... }

Whether the host is fixed-tick, and which schedule its simulation runs in, is
knowledge only a composition has. The classifier reads the block's SYMBOLS; the
irreducibility is in the enclosing statement, which it never looks at.

⚠ AND A REDUCIBLE BLOCK CAN STILL BE WORTHLESS TO CARVE. `:214` is a bare
`add_systems(Startup, one_system)` -- genuinely reducible, and moving it buys
nothing, because there is no ordering knowledge to relocate. The number to carve by
is how much SCHEDULING AUTHORITY the composition is holding, not how many blocks
are reducible.

⚠ I then wrote a guard-detector to find the enclosing `if` automatically and it
reported "none found" for all three rows, including the one I had just read. A
resolver that finds nothing looks exactly like a tree with nothing to resolve, so
the reasons above are recorded from READING and not from that heuristic.
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
#: Vocabulary every capability already depends on; naming it is not a leak.
SHARED = {"ambition_platformer2d_shared_tangle"}


def use_map(text: str) -> dict[str, str]:
    """Bare name -> owning crate, from the file's `use ambition_x::{a, b}` statements."""
    owner: dict[str, str] = {}
    for m in re.finditer(r"use\s+(ambition_[a-z_0-9]+)::([^;]+);", text, re.S):
        crate, tail = m.group(1), m.group(2)
        # ⛔ RESOLVE A RE-EXPORT MODULE IN THE `use` ITSELF. The host writes
        # `use ambition_platformer2d_runtime::host_input::{commit_seat_raw_frames, …}`
        # and then names those systems BARE, so attributing them to the crate in the
        # `use` line credits the umbrella rather than the crate that defines them.
        # (the per-name resolution happens below, when each name is attributed)
        reexports = reexport_map()
        for name in re.findall(r"\b([a-z_][a-z_0-9]*)\b", tail):
            owner.setdefault(name, reexports.get(name, crate))
        for name in re.findall(r"\b([A-Z][A-Za-z0-9]*)\b", tail):
            owner.setdefault(name, reexports.get(name, crate))
    return owner


_REEXPORT: dict[str, str] | None = None


def reexport_map() -> dict[str, str]:
    """Re-exported SYSTEM NAME -> the crate that actually defines it.

    ⛔⛔ WITHOUT THIS THE SCRIPT POINTS CARVES AT THE WRONG CRATE.
    `ambition_platformer2d_runtime::host_input` is a re-export facade, so blocks using
    it read as `ambition_platformer2d_runtime` while the systems belong elsewhere — and
    "elsewhere" is another session's lane, so the mislabel is a lane violation waiting to
    happen rather than a cosmetic one.

    ⚠ PER NAME, NOT PER MODULE, and that was the third try. `host_input` re-exports from
    FOUR crates (`ambition_characters`, `ambition_dialog`,
    `ambition_platformer2d_actor_monolith`, `ambition_platformer2d_shared_tangle`), so
    mapping the module to a single crate silently skipped it — which is exactly the
    module this function exists for.
    """
    global _REEXPORT
    if _REEXPORT is not None:
        return _REEXPORT
    out: dict[str, str] = {}
    for manifest in (ROOT / "crates").glob("*/Cargo.toml"):
        lib = manifest.parent / "src" / "lib.rs"
        if not lib.exists():
            continue
        # Every `pub use ambition_x::path::{a, b, c};` anywhere in the crate root.
        for m in re.finditer(r"pub use (ambition_[a-z_0-9]+)::([^;]+);", lib.read_text()):
            crate, tail = m.group(1), m.group(2)
            for name in re.findall(r"\b([a-z_][a-z_0-9]*)\b", tail):
                out.setdefault(name, crate)
            for name in re.findall(r"\b([A-Z][A-Za-z0-9]*)\b", tail):
                out.setdefault(name, crate)
    _REEXPORT = out
    return out


_DEPS: dict[str, set[str]] = {}


def depends_on(crate: str, other: str) -> bool:
    """Does `crate`'s manifest name `other` as a dependency?

    ⭐ THIS IS THE HALF THAT MAKES "REDUCIBLE" MEAN SOMETHING. Without it a block whose
    systems all belong to one capability looks carveable even when that capability
    cannot name what the block orders against — and carving it would invert a
    dependency, which is the failure a count-driven campaign walks into.
    """
    if crate not in _DEPS:
        manifest = ROOT / "crates" / crate / "Cargo.toml"
        text = manifest.read_text() if manifest.exists() else ""
        _DEPS[crate] = set(re.findall(r"^(ambition_[a-z_0-9]+)\s*=", text, re.M))
    return other in _DEPS[crate]


def blocks(lines: list[str]) -> list[tuple[int, str]]:
    out = []
    for i, line in enumerate(lines):
        if "add_systems(" not in line and "configure_sets(" not in line:
            continue
        depth, end = 0, i
        for j in range(i, len(lines)):
            depth += lines[j].count("(") - lines[j].count(")")
            if depth <= 0 and j > i:
                end = j
                break
        out.append((i + 1, "\n".join(lines[i : end + 1])))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("files", nargs="*", help="host files to classify")
    args = ap.parse_args()
    files = args.files or [
        "crates/ambition_platformer2d_runtime/src/combat_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/player_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/progression_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/sim_core_resources.rs",
        "crates/ambition_platformer2d_runtime/src/world_gating.rs",
        "crates/ambition_platformer2d_host/src/lib.rs",
    ]
    total_r = total_i = 0
    for rel in files:
        path = ROOT / rel
        if not path.exists():
            print(f"  (absent: {rel})")
            continue
        text = path.read_text()
        owners = use_map(text)
        own_crate = re.search(r"crates/([a-z_0-9]+)/", rel)
        own_crate = own_crate.group(1) if own_crate else ""
        red, irr = [], []
        for line, body in blocks(text.splitlines()):
            # ⛔⛔ THE SYSTEMS INSTALLED, NOT EVERY NAME MENTIONED. A block can install
            # the HOST'S OWN system and merely ORDER it against a foreign set — that is
            # the composition's job and cannot be carved anywhere, because the system
            # belongs to the host. Counting anchors as evidence of carveability called
            # `crate::portal::tag_portal_camera_continuity_camera .after(camera_follow)`
            # reducible, which is backwards: the only foreign name in it is the anchor.
            #
            # ⇒ Strip the ordering combinators before attributing, so `.after(...)`,
            # `.before(...)`, `.in_set(...)` and `.run_if(...)` arguments do not count.
            installed = re.sub(
                r"\.(?:after|before|in_set|run_if|ambiguous_with)\s*\([^()]*(?:\([^()]*\)[^()]*)*\)",
                "",
                body,
            )
            # A block naming `crate::` installs something of the host's own.
            owns_a_system = "crate::" in installed or any(
                bare in owners and owners[bare] == own_crate
                for bare in re.findall(r"\b([a-z_][a-z_0-9]*)\b", installed)
            )
            # ⭐ RESOLVE RE-EXPORTS FIRST, so `runtime::host_input::foo` is attributed
            # to the crate that DEFINES `foo`, not the umbrella it is spelled through.
            resolved = installed
            for prefix, real in reexport_map().items():
                resolved = resolved.replace(f"{prefix}::", f"{real}::")
            named = set(re.findall(r"\b(ambition_[a-z_0-9]+)::", resolved))
            for bare in re.findall(r"\b([a-z_][a-z_0-9]*)\b", installed):
                if bare in owners:
                    named.add(owners[bare])
            # ⛔ NAMING shared vocabulary is free; INSTALLING a shared crate's SYSTEMS
            # is not. `SHARED` is excluded because every capability may name its sets and
            # run-conditions — but a block that registers `shared_tangle`'s own systems is
            # installing another crate's code, and the owner cannot take that with it.
            # Measured: the camera block installs `camera_ease::tick_camera_shake` and
            # `tick_finish_zoom` beside render's `camera_follow`, and read as
            # single-capability until this line existed.
            installs_shared = any(f"{s}::" in resolved for s in SHARED)
            named -= SHARED | {own_crate}
            if installs_shared and named:
                irr.append((line, sorted(named | SHARED)))
                continue
            # ⛔⛔ A `configure_sets` THAT PLACES A CAPABILITY'S SET INSIDE THE HOST'S OWN
            # PHASE VOCABULARY IS THE COMPOSITION'S JOB, not a leak. The capability
            # PUBLISHES a set and decides the order among its own stages; the composition
            # decides where that set sits in ITS phase order — which is the one thing a
            # capability cannot know. Measured: `ambition_encounter::EncounterLifecycleSet`
            # placed between `ProgressionSet::BossAdvance` and `BossHazards`, and mount's
            # four stages placed `.in_set(CombatSet::Settle)`, whose own comment says
            # exactly this. Both read as carveable until this line existed.
            if "configure_sets(" in body and "add_systems(" not in body:
                irr.append((line, sorted(named) or ["<set placement>"]))
                continue
            if owns_a_system:
                # The host installs one of its own systems here; the block stays.
                irr.append((line, sorted(named) or ["<host-owned>"]))
                continue
            # ⛔⛔ AND THE ANCHORS DECIDE WHETHER THE OWNER CAN ACTUALLY TAKE IT. A block
            # installing ONE capability's systems is only carveable if that capability
            # can NAME everything the block orders against. Measured counter-example:
            # `projectile_visuals` installs `ambition_render` systems `.after` an
            # `ambition_platformer2d_runtime` set, and render does not depend on runtime
            # — carving it would invert a dependency to lower a number.
            anchors = set(re.findall(r"\b(ambition_[a-z_0-9]+)::", body)) - SHARED
            for bare in re.findall(r"\b([a-z_][a-z_0-9]*)\b", body):
                if bare in owners:
                    anchors.add(owners[bare])
            anchors -= SHARED | {own_crate} | named
            if len(named) == 1:
                owner_crate = next(iter(named))
                # ⛔ SHARED IS EXCLUDED FROM `named`, NOT FROM REACHABILITY. A block may
                # name shared sets freely — but the capability that would TAKE it still
                # has to depend on the shared crate to write them down. Measured:
                # `ambition_characters` names `GameplaySimulationRoot` and the monolith
                # phases and does NOT depend on `shared_tangle`, so its block read as
                # carveable while the owner could not compile the carve.
                shared_used = {s for s in SHARED if f"{s}::" in body}
                unreachable = sorted(
                    a
                    for a in (anchors | shared_used)
                    if not depends_on(owner_crate, a)
                )
                if unreachable:
                    irr.append((line, [owner_crate, *unreachable]))
                    continue
            if len(named) == 1:
                red.append((line, next(iter(named))))
            elif len(named) > 1:
                irr.append((line, sorted(named)))
        total_r += len(red)
        total_i += len(irr)
        if red or irr:
            print(f"\n{rel}")
            print(f"   REDUCIBLE   {len(red):>3}  (one capability + shared vocabulary)")
            for line, crate in red:
                print(f"       line {line:>4}  {crate}")
            print(f"   IRREDUCIBLE {len(irr):>3}  (two capabilities that do not depend on each other)")
    print(f"\n  REDUCIBLE   {total_r}   — a capability could install these itself")
    print(f"  IRREDUCIBLE {total_i}   — the composition is the only place that can name both")
    print("\n⚠ Reducible is an UPPER BOUND: a single-capability block can still be "
          "entangled by a `.chain()` that crosses a lane boundary.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
