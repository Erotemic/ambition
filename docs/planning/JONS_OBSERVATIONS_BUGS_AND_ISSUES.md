# Agents should only edit this file to mark something as potentially done. Jon
# will remove it if it is actually done, or mark it not actually solved if an
# attempt doesn't work. AGENT ENTRIES TO THIS DOCUMENT SHOULD BE SMALL. NO LONG
# PARAGRAPHS. ONE SENTENCE ABOUT WHAT IS DONE. BULLET LISTS IF THERE IS A SET
# OF TASKS NEEDED TO COMPLETE THE ITEM.
#
# Markers, one per agent entry, always first: ✔ done · ◐ partly done · ▢ not
# done · ⊙ needs Jon's decision. Reasoning and measurements belong in the run
# ledger and in commit messages, not here.


* The Sanic music is much louder than the rest of the tunes, and the SFX sanic uses are also way too loud. We need a way to programmatically inspect the relative levels of different SFX and music scores so we identify things that might blow somebodies ear out and then fix them. 
  * ✔ `scripts/audio_levels.py` reports all 782 sounds on one page; the loud SFX are fixed (the procedural path had no loudness target and sat +8.4 dB over the packed one).
  * ⊙ but your premise about the music was wrong — Sanic's score is 19th of 77 and only +2.2 LU over its cohort, so nothing was lowered there.

* Sanic hitting the spikes should not be an insta kill. It should hurt him and knock out his rings. Sanic should only die after getting hit with 0 rings (or crushed, but there is no way to do that in the demo yet). This needs to be a fairly faithful reimplementation of sonic physics and mechanics.
  * ✔ Fixed — the ring mechanic already existed and four places were treating a reset as damage.

* The goblins in the goblin encounter don't have sprites anymore. They have magenta boxes. I think "Goblin" was never a proper enemy multi-instance character.
  * ✔ The goblins themselves are fixed; three `large_brute` mobs still draw placeholders because no catalog row is right for them.
  * ⊙ Your character-template ruling makes those three a real character definition (or an explicit art borrow), not a costume — still your casting call.

* Sanic still says "fly" instead of "super transform" or "untransform" which is what that button maps to. 
  * ✔ Fixed — the button reads "Transform".

* Sanic should not pick up anything that turns him into super sanic in the level. 
  * ✔ It came out: `monitor_super` is deleted from the course, from the authoring script and from `monitors.rs`, and a test now counts the monitors so no second one gets authored quietly. The D key is the only way into the form.

* Super sanics spikes are clipped by the sprite renderer. This might need a
  structural fix. We should not be able to clip sprite artwork so easily.

  ▢ Confirmed and measured 2026-08-16, and you were right that it is structural: `super_sanic`'s spikes are clipped at the logical frame's top edge, and 23 of 133 sheets show the same signature (`robot`, `player_extended`, four robot variants, `puppy_slug`, `ninja_shadow_oni_leader`). Tracked as D129 in the queue.

* The current player V3 collision / hurt box  is larger than the player sprite. It needs to be slightly inset from the visible parts of the player. It should be under the main head, and well within the player arms. The player hitbox needs to be very forgiving to the player.

  ✔ Fixed and guarded (2026-08-16): v3's sheet now authors his box (57×91) instead of measuring the full 71×103 idle silhouette you reported at 1.28× wide; a new test compares the two rectangles directly. ⚠ the snake and Sanic still remain on the hand-tuned `collision_scale` road.

* A sword respects an authored hurtbox and a bolt never has — `step_projectiles` tests the coarse `CenteredAabb` while melee consults `DamageableVolumes`. [agent-found]

  ▢ Found 2026-08-08 and deliberately not fixed, because it changes how bolts connect and the feel call is yours.
  * ◐ Half of it LANDED since: projectiles now resolve victims through `StrikeVictim`, the same named role melee uses, so a body publishing an EMPTY volume list offers no target — a bolt no longer lands on something a sword passes through. Only the PRECISION half is left (the overlap test is still the coarse box), and that is the feel call. Tracked as decision 1.

* In smash if you throw out an attack you hurt yourself.
  * ✔ Fixed — the swing also broadcast a body-scanning volume that came back around to its owner; bodies are now resolved by identity and a test poisons the old route.

* In smash there does not seem to be any knockback.
  * ✔ Fixed — the engine had every piece and the stage reached none of them: every basic swing is prefab-derived and prefab swings author zero growth, so a hit at 150% launched exactly as far as one at 0%. Smash now declares the growth as a rule (a hit doubles its launch at 100%); Ambition stays flat.

* In mary-o secret blocks or invisible blocks (or question mark blocks which currently work correctly) need to change their tile sprite to spent blocks. A brick block with a quasar in 1-1 just keeps its brick texture. That needs to be fixed.
  * ✔ Fixed 2026-08-19 — the hidden/invisible half was already right; only the BRICK look kept its masonry (deliberately, so a spent brick wouldn't announce which one had been special). Your call overrides that: a paid-off brick now takes the paint off, guarded by the whole look × spent table.

* In mary-o she can only have 1 fireball out a time. We should allow her to have 2 out a time.
  * ✔ `MAX_LIVE_SPARKS` is 2 and a test pins the count rather than the constant.

* In mary-o 1-2 the flagpole doesn't have a flag. 
  * ✔ Fixed — the map authored the shaft but neither the banner nor the finial, so 1-2's pole was also missing its top knob, which you hadn't reported. A guard now fails any room that stands a shaft without both.

* In mary-o when you restart the level all item blocks and enemies and anything else that is part of the stage should reset. Currently some blocks from the last run remain spent
  * ⊙ Broken bricks, spent `?`-blocks and discovered hidden blocks all clear on a replay and the art re-derives from that state, so I cannot reproduce it — tell me the block and the room if you see it again.
  * ⊙ Same ask as decision 4 in `awaiting-maintainer-decision.md` (*which game, and roughly when*) — answering it in either place unblocks both.

* In mary-o we need an SFX for when you collect coins
  * ✔ Fixed for both paths — loose coins were emitting the cue and your audio fragment never authorized it, and coin blocks were playing the brick-smash thunk instead.
  * ⊙ One case left on the thunk deliberately: touching a powerup you already outrank credits coins, and I judged that shouldn't sound like collecting one. Say if you disagree, it's one line.

* In mary-o we need need a block that contains coins. (i.e. the multi-coin block where num-coins=1 is the default instance of that block). It just visually pops out a coin when you jump up into it. It's not a real coin entity, just a vfx and your coin count directly goes up by 1. When the counter goes to zero the brick becomes spent until reset.
  * ◐ The block, its parse (bare `Coins` = 1) and the per-block counter are in, with the reset re-arming a partly-spent block; the coin *pop* VFX is not — the block flinches and the cue plays, but nothing draws a coin arcing out.

* In mary-o small mary-o should not be able to headbutt bricks to break them. Only tall or fire should be able to break bricks with the headbutt.
  * ✔ Fixed — nothing in the break path asked what form she was in. Three existing brick tests would have gone vacuously green under the new gate and were re-armed.

* In mary-o when you die the level doesn't restart you just stay right where you were. When you die you should restart the level with 1 less life. For now let's allow lives to go negative and the user to play forever, so no game over screen yet.
  * ◐ All three ways to die (a hit, the timeout, a pit or spike) are covered end to end and every one puts her back at spawn — poisoning any of the three reproduces your sentence, so if you still see it, it is a fourth route.
  * ✔ Lives do count down — `spend_lives_on_death` has decremented `MaryOLevelState::lives` off `ActorDiedMessage` since `52e34be60`, on all three death roads; the ▢ here predated it.

* Spent blocks in 1-2 don't look spent. There are is also no tile texture in 1-2.
  * ✔ Spent blocks fixed — every block in the cavern was opted out of art updates, because an authored colour meant "no sprite yet" and nothing could ever repaint it.
  * ⊙ The missing tile texture is separate and is your call: the colour sweep is doing what you asked. Making it a tint instead lands near black, because those constants are fill values, so it needs your colours re-picked.

* JON (2026-08-08 STILL OPEN) The snake and AI slop are still way too big visually, and the sprite might not match the box for the snake.

  ◐ Their collision bodies are the right size in world units now (snake 1.00x Mary-O's width, slop 1.09x), and what is left is that the drawn quad is 2.46x the body inside it.
  * ✔ DECIDED 2026-08-17 (now D165): shared unit FIRST, quad-from-bbox revisited after. A character DECLARES its height in base-grid pixels (16 to a tile) and art scales to it. Every `collision_scale` will multiply one shared reference unit instead of its own sheet's frame size, so the numbers become comparable; `collision_scale` is NOT deleted in this slice.
  * Sizing the quad from the bbox without also cropping the drawn region was tried and reverted: it stretches the art badly.
  * It needs four coupled sites, not the three the design doc names — there are two render-size publishers, and fixing one leaves both of the characters you complained about untouched.

* Low priority: For the web build we can't use kaledioscope because lunex doesn't support wasm

* In 1-2 jumping into the invisible brick from below doesn't seem to trigger it.
  * ✔ Fixed — you were right and I was wrong; it genuinely never fired. A `BonkOnly` branch skipped the head-contact arm under a comment claiming it fell through, which `else if` does not do.
  * ✔ The invisible-payout half is fixed too — `a_discovered_hidden_block_reveals_itself` passes at HEAD; it had never run, because Mary-O's eight art assertions are behind `--features visible`.

* The pirates in the cover are horribly miss-sized. The heavies need to get a little smaller (this should probably be something done in data by the sprite renderer, not in code) and the other pirates need to probably scale up 2x, They are as tall as the player robot who is supposed to be chibi
  * ▢ Same root as the snake/AI-slop item above and unblocked by the same ruling (shared unit first): the drawn quad does not derive from the body, so per-character `collision_scale` tunes a BOX while you are looking at ART. These three reports are the shared unit's customers — if it does not settle them, the bbox route comes back with evidence.
  * ⚠ the numbers confirm they are not comparable: heavies 1.95, other pirates 1.60, `robot` 2.10 — the robot's is the LARGEST, yet he reads chibi, because each scale multiplies its OWN sheet's frame size rather than a shared unit.


* The pirates in the pirate sky no longer ride their sharks. 
  * ✔ Fixed — your 2026-07-06 editor session dropped the four mount refs in `sandbox.ldtk`; they are restored byte-identical from git and guarded. GNU-ton's boss mount turned out never to have been covered at all.

* In the sky enemy the instance of iron marry doesn't use her swordgun, she shoots fireballs, which is not something her character should be able to do. I suppose we do need a distinction between unique characters and re spawning archetype characters.
  * ✔ Answered by you 2026-08-10 — a character is a reusable authored template, and a body always receives that character's prepared kit. D73 is closed; the current model is `docs/systems/actors-brains-and-character-content.md`. The migration plan and its Iron Mary acceptance evidence are archived at `docs/archive/planning-superseded/2026-08-13/character-template-architecture-2026-08-10.md`.

* Changing rooms flashes magenta squares for a brief moment. We need to have cleaner transitions between rooms than that.
  * ⊙ Did NOT reproduce on the one route I could drive: pressing F through `pirate_cove -> central_hub_complex` and photographing frames 92-97 of the transition shows zero magenta (detector proved live by planting a magenta square, which it counts exactly). That route draws a cover, so which rooms did you see it between?

* After I fought in the pirate sky, and enemies died and dropped their swords I walked into the ninja dojo and there was a laser sword gun just existing there. I was able to fly to it and pick it up and use it. So props are not being despawned correctly - or made intangible and queued for removal - when you leave a room. I suppose there is an interesting question because it means we have to answer the question: in the ambition game, what should happen if you leave an item somewhere? Should it despawn? When? If you come back should it still be there? For the skyrim aspect of the game I think sometimes we do need items to remember where they were and if they were moved, but maybe we defer that and just have items be scoped to rooms. 
  * ⊙ This is decision 7 in `awaiting-maintainer-decision.md`, waiting on YOU, not on engineering: the lifetime bug is already fixed for coin/health/ability drops (they and their visuals share room scope), and what is left for a dropped WEAPON is a product rule — vanish on leaving, persist on returning, or something else.


* When I have the laser sword in ambition and I use it, I incorrectly still use my normal jab attack. Holding an item should reroute normal attack actions to the item action, which might be like throw for bombs or fire for the gun sword. Some items may do different things depending on if your attack is a directioned tilt or airial or neutral, but the default for the gun sword is they all route to the one action the item has: shoot (I guess direction does change which way it shoots).

* In the ambition game, when I move from one room to another that is separated in LDTK, the camera moved as if there is a pan that should be happening. The camera room transition pans are just wrong.


* The main character shield sprite has the bubble in the wrong place, just kinda to the upper left. 
  * ✔ Fixed — the block row drew the shield at a hardcoded (64,63) while your torso is at (112,129), so it landed 48px left and 66px high. It now centres on the torso anchor.
  * ⊙ The bubble surrounds your torso and your head pokes above it. That is a radius, not a bug — say if you want it to cover the whole robot.

* In smash, choosing robot v3, if you do your attack (among other issues with smash combat right now) the VFX happens in the top left corner, not in the authored area for the character.  
  * ◐ A slash whose owner could not be found was being drawn at the world origin; that now warns instead. The attack art is measured clean, so please reproduce it once and tell me if a warning appears.

* NOTE: developing out the smash combat system and the ambition combat system should be very similar and feed each other, because I want the combat in ambition to feel a little smash like. I want knockback to increase depending on how damaged you are. The ambition game will have health, and not percent, so there will be a limit, and maybe some enemy characters won't have this property, but the main character will. The knockback is what will make this game fun.

* We need an animation for a main character for when they are knocked down. We need an animation (or at least architecture slots) for a slow getup, a tech, and a getup attack. All smash characters will need this too.


* in the title menu FPS is 60 FPS, whereas ambition itself gets 140 FPS, and I don't know if the title 60FPS is intentional
  * ⊙ Not intentional — there is ONE global `bevy_framepace` limiter driven by the Video `frame_cap` setting and nothing anywhere paces the title differently, so the split is emergent; say whether you want the title capped on purpose.


* When you challenge PCA in the C4 symmetry room we should change the music to a smash track.
  * ▢ **RE-PRICED 2026-08-21: it is NOT "one value on the encounter", and the earlier note had the shape wrong.** Traced end to end:
    * ⭐ the moment is `assets/dialogue/sandbox/symmetry.yarn:56` — the *"Challenge it."* choice fires `<<challenge>>`.
    * ⭐ **the track exists**, so no regen is needed: `super_smash_siblings_theme`, `_grand_symphony` and `_character_select` are all in `music_registry.ron` already.
    * ⛔ **the room's own music must STAY.** `symmetry_room` authors `music_track: for_emmy_forever_ago` in `sandbox.ldtk`, which is right for the puzzle — the CHALLENGE is what should swap, not the room.
    * ⛔⛔ **and it must not go in `cmd_challenge`.** That command is the GENERIC dialogue-gated fight trigger whose own doc says *"Any content … arms a boss/duel by authoring this one command on a choice; no Rust per-NPC branch"* — putting a track there would play a smash theme for every challenge in the game.
    * ⇒ **the slice is a new authored command**, `<<music …>>`, sibling to the `play_sfx` already in `yarn_vocabulary.rs`, plus one authored line on that choice. ⚠ **the cost that was missed:** the music director is AUTHORITY-GOVERNED (`MusicAuthority::Governed { authorized }`), so a requested track that the active context does not authorize is refused — the symmetry room's context has to authorize the smash track as well as the command existing.


* I want to implement a camera mode for gravity where the camera just follows the player's reference frame. We should be careful so we use player-reference frame inputs in this mode. It doesn't need special gravity affordances. 
  * ✔ SHIPPED 2026-08-17. Gameplay → **Camera Frame**, *world-fixed* / *player-relative*. The world-observer camera stays the default and remains an option.
  * ⭐ the player-reference-frame input you asked to be careful about needs no separate setting: a player-relative view makes screen axes BE body axes, so all three input frame modes collapse onto body-relative — an identity, pinned by `a_player_relative_view_collapses_every_input_mode`. The movement/aim rows say they are inactive rather than being overwritten, so your choice survives switching back. Design: [`engine/camera-reference-frame-policy.md`](engine/camera-reference-frame-policy.md).


* Holding up for 2 seconds should be an alternative way of entering a door or interacting with an object.


* There isn't a quit to title option in the smash menu selection.
  * ⊙ `PauseEntry::QuitToTitle` exists and smash's `visible` feature does install the shell that shows it, so which screen did you mean — the pause menu, or the character SELECT screen (which is the demo's own UI and has no such row)?


* The smash UI for character select looks good, but the controls don't feel good, they are very hard to use with a gamepad. 
  * ⛔ WITHDRAWN — I suggested the pad was not its own source under the default policy. It is: smash claims `JoinToClaim` on its own routes, so that explanation does not apply and this stays a feel report.

* In smash it should be easy for 2 controllers to select their own characters, or turn other characters off or into cpus, any controller should be able to turn a slot into a player if there is a controller connected to it.
  * ▢ ⛔ I first wrote here that this was built and switched off. **That was wrong — I truncated my own grep with `| head`.** Smash DOES claim `InputAssignmentPolicy::JoinToClaim`, route-scoped, while the select or gameplay route is up (`demo_smash/src/lib.rs:1508`), and releases it on leaving. So keyboard + one pad should already be two players and the enum default never applies here.
  * ⊙ Which means the report is about something further in: does the second source get a SLOT and fail to claim a character, or never appear at all? Say which and it narrows to seats vs the select screen.
  * ◐ 2026-08-20: the SEATING half is measured good — `seats_offered_under` gives keyboard + N pads N+1 slots under the couch policy, nothing else in the tree writes `InputAssignmentPolicy`, and `the_screen_decides::two_players_take_controllers_pick_fighters_and_the_battle_starts` seats keyboard-as-device-0 beside pad 1 and starts the match; so if it still fails for you it is the claiming/feel half, not the seats.


*  Note, in ambition I can't use "F" to go through doors anymore, and in smash, I see the new emmy sprite on the select screen, but her character is the old sprite in the match.
  * ✔ The F half does NOT reproduce on the keyboard preset — pressed it against a live Door and the room changed: `capture_scene pirate_cove 0,0 out.png 1280x720 --warmup 90 --press f` logs `room-transition begin pirate_cove -> central_hub_complex` then `room-loaded`.
  * ⊙ So if it still fails for you it is the gamepad or a saved rebind, not the door — say which you were using.
  * ◐ **The emmy sprite half, measured 2026-08-21 — she is one of FOUR sheets whose reduced tier is not a reduction.** Her catalog row is `sheet: "sprites/noether_spritesheet.png"`, and the select screen and the match resolve the same id, so the difference has to be WHICH FILE each road reads. Compared all 198 sheets, canonical vs `sprites_0_5x`:

```text
median width ratio across 198 sheets      0.513   (tiers ARE normally half)
noether        4096x3875 -> 4095x4041     1.00    ⛔ not reduced at all
perfect_cellular_automaton                1.01    ⛔ same shape
carl_stargan   1650x16328 -> 1728x1349    1.05
pugnacious_polygon                        0.81
```

  ⇒ **the mechanism**: `noether` and `perfect_cellular_automaton` saturate the 4096 texture cap, so the "half" tier packs to the SAME cap and is a differently-packed sheet rather than a scaled one. A body drawn from that tier reads different frame rects than one drawn from the canonical — which is what "the new sprite on select, the old one in the match" looks like. ⚠ the causal link is a strong inference, not yet a capture; the falsifier is to force full quality and see her change in the match.
  ⛔ **and it is invisible to the incremental regen**: canonical mtime 2026-08-19 21:14 is OLDER than the tier's 2026-08-20 11:37, so an mtime check calls the tier fresh and skips it. That is also why the goal's art-tier check keeps going stale and why regenerating did not fix this.                           

* When I change the video quality in ambition, my sprite went from the robot v3 character to the robot v2 character. 
  * ▢ Three causes eliminated (missing art, a missing `_actor.ron` sidecar, a per-tier sheet collision — none of the colliding rig targets is a character id). Not eliminated: quality variants resolve through a separate FILE-ROOT index (`from_baked_table_by_file_root`) rather than the shared target road, and `3bf154974` (2026-08-08) made a quality change re-materialize on-screen bodies instead of only the next room — that re-materialization path is the likely site. Owner doc: `sprite-residency-and-live-quality.md`.
  * ⊙ Two things that would settle it: was your report BEFORE or AFTER 2026-08-08, and does it swap back if you change quality again?
  * ✖ **FOURTH CAUSE ELIMINATED 2026-08-21 — it is not tier REPACKING.** The Emmy finding two entries above gave an obvious candidate: four sheets saturate the 4096 texture cap, so their "reduced" tier is a differently-PACKED sheet rather than a scaled one, and different frame rects would read as a different character. Tested against the robots — all thirteen reduce cleanly:

```text
player_robot_v2   2815x2312 -> 1472x1215   0.52
player_robot_v3   3072x2484 -> 1600x1259   0.52
… 11 more robot sheets, every ratio 0.42-0.57, none capped
```

  ⇒ **v2 and v3 are separate, correctly-reduced sheets at every tier**, so a quality change cannot turn one into the other by geometry. That leaves RESOLUTION — which sheet gets chosen — and so it strengthens rather than replaces the standing hypothesis: `from_baked_table_by_file_root` resolving quality variants through a separate FILE-ROOT index instead of the shared target road.

## 2026-08-20 — doors do not work in Ambition, and Mary-O's 1-1 loops

Jon: *"In ambition I cannot go through any doors anymore. This is both interact
doors and contact doors. In maryo finishing 1-1 sends you back to 1-1. These are
huge regressions, not sure how we didn't have a test to catch these."*

  ✔ **Contact half DIAGNOSED, and the mechanism is fine.** She stalls at x=1841
  walking at the right edge exit — but there is no wall: the Collision IntGrid is
  empty through the whole zone except its BOTTOM ROW, where three cells form a
  16px floor lip. She collides with its side. Pulse a jump and she is in after
  211 frames and the room changes to `scroll_lab`, so contact transitions work.
  ⛔ **not a regression** — those three cells are solid in every commit back to
  2026-08-15. It is a standing conflict with `EdgeExit`'s contract ("walks off
  the screen into it"), and the fix is CONTENT: clear the three cells, or accept
  that these exits are hopped. Tracked as D174.

  ▢ **The interact half does not reproduce anywhere I can build it**, and the
  eliminations are worth more than the guess they replace. Ruled out by
  measurement, not inspection:

  | suspect | verdict |
  |---|---|
  | LDtk assets damaged by an editor session | ✖ EntityRefs intact (4/6/0/0, unchanged); the ONLY difference in all four dirty worlds is `nextUid`, same byte length |
  | the game loads different world files than the tests | ✖ `game/ambition_content/assets/worlds/*` are SYMLINKS into the submodule; `static_world_text!` compiles in the same bytes the tests read |
  | body contact (landed 2026-08-19) blocking her | ✖ `BodyContactSnapshot` is EMPTY at the stall |
  | the demo-binary-vs-shipped-host split | ✖ driven through the real launcher on a `SimulationHost::Rollback` host, finishing 1-1 lands in 1-2 with a body in it |
  | doors themselves | ✖ she WALKS across `central_hub_complex` into a door holding interact and the room changes |
  | the TOUCH OVERLAY, installed unconditionally by `add_presentation_plugins` whenever `mobile_touch` is compiled — which `desktop_dev` does, with no runtime gate on desktop | ✖ the door opens with `TouchControlsPlugin` in the app; it was the last STRUCTURAL difference between the compositions that work and the binary that does not |

  ⏸ **PARKED FOR AN INTERACTIVE SESSION — Jon, 2026-08-21:** *"if you can't
  find it right now, move on to bigger architecture tasks. we can debug in an
  interactive session."* ⛔ do NOT spend another autonomous session on the
  interact half; every composition reachable from this tree has been tried and
  the eliminations below are the record. Pick it up WITH Jon at a keyboard.

  ⇒ what is left is on Jon's machine and not in the tree: persisted state
  (`~/.local/share/ambition/`), the `desktop_dev` feature set, real devices, or
  playing to the pole rather than warping. **The 30-second probe that splits
  it:** `AMBITION_DATA_DIR=$(mktemp -d) ./run_game.sh` — `data_dir_root()`
  honours that variable, so it boots with a throwaway save and settings. If
  doors work there, it is persisted state; if not, the `[world-event]` lines at
  the moment of the press name the destination.

  ⭐ **and the coverage gap that let this happen is closed either way.** Every
  room-transition test put the body in the zone by ASSIGNMENT
  (`kin.pos = zone.aabb.center()`), so nothing between "a body is walking" and
  "a body is in that zone" was covered; and the level tests asked whether the
  room CHANGED, never WHICH, which `Replay` satisfies. Both now have guards
  (`walking_into_a_loading_zone`, `level_lap`, `mary_o_lap_in_the_host`).

