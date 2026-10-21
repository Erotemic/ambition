# The Inugami's howl: auditions

Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in
`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS).

## The candidates

- `solo/howl__a_spirit.wav`: one wolf's howl, high and mostly breath, in a long cold room: the dog it was
- `solo/howl__b_chorus.wav`: three throats that are not quite one, a little apart in pitch and time: the spirits in it howl with it
- `solo/howl__c_giant.wav`: the giant head: a bigger throat (closer formants), not a lower buzz; a sub under the held note
- `solo/howl__d_drawn_in.wav`: a long breath drawn in, then the howl: the tell is in the sound
- `solo/howl__e_fire.wav`: the giant's howl in two near voices, and the rush of its red fire under it

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_spirit | 2800 | 40 | 2541 | 1222 | -19.4 | 1/97/2/0/0 | 599 |
| b_chorus | 3000 | 51 | 2632 | 1121 | -19.4 | 0/96/4/0/0 | 584 |
| c_giant | 3200 | 80 | 2753 | 1360 | -19.4 | 1/95/3/0/0 | 441 |
| d_drawn_in | 3200 | 236 | 2519 | 1122 | -19.4 | 0/95/4/0/0 | 616 |
| e_fire | 3200 | 85 | 2717 | 1710 | -19.4 | 1/95/3/0/0 | 478 |

## The reel

- `reel/howl__all.wav`: 0.0 s current; 1.9 s a_spirit; 5.5 s b_chorus; 9.3 s c_giant; 13.3 s d_drawn_in; 17.3 s e_fire

Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`.
