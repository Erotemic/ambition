#!/usr/bin/env python3
"""Measure the campaign characters' sheet rows against `body_state_clip` (POSE-CONTINUITY).

`body_state_clip` (`crates/ambition_character_sprites/src/anim/mod.rs`) asks a
sheet for a state's row by a preference chain: the state's own pose first, then
older rows a sheet may draw instead. A Smash fighter's sheet draws every first
choice (the roster rollout, 2026-10-31). A campaign-only character's sheet was
never given them. This reads each chain from the source and each sheet's rows
from its newest published record, and reports per sheet which first choices it
draws and, for each it lacks, the row the chain lands on.

    python3 scripts/measure/campaign_sheet_gaps.py            # summary
    python3 scripts/measure/campaign_sheet_gaps.py --sheets   # one line a sheet
"""
import argparse
import collections
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ANIM = os.path.join(REPO, "crates/ambition_character_sprites/src/anim/mod.rs")
BODY = os.path.join(REPO, "crates/ambition_characters/src/actor/body.rs")
CATALOG = os.path.join(REPO, "game/ambition_content/assets/data/character_catalog.ron")
RULES = os.path.join(REPO, "game/ambition_demo_smash/assets/data/smash_rules.ron")
BUNDLES = os.path.join(REPO, "game/ambition_content_library/generated/art_publication/bundles")

parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
parser.add_argument("--sheets", action="store_true", help="print one line for each campaign sheet")
args = parser.parse_args()


def chains():
    """Every preference chain `body_state_clip` can return, as written."""
    anim = open(ANIM).read()
    start = anim.index("pub fn body_state_clip(")
    body = anim[start:anim.index("\npub fn pick_body_anim(", start)]
    texts = [body]
    # The guard break's and the recoil's chains, which it returns by name.
    for m in re.finditer(r"pub fn clip_chain\(self\).*?\n    \}", anim, re.S):
        texts.append(m.group(0))
    out = []
    for text in texts:
        out += [re.findall(r'"([a-z_0-9]+)"', m.group(1)) for m in re.finditer(r"&\[((?:\"[a-z_0-9]+\",?\s*)+)\]", text)]
    body_rs = open(BODY).read()
    for name in ("PETTING_CLIPS", "PETTED_CLIPS"):
        m = re.search(name + r': &\[&str\] = &\[(.*?)\];', body_rs)
        out.append(re.findall(r'"([a-z_0-9]+)"', m.group(1)))
    return out


def roster():
    text = open(RULES).read()
    block = text[text.index("roster: ["):]
    block = block[:block.index("]")]
    return set(re.findall(r'"([a-z_0-9]+)"', block))


def catalog_sheets():
    """Each catalog character's sheet manifest, by character id."""
    text = open(CATALOG).read()
    text = text[text.index("    characters: {"):]
    by_id = {}
    for m in re.finditer(r'^        "([a-z_0-9]+)": \(\n(.*?)^        \),', text, re.S | re.M):
        manifest = re.search(r'manifest: "([^"]+)"', m.group(2))
        if manifest:
            by_id[m.group(1)] = manifest.group(1)
    return by_id


def published_records(wanted):
    """The newest published full-tier record of each manifest in `wanted`."""
    newest = {}
    for bundle in os.listdir(BUNDLES):
        full = os.path.join(BUNDLES, bundle, "runtime", "full")
        for manifest in wanted:
            path = os.path.join(full, manifest)
            if os.path.exists(path):
                mtime = os.path.getmtime(path)
                if manifest not in newest or mtime > newest[manifest][0]:
                    newest[manifest] = (mtime, path)
    return {manifest: path for manifest, (_, path) in newest.items()}


def rows_of(manifest):
    """The rows of the newest published record of `manifest`, or None."""
    path = RECORDS.get(manifest)
    if path is None:
        return None
    return set(re.findall(r'animation: "([a-z_0-9]+)"', open(path).read()))


all_chains = chains()
smash = roster()
sheets = collections.defaultdict(list)
for character, manifest in catalog_sheets().items():
    sheets[manifest].append(character)

campaign = {m: ids for m, ids in sheets.items() if not any(i in smash for i in ids)}
RECORDS = published_records(campaign)
print(f"{len(all_chains)} chains in body_state_clip; {len(sheets)} catalog sheets, "
      f"{len(campaign)} drawn by no Smash fighter")

unpublished = []
missing_first = collections.Counter()
landings = collections.Counter()
nothing = collections.Counter()
lines = []
for manifest, ids in sorted(campaign.items()):
    rows = rows_of(manifest)
    if rows is None:
        unpublished.append(manifest)
        continue
    lacks = []
    for chain in all_chains:
        if chain[0] in rows:
            continue
        land = next((row for row in chain[1:] if row in rows), None)
        missing_first[chain[0]] += 1
        landings[(chain[0], land)] += 1
        if land is None:
            nothing[chain[0]] += 1
        lacks.append(f"{chain[0]}->{land or 'NONE'}")
    lines.append((len(lacks), manifest, ids, lacks))

measured = len(lines)
print(f"measured {measured} published campaign sheets ({len(unpublished)} not published here)")
drawn = sum(1 for n, *_ in lines if n == 0)
print(f"sheets that draw every first choice: {drawn} of {measured}")
print("first choices missing, by how many sheets lack them (and where those land):")
for first, n in missing_first.most_common():
    lands = ", ".join(f"{land or 'NONE'} x{c}" for (f, land), c in landings.most_common() if f == first)
    print(f"  {first:24s} {n:4d}   {lands}")
if args.sheets:
    for n, manifest, ids, lacks in sorted(lines, reverse=True):
        print(f"{n:3d} {manifest}  [{', '.join(sorted(ids)[:3])}{', ...' if len(ids) > 3 else ''}]")
if unpublished:
    print("not published here:", ", ".join(sorted(unpublished)[:12]), "..." if len(unpublished) > 12 else "")
