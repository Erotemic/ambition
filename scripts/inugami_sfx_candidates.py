"""The Inugami's howl: auditions (2026-10-21).

Jon, 2026-10-21: "we probably need a howl sfx". The Inugami howls when it
rises, at its transition, when it comes back in the final hall, with its
howl move, and when it dies; every one of those plays the generic roar
(`boss.generic.intro_roar`), which is a low roar and not a howl. This
renders five candidates for its howl (round 2: on the contour of a real
wolf's howl, from the spectrogram Jon sent), each level-matched to the cue in
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
    C[cid] = dict(duration_ms=dur, seed=seed, layers=layers, note=note, postprocess=W.post(peak, 9000, True, room=room))


# Round 2. Jon: "None of these are good. a_spirit doesn't have a howl at
# all", with a spectrogram of a wolf's howl. Round 1 voiced the howl in the
# creature throat (pulses, formants, breath): a roar's model. A wolf's howl
# is a near-pure tone. In his picture f0 starts at ~375 Hz, steps up at
# 0.85 s and peaks at ~620 Hz at 1.7 s, dips a little (a fast frequency
# modulation), then glides down over four seconds, with a step at 4.5 s, to
# ~440 Hz at 6.2 s (mean ~520 Hz). The 2nd harmonic is the brightest line,
# f0 is under it, the 3rd is faint, and there is almost no noise.

#: The howl's f0 contour from the spectrogram: (seconds, Hz).
HOWL = [(0.0, 375), (0.3, 400), (0.55, 430), (0.8, 450), (0.86, 520), (1.0, 540), (1.3, 590), (1.7, 620), (1.76, 610),
        (1.8, 562), (1.86, 556), (2.5, 540), (3.5, 520), (4.4, 505), (4.47, 476), (5.5, 465), (6.0, 445)]
#: Its harmonics, in the brightness the picture shows.
VOICE = [{"ratio": 1.0, "gain_db": -6.0}, {"ratio": 2.0, "gain_db": 0.0}, {"ratio": 3.0, "gain_db": -14.0},
         {"ratio": 4.0, "gain_db": -26.0}]


def contour(scale_t=1.0, scale_f=1.0, cents_drift=10.0, seed=0, dur_s=None):
    """The howl's contour as `points` over the layer: its time scaled by
    `scale_t`, its pitch by `scale_f`, and a slow wander of a few cents (a
    voice is never a dial)."""
    rng = np.random.default_rng(seed)
    pts = [(t * scale_t, hz * scale_f) for t, hz in HOWL]
    total = dur_s or pts[-1][0]
    out = []
    walk = 0.0
    k = 0
    for t, hz in pts:
        out.append((t, hz))
    # The wander: a smoothed random walk sampled every 0.12 s, laid over the
    # contour between its own points.
    dense = []
    t = 0.0
    while t <= total:
        walk = 0.8 * walk + rng.normal(0.0, cents_drift * 0.45)
        base = float(np.exp(np.interp(t, [q for q, _ in pts], np.log([h for _, h in pts]))))
        dense.append((t, base * 2 ** (walk / 1200.0)))
        t += 0.12
        k += 1
    # Keep the contour's own steps sharp: its points win over the wander's.
    merged = sorted(dense + out)
    keep = []
    for t, hz in merged:
        if keep and abs(t - keep[-1][0]) < 0.03:
            if (t, hz) in out:
                keep[-1] = (t, hz)
            continue
        keep.append((t, hz))
    return [[round(t / total, 5), round(hz, 2)] for t, hz in keep if t <= total]


def howl_layer(name, start_ms, dur_s, *, scale_f=1.0, seed=0, gain=0.0, harmonics=None, pan=0.0, detune=0.0):
    dur_ms = dur_s * 1000.0
    return {"name": name, "kind": "synth_tone", "waveform": "sine", "seed": seed,
            "frequency_hz": {"points": contour(dur_s / HOWL[-1][0], scale_f * 2 ** (detune / 1200.0), seed=seed, dur_s=dur_s),
                             "curve": "exp"},
            "amplitude_points": [[0, 0.0], [0.03, 0.55], [0.14, 0.92], [0.28, 1.0], [0.7, 0.86], [0.9, 0.6], [1, 0.0]],
            "harmonics": harmonics or VOICE, "vibrato": {"rate_hz": 4.6, "depth_cents": 6, "delay_ms": 600},
            "stereo_spread_cents": 4.0, "gain_db": gain, "start_ms": start_ms, "duration_ms": dur_ms, "pan": pan,
            "envelope": W.env(25, dur_ms - 400, 375)}


def air(name, start_ms, dur_ms, gain):
    """A little breath under the tone: there is almost none in the picture."""
    return W.noise(name, "pink", gain, start_ms, dur_ms, W.env(120, dur_ms - 600, 480), [W.bp(1100, 0.7), W.lp(3000)])


# Round 3. Jon: "Ok howl a wolf is better, the others feel like there are
# many wolfs in the pack, but it sounds like a siren, it doesn't have the
# timbre of a wolf." One voice, a_wolf's contour, and a throat: the SFX
# renderer's `howl` mode (formants that move with the mouth, jitter and
# shimmer as random walks, breath pulsed with the voice, a rough onset).
# Round 2's a_wolf is kept as `z_siren`, the pure tone, to compare.

#: The howl's end: the voice droops as it gives out.
DROOP = [(6.1, 430), (6.3, 392)]


def throat_howl(name, *, seed, scale_f=1.0, formants=None, mouth=None, jitter=0.006, shimmer=0.1, breath=0.06,
                onset_ms=180, onset_rough=0.35, drift=10.0, tilt=-10.0, gain=0.0):
    dur_s = DROOP[-1][0]
    pts = [[round(t / dur_s, 5), round(hz * scale_f, 2)] for t, hz in HOWL + DROOP]
    return {"name": name, "kind": "noise_burst", "mode": "howl", "seed": seed, "duration_ms": dur_s * 1000.0,
            "gain_db": gain, "start_ms": 0, "f0_hz": pts,
            "amp": [[0, 0], [0.02, 0.5], [0.1, 0.9], [0.28, 1.0], [0.75, 0.85], [0.93, 0.55], [1, 0]],
            "mouth": mouth or [[0, 0.15], [0.12, 0.5], [0.28, 1.0], [0.55, 0.8], [0.85, 0.55], [1, 0.05]],
            "harmonics": 12, "tilt_db_per_octave": tilt,
            # Three lines, as in Jon's picture: f0, the 2nd (brightest) and a
            # faint 3rd. A harmonic far from every formant keeps almost
            # nothing; six bright lines were brass, not a wolf.
            "formants": formants or [[780, 1150, 520, 1.0], [1650, 2100, 600, 0.22], [2900, 3300, 800, 0.05]],
            "floor": 0.008, "jitter": jitter, "jitter_hz": 28, "drift_cents": drift, "shimmer": shimmer, "shimmer_hz": 22,
            "breath": breath, "onset_ms": onset_ms, "onset_rough": onset_rough}


cand("a_throat", 6500, [throat_howl("howl", seed=831)], 831,
     "a_wolf's howl in a throat: the vowel opens to the peak and closes as it falls, a little jitter, a little breath",
     room=0.45)
cand("b_breathy", 6500, [throat_howl("howl", seed=832, breath=0.16, shimmer=0.16, jitter=0.008)], 832,
     "more air in it: breath through the throat with every pulse, a less steady voice", room=0.45)
cand("c_rough", 6500, [throat_howl("howl", seed=833, jitter=0.012, shimmer=0.18, onset_ms=420, onset_rough=0.6,
                                   breath=0.1, drift=18.0)], 833,
     "a rough voice: it starts with a growl under it, and wavers more as it goes", room=0.45)
cand("d_dark", 6500, [throat_howl("howl", seed=834, mouth=[[0, 0.05], [0.15, 0.3], [0.3, 0.6], [0.6, 0.45], [1, 0.0]],
                                  tilt=-12.0, breath=0.08)], 834,
     "a darker vowel: the mouth opens less, an 'oo' more than an 'ow'", room=0.45)
cand("e_big_throat", 6500, [throat_howl("howl", seed=835, scale_f=0.86,
                                        formants=[[620, 950, 460, 1.0], [1350, 1750, 560, 0.25], [2400, 2800, 760, 0.06]],
                                        breath=0.09, jitter=0.008)], 835,
     "a bigger throat (formants lower), the howl a little lower: the giant head", room=0.6)
cand("z_siren", 6400, [howl_layer("howl", 0, 6.2, seed=1), air("air", 0, 6200, -34)], 821,
     "round 2's a_wolf, the pure tone, to compare", room=0.45)


def main():
    import shutil
    # Each round replaces the last: its solos and outputs are not this round's.
    for sub in ("solo", "out", "recipes", "reel"):
        shutil.rmtree(HERE / sub, ignore_errors=True)
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
