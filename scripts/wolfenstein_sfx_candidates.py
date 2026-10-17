"""Wolfenstein fight SFX auditions (2026-10-17): candidates and in-context reels.

Jon, 2026-10-17: "We need to make sure we have good sfx for the Wolfenstein
fight." Every sound of the fight was borrowed from another boss (a bear's
swipe, the T-rex's chomp and growls, a robot's lunge and spark, the generic
roar). This renders the sounds now in use, three or four candidates for each
sound worth replacing, and reels that play them in the fight's own timing, so
Jon can choose by ear. Nothing here changes a shipped cue; a chosen candidate
is ported later as a recipe with the same layers and seed (the loop of
`untracked/sfx-candidates/robot_blade/make.py`).

Run with the SFX renderer's venv, from the repo root:

    ~/.cache/ambition-tool-venvs/ambition_sfx_renderer/bin/python scripts/wolfenstein_sfx_candidates.py
    ... scripts/wolfenstein_sfx_candidates.py measure   # only the table of the cues in use

Writes `untracked/sfx-candidates/wolfenstein/` (untracked): `recipes/`,
`out/`, `solo/<sound>__<candidate>.wav`, `reel/<reel>.wav` and `INDEX.md`
(a copy of the index is committed beside this script as
`wolfenstein_sfx_candidates.md`). Each solo and each reel slot is level-
matched to the cue in use (loudest 400 ms RMS, within 1 dB), so a choice is
not a choice of the louder one.

The voice: the renderer's `creature` mode (the T-rex's throat). Size is in
the formant spacing (memory: a low buzz reads as an engine, not an animal):
the T-rex is 290 Hz; this wolf is 480-700 Hz, a big dog's throat in a giant.
"""

from __future__ import annotations

import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np
import soundfile as sf
import yaml

REPO = Path(__file__).resolve().parents[1]
RENDERER = REPO / "tools/ambition_sfx_renderer"
SHIPPED = RENDERER / "output"
HERE = REPO / "untracked/sfx-candidates/wolfenstein"
PY = sys.executable
SR = 48000
JOBS = 6

# The cues the fight plays now (wolfenstein.rs and his telegraphs in
# boss_profiles.ron).
IN_USE = {
    "rake": "boss.bear_mauler.swipe",
    "bite": "boss.trex.chomp",
    "lunge": "enemy.robot.guardian_lunge",
    "hammer": "boss.gnu_ton.stomp",
    "shock": "vfx.explosion.shockwave",
    "howl": "boss.generic.intro_roar",
    "arc": "vfx.generic_world.electric_arc",
    "bolt": "vfx.generic_world.electric_burst",
    "sputter": "enemy.robot.death_spark",
    "step": "player.land",
    "land": "boss.trex.stomp",
    "tell_snarl": "boss.trex.growl_snarl_a",
    "tell_grunt": "boss.trex.growl_grunt_a",
    "tell_huff": "boss.trex.growl_huff_a",
    "tell_rise": "boss.trex.growl_rise_a",
    "tell_lash": "ability.lightning.arc_loop",
    "tell_charge": "ability.lightning.charge",
}


# ── Recipe pieces ────────────────────────────────────────────────────────────
def env(a, h, r):
    return {"attack_ms": a, "hold_ms": h, "release_ms": r}


def bp(hz, q):
    return {"effect": "bandpass", "center_hz": hz, "q": q}


def lp(hz):
    return {"effect": "lowpass", "cutoff_hz": hz}


def hp(hz):
    return {"effect": "highpass", "cutoff_hz": hz}


def grains(name, hz, q, gain, start, dur, count, decay, spread, color="white", pan=0.0):
    return {"name": name, "kind": "noise_burst", "mode": "grains", "color": color, "grain_count": count,
            "grain_decay_ms": decay, "spread_ms": spread, "grain_start_ms": 0, "gain_db": gain,
            "start_ms": start, "duration_ms": dur, "pan": pan, "envelope": env(0, dur * 0.4, dur * 0.6),
            "effects": [bp(hz, q)]}


def noise(name, color, gain, start, dur, e, effects, mode="burst", pan=0.0):
    return {"name": name, "kind": "noise_burst", "mode": mode, "color": color, "gain_db": gain,
            "start_ms": start, "duration_ms": dur, "pan": pan, "envelope": e, "effects": effects}


def sweep(name, color, a, b, q, gain, start, dur, e, pan=0.0):
    return {"name": name, "kind": "noise_burst", "mode": "air_sweep", "color": color, "start_hz": a,
            "end_hz": b, "q": q, "gain_db": gain, "start_ms": start, "duration_ms": dur, "pan": pan, "envelope": e}


def tone(name, wave, a, b, gain, start, dur, e, harmonics=None, tremolo=None, noise_amount=0.0, effects=None, curve="exp"):
    layer = {"name": name, "kind": "synth_tone", "waveform": wave,
             "frequency_hz": {"start": a, "end": b, "curve": curve}, "gain_db": gain, "start_ms": start,
             "duration_ms": dur, "envelope": e}
    if harmonics:
        layer["harmonics"] = harmonics
    if tremolo:
        layer["tremolo"] = tremolo
    if noise_amount:
        layer["noise_amount"] = noise_amount
    if effects:
        layer["effects"] = effects
    return layer


def mode(name, m, gain, start, dur, e, effects=None, **kw):
    return {"name": name, "kind": "noise_burst", "mode": m, "gain_db": gain, "start_ms": start,
            "duration_ms": dur, "envelope": e, "effects": effects or [], **kw}


def voice(start, dur, f0, jaw, amp, spacing, breath=0.4, doubling=0.2, jitter=0.05, flutter=0.08,
          tilt=2000, level=1.0, f1=(300, 700), breaks=None):
    v = {"start_ms": start, "duration_ms": dur, "level": level, "f0_hz": f0, "jaw": jaw, "amp": amp,
         "doubling": doubling if isinstance(doubling, list) else [[0, doubling], [1, doubling]],
         "spacing_hz": spacing, "f1_hz": list(f1), "breath": breath, "jitter": jitter, "tilt_hz": tilt,
         "flutter": flutter, "formants": 8}
    if breaks:
        v["breaks"] = breaks
    return v


def throat(name, dur, voices, seed, room=0.35, room_mix=0.18, saturate=1.4, sub=None, gain=0.0):
    layer = {"name": name, "kind": "noise_burst", "mode": "creature", "seed": seed, "duration_ms": dur,
             "gain_db": gain, "voices": voices,
             "finish": {"shape_db": [[0, 0], [0.8, 0], [1, -60]], "saturate": saturate, "room_s": room,
                        "room_mix": room_mix, "room_seed": 7, "cut_ms": 25, "peak_db": -6}}
    if sub:
        layer["sub"] = sub
    return layer


def post(peak=-4.0, low=12000, comp=True, out_ms=30, room=None):
    chain = [hp(40), lp(low)]
    if comp:
        chain.append({"effect": "compressor", "threshold_db": -18, "ratio": 2.4, "attack_ms": 2, "release_ms": 80})
    if room:
        chain.append({"effect": "reverb", "room_size": room, "damping": 0.6, "wet_level": 0.12, "dry_level": 1.0})
    chain += [{"effect": "fade_edges", "fade_in_ms": 0.5, "fade_out_ms": out_ms},
              {"effect": "normalize_peak", "target_db": peak}]
    return chain


# ── The candidates: (sound, id) -> spec ──────────────────────────────────────
C: dict[tuple[str, str], dict] = {}


def cand(sound, cid, dur, layers, seed, peak=-4.0, low=12000, comp=True, room=None, note=""):
    C[(sound, cid)] = dict(duration_ms=dur, seed=seed, layers=layers, note=note,
                           postprocess=post(peak, low, comp, room=room))


# HOWL. The wolf in him, and the thing that calls the pack.
cand("howl", "a_lone_wolf", 2400, [throat("howl", 2400, [voice(
    0, 2200, [[0, 330], [0.18, 520], [0.7, 505], [0.9, 470], [1, 400]], [[0, 0.3], [0.2, 0.85], [0.85, 0.8], [1, 0.45]],
    [[0, 0], [0.12, 0.75], [0.3, 1], [0.85, 0.9], [1, 0]], 700, breath=0.16, doubling=0.02, jitter=0.012,
    flutter=0.04, tilt=2600, f1=(320, 640))], seed=711, room=0.6, room_mix=0.24)], 711,
    note="a classic wolf: one voice, rising to a held high note, a wide room")
cand("howl", "b_giant", 2600, [throat("howl", 2600, [voice(
    0, 2400, [[0, 190], [0.2, 360], [0.7, 345], [0.9, 310], [1, 240]], [[0, 0.2], [0.22, 0.95], [0.85, 0.85], [1, 0.4]],
    [[0, 0], [0.12, 0.8], [0.3, 1], [0.85, 0.9], [1, 0]], 480, breath=0.34,
    doubling=[[0, 0.55], [0.15, 0.08], [0.8, 0.12], [1, 0.4]], jitter=0.03, flutter=0.06, tilt=2100, f1=(280, 600))],
    seed=712, room=0.7, room_mix=0.26, sub={"start_ms": 0, "duration_ms": 1800, "level": 0.18,
                                            "freq_hz": [[0, 52], [0.3, 64], [1, 48]], "amp": [[0, 0], [0.2, 1], [1, 0]]})], 712,
    note="a giant's wolf: lower, a growl at the start and the end, a sub under it")
cand("howl", "c_stitched", 2600, [throat("howl", 2600, [
    voice(0, 2300, [[0, 300], [0.2, 470], [0.7, 455], [1, 380]], [[0, 0.3], [0.2, 0.85], [1, 0.5]],
          [[0, 0], [0.12, 0.8], [0.3, 1], [0.85, 0.9], [1, 0]], 640, breath=0.2, doubling=0.04, jitter=0.015, tilt=2500),
    voice(80, 2200, [[0, 120], [0.2, 182], [0.7, 176], [1, 150]], [[0, 0.2], [0.25, 0.7], [1, 0.4]],
          [[0, 0], [0.15, 0.7], [0.4, 1], [0.85, 0.85], [1, 0]], 1000, breath=0.45, doubling=0.15, jitter=0.04,
          tilt=1500, level=0.55, f1=(350, 750))], seed=713, room=0.6, room_mix=0.24)], 713,
    note="two throats at once: the wolf's howl and a man's moan an octave and a fifth under it")
cand("howl", "d_galvanic", 2600, [
    throat("howl", 2600, [voice(0, 2400, [[0, 200], [0.2, 365], [0.7, 350], [1, 250]],
                               [[0, 0.2], [0.22, 0.95], [1, 0.4]], [[0, 0], [0.12, 0.8], [0.3, 1], [0.85, 0.9], [1, 0]],
                               500, breath=0.3, doubling=[[0, 0.5], [0.15, 0.08], [1, 0.3]], jitter=0.03, tilt=2200)],
           seed=714, room=0.65, room_mix=0.24),
    grains("crackle_lo", 3200, 0.7, -16, 300, 2000, 70, 1.4, 1900),
    grains("crackle_hi", 6200, 0.9, -21, 500, 1800, 50, 0.9, 1700, pan=0.3)], 714,
    note="the giant's howl with the bolts in his neck crackling under it")

# THE TELLS (now the T-rex's growls). One wolf throat at three sizes, each
# with the four tells: the snarl (rake), the grunt (hammer), the huff (howl),
# the rise (lunge).
for size, spacing, extra, seed in (("a_wolf", 680, 0.0, 721), ("b_dire", 520, 0.0, 731), ("c_stitched", 560, 0.5, 741)):
    def man(start, dur, f0, amp):
        return [voice(start, dur, f0, [[0, 0.2], [1, 0.3]], amp, 1000, breath=0.5, doubling=0.3, jitter=0.05,
                      tilt=1400, level=extra, f1=(350, 700))] if extra else []
    cand("tell_snarl", size, 750, [throat("snarl", 750, [voice(
        0, 650, [[0, 150], [0.4, 185], [1, 170]], [[0, 0.35], [0.3, 0.55], [1, 0.5]],
        [[0, 0], [0.15, 0.8], [0.5, 1], [0.9, 0.9], [1, 0]], spacing, breath=0.45, doubling=0.75, jitter=0.09,
        flutter=0.14, tilt=2400)] + man(30, 600, [[0, 90], [1, 105]], [[0, 0], [0.2, 0.8], [0.9, 0.8], [1, 0]]),
        seed=seed, room=0.3)], seed, note="rake tell: a bared-teeth snarl")
    cand("tell_grunt", size, 1150, [throat("grunt", 1150, [voice(
        0, 1050, [[0, 95], [0.6, 120], [1, 130]], [[0, 0.1], [0.5, 0.3], [1, 0.45]],
        [[0, 0], [0.3, 0.6], [0.85, 1], [1, 0.0]], spacing, breath=0.35, doubling=0.85, jitter=0.07,
        flutter=0.1, tilt=1600)] + man(0, 1050, [[0, 70], [1, 85]], [[0, 0], [0.4, 0.6], [0.9, 1], [1, 0]]),
        seed=seed + 1, room=0.3)], seed + 1, note="hammer tell: a strained grunt that builds to the slam")
    cand("tell_huff", size, 900, [throat("huff", 900, [
        voice(0, 330, [[0, 140], [1, 120]], [[0, 0.6], [1, 0.4]], [[0, 0], [0.2, 1], [1, 0]], spacing, breath=0.92,
              doubling=0.3, jitter=0.1, tilt=3000),
        voice(420, 420, [[0, 150], [1, 125]], [[0, 0.7], [1, 0.5]], [[0, 0], [0.2, 1], [1, 0]], spacing, breath=0.9,
              doubling=0.3, jitter=0.1, tilt=3000)], seed=seed + 2, room=0.25)], seed + 2,
        note="howl tell: two deep breaths in through the nose")
    cand("tell_rise", size, 900, [throat("rise", 900, [voice(
        0, 820, [[0, 100], [0.7, 210], [1, 260]], [[0, 0.2], [0.8, 0.8], [1, 0.9]],
        [[0, 0], [0.3, 0.6], [0.9, 1], [1, 0]], spacing, breath=0.4, doubling=[[0, 0.8], [1, 0.3]], jitter=0.08,
        tilt=2300)] + man(0, 800, [[0, 70], [1, 120]], [[0, 0], [0.5, 0.6], [0.95, 1], [1, 0]]),
        seed=seed + 3, room=0.3)], seed + 3, note="lunge tell: a growl rising into the leap")

# THE LIGHTNING. The neck bolts' crackle (his arc, the lash tell, the aura).
cand("arc", "a_dry_snaps", 520, [
    grains("snaps", 4200, 0.8, -6, 0, 520, 34, 1.1, 470),
    grains("snaps_lo", 1700, 0.9, -12, 20, 480, 16, 1.8, 430, pan=-0.3)], 751, comp=False,
    note="dry snapping sparks, no hum")
cand("arc", "b_sizzle_buzz", 560, [
    tone("buzz", "square", 120, 118, -16, 0, 560, env(4, 360, 180), noise_amount=0.35,
         tremolo={"rate_hz": 19, "depth": 0.8}, effects=[lp(2600), hp(150)]),
    grains("snaps", 4600, 0.8, -8, 30, 520, 24, 1.0, 460),
    noise("sizzle", "white", -20, 0, 560, env(10, 300, 250), [bp(7000, 0.7)])], 752,
    note="an electric buzz with snaps and a sizzle on top")
cand("arc", "c_tesla", 600, [
    tone("corona", "saw", 300, 305, -17, 0, 600, env(3, 380, 200), harmonics=[
        {"ratio": 1.0, "gain_db": 0}, {"ratio": 2.01, "gain_db": -6}, {"ratio": 3.0, "gain_db": -10}],
        tremolo={"rate_hz": 47, "depth": 0.9}, effects=[bp(1800, 0.6)]),
    grains("discharge", 3600, 0.7, -7, 0, 600, 28, 1.3, 560)], 753,
    note="a Tesla coil: a fizzing corona tone and discharge snaps")

# The strike from the ceiling (galvanize x3, the transition's bolt).
cand("bolt", "a_thunderclap", 1800, [
    grains("crack", 2600, 0.45, 0, 0, 120, 7, 2.2, 28),
    noise("tear", "pink", -8, 0, 360, env(0, 40, 300), [bp(2200, 0.5)]),
    noise("rumble", "brown", -6, 30, 1770, env(20, 180, 1500), [lp(420)])], 761, comp=True, low=14000,
    note="close thunder: a crack, a tearing, a long rumble")
cand("bolt", "b_electric_snap", 900, [
    grains("crack", 3200, 0.5, 0, 0, 90, 6, 1.6, 20),
    grains("crackle", 4800, 0.8, -8, 40, 700, 40, 1.0, 640),
    noise("sizzle", "white", -14, 20, 860, env(5, 120, 700), [bp(6200, 0.6)]),
    tone("zap", "saw", 900, 140, -14, 0, 260, env(1, 30, 220), effects=[lp(5000)])], 762, low=15000,
    note="an electric snap and a falling zap, sizzle and crackle after; no thunder")
cand("bolt", "c_galvanic_slam", 1500, [
    grains("crack", 2600, 0.45, 0, 0, 110, 7, 2.0, 25),
    tone("body", "sine", 140, 36, -4, 0, 700, env(1, 80, 600)),
    noise("rumble", "brown", -10, 40, 1400, env(30, 150, 1200), [lp(380)]),
    grains("crackle", 4200, 0.8, -12, 120, 1200, 40, 1.1, 1100)], 763, low=14000,
    note="a crack with a body slam under it (a drop to 36 Hz), a rumble and crackle after")

# The arc lash (a ray from his neck, 0.3 s).
cand("lash", "a_beam_buzz", 560, [
    tone("beam", "saw", 98, 104, -6, 0, 520, env(3, 330, 180), harmonics=[
        {"ratio": 1.0, "gain_db": 0}, {"ratio": 2.0, "gain_db": -3}, {"ratio": 3.02, "gain_db": -7}],
        tremolo={"rate_hz": 31, "depth": 0.55}, noise_amount=0.25, effects=[bp(1400, 0.45)]),
    grains("snaps", 4400, 0.8, -9, 0, 540, 30, 1.0, 500),
    grains("crack", 2800, 0.5, -3, 0, 70, 4, 1.8, 14)], 771,
    note="a buzzing beam: a crack, then a held electric buzz with snaps")
cand("lash", "b_whip", 600, [
    sweep("whip", "white", 1400, 8800, 1.0, -3, 0, 140, env(2, 40, 90)),
    grains("crack", 3600, 0.5, 0, 90, 80, 4, 1.6, 12),
    noise("sizzle", "white", -12, 110, 490, env(5, 120, 360), [bp(6400, 0.7)]),
    grains("crackle", 4200, 0.8, -10, 120, 480, 22, 1.0, 440)], 772, low=15000,
    note="a whip of current: the air tears up, cracks, then sizzles")
cand("lash", "c_tearing", 600, [
    sweep("tear", "pink", 700, 6400, 0.7, -4, 0, 360, env(2, 160, 190)),
    tone("under", "square", 82, 78, -14, 0, 520, env(3, 300, 210), noise_amount=0.3,
         tremolo={"rate_hz": 23, "depth": 0.7}, effects=[lp(1800)]),
    grains("snaps", 4800, 0.8, -9, 40, 540, 26, 0.9, 500)], 773,
    note="the current tears down the line: a rising rip over a low buzz")

# The galvanize tell (1.3 s): he charges.
cand("charge", "a_wind_up", 1400, [
    tone("whine", "sine", 180, 1100, -12, 0, 1300, env(200, 950, 150), harmonics=[
        {"ratio": 1.0, "gain_db": 0}, {"ratio": 2.0, "gain_db": -9}]),
    grains("crackle_early", 3800, 0.8, -16, 0, 700, 10, 1.2, 680),
    grains("crackle_late", 4300, 0.8, -9, 650, 700, 40, 1.0, 680)], 781,
    note="a capacitor charging: a rising whine and crackle that thickens")
cand("charge", "b_coil_hum", 1400, [
    tone("hum", "saw", 60, 62, -12, 0, 1350, env(250, 900, 200), harmonics=[
        {"ratio": 1.0, "gain_db": 0}, {"ratio": 2.0, "gain_db": -4}, {"ratio": 3.0, "gain_db": -8}],
        tremolo={"rate_hz": 9, "depth": 0.35}, effects=[lp(1600)]),
    tone("whine", "sine", 400, 1400, -20, 400, 950, env(300, 550, 100)),
    grains("crackle", 4000, 0.8, -10, 300, 1100, 45, 1.1, 1050)], 782,
    note="the coils hum up, a whine climbs out of them, sparks thicken")

# The sputter when he is flat after the galvanize.
cand("sputter", "a_fizzle", 520, [
    grains("sparks", 3800, 0.8, -6, 0, 520, 7, 2.4, 480),
    tone("dying", "square", 90, 55, -16, 0, 300, env(2, 60, 230), noise_amount=0.4, effects=[lp(1500)])], 791,
    note="a few weak sparks and a dying buzz")
cand("sputter", "b_short_out", 420, [
    grains("burst", 4400, 0.8, -4, 0, 160, 12, 1.0, 140),
    grains("tail", 3000, 0.9, -12, 150, 270, 4, 2.8, 250)], 792,
    note="a short-out: a burst of sparks, then two late ones")

# THE RAKE (three blows at 0, 0.3, 0.6 s).
cand("rake", "a_claws", 360, [
    sweep("arm", "pink", 3000, 480, 0.7, -3, 0, 300, env(4, 110, 170)),
    grains("nails", 3400, 0.8, -7, 90, 90, 4, 3.5, 40)], 801,
    note="a heavy arm's swipe and four nails in the air")
cand("rake", "b_coat", 420, [
    sweep("coat", "pink", 1700, 260, 0.6, -2, 0, 380, env(10, 140, 220)),
    sweep("arm", "white", 3600, 900, 0.8, -9, 30, 230, env(4, 70, 150)),
    grains("nails", 3200, 0.8, -9, 120, 80, 4, 3.0, 30)], 802,
    note="the coat and the arm: a lower, longer swipe with the nails in it")
cand("rake", "c_rip", 380, [
    sweep("arm", "pink", 2800, 520, 0.7, -4, 0, 260, env(4, 90, 160)),
    mode("rip", "scrape", -6, 70, 260, env(3, 60, 190), effects=[bp(2300, 0.7)], color="white")], 803,
    note="a swipe that rips: a scrape like cloth tearing")

# THE HAMMER (both fists on the floor) and the shock it sends along it.
cand("hammer", "a_stone", 1200, [
    mode("thud", "thud", 0, 0, 500, env(0, 60, 430), color="brown"),
    tone("boom", "sine", 92, 34, -3, 0, 700, env(1, 70, 620)),
    grains("grit", 2400, 0.7, -12, 30, 700, 22, 3.2, 650, color="pink")], 811, room=0.55,
    note="fists on stone: a thud, a boom to 34 Hz, falling grit, a stone room")
cand("hammer", "b_iron_floor", 1400, [
    mode("thud", "thud", 0, 0, 500, env(0, 60, 430), color="brown"),
    mode("plate", "metal_gong", -11, 10, 1300, env(1, 100, 1150)),
    tone("boom", "sine", 88, 36, -5, 0, 650, env(1, 60, 580))], 812, room=0.5,
    note="fists on the castle's iron floor: a thud and the plates ringing")
cand("hammer", "c_body_slam", 1000, [
    mode("flesh", "flesh_strike", -2, 0, 420, env(0, 40, 370), weight="heavy"),
    mode("thud", "thud", -3, 0, 480, env(0, 50, 420), color="brown"),
    tone("boom", "sine", 80, 32, -4, 0, 600, env(1, 60, 530))], 813,
    note="his weight: a heavy body hit, a thud, a boom")
cand("shock", "a_rolling_rumble", 1800, [
    noise("rumble", "brown", 0, 0, 1800, env(40, 700, 1050), [lp(320)]),
    grains("debris", 2000, 0.7, -10, 50, 1600, 34, 3.5, 1500, color="pink")], 821,
    note="the shock rolls along the floor: a rumble and debris")
cand("shock", "b_electric_ground", 1800, [
    noise("rumble", "brown", -2, 0, 1800, env(40, 650, 1100), [lp(320)]),
    grains("crackle", 3600, 0.8, -8, 60, 1600, 46, 1.1, 1500),
    tone("buzz", "square", 70, 66, -18, 0, 1500, env(30, 900, 560), noise_amount=0.3, effects=[lp(1200)])], 822,
    note="the shock is current in the floor: a rumble that crackles and buzzes")

# THE LUNGE and its BITE (0.12 s in).
cand("lunge", "a_pounce", 520, [
    sweep("body", "pink", 2400, 380, 0.6, -2, 0, 480, env(6, 180, 290)),
    sweep("coat", "brown", 900, 200, 0.5, -8, 20, 470, env(10, 160, 290))], 831,
    note="a big body leaving the floor: a low whoosh and the coat")
cand("lunge", "b_pounce_snarl", 620, [
    sweep("body", "pink", 2400, 380, 0.6, -3, 0, 480, env(6, 180, 290)),
    throat("snarl", 620, [voice(0, 540, [[0, 160], [0.5, 210], [1, 180]], [[0, 0.5], [0.5, 0.8], [1, 0.6]],
                                [[0, 0], [0.2, 1], [0.8, 0.8], [1, 0]], 560, breath=0.5, doubling=0.7, jitter=0.09)],
           seed=832, room=0.2, gain=-4)], 832,
    note="the same whoosh with a snarl in it")
cand("bite", "a_snap", 300, [
    grains("teeth", 2100, 0.6, 0, 0, 40, 2, 1.4, 8),
    mode("jaw", "thud", -6, 0, 160, env(0, 20, 130), color="brown"),
    grains("click", 5200, 0.9, -9, 2, 30, 2, 0.8, 6)], 841, low=15000,
    note="jaws snap shut: teeth on teeth and the jaw's knock")
cand("bite", "b_chomp", 420, [
    grains("teeth", 2100, 0.6, 0, 0, 40, 2, 1.4, 8),
    mode("flesh", "flesh_light", -5, 10, 300, env(0, 40, 250)),
    mode("jaw", "thud", -7, 0, 160, env(0, 20, 130), color="brown")], 842,
    note="a snap with flesh in it")
cand("bite", "c_snap_snarl", 700, [
    grains("teeth", 2100, 0.6, 0, 0, 40, 2, 1.4, 8),
    mode("jaw", "thud", -6, 0, 160, env(0, 20, 130), color="brown"),
    throat("snarl", 700, [voice(60, 600, [[0, 190], [1, 150]], [[0, 0.5], [1, 0.3]],
                                [[0, 0], [0.1, 1], [0.7, 0.6], [1, 0]], 560, breath=0.55, doubling=0.7, jitter=0.1)],
           seed=843, room=0.2, gain=-6)], 843,
    note="a snap, then the snarl through his teeth")

# HIS STEPS and the LANDING after the bound.
cand("step", "a_boot", 260, [
    mode("heel", "thud", 0, 0, 220, env(0, 25, 190), color="brown"),
    grains("leather", 900, 0.8, -14, 10, 120, 3, 4.0, 60, color="pink")], 851, low=6000,
    note="a heavy boot: a low heel, a little leather")
cand("step", "b_boot_stone", 300, [
    mode("heel", "thud", 0, 0, 240, env(0, 25, 210), color="brown"),
    grains("grit", 2600, 0.8, -15, 15, 160, 6, 2.5, 90, color="pink"),
    tone("weight", "sine", 70, 45, -8, 0, 200, env(1, 20, 170))], 852, low=8000,
    note="boot on stone: the heel, his weight, a little grit")
cand("land", "a_giant_lands", 1100, [
    mode("thud", "thud", 0, 0, 600, env(0, 60, 530), color="brown"),
    mode("flesh", "flesh_deep", -6, 0, 400, env(0, 40, 350)),
    tone("boom", "sine", 85, 32, -3, 0, 700, env(1, 60, 630)),
    grains("debris", 2200, 0.7, -12, 30, 900, 24, 3.2, 850, color="pink")], 861, room=0.5,
    note="he lands from the wall: a thud, his weight, a boom, debris")


# ── Render, load, measure ────────────────────────────────────────────────────
def cue_id(sound, cid):
    return f"audition.wolfenstein.{sound}.{cid}"


def recipe(sound, cid, spec):
    return {"schema": "ambition.sfxir.v1", "id": cue_id(sound, cid), "duration_ms": spec["duration_ms"],
            "render": {"sample_rate": SR, "channels": 2, "seed": spec["seed"], "buffer_size": 128,
                       # Our own band limits; the default safety lowpass takes a crack's crack.
                       "safety_profile": "none"},
            "layers": spec["layers"], "postprocess": spec["postprocess"],
            "notes": f"Wolfenstein audition {sound}/{cid}: {spec['note']}"}


def render(path: Path):
    done = subprocess.run([PY, "-m", "ambition_sfx_renderer", "render", str(path), "--outdir", str(HERE / "out"),
                           "--format-policy", "wav", "--force"], cwd=RENDERER, capture_output=True, text=True,
                          # One thread per render, so JOBS is the whole of the load.
                          env={**os.environ, "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"})
    if done.returncode:
        raise SystemExit(f"{path.name}: {done.stderr[-2000:]}")


def load(cue_dir: Path) -> np.ndarray:
    wavs = sorted(cue_dir.glob("*.wav")) or sorted(cue_dir.glob("*.ogg"))
    if not wavs:
        raise SystemExit(f"no audio in {cue_dir}")
    a, sr = sf.read(wavs[0], always_2d=True)
    if sr != SR:
        raise SystemExit(f"{cue_dir}: {sr} Hz")
    if a.shape[1] == 1:
        a = np.repeat(a, 2, axis=1)
    return a.astype(np.float32)


BANDS = ((0, 200), (200, 800), (800, 3000), (3000, 8000), (8000, 24000))


def moving_mean(x: np.ndarray, win: int) -> np.ndarray:
    """The mean of each `win`-sample window (the full windows only)."""
    c = np.concatenate([[0.0], np.cumsum(x, dtype=np.float64)])
    return (c[win:] - c[:-win]) / win


def loudness(a: np.ndarray) -> float:
    """The loudest 400 ms RMS, in dB: what the ear compares two cues by."""
    mono = a.mean(axis=1)
    win = int(0.4 * SR)
    if len(mono) <= win:
        return float(20 * np.log10(np.sqrt((mono ** 2).mean()) + 1e-9))
    power = moving_mean(mono ** 2, win)
    return float(10 * np.log10(power.max() + 1e-12))


def measure(a: np.ndarray) -> dict:
    """Length, how long it is heard, onset and peak (10 ms windows), the loud
    400 ms, and the share of its energy in each band."""
    mono = a.mean(axis=1)
    win = SR // 100
    rms = np.sqrt(np.maximum(moving_mean(np.pad(mono ** 2, (win // 2, win - win // 2 - 1)), win), 0))
    loud = np.where(rms > rms.max() * 0.1)[0]
    spec = np.abs(np.fft.rfft(mono)) ** 2
    freq = np.fft.rfftfreq(len(mono), 1 / SR)
    total = spec.sum() or 1.0
    return dict(
        ms=round(1000 * len(mono) / SR),
        onset_ms=round(1000 * int(loud[0]) / SR) if len(loud) else 0,
        heard_ms=round(1000 * (int(loud[-1]) - int(loud[0])) / SR) if len(loud) else 0,
        peak_at_ms=round(1000 * int(rms.argmax()) / SR),
        loud_db=round(loudness(a), 1),
        bands=[round(100 * spec[(freq >= lo) & (freq < hi)].sum() / total) for lo, hi in BANDS],
        centroid_hz=round(float((spec * freq).sum() / total)),
    )


def table(rows: list[tuple[str, np.ndarray]]) -> list[str]:
    out = ["| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |",
           "|---|---:|---:|---:|---:|---:|---|---:|"]
    for name, a in rows:
        m = measure(a)
        out.append(f"| {name} | {m['ms']} | {m['onset_ms']} | {m['heard_ms']} | {m['peak_at_ms']} | {m['loud_db']} | "
                   f"{'/'.join(str(b) for b in m['bands'])} | {m['centroid_hz']} |")
    return out


def matched(a: np.ndarray, to: np.ndarray) -> np.ndarray:
    """`a` at the loudness of `to` (within 1 dB by construction)."""
    return a * float(10 ** ((loudness(to) - loudness(a)) / 20))


# ── Reels ────────────────────────────────────────────────────────────────────
def mix(events, length_s) -> np.ndarray:
    """events: (time s, audio, gain, pan -1..1)."""
    out = np.zeros((int(length_s * SR) + 1, 2), np.float32)
    for t, a, g, pan in events:
        i = int(t * SR)
        j = min(len(out), i + len(a))
        if j <= i:
            continue
        left, right = np.cos((pan + 1) * np.pi / 4) * np.sqrt(2), np.sin((pan + 1) * np.pi / 4) * np.sqrt(2)
        out[i:j, 0] += a[: j - i, 0] * g * left
        out[i:j, 1] += a[: j - i, 1] * g * right
    return out


def main(only_measure=False):
    HERE.mkdir(parents=True, exist_ok=True)
    current = {sound: load(SHIPPED / cue) for sound, cue in IN_USE.items()}
    index = ["# Wolfenstein fight SFX auditions", "",
             "Written by `scripts/wolfenstein_sfx_candidates.py` (2026-10-17). Audio in",
             "`untracked/sfx-candidates/wolfenstein/` (`solo/`, `reel/`). Each candidate is matched to the cue in use",
             "(loudest 400 ms RMS). A sound's solos share one headroom gain, so a peaky candidate set can sit",
             "a little under the `__current` solo; in a reel the cue in use and the candidates share one gain,",
             "so the reels are the fair comparison.", "",
             "## The cues in use (all borrowed)", ""]
    index += table([(f"{s}: `{IN_USE[s]}`", a) for s, a in current.items()])
    if only_measure:
        print("\n".join(index))
        return
    (HERE / "recipes").mkdir(exist_ok=True)
    paths = []
    for (sound, cid), spec in C.items():
        p = HERE / "recipes" / f"{cue_id(sound, cid)}.sfx.yaml"
        p.write_text(yaml.safe_dump(recipe(sound, cid, spec), sort_keys=False))
        paths.append(p)
    with ThreadPoolExecutor(JOBS) as pool:
        list(pool.map(render, paths))
    got = {key: load(HERE / "out" / cue_id(*key)) for key in C}
    # The cue in use that each sound replaces (the lash has none of its own:
    # the fight plays the bolt there).
    ref = {"charge": "tell_charge", "lash": "bolt"}
    for key, a in list(got.items()):
        got[key] = matched(a, current[ref.get(key[0], key[0])])
    (HERE / "solo").mkdir(exist_ok=True)
    for sound, a in current.items():
        sf.write(HERE / "solo" / f"{sound}__current.wav", a, SR)
    for sound in dict.fromkeys(s for s, _ in C):
        # One headroom gain for a sound's candidates (a crack is peaky for its
        # loudness), so they stay matched to each other; the cue in use is
        # written as shipped.
        keys = [k for k in got if k[0] == sound]
        room = min(1.0, 0.95 / max(float(np.abs(got[k]).max()) for k in keys))
        for k in keys:
            sf.write(HERE / "solo" / f"{sound}__{k[1]}.wav", got[k] * room, SR)

    index += ["", "## The candidates", ""]
    for sound in dict.fromkeys(s for s, _ in C):
        index.append(f"### {sound}")
        index.append("")
        for (s, cid), spec in C.items():
            if s == sound:
                index.append(f"- `solo/{sound}__{cid}.wav`: {spec['note']}")
        index.append("")
        rows = [("current", current[ref.get(sound, sound)])] + [(cid, got[(s, cid)]) for s, cid in C if s == sound]
        index += table(rows) + [""]

    (HERE / "reel").mkdir(exist_ok=True)
    reels: list[tuple[str, str, list[str]]] = []
    audio: dict[str, np.ndarray] = {}

    def reel(name, what, events, length):
        """Write a reel; `events` are (t, audio, gain, pan, label)."""
        audio[name] = mix([e[:4] for e in events], length)
        marks = [f"{e[0]:.2f}s {e[4]}" for e in events if e[4]]
        reels.append((name, what, marks))

    pick = lambda sound, cid: current[ref.get(sound, sound)] if cid == "current" else got[(sound, cid)]
    cids = lambda sound: ["current"] + [c for s, c in C if s == sound]

    # Howl: the tell (two huffs), the howl as the strike opens, the pack.
    for cid in cids("howl"):
        reel(f"howl__{cid}", "the howl tell (in use), the howl 0.9 s in",
             [(0.0, current["tell_huff"], 0.8, 0, "tell"), (0.9, pick("howl", cid), 1.0, 0, f"howl {cid}")], 4.2)
    # Tells: each tell, then the strike it leads to (the strike sounds in use).
    for cid in cids("tell_snarl"):
        sz = lambda t: current[t] if cid == "current" else got[(t, cid)]
        ev, t = [], 0.0
        for tell, dur, strike in (("tell_snarl", 0.7, "rake"), ("tell_grunt", 1.1, "hammer"),
                                  ("tell_huff", 0.9, "howl"), ("tell_rise", 0.85, "lunge")):
            ev += [(t, sz(tell), 1.0, 0, tell), (t + dur, current[strike], 0.8, 0, strike)]
            t += dur + (2.6 if strike == "howl" else 1.3)
        reel(f"tells__{cid}", "the four tells, each with the strike it leads to (strikes in use)", ev, t)
    # The galvanize: the charge, three strikes at once, then the sputter.
    for cid in cids("bolt"):
        bolt = pick("bolt", cid)
        reel(f"galvanize_bolt__{cid}", "the charge (in use) 1.3 s, three ceiling strikes, the sputter",
             [(0.0, current["tell_charge"], 0.7, 0, "charge"), (1.3, bolt, 0.9, -0.6, f"bolts {cid}"),
              (1.312, bolt, 0.9, 0.0, ""), (1.325, bolt, 0.9, 0.6, ""),
              (2.0, current["sputter"], 0.6, 0, "sputter (in use)")], 4.0)
    for cid in cids("charge"):
        ch = current["tell_charge"] if cid == "current" else got[("charge", cid)]
        reel(f"galvanize_charge__{cid}", "the charge 1.3 s, then the strike in use",
             [(0.0, ch, 1.0, 0, f"charge {cid}"), (1.3, current["bolt"], 0.9, 0, "bolt (in use)")], 3.5)
    for cid in cids("sputter"):
        sp = pick("sputter", cid)
        reel(f"sputter__{cid}", "flat after the galvanize: the bolts sputter every 0.45 s",
             [(i * 0.45, sp, 1.0, 0, f"sputter {cid}" if i == 0 else "") for i in range(5)], 3.0)
    # The lash: the crackle of the tell (0.85 s), then the lash.
    for cid in cids("lash"):
        reel(f"lash__{cid}", "the lash tell crackle (arc candidate a), then the lash 0.85 s in",
             [(0.0, got[("arc", "a_dry_snaps")], 0.8, 0, "tell crackle"), (0.45, got[("arc", "a_dry_snaps")], 0.8, 0, ""),
              (0.85, pick("lash", cid), 1.0, 0, f"lash {cid}")], 2.2)
    for cid in cids("arc"):
        arc = pick("arc", cid)
        reel(f"arc__{cid}", "the neck bolts' crackle three times, then a lash (candidate a)",
             [(0.0, arc, 1.0, 0, f"arc {cid}"), (0.6, arc, 1.0, 0.2, ""), (1.2, arc, 1.0, -0.2, ""),
              (1.8, got[("lash", "a_beam_buzz")], 0.9, 0, "lash a")], 3.0)
    # The rake: the snarl tell (in use), three blows at 0, 0.3, 0.6 s.
    for cid in cids("rake"):
        r = pick("rake", cid)
        reel(f"rake__{cid}", "the rake tell (in use) 0.7 s, then three blows 0.3 s apart",
             [(0.0, current["tell_snarl"], 0.7, 0, "tell"), (0.7, r, 1.0, 0.2, f"blow 1 {cid}"),
              (1.0, r, 1.0, -0.1, "blow 2"), (1.3, r, 1.0, 0.25, "blow 3")], 2.4)
    # The hammer: the grunt tell (in use) 1.1 s, the slam and the shock together.
    for cid in cids("hammer"):
        reel(f"hammer__{cid}", "the hammer tell (in use) 1.1 s, the slam with the shock in use",
             [(0.0, current["tell_grunt"], 0.7, 0, "tell"), (1.1, pick("hammer", cid), 1.0, 0, f"slam {cid}"),
              (1.1, current["shock"], 0.7, 0, "shock (in use)")], 3.4)
    for cid in cids("shock"):
        reel(f"shock__{cid}", "the slam in use, the shock rolling with it",
             [(0.0, current["hammer"], 0.8, 0, "slam (in use)"), (0.0, pick("shock", cid), 1.0, 0, f"shock {cid}")], 2.8)
    # The lunge and the bite 0.12 s in.
    for cid in cids("bite"):
        reel(f"bite__{cid}", "the lunge (candidate a), the bite 0.12 s in",
             [(0.0, got[("lunge", "a_pounce")], 0.9, 0, "lunge a"), (0.12, pick("bite", cid), 1.0, 0, f"bite {cid}")], 1.6)
    for cid in cids("lunge"):
        reel(f"lunge__{cid}", "the lunge, the bite in use 0.12 s in",
             [(0.0, pick("lunge", cid), 1.0, 0, f"lunge {cid}"), (0.12, current["bite"], 0.9, 0, "bite (in use)")], 1.6)
    # Steps: he walks (a step each 0.42 s), then he lands from the wall.
    for cid in cids("step"):
        st = pick("step", cid)
        reel(f"steps__{cid}", "six steps, a step each 0.42 s, then the landing in use",
             [(i * 0.42, st, 1.0, 0.15 if i % 2 else -0.15, f"step {cid}" if i == 0 else "") for i in range(6)]
             + [(3.0, current["land"], 0.9, 0, "land (in use)")], 4.4)
    for cid in cids("land"):
        reel(f"land__{cid}", "the bound: the lunge (candidate a) up the wall, the landing 1.0 s later",
             [(0.0, got[("lunge", "a_pounce")], 0.8, 0, "lunge a"), (1.0, pick("land", cid), 1.0, 0, f"land {cid}")], 2.6)

    # One gain per family (the reels `<family>__*`), so its candidates keep
    # their level match against each other.
    for family in dict.fromkeys(n.split("__")[0] for n in audio):
        names = [n for n in audio if n.split("__")[0] == family]
        gain = 0.7 / max(float(np.abs(audio[n]).max()) for n in names)
        for n in names:
            sf.write(HERE / "reel" / f"{n}.wav", audio[n] * gain, SR)

    index += ["## The reels", "", "Each reel is the candidate in the fight's own timing; `__current` is the cue in use.", ""]
    for name, what, marks in reels:
        index.append(f"- `reel/{name}.wav` ({what}): " + "; ".join(marks))
    (HERE / "INDEX.md").write_text("\n".join(index) + "\n")
    (Path(__file__).with_suffix(".md")).write_text("\n".join(index) + "\n")
    print(f"{len(C)} candidates, {len(reels)} reels -> {HERE}")


if __name__ == "__main__":
    main(only_measure=sys.argv[1:2] == ["measure"])
