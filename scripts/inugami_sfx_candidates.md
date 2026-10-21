# The Inugami's howl: auditions

Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in
`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS).

## The candidates

- `solo/howl__a_measured.wav`: the reference, by its numbers: the rough two-voice yelp, then a near-pure held note that falls and darkens
- `solo/howl__b_no_yelp.wav`: the same held howl with no yelp at its start
- `solo/howl__c_giant.wav`: a_measured a major third lower, in a bigger room: the giant head
- `solo/howl__z_siren.wav`: round 2's a_wolf, the pure tone, to compare

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_measured | 5000 | 329 | 3750 | 532 | -19.4 | 0/95/5/0/0 | 640 |
| b_no_yelp | 5000 | 795 | 3270 | 2400 | -19.4 | 0/99/1/0/0 | 631 |
| c_giant | 5000 | 330 | 3709 | 571 | -19.4 | 0/99/1/0/0 | 514 |
| z_siren | 6400 | 16 | 6066 | 3320 | -19.4 | 0/20/80/0/0 | 953 |

## The reel

- `reel/howl__all.wav`: 0.0 s current; 1.9 s a_measured; 7.7 s b_no_yelp; 13.5 s c_giant; 19.3 s z_siren; 26.5 s reference

Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`.
