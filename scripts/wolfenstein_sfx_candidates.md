# Wolfenstein fight SFX auditions

Written by `scripts/wolfenstein_sfx_candidates.py` (2026-10-17). Audio in
`untracked/sfx-candidates/wolfenstein/` (`solo/`, `reel/`). Each candidate is matched to the cue in use
(loudest 400 ms RMS). A sound's solos share one headroom gain, so a peaky candidate set can sit
a little under the `__current` solo; in a reel the cue in use and the candidates share one gain,
so the reels are the fair comparison.

## The cues in use (all borrowed)

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| rake: `boss.bear_mauler.swipe` | 330 | 0 | 236 | 104 | -25.7 | 63/25/10/2/0 | 372 |
| bite: `boss.trex.chomp` | 400 | 0 | 145 | 5 | -26.8 | 77/19/3/1/0 | 209 |
| lunge: `enemy.robot.guardian_lunge` | 460 | 15 | 359 | 324 | -25.4 | 86/12/1/0/0 | 135 |
| hammer: `boss.gnu_ton.stomp` | 560 | 0 | 245 | 5 | -24.8 | 95/5/0/0/0 | 121 |
| shock: `vfx.explosion.shockwave` | 560 | 0 | 215 | 38 | -23.2 | 83/14/2/0/0 | 171 |
| howl: `boss.generic.intro_roar` | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| arc: `vfx.generic_world.electric_arc` | 252 | 0 | 199 | 11 | -20.3 | 0/80/17/3/0 | 829 |
| bolt: `vfx.generic_world.electric_burst` | 280 | 0 | 226 | 11 | -18.1 | 0/76/21/3/0 | 869 |
| sputter: `enemy.robot.death_spark` | 820 | 2 | 587 | 436 | -27.8 | 62/19/14/5/0 | 519 |
| step: `player.land` | 110 | 0 | 47 | 6 | -25.6 | 61/34/4/0/0 | 239 |
| land: `boss.trex.stomp` | 640 | 0 | 488 | 5 | -23.0 | 95/5/0/0/0 | 89 |
| tell_snarl: `boss.trex.growl_snarl_a` | 780 | 4 | 522 | 126 | -16.5 | 37/54/9/0/0 | 403 |
| tell_grunt: `boss.trex.growl_grunt_a` | 420 | 0 | 282 | 76 | -18.6 | 51/46/3/0/0 | 263 |
| tell_huff: `boss.trex.growl_huff_a` | 470 | 1 | 246 | 62 | -20.4 | 17/70/12/0/0 | 482 |
| tell_rise: `boss.trex.growl_rise_a` | 980 | 24 | 749 | 505 | -15.6 | 52/45/3/0/0 | 254 |
| tell_lash: `ability.lightning.arc_loop` | 900 | 0 | 673 | 54 | -19.4 | 1/4/93/1/0 | 1116 |
| tell_charge: `ability.lightning.charge` | 650 | 0 | 475 | 82 | -20.2 | 1/91/7/1/0 | 540 |

## The candidates

### howl

- `solo/howl__a_lone_wolf.wav`: a classic wolf: one voice, rising to a held high note, a wide room
- `solo/howl__b_giant.wav`: a giant's wolf: lower, a growl at the start and the end, a sub under it
- `solo/howl__c_stitched.wav`: two throats at once: the wolf's howl and a man's moan an octave and a fifth under it
- `solo/howl__d_galvanic.wav`: the giant's howl with the bolts in his neck crackling under it

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 1100 | 6 | 682 | 193 | -19.4 | 91/8/1/0/0 | 118 |
| a_lone_wolf | 2400 | 80 | 2052 | 300 | -19.4 | 0/98/2/0/0 | 516 |
| b_giant | 2600 | 56 | 2280 | 2035 | -19.4 | 6/86/8/0/0 | 506 |
| c_stitched | 2600 | 75 | 2170 | 1968 | -19.4 | 4/91/5/0/0 | 490 |
| d_galvanic | 2600 | 46 | 2240 | 673 | -19.4 | 1/91/7/1/0 | 638 |

### tell_snarl

- `solo/tell_snarl__a_wolf.wav`: rake tell: a bared-teeth snarl
- `solo/tell_snarl__b_dire.wav`: rake tell: a bared-teeth snarl
- `solo/tell_snarl__c_stitched.wav`: rake tell: a bared-teeth snarl

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 780 | 4 | 522 | 126 | -16.5 | 37/54/9/0/0 | 403 |
| a_wolf | 750 | 13 | 686 | 580 | -16.5 | 37/61/2/0/0 | 363 |
| b_dire | 750 | 22 | 668 | 508 | -16.5 | 38/57/5/0/0 | 375 |
| c_stitched | 750 | 20 | 646 | 491 | -16.5 | 37/61/2/0/0 | 352 |

### tell_grunt

- `solo/tell_grunt__a_wolf.wav`: hammer tell: a strained grunt that builds to the slam
- `solo/tell_grunt__b_dire.wav`: hammer tell: a strained grunt that builds to the slam
- `solo/tell_grunt__c_stitched.wav`: hammer tell: a strained grunt that builds to the slam

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 420 | 0 | 282 | 76 | -18.6 | 51/46/3/0/0 | 263 |
| a_wolf | 1150 | 65 | 964 | 780 | -18.6 | 36/63/1/0/0 | 311 |
| b_dire | 1150 | 55 | 965 | 866 | -18.6 | 44/53/2/0/0 | 291 |
| c_stitched | 1150 | 46 | 975 | 708 | -18.6 | 46/51/3/0/0 | 301 |

### tell_huff

- `solo/tell_huff__a_wolf.wav`: howl tell: two deep breaths in through the nose
- `solo/tell_huff__b_dire.wav`: howl tell: two deep breaths in through the nose
- `solo/tell_huff__c_stitched.wav`: howl tell: two deep breaths in through the nose

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 470 | 1 | 246 | 62 | -20.4 | 17/70/12/0/0 | 482 |
| a_wolf | 900 | 32 | 275 | 80 | -20.4 | 8/85/7/0/0 | 522 |
| b_dire | 900 | 3 | 322 | 209 | -20.4 | 11/70/19/0/0 | 584 |
| c_stitched | 900 | 6 | 335 | 121 | -20.4 | 12/74/13/0/0 | 551 |

### tell_rise

- `solo/tell_rise__a_wolf.wav`: lunge tell: a growl rising into the leap
- `solo/tell_rise__b_dire.wav`: lunge tell: a growl rising into the leap
- `solo/tell_rise__c_stitched.wav`: lunge tell: a growl rising into the leap

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 980 | 24 | 749 | 505 | -15.6 | 52/45/3/0/0 | 254 |
| a_wolf | 900 | 69 | 745 | 717 | -15.6 | 18/78/4/0/0 | 461 |
| b_dire | 900 | 46 | 773 | 392 | -15.6 | 30/65/5/0/0 | 422 |
| c_stitched | 900 | 59 | 760 | 557 | -15.6 | 22/74/3/0/0 | 424 |

### arc

- `solo/arc__a_dry_snaps.wav`: dry snapping sparks, no hum
- `solo/arc__b_sizzle_buzz.wav`: an electric buzz with snaps and a sizzle on top
- `solo/arc__c_tesla.wav`: a Tesla coil: a fizzing corona tone and discharge snaps

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 252 | 0 | 199 | 11 | -20.3 | 0/80/17/3/0 | 829 |
| a_dry_snaps | 520 | 0 | 452 | 206 | -20.3 | 0/0/34/65/1 | 3835 |
| b_sizzle_buzz | 560 | 0 | 505 | 263 | -20.3 | 6/7/13/61/13 | 4801 |
| c_tesla | 600 | 0 | 541 | 6 | -20.3 | 0/16/44/40/0 | 2698 |

### bolt

- `solo/bolt__a_thunderclap.wav`: close thunder: a crack, a tearing, a long rumble
- `solo/bolt__b_electric_snap.wav`: an electric snap and a falling zap, sizzle and crackle after; no thunder
- `solo/bolt__c_galvanic_slam.wav`: a crack with a body slam under it (a drop to 36 Hz), a rumble and crackle after

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 280 | 0 | 226 | 11 | -18.1 | 0/76/21/3/0 | 869 |
| a_thunderclap | 1800 | 0 | 421 | 12 | -18.1 | 28/23/32/16/0 | 1536 |
| b_electric_snap | 900 | 0 | 740 | 5 | -18.1 | 3/8/22/51/16 | 4710 |
| c_galvanic_slam | 1500 | 0 | 883 | 96 | -18.1 | 95/1/1/3/0 | 243 |

### lash

- `solo/lash__a_beam_buzz.wav`: a buzzing beam: a crack, then a held electric buzz with snaps
- `solo/lash__b_whip.wav`: a whip of current: the air tears up, cracks, then sizzles
- `solo/lash__c_tearing.wav`: the current tears down the line: a rising rip over a low buzz

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 280 | 0 | 226 | 11 | -18.1 | 0/76/21/3/0 | 869 |
| a_beam_buzz | 560 | 0 | 491 | 5 | -18.1 | 39/49/5/6/0 | 583 |
| b_whip | 600 | 7 | 521 | 89 | -18.1 | 3/2/19/59/17 | 5104 |
| c_tearing | 600 | 0 | 482 | 164 | -18.1 | 45/9/25/20/1 | 1561 |

### charge

- `solo/charge__a_wind_up.wav`: a capacitor charging: a rising whine and crackle that thickens
- `solo/charge__b_coil_hum.wav`: the coils hum up, a whine climbs out of them, sparks thicken

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 650 | 0 | 475 | 82 | -20.2 | 1/91/7/1/0 | 540 |
| a_wind_up | 1400 | 0 | 1276 | 285 | -20.2 | 0/79/19/2/0 | 652 |
| b_coil_hum | 1400 | 47 | 1280 | 693 | -20.2 | 51/24/20/5/0 | 650 |

### sputter

- `solo/sputter__a_fizzle.wav`: a few weak sparks and a dying buzz
- `solo/sputter__b_short_out.wav`: a short-out: a burst of sparks, then two late ones

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 820 | 2 | 587 | 436 | -27.8 | 62/19/14/5/0 | 519 |
| a_fizzle | 520 | 0 | 441 | 6 | -27.8 | 43/5/19/33/0 | 1965 |
| b_short_out | 420 | 0 | 354 | 55 | -27.8 | 0/0/33/65/1 | 3996 |

### rake

- `solo/rake__a_claws.wav`: a heavy arm's swipe and four nails in the air
- `solo/rake__b_coat.wav`: the coat and the arm: a lower, longer swipe with the nails in it
- `solo/rake__c_rip.wav`: a swipe that rips: a scrape like cloth tearing

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 330 | 0 | 236 | 104 | -25.7 | 63/25/10/2/0 | 372 |
| a_claws | 360 | 11 | 240 | 134 | -25.7 | 0/15/65/19/0 | 2011 |
| b_coat | 420 | 13 | 306 | 136 | -25.7 | 4/35/54/8/0 | 1306 |
| c_rip | 380 | 9 | 216 | 115 | -25.7 | 0/16/68/16/0 | 1957 |

### hammer

- `solo/hammer__a_stone.wav`: fists on stone: a thud, a boom to 34 Hz, falling grit, a stone room
- `solo/hammer__b_iron_floor.wav`: fists on the castle's iron floor: a thud and the plates ringing
- `solo/hammer__c_body_slam.wav`: his weight: a heavy body hit, a thud, a boom

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 560 | 0 | 245 | 5 | -24.8 | 95/5/0/0/0 | 121 |
| a_stone | 1200 | 0 | 622 | 87 | -24.8 | 99/0/0/0/0 | 77 |
| b_iron_floor | 1400 | 0 | 744 | 91 | -24.8 | 92/8/0/0/0 | 105 |
| c_body_slam | 1000 | 0 | 495 | 12 | -24.8 | 95/4/1/0/0 | 93 |

### shock

- `solo/shock__a_rolling_rumble.wav`: the shock rolls along the floor: a rumble and debris
- `solo/shock__b_electric_ground.wav`: the shock is current in the floor: a rumble that crackles and buzzes

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 560 | 0 | 215 | 38 | -23.2 | 83/14/2/0/0 | 171 |
| a_rolling_rumble | 1800 | 6 | 1563 | 484 | -23.2 | 60/13/26/1/0 | 560 |
| b_electric_ground | 1800 | 6 | 1401 | 709 | -23.2 | 68/9/10/14/0 | 880 |

### lunge

- `solo/lunge__a_pounce.wav`: a big body leaving the floor: a low whoosh and the coat
- `solo/lunge__b_pounce_snarl.wav`: the same whoosh with a snarl in it

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 460 | 15 | 359 | 324 | -25.4 | 86/12/1/0/0 | 135 |
| a_pounce | 520 | 18 | 384 | 218 | -25.4 | 4/30/55/11/0 | 1534 |
| b_pounce_snarl | 620 | 16 | 506 | 218 | -25.4 | 14/53/27/5/0 | 926 |

### bite

- `solo/bite__a_snap.wav`: jaws snap shut: teeth on teeth and the jaw's knock
- `solo/bite__b_chomp.wav`: a snap with flesh in it
- `solo/bite__c_snap_snarl.wav`: a snap, then the snarl through his teeth

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 400 | 0 | 145 | 5 | -26.8 | 77/19/3/1/0 | 209 |
| a_snap | 300 | 0 | 86 | 5 | -26.8 | 36/23/29/12/0 | 1199 |
| b_chomp | 420 | 0 | 270 | 35 | -26.8 | 35/42/21/2/0 | 602 |
| c_snap_snarl | 700 | 0 | 591 | 5 | -26.8 | 32/59/8/1/0 | 470 |

### step

- `solo/step__a_boot.wav`: a heavy boot: a low heel, a little leather
- `solo/step__b_boot_stone.wav`: boot on stone: the heel, his weight, a little grit

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 110 | 0 | 47 | 6 | -25.6 | 61/34/4/0/0 | 239 |
| a_boot | 260 | 0 | 145 | 5 | -25.6 | 66/22/11/1/0 | 372 |
| b_boot_stone | 300 | 0 | 171 | 5 | -25.6 | 95/3/1/0/0 | 111 |

### land

- `solo/land__a_giant_lands.wav`: he lands from the wall: a thud, his weight, a boom, debris

| cue | ms | onset | heard | peak@ | loud dB | bands % <200/<800/<3k/<8k/8k+ | centroid Hz |
|---|---:|---:|---:|---:|---:|---|---:|
| current | 640 | 0 | 488 | 5 | -23.0 | 95/5/0/0/0 | 89 |
| a_giant_lands | 1100 | 0 | 609 | 24 | -23.0 | 97/2/1/0/0 | 88 |

## The reels

Each reel is the candidate in the fight's own timing; `__current` is the cue in use.

- `reel/howl__current.wav` (the howl tell (in use), the howl 0.9 s in): 0.00s tell; 0.90s howl current
- `reel/howl__a_lone_wolf.wav` (the howl tell (in use), the howl 0.9 s in): 0.00s tell; 0.90s howl a_lone_wolf
- `reel/howl__b_giant.wav` (the howl tell (in use), the howl 0.9 s in): 0.00s tell; 0.90s howl b_giant
- `reel/howl__c_stitched.wav` (the howl tell (in use), the howl 0.9 s in): 0.00s tell; 0.90s howl c_stitched
- `reel/howl__d_galvanic.wav` (the howl tell (in use), the howl 0.9 s in): 0.00s tell; 0.90s howl d_galvanic
- `reel/tells__current.wav` (the four tells, each with the strike it leads to (strikes in use)): 0.00s tell_snarl; 0.70s rake; 2.00s tell_grunt; 3.10s hammer; 4.40s tell_huff; 5.30s howl; 7.90s tell_rise; 8.75s lunge
- `reel/tells__a_wolf.wav` (the four tells, each with the strike it leads to (strikes in use)): 0.00s tell_snarl; 0.70s rake; 2.00s tell_grunt; 3.10s hammer; 4.40s tell_huff; 5.30s howl; 7.90s tell_rise; 8.75s lunge
- `reel/tells__b_dire.wav` (the four tells, each with the strike it leads to (strikes in use)): 0.00s tell_snarl; 0.70s rake; 2.00s tell_grunt; 3.10s hammer; 4.40s tell_huff; 5.30s howl; 7.90s tell_rise; 8.75s lunge
- `reel/tells__c_stitched.wav` (the four tells, each with the strike it leads to (strikes in use)): 0.00s tell_snarl; 0.70s rake; 2.00s tell_grunt; 3.10s hammer; 4.40s tell_huff; 5.30s howl; 7.90s tell_rise; 8.75s lunge
- `reel/galvanize_bolt__current.wav` (the charge (in use) 1.3 s, three ceiling strikes, the sputter): 0.00s charge; 1.30s bolts current; 2.00s sputter (in use)
- `reel/galvanize_bolt__a_thunderclap.wav` (the charge (in use) 1.3 s, three ceiling strikes, the sputter): 0.00s charge; 1.30s bolts a_thunderclap; 2.00s sputter (in use)
- `reel/galvanize_bolt__b_electric_snap.wav` (the charge (in use) 1.3 s, three ceiling strikes, the sputter): 0.00s charge; 1.30s bolts b_electric_snap; 2.00s sputter (in use)
- `reel/galvanize_bolt__c_galvanic_slam.wav` (the charge (in use) 1.3 s, three ceiling strikes, the sputter): 0.00s charge; 1.30s bolts c_galvanic_slam; 2.00s sputter (in use)
- `reel/galvanize_charge__current.wav` (the charge 1.3 s, then the strike in use): 0.00s charge current; 1.30s bolt (in use)
- `reel/galvanize_charge__a_wind_up.wav` (the charge 1.3 s, then the strike in use): 0.00s charge a_wind_up; 1.30s bolt (in use)
- `reel/galvanize_charge__b_coil_hum.wav` (the charge 1.3 s, then the strike in use): 0.00s charge b_coil_hum; 1.30s bolt (in use)
- `reel/sputter__current.wav` (flat after the galvanize: the bolts sputter every 0.45 s): 0.00s sputter current
- `reel/sputter__a_fizzle.wav` (flat after the galvanize: the bolts sputter every 0.45 s): 0.00s sputter a_fizzle
- `reel/sputter__b_short_out.wav` (flat after the galvanize: the bolts sputter every 0.45 s): 0.00s sputter b_short_out
- `reel/lash__current.wav` (the lash tell crackle (arc candidate a), then the lash 0.85 s in): 0.00s tell crackle; 0.85s lash current
- `reel/lash__a_beam_buzz.wav` (the lash tell crackle (arc candidate a), then the lash 0.85 s in): 0.00s tell crackle; 0.85s lash a_beam_buzz
- `reel/lash__b_whip.wav` (the lash tell crackle (arc candidate a), then the lash 0.85 s in): 0.00s tell crackle; 0.85s lash b_whip
- `reel/lash__c_tearing.wav` (the lash tell crackle (arc candidate a), then the lash 0.85 s in): 0.00s tell crackle; 0.85s lash c_tearing
- `reel/arc__current.wav` (the neck bolts' crackle three times, then a lash (candidate a)): 0.00s arc current; 1.80s lash a
- `reel/arc__a_dry_snaps.wav` (the neck bolts' crackle three times, then a lash (candidate a)): 0.00s arc a_dry_snaps; 1.80s lash a
- `reel/arc__b_sizzle_buzz.wav` (the neck bolts' crackle three times, then a lash (candidate a)): 0.00s arc b_sizzle_buzz; 1.80s lash a
- `reel/arc__c_tesla.wav` (the neck bolts' crackle three times, then a lash (candidate a)): 0.00s arc c_tesla; 1.80s lash a
- `reel/rake__current.wav` (the rake tell (in use) 0.7 s, then three blows 0.3 s apart): 0.00s tell; 0.70s blow 1 current; 1.00s blow 2; 1.30s blow 3
- `reel/rake__a_claws.wav` (the rake tell (in use) 0.7 s, then three blows 0.3 s apart): 0.00s tell; 0.70s blow 1 a_claws; 1.00s blow 2; 1.30s blow 3
- `reel/rake__b_coat.wav` (the rake tell (in use) 0.7 s, then three blows 0.3 s apart): 0.00s tell; 0.70s blow 1 b_coat; 1.00s blow 2; 1.30s blow 3
- `reel/rake__c_rip.wav` (the rake tell (in use) 0.7 s, then three blows 0.3 s apart): 0.00s tell; 0.70s blow 1 c_rip; 1.00s blow 2; 1.30s blow 3
- `reel/hammer__current.wav` (the hammer tell (in use) 1.1 s, the slam with the shock in use): 0.00s tell; 1.10s slam current; 1.10s shock (in use)
- `reel/hammer__a_stone.wav` (the hammer tell (in use) 1.1 s, the slam with the shock in use): 0.00s tell; 1.10s slam a_stone; 1.10s shock (in use)
- `reel/hammer__b_iron_floor.wav` (the hammer tell (in use) 1.1 s, the slam with the shock in use): 0.00s tell; 1.10s slam b_iron_floor; 1.10s shock (in use)
- `reel/hammer__c_body_slam.wav` (the hammer tell (in use) 1.1 s, the slam with the shock in use): 0.00s tell; 1.10s slam c_body_slam; 1.10s shock (in use)
- `reel/shock__current.wav` (the slam in use, the shock rolling with it): 0.00s slam (in use); 0.00s shock current
- `reel/shock__a_rolling_rumble.wav` (the slam in use, the shock rolling with it): 0.00s slam (in use); 0.00s shock a_rolling_rumble
- `reel/shock__b_electric_ground.wav` (the slam in use, the shock rolling with it): 0.00s slam (in use); 0.00s shock b_electric_ground
- `reel/bite__current.wav` (the lunge (candidate a), the bite 0.12 s in): 0.00s lunge a; 0.12s bite current
- `reel/bite__a_snap.wav` (the lunge (candidate a), the bite 0.12 s in): 0.00s lunge a; 0.12s bite a_snap
- `reel/bite__b_chomp.wav` (the lunge (candidate a), the bite 0.12 s in): 0.00s lunge a; 0.12s bite b_chomp
- `reel/bite__c_snap_snarl.wav` (the lunge (candidate a), the bite 0.12 s in): 0.00s lunge a; 0.12s bite c_snap_snarl
- `reel/lunge__current.wav` (the lunge, the bite in use 0.12 s in): 0.00s lunge current; 0.12s bite (in use)
- `reel/lunge__a_pounce.wav` (the lunge, the bite in use 0.12 s in): 0.00s lunge a_pounce; 0.12s bite (in use)
- `reel/lunge__b_pounce_snarl.wav` (the lunge, the bite in use 0.12 s in): 0.00s lunge b_pounce_snarl; 0.12s bite (in use)
- `reel/steps__current.wav` (six steps, a step each 0.42 s, then the landing in use): 0.00s step current; 3.00s land (in use)
- `reel/steps__a_boot.wav` (six steps, a step each 0.42 s, then the landing in use): 0.00s step a_boot; 3.00s land (in use)
- `reel/steps__b_boot_stone.wav` (six steps, a step each 0.42 s, then the landing in use): 0.00s step b_boot_stone; 3.00s land (in use)
- `reel/land__current.wav` (the bound: the lunge (candidate a) up the wall, the landing 1.0 s later): 0.00s lunge a; 1.00s land current
- `reel/land__a_giant_lands.wav` (the bound: the lunge (candidate a) up the wall, the landing 1.0 s later): 0.00s lunge a; 1.00s land a_giant_lands
