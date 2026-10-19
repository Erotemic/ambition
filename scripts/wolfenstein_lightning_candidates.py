"""Wolfenstein's lightning SFX auditions (2026-10-18): a zap, and a strike.

Jon, 2026-10-18: "The initial lightning sounds needs to sound more like a
zap+thunderclap rather than an upward beep, we can do better for lightning
sfx. Not every lightning should thunderclap, but we do it for effect and when
there is an impact."

So two sounds. The ZAP is every arc that hits nothing: a coil crackling, a
flash across the throne. The STRIKE is a zap with a thunderclap behind it:
the lightning that strikes him (the coils' jolts, the roar, the transition,
the serum) and the galvanize's strikes on the floor, and the first flash of
his intro, for effect. Three candidates each, level-matched to the cue in use
(`vfx.generic_world.electric_burst`), and reels of his intro's timeline that
pair them.

Run with the SFX renderer's venv, from the repo root:

    ~/.cache/ambition-tool-venvs/ambition_sfx_renderer/bin/python scripts/wolfenstein_lightning_candidates.py

Writes `untracked/sfx-candidates/wolfenstein/` (`solo/zap__*`,
`solo/strike__*`, `reel/intro__*`) and `LIGHTNING.md` there (copied beside
this script as `wolfenstein_lightning_candidates.md`). It reuses the recipe
pieces and the render, load and level-match of
`wolfenstein_sfx_candidates.py`.
"""

from __future__ import annotations

import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import soundfile as sf
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wolfenstein_sfx_candidates as W  # noqa: E402
from wolfenstein_sfx_candidates import env, grains, hp, lp, bp, noise, sweep, tone  # noqa: E402

C: dict[tuple[str, str], dict] = {}


def cand(sound, cid, dur, layers, seed, peak=-4.0, low=15000, comp=True, room=None, note=""):
    C[(sound, cid)] = dict(duration_ms=dur, seed=seed, layers=layers, note=note,
                           postprocess=W.post(peak, low, comp, room=room))


def crack(gain=0.0, hz=3400, start=0):
    """The snap of the discharge itself: a few hard grains in 20 ms."""
    return grains("crack", hz, 0.45, gain, start, 70, 6, 1.4, 18)


# THE ZAP: an arc that hits nothing. A crack and a sizzle; no tone that rises.
cand("zap", "a_crack_sizzle", 450, [
    crack(),
    grains("crackle", 5200, 0.8, -9, 15, 380, 26, 0.9, 340),
    noise("sizzle", "white", -15, 0, 450, env(1, 60, 380), [bp(6800, 0.6)])], 801,
    note="a hard crack and a sizzling crackle that dies away")
cand("zap", "b_whip", 420, [
    crack(hz=4200),
    sweep("whip", "white", 9500, 1400, 1.4, -6, 0, 200, env(0, 20, 180)),
    grains("crackle", 4600, 0.8, -12, 60, 340, 18, 1.0, 300)], 802,
    note="a crack that whips down through the air, crackle after")
cand("zap", "c_coil_snap", 520, [
    crack(-1.0, hz=3000),
    tone("corona", "saw", 118, 112, -15, 0, 300, env(1, 80, 220), tremolo={"rate_hz": 60, "depth": 0.9},
         noise_amount=0.4, effects=[lp(3200), hp(90)]),
    grains("crackle", 4800, 0.8, -10, 20, 480, 24, 1.0, 440)], 803,
    note="a Tesla coil's snap: the crack, a short mains buzz, crackle")

# THE STRIKE: the zap, and a thunderclap behind it.
cand("strike", "a_close_clap", 2000, [
    crack(),
    noise("tear", "pink", -7, 0, 320, env(0, 30, 280), [bp(2400, 0.5)]),
    noise("clap", "white", -3, 25, 140, env(0, 10, 125), [lp(3600), hp(120)]),
    noise("rumble", "brown", -5, 60, 1940, env(30, 200, 1600), [lp(380)])], 811, low=14000,
    note="close thunder: the crack, a tearing clap, a long rumble")
cand("strike", "b_zap_then_boom", 2300, [
    crack(hz=4000),
    sweep("whip", "white", 9000, 1600, 1.4, -8, 0, 180, env(0, 20, 160)),
    tone("boom", "sine", 95, 34, -3, 70, 900, env(2, 90, 800)),
    noise("clap", "white", -5, 70, 160, env(0, 12, 140), [lp(2800), hp(100)]),
    noise("rumble", "brown", -6, 110, 2190, env(40, 250, 1800), [lp(320)])], 812, low=14000, room=0.7,
    note="the zap, then a boom: a sub drop under a clap, a rumble in a big room")
cand("strike", "c_split_sky", 2600, [
    crack(),
    grains("tear", 2800, 0.6, -6, 10, 320, 30, 1.6, 280),
    noise("clap", "white", -4, 30, 150, env(0, 10, 135), [lp(3200), hp(120)]),
    noise("roll_a", "brown", -7, 80, 1200, env(40, 150, 1000), [lp(420)]),
    noise("roll_b", "brown", -9, 700, 1900, env(200, 300, 1400), [lp(300)])], 813, low=14000,
    note="the sky splits: the crack and a tearing crackle, a clap, thunder that rolls twice")


def main():
    W.HERE.mkdir(parents=True, exist_ok=True)
    ref = W.load(W.SHIPPED / W.IN_USE["bolt"])
    arc = W.load(W.SHIPPED / W.IN_USE["arc"])
    howl = W.load(W.SHIPPED / W.IN_USE["howl"])
    (W.HERE / "recipes").mkdir(exist_ok=True)
    paths = []
    for (sound, cid), spec in C.items():
        p = W.HERE / "recipes" / f"{W.cue_id(sound, cid)}.sfx.yaml"
        p.write_text(yaml.safe_dump(W.recipe(sound, cid, spec), sort_keys=False))
        paths.append(p)
    with ThreadPoolExecutor(W.JOBS) as pool:
        list(pool.map(W.render, paths))
    got = {key: W.matched(W.load(W.HERE / "out" / W.cue_id(*key)), ref) for key in C}

    (W.HERE / "solo").mkdir(exist_ok=True)
    for sound in ("zap", "strike"):
        keys = [k for k in got if k[0] == sound]
        room = min(1.0, 0.95 / max(float(abs(got[k]).max()) for k in keys))
        for k in keys:
            sf.write(W.HERE / "solo" / f"{sound}__{k[1]}.wav", got[k] * room, W.SR)

    # His intro (wolfenstein.rs, `FLASHES`, `JOLTS`, `ROAR_AT`): which arcs
    # zap and which strike. The first flash strikes for effect; the bright
    # one of the flurry strikes; the jolts and the roar's bolt hit him.
    def intro(zap, strike):
        return [(0.9, strike, 1.0, -0.3, "flash: the coils fire, he is seen (strike)"),
                (1.3, zap, 0.35, 0.3, "crackle"), (1.7, zap, 0.35, -0.3, ""), (2.1, zap, 0.35, 0.3, ""),
                (2.3, zap, 0.8, 0.3, "flash (zap)"), (2.55, zap, 0.7, -0.3, "flash (zap)"),
                (2.95, strike, 1.0, 0.0, "flash (strike)"), (3.2, zap, 0.75, 0.3, "flash (zap)"),
                (3.45, zap, 0.5, -0.3, "the charge"), (3.75, zap, 0.55, 0.3, ""), (3.95, zap, 0.6, -0.3, ""),
                (4.1, strike, 1.0, 0.0, "jolt (strike)"), (4.6, strike, 1.0, 0.0, "jolt (strike)"),
                (5.8, strike, 1.0, 0.0, "the roar's bolt (strike)"), (5.8, howl, 0.9, 0.0, "the howl (in use)")]

    reels = [("intro__current", "the cues in use: the arc and the burst", intro(arc, ref))]
    for z, s in (("a_crack_sizzle", "a_close_clap"), ("b_whip", "b_zap_then_boom"), ("c_coil_snap", "c_split_sky")):
        reels.append((f"intro__{z}__{s}", f"zap {z}, strike {s}", intro(got[("zap", z)], got[("strike", s)])))
    (W.HERE / "reel").mkdir(exist_ok=True)
    audio = {name: W.mix([e[:4] for e in events], 8.5) for name, _, events in reels}
    gain = 0.7 / max(float(abs(a).max()) for a in audio.values())
    for name, a in audio.items():
        sf.write(W.HERE / "reel" / f"{name}.wav", a * gain, W.SR)

    index = ["# Wolfenstein's lightning: the zap and the strike", "",
             "Written by `scripts/wolfenstein_lightning_candidates.py` (2026-10-18). Audio in",
             "`untracked/sfx-candidates/wolfenstein/`. Matched to the cue in use (loudest 400 ms RMS).", "",
             "The ZAP is an arc that hits nothing; the STRIKE is a zap with a thunderclap, for effect and",
             "for an impact.", ""]
    for sound in ("zap", "strike"):
        index += [f"## {sound}", ""]
        index += [f"- `solo/{sound}__{cid}.wav`: {spec['note']}" for (s, cid), spec in C.items() if s == sound]
        index += [""] + W.table([("current", ref)] + [(cid, got[(s, cid)]) for s, cid in C if s == sound]) + [""]
    index += ["## The reels: his intro, 8.5 s", ""]
    for name, what, events in reels:
        marks = "; ".join(f"{e[0]:.2f}s {e[4]}" for e in events if e[4])
        index.append(f"- `reel/{name}.wav` ({what}): {marks}")
    (W.HERE / "LIGHTNING.md").write_text("\n".join(index) + "\n")
    Path(__file__).with_suffix(".md").write_text("\n".join(index) + "\n")
    print(f"{len(C)} candidates, {len(reels)} reels -> {W.HERE}")


if __name__ == "__main__":
    main()
