#!/usr/bin/env python3
"""Print the Steam game id of a non-Steam shortcut, by its name.

``deploy_to_steamdeck.sh`` runs this ON the Steam Deck. A program launched
through the Steam client gets Steam Input's virtual gamepad in desktop mode as
in game mode; launched any other way, the Deck's controls are Steam's desktop
keyboard-and-mouse layout. ``steam steam://rungameid/<id>`` launches a
shortcut through the client, and this prints that ``<id>``.

The shortcuts live in ``~/.local/share/Steam/userdata/<user>/config/
shortcuts.vdf``, a binary VDF: a map is ``0x00 name 0x00`` then its fields, a
string field ``0x01 name 0x00 value 0x00``, an int32 field ``0x02 name 0x00``
and four little-endian bytes, and ``0x08`` ends a map. Each shortcut has an
``appid`` (int32) and an ``AppName`` (the case of the key varies). The game id
is ``appid << 32 | 0x02000000``.

Usage: ``steam_shortcut_gameid.py NAME [SHORTCUTS_VDF...]``. With no files it
reads every user's ``shortcuts.vdf``. Exits 1 when no shortcut has the name.
"""

import glob
import os
import struct
import sys


def parse_map(data: bytes, at: int) -> tuple[dict, int]:
    """Parse map fields from ``at`` up to its ``0x08``; return it and the end."""
    fields: dict = {}
    while True:
        kind = data[at]
        at += 1
        if kind == 0x08:
            return fields, at
        end = data.index(b"\x00", at)
        name = data[at:end].decode("utf-8", "replace")
        at = end + 1
        if kind == 0x00:
            fields[name], at = parse_map(data, at)
        elif kind == 0x01:
            end = data.index(b"\x00", at)
            fields[name] = data[at:end].decode("utf-8", "replace")
            at = end + 1
        elif kind == 0x02:
            fields[name] = struct.unpack_from("<i", data, at)[0]
            at += 4
        else:
            raise ValueError(f"unknown binary VDF field type {kind:#x} at {at - 1}")


def shortcuts(data: bytes) -> list[dict]:
    """The shortcut maps of one ``shortcuts.vdf``."""
    root, _ = parse_map(data, 0)
    table = next(iter(root.values()), {})
    return [entry for entry in table.values() if isinstance(entry, dict)]


def game_id(appid: int) -> int:
    return ((appid & 0xFFFFFFFF) << 32) | 0x02000000


def find(name: str, files: list[str]) -> int | None:
    for path in files:
        with open(path, "rb") as handle:
            data = handle.read()
        for entry in shortcuts(data):
            names = {key.lower(): value for key, value in entry.items()}
            if names.get("appname") == name and "appid" in names:
                return game_id(names["appid"])
    return None


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    name = argv[1]
    files = argv[2:] or glob.glob(
        os.path.expanduser("~/.local/share/Steam/userdata/*/config/shortcuts.vdf")
    )
    found = find(name, files)
    if found is None:
        return 1
    print(found)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
