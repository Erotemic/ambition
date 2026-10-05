#!/usr/bin/env python3
"""Which body-rig joint each part track rides, measured from what shipped.

A character with a published body rig (`<target>_body_rig.ron`, the
simulation's joints) and a part flipbook (`<target>_parts.ron`, the draws the
game places) states the same pose twice: once as joint frames, once as one
transform per part per frame. They are one decomposition only if every part
rides a joint: its transform, taken in that joint's frame, is the same in every
frame of every clip (`docs/planning/engine/semantic-part-rendering-and-ragdolls.md`,
"one semantic body decomposition").

For each track, every joint is tried: in each frame the track draws, its pivot
and angle are carried into the joint's frame (the joint's world frame composed
down its parent chain), and the spread of that local placement over the frames
is the residual. The track rides the joint with the smallest residual; a
residual under `--tolerance` sheet pixels (and degrees) is a rigid ride. A track
no joint holds (an effect layer, a cape, a part that slides along its bone) is
listed apart: that is a part the pose does not yet name.

    python3 scripts/measure_track_joints.py [--target pirate_raider] [--tolerance 0.75]
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

from ambition_sprite2d_renderer.authoring.part_flipbook import PartFlipbook  # noqa: E402

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"

NUMBER = r"(-?[\d.]+(?:e-?\d+)?)"
POSE = re.compile(
    rf"\(translation: \({NUMBER}, {NUMBER}\), rotation: {NUMBER}, scale: \({NUMBER}, {NUMBER}\)\)"
)


def read_body_rig(path: Path):
    """`(joints, clips)`: joints as `[(name, parent)]` in the rig's order, clips
    as `{name: [[(x, y, rotation, sx, sy)] per joint] per frame}`, each pose
    local to its parent (`authoring/body_rig.py`)."""
    text = path.read_text()
    joints = [
        (name, parent or None)
        for name, parent in re.findall(r'\(name: "([^"]+)", parent: (?:None|Some\("([^"]+)"\))\)', text.split("attachments:")[0])
    ]
    clips = {}
    body = text.split("clips:", 1)[1]
    for match in re.finditer(r'"([^"]+)": \(\s*looping: \w+,\s*frame_duration_s: [\d.]+,\s*frames: \[', body):
        name, start = match.group(1), match.end()
        depth, end = 1, start
        while depth:
            ch = body[end]
            depth += {"[": 1, "]": -1}.get(ch, 0)
            end += 1
        frames = []
        for frame in re.findall(r"\[([^\[\]]*)\]", body[start : end - 1]):
            poses = [tuple(float(v) for v in pose) for pose in POSE.findall(frame)]
            if poses:
                frames.append(poses)
        clips[name] = frames
    return joints, clips


def world_frames(joints, poses):
    """Each joint's world frame `(x, y, angle, sx, sy)` from local poses."""
    index = {name: i for i, (name, _) in enumerate(joints)}
    out = {}

    def resolve(i):
        if i in out:
            return out[i]
        name, parent = joints[i]
        x, y, rotation, sx, sy = poses[i]
        if parent is None:
            out[i] = (x, y, rotation, sx, sy)
        else:
            px, py, pr, psx, psy = resolve(index[parent])
            lx, ly = psx * x, psy * y
            c, s = math.cos(pr), math.sin(pr)
            out[i] = (px + lx * c - ly * s, py + lx * s + ly * c, pr + rotation, sx, sy)
        return out[i]

    return [resolve(i) for i in range(len(joints))]


def local_in(joint, at, rotation):
    """A draw's pivot and angle in `joint`'s frame (its scale divided out)."""
    jx, jy, jr, jsx, jsy = joint
    dx, dy = at[0] - jx, at[1] - jy
    c, s = math.cos(-jr), math.sin(-jr)
    lx, ly = dx * c - dy * s, dx * s + dy * c
    return (lx / (jsx or 1.0), ly / (jsy or 1.0), math.degrees(rotation - jr))


def spread(samples):
    """The largest distance of a sample from the samples' mean, in sheet pixels
    for the place and in degrees for the angle (wrapped)."""
    n = len(samples)
    mx = sum(s[0] for s in samples) / n
    my = sum(s[1] for s in samples) / n
    ref = samples[0][2]
    angles = [((s[2] - ref + 180.0) % 360.0) - 180.0 for s in samples]
    ma = sum(angles) / n
    place = max(math.hypot(s[0] - mx, s[1] - my) for s in samples)
    angle = max(abs(a - ma) for a in angles)
    return place, angle


def measure(target: str, tolerance: float):
    joints, clips = read_body_rig(SPRITES / f"{target}_body_rig.ron")
    flipbook = PartFlipbook.from_published(SPRITES / f"{target}_parts.ron")
    feet = flipbook.feet
    # track -> joint index -> samples
    samples = defaultdict(lambda: defaultdict(list))
    compared = 0
    for row, (_, frames) in flipbook.clips.items():
        rig_frames = clips.get(row)
        if not rig_frames or len(rig_frames) != len(frames):
            continue
        for draws, poses in zip(frames, rig_frames):
            if len(poses) != len(joints):
                continue
            compared += 1
            world = world_frames(joints, poses)
            for draw in draws:
                if draw.track is None:
                    continue
                at = (draw.at[0], draw.at[1])
                for j, joint in enumerate(world):
                    samples[draw.track][j].append(local_in(joint, at, draw.rotation))
    rides, loose = [], []
    for track, by_joint in samples.items():
        scored = sorted((max(spread(s)[0], spread(s)[1] * 0.1), j, spread(s), len(s)) for j, s in by_joint.items())
        score, j, (place, angle), count = scored[0]
        row = (track, joints[j][0], place, angle, count)
        (rides if place <= tolerance and angle <= tolerance * 4 else loose).append(row)
    return compared, sorted(rides), sorted(loose, key=lambda r: -r[2])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--target", action="append", help="repeatable; default every target with both files")
    parser.add_argument("--tolerance", type=float, default=0.75, help="sheet pixels (and 4x as many degrees)")
    args = parser.parse_args()
    targets = args.target or sorted(
        path.name[: -len("_body_rig.ron")]
        for path in SPRITES.glob("*_body_rig.ron")
        if (SPRITES / path.name.replace("_body_rig.ron", "_parts.ron")).exists()
    )
    if not targets:
        print("no target publishes both a body rig and a part flipbook", file=sys.stderr)
        return 2
    total_rides = total_loose = 0
    for target in targets:
        compared, rides, loose = measure(target, args.tolerance)
        if compared == 0:
            print(f"{target}: no clip has the same frames in both files; nothing compared", file=sys.stderr)
            return 2
        total_rides += len(rides)
        total_loose += len(loose)
        print(f"{target}: {compared} frames, {len(rides)} tracks ride a joint, {len(loose)} ride none")
        for track, joint, place, angle, count in rides:
            print(f"  ride   {track:<22} {joint:<16} {place:6.2f} px {angle:6.2f} deg  ({count} draws)")
        for track, joint, place, angle, count in loose:
            print(f"  loose  {track:<22} nearest {joint:<16} {place:6.2f} px {angle:6.2f} deg  ({count} draws)")
    print(f"total: {total_rides} tracks ride a joint, {total_loose} ride none, over {len(targets)} targets")
    return 0


if __name__ == "__main__":
    sys.exit(main())
