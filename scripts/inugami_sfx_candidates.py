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


# Round 4. Jon: "Rough is bad it just wobbles. And yes they all sound
# synthetic." He chose a reference: "Lone wolf howling" by mokasza
# (freesound 810171, CC BY 4.0, made with an AI model), to be matched by
# the generator, not shipped. Measured (`measure_reference`):
# - 0.35-0.8 s, a rough yelp: two pitches at once (~650 and ~430 Hz), rich
#   harmonics, noise 18 dB under the voice;
# - then near a pure tone: h2 32-40 dB under f0 while it holds at ~650 Hz;
#   a slow fall to 620 Hz by 2.6 s, a faster one to 520 Hz by 4.0 s, where
#   h2 grows to -7 dB;
# - pitch jitter ~14 cents RMS, FAST (15-120 Hz); shimmer ~11% at 8-40 Hz;
# - loud to 2.75 s, -11 dB at 3.0 s, -20 at 3.5, -26 at 4.0, out by 4.4 s,
#   a short room after; almost no breath.

REFERENCE = W.REPO / "untracked/sfx-candidates/810171__mokasza__lone-wolf-howling.mp3"

#: The held howl's f0 from the reference: (seconds, Hz).
HELD = [(0.8, 600), (0.9, 642), (1.0, 652), (1.2, 651), (1.5, 648), (1.8, 643), (2.0, 638), (2.2, 631), (2.4, 624),
        (2.6, 620), (2.7, 612), (2.8, 598), (2.9, 588), (3.0, 582), (3.1, 570), (3.2, 557), (3.4, 550), (3.6, 541),
        (3.8, 527), (4.0, 517), (4.1, 500), (4.3, 470), (4.4, 455)]
#: Its loudness: (seconds, dB).
LOUD = [(0.8, -6), (0.9, 0), (2.75, -1), (3.0, -11), (3.5, -20), (4.0, -26), (4.3, -40), (4.4, -80)]


def span(points, t0, t1):
    """`points` (seconds, value) as fractions of the span t0..t1."""
    return [[round((t - t0) / (t1 - t0), 5), v] for t, v in points]


def db_amp(points):
    return [[u, round(10 ** (db / 20.0), 5)] for u, db in points]


def held_howl(name, *, seed, scale_f=1.0, t0=0.8, t1=4.4, jitter=0.012, shimmer=0.12, h2_hold=-9.0, h2_fall=15.0):
    f0 = [[u, round(hz * scale_f, 2)] for u, hz in span(HELD, t0, t1)]
    return {"name": name, "kind": "noise_burst", "mode": "howl", "seed": seed, "start_ms": t0 * 1000.0,
            "duration_ms": (t1 - t0) * 1000.0, "gain_db": 0.0, "f0_hz": f0, "amp": db_amp(span(LOUD, t0, t1)),
            "mouth": [[0, 0.5], [1, 0.5]], "harmonics": 5, "tilt_db_per_octave": -14.0,
            # One resonance on f0: the held note is near a pure tone.
            "formants": [[650 * scale_f, 650 * scale_f, 600, 1.0]], "floor": 0.0006,
            "harmonic_db": {"2": span([(t0, h2_hold), (2.7, h2_hold), (3.2, h2_fall), (t1, h2_fall)], t0, t1),
                            "3": [[0, -18], [1, -18]]},
            "jitter": jitter, "jitter_hz": 90, "drift_cents": 0.0, "shimmer": shimmer, "shimmer_hz": 24,
            "breath": 0.05, "breath_curve": span([(t0, 1), (2.9, 1), (3.3, 3), (4.2, 5)], t0, t1), "onset_ms": 1, "onset_rough": 0.0}


def yelp(name, *, seed, start, dur, f0, scale_f=1.0, gain=0.0, h2=0.0, h3=0.0, floor=0.12, breath=0.3):
    """The rough start: a short, harsh, breathy voice."""
    return {"name": name, "kind": "noise_burst", "mode": "howl", "seed": seed, "start_ms": start * 1000.0,
            "duration_ms": dur * 1000.0, "gain_db": gain, "f0_hz": [[u, round(hz * scale_f, 2)] for u, hz in f0],
            "amp": [[0, 0], [0.12, 1], [0.8, 0.9], [1, 0]], "mouth": [[0, 0.6], [1, 0.6]], "harmonics": 8,
            "tilt_db_per_octave": -9.0, "formants": [[800 * scale_f, 800 * scale_f, 420, 1.0]], "floor": floor,
            "harmonic_db": {"2": [[0, h2], [1, h2]], "3": [[0, h3], [1, h3]]}, "jitter": 0.02, "jitter_hz": 90,
            "drift_cents": 3, "shimmer": 0.25, "shimmer_hz": 30, "breath": breath, "onset_ms": dur * 600.0, "onset_rough": 0.5}


def the_howl(seed, scale_f=1.0, with_yelp=True):
    layers = [held_howl("held", seed=seed, scale_f=scale_f)]
    if with_yelp:
        layers += [yelp("yelp_hi", seed=seed + 1, start=0.33, dur=0.32, f0=[[0, 560], [0.3, 660], [1, 650]], scale_f=scale_f,
                        h2=-30.0, h3=-30.0, floor=0.0005, breath=0.25),
                   yelp("yelp_lo", seed=seed + 2, start=0.56, dur=0.3, f0=[[0, 431], [0.6, 425], [1, 590]], scale_f=scale_f,
                        gain=-2.0, h2=6.0, h3=6.0, floor=0.01, breath=0.12),
                   # The reference's yelp noise is a blob under ~1.2 kHz.
                   W.noise("yelp_air", "pink", -4, 340, 380, W.env(20, 200, 160),
                           [W.bp(650 * scale_f, 1.1), W.lp(1300 * scale_f), W.lp(1300 * scale_f)])]
    else:
        # No yelp: the held note comes in on a quick scoop.
        layers[0]["f0_hz"][0][1] = round(540 * scale_f, 2)
    return layers


cand("a_measured", 5000, the_howl(841), 841,
     "the reference, by its numbers: the rough two-voice yelp, then a near-pure held note that falls and darkens",
     room=0.55)
cand("b_no_yelp", 5000, the_howl(842, with_yelp=False), 842, "the same held howl with no yelp at its start", room=0.55)
cand("c_giant", 5000, the_howl(843, scale_f=0.8), 843, "a_measured a major third lower, in a bigger room: the giant head",
     room=0.8)
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
    if REFERENCE.exists():
        import subprocess as sp
        ref_wav = HERE / "out" / "reference.wav"
        sp.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(REFERENCE), "-ar", str(W.SR), "-ac", "2", str(ref_wav)], check=True)
        solos["reference"] = W.matched(sf.read(ref_wav, always_2d=True)[0].astype(np.float32), current)
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
