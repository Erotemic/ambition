# Enemy patterns: notice, space, telegraph, attack

Jon, 2026-10-30:

> "I want to really focus on building characters that notice, space, telegraph, attack similar to hollow knight enemies. This likely requires custom posing and custom art / expression animations for some of them. And we could introduce more variants, for instance, maybe the risen conscript had a variant where they are missing legs and crawl on the ground with their arms. Maybe we have a few facists with swords or clubs. We could improve the shielded shock trooper quite a bit with improved art and posing."

Primeval trail first, Castle Godwin second.

## The cycle

Each enemy reads in this order:

1. **Idle or patrol.**
2. **Notice:** a held beat with a startle pose.
3. **Space:** it keeps its own range.
4. **Telegraph:** a held windup pose, distinct from idle and walk.
5. **Attack.**
6. **Recover:** a window in which it can be punished.

Two enemies differ in their pattern, not only in their numbers.

## What the engine gives a character

| Want | Where | Field |
|---|---|---|
| notice beat | `MeleeTactics::notice_s` (brute) | stands, facing its foe, for that long on first sight; again after losing it |
| spacing | `MeleeTactics::footsies` | holds `hold_px` out, sways, commits after `wait_s` |
| pack spacing | `MeleeTactics::waits_its_turn`, `lurk_px` | hangs back behind an ally; waits beside a foe above it |
| hit and run | `BrainProfile::retreat_after_swing_s`, `MeleeTactics::flinch_back_s` | backs off after a swing / when hit |
| pacing | `BrainProfile::attack_cooldown_s` | the gap between swings |
| telegraph length | the action set's `windup_s` | how long the windup row is held |
| charge | `Lunge(rush_px)` | stands through its windup, then runs `rush_px` over its strike, which is live the whole way; the rush counts as reach, so the charge starts from as far as it will run |
| pounce | `Lunge(rush_px, leap_speed)` | the charge leaves the ground: an arc under its gravity |
| attack by distance | a running attack (`attack_dash` in the character's moveset file) | a body in a run reads its running attack's reach and presses it (`BrainSnapshot::running`); standing, its standing attack |
| ranged tell | `BrainProfile::ranged_volley` (skirmisher) | `windup_s` of warning before it fires |
| dive tell | `MeleeTactics::dive_tell_s` (flyer) | lined up over its foe, it hangs that long before it dives |
| counter, fade | `MeleeTactics::counter`, `fades` | the shock warden's answer, the spectral wolf |

## What the art gives a character

Row names are the contract. A sheet without a row draws as before (`PinnedRow`).

- **`notice`:** pinned while the brute is noticing. A startle: head up and turned to the player, crest or frill raised.
- **`windup`:** pinned over a melee swing's startup, a skirmisher's volley windup, and a flyer's tell before its dive. An exaggerated, held anticipation pose: crouched, cocked, horns down. Every frame is the full pose, because the row loops on the sim clock and only a small shiver is allowed.
- **`attack_side`:** the strike. It is played by the move's progress over the whole move, so the frames after the windup land on the strike and the recovery. A row alias (`ROW_ALIASES`) can name another clip, e.g. the raptor's `attack_side` is its `pounce`.

The pin system is `body::pin_telegraph_rows` (`ambition_platformer2d_actor_monolith/src/body/telegraph_rows.rs`). It runs after the fade rows, and leaves another writer's pin alone.

## The primeval trail (done 2026-10-30)

| creature | pattern |
|---|---|
| compsognathus | a darting pack: notices, takes turns, bites, skips back out |
| raptor | notices; backs well off after each strike; comes in at a run and pounces from ~200 px (its running attack); caught up close, it bites |
| emu | kick and run |
| ostrich, ostrich hen | strut just out of reach, then the kick, then away |
| cassowary | waits, waits, kicks hard |
| shoebill | the long stare (holds far off), then the lunge |
| pachycephalosaurus | holds its distance; dome aimed and a foot scraping; a 220 px head-down rush; dazed after |
| triceratops | horns lowered and pawing; a 260 px gore charge; a second to turn its bulk round |
| dilophosaurus | coils with its jaws wide before each spit |
| parrot, owl, swallows, vulture | hang over the player (0.25-0.6 s) before each dive |
| kiwi, chickens, penguins | skittish, or taking turns |

Tests: `game/ambition_app/tests/an_enemy_notices_then_telegraphs.rs`.

## Castle Godwin (done 2026-10-30)

- **New characters:**
  - the crawling conscript (legless, drags itself on its arms; lurks, then a long-telegraphed lunge-grab);
  - the fascist swordsman (a quick sabre thrust from range);
  - the fascist clubman (club high for a long time, then a slam he is winded after).
- **Shock warden:** redrawn, with a planted guard.
- **Notice and windup rows:** the base conscript, the enforcer, the warden, the attack hound, the failed prototype and the searchlight crawler have them too.
- **The attack hound** bites standing; coming in at a run it lunges (a crouch, then a low 180 px spring; `movesets/evil_attack_hound.ron`).

Tests: `game/ambition_app/tests/castle_godwin_variants.rs`.

## Not yet

- **Flyers' art:** the parrot, owl, swallows and vulture hang before a dive, but their sheets have no `windup` row yet.
- **Barks** fire on provoke, not on notice. A creature has no voice cue for noticing yet.
- **Footsies and a running attack don't mix.** Holding in a band slows a body out of its run, so a pouncer should hit and run instead.
