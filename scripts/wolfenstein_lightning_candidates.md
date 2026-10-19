# Wolfenstein's lightning: the zap and the strike

Written by `scripts/wolfenstein_lightning_candidates.py` (2026-10-18). Audio in
`untracked/sfx-candidates/wolfenstein/`. Matched to the cue in use (loudest 400 ms RMS).

The ZAP is an arc that hits nothing; the STRIKE is a zap with a thunderclap, for effect and
for an impact.

## zap

- `solo/zap__a_crack_sizzle.wav`: a hard crack and a sizzling crackle that dies away
- `solo/zap__b_whip.wav`: a crack that whips down through the air, crackle after
- `solo/zap__c_coil_snap.wav`: a Tesla coil's snap: the crack, a short mains buzz, crackle

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 280 | 0 | 226 | 11 | -18.1 | 0/76/21/3/0 | 869 |
| a_crack_sizzle | 450 | 0 | 368 | 5 | -18.1 | 3/4/19/53/22 | 5452 |
| b_whip | 420 | 0 | 310 | 5 | -18.1 | 5/5/19/64/7 | 4275 |
| c_coil_snap | 520 | 0 | 391 | 5 | -18.1 | 14/14/24/47/2 | 3034 |

## strike

- `solo/strike__a_close_clap.wav`: close thunder: the crack, a tearing clap, a long rumble
- `solo/strike__b_zap_then_boom.wav`: the zap, then a boom: a sub drop under a clap, a rumble in a big room
- `solo/strike__c_split_sky.wav`: the sky splits: the crack and a tearing crackle, a clap, thunder that rolls twice

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 280 | 0 | 226 | 11 | -18.1 | 0/76/21/3/0 | 869 |
| a_close_clap | 2000 | 0 | 969 | 30 | -18.1 | 14/20/50/15/0 | 1662 |
| b_zap_then_boom | 2300 | 0 | 859 | 75 | -18.1 | 95/1/2/2/0 | 215 |
| c_split_sky | 2600 | 0 | 1335 | 36 | -18.1 | 11/20/51/18/0 | 1784 |

## The reels: his intro, 8.5 s

- `reel/intro__current.wav` (the cues in use: the arc and the burst): 0.90s flash: the coils fire, he is seen (strike); 1.30s crackle; 2.30s flash (zap); 2.55s flash (zap); 2.95s flash (strike); 3.20s flash (zap); 3.45s the charge; 4.10s jolt (strike); 4.60s jolt (strike); 5.80s the roar's bolt (strike); 5.80s the howl (in use)
- `reel/intro__a_crack_sizzle__a_close_clap.wav` (zap a_crack_sizzle, strike a_close_clap): 0.90s flash: the coils fire, he is seen (strike); 1.30s crackle; 2.30s flash (zap); 2.55s flash (zap); 2.95s flash (strike); 3.20s flash (zap); 3.45s the charge; 4.10s jolt (strike); 4.60s jolt (strike); 5.80s the roar's bolt (strike); 5.80s the howl (in use)
- `reel/intro__b_whip__b_zap_then_boom.wav` (zap b_whip, strike b_zap_then_boom): 0.90s flash: the coils fire, he is seen (strike); 1.30s crackle; 2.30s flash (zap); 2.55s flash (zap); 2.95s flash (strike); 3.20s flash (zap); 3.45s the charge; 4.10s jolt (strike); 4.60s jolt (strike); 5.80s the roar's bolt (strike); 5.80s the howl (in use)
- `reel/intro__c_coil_snap__c_split_sky.wav` (zap c_coil_snap, strike c_split_sky): 0.90s flash: the coils fire, he is seen (strike); 1.30s crackle; 2.30s flash (zap); 2.55s flash (zap); 2.95s flash (strike); 3.20s flash (zap); 3.45s the charge; 4.10s jolt (strike); 4.60s jolt (strike); 5.80s the roar's bolt (strike); 5.80s the howl (in use)
