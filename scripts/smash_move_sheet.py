#!/usr/bin/env python3
"""One picture per fighter: each move's key frames with its hit volumes drawn on.

For auditing a Smash fighter's moves by eye: does the volume sit on the part of
the picture that hits (the fist, the blade, the foot), is it live while that
part is out, and does the hit have an effect to read?

It photographs each move with `moveset_render --overlay art` (the real art, no
debug overlay) and draws the volumes itself from the render's manifest, which
records every shot's world view and the engine's own strike and hurt volumes.
So the boxes are the engine's geometry, not a second copy of the moveset.

Per move, the row shows: the last startup frame, every frame with a live strike
(up to `--max-active`, spread evenly), and the first frame after. Red = strike
volume, cyan = the subject's hurtbox. Each frame is labelled with its tick and
the move's phase.

    python3 scripts/smash_move_sheet.py --character npc_bob --out /tmp/sheets
    python3 scripts/smash_move_sheet.py --character npc_bob --verbs smash_forward,attack_up --out /tmp/sheets

Needs `target/debug/moveset_render` (cargo build -p ambition_app_tools --bin
moveset_render) and `xvfb-run`.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
RENDER = ROOT / "target/debug/moveset_render"

VERBS = [
    "attack", "attack_forward", "attack_up", "attack_down", "attack_dash",
    "attack_air", "attack_air_forward", "attack_air_back", "attack_air_up", "attack_air_down",
    "smash_forward", "smash_up", "smash_down",
    "special", "special_forward", "special_up", "special_down", "grab",
]

#: World px kept around the subject in each crop.
CROP_HALF_W = 95.0
CROP_UP = 110.0
CROP_DOWN = 45.0
#: Pixels per world px in the sheet.
SCALE = 2.0


def render(character: str, verb: str, out: Path, frames: int, quality: str) -> Path:
    dest = out / "renders" / character / verb
    if (dest / "manifest.json").exists():
        return dest
    dest.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, AMBITION_QUALITY_PROFILE=quality)
    cmd = [
        "xvfb-run", "-a", str(RENDER), "--character", character, "--verb", verb,
        "--overlay", "art", "--frames", str(frames), "--view-width", "260",
        "--out", str(dest),
    ]
    subprocess.run(cmd, env=env, cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=900)
    return dest


def subject_of(shot: dict) -> dict | None:
    for body in shot["observation"]["bodies"]:
        if body.get("role") == "subject":
            return body
    return None


def pick_shots(shots: list[dict], max_active: int) -> list[dict]:
    live = [i for i, s in enumerate(shots) if any(st.get("subject_owned") for st in s["observation"]["strikes"])]
    if not live:
        # No strike volume: show the move's own frames spread over it.
        moving = [i for i, s in enumerate(shots) if s.get("move")]
        if not moving:
            return []
        step = max(1, len(moving) // 5)
        return [shots[i] for i in moving[::step][:5]]
    first, last = live[0], live[-1]
    picks = [max(0, first - 2)]
    if len(live) <= max_active:
        picks += live
    else:
        picks += [live[round(k * (len(live) - 1) / (max_active - 1))] for k in range(max_active)]
    picks.append(min(len(shots) - 1, last + 2))
    seen, out = set(), []
    for i in picks:
        if i not in seen:
            seen.add(i)
            out.append(shots[i])
    return out


def draw_shot(render_dir: Path, shot: dict) -> Image.Image:
    img = Image.open(render_dir / shot["file"]).convert("RGB")
    x0, y0, x1, y1 = shot["view"]
    sx = img.width / (x1 - x0)
    sy = img.height / (y1 - y0)
    subject = subject_of(shot)
    cx, cy = subject["collision"]["pos"] if subject else ((x0 + x1) / 2, (y0 + y1) / 2)
    facing = subject.get("facing", 1.0) if subject else 1.0

    def px(x, y):
        return ((x - x0) * sx, (y - y0) * sy)

    draw = ImageDraw.Draw(img)
    if subject:
        for hb in subject["hurtboxes"]:
            (hx, hy), (hw, hh) = hb["pos"], hb["half"]
            draw.rectangle([px(hx - hw, hy - hh), px(hx + hw, hy + hh)], outline=(80, 230, 255), width=1)
    for st in shot["observation"]["strikes"]:
        if not st.get("subject_owned"):
            continue
        (hx, hy), (hw, hh) = st["pos"], st["half"]
        shape = st.get("shape", {})
        if shape.get("kind") == "polygon" and shape.get("points"):
            pts = [px(*p) for p in shape["points"]]
            draw.polygon(pts, outline=(255, 40, 40))
        else:
            draw.rectangle([px(hx - hw, hy - hh), px(hx + hw, hy + hh)], outline=(255, 40, 40), width=2)
    # Crop around the subject, facing-aware so the strike side gets the room.
    lo = cx - CROP_HALF_W * (0.6 if facing > 0 else 1.4)
    hi = cx + CROP_HALF_W * (1.4 if facing > 0 else 0.6)
    a = px(lo, cy - CROP_UP)
    b = px(hi, cy + CROP_DOWN)
    crop = img.crop((int(a[0]), int(a[1]), int(b[0]), int(b[1])))
    w = int((hi - lo) * SCALE)
    h = int((CROP_UP + CROP_DOWN) * SCALE)
    crop = crop.resize((w, h), Image.Resampling.LANCZOS)
    label = f"t{shot['action_tick']} {(subject or {}).get('move_state', {}) and subject['move_state'].get('phase', '')}"
    ImageDraw.Draw(crop).text((4, 2), label, fill=(255, 255, 255))
    return crop


def row_image(character: str, verb: str, render_dir: Path, max_active: int) -> Image.Image | None:
    manifest = render_dir / "manifest.json"
    if not manifest.exists():
        return None
    m = json.loads(manifest.read_text())
    shots = pick_shots(m["shots"], max_active)
    title = f"{verb} -> {','.join(m.get('observed_moves') or []) or 'NOTHING PLAYED'}"
    if not shots:
        img = Image.new("RGB", (600, 40), (30, 30, 30))
        ImageDraw.Draw(img).text((6, 12), title + "  (no frames of the move)", fill=(255, 120, 120))
        return img
    tiles = [draw_shot(render_dir, s) for s in shots]
    w = sum(t.width for t in tiles) + 4 * len(tiles)
    h = tiles[0].height + 16
    img = Image.new("RGB", (w, h), (20, 20, 24))
    ImageDraw.Draw(img).text((4, 2), title, fill=(255, 230, 120))
    x = 0
    for t in tiles:
        img.paste(t, (x, 16))
        x += t.width + 4
    return img


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--character", required=True)
    ap.add_argument("--verbs", default=",".join(VERBS))
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--frames", type=int, default=80)
    ap.add_argument("--max-active", type=int, default=3)
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--quality", default="high")
    ap.add_argument("--rerender", action="store_true")
    args = ap.parse_args()
    if not RENDER.exists():
        print(f"missing {RENDER}: cargo build -p ambition_app_tools --bin moveset_render", file=sys.stderr)
        return 2
    verbs = [v for v in args.verbs.split(",") if v]
    if args.rerender:
        for v in verbs:
            d = args.out / "renders" / args.character / v
            if (d / "manifest.json").exists():
                (d / "manifest.json").unlink()
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        dirs = list(pool.map(lambda v: render(args.character, v, args.out, args.frames, args.quality), verbs))
    rows = [r for r in (row_image(args.character, v, d, args.max_active) for v, d in zip(verbs, dirs)) if r]
    if not rows:
        print("nothing rendered", file=sys.stderr)
        return 1
    # Two sheets of half the moves each keep a page readable.
    pages = [rows[: (len(rows) + 1) // 2], rows[(len(rows) + 1) // 2 :]]
    for n, page in enumerate(p for p in pages if p):
        w = max(r.width for r in page)
        h = sum(r.height + 6 for r in page)
        sheet = Image.new("RGB", (w, h), (12, 12, 14))
        y = 0
        for r in page:
            sheet.paste(r, (0, y))
            y += r.height + 6
        path = args.out / f"{args.character}_moves_{n + 1}.png"
        sheet.save(path)
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
