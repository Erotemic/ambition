"""Measure a howl the way the Inugami's reference was measured (2026-10-21):
its f0 contour, the level of h2..h4 under f0, the noise under the voice, the
pitch jitter (cents RMS, by band) and the shimmer, so a candidate can be
compared with the reference by numbers as well as by ear.

    ~/.cache/ambition-tool-venvs/ambition_sfx_renderer/bin/python scripts/howl_measure.py A.wav [B.wav ...]
"""

from __future__ import annotations

import sys

import numpy as np
import soundfile as sf
from scipy import signal


def measure(path: str, hold=(0.9, 3.9)) -> None:
    a, sr = sf.read(path, always_2d=True)
    m = a.mean(1)
    print(f"== {path.split('/')[-1]}")
    n = 4096
    f = np.fft.rfftfreq(n, 1 / sr)
    rows = []
    for t in np.arange(0.4, 4.5, 0.3):
        i = int(t * sr)
        if i + n > len(m):
            break
        spec = np.abs(np.fft.rfft(m[i:i + n] * np.hanning(n)))
        band = (f > 350) & (f < 800)
        f0 = f[band][np.argmax(spec[band])]
        level = [20 * np.log10(spec[(f > k * f0 - 20) & (f < k * f0 + 20)].max() + 1e-12) for k in range(1, 5)]
        mask = (f > 300) & (f < 1500)
        for k in range(1, 4):
            mask &= ~((f > k * f0 - 60) & (f < k * f0 + 60))
        noise = 20 * np.log10(np.sqrt(np.mean(spec[mask] ** 2)) + 1e-12)
        rows.append(f"{t:.1f}s {f0:4.0f}Hz h2..h4 " + " ".join(f"{v - level[0]:+3.0f}" for v in level[1:]) + f" noise {noise - level[0]:+3.0f}")
    print("\n".join(rows))
    b, a2 = signal.butter(4, [420 / (sr / 2), 760 / (sr / 2)], "band")
    z = signal.hilbert(signal.filtfilt(b, a2, m))
    inst = np.diff(np.unwrap(np.angle(z))) * sr / (2 * np.pi)
    env = np.abs(z)[:-1]
    k = np.ones(int(0.005 * sr)) / int(0.005 * sr)
    f0 = np.convolve(inst, k, "same")[int(hold[0] * sr):int(hold[1] * sr)]
    amp = np.convolve(env, k, "same")[int(hold[0] * sr):int(hold[1] * sr)]
    d = int(0.3 * sr) | 1
    cents = 1200 * np.log2(f0 / signal.savgol_filter(f0, d, 2))
    shim = amp / signal.savgol_filter(amp, d, 2) - 1
    ff, p = signal.welch(cents[::48], sr / 48, nperseg=min(2048, len(cents[::48])))
    share = {f"{lo}-{hi}": p[(ff >= lo) & (ff < hi)].sum() / p[(ff >= 0.5) & (ff < 120)].sum() for lo, hi in ((0.5, 8), (8, 15), (15, 120))}
    print(f"held: jitter {cents.std():.1f} cents RMS (" + ", ".join(f"{b} Hz {v:.2f}" for b, v in share.items())
          + f"), shimmer {shim.std():.3f}")


if __name__ == "__main__":
    for path in sys.argv[1:]:
        measure(path)
