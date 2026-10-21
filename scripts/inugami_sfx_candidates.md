# The Inugami's howl: auditions

Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in
`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS).

## The candidates

- `solo/howl__a_throat.wav`: a_wolf's howl in a throat: the vowel opens to the peak and closes as it falls, a little jitter, a little breath
- `solo/howl__b_breathy.wav`: more air in it: breath through the throat with every pulse, a less steady voice
- `solo/howl__c_rough.wav`: a rough voice: it starts with a growl under it, and wavers more as it goes
- `solo/howl__d_dark.wav`: a darker vowel: the mouth opens less, an 'oo' more than an 'ow'
- `solo/howl__e_big_throat.wav`: a bigger throat (formants lower), the howl a little lower: the giant head
- `solo/howl__z_siren.wav`: round 2's a_wolf, the pure tone, to compare

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_throat | 6500 | 9 | 6268 | 4116 | -19.4 | 0/33/67/0/0 | 877 |
| b_breathy | 6500 | 9 | 6264 | 1177 | -19.4 | 0/34/66/0/0 | 872 |
| c_rough | 6500 | 7 | 6276 | 3184 | -19.4 | 0/33/67/0/0 | 872 |
| d_dark | 6500 | 7 | 6271 | 752 | -19.4 | 0/67/33/0/0 | 687 |
| e_big_throat | 6500 | 8 | 6270 | 4085 | -19.4 | 0/51/49/0/0 | 703 |
| z_siren | 6400 | 16 | 6066 | 3320 | -19.4 | 0/20/80/0/0 | 953 |

## The reel

- `reel/howl__all.wav`: 0.0 s current; 1.9 s a_throat; 9.2 s b_breathy; 16.5 s c_rough; 23.8 s d_dark; 31.1 s e_big_throat; 38.4 s z_siren

Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`.
