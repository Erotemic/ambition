#!/usr/bin/env python3
"""Measure one fighter's clip transitions from its published parts (POSE-CONTINUITY).

The animator resets to a new clip's frame 0 (`ambition_sprite_sheet`'s
`character::animator`), so the drawn pose jumps from the old clip's current
frame to the new clip's first. For each gameplay transition A->B this reports:

- end: the largest body-track displacement (px) from A's last frame to B[0];
- int_med / int_max: the same from every frame of A (an interruption);
- within_A / within_B: the p95 frame-to-frame displacement inside each clip;
- pops: body tracks drawn on one side of the cut and not the other;
- verdict: `pop` when the end jump is over twice the larger in-clip step,
  `interrupt-pop` when only an interruption is.

Effect tracks (sparks, blades, rings, jets, faces, visor) are left out: they
are meant to appear and vanish.

    python3 scripts/measure/pose_transitions.py player_robot_v3
    python3 scripts/measure/pose_transitions.py --parts path/to/x_parts.ron
"""
import argparse, glob, math, os, re, statistics, sys

parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
parser.add_argument("target", nargs="?", help="a published sprite target, e.g. player_robot_v3")
parser.add_argument("--parts", help="a parts RON to read instead of the newest published one")
args = parser.parse_args()
if args.parts:
    path = args.parts
elif args.target:
    found = glob.glob(f"game/ambition_content_library/generated/art_publication/bundles/*/runtime/full/sprites/{args.target}_parts.ron")
    if not found:
        sys.exit(f"no published parts for {args.target!r}: publish its sheet first")
    path = max(found, key=os.path.getmtime)
else:
    parser.error("name a target or --parts")
print(f"parts: {path}")
text = open(path).read()
tracks = re.search(r'tracks: \[(.*?)\]', text).group(1)
tracks = [t.strip().strip('"') for t in tracks.split(',') if t.strip()]
clips = {}
body = text[text.index('clips: {'):]
for m in re.finditer(r'^        "([a-z_0-9]+)": \(frame_duration_s: ([0-9.]+)(?:, [a-z_]+: [A-Za-z]+)*, frames: \[(.*?)^        \]\)', body, re.S | re.M):
    name, frames_txt = m.group(1), m.group(3)
    frames = []
    for fm in re.finditer(r'^            \[(.*?)^            \]', frames_txt, re.S | re.M):
        frame = {}
        for e in re.finditer(r'\(part: \d+, at: \(([-0-9.]+), ([-0-9.]+)\), rotation: ([-0-9.]+), scale: \(([-0-9.]+), ([-0-9.]+)\), track: (\d+)\)', fm.group(1)):
            frame[int(e.group(6))] = (float(e.group(1)), float(e.group(2)))
        frames.append(frame)
    clips[name] = frames

BODY = {i for i, t in enumerate(tracks) if not any(k in t for k in ('spark', 'line', 'glow', 'blade', 'ring', 'sliver', 'ripple', 'streak', 'orb', 'bubble', 'visor', 'face_', 'jet'))}

def jump(a, b):
    common = [t for t in a if t in b and t in BODY]
    if not common:
        return 0.0, 0
    d = max(math.dist(a[t], b[t]) for t in common)
    pops = len([t for t in set(a) ^ set(b) if t in BODY])
    return d, pops

def within(frames):
    steps = [jump(frames[i], frames[i + 1])[0] for i in range(len(frames) - 1)]
    if not steps:
        return 0.0
    steps.sort()
    return steps[min(len(steps) - 1, int(0.95 * len(steps)))]

PAIRS = [
    ('idle', 'walk'), ('walk', 'idle'), ('walk', 'run'), ('run', 'run_stop'), ('run_stop', 'idle'),
    ('idle', 'jump_squat'), ('run', 'jump_squat'), ('jump_squat', 'jump'), ('jump', 'fall'),
    ('fall', 'land'), ('fall', 'land_hard'), ('land', 'idle'), ('land', 'run'), ('turnaround', 'run'),
    ('idle', 'crouch_start'), ('crouch_start', 'crouch'), ('crouch', 'crouch_end'), ('crouch_end', 'idle'),
    ('idle', 'jab'), ('jab', 'idle'), ('run', 'dash_attack'), ('dash_attack', 'idle'),
    ('fall', 'air_neutral'), ('air_neutral', 'fall'), ('hit', 'idle'), ('hit', 'fall'),
    ('fall', 'wall_slide'), ('wall_slide', 'wall_jump'), ('fall', 'ledge_catch'), ('ledge_catch', 'ledge_grab'),
    ('ledge_climb', 'idle'), ('idle', 'talk'), ('teeter', 'idle'), ('idle', 'shield_raise'),
]
print(f"{'transition':<26}{'end':>7}{'int_med':>8}{'int_max':>8}{'within_A':>9}{'within_B':>9}{'pops':>5}  verdict")
rows = []
for a, b in PAIRS:
    if a not in clips or b not in clips:
        print(f"{a+'->'+b:<26} missing clip")
        continue
    A, B = clips[a], clips[b]
    end, pops = jump(A[-1], B[0])
    ints = [jump(f, B[0])[0] for f in A]
    wa, wb = within(A), within(B)
    ref = max(wa, wb, 1.0)
    verdict = 'pop' if end > 2 * ref else ('interrupt-pop' if max(ints) > 2 * ref else 'ok')
    rows.append((a, b, end, verdict))
    print(f"{a+'->'+b:<26}{end:7.1f}{statistics.median(ints):8.1f}{max(ints):8.1f}{wa:9.1f}{wb:9.1f}{pops:5d}  {verdict}")
print()
print("clips:", len(clips), " body tracks:", len(BODY), "of", len(tracks))
print("pops at a natural end:", sum(1 for r in rows if r[3] == 'pop'), "of", len(rows))
print("pops only when interrupted:", sum(1 for r in rows if r[3] == 'interrupt-pop'), "of", len(rows))
