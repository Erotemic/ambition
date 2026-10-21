"""The Inugami's howl: auditions (2026-10-21).

Jon, 2026-10-21: "we probably need a howl sfx". The Inugami howls when it
rises, at its transition, when it comes back in the final hall, with its
howl move, and when it dies; every one of those plays the generic roar
(`boss.generic.intro_roar`), a low roar and not a howl.

Each candidate is a DESCRIBED howl (the SFX renderer's `howl` mode with a
`howl:` block of a dozen named numbers), level-matched to the cue in use,
and the reel plays the cue in use, each candidate and (if Jon's download is
there) the reference recording, which is a check and never a source of
data. Variations for the spectral wolves (level enemies) are changes to a
few of the numbers.

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


# Rounds 1-4 are in git history (round 1 a creature throat, 2 a pure tone on a
# contour read off Jon's spectrogram, 3 a throat on it, 4 the reference's
# pitch track copied).

# Round 5. Jon: "I don't want to extract raw data and reuse it from the
# sample, that wasn't the goal, the goal was to see what it takes to build a
# wolf sfx from first principles and build the concise text encoding of it
# that is interpretable." Then: "the current versions sound great. We want to
# be able to build variations as necessary, as we are going to also add
# spectral wolf level enemies."
#
# Round 4 copied the reference's pitch track (23 points) and loudness. Here a
# howl is its DESCRIPTION, a dozen named numbers (the SFX renderer's `howl`
# mode, `howl:` block): a yelp, a rise, a sagging hold, a fall that darkens,
# a droop. The reference is a check (scripts/howl_measure.py), and nothing of
# it is in a recipe. What came from it is the shape and a few round numbers a
# person could say: a held note near 650 Hz, a fall of about three and a half
# semitones, a second harmonic that grows from -35 to -7 dB as it falls.

REFERENCE = W.REPO / "untracked/sfx-candidates/810171__mokasza__lone-wolf-howling.mp3"


def howl(seed, **description):
    """One described howl as a layer, as long as its description."""
    return {"name": "howl", "kind": "noise_burst", "mode": "howl", "seed": seed, "start_ms": 0,
            "duration_ms": 1000.0 * described_seconds(description), "gain_db": 0.0, "howl": description}


def described_seconds(d):
    return sum(float(d.get(key, default)) for key, default in
               (("yelp_s", 0.5), ("rise_s", 0.12), ("hold_s", 1.8), ("fall_s", 1.5), ("droop_s", 0.3)))


#: The lone wolf of the reference, said in words: Jon's "sound great" round.
LONE = dict(pitch_hz=650, yelp_s=0.5, rise_s=0.12, hold_s=1.75, sag_semitones=0.8, fall_s=1.5, fall_semitones=3.6,
            fall_db=-25, droop_s=0.3, droop_semitones=1.6, bright_db=-9, dark_db=15, jitter_cents=15, shimmer=0.12,
            breath=0.05)


def variant(seed, note, room, **change):
    d = dict(LONE, **change)
    cand_id = note[0]
    C[cand_id] = dict(duration_ms=1000.0 * described_seconds(d) + 400, seed=seed, layers=[howl(seed, **d)],
                      note=note[1], postprocess=W.post(-4.0, 9000, True, room=room), description=d)


variant(851, ("a_lone", "the lone wolf, from its description alone: it should sound like round 4's a_measured"), 0.55)
variant(852, ("b_giant", "the Inugami: the lone wolf a major third lower, slower, in a bigger room"), 0.8,
        pitch_hz=520, hold_s=2.1, fall_s=1.8)
variant(853, ("c_spectral_wolf", "a spectral wolf (a level enemy): higher, shorter, a quick yelp"), 0.7,
        pitch_hz=760, yelp_s=0.35, hold_s=0.9, fall_s=1.0, fall_semitones=3.0)
variant(854, ("d_far_call", "a spectral wolf far off: no yelp, a long thin hold, more breath"), 0.95,
        yelp_s=0, pitch_hz=700, hold_s=2.4, fall_s=1.6, jitter_cents=11, breath=0.09)
variant(855, ("e_hurt", "a wolf hurt: a short yelp and a fast fall, no hold to speak of"), 0.5,
        pitch_hz=820, yelp_s=0.3, rise_s=0.06, hold_s=0.25, fall_s=0.55, fall_semitones=7, droop_s=0.2,
        droop_semitones=3, jitter_cents=24)


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
    index += [f"- `solo/howl__{cid}.wav`: {spec['note']}" + (
        "\n  `" + ", ".join(f"{k}: {v}" for k, v in spec["description"].items()) + "`" if "description" in spec else "")
        for cid, spec in C.items()]
    index += [""] + W.table([("current `" + IN_USE + "`", current)] + list(audio.items()))
    index += ["", "## The reel", "", "- `reel/howl__all.wav`: " + "; ".join(marks), "",
              "Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`."]
    (HERE / "INDEX.md").write_text("\n".join(index) + "\n")
    Path(__file__).with_suffix(".md").write_text("\n".join(index) + "\n")
    print("\n".join(index))


if __name__ == "__main__":
    main()
