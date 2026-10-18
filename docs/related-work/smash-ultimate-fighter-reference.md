# Smash Ultimate fighter reference: movement, evades, grabs

**Checked 2026-10-18.** This page records how *Super Smash Bros. Ultimate*
fighters move, so Ambition's fighters can be compared against a known-good feel
without rediscovering it. It only describes Ultimate. Ambition's own numbers
live in the files that author them (see [Where Ambition's side lives](#where-ambitions-side-lives)),
and nothing here copies them, so this page never goes stale against the code.

Jon, 2026-10-18: *"We need to get the character base stats and movement right.
It has to feel good when two humans are playing each other."* The knockback
side of the same reference (launch speed, tumble, Sakurai angle, KO percents) is
in [`../planning/demos/smash-parity-inventory.md`](../planning/demos/smash-parity-inventory.md)
and the survey notes behind it.

## Sources and attribution

| Source | What it gives | Kind | Licence |
|---|---|---|---|
| [Ultimate Frame Data: stats](https://ultimateframedata.com/stats.php) | per-fighter walk, initial dash, run, air speed and acceleration, gravity, fall speeds, weight, jump heights and durations, landing lag, dodges, rolls, grab range, out-of-shield grab | third-party; credits Zeckemyro for frame data | facts, attributed |
| [Ultimate Frame Data: Mario](https://ultimateframedata.com/mario) | per-move frame data, including the standing, dash and pivot grabs and the directional air dodges | third-party | facts, attributed |
| [SmashWiki: Roll](https://www.ssbwiki.com/Roll) | forward and back roll **distances** in Ultimate, measured in Training mode and rescaled for the stage grid's error | community wiki | CC BY-SA |
| [SmashWiki: Traction](https://www.ssbwiki.com/Traction) | per-fighter traction (ground deceleration) in Ultimate | community wiki | CC BY-SA |
| [SmashWiki: Jump](https://www.ssbwiki.com/Jump#Jump_squat) | every Ultimate fighter has a 3-frame jump squat (exceptions: Kazuya 7, Giga Bowser 15) | community wiki | CC BY-SA |
| [SmashWiki: Short hop](https://www.ssbwiki.com/Short_hop) | a short hop is a jump released during the jump squat; Ultimate adds the two-button short hop and the short-hop-attack 0.85x damage | community wiki | CC BY-SA |
| [SmashWiki: Dash](https://www.ssbwiki.com/Dash) | initial dash vs run; turnaround only in an initial dash's first frames; a turnaround dash waits 3 frames before accelerating | community wiki | CC BY-SA |
| [SmashWiki: Distance unit](https://www.ssbwiki.com/Distance_unit) and [Talk page](https://www.ssbwiki.com/Talk:Distance_unit) | 1 unit is about 0.1 m since Melee; Battlefield and the Ω forms are 160 units wide (Steve's blocks are 10 units) | community wiki | CC BY-SA |
| [SmashWiki: Final Destination (SSBU)](https://www.ssbwiki.com/Final_Destination_(SSBU)) | Ultimate's Final Destination is the same length as Battlefield | community wiki | CC BY-SA |
| [SmashWiki: Shield](https://www.ssbwiki.com/Shield), [Shield drop lag](https://www.ssbwiki.com/Shield_drop_lag) | Ultimate's perfect shield is on the shield's RELEASE, a 5-frame window | community wiki | CC BY-SA |

The per-fighter table is
[`data/smash_ultimate_fighter_stats.csv`](data/smash_ultimate_fighter_stats.csv):
90 fighters × 40 columns, in Ultimate's own units, spelled exactly as the
sources spell them (`??`, `**`, `20/25` and `(2) 6` included). It is written
by [`scripts/fetch_smash_ultimate_reference.py`](../../scripts/fetch_smash_ultimate_reference.py);
re-run it to refresh the table and diff the result. Its columns are named after the source columns in the
script's `UFD_TABLES`. Distances are units; times are frames at 60 Hz; speeds
are units per frame; gravity and acceleration are units per frame².

## Units, and putting Ultimate on our scale

Ultimate simulates at 60 frames a second. A fighter's numbers only mean
something against the stage, so convert through the stage: Battlefield and Final
Destination are **160 units** wide. If an Ambition stage of width `W` px
stands for one of them:

```text
k  = W / 160          px per unit       (k = 3 for a 480 px stage)
px          = units   · k
px/s        = u/frame · k · 60          (k = 3: × 180)
px/s²       = u/frame²· k · 60²         (k = 3: × 10 800)
seconds     = frames / 60
```

This is a stage-relative calibration. Body height gives a second check: at about
0.1 m a unit, a 1.6 m fighter is 16 units, or 48 px at `k = 3`. When the two
disagree, the stage is the one that matters, because it is what a player's
movement is measured against.

Time is authored in **seconds** in Ambition, never in frames, so a fighter
behaves the same at any simulation rate. Compare Ultimate's frame counts by
converting them, and read differences of one frame as quantisation.

## Roster spread

Across the whole roster in the table (n is the number of fighters with a
numeric value). The px columns are at `k = 3`.

| Stat | min | p10 | median | p90 | max | median at k = 3 |
|---|---|---|---|---|---|---|
| walk speed (u/f) | 0.62 | 0.82 | 1.11 | 1.38 | 1.575 | 200 px/s |
| initial dash (u/f) | 1.45 | 1.68 | 1.91 | 2.25 | 2.45 | 344 px/s |
| run speed (u/f) | 1.18 | 1.43 | 1.71 | 2.25 | 3.85 | 308 px/s |
| initial dash length (frames) | 7 | 10 | 10 | 15 | 16 | 0.17 s |
| air speed (u/f) | 0.735 | 0.88 | 1.05 | 1.26 | 1.344 | 189 px/s |
| air acceleration, max (u/f²) | 0.03 | 0.05 | 0.075 | 0.10 | 0.13 | 810 px/s² |
| gravity (u/f²) | 0.053 | 0.07 | 0.091 | 0.13 | 0.23 | 983 px/s² |
| fall speed (u/f) | 0.98 | 1.3 | 1.6 | 1.85 | 2.1 | 288 px/s |
| fast-fall speed (u/f) | 1.568 | 2.08 | 2.53 | 2.96 | 3.36 | 455 px/s |
| full hop (u) | 16.5 | 26.2 | 33 | 38.9 | 50.5 | 99 px |
| short hop (u) | 10 | 13 | 16.3 | 18.4 | 22.1 | 49 px |
| short-hop airtime (frames) | 24 | 30 | 37 | 42 | 50 | 0.62 s |
| full-hop airtime (frames) | 34 | 42 | 50 | 58 | 71 | 0.83 s |
| traction (u/f²) | 0.076 | 0.085 | 0.11 | 0.13 | 0.138 | 1188 px/s² |
| forward roll distance (u) | 26.5 | 28.8 | 31.8 | 35.5 | 39.7 | 95 px |
| back roll distance (u) | 26.4 | 28.9 | 31.5 | 35 | 36.6 | 94 px |
| forward roll total (frames) | 26 | 28 | 29 | 31 | 35 | 0.48 s |
| back roll total (frames) | 32 | 33 | 34 | 36 | 39 | 0.57 s |
| neutral air dodge total (frames) | 38 | 43 | 49 | 58 | 63 | 0.82 s |
| hard landing lag (frames) | 3 | 4 | 4 | 5 | 6 | 0.07 s |
| weight | 62 | 79 | 95 | 112 | 135 | — |

Ratios that hold across the roster, and are worth more than any one number:

- **Short hop ≈ 48 % of full hop** in height (range 34–63 %).
- **Initial dash ≈ 1.10 × run speed** (range 0.6–1.45). The two are independent:
  Ganondorf's initial dash is 1.40 × his run, Sonic's 0.60 ×.
- **Run ≈ 1.65 × air speed** (range 0.95–3.2). Fighters cover ground faster than
  air, and drift in the air at about half their run.
- **Fast fall = 1.6 × fall speed** for every fighter but Samus and Dark Samus (1.63).
- **A fighter reaches its fall speed fast.** Mario hits 1.5 u/f after 17 frames
  of falling; the cap, not gravity, sets most of a descent.
- **Everyone's jump squat is 3 frames** (0.05 s).

## Archetype anchors

Twelve fighters that span the cast's shapes. Ultimate units; airtime in frames.

| Fighter | Walk | Initial dash | Dash frames | Run | Air speed | Air accel | Gravity | Fall / fast fall | Full / short hop | FH / SH airtime | Weight | Traction |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Mario | 1.155 | 1.936 | 10 | 1.76 | 1.208 | 0.08 | 0.087 | 1.5 / 2.4 | 36.33 / 17.54 | 56 / 40 | 98 | 0.102 |
| Fox | 1.523 | 2.09 | 10 | 2.402 | 1.11 | 0.09 | 0.23 | 2.1 / 3.36 | 35 / 16.4 | 36 / 24 | 77 | 0.115 |
| Sheik | 1.47 | 2.178 | 7 | 2.42 | 1.155 | 0.09 | 0.15 | 1.75 / 2.8 | 39 / 18.75 | 47 / 32 | 78 | 0.106 |
| Captain Falcon | 0.987 | 1.98 | 15 | 2.552 | 1.218 | 0.075 | 0.12 | 1.865 / 2.984 | 37.31 / 17.98 | 48 / 34 | 104 | 0.105 |
| Sonic | 1.444 | 2.31 | 10 | 3.85 | 1.208 | 0.05 | 0.09 | 1.65 / 2.64 | 35 / 16.89 | 53 / 38 | 86 | 0.138 |
| Bowser | 0.901 | 2.255 | 13 | 1.971 | 1.155 | 0.05 | 0.125 | 1.77 / 2.832 | 33 / 15.7 | 45 / 31 | 135 | 0.104 |
| Ganondorf | 0.767 | 1.87 | 15 | 1.34 | 0.83 | 0.04 | 0.108 | 1.65 / 2.64 | 25.49 / 12.24 | 41 / 30 | 118 | 0.125 |
| King Dedede | 1.029 | 1.815 | 10 | 1.496 | 0.735 | 0.05 | 0.097 | 1.95 / 3.12 | 32.85 / 16.02 | 48 / 36 | 127 | 0.085 |
| Jigglypuff | 0.735 | 1.65 | 13 | 1.271 | 1.332 | 0.13 | 0.053 | 0.98 / 1.568 | 19.79 / 11.26 | 51 / 41 | 68 | 0.087 |
| Peach | 0.924 | 1.826 | 15 | 1.595 | 1.029 | 0.12 | 0.07 | 1.19 / 1.904 | 30.03 / 14.5 | 57 / 41 | 89 | 0.111 |
| Marth | 1.575 | 2.255 | 10 | 1.964 | 1.071 | 0.08 | 0.075 | 1.58 / 2.528 | 33.66 / 16.26 | 55 / 41 | 90 | 0.114 |
| Little Mac | 1.386 | 2.365 | 15 | 2.464 | 1.208 | 0.04 | 0.09 | 1.95 / 3.12 | 26 / 12.53 | 44 / 33 | 87 | 0.116 |

What the anchors show:

- **Heavy is not slow.** Bowser runs 12 % faster than Mario and his initial dash
  is faster still. His weight is in his size, his commitment and his weight
  stat, not in his legs.
- **Same height, different jump.** Fox and Mario jump within 4 % of each other's
  height, but Fox has 2.6 × the gravity and lands in 36 frames to Mario's 56. A
  jump is fixed by its **height and airtime together**. Matching one leaves the
  other free.
- **Floaty is low gravity and a low fall cap,** with high air acceleration
  (Jigglypuff, Peach): they steer well and come down slowly.
- **The slow archetype is slow everywhere** (Ganondorf): short jump, low air
  speed, low air acceleration, long dash, but a fast *initial* dash so his first
  step still threatens.

## Evades and grabs

Frames at 60 Hz. "Intangible" is the protected window; total is the first
actionable frame.

| Fighter | Fwd roll: distance, intangible, total | Back roll: distance, intangible, total | Spot dodge total (cancel / full), intangible | Neutral air dodge total, intangible | Grab range (u) | Hard landing |
|---|---|---|---|---|---|---|
| Mario | 32.17 u, 4–15, 29 | 30.63 u, 5–16, 34 | 20/25, 3–17 | 52, 3–29 | 12.0 | 4 |
| Fox | 31.8 u, 4–12, 26 | 31.75 u, 4–14, 32 | 18/23, 3–14 | 38, 2–26 | 11.0 | 4 |
| Sheik | 31.85 u, 4–12, 26 | 31.8 u, 4–14, 32 | 18/23, 3–14 | 44, 2–26 | 12.5 | 4 |
| Bowser | 27.28 u, 4–16, 32 | 27.88 u, 5–17, 37 | 23/28, 3–18 | 46, 4–32 | 20.0 | 6 |
| Ganondorf | 32.69 u, 4–16, 32 | 32.92 u, 5–17, 37 | 23/28, 3–18 | 46, 4–32 | 14.0 | 5 |
| Jigglypuff | 35.02 u, 4–15, 30 | 34.97 u, 5–16, 35 | 21/26, 3–17 | 63, 4–25 | 12.0 | 4 |

A roll **travels about 32 units (a fifth of the stage)** and is
protected for about 11 frames of its ~29. It moves a long way and is punishable
at its destination, which is what makes it a choice.
Back rolls are about 5 frames slower than forward rolls in Ultimate.
Repeated dodges stale: each one is slower and less protected.

Mario's directional air dodges (frame 3–21 intangible for every direction):

| Direction | Total | Landing lag |
|---|---|---|
| neutral | 52 (intangible 3–29) | 10 |
| down | 71 | 11–19 |
| diagonal down | 77 | 11–19 |
| left / right | 87 | 11–19 |
| diagonal up | 102 | 11–19 |
| up | 116 | 11–19 |

Mario's grabs (startup, active, total, end lag):

| Grab | Startup | Active | Total | End lag |
|---|---|---|---|---|
| standing | 6 | 6–7 | 34 | 27 |
| dash | 9 | 9–10 | 42 | 32 |
| pivot | 10 | 10–11 | 36 | 25 |

A grab out of shield comes out on frame 6 for most of the cast (frame 10 after
shield stun). Grab range is about 12 units for most fighters; the long ones
(Samus 42, Bowser 20, DK 19.5) are tethers or giants.

## Known limits of this data

- **A ballistic model reproduces short hops, not full hops.** From the table's
  height, gravity and fall speed alone, a plain model (rise under gravity, fall
  capped) predicts every anchor's short-hop airtime within 3 %. It
  overpredicts every full hop's by 8–12 %. Ultimate's full-hop mechanics
  have something the table does not carry. **Fit a full hop to its height and
  its airtime**, not to the gravity column.
- The roll distances are SmashWiki's Training-mode readings, not parameter
  dumps; treat them as ± a unit.
- Ultimate's jump heights are listed to the body's origin. The page does not
  say how airtime was timed (from the press or from leaving the ground); a
  3-frame difference is within that ambiguity.
- Some fighters have source gaps (`??`, `**`, blank), mostly DLC fighters and the
  Ice Climbers' partner.

## Where Ambition's side lives

This page names the authorities and copies none of their values:

- The engine's movement parameters and their defaults:
  [`MovementTuning`](../../crates/ambition_platformer2d_core/src/movement/tuning.rs).
- What a fighter may author about its own body: the `smash_fighter` facet's
  `body:` block (`FighterBodyAuthoring`,
  [`smash_fighter/mod.rs`](../../crates/ambition_characters/src/smash_fighter/mod.rs)),
  and a catalog row's `axis_tuning`.
- What every Smash match imposes on every body (jump squat, initial dash,
  dodges, shield):
  [`match_rules.ron`](../../game/ambition_demo_smash/assets/data/match_rules.ron).
- The stage the conversion factor `k` is read from:
  [`smash_stages.ron`](../../game/ambition_demo_smash/assets/data/smash_stages.ron).

Comparisons are made by tools that read both sides at run time, never by
copying a number into prose.

## What it changes for Ambition

- **For Smash**, the targets are ranges, not copies. A fighter should land
  inside the roster spread above on every axis and have a deliberate *shape*
  (which anchor it resembles, and where it departs). Numbers that matter more
  than any single value are the ratios: short to full hop, initial dash to run,
  run to air, and how fast a fall reaches its cap.
- **For Ambition's own game**, the same reference may inform how a character
  like Alice, Bob or the PCA feels when it is played. It does not bind it: a
  number whose meaning differs per game is authored per game, and a platformer
  that is not a fighting game may want different jumps. When a character plays
  in both, decide each game's numbers on purpose and record which reference, if
  any, each one follows.
