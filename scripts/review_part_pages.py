#!/usr/bin/env python3
"""See a character's part page as the CURRENT renderer code draws it, beside
the one the game ships, before a full sprite regen.

Generated sprites are gitignored and a full regen takes hours, so renderer
work lands as code long before anyone can look at its pixels: on 2026-10-04
Jon inspected `weird_hermit_parts.png` and `carl_stargan_parts.png` from the
previous day's regen and saw none of the day's work. This renders each named
target through its own publish path (`render_sheet`, the CLI's `sheet`) into
`untracked/part_review/<target>/` and writes `untracked/part_review/index.html`:
per character, the shipped part page and the new one side by side, with parts,
part texels, packed page texels and draws a frame for each.

    scripts/review_part_pages.py weird_hermit carl_stargan sybil
    scripts/review_part_pages.py --jobs 2 --out untracked/part_review alice bob

Names are the renderer's target names (`python -m ambition_sprite2d_renderer
list`). Nothing under `crates/` is written.
"""

from __future__ import annotations

import argparse
import html
import json
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
RENDERER = REPO / "tools" / "ambition_sprite2d_renderer"
SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(RENDERER))

RENDER = """
import sys
from pathlib import Path
from ambition_sprite2d_renderer.cli.commands import _get_target
_get_target(sys.argv[1]).render_sheet(Path(sys.argv[2]))
"""


def renderer_commit() -> str:
    """The renderer's HEAD, with `+dirty` when its tracked files have local changes."""
    head = subprocess.run(["git", "-C", str(RENDERER), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(["git", "-C", str(RENDERER), "status", "--porcelain", "--untracked-files=no"], capture_output=True, text=True).stdout.strip()
    return f"{head}+dirty" if dirty else head


def render(target: str, out: Path) -> tuple[str, int, str]:
    out.mkdir(parents=True, exist_ok=True)
    run = subprocess.run(
        [sys.executable, "-c", RENDER, target, str(out)],
        cwd=RENDERER,
        env={**__import__("os").environ, "PYTHONPATH": str(RENDERER)},
        capture_output=True,
        text=True,
    )
    (out / "render.log").write_text(run.stdout + run.stderr)
    return target, run.returncode, (run.stdout + run.stderr)[-2000:]


def stats(ron: Path) -> dict:
    from ambition_sprite2d_renderer.authoring.part_flipbook import PartFlipbook
    from measure_part_waste import waste

    flipbook = PartFlipbook.from_published(ron)
    result = waste(flipbook)
    result["realize"] = "baked" if "realize: Baked" in ron.read_text() else "parts"
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("targets", nargs="+")
    parser.add_argument("--out", type=Path, default=REPO / "untracked" / "part_review")
    parser.add_argument("--jobs", type=int, default=2, help="targets rendered at once (the machine is shared)")
    args = parser.parse_args()
    out = args.out.resolve()
    commit = renderer_commit()
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(lambda t: render(t, out / t), args.targets))
    rows = []
    for target, code, tail in results:
        new_rons = sorted((out / target).glob("*_parts.ron"))
        if code != 0 or not new_rons:
            rows.append({"target": target, "error": f"exit {code}, no part flipbook" if code == 0 else f"exit {code}", "tail": tail})
            print(f"{target}: FAILED ({code})\n{tail}", file=sys.stderr)
            continue
        for new_ron in new_rons:
            stem = new_ron.name[: -len("_parts.ron")]
            row = {"target": target, "stem": stem, "new": stats(new_ron), "new_png": str(new_ron.with_suffix(".png").relative_to(out)),
                   "renderer": commit, "rendered_at": time.strftime("%Y-%m-%d %H:%M")}
            old_ron = SPRITES / f"{stem}_parts.ron"
            if old_ron.exists():
                shutil.copy2(old_ron.with_suffix(".png"), out / target / f"{stem}_parts.shipped.png")
                row["old"] = stats(old_ron)
                row["old_png"] = f"{target}/{stem}_parts.shipped.png"
            rows.append(row)
            old = row.get("old")
            print(f"{stem:32} parts {old['parts'] if old else '-':>5} -> {row['new']['parts']:<5} part texels "
                  f"{old['part_texels'] if old else '-':>9} -> {row['new']['part_texels']:<9} draws "
                  f"{old['draws_per_frame_max'] if old else '-'} -> {row['new']['draws_per_frame_max']}  {row['new']['realize']}")
    # Rows of targets not rendered this time are kept, stamped with the
    # renderer commit they were rendered at: the index is every character
    # reviewed so far, and says which rows predate the current code.
    previous = out / "review.json"
    if previous.exists():
        rendered = set(args.targets)
        rows += [row for row in json.loads(previous.read_text()) if row["target"] not in rendered]
    rows.sort(key=lambda row: row["target"])
    (out / "review.json").write_text(json.dumps(rows, indent=1, default=str))
    cells = []
    for row in rows:
        if "error" in row:
            cells.append(f"<section><h2>{html.escape(row['target'])}</h2><p class=bad>{html.escape(row['error'])}</p>"
                         f"<pre>{html.escape(row['tail'])}</pre></section>")
            continue
        old, new = row.get("old"), row["new"]

        def line(s):
            return (f"{s['parts']} parts · {s['part_texels']:,} part texels · {s['page_texels']:,} page texels · "
                    f"{s['draws_per_frame_max']} draws max · {s['realize']}")

        ratio = f" — {new['part_texels'] / max(1, old['part_texels']):.0%} of shipped" if old else ""
        stamp = row.get("renderer", "unknown")
        stale = "" if stamp == commit else " <span class=bad>(older renderer code: re-run this target)</span>"
        cells.append(
            f"<section><h2>{html.escape(row['stem'])}{ratio}</h2>"
            f"<p class=stamp>rendered {html.escape(row.get('rendered_at', '?'))} at renderer {html.escape(stamp)}{stale}</p><div class=pair>"
            + (f"<figure><figcaption>shipped: {line(old)}</figcaption><img src='{row['old_png']}'></figure>" if old else "")
            + f"<figure><figcaption>current code: {line(new)}</figcaption><img src='{row['new_png']}'></figure>"
            + "</div></section>"
        )
    (out / "index.html").write_text(
        "<!doctype html><meta charset=utf-8><title>Part page review</title><style>"
        "body{font:14px system-ui;background:#222;color:#ddd;margin:16px}section{margin:24px 0}"
        ".pair{display:flex;gap:16px;flex-wrap:wrap}figure{margin:0;flex:1;min-width:320px}"
        "img{max-width:100%;background:#444;image-rendering:pixelated}.bad{color:#f66}.stamp{color:#999;margin:2px 0 8px}</style>"
        "<h1>Part pages: shipped vs current renderer code</h1>"
        f"<p>Rendered by scripts/review_part_pages.py at renderer {html.escape(commit)}; nothing here is installed in the game until a sprite regen.</p>"
        + "".join(cells)
    )
    print(f"wrote {out / 'index.html'}")
    return 0 if all("error" not in r for r in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
