# The Inugami's howl: auditions

Written by `scripts/inugami_sfx_candidates.py` (2026-10-21). Audio in
`untracked/sfx-candidates/inugami/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS).

## The candidates

- `solo/howl__a_lone.wav`: the lone wolf, from its description alone: it should sound like round 4's a_measured
  `pitch_hz: 650, yelp_s: 0.5, rise_s: 0.12, hold_s: 1.75, sag_semitones: 0.8, fall_s: 1.5, fall_semitones: 3.6, fall_db: -25, droop_s: 0.3, droop_semitones: 1.6, bright_db: -9, dark_db: 15, jitter_cents: 15, shimmer: 0.12, breath: 0.05`
- `solo/howl__b_giant.wav`: the Inugami: the lone wolf a major third lower, slower, in a bigger room
  `pitch_hz: 520, yelp_s: 0.5, rise_s: 0.12, hold_s: 2.1, sag_semitones: 0.8, fall_s: 1.8, fall_semitones: 3.6, fall_db: -25, droop_s: 0.3, droop_semitones: 1.6, bright_db: -9, dark_db: 15, jitter_cents: 15, shimmer: 0.12, breath: 0.05`
- `solo/howl__c_spectral_wolf.wav`: a spectral wolf (a level enemy): higher, shorter, a quick yelp
  `pitch_hz: 760, yelp_s: 0.35, rise_s: 0.12, hold_s: 0.9, sag_semitones: 0.8, fall_s: 1.0, fall_semitones: 3.0, fall_db: -25, droop_s: 0.3, droop_semitones: 1.6, bright_db: -9, dark_db: 15, jitter_cents: 15, shimmer: 0.12, breath: 0.05`
- `solo/howl__d_far_call.wav`: a spectral wolf far off: no yelp, a long thin hold, more breath
  `pitch_hz: 700, yelp_s: 0, rise_s: 0.12, hold_s: 2.4, sag_semitones: 0.8, fall_s: 1.6, fall_semitones: 3.6, fall_db: -25, droop_s: 0.3, droop_semitones: 1.6, bright_db: -9, dark_db: 15, jitter_cents: 11, shimmer: 0.12, breath: 0.09`
- `solo/howl__e_hurt.wav`: a wolf hurt: a short yelp and a fast fall, no hold to speak of
  `pitch_hz: 820, yelp_s: 0.3, rise_s: 0.06, hold_s: 0.25, sag_semitones: 0.8, fall_s: 0.55, fall_semitones: 7, fall_db: -25, droop_s: 0.2, droop_semitones: 3, bright_db: -9, dark_db: 15, jitter_cents: 24, shimmer: 0.12, breath: 0.05`

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_lone | 4570 | 0 | 3850 | 223 | -19.4 | 0/95/5/0/0 | 640 |
| b_giant | 5220 | 0 | 4498 | 1356 | -19.4 | 0/99/1/0/0 | 513 |
| c_spectral_wolf | 3070 | 0 | 2362 | 120 | -19.4 | 0/94/6/0/0 | 752 |
| d_far_call | 4820 | 0 | 4138 | 2322 | -19.4 | 0/99/1/0/0 | 681 |
| e_hurt | 1760 | 0 | 1133 | 82 | -19.4 | 0/42/58/0/0 | 827 |

## The reel

- `reel/howl__all.wav`: 0.0 s current; 1.9 s a_lone; 7.3 s b_giant; 13.3 s c_spectral_wolf; 17.2 s d_far_call; 22.8 s e_hurt; 25.3 s reference

Wolfenstein's living howls (2026-10-17): `untracked/sfx-candidates/wolfenstein/solo/howl__*.wav`.
