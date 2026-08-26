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

  ✔ Found 2026-08-08, deferred for the feel call, and CLOSED 2026-08-22 once you ruled for it — see below.
  * ◐ Half of it LANDED since: projectiles now resolve victims through `StrikeVictim`, the same named role melee uses, so a body publishing an EMPTY volume list offers no target — a bolt no longer lands on something a sword passes through. ~~Only the PRECISION half is left~~ — ✔ **CLOSED 2026-08-22**: you ruled for it (decision 1) and `step_projectiles` now asks the same `strike_reaches_victim` rule melee does. ⛔ this is a real feel change on shipped content and it is the intended one: a shot that used to land on a body whose authored volume is tighter than its box will now miss it, and one that grazes an edge will now connect.

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
  * ⭐⭐ **MEASURED 2026-08-22: the shared unit is BUILT, and it CANNOT fix the 2.46× — the two halves are independent, not sequential.** `catalog_join` resolves a height into `scale = height / body_h`, then `render = frame × scale` and `collision = body × scale`. So `render / collision = frame / body` **with the scale cancelling**: the quad/body ratio is a property of the ART'S PADDING and no choice of height moves it. ⇒ D165's *"shared unit FIRST, quad-from-bbox after"* is not a dependency — the second half was never downstream of the first, and it can start whenever.
  * ⚠ **and the UNITS are not the same statement.** D165 says a character declares its height in *"base-grid pixels (16 to a tile)"*; the implemented field is `standing_height` in WORLD px, and Mary-O's tile is `T = 32.0` world px. Those differ by 2× in her game. Before wiring anything to D165's wording, say which unit is meant — the authoring grid is not the world unit.
  * ✔ **the quad-from-bbox half is BUILT TOO, and this row's own note about it is stale (re-read 2026-08-22, by a different route: the code, not the design doc).** `sprite_body_collision_for_character_id_from_data` has two branches. With a `standing_height`: `scale = height / body_h` where `body_h` is the sheet's MEASURED body, then `render = frame x scale` — so the quad derives from the body and **cannot stretch**, because both axes take the same scale and the frame's aspect is kept. Without one: `ldtk_collision.max x collision_scale / frame_h`, which is the level-editor rectangle you are actually complaining about. ⇒ the stretching note describes an older approach that is no longer what this code does.
  * ⭐⭐ **RUN AGAINST REAL DATA, and the mechanism honors your input exactly: the three heavies derive 58.70 / 56.20 / 60.40, to the millimetre.** The six `Standard` pirates derive **48.00 — the robot's default, exactly** — which is your report, measured. There is now a guard (`an_authored_standing_height_is_the_height_the_body_derives`) pinning the equality for every row that authors a height; it names no character and no number, so retuning any of them is an ordinary content edit. Poison-checked: dividing the scale by the frame instead of the body turns 58.7 into 40.15 and the guard names the character.
  * ⚠ **but the audit surfaces a SECOND size fact you have not been told, and it is the one your eye is probably catching.** At the same 48.00 body, the DRAWN quad ranges from 1.00x to 1.57x across ordinary humanoid NPCs — `npc_ninja_shadow_duelist` 1.00, `npc_pirate_admiral` / `lookout` / `navigator` / `quartermaster` / `raider` 1.02, `npc_pirate_cutlass_viper` **1.44**, `npc_lab_raider` **1.57**. The engine considers all of them exactly the same height; on screen `lab_raider` is half again taller than the admiral. That ratio is `frame / body` — pure ART PADDING — and no height you author moves it, because the scale cancels. ⇒ **this half really is "done in data by the sprite renderer, not in code", exactly as you guessed**: it is a re-crop of the sheets whose frames carry 40-60% empty space. (`player_robot_v3` 2.81 and `npc_puppy_slug` 3.07 are not comparable — the robot lineage is `SpriteAuthored` so this number never ships for it, and a sprawled quadruped legitimately draws past its body box.)
  * ⇒ ⭐⭐ **so all three of your size reports reduce to ONE blocked question, and it is the six numbers.** **3 of 145** rows author a height at HEAD — the three heavies you ruled on (`broadside_bess` 58.7, `iron_mary` 56.2, `salt_annet` 60.4) — so 142 characters still take the legacy branch. The data the good branch needs is already there: **203 of 206 spritesheets publish `body_metrics`**, all six Standard pirates included. Nothing is left to build; the six numbers are the whole remaining input.

* Low priority: For the web build we can't use kaledioscope because lunex doesn't support wasm

* In 1-2 jumping into the invisible brick from below doesn't seem to trigger it.
  * ✔ Fixed — you were right and I was wrong; it genuinely never fired. A `BonkOnly` branch skipped the head-contact arm under a comment claiming it fell through, which `else if` does not do.
  * ✔ The invisible-payout half is fixed too — `a_discovered_hidden_block_reveals_itself` passes at HEAD; it had never run, because Mary-O's eight art assertions are behind `--features visible`.

* The pirates in the cover are horribly miss-sized. The heavies need to get a little smaller (this should probably be something done in data by the sprite renderer, not in code) and the other pirates need to probably scale up 2x, They are as tall as the player robot who is supposed to be chibi
  * ▢ Same root as the snake/AI-slop item above and unblocked by the same ruling (shared unit first): the drawn quad does not derive from the body, so per-character `collision_scale` tunes a BOX while you are looking at ART. These three reports are the shared unit's customers — if it does not settle them, the bbox route comes back with evidence.
  * ⚠ the numbers confirm they are not comparable: heavies 1.95, other pirates 1.60, `robot` 2.10 — the robot's is the LARGEST, yet he reads chibi, because each scale multiplies its OWN sheet's frame size rather than a shared unit.
  * ⭐ **MEASURED 2026-08-22, and the shared unit ALREADY EXISTS AND WORKS — the gap is that nobody authors it.** `catalog_join` resolves `standing_height` (world px, feet to top of visible body) into `scale = height / body_h`, so a character measures exactly that tall with its art's aspect kept. **Zero of 145 catalog rows author it**, so every `Standard` character falls to `body_kind`'s default — **48.0, which is the player robot's own height**. That is why an adult pirate is exactly as tall as your chibi robot: the engine is making them equal on purpose, and nothing has ever said they should differ.
  * ⭐ **and it explains the paradox in your numbers.** `robot`'s 2.10 is INERT — the robot lineage is one of only two things in the repo on `BodySource::SpriteAuthored`, so it publishes `ActorRenderSize` and `collision_scale` is never consulted for it. The pirates' 1.95/1.60 are live. You were comparing a number that does nothing with two that do.
  * ⭐ **and the two render-size publishers AGREE today — there is no code defect left to find.** `print_the_two_render_size_publishers` (an ignored report in `enemy_body_scale.rs`) asks the catalog route and the renderer route the same question about the same box: every row reads `drawn/box 1.00`, `stretch 1.000`, and the two quads are identical. D165's *"find both before editing either"* is done.
  * ⭐ **the measured cast, from that report** — every `Standard` humanoid is exactly 48.0 tall because that is the default, and your robot is 48.0 too:

    ```text
    npc_pirate_raider/lookout/navigator/quartermaster   25.3 x 48.0
    npc_pirate_admiral                                  27.0 x 48.0
    npc_pirate_cutlass_viper                            41.4 x 48.0
    npc_alice                                           17.5 x 48.0
    goblin / npc_goblin_brute                      51.6 / 45.2 x 48.0
    npc_pirate_heavy_broadside_bess                     69.3 x 58.7   ← Wide, legacy road
    npc_pirate_heavy_iron_mary                          70.7 x 56.2   ← Wide
    npc_pirate_heavy_salt_annet                         68.1 x 60.4   ← Wide
    ```

    ⇒ the ordinary pirates are EXACTLY your robot's height, and the heavies are only ~20% taller than them rather than hulking. That is the whole report, in numbers.
  * ◐ **the three heavies are ON the shared road now (2026-08-22), at their own measured heights** — `broadside_bess 58.7`, `iron_mary 56.2`, `salt_annet 60.4`. `Wide` has no default, so they rode `ldtk_box × collision_scale`; authoring the height they already had swaps the road without changing the look (two are byte-identical; `salt_annet` moved 0.1px in width because I authored the rounded number). ⇒ **making them smaller is now editing one number each**, which is what you asked for.
  * ▢ **the six Standard pirates still need your number.** `npc_pirate_admiral/cutlass_viper/lookout/navigator/quartermaster/raider` are `Standard` (48 today; your "2x" = 96). The three heavies — `broadside_bess`, `iron_mary`, `salt_annet` — are `body_kind: Wide`, which deliberately has NO default height, so they still ride the legacy `ldtk_box × collision_scale` road; they need a `standing_height` each rather than a smaller scale. ⚠ 17 rows author no `body_kind` at all, and `Wide`/`Floating`/`Crawler` (27 rows) have no shared unit by design — say whether they should.


* The pirates in the pirate sky no longer ride their sharks. 
  * ✔ Fixed — your 2026-07-06 editor session dropped the four mount refs in `sandbox.ldtk`; they are restored byte-identical from git and guarded. GNU-ton's boss mount turned out never to have been covered at all.

* In the sky enemy the instance of iron marry doesn't use her swordgun, she shoots fireballs, which is not something her character should be able to do. I suppose we do need a distinction between unique characters and re spawning archetype characters.
  * ✔ Answered by you 2026-08-10 — a character is a reusable authored template, and a body always receives that character's prepared kit. D73 is closed; the current model is `docs/systems/actors-brains-and-character-content.md`. The migration plan and its Iron Mary acceptance evidence are archived at `docs/archive/planning-superseded/2026-08-13/character-template-architecture-2026-08-10.md`.

* Changing rooms flashes magenta squares for a brief moment. We need to have cleaner transitions between rooms than that.
  * ⊙ Did NOT reproduce on the one route I could drive: pressing F through `pirate_cove -> central_hub_complex` and photographing frames 92-97 of the transition shows zero magenta (detector proved live by planting a magenta square, which it counts exactly). That route draws a cover, so which rooms did you see it between?

* After I fought in the pirate sky, and enemies died and dropped their swords I walked into the ninja dojo and there was a laser sword gun just existing there. I was able to fly to it and pick it up and use it. So props are not being despawned correctly - or made intangible and queued for removal - when you leave a room. I suppose there is an interesting question because it means we have to answer the question: in the ambition game, what should happen if you leave an item somewhere? Should it despawn? When? If you come back should it still be there? For the skyrim aspect of the game I think sometimes we do need items to remember where they were and if they were moved, but maybe we defer that and just have items be scoped to rooms. 
  * ⊙ This is decision 7 in `awaiting-maintainer-decision.md`, waiting on YOU, not on engineering: the lifetime bug is already fixed for coin/health/ability drops (they and their visuals share room scope), and what is left for a dropped WEAPON is a product rule — vanish on leaving, persist on returning, or something else.


* When I have the laser sword in ambition and I use it, I incorrectly still use my normal jab attack. Holding an item should reroute normal attack actions to the item action, which might be like throw for bombs or fire for the gun sword. Some items may do different things depending on if your attack is a directioned tilt or airial or neutral, but the default for the gun sword is they all route to the one action the item has: shoot (I guess direction does change which way it shoots).
  * ✔ ALREADY FIXED, and now pinned. The press was claimed by two authorities — equipping cleared the item's melee out of your `ActionSet`, but your MOVESET still bound `attack`, and the Attack slot is their union — so `trigger_moveset_moves` arbitrates by what is in the hand: a weapon with a swing answers with its own, a weapon without one leaves the press to `fire_held_ranged_system`, which shoots along your aim.
  * ⊙ so the gun-sword routes Attack to the bolt today; say if the DIRECTIONAL half you described (a tilt or aerial doing something different from neutral) is still wanted, because every direction currently routes to the one shot.

* In the ambition game, when I move from one room to another that is separated in LDTK, the camera moved as if there is a pan that should be happening. The camera room transition pans are just wrong.
  * ⊙ **I could not reproduce it, and three mechanisms are refuted — please name the two rooms.** Measured: the room id and your body's teleport land on the SAME frame; `PresentedPose` parks at the destination on a teleport rather than leaving a stale delta; and a census of every authored transition says all 151 (127 Door, 24 EdgeExit) cross an area boundary, so `room_changed` is true for all of them and the camera adopts the new target instead of easing to it.
  * ◐ **the reset-to-spawn half is closed 2026-08-24: the PLACER asks now** (`PlayerBlinkCameraState::snap_after_placement`), because `blink_cam.reset()` was clearing the snap on the one teleport that most needed it. BOTH reset roads adopt it — the session reset and `reset_sandbox`, which is the one a hazard death takes and therefore the one that fires most. ✔ both roads call ONE verb now (`reset_to_spawn`), so the ordering that caused it is unspellable; poison-verified at the verb and at the hazard-death call site.
  * ✔ **CLOSED 2026-08-26 — the follow path has the term now.** Same predicate the cast camera uses (`presented_pose::travelled_under_own_power`), a `Local` record of where the subject was, and a straddling arm so it fires on PLACEMENT rather than on speed. ⚠ ONE CONFLICT FOUND: a portal transit is a teleport that ALREADY has a continuity policy — it holds the body at the same screen position by offsetting the view — so snapping on top of it produced a 178px visible step. The portal's own presentation inputs say when it is presenting a translation, and only the teleport term is suppressed. Original report: the camera snaps ONLY on a room change or a blink, never on "the subject teleported". A synthetic teleport inside one room panned it 440px over about 40 ticks. ⚠ the multi-fighter cast camera already has the missing term (`CastFraming::teleported` arms a settle allowance); it is the single-subject FOLLOW path that has none.
  * ⛔⛔ **AND THE PORTAL LAB IS NOT ITS CUSTOMER — I tried to make it one and the suite refused.** `sandbox`'s `portal_lab` does author fourteen portals in one room, so a transit there IS a same-room teleport; but a portal transit is the case that must NOT snap. Adding the term made `c135_to_c134_preserves_screen_position_and_keeps_falling` fail with a 177px visible step, which is the whole point of `PortalCameraTransitMode::Continuous` (Ambition's default): walking a portal is meant to be seamless, so the camera FOLLOWS the body through rather than adopting the far side. ⇒ a snap-on-teleport term has to be asked for by whoever places the body, not inferred from the position jumping — a portal transit and a respawn look identical from the camera's side and want opposite answers.


* The main character shield sprite has the bubble in the wrong place, just kinda to the upper left. 
  * ✔ Fixed — the block row drew the shield at a hardcoded (64,63) while your torso is at (112,129), so it landed 48px left and 66px high. It now centres on the torso anchor.
  * ⊙ The bubble surrounds your torso and your head pokes above it. That is a radius, not a bug — say if you want it to cover the whole robot.

* In smash, choosing robot v3, if you do your attack (among other issues with smash combat right now) the VFX happens in the top left corner, not in the authored area for the character.  
  * ◐ A slash whose owner could not be found was being drawn at the world origin; that now warns instead. The attack art is measured clean, so please reproduce it once and tell me if a warning appears.

* NOTE: developing out the smash combat system and the ambition combat system should be very similar and feed each other, because I want the combat in ambition to feel a little smash like. I want knockback to increase depending on how damaged you are. The ambition game will have health, and not percent, so there will be a limit, and maybe some enemy characters won't have this property, but the main character will. The knockback is what will make this game fun.

* We need an animation for a main character for when they are knocked down. We need an animation (or at least architecture slots) for a slow getup, a tech, and a getup attack. All smash characters will need this too.
  * ✔ **the slots and the MECHANICS both exist** — `movement/knockdown.rs` is the full tumble → knockdown → tech → getup cycle: a tech window (~20 frames, the Ultimate one), a lockout for a mistimed tech, tech roll and wall tech as distinct motions, invulnerability on a successful tech and on standing up, and `MovementOp::GetupAttack`. The animation rows are `knockdown` / `getup` / `tech` / `getup_attack`, and a sheet without them falls back rather than breaking.
  * ▢ **what is missing is ART, on 9 of the 16 select-screen fighters.** Carrying all four rows: `player_robot_v3`, `perfect_cellular_automaton`, `npc_emmy_noether`, `npc_carl_stargan`, `special_patent_clerk`, `pointed_polygon`, `pugnacious_polygon`. Carrying none: `george_booul`, `mary_o_v2_tall`, `sanic`, `npc_pirate_admiral`, `npc_ninja_shadow_oni_leader`, `npc_alice`, `npc_bob`, `npc_oiler`, `goblin`.
  * ⊙ **Re-measured 2026-08-26 across all 203 shipped sheets, and the count is 10, not 7** — `author`, `officer` and `projectile_polygon` also carry all four, so nothing regressed since the row was written. Every one of the ten is a fighter whose rig was built after the knockdown cycle existed. ⚠ and the nine are NINE SEPARATE RIGS (`george_booul`, `sanic`, `_pirate_rig`, `ninja_side`, `alice_cryptographer`, `bob_engineer`, `oiler`, `goblin_side`, `mary_o_v2`) rather than one shared builder, so this is per-rig pose authoring — the scientist trio's `build_scientist_fighter_rigs.py` covers three of the ten but none of the nine. ⇒ **what the nine show today is the named fallback, not nothing**: `body_state_clip` falls `knockdown → prone → land_hard → hit → idle` and `getup → land_recovery → idle`, so a knocked-down goblin plays its HIT pose while lying there. That is what makes this ART rather than a bug, and it is also why it reads as wrong rather than as missing.


* in the title menu FPS is 60 FPS, whereas ambition itself gets 140 FPS, and I don't know if the title 60FPS is intentional
  * ⊙ Not intentional — there is ONE global `bevy_framepace` limiter driven by the Video `frame_cap` setting and nothing anywhere paces the title differently, so the split is emergent; say whether you want the title capped on purpose.


* When you challenge PCA in the C4 symmetry room we should change the music to a smash track.
  * ✔ SHIPPED 2026-08-22. `<<music "super_smash_siblings_theme">>` is authored on the *Challenge it.* choice; the claim outranks the room, is outranked by a live fight, and is released when you leave the room.
  * ⊙ **the authorization cost in the trace below was wrong** — gameplay contexts pass no allowlist, so the whole registry is already authorized and nothing needed authorizing.
  * ⊙ **RE-PRICED 2026-08-21: it is NOT "one value on the encounter", and the earlier note had the shape wrong.** Traced end to end:
    * ⭐ the moment is `assets/dialogue/sandbox/symmetry.yarn:56` — the *"Challenge it."* choice fires `<<challenge>>`.
    * ⭐ **the track exists**, so no regen is needed: `super_smash_siblings_theme`, `_grand_symphony` and `_character_select` are all in `music_registry.ron` already.
    * ⛔ **the room's own music must STAY.** `symmetry_room` authors `music_track: for_emmy_forever_ago` in `sandbox.ldtk`, which is right for the puzzle — the CHALLENGE is what should swap, not the room.
    * ⛔⛔ **and it must not go in `cmd_challenge`.** That command is the GENERIC dialogue-gated fight trigger whose own doc says *"Any content … arms a boss/duel by authoring this one command on a choice; no Rust per-NPC branch"* — putting a track there would play a smash theme for every challenge in the game.
    * ⇒ **the slice is a new authored command**, `<<music …>>`, sibling to the `play_sfx` already in `yarn_vocabulary.rs`, plus one authored line on that choice. ⚠ **the cost that was missed:** the music director is AUTHORITY-GOVERNED (`MusicAuthority::Governed { authorized }`), so a requested track that the active context does not authorize is refused — the symmetry room's context has to authorize the smash track as well as the command existing.


* I want to implement a camera mode for gravity where the camera just follows the player's reference frame. We should be careful so we use player-reference frame inputs in this mode. It doesn't need special gravity affordances. 
  * ✔ SHIPPED 2026-08-17. Gameplay → **Camera Frame**, *world-fixed* / *player-relative*. The world-observer camera stays the default and remains an option.
  * ⭐ the player-reference-frame input you asked to be careful about needs no separate setting: a player-relative view makes screen axes BE body axes, so all three input frame modes collapse onto body-relative — an identity, pinned by `a_player_relative_view_collapses_every_input_mode`. The movement/aim rows say they are inactive rather than being overwritten, so your choice survives switching back. Design: [`engine/camera-reference-frame-policy.md`](engine/camera-reference-frame-policy.md).


* Holding up for 2 seconds should be an alternative way of entering a door or interacting with an object.
  * ✔ SHIPPED 2026-08-22. Two seconds of Up buffers an interact, beside the press and the double-tap; the door prompt says so and the hold matches the possession gesture's length.
  * ⊙ ⛔ **a warning for whoever tests input next:** `AgentAction::up_pressed` is the rising EDGE, not the level. Re-sending it every tick is a machine-gun double-tap that opens a door in four ticks, and it made the first draft of the regression pass with the feature unwired.


* There isn't a quit to title option in the smash menu selection.
  * ⊙ `PauseEntry::QuitToTitle` exists and smash's `visible` feature does install the shell that shows it, so which screen did you mean — the pause menu, or the character SELECT screen (which is the demo's own UI and has no such row)?


* The smash UI for character select looks good, but the controls don't feel good, they are very hard to use with a gamepad. 
  * ⛔ WITHDRAWN — I suggested the pad was not its own source under the default policy. It is: smash claims `JoinToClaim` on its own routes, so that explanation does not apply and this stays a feel report.
  * ✔ **FOUND IT, 2026-08-26 — and it was not a feel question after all: THE CURSOR COULD NEVER SNAP TO A PORTRAIT, on any device.** `drive_the_cursor` carries the rule in its own comment — *"A HELD STICK ROAMS; A TAP STILL SNAPS"* — and then tested `nav` FIRST, so the snap branch only ran on a frame with a direction edge and NO held deflection. That frame does not exist on real hardware: `decode_menu_frame` builds `nav` from the held d-pad and held arrow keys as well as the stick (`held_x`/`held_y`), so on the very frame any edge fires, the same direction is held. ⇒ every input — stick, d-pad, keyboard — took the roam branch, and driving the grid meant steering a free pointer over nineteen small portraits with a thumbstick. The edge goes first now: a flick lands on the next portrait's centre, and a deflection still held between repeats roams freely, so the genre's roaming hand survives for a player who wants to sweep. Guarded by `a_flick_snaps_even_though_the_same_direction_is_held`, which drives the edge and the deflection TOGETHER because that is the shape production sends — the two existing cursor tests each drove only one and both passed over the defect. Poisoned by restoring the old branch order: the cursor stops 23.6px short of a centre.

* In smash it should be easy for 2 controllers to select their own characters, or turn other characters off or into cpus, any controller should be able to turn a slot into a player if there is a controller connected to it.
  * ▢ ⛔ I first wrote here that this was built and switched off. **That was wrong — I truncated my own grep with `| head`.** Smash DOES claim `JoinToClaim`, route-scoped, while the select or gameplay route is up, and releases it on leaving. So keyboard + one pad should already be two players and the enum default never applies here. ⚠ the claim moved on 2026-08-21: it is now one owned `LocalSeatOffer` carrying the seat count and the policy together, so grep `offer.claim(SMASH_SELECT_EXPERIENCE` rather than the old resource names.
  * ✔ **AND ONE HALF OF THIS REPORT WAS A REAL BUG, found 2026-08-21.** *"any controller should be able to turn a slot into a player"* worked; driving the slot afterwards did not. The screen keyed its cursors by INPUT SEAT and then used that same index as the ROSTER CARD — so with a CPU sitting between two people, the second person's presses landed on the machine's card and their own was unreachable. `SmashSelect::slot_driven_by` is the translation; a second human can now select through a sparse roster. Fixed in `5902930a7`.
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

  ⛔⛔ **THAT MECHANISM IS WRONG, AND THE MEASUREMENT THAT PRODUCED IT COMPARED
  ONE PAGE OF A SEVEN-PAGE ATLAS. Re-measured 2026-08-26.** The census above read
  the width of `noether_spritesheet.png` — page ZERO — and found 4096 on both
  sides. Noether's canonical atlas is **seven pages**; her 0.5x tier is **two**.
  Comparing total atlas pixels instead:

```text
                              pages  total px      ratio
noether        sprites            7  115,581,204   1.000
               sprites_0_5x       2   29,147,136   0.252
               sprites_0_25x      1    7,468,032   0.065
perfect_cellular_automaton
               sprites            7  106,171,584   1.000
               sprites_0_5x       2   26,841,903   0.253
goblin (control, 1 page)          1    2,185,530   1.000
               sprites_0_5x       1      613,760   0.281
```

  ⇒ **every tier is exactly the reduction it claims**, and the two "offenders"
  were precisely the two sheets with seven pages — which is what a single-page
  comparison is guaranteed to get wrong. The frame rects agree: noether's first
  frame is `w: 148, h: 331` canonical and `w: 74, h: 166` at 0.5x.

  ⛔ **so the "invisible to the incremental regen" corollary is moot too** — it
  was reasoning about why a defect that does not exist had not been fixed.

  ⊙ **WHICH LEAVES THE REPORT OPEN WITH A CLEAN SLATE, and three causes now
  eliminated rather than one added**: (1) the tiers are real reductions; (2) the
  three asset roots agree byte-for-byte — `crates/…/assets`,
  `game/ambition_content/assets` and the web root are one file by symlink
  (`md5 22db3461dfa5`); (3) select and match resolve the same catalog id. ⇒ the
  next hypothesis has to be about WHICH TIER each road asks for at the moment it
  draws, not about what the tiers contain. The falsifier Jon can run is still the
  cheapest one: force full quality and see whether she changes in the match.

  ⛔ **TWO MORE ELIMINATED THE SAME HOUR, both of which looked like the answer.**
  (4) The renderer's GENERATED art and the PUBLISHED art are the same bytes
  (`tools/…/generated/noether/` and `crates/…/assets/sprites/` both
  `md5 22db3461dfa5`), so "new art rendered, old art shipped" is not it.
  (5) **The select screen resolves a PORTRAIT and the match resolves a
  SPRITESHEET, and those genuinely CAN disagree** — `portrait_for_declared_character`
  prefers a registered definition's portrait TARGET over the catalog row and warns
  *"two declarations of one character disagree"* when they differ. ⇒ but
  `with_portrait` has **no caller anywhere in the tree**, so the registered target
  is always `None` and both roads land on the same catalog manifest. ⚠ that fork
  is the one to re-check the day anything starts calling it.

  ⇒ six causes eliminated, and every road that can be compared from a file
  agrees. What is left needs a screen.                           

* When I change the video quality in ambition, my sprite went from the robot v3 character to the robot v2 character. 
  * ▢ **DOES NOT REPRODUCE HEADLESS, and the test now exercises YOUR case rather than a proxy.** `quality_change_keeps_each_character.rs` boots direct gameplay, finds the PrimaryPlayer's own worn character resident, changes the profile to Potato, and proves that sheet MOVES tier while its file root is unchanged — so resolution is not picking a different character. Ten causes eliminated in total. What is left is WHEN, which no file can answer: the falsifier is to change quality twice in a live session and say whether it swaps back. Owner doc: `sprite-residency-and-live-quality.md`.
  * ⊙ Two things that would settle it: was your report BEFORE or AFTER 2026-08-08, and does it swap back if you change quality again?
  * ✖ **FOURTH CAUSE ELIMINATED 2026-08-21 — it is not tier REPACKING.** The Emmy finding two entries above gave an obvious candidate: four sheets saturate the 4096 texture cap, so their "reduced" tier is a differently-PACKED sheet rather than a scaled one, and different frame rects would read as a different character. Tested against the robots — all thirteen reduce cleanly:

```text
player_robot_v2   2815x2312 -> 1472x1215   0.52
player_robot_v3   3072x2484 -> 1600x1259   0.52
… 11 more robot sheets, every ratio 0.42-0.57, none capped
```

  ⇒ **v2 and v3 are separate, correctly-reduced sheets at every tier**, so a quality change cannot turn one into the other by geometry. That leaves RESOLUTION — which sheet gets chosen.
  * ✖ **FIFTH CAUSE ELIMINATED the same day — it is not the first-record-wins discard either.** Reading the suspect function, `from_baked_table_by_file_root` keeps only `records.into_iter().next()` and silently drops the rest, so a file root whose baked RON holds several records is decided by ORDER. Measured: exactly ONE baked file in the whole tree holds more than one record (`creator_lab_props_spritesheet.ron`, 8 — props, not characters), and its order is byte-identical across all four tiers (`genesis_vat` first everywhere). ⇒ cannot swap a robot.
  ✔ **THE LATENT TRAP IS CLOSED (2026-08-21), by the refusal this line asked for.** `from_baked_table_by_file_root` no longer keeps `records[0]`; a file root holding several records is REFUSED and recorded in `SheetRegistry::ambiguous_file_roots()`, the same carry-the-fact-to-a-caller-with-a-catalog shape `shadowed_targets()` already uses — and the same posture `AuthoredSheets::insert_ron` already took. Behaviour-preserving, proven three ways: `creator_lab_props` is the only multi-record root, it is no character's `manifest_target`, and it publishes no body, so the sole consumer (`attack_hitbox`, which needs `body_metrics`) already answered `None`. Guarded by a hand-built two-record table asserting the root resolves to NOTHING — the assertion that fails against the old code.
  * ✖ **SIXTH CAUSE ELIMINATED — the variant PATH is deterministic.**
`scaled_logical_asset_path` is `Some("{folder}_{suffix}/{filename}")` unless the
name is source-qualified, so `sprites/player_robot_v3_spritesheet.png` can only
ever become `sprites_0_5x/player_robot_v3_spritesheet.png`. There is no index, no
order and no lookup in it — a file root cannot become a different character's
here. And the baked keys are `<root>.0_5x` / `.0_25x` / `.potato`, derived from
the folder, equally mechanical.

  ⇒ **STATIC ANALYSIS IS EXHAUSTED, and that is itself the finding.** Six causes
eliminated by measurement: missing art · missing `_actor.ron` sidecar · per-tier
sheet collision · tier repacking · first-record-wins · variant path construction.
Every layer that could *choose the wrong sheet* is deterministic and was checked.

  ⇒ **so the defect is in WHEN, not WHICH** — the re-materialization the
observation already suspected (`3bf154974`, 2026-08-08, which made a quality
change rebuild on-screen bodies instead of only the next room). A body rebuilt
while its new sheet handle is still loading, or rebuilt in an order that reads a
half-swapped registry, produces a correct path resolving to the previous image.
⛔ that cannot be caught by reading files — it needs a live capture across an
Apply, which is what `sprite-residency-and-live-quality.md` asks for.

  * ⊙ **the convergence read end-to-end, and one CLASS of body provably never
converges.** `converge_character_residency_to_active_quality` →
`demote_stale_realizations` retires each stale realization and re-demands it —
but only where `declared.contains_key(token)`. `publish_under` inserts into
`sheets` and NEVER into `declared`, and that exclusion is deliberate: *"art it
did not build, so retiring it would delete a face with no way to draw it
again."* Same for the id-keyed row a character gets when published without ever
being declared.
  * ⊙ **TRIED to build the reproduction headlessly, 2026-08-21. It did not ship,
and TWO of the three things it seemed to find were my own errors — recorded
because both are traps the next attempt will hit.**
    1. ✔ **TRUE and useful**: the seam is unreachable from a shell-host
       composition — `GameAssets` is ABSENT there, because character
       realizations are presentation state. It needs
       `build_visible_app(VisibleRenderMode::NoWindow, true)`, the builder
       `boot_budget` uses. That is why this seam has no coverage: structure, not
       neglect.
    2. ⛔ **FALSE — "141 declared, ZERO resolve to a sheet" is a TAUTOLOGY, not an
       anomaly.** `declared_character_ids()` filters to tokens that have NO
       resident sheet (`!self.sheets.contains_key(token)`), exactly as
       `is_declared`'s own doc says: *"declared and has no resident
       realization."* I iterated the set defined as having no sheet and asserted
       each had one. There is nothing wrong here.
    3. ⛔ **FALSE — "`publish` has no production caller".** Every
       `characters.publish(` in the tree is test code, including a file named
       `quality_convergence_tests.rs`, which made it look airtight. Production
       reaches it inside `materialize_declared_character_sprite` as
       **`sprites.publish(&cid, asset)`** — a different RECEIVER NAME, invisible
       to that grep.
  ⇒ **what the attempt actually leaves behind**: the visible-app boot is the way
  in, and there is **no public accessor that enumerates RESIDENT sheets**
  (`ready_token_count()` gives a count and nothing else) — which is the real
  reason the test could not be written, and the smallest thing to add when
  somebody builds it.

  ⇒ **so a host-published realization is FROZEN at whatever tier it was
published at, by design** — the intro's NPCs are named in the sibling comment as
exactly this case. That is a documented limitation rather than the swap, and it
is worth knowing before somebody "fixes" the guard: removing it deletes faces
permanently. ⛔ if a robot v3 body is ever published this way, it would hold its
old realization across a quality change while a declared sibling moved — which
is the shape to test first when the live capture happens.

## 2026-08-25 — Jon's session: hitbox, menus, touch, naming, respawn

Recorded verbatim from Jon while he played; none of these are triaged yet.

* Sanic's hitbox is always UNDER the surface. Happens in smash and in his own game.
* Menu up/down select (the control text) OVERLAPS the buttons in many menus.
* Smash "quit to title" quits to a DIFFERENT GAME — often Ambition itself. From there
  a second quit does reach the real title screen.
* Opening the menu on the TITLE SCREEN puts the settings menu BEHIND the select-game
  menu, which makes it unusable; same in loading screens. Jon: *"Typically the pause
  menu should supercede whatever is behind it unless it is a live online game -
  which we don't have yet."*
* Touch: using the on-screen JOYSTICK should override the screen's own touch controls,
  so the joystick can manipulate a token.
* The character select grid should favour MORE COLUMNS THAN ROWS.
* "Mary-O (Tall)" should just be **"Mary-O"**. Jon: *"the characters should not be
  required to have a unique presentation name. Or maybe they have an optional
  distinguished presentation name if we really need it."*
* Smash respawn: jumping while respawning RAISES THE CHARACTER UP ON THE PLATFORM.
  ⚠ adjacent to D192 — a returning fighter should not be able to act before it lands.
* HITBOXES ARE AUTHORED TOO SMALL, and ⛔ Jon does NOT want them magically scaled.
  He wants a better way to AUTHOR them, so they make sense and so there are good
  patterns for building new characters. Jon: *"Something that should be generally
  true for a direction smash is that they should hit a fair bit of area in the
  direction of the player hitbox, often at least as tall or wide as the character
  dimension. e.g. forward smash should have a hit geometry such that everywhere in
  front of the character gets hit and there aren't often holes a character can duck
  under. similar for arcs of up airs and other attacks. the hitboxes are generally
  too conservative. and note some attacks might be special and break those rules of
  thumb. a we are going to give the cast unique and interesting moves."* ⇒ D203.

* UP-B CAN OFTEN BE USED MORE THAN ONCE without going into freefall. Jon: *"only a
  few should be exempt from that general rule."* ⇒ D204. ⚠ adjacent to main's
  `Helplessness is an episode, not a count of charges` — the episode may be the
  right home for the once-per-airtime rule rather than a charge count.

* A TELEPORTING UP-B with player-controlled direction, like Mewtwo's in Smash.
  Jon: *"the author should have this up b. and the robot might have a similar
  teleport up b similar to its blink in the game."* ⇒ D205. ⭐ the engine already
  has the mechanic — `blink_aiming` / `blink_aim_offset` / `blink_hold_active` /
  `blink_hold_timer` in `movement/model.rs` is a hold-aim-release teleport — so
  this is WIRING an existing verb as a recovery, not inventing one.

* SFX NOISE in a Goblin vs PCA (`perfect_cellular_automaton`) fight IN SMASH — either side triggers far too
  many sounds. Jon wants a test that stages that fight and COUNTS the triggers,
  because the volume of them may be a symptom of a deeper bug, not a mix problem.

## 2026-08-24 — Smash: a shield roll throws the fighter across the stage

Jon: *"Another issue is that shield rolls have too much motion to them. They
send the character flying across the stage. They should not be giving that much
velocity, and they probably should stop at the end of the roll and leave the
character punishable for a frame or two."*

✔ FIXED 2026-08-25. Jon was right and the earlier verdict here was wrong: the
roll did not take its velocity back when it ended, so the body kept sliding at
the full roll speed through the whole cooldown and then rolled again. Held
guard+direction covered **1339px in three seconds on a ~480px stage**. A roll
now sheds its own push and ends standing: 229.7px per roll-cycle → 114.8px, and
1339px → 804px over the same three seconds.

⛔ THE EARLIER MEASUREMENTS IN THIS ENTRY WERE ARTEFACTS AND ARE WITHDRAWN
(11.2px ground roll, ~26px/s chained). They were taken through `App::update()`,
which is a FRAME and not a sim tick — the probe sampled ONE TICK of a roll
(8.8px) and it was read as the whole roll, which is 124px. Re-measured in the
kernel, where a tick is a tick. ⇒ knockback was never needed to explain this.

⚠ WHAT IS LEFT IS A TUNING NUMBER, NOT A BUG: the roll's own distance is still
124px, about a quarter of the stage. If it still reads as too far when you play
it, the knob is `DODGE_ROLL_SPEED` (530px/s over `DODGE_ROLL_TIME` 0.22s) — one
value, and unchanged so far because that is a feel call rather than a defect.
`cargo run -p ambition_demo_smash_app --bin roll_probe` prints the
frame-by-frame if you want to see a clean roll for comparison.

✔ **2026-08-25 — AND THERE WAS A SECOND, REAL BUG UNDERNEATH, which your later
report named exactly**: *"roll distance is input/history-dependent after the roll
has already begun"*. Two causes, both fixed (D235). The same roll covered 106px
with the guard held and 33px with it released, because ordinary friction was
gated on `shield_held` rather than on the roll being active; and a roll spammed
three times covered 27px instead of 106, because dodge STALING was shortening the
maneuver clock rather than only the invulnerable window. A roll's distance is now
the same however you let go of the button and however many you have just thrown.
The 124px feel number above is unchanged and still yours to call.

## 2026-08-24 — Smash: a name floats over everyone EXCEPT player one, and barks fire on every hit

Jon: *"Something I would like fixed in smash is it looks like non-player 1 gets
a name over their head, whereas player 1 does not. This is player 1 centric
behavior, and we should have none of it. A second thing I would like is in smash
to not have barks happen every time a character is hit. Make it a more rare
event. Not never, but I'd like it to happen less often."*

✔ Nameplates: the Smash stage now labels every fighter the same way.
✔ Barks: Smash declares one hit in six; the rate is a knob, so say if it is
still too chatty (or too quiet) and it is one number.

## 2026-08-24 — Smash: the ledge has no rules

Jon: *"in smash we haven't built any of the ledge rules yet. A character can
just stay on the ledge, and there is no way to knock them off. If you get hit
you should fall off the ledge at least."*

◐ A hit now takes the hang: the rule existed and only the PLAYER road called it, so on
the actor road every fighter in the arena hung through an edge-guard untouched. The
getup vocabulary turned out to be complete already (roll, ledge jump, getup attack,
climb, drop) and ledge trumping is live. Two more landed 2026-08-24: catching the
ledge now restores the jump and air dodge at the LATCH (Jon: *"just grabbing the
ledge should restore the jumps"* — it was on two of the five ways OUT of a hang and
absent from the other three, so dropping off the lip cost you everything), and a
fighter can now reach the ledge at all to guard it — the corner test asked for the
nearest edge in both terms, so the CPU flipped to "cornered" 90px from the lip and
retreated. What remains is a hang time limit and a regrab count, both re-researched
because D201's first pass had the reference facts wrong. The hang limit is now
SHIPPED too — a body that hangs past 5 s is dropped, which is the genre's own
rule and the direct answer to "a character can just stay on the ledge". What is
left is a regrab COUNT, which is available and deliberately unbuilt until play
shows stalling surviving the four penalties the ledge now charges — queued as
D201.

✔ And 2026-08-25: stealing the ledge now THROWS the previous holder off it
(`ledge_trump_pop`, a declared match rule) rather than dropping them on the
spot, so a trump is a real edge-guard option.

## 2026-08-21 — Mary-O: the pole victory TRANSLATES her, and side contact hits the snake

Jon: *"if she runs into solid snake from the side, the snake gets hit instead of
her. And when she gets a victory for the pole, her sprite seems to just translate
instead of using the climb animation to slide down the pole and then the walk
animation to move after. We should not hack these in, these should be done via
scripted control of the character in an elegant way so we get these animations
for free."*

  ✔ **FIXED 2026-08-21 — she was told she was standing still.** The sequence
  imposed `Vec2::ZERO` as her velocity through `constrain_body_pose`, whose own
  doc names *"a scripted end-of-level slide"* and has taken an imposed velocity
  all along. So every reader of her motion heard "not moving" while her position
  jumped, and the animation picker — which reads exactly those facts — correctly
  chose Idle. ⇒ `step_flag_sequence` returns a `FlagDrive { pos, vel, on_pole }`
  now: the true velocity is imposed, and `on_pole` sets `BodyMode::Climbing`,
  which the picker already turns into the climb clip. **No clip is named anywhere
  in the flag code**, which is the part Jon asked for. Guarded by
  `the_slide_climbs_and_the_walk_off_walks`, which fails against the old zeroed
  facts. ⚠ still to see on hardware — this pins the facts the picker consumes,
  not the pixels.

  ✔ ~~THE POLE SEQUENCE MUST DRIVE HER, NOT MOVE HER.~~ Today `run_flag_sequence`
  writes `FlagSequence::driven` onto the body through `constrain_body_pose` every
  tick the phase is not `Idle` — a POSITION write, which is exactly the
  "translate" Jon is seeing, and it is why no animation plays. ⇒ the sequence
  should hold the character through the scripted-control seam and express the
  slide as a CLIMB and the walk-off as MOVEMENT INPUT, so the animation picker
  chooses the clips for free. ⛔ Jon named the approach, so do not pin a clip by
  hand as a shortcut — that is the hack he is refusing in advance.

  ✔ **FIXED 2026-08-21 — the stomp band was a fixed 16px, and the snake is 9.48
  tall.** `player_touch` called it a stomp whenever her feet landed within 16px
  of the enemy's head; two bodies on the same ground have their feet on the same
  line, so her feet sat exactly `enemy_height` below its head — inside the band
  for anything 16 or shorter. MEASURED from the shipped art: the snake's authored
  collision box is 21.33 x 9.48, barely half the band. The band is clamped to half
  the body now, which is the rule these games actually use (feet above the
  enemy's middle) and is height-independent, so a shorter enemy authored tomorrow
  cannot reopen it. Guarded by `running_into_a_short_enemy_on_flat_ground_hits_its_side`
  plus a control proving short enemies are still stompable from above; the first
  fails against the old rule.

  ✔ ~~Side contact with Solid Snake damages the SNAKE, not her.~~
  ⭐ **and the other direction went green with it.**
  `power_loop::her_spark_damages_a_snake_through_the_shared_hit_pipeline` had
  been failing with *"was 1, now 1"* — her spark did NOT damage a snake that
  should take it — and it predated both of that session's movement changes. It
  was an unrealistic FIXTURE, not a second defect in the seam: the firer carried
  no `ActorFaction`, so `strike_reaches_victim` could not tell whose strike it
  was. Repaired to model production construction rather than weakened. Mary-O is
  151 + 11 green.

## 2026-08-21 — Mary-O: tall crouch is too tall, and death keeps the camera panning

Jon: *"Her tall crouch should be the same height (in terms of collision as her
small form, but currently its a bit too tall, so she can't clear places she used
to be able to), and also when you die, the camera will still keep panning. Her
death should stop her velocity to play her death animation, so the camera should
stop too as a side effect."*

  ✔ **FIXED 2026-08-21 — crouch is HALF height, not 0.55.** Measured by
  restoring the old ratio: her crouched grown body stood **35.20** against a
  **32.00** small form, missing every gap authored for the small form by 3.20
  units. Half gives 32.00 exactly. ⚠ the engine's own `shape` test asks only
  `crouch.y < standing.y`, so it was blind to the ratio and stayed green
  throughout; the guard with teeth is
  `her_grown_crouch_fits_where_her_small_form_fits`, which names the gap she must
  fit and fails on the old value.

  ✔ ~~Tall crouch collision height must EQUAL small-form height.~~ It is the SMB1
  rule and it is what makes a crouch-slide under a one-tile gap work at either
  size. The symptom Jon names — places she used to clear and no longer can — is
  the observable, so the guard belongs on the crouched envelope, not on a
  constant.

  ✔ **FIXED 2026-08-21 — `step_body` freezes an out-of-play body, and the camera
  stops because it is following a body that stopped.** ⛔ still do not fix this
  by pinning the camera on death — Jon states the causal order explicitly, and a
  camera that stops while the body drifts is the same bug wearing a hat.

## 2026-08-20 — doors do not work in Ambition, and Mary-O's 1-1 loops

Jon: *"In ambition I cannot go through any doors anymore. This is both interact
doors and contact doors. In maryo finishing 1-1 sends you back to 1-1. These are
huge regressions, not sure how we didn't have a test to catch these."*

  ✔ **Contact half DIAGNOSED, and the mechanism is fine.** She stalls at x=1841
  walking at the right edge exit — but there is no wall: the Collision IntGrid is
  empty through the whole zone except its BOTTOM ROW, where three cells form a
  16px floor lip. She collides with its side. Pulse a jump and she is in after
  211 frames and the room changes to `scroll_lab`, so contact transitions work.
  ⛔ **not a regression** — those cells are solid in every commit back to
  2026-08-15. It is a standing conflict with `EdgeExit`'s contract ("walks off
  the screen into it").

  ✔ **FIXED 2026-08-21 — it was a SILL, not a lip, and the opening was a
  window.** The exits are holes in a wall three cells thick and its bottom two
  rows were still solid, so each opening sat 32px above the floor. Cleared, and
  the floor at row 61 is unbroken across the level so no pit appears
  (`ambition_map_assets` `db7e72f`). She walks out now with the jump deleted
  from the test. ⛔ and the census that called it five zones was wrong: the other
  three have solid cells in their bottom row because that row IS the floor.

  ✔ **RESOLVED ON JON'S MACHINE, 2026-08-21** — *"moving through trigger and
  contact loading zones works again."* No fix was ever committed for it, which
  matches every measurement below: the defect was never in the tree. ⇒ treat the
  eliminations as the durable output, and note what actually guards it now —
  `walking_into_a_loading_zone.rs`, `mary_o_lap_in_the_host.rs` and
  `fly_to_the_hall_of_characters.rs` drive real doors through the shipped
  launcher, which is the test Jon asked for and did not exist when this was
  filed.

  ▢ **The interact half did not reproduce anywhere I could build it**, and the
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

