"""The Inugami's howl: auditions (2026-10-21).

Jon, 2026-10-21: "we probably need a howl sfx". The Inugami howls when it
rises, at its transition, when it comes back in the final hall, with its
howl move, and when it dies; every one of those plays the generic roar
(`boss.generic.intro_roar`), which is a low roar and not a howl. This
renders five candidates for a spirit's howl, each level-matched to the cue in
use, and a reel that plays the cue in use and then each candidate. Nothing
here changes a shipped cue; the chosen one is ported later as a recipe with
the same layers and seed.

Wolfenstein's own four howl candidates (2026-10-17, not chosen yet) are in
`untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`: the living wolf.
These are the wolf after it is free of him: a spirit, so more breath, a
longer room, and voices that are not quite one.

Run with the SFX renderer's venv, from the repo root:

    ~/.cache/ambition-tool-venvs/ambition_sfx_renderer/bin/python scripts/inugami_sfx_candidates.py

Writes `untracked/sfx-candidates/inugami/` (untracked): `recipes/`, `out/`,
`solo/howl__<candidate>.wav`, `reel/howl__all.wav` and `INDEX.md` (a copy
is committed beside this script as `inugami_sfx_candidates.md`).
"""

from __future__ import annotations

import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np
import soundfile as sf
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wolfenstein_sfx_candidates as W  # noqa: E402

HERE = W.REPO / "untracked/sfx-candidates/inugami"
IN_USE = "boss.generic.intro_roar"
W.HERE = HERE

C: dict[str, dict] = {}


def cand(cid, dur, layers, seed, note, room=None, peak=-4.0):
    C[cid] = dict(duration_ms=dur, seed=seed, layers=layers, note=note, postprocess=W.post(peak, 12000, True, room=room))


def wolf(start, dur, f0, spacing, *, breath, level=1.0, jitter=0.015, flutter=0.05, tilt=2600, doubling=0.03, f1=(320, 660)):
    """One howling throat: it rises to a held high note and falls away."""
    return W.voice(start, dur, f0, [[0, 0.3], [0.2, 0.85], [0.85, 0.8], [1, 0.45]],
                   [[0, 0], [0.12, 0.75], [0.3, 1], [0.85, 0.9], [1, 0]], spacing, breath=breath, doubling=doubling,
                   jitter=jitter, flutter=flutter, tilt=tilt, level=level, f1=f1)


def ghost_fire(name, start, dur, gain):
    """The rush of its fire: low, soft and wide, a little crackle in it."""
    return [W.noise(name, "pink", gain, start, dur, W.env(dur * 0.3, dur * 0.4, dur * 0.3), [W.bp(700, 0.5), W.lp(2600)]),
            W.grains(name + "_crackle", 2400, 0.8, gain - 6, start + dur * 0.1, dur * 0.8, 40, 2.0, dur * 0.8)]


cand("a_spirit", 2800, [W.throat("howl", 2800, [
    wolf(0, 2600, [[0, 420], [0.18, 610], [0.7, 595], [0.9, 560], [1, 470]], 720, breath=0.42, flutter=0.07)],
    seed=811, room=0.95, room_mix=0.4)], 811,
    "one wolf's howl, high and mostly breath, in a long cold room: the dog it was")
cand("b_chorus", 3000, [W.throat("howl", 3000, [
    wolf(0, 2600, [[0, 400], [0.18, 590], [0.7, 575], [1, 450]], 700, breath=0.3),
    wolf(90, 2600, [[0, 410], [0.2, 604], [0.7, 588], [1, 462]], 680, breath=0.36, level=0.6, jitter=0.02),
    wolf(170, 2500, [[0, 300], [0.2, 442], [0.7, 431], [1, 340]], 560, breath=0.4, level=0.45, jitter=0.02)],
    seed=812, room=0.9, room_mix=0.36)], 812,
    "three throats that are not quite one, a little apart in pitch and time: the spirits in it howl with it")
cand("c_giant", 3200, [W.throat("howl", 3200, [
    wolf(0, 3000, [[0, 260], [0.2, 420], [0.7, 405], [0.9, 380], [1, 300]], 470, breath=0.32,
         doubling=[[0, 0.4], [0.15, 0.06], [0.8, 0.1], [1, 0.3]], tilt=2200, f1=(280, 600))],
    seed=813, room=0.95, room_mix=0.38,
    sub={"start_ms": 100, "duration_ms": 2400, "level": 0.14, "freq_hz": [[0, 58], [0.3, 70], [1, 52]],
         "amp": [[0, 0], [0.25, 1], [1, 0]]})], 813,
    "the giant head: a bigger throat (closer formants), not a lower buzz; a sub under the held note")
cand("d_drawn_in", 3200, [
    W.sweep("breath_in", "pink", 300, 2400, 0.8, -12, 0, 650, W.env(600, 20, 40)),
    W.throat("howl", 3200, [
        wolf(600, 2500, [[0, 400], [0.18, 600], [0.7, 585], [1, 460]], 690, breath=0.36, flutter=0.06)],
        seed=814, room=0.95, room_mix=0.38)], 814,
    "a long breath drawn in, then the howl: the tell is in the sound")
cand("e_fire", 3200, [
    W.throat("howl", 3200, [
        wolf(0, 2900, [[0, 300], [0.2, 470], [0.7, 455], [0.9, 420], [1, 330]], 520, breath=0.34, tilt=2300, f1=(290, 620)),
        wolf(120, 2700, [[0, 304], [0.2, 476], [0.7, 460], [1, 336]], 540, breath=0.4, level=0.45, jitter=0.025)],
        seed=815, room=0.9, room_mix=0.34),
    *ghost_fire("fire", 0, 3000, -18)], 815,
    "the giant's howl in two near voices, and the rush of its red fire under it")


def main():
    HERE.mkdir(parents=True, exist_ok=True)
    (HERE / "recipes").mkdir(exist_ok=True)
    (HERE / "solo").mkdir(exist_ok=True)
    (HERE / "reel").mkdir(exist_ok=True)
    current = W.load(W.SHIPPED / IN_USE)
    paths = []
    for cid, spec in C.items():
        r = W.recipe("howl", cid, spec)
        r["id"] = f"audition.inugami.howl.{cid}"
        r["notes"] = f"Inugami audition howl/{cid}: {spec['note']}"
        p = HERE / "recipes" / f"{r['id']}.sfx.yaml"
        p.write_text(yaml.safe_dump(r, sort_keys=False))
        paths.append(p)
    with ThreadPoolExecutor(W.JOBS) as pool:
        list(pool.map(W.render, paths))
    audio = {cid: W.matched(W.load(HERE / "out" / f"audition.inugami.howl.{cid}"), current) for cid in C}
    # One headroom gain for the solos and one for the reel, so loudness stays
    # the comparison it was matched to.
    solos = {"current": current, **audio}
    gain = 0.89 / max(float(np.abs(a).max()) for a in solos.values())
    for cid, a in solos.items():
        sf.write(HERE / "solo" / f"howl__{cid}.wav", a * gain, W.SR)
    t, events, marks = 0.0, [], []
    for cid, a in solos.items():
        events.append((t, a, 1.0, 0))
        marks.append(f"{t:.1f} s {cid}")
        t += len(a) / W.SR + 0.8
    reel = W.mix(events, t)
    sf.write(HERE / "reel" / "howl__all.wav", reel * (0.89 / float(np.abs(reel).max())), W.SR)

    index = ["# The Inugami's howl: auditions", "",
             "Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in",
             "`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use",
             "(loudest 400 ms RMS).", "",
             "## The candidates", ""]
    index += [f"- `solo/howl__{cid}.wav`: {spec['note']}" for cid, spec in C.items()]
    index += [""] + W.table([("current `" + IN_USE + "`", current)] + list(audio.items()))
    index += ["", "## The reel", "", "- `reel/howl__all.wav`: " + "; ".join(marks), "",
              "Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`."]
    (HERE / "INDEX.md").write_text("\n".join(index) + "\n")
    Path(__file__).with_suffix(".md").write_text("\n".join(index) + "\n")
    print("\n".join(index))


if __name__ == "__main__":
    main()
