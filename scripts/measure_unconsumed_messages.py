#!/usr/bin/env python3
"""Which `add_message::<T>` types have no reader outside the crate that registers them?

⭐ WHY. A producer that is DEFINED, INSTALLED, ORDERED and never READ looks exactly like
a working one from every install site. Grepping the definition side finds nothing wrong;
only "who consumes this" does. Two seams were found this way on 2026-09-06:

  · `ambition_input::SemanticActionPressed` — the provider action road, wired end to end
    and read by nobody. DORMANT: `tracks.md`'s provider-actions row is its named future
    customer.
  · `ambition_game_shell::ShellAbandonRequested` — the `Exit Match` row Jon asked for in
    a W8 playtest. The shell draws the row and reports the press; no experience inserts
    `ShellAbandonOffer` and none reads the request, so the row never appears and would do
    nothing if it did.

⛔⛔ A HIT IS A QUESTION, NOT A VERDICT, and the three answers want different things:
  DORMANT      a named future customer exists (a planning row). Restraint, not rot.
  DEAD         nothing intends to consume it. A deletion candidate.
  FALSE POSITIVE  it IS consumed, by a road this script cannot see.

⚠ THE FALSE-POSITIVE MODE IS REAL AND MEASURED: `RunAuthoredCommand` reads as unconsumed
because it is drained MANUALLY —
`world.get_resource_mut::<Messages<T>>()` then `.drain()` — rather than through a
`MessageReader`. Any hit must be grepped by hand before it is believed. A test-only
reader (`SuddenDeathBegan`) is a fourth answer again: covered, but not shipped.
⛔⛔ AND THE TECHNIQUE DOES NOT TRANSFER TO RESOURCES — tried 2026-09-06, abandoned, and
recorded here so nobody builds the noisy version. The same census over the 388 declared
`Resource` types reported 43 "never read", and the first THREE spot-checks were all
false positives:

  · `KeyboardOwner`      read as `Option<Res<crate::sources::KeyboardOwner>>` — the
                         pattern demanded an unqualified `Res<KeyboardOwner>`;
  · `LdtkWorldAssets`    read as `Option<&LdtkWorldAssets>`, a plain reference param;
  · `CharacterSpriteAssets` passed as `&mut CharacterSpriteAssets` to a helper.

⇒ A MESSAGE has essentially one read shape (`MessageReader<T>`), which is why this census
works. A RESOURCE has many — `Res`/`ResMut`, path-qualified or not, `Option<..>`, plain
`&T`/`&mut T` params, `SystemParam` struct fields, `world.resource::<T>()`, `Single<&T>` —
so a grep-shaped census produces a list nobody can act on. Resolving it properly needs
type resolution, not regex.
"""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main() -> int:
    registered: dict[str, set[str]] = {}
    for path in (ROOT / "crates").rglob("*.rs"):
        if "tests" in path.name:
            continue
        for m in re.finditer(r"add_message::<([A-Za-z0-9_:]+)>", path.read_text()):
            registered.setdefault(m.group(1).split("::")[-1], set()).add(path.parts[-3])
    print(f"message types registered: {len(registered)}")

    orphans = []
    for name, owners in sorted(registered.items()):
        hits = subprocess.run(
            ["grep", "-rl", f"MessageReader<.*{name}", "crates", "game"],
            capture_output=True,
            text=True,
            cwd=ROOT,
        ).stdout.split()
        outside = {p.split("/")[1] for p in hits if p.startswith("crates/")}
        outside |= {"<game>" for p in hits if p.startswith("game/")}
        outside -= owners
        if not hits:
            orphans.append((name, sorted(owners), "NO READER AT ALL"))
        elif not outside:
            orphans.append((name, sorted(owners), "read only inside its own crate"))

    for name, owners, why in orphans:
        print(f"  {name:<38} {why:<30} owner={owners}")
    print(
        f"\n{len(orphans)} of {len(registered)} types have no reader outside the crate "
        "that registers them"
    )
    print(
        "\n⚠ EACH HIT IS A QUESTION. Grep it by hand: a manual `Messages<T>` drain reads "
        "as unconsumed here and is not."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
