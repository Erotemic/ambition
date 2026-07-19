#!/usr/bin/env python3
"""
Export the vanity card as an engine-loadable image sequence.

Unlike `frame_demo.export_gif`, which samples the timeline at a fixed fps and
therefore emits many identical frames, this walks the SAME beat list and emits
each distinct picture ONCE with the duration it should be held for. The engine
side (`ShellSegmentPresentation::ImageSequence`) carries a per-frame hold, so
holds cost one image regardless of how long they last.

Two levels of collapse happen here:

  1. consecutive identical pictures merge into one timeline entry (their holds
     add), e.g. the tail of a `play` beat running into the bubble-delay lead-in
     of the next `hold` beat;
  2. identical pictures anywhere in the timeline share ONE png on disk (content
     hash), e.g. the neutral first panel appearing both before and after its
     speech bubble.

Outputs
-------
  game/ambition_content/assets/vanity_card/frame_NN.png   payload (gitignored)
  game/ambition_content/assets/data/vanity_card.ron       manifest (committed)

The manifest is committed on purpose: it is the contract the host composes from,
and it is what lets a checkout without the payload render "missing frame 3 of 9"
with correct timing instead of showing nothing.

Run:  python3 export_sequence.py
"""

import argparse
import hashlib
import os

from PIL import Image

from utils import load_config, out_path
from frame_demo import (
    load_panel_frames,
    load_pil_font,
    make_panel_rect,
    render_frame_pil,
)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, "..", ".."))

DEFAULT_PAYLOAD_DIR = os.path.join(
    REPO_ROOT, "game", "ambition_content", "assets", "vanity_card"
)
DEFAULT_MANIFEST = os.path.join(
    REPO_ROOT, "game", "ambition_content", "assets", "data", "vanity_card.ron"
)
# Path prefix the manifest records, relative to the `game://` asset source root
# (the content crate's assets dir). The Rust side prepends the source scheme.
ASSET_SUBDIR = "vanity_card"


def build_timeline(frames_pil: list, beats: list, cfg: dict, W: int, H: int) -> list:
    """Expand beats into [(PIL image, hold_seconds)], merging consecutive dupes.

    Every entry's duration is the real authored duration, not an fps sample.
    """
    ac = cfg["animation"]
    pil_font = load_pil_font(ac["bubble_font_size"])
    panel_rect = make_panel_rect(W, H, frames_pil[0])
    bubble_delay = ac["bubble_delay"]

    timeline: list = []

    def push(img: Image.Image, hold: float) -> None:
        """Append, merging into the previous entry if the picture is identical."""
        if hold <= 0:
            return
        if timeline and timeline[-1][0].tobytes() == img.tobytes():
            timeline[-1][1] += hold
            return
        timeline.append([img, hold])

    def render(frame_idx: int, bubble: str | None, side: str) -> Image.Image:
        return render_frame_pil(
            frames_pil[frame_idx],
            bubble,
            side,
            255 if bubble else 0,
            panel_rect,
            cfg,
            W,
            H,
            pil_font,
            transparent=True,
        )

    for beat in beats:
        if beat["type"] == "hold":
            duration = float(beat["duration"])
            bubble = beat.get("speech_bubble")
            side = beat.get("bubble_side", "right")
            idx = beat["frame"]
            if bubble:
                # The bubble pops in after a short delay, so a bubbled hold is
                # exactly two pictures: bare panel, then panel + bubble.
                lead = min(bubble_delay, duration)
                push(render(idx, None, side), lead)
                push(render(idx, bubble, side), duration - lead)
            else:
                push(render(idx, None, side), duration)

        elif beat["type"] == "play":
            seq = beat.get("frames", [])
            beat_fps = beat.get("fps", 8)
            frame_dur = 1.0 / max(beat_fps, 0.01)
            for frame_idx in seq:
                push(render(frame_idx, None, "right"), frame_dur)

        else:
            print(f"  WARNING: unknown beat type {beat['type']!r}, skipped")

    return timeline


def write_outputs(timeline: list, payload_dir: str, manifest_path: str) -> None:
    """Write deduped pngs + the RON manifest."""
    os.makedirs(payload_dir, exist_ok=True)
    os.makedirs(os.path.dirname(manifest_path), exist_ok=True)

    # Content-hash dedup: identical pictures anywhere in the timeline share a file.
    by_digest: dict = {}
    entries: list = []
    for img, hold in timeline:
        digest = hashlib.sha256(img.tobytes()).hexdigest()
        name = by_digest.get(digest)
        if name is None:
            name = f"frame_{len(by_digest):02d}.png"
            by_digest[digest] = name
            img.save(os.path.join(payload_dir, name))
        entries.append((name, hold))

    total_ms = sum(int(round(hold * 1000)) for _, hold in entries)
    lines = [
        "// GENERATED by tools/vanity_card_prep/export_sequence.py — do not hand-edit.",
        "//",
        "// The startup vanity card as a held image sequence. `hold_ms` is how long",
        "// each frame stays on screen; the shell derives the whole card's duration",
        "// from their sum, so the animation cannot drift against the card's own",
        "// lifetime. Paths are relative to the `game://` asset source.",
        "//",
        f"// {len(entries)} timeline entries over {len(by_digest)} unique images,"
        f" {total_ms}ms total.",
        "(",
        "    frames: [",
    ]
    for name, hold in entries:
        lines.append(
            f'        (path: "{ASSET_SUBDIR}/{name}", hold_ms: {int(round(hold * 1000))}),'
        )
    lines += ["    ],", ")", ""]

    with open(manifest_path, "w") as f:
        f.write("\n".join(lines))

    print(f"  {len(entries)} timeline entries → {len(by_digest)} unique pngs")
    print(f"  total runtime {total_ms}ms")
    print(f"  payload  → {os.path.relpath(payload_dir, REPO_ROOT)}")
    print(f"  manifest → {os.path.relpath(manifest_path, REPO_ROOT)}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--width", type=int, default=1280)
    ap.add_argument("--height", type=int, default=720)
    ap.add_argument("--payload-dir", default=DEFAULT_PAYLOAD_DIR)
    ap.add_argument("--manifest", default=DEFAULT_MANIFEST)
    args = ap.parse_args()

    cfg = load_config()
    print("=== Loading panels ===")
    frames_pil = load_panel_frames(cfg)
    if not frames_pil:
        raise SystemExit(
            f"no panels found under {out_path(cfg, 'panels')} — "
            "run `python3 extract_panels.py` first"
        )

    beats = cfg["panel_animation"]["beats"]
    print("=== Expanding beats ===")
    timeline = build_timeline(frames_pil, beats, cfg, args.width, args.height)
    if not timeline:
        raise SystemExit("beat expansion produced no frames")

    print("=== Writing sequence ===")
    write_outputs(timeline, args.payload_dir, args.manifest)


if __name__ == "__main__":
    main()
