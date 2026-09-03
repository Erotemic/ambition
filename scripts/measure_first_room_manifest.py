#!/usr/bin/env python3
"""What decodes at boot that no first-room cover ever waited for?

Jon's host run shows an UNCOVERED first-room decode: 98 images / 83 MB between
2.3 s and 3.0 s, with 203 ms and 125 ms frames in it. The reveal barrier only
holds for what the first-room manifest NAMES, so the population that matters is
everything decoded before `room-loaded` that the manifest could not have listed.

This reads a boot's own stderr and answers that, per image:

    road            the demand stamp (`character-sheet`, `parallax`, …)
    MP              megapixels decoded
    before/after    relative to the first `room-loaded`
    coverable       could the first-room manifest have named it?

⭐ THE THREE LINES IT READS, all already emitted at AMBITION_PROFILE_CENSUS=1:

    [image]           {at}s f{frame} {W}x{H} {MP}MP live={n} {path} {demand}
    [world-event]     {at}s f{frame} room-loaded {room_id}
    [first-room-art]  room '{id}' ready after N updates (M …): T assets, C characters

⛔⛔ "COVERABLE" IS AN UPPER BOUND, NOT A PLAN. It says the image arrived on a
road the manifest speaks — a demand stamp that names content art — not that the
manifest would have resolved it, nor that waiting for it is wanted. A cover that
waits for everything coverable is a longer cover, which is a product decision
and not this script's to make. The number's job is to size the question.

⛔ AND AN UNSTAMPED IMAGE IS NOT A ZERO. `demand=unknown` means the image
reached `Assets<Image>` by a road that stamps nothing, so this cannot say
whether a manifest could name it. Those are reported in their own bucket rather
than silently counted as uncoverable — absence of a stamp is absence of
evidence.

Usage:
    scripts/measure_first_room_manifest.py <boot-stderr.txt>
    scripts/measure_first_room_manifest.py <bundle-dir>     # reads its stamped log
    ... | scripts/measure_first_room_manifest.py -
"""

from __future__ import annotations

import argparse
import collections
import re
import sys
from pathlib import Path

# The stamped-log prefix `profile_desktop.sh` adds; optional, since a raw
# `2>` capture has none.
STAMP = re.compile(r"^\[\s*[0-9.]+s\]\s?")
# ⛔⛔ THE FRAME FIELD IS OPTIONAL, AND THE ROAD IS AFTER `via`. My first
# version required `f{frame}` and took the road as the first token after the
# path — both from reading the CURRENT source. A real captured boot
# (`headless-room-frame-20260902T135533Z`) has no frame field at all, because it
# predates that column, and its demand reads `demand→insert 232ms via fx-sheet`.
# The parser matched NOTHING on 8 real `[image]` lines. It refused rather than
# reporting zero, which is the only reason this is a note and not a finding.
IMAGE = re.compile(
    r"\[image\]\s+([0-9.]+)s\s+(?:f\s*(\d+)\s+)?(\d+)x(\d+)\s+"
    r"([0-9.]+)MP\s+live=(\S+)\s+(\S+)\s*(.*)$"
)
# `demand_phrase()` has exactly three shapes; the road is the token after `via`.
VIA = re.compile(r"\bvia\s+(\S+)")
UNKNOWN_DEMAND = re.compile(r"\bdemand=unknown\b")
# The rolling `[image-census]` window line, for the denominator below.
#: ⭐ THE RESIDENT MB IS CAPTURED TOO, and it exists NOWHERE ELSE in a bundle.
#: `asset_activity.csv` carries `images_resident` as a COUNT and no byte column,
#: so the only record of how much memory the images actually hold is this line.
#: `docs/planning/engine/asset-preparation-and-residency.md` Open work 4 is
#: waiting on exactly that number to choose a residency budget.
#: ⚠ Optional group: an older capture's line may stop after the megapixels.
IMAGE_CENSUS = re.compile(
    r"\[image-census\][^|]*\|\s*total (\d+) images,\s*([0-9.]+)MP"
    r"(?:,\s*([0-9.]+)MB resident)?"
)
ROOM_LOADED = re.compile(r"\[world-event\]\s+([0-9.]+)s\s+f\s*(\d+)\s+room-loaded\s+(\S+)")
FIRST_ROOM_ART = re.compile(
    r"\[first-room-art\]\s+room '([^']+)' ready after (\d+) updates "
    r"\((\d+) of them waiting only on GPU uploads\): (\d+) assets, (\d+) characters"
)

# Demand roads that name CONTENT ART — the vocabulary a room manifest speaks.
# Measured 2026-09-02 from every literal reaching note_demand /
# load_sheet_image / load_sprite_pages; see asset-preparation-and-residency.md.
CONTENT_ROADS = {
    "character-sheet",
    "boss-sheet",
    "portrait",
    "parallax",
    "fx-sheet",
    "projectile-art",
    "held-item",
    "shrine-sheet",
    "vanity-card",
    "asset-manifest",
}


def parse(lines) -> dict:
    images: list[dict] = []
    room_loaded: tuple[float, int, str] | None = None
    first_room_art: dict | None = None
    census_total: tuple[int, float] | None = None
    census_resident_mb: float | None = None

    for raw in lines:
        line = STAMP.sub("", raw.rstrip("\n"))
        hit = IMAGE.search(line)
        if hit:
            at, frame, width, height, mp, live, path, tail = hit.groups()
            via = VIA.search(tail)
            if via:
                road = via.group(1)
            elif UNKNOWN_DEMAND.search(tail):
                road = "unknown"
            else:
                road = ""
            images.append(
                {
                    "at": float(at),
                    "frame": int(frame) if frame else None,
                    "mp": float(mp),
                    "path": path,
                    "road": road,
                    "raw_demand": tail.strip(),
                }
            )
            continue
        if room_loaded is None:
            hit = ROOM_LOADED.search(line)
            if hit:
                room_loaded = (float(hit.group(1)), int(hit.group(2)), hit.group(3))
                continue
        if first_room_art is None:
            hit = FIRST_ROOM_ART.search(line)
            if hit:
                first_room_art = {
                    "room": hit.group(1),
                    "updates": int(hit.group(2)),
                    "gpu_updates": int(hit.group(3)),
                    "assets": int(hit.group(4)),
                    "characters": int(hit.group(5)),
                }
        hit = IMAGE_CENSUS.search(line)
        if hit:
            census_total = (int(hit.group(1)), float(hit.group(2)))
            # kept SEPARATE from `census_total` so the pair's shape — and every
            # test that reads it — stays what it was.
            if hit.group(3):
                census_resident_mb = float(hit.group(3))
    return {
        "images": images,
        "room_loaded": room_loaded,
        "first_room_art": first_room_art,
        "census_total": census_total,
        "census_resident_mb": census_resident_mb,
    }


def report(parsed: dict) -> int:
    images = parsed["images"]
    loaded = parsed["room_loaded"]
    art = parsed["first_room_art"]

    if not images:
        print(
            "⛔ NO `[image]` LINES IN THIS LOG. Either the run was not started "
            "with AMBITION_PROFILE_CENSUS=1, or it is not a boot. Absent is not "
            "zero: this is the census refusing to report an empty population as "
            "a finding."
        )
        return 2
    if loaded is None:
        print(
            f"⛔ {len(images)} image(s) decoded but NO `room-loaded` in this log. "
            "Without it there is no boundary to measure against — every image "
            "would count as 'before the first room', which is true and useless."
        )
        return 2

    at, frame, room = loaded
    # ⛔⛔ COMPARE FRAMES, NOT TIMES, AND THE EMITTER SAYS SO. asset_census.rs
    # stamps `[image]` with "the same frame stamp as `[world-event]`, so 'before
    # or after room-loaded' is a comparison of two integers rather than of two
    # wall clocks: the census runs in `Last`, after a long activation frame's
    # work, so its time can read AFTER a `room-loaded` that the insertion in
    # `PreUpdate` of the same frame actually preceded."
    #
    # That is not hypothetical on the 2026-09-02 host bundle: the player sheet's
    # second decode is frame 1129 against `room-loaded` at frame 1131 — two
    # frames BEFORE it — while its game clock reads 3.924 s against 3.863 s,
    # i.e. after. Ordering by time moved 7.6 MP into the wrong bucket and made
    # "6 images / 16.1 MP before the first room" out of 7 / 23.7.
    if frame is not None and all(i["frame"] is not None for i in images):
        ordered_by = "frame"
        before = [i for i in images if i["frame"] < frame]
        after = [i for i in images if i["frame"] >= frame]
    else:
        # An older capture with no `f NNN` column. Time is all there is, and the
        # report says which was used rather than letting a reader assume.
        ordered_by = "game time (no frame column in this capture)"
        before = [i for i in images if i["at"] < at]
        after = [i for i in images if i["at"] >= at]

    # ⛔⛔ THE DENOMINATOR, FIRST, BECAUSE THIS LIST IS A SAMPLE AND READS LIKE A
    # POPULATION. `[image]` prints only for decodes at or above
    # `ImageCensus::NOTABLE_MEGAPIXELS` (1.0 MP). On the 2026-09-02 host bundle
    # that is 11 printed lines against 252 images actually decoded — 4% by
    # count, about half the megapixels. Anyone reading "6 images before
    # room-loaded" without this believes they have seen the boot.
    total = parsed.get("census_total")
    if total:
        print(
            f"⚠ SAMPLE, NOT POPULATION: `[image]` prints only decodes >= 1.0 MP. "
            f"This run's census reports {total[0]} images / {total[1]:.1f} MP "
            f"decoded in total; {len(images)} were notable enough to print."
        )
    else:
        print(
            "⚠ SAMPLE, NOT POPULATION: `[image]` prints only decodes >= 1.0 MP, "
            "and this log has no `[image-census]` line to say how many were "
            "decoded in total. The counts below are a floor."
        )
    print(f"first room-loaded: {room} at {at:.3f}s (frame {frame})")
    print(f"ordered by:        {ordered_by}")
    if art:
        print(
            f"first-room-art:    room '{art['room']}' ready after {art['updates']} "
            f"updates, {art['assets']} assets, {art['characters']} characters"
        )
    else:
        print(
            "first-room-art:    ⚠ no `[first-room-art]` line — this boot took a "
            "route that never ran the first-room cover, so 'what the cover "
            "waited for' is unanswerable from this log"
        )

    coverable = [i for i in before if i["road"] in CONTENT_ROADS]
    unstamped = [i for i in before if not i["road"] or i["road"] in {"unknown", "?"}]
    other = [i for i in before if i not in coverable and i not in unstamped]

    print(
        f"\ndecoded BEFORE room-loaded: {len(before)} images, "
        f"{sum(i['mp'] for i in before):.1f} MP"
    )
    print(f"decoded after:              {len(after)} images, {sum(i['mp'] for i in after):.1f} MP")

    by_road: dict[str, list[dict]] = collections.defaultdict(list)
    for image in before:
        by_road[image["road"] or "(no stamp)"].append(image)
    print(f"\n{'road':<20} {'images':>7} {'MP':>8}   coverable by a room manifest?")
    for road, rows in sorted(by_road.items(), key=lambda kv: -sum(i["mp"] for i in kv[1])):
        mark = (
            "yes — content art"
            if road in CONTENT_ROADS
            else "UNKNOWN — no demand stamp"
            if road in {"(no stamp)", "unknown", "?"}
            else "no — not a content road"
        )
        print(f"{road:<20} {len(rows):>7} {sum(i['mp'] for i in rows):>8.1f}   {mark}")

    print(
        f"\n⇒ {len(coverable)} image(s) / {sum(i['mp'] for i in coverable):.1f} MP "
        "arrived before the first room on a road a manifest speaks — the "
        "population a first-room cover COULD name."
    )
    if unstamped:
        print(
            f"⚠ {len(unstamped)} image(s) / {sum(i['mp'] for i in unstamped):.1f} MP "
            "carry NO demand stamp, so this cannot say whether a manifest could "
            "name them. Not counted either way."
        )
    if other:
        print(
            f"  {len(other)} image(s) / {sum(i['mp'] for i in other):.1f} MP came by "
            "roads outside the content vocabulary."
        )
    if art and coverable:
        print(
            f"\n⚠ The cover it ran waited for {art['assets']} assets. That is not "
            f"directly comparable to {len(coverable)} images — a manifest asset "
            "may be several pages, and the cover counts what it NAMED rather "
            "than what decoded. The two numbers bound the gap; they do not "
            "subtract."
        )
    print(
        "\n⛔ COVERABLE IS AN UPPER BOUND. It says the image arrived on a road "
        "the manifest speaks, not that the manifest would resolve it or that "
        "waiting for it is wanted. A cover that waits for everything coverable "
        "is a longer cover, which is a product decision."
    )
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("log", help="a boot's stderr, a profiling bundle dir, or - for stdin")
    args = ap.parse_args(argv)

    if args.log == "-":
        return report(parse(sys.stdin))
    path = Path(args.log)
    if path.is_dir():
        for name in ("game-stderr-stamped.txt", "game-stdout-stamped.txt"):
            candidate = path / name
            if candidate.exists():
                path = candidate
                break
        else:
            print(f"⛔ no stamped log under {args.log}")
            return 2
    if not path.exists():
        print(f"⛔ no such log: {path}")
        return 2
    return report(parse(path.read_text(errors="ignore").splitlines()))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
