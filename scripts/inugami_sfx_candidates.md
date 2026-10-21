# The Inugami's howl: auditions

Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in
`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS).

## The candidates

- `solo/howl__a_wolf.wav`: a wolf's howl on the contour of Jon's spectrogram: 6.2 s, near-pure, the 2nd harmonic brightest
- `solo/howl__b_short.wav`: the same howl, its whole contour told in 3.6 s, for a fight
- `solo/howl__c_spirit.wav`: the short howl and its ghost: a second voice 0.38 s behind it and a little sharp, in a big cold room
- `solo/howl__d_pack.wav`: the head howls and its blue spirits howl with it: two more wolves come in, a minor third over and under
- `solo/howl__e_giant.wav`: the giant head: the howl a fourth lower, fuller (more harmonics), and an octave under it, soft

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_wolf | 6400 | 16 | 6066 | 3320 | -19.4 | 0/20/80/0/0 | 953 |
| b_short | 3800 | 13 | 3484 | 1640 | -19.4 | 0/21/79/0/0 | 956 |
| c_spirit | 4600 | 13 | 3997 | 1678 | -19.4 | 0/21/79/0/0 | 948 |
| d_pack | 5200 | 13 | 4318 | 1694 | -19.4 | 0/26/74/0/0 | 951 |
| e_giant | 4400 | 13 | 3874 | 1149 | -19.4 | 3/71/26/0/0 | 686 |

## The reel

- `reel/howl__all.wav`: 0.0 s current; 1.9 s a_wolf; 9.1 s b_short; 13.7 s c_spirit; 19.1 s d_pack; 25.1 s e_giant

Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`.
