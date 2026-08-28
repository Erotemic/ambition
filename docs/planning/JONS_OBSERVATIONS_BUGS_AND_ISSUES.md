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
  ⊙ **Re-swept 2026-08-26 by rendering all 186 targets: MOST of the sheets you named are clean now** — `robot`, `player_extended`, the robot variants and `ninja_shadow_oni_leader` render without the guard firing, and `super_sanic` was fixed 2026-08-18. ⛔ `puppy_slug` is NOT: it still cuts 17 frames. ⚠ the defect is not gone, the population moved: **40 of 186** targets clip, and the one that matters is `perfect_cellular_automaton` (53 frames) because it is the only select-screen fighter in the list. Details in D129.

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
  * ✔ **the UNITS question is ANSWERED and needed nothing from you (2026-08-28).** This used to say D165's *"base-grid pixels (16 to a tile)"* and the implemented world-px `standing_height` differ by 2× in Mary-O, and to hold off wiring anything until somebody said which was meant. Measured: **the two games use different tile sizes** — the LDtk worlds declare `"defaultGridSize": 16` and `game/ambition_demo_mary_o` declares its own `const T: f32 = 32.0`. So 48 world px is three tiles in one game and one and a half in the other, and `standing_height` is WORLD PIXELS in both. ⇒ nothing converts and nothing needed deciding; what is not portable is saying a height in TILES.
  * ✔ **the quad-from-bbox half is BUILT TOO, and this row's own note about it is stale (re-read 2026-08-22, by a different route: the code, not the design doc).** `sprite_body_collision_for_character_id_from_data` has two branches. With a `standing_height`: `scale = height / body_h` where `body_h` is the sheet's MEASURED body, then `render = frame x scale` — so the quad derives from the body and **cannot stretch**, because both axes take the same scale and the frame's aspect is kept. Without one: `ldtk_collision.max x collision_scale / frame_h`, which is the level-editor rectangle you are actually complaining about. ⇒ the stretching note describes an older approach that is no longer what this code does.
  * ⭐⭐ **RUN AGAINST REAL DATA, and the mechanism honors your input exactly: the three heavies derive 58.70 / 56.20 / 60.40, to the millimetre.** The six `Standard` pirates derive **48.00 — the robot's default, exactly** — which is your report, measured. There is now a guard (`an_authored_standing_height_is_the_height_the_body_derives`) pinning the equality for every row that authors a height; it names no character and no number, so retuning any of them is an ordinary content edit. Poison-checked: dividing the scale by the frame instead of the body turns 58.7 into 40.15 and the guard names the character.
  * ⚠ **but the audit surfaces a SECOND size fact you have not been told, and it is the one your eye is probably catching.** At the same 48.00 body, the DRAWN quad ranges from 1.00x to 1.57x across ordinary humanoid NPCs — `npc_ninja_shadow_duelist` 1.00, `npc_pirate_admiral` / `lookout` / `navigator` / `quartermaster` / `raider` 1.02, `npc_pirate_cutlass_viper` **1.44**, `npc_lab_raider` **1.57**. The engine considers all of them exactly the same height; on screen `lab_raider` is half again taller than the admiral. That ratio is `frame / body` — pure ART PADDING — and no height you author moves it, because the scale cancels.

    ⛔⛔ **AND THE CONCLUSION THAT FOLLOWED FROM IT WAS WRONG — corrected 2026-08-28 from the code, not from a re-measurement.** It said this half was "done in data by the sprite renderer": a re-crop of the sheets carrying 40-60% empty space. **A re-crop would move nothing on screen.** `sprite_render_size_scaled` says so in its own doc — *"Transparent frame padding remains part of the quad; only the body's measured extent determines world scale, so the art is not stretched"* — and the arithmetic is one line:

    ```text
    fit  = min(collision.x / body_px.x, collision.y / body_px.y)   ← no `frame`
    quad = frame × fit          shrinks when you re-crop
    ink  = body_px × fit        DOES NOT
    ```

    ⚠⚠ **AND THAT CORRECTION WAS ITSELF HALF WRONG — corrected again the same day, by a measurement rather than an argument.** It is true on the `standing_height` road and FALSE on the legacy one, and `catalog_join.rs:154` is where they part:

    ```text
    height road   scale = height / body_h                 no `frame` anywhere
    legacy road   scale = ldtk_max × collision_scale / FRAME_H
                  ⇒ collision = body_px × ldtk_max × cs / FRAME_H
    ```

    ⇒ **on the legacy road the frame is a DIVISOR of how big the character is**, so transparent margin really is a size control there: pad the frame and the character shrinks, crop it and they grow. Measured, not derived — widening `perfect_cellular_automaton`'s frame from 492x684 to 654x846 with **no art change at all** took its body from 67.8 to 54.8. ⇒ **27 of 134 characters are on that road** (every `Wide`/`Floating`/`Crawler` that does not author a height), and for them your "re-crop the sheets" instinct was right.

    ⇒ the padding is TRANSPARENT, so `frame/body` is how much empty margin each sheet carries, not how tall anyone looks — **for the 107 characters on the height road**, which is every `Standard` (48 by default) plus the four rows that author a number. `npc_lab_raider` and `npc_pirate_admiral` draw a body **48.0 px tall each** — measured at HEAD by `print_the_two_render_size_publishers`, whose `drawn(render)` column is the ink: admiral `27.0x48.0` inside a `44.1x48.9` quad, lab_raider `51.6x48.0` inside `71.3x75.4`. Same height, different margins. ⇒ **re-cropping is a memory and overdraw win and NOT a size fix**, so it is not on this item's road; the six numbers are. (`player_robot_v3` 2.81 and `npc_puppy_slug` 3.07 are not comparable — the robot lineage is `SpriteAuthored` so this number never ships for it, and a sprawled quadruped legitimately draws past its body box.)
  * ⇒ ⭐⭐ **so all three of your size reports reduce to ONE blocked question, and it is the six numbers.** **22 of 132** rows author a height as of 2026-08-28 — the three heavies you ruled on (`broadside_bess` 58.7, `iron_mary` 56.2, `salt_annet` 60.4) plus nineteen authored at the size they already had, and `perfect_cellular_automaton`. ⚠ **the "142 characters still take the legacy branch" this used to say was never right**: a `Standard` row gets 48 from `body_kind` and is already on the good road, so the legacy population was 25 and is now **3** — the puppy slug, the parrot and the flying shark, which are the only characters placed with boxes that DISAGREE. The data the good branch needs is already there: **203 of 206 spritesheets publish `body_metrics`**, all six Standard pirates included. Nothing is left to build; the six numbers are the whole remaining input.

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
  * ▢ **the six Standard pirates still need your number, and that is now the WHOLE remaining input.** `npc_pirate_admiral/cutlass_viper/lookout/navigator/quartermaster/raider` are `Standard` (48 today; your "2x" = 96 — your own words, so this is a yes/no rather than a number you have to invent). ⚠ **correction, 2026-08-28: the sentence that used to stand here said the three heavies "still ride the legacy `ldtk_box × collision_scale` road" and need a `standing_height` each. They HAVE one** — `broadside_bess 58.7`, `iron_mary 56.2`, `salt_annet 60.4`, the only three of 145 catalog rows that author a height, exactly as the ◐ line above says. The stale half was contradicted by three other lines in this same file. ⚠ **the `Wide`/`Floating`/`Crawler` half of this question is ANSWERED and did not need you** (2026-08-28). They have no shared DEFAULT height by design — *"inventing one for them would be changing sizes to satisfy a pattern rather than a report"* — but that was never an argument against each authoring its OWN. Nineteen of them are placed with exactly one spawn box everywhere they appear, so their heights were already determined; they are authored now at the sizes they had, and nothing changed on screen. ⇒ what is left of that question is three characters whose boxes genuinely disagree, and those are entry 36 in `awaiting-maintainer-decision.md`.

    ⚠⚠ **AND YOUR TWO SENTENCES IN THIS REPORT CONFLICT AT 2×, which is worth seeing before you answer.** You asked for the heavies to get *"a little smaller"* and the ordinary pirates to scale *"up 2x"*. Those are the same report about the same six-plus-three cast, and here is where 2× lands them:

    ```text
                        today            at "2x"
    ordinary pirates     48.0             96.0
    broadside_bess       58.7             58.7  (or less — you asked for smaller)
    iron_mary            56.2             56.2
    salt_annet           60.4             60.4
    ```

    ⇒ the deckhands would stand **60% taller than the heavies**, and the heavies are the ones meant to read as hulking. ⭐ the reading that satisfies both sentences is that the ordinary pirates want roughly **1.4–1.5× the robot (≈68–72)** and the heavies want to keep a clear margin above them (≈85–95), which is a bigger heavies number, not a smaller one — your *"a little smaller"* was aimed at how they looked BEFORE 2026-08-22, when they rode `ldtk_box × collision_scale` and drew much larger quads. ⛔ I have not authored any of this; it is nine numbers and they are yours.


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
* ✔ FIXED 2026-08-26 (D201). Smash respawn: jumping while respawning RAISES THE
  CHARACTER UP ON THE PLATFORM. The beat D192 added claimed no control hold and
  no `OutOfPlay`, so a waiting body still answered the pad — and worse, the ACTOR
  road (which is the road a Smash fighter takes) passed a hard-coded `false` for
  out-of-play under a comment asserting only a participant's body could ever hold
  one. Both fixed; the waiting body is now held still and holds
  `ControlHold::Sequence`. Probed red at 174.7px of motion in 60 frames.
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

* ✔ FIXED 2026-08-26 (D206). SFX NOISE in a Goblin vs PCA
  (`perfect_cellular_automaton`) fight IN SMASH. Jon was right that the volume was
  a symptom: the two traded 6,908 body-CONTACT hit events in 3,776 ticks (109.8/s)
  because contact damage fires every tick two bodies overlap, and every one asked
  for `player.hit`. Both characters author a contact-damage block — correct for
  Ambition, wrong for a versus match — so the match now answers it rather than the
  character. 114.2 sfx/s → 11.9/s.

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


## GPT review, relayed by Jon 2026-08-26 — Smash cross-boundary pass

* Sudden death enters at 150% but does not reduce the contenders to one stock. `open_the_sudden_death_round` only changes `BodyHealth`, so a tied fighter with two or three stocks just spends one on the first KO and respawns fresh, with the clock disabled. The entry transaction should set each contender's remaining stock to one alongside the damage.
  * ✔ ALREADY FIXED AT HEAD — `open_the_sudden_death_round` sets `stocks.remaining = 1` for every contender, with a comment naming this exact bug; the review is reading an older tree.

* The same sudden-death sim system writes the persistent "SUDDEN DEATH" `HudReadouts` slot. A rollback before the timeout retracts `BodyHealth` and the message but not `HudReadouts`, so the banner can survive as speculative presentation. Belongs with confirmed-result presentation cleanup, not with the game rules.
  * ✔ the sim no longer writes the slot. The banner is DERIVED every frame from `SuddenDeathEntered`, the rollback-registered latch the card system already consulted — so it appears and disappears with the round it names and no retraction has to be remembered anywhere. ⭐ the same shape as the knockout beat above: a presentation fact written from inside the simulation cannot be rewound, because presentation is not rollback state.

* Every `MoveEventKind::Ranged` passes through a hidden 1.1-second body refire cooldown at the effect consumer, while Projectile Polygon's authored Charge Shot is 0.58 seconds long. The move-start authority does not consult that cooldown.
  * ✔ ALREADY FIXED AT HEAD — `weapon_ready` refuses a firing move at ACCEPTANCE on `ranged_cooldown <= 0.0`, `start_move` spends the authored `RangedActionSpec::refire_s`, and `brain_effects.rs` applies the low-level refire rejection only to `RangedCommitment::Attempt`, never to a committed move. ⛔ THIS IS THE THIRD STALE ONE and I called that batch "two" — I checked the two I could reach by grepping a symbol and stopped, which is the partial-sweep error again.

* `emit_knockout_beat` asks for one or two `VfxMessage::Impact`s and calls them "expanding rings", but `Impact` is the ordinary hit-marker API. Asset-backed, it resolves to the shipped generic hit effect, so an elimination draws the normal damage-impact art twice at the blast line. Semantic misuse of the VFX vocabulary.
  * ✔ the beat names `ids::SHOCKWAVE` — authored as *"the expanding ring a committed heavy throws"*, which is exactly what the field claimed to draw — through `VfxMessage::Effect`, and `rings: u32` became `ring_scale: f32`. ⭐ THE COUNT WAS NEVER A DOUBLING ANYWAY: two copies of one clip at one position on one tick are coincident, so "an elimination gets a second one" doubled the alpha and never the size; an elimination is now BIGGER. `beat_reach` takes the ring's own extent as a `max` so a bigger ring cannot quietly start spilling off the frame edge. ⛔ IT LANDED IN `bd37e4ddc` WITHOUT BEING MENTIONED IN THAT COMMIT'S MESSAGE — a broad `git add crates` swept up work in progress from a different item. The commit is green and correct; the message is incomplete, and that is the `git commit` takes the WHOLE INDEX trap.

* Stock-loss presentation has no composition policy. The launch-trail module says its flare/plume sit "on top of the hit spark and camera shake", and knockout adds another layer on top of that. Nothing at the stock-spend boundary retires or attenuates the cues whose job was to PREDICT danger, so all three modules can be locally correct and still over-signal.
  * ✔ the policy is in the GATE, not in an amplitude. `LaunchedBodiesView` is the sim's resolved "this motion is INVOLUNTARY" fact, and a body the world has its hands off is not moving involuntarily — it is not moving at all. Excluding `OutOfPlay` retires the trail at the instant the thing it was predicting happens, so the knockout owns that beat alone. ⭐ no amplitude was guessed: the predictive cue simply stops when the prediction resolves.

* `KnockoutsView` is a rebuilt read-model used as a one-tick event queue: cleared on every simulation advance but rendered once per frame. Catch-up resimulation can erase an intermediate KO before presentation samples it, and a KO on the latest speculative advance renders immediately, bypassing the confirmed-effect quarantine.
  * ✔ the view is DELETED. The beat is a `KnockoutBeatRequested` message on the confirmed-effect quarantine — journalled by producing frame, replaced on resimulation, released only when confirmed, discarded with an abandoned branch — which is what the read-model was imitating badly. The `SimTick` double-draw guard went with it: a reader cursor does not need one.

* `KnockoutsView`'s `LastSeenBodies` history is a non-rollback `Local`, so after a rewind the "where did the body leave play?" lookup can come from the abandoned future branch. This transient event belongs on the confirmed/journaled presentation-event path used by SFX/VFX/camera shake.
  * ✔ the cache is DELETED, and D201 is why it could be. It existed because the respawn teleported the body on the tick the stock was spent, so the KO position was gone before any consumer looked; a body waiting out its death window is no longer placed until the window closes, so `spend_fighter_stocks` reads the position directly. ⭐ the fix removed state rather than registering more.

* `guard_covers_hit` shifts the full-width coverage band by `shield_tilt` even though the code promises a full shield can never be poked. At max tilt (0.34 half-heights) a 100% shield covers about -0.66 to +1.34, exposing the opposite outer third; the test misses it because its full-shield control samples only -0.3. Bound the tilt by the amount already exposed — at full coverage the allowed shift is zero.
  * ✔ ALREADY FIXED AT HEAD — `guard_covers_hit` returns `true` on `coverage >= 1.0` before tilt is read, and its doc states that contract; the review is reading an older tree.

* Negative result, same pass: camera reset, crouch scheduling, Z-drop, recovery edge-cancel, defense-policy composition and the deterministic item RNG produced no additional defects under the cross-boundary checks.

## Full review of HEAD `f56b5ea`, relayed by Jon 2026-08-26 — worked in the order Jon gave

* **1. Same-frame Back+Special is double-counted as both halves of wavebounce.** Production orders `CombatSet::Trigger` → `CombatSet::Playback`; the accepted Special opens `special_turn_window` but never records the stick sign that belonged to that press, so `apply_special_turn_flicks` in Playback sees `prev_lateral_sign == 0` and calls the SAME tick's Back a fresh post-press flick — facing flips twice and drift reverses. Holding Back a frame first hides it. The accepted Special must set a baseline sign. Also: `flick_window_ticks` is aged in scaled `sim_dt()` seconds here and in integer ticks for ordinary attack flicks — one authored knob, two clock semantics. The regression must run the real Trigger→Playback graph; the existing arm installs the two halves into bare `Update`.
  * ✔ `bd37e4dd` — the acceptance seeds `prev_lateral_sign` from the press's own stick through one shared `lateral_flick_sign`, the window counts ticks, and the harness runs the production order so the turnaround-B arm IS the regression. ⚠ its descendant is open below: that helper's threshold is not the ordinary flick's.

* **2. Windbox loses its identity before hit resolution.** Lowering reduces it to `HitKnockback { flinchless: true }`, so `apply_body_hit_reaction` still knocks the body off its ledge, refunds `air_dodge_spent`, clears `post_recovery_helpless` and charges `hitstop_timer` before the flinchless branch. `ResolvedBodyHit` carries no reaction kind, so `impact_hitstop::is_a_connect()` sees `HitSource::Melee` and a windbox can freeze the whole match. Carry the reaction kind through resolution and define the windbox policy once; the ledge question needs an explicit rule rather than inherited injury handling.
  * ✔ `bd37e4dd` — `HitKnockback::reaction` carries `HitReaction`, the four injury facts are gated on it, the ledge push is kept by its own stated rule, and the match freeze follows from the gust earning no hitlag.

* **3. D192 canonicalizes simultaneous respawns with `Entity`.** `tick_pending_respawn` sorts a `Vec<Entity>` under a comment saying query order is not guaranteed and message order decides placement — but `Entity` is allocator identity. The clank work in the same window moved to `SimId` for exactly this reason. Sort on `MatchSeat` (or `SimId`), or remove the sort and correct the comment; a fake canonicalization is worse than none.
  * ◐ `df9e887ff` sorted on `SimId`. ⚠ THE REVIEW THEN REVISED ITS OWN PREMISE: it traced every `FighterRespawnDue` consumer and found each fighter placed at its OWN seat's position, so the operation is commutative and nothing needs canonical order today. The remaining work is to DELETE the sort and the claim rather than keep anticipatory ordering infrastructure. See the open row below.

* **4. The mount carve moved implementation but not rollback ownership.** `actor_monolith/rollback_registration.rs` still registers the seven `ambition_mount::` components and their `MapEntities`, while `runtime/rollback/mod.rs` says domains own their own declarations and composes six `register_rollback_state` calls with no mount one. Give `ambition_mount` its own `register_rollback_state`, preserve the wire names, and watch the registrar's owner string.
  * ✔ `1d5f2b708` — wire names unchanged (the contract still counts 375), only the owner label moved.

* **5. The saddle COG mixes mount-local and world vectors.** `ambition_mount/src/lib.rs` adds `cog_local` (local) to `frame.to_world(rider_offset - cog_local)` (world). Default gravity hides it; rotated gravity with unequal masses and a nonzero saddle offset pins the COG term to screen axes. Predates this window (5c9b11a, 2026-08-24). Either the invariant is just `mount_kin.pos + frame.to_world(rider_offset)`, or a true rigid pair must rotate BOTH bodies about one world COG.
  * ✔ `1d5f2b708` — the first reading, because a constraint with authority over ONE body cannot express a two-body rotation. ⚠ its descendant is open below: the new helper's doc claims facing-relative `+x` and `to_world` does not know facing.

* **6. Three-way clank applies rebound more than once per fighter.** Arbitration emits `AttacksClanked` per qualifying pair (AB, AC, BC) and `rebound_from_clanks` adds a full impulse per message. The new three-way arm cannot see it: all three bodies sit at `Vec2::ZERO`, where the rebound axis is deliberately zero. Decide the rule — summed pairwise or one bounded recoil per participant — before clanking is enabled (`clank_damage_window` is 0.0 in Smash today).
  * ✔ ONE BOUNDED RECOIL PER PARTICIPANT (`707871fcc`). The genre has a rebound, not a rebound COUNT, and this repo already fixed the same shape when a 2×2 volume overlap produced four rebounds for one clash. The DIRECTION is the sum of the pair axes — a fighter that clanked two opponents is pushed away from both — and the SPEED is not: being outnumbered is not a reason to fly faster. Accumulated in a `Vec` in message order, because a hash map's iteration is not deterministic. The new arm spreads the three fighters into a line; the existing one stands them all at `Vec2::ZERO` where the rebound axis is deliberately zero, so it could never have measured recoil.

* Hygiene, same window: trailing whitespace in this file and a new blank line at EOF in `game/ambition_app/tests/duel_arena.rs`.
  * ✔ both fixed (`df9e887ff`).

## Follow-up review of HEAD `bd37e4dd`, relayed by Jon 2026-08-26

Three of its "still open" items were already done between `bd37e4dd` and this
entry — D192's `Entity` ordering (pushed as `df9e887ff`), the mount rollback
ownership, and the mount rotated-gravity COG — so only the genuinely new
findings carry a ▢.

* **`HitReaction` was missing from `pending_player_hits_checksum`.** The field decides six victim outcomes and two staged hits differing only in Strike-vs-Windbox fingerprinted identically, contradicting the projection's own "everything that decides what the hit DOES participates". Not snapshot loss — the queue is clone-rewound — but the desync oracle was blind to the most consequential field the pulse carries.
  * ✔ Tagged, with a poison arm and an equality premise guard; `GGRS_ROLLBACK_SCHEMA_VERSION` 114 → 115 because a checksum projection change is a wire change.

* **B-reverse has a second, softer definition of "flick".** `lateral_flick_sign` accepts any deflection past `directional_deadzone` (0.5), while the ordinary attack flick needs `flick_threshold` (0.8) after re-arming under 0.35 — so 0.65, which the CPU's own `TILT_DEFLECTION` deliberately calls a TILT, is a B-reverse flick. And the window differs too: ordinary flicks admit `age_ticks <= flick_window_ticks` (four subsequent ticks), while special-turn spends one on the press tick and admits three. One authored knob, two temporal and two magnitude meanings. Either share one directional-edge semantic or give the special turn its own named threshold and window — but not the current middle state.
  * ✔ its own knobs: `special_turn_deflection` (0.5) and `special_turn_window_ticks` (4, counting SUBSEQUENT ticks — the acceptance tick is free, so the count now means what the ordinary flick's `age_ticks <= n` means). `lateral_flick_sign` is `special_turn_stick_sign`: it never read a flick. ⭐ THE SOFT READING IS CORRECT — a B-reverse in this genre is a stick tap, not a smash input — so the behaviour is unchanged and what is new is that the split is AUTHORED. A contrast arm pins the table: 0.65 is an edge to the special turn and not a flick to the attack gesture; 0.85 is both.

* **`VolumeReaction::Windbox` still permits authored damage.** The contract says "no damage", lowering publishes whatever `damage` is authored, and every fixture merely remembers to write zero. No shipped move authors a Windbox yet, so this is the moment to reject `Windbox + damage != 0` at preparation with a useful error rather than silently discarding an authored number. ⚠ the neighbouring question — whether a successful gust counts as a move connect for `on_hit` and OnHit cancels — is a separate decision and must not be settled by accident here.
  * ✔ `73a83a8b5` — `CatalogError::WindboxWithDamage` rejects it at preparation naming entity/move/window, with a zero-damage arm so the mechanic stays authorable. The `on_hit` question is deliberately still open.

* **Room vocabulary has the same unfinished rollback ownership the mount did.** `actor_monolith/rollback_registration.rs` — whose header promises it "names only state defined in this crate" — registers `ambition_platformer2d_world::rooms::{RoomSet, ActiveRoomMetadata, RoomMusicRequest}` and defines `room_set_checksum`. The world crate already owns `register_gate_portal_rollback_state`, so the machinery exists.
  * ✔ `73a83a8b5` — `rooms::register_rollback_state` beside the gate-portal one; `room_set_checksum` moved with the type; stable names unchanged.

## Third review, of HEAD `f17872dc`, relayed by Jon 2026-08-26

Its priorities 3 and 4 — the windbox damage rejection and the room rollback move
— were already done at `73a83a8b5` before this arrived, and its 8 (stale
receipts) is done above.

* **The staged-hit checksum does not identify WHO is hitting WHOM.** The windbox omission was one instance of a larger hole: `pending_player_hits_checksum` reduces `attacker` to `is_some()` and `HitTarget::Body(entity)` to the tag `1`, so "A hits X" and "B hits Y" fingerprint identically while every other field is equal. `HitTarget::Body` is documented as the complete victim-routing answer and production uses it for projectiles, contact damage, empowerment, enemy hits and blast-zone events. ⛔ the fix is NOT to hash `Entity` — that is allocator identity and trades one blindness for a false alarm. The queue should carry stable semantic identity across the frame boundary, which would also retire its `MapEntities`.
  * ✔ `9352d3255` — the queue's row is a `StagedPlayerHit` carrying both bodies' stable ids, resolved once at staging where a `World` is in hand, because a registered checksum is a bare `fn(&T) -> u64` and can never look an entity up. ⛔ `Entity` is deliberately NOT hashed. Schema 116 → 117.

* **The new `saddle_world_offset` promises facing-relative `+x` and cannot deliver it.** Its doc says the offset is `+x toward the mount's facing side`, but `AccelerationFrame::to_world` knows gravity-relative side/down and nothing about facing — so a saddle authored at `x = +5` stays on the same gravity-relative side when the mount turns around. Invisible today because production saddles author `x = 0`, and the new rotated-gravity arm never flips facing. Either mirror `x` by facing, or stop claiming the axis is facing-relative. ⇒ prefer mirroring, to match the rest of the authored character-local geometry convention.
  * ✔ mirrored, and the decision is settled by the constraint itself: it already hands the rider `mount_kin.facing`, so an offset that did not mirror would put a rider authored on one shoulder onto the other the moment the mount turned. A neutral facing keeps the authored side — `signum(0.0)` would have collapsed the offset.

* **Delete the respawn sort rather than elaborate it.** Revised premise, and the review revised its own: every `FighterRespawnDue` consumer places a fighter at its own seat's position, so the operation is commutative and no canonical order is needed. `df9e887ff` replaced a fake canonicalization with a real one nothing consumes. Remove the sort and the comment claiming message order is meaningful; a future order-sensitive ruleset should name its own key.
  * ✔ removed. ⛔⛔ AND THE TEST WAS PASSING BY LUCK: with two bodies the query returns them in allocation order anyway, so the ordering assertion survived the sort's removal. It now measures the SET — both fighters return on one tick, unchanged by spawn order — which is the property that would actually break.

## Fourth review, of HEAD `e17fe564`, relayed by Jon 2026-08-26

Its HIGH finding is on D201 (another agent's work in the same window), not on
the facade/review lane. Its two LOW items were both mine and are fixed.

* **`OutOfPlay` has a stronger documented meaning than an implemented one, and D201 is its second customer.** The type promises "no control, no hurtbox, no world interaction"; D201 delivered the movement/input half for actor bodies and not the combat half. VICTIM SIDE: the melee victim query in `hitbox/mod.rs` carries no `Without<OutOfPlay>` and gates only on `is_corpse()` — and the stock spend calls `health.reset()` first, so a waiting fighter is specifically NOT a corpse. The actor damage road reads `CombatStanding::of(disposition, active_combatant)`, and prepared match construction sets every fighter `ActorDisposition::Hostile`, so dropping `ActiveCombatant` does not make it undamageable: it can accumulate percent during the interlude and return from its stock at 8% or 12% rather than 0. ATTACKER SIDE: the spend does not cancel `MovePlayback`, and `advance_move_playback` consults neither `OutOfPlay` nor `ActiveCombatant` — so a fighter KO'd mid-swing keeps advancing its move clock, opening hit volumes and firing authored events for the whole death beat. Smash does call `cancel_move_playback`, but inside `place_respawning_fighters`, one lifecycle boundary too late.
  * ✔ both halves. ⭐ THE VICTIM HALF NEEDED NO NEW LIST: `body_is_corpse`'s own doc already promised *"extend THIS function for any future intangibility cause and every combat boundary inherits it at once"* — so it became `body_is_untouchable(health, out_of_play)`, and changing the SIGNATURE made all thirteen boundaries a compile error until each answered the new question. Melee, boss, actor-damage, footstool, capture, empowerment, target volumes, interact, affordances, sentry, vortex and possession, with no hand-kept list to go stale. The attacker half cancels `MovePlayback` through the canonical `cancel_move_playback` where the episode OPENS rather than where the fighter returns. Both poison-verified; ⚠ the worktree3 agent that owns D201 was clean and idle when I took this.

* The stale `RESOURCE_WAIVED` prose for `RespawnInterval` described D192's ticks-and-`PendingRespawn` architecture after D201 moved the window into `DeathInterlude`.
  * ✔ corrected — the waiver is keyed by type name, so nothing failed; a stale reason hands the next reviewer an architecture that is gone.

* A blank line at EOF in `actor_monolith/rollback_registration.rs`, left by my own removal of `room_set_checksum`.
  * ✔ fixed.

* D202 and D206 were both examined and accepted by the review; D206's one `MatchSeat` check has a crisp semantic reason, with the caveat that a SECOND such check would belong in an effective match-body policy rather than another `if MatchSeat`.
  * ✔ no action.

## Fifth review, of HEAD `38df5cc78`, relayed by Jon 2026-08-26

Three substantive items, all mine, all closed.

* **`OutOfPlay` did not generically end a body's move.** I put the teardown inside `spend_fighter_stocks`, which fixed Smash and left the other real customer broken: `session::death::open_death_interlude` — how Mary-O and every non-stock ruleset die — inserted `OutOfPlay` and left the move clock running, so a body could die mid-swing and go on opening hit windows and firing authored events.
  * ✔ `end_moves_for_bodies_out_of_play`, an INVARIANT re-established each tick at the head of `CombatSet::Trigger`, not a transition each death road remembers. ⛔ deliberately NOT an `Add` observer: that fires when GGRS re-inserts a component during a snapshot restore, so it would tear down moves on a rollback that merely replayed a body into the same state. The stocks seam keeps only the ELIMINATED case, because elimination opens no death window and so never becomes out-of-play.

* **CPUs still targeted fighters waiting to respawn — a live gameplay defect.** `select_actor_targets` filtered on `hp.current() > 0` under a comment calling health "the one uniform liveness gate", and D201 made that false: the stock spend calls `health.reset()`, so a body lying untouchable at the blast line reads FULL HEALTH for the whole interval. A surviving CPU went on selecting, chasing and aiming at it; the hit filters stop it hurting that body and say nothing about where it walks.
  * ✔ both sides gated through `body_is_untouchable` — an out-of-play body is not a candidate, and an out-of-play actor does not acquire either (it was refreshing its own `ActorTarget` while dead and coming back holding the lock). The stale comment is corrected at the source.

* **The safe-position extraction stopped one step short** — types, codecs and consumers moved to `shared_tangle`, and the two rollback declarations stayed in the monolith.
  * ✔ moved. ⛔⛔ THIS IS THE THIRD TIME TODAY and the second time I caused it: mount and rooms were the same shape, I wrote the lesson down for both, and then repeated it an hour later. Moving a type is FOUR things — the type, its codec, its consumers, and its ROLLBACK DECLARATION — and the declaration is the one that goes on compiling where it is.

## Jon, 2026-08-27: "in the second match I select characters press start, but it just brings me back to the character screen"

* ✔ **FIXED, and the reset's guard was a PROXY that goes false on the second visit.** `reset_select_frontend_on_arrival` gated on the select UI root not existing yet, standing in for *"I have not reset for this visit"*. On the second arrival the first visit's root is still there — measured `ui_roots=1` — so the body never ran and `MatchParticipantRoster`, `StartRequested` and the previous decision all stood. `start_the_battle_when_asked` refuses while a roster stands (`!on_select || roster.is_some()`), so the press did nothing. ⭐ THE ROUTER ALREADY NAMES THE ARRIVAL: `ShellActivationId`, the same id the world-event log prints, so remembering the one the reset ran for says what the entity count was guessing at. Poison-verified: reverting to a once-ever guard puts the regression red.

* ⭐⭐ **AND THE LOG'S SHAPE POINTED AT A DIFFERENT DEFECT, WHICH WAS ALSO REAL.** `session-start scope=N` / `room-loaded` / `session-end scope=N` one frame apart reads as "the stage opened and something closed it", and a retired session's `ActiveMatch` does outlive its session by at least a frame. `StocksMatchSettled` names the match it decided and `verdict()` compares instances — but with BOTH sides of that comparison naming the retired match it agrees, so the previous match's `NoContest` applies to the match that replaced it. Only the SESSION knows which of the two is current, so `return_to_the_select_screen_when_the_match_ends` now refuses to read an `ActiveMatch` whose session is not `ActiveSessionScope::current()`. ⚠ The old road hid this: the 4.5s countdown and `fully_confirmed()` were always beaten by the new activation. The immediate exit Jon asked for on `NoContest` runs on the first frame the router says "on stage" — exactly the frame the leftover is still there.

* ⛔ **THE FIXTURE FAILED TWICE BEFORE THE SUBJECT RAN, and both premises were worth asserting.** Asking to stop on the first gameplay frame throws the request away (`abandon_the_match_when_the_shell_asks` returns without acting when no `ActiveMatch` is seated, and the message is consumed either way); and deciding a cast on the frame the route flips is wiped by the arrival reset landing behind it. A player sees the fresh lobby before picking, and so does the test now.

## Sixth review (GPT 5.6, partial — died mid-Finding-4), relayed by Jon 2026-08-27

⚠ TWO REVIEWS IN ONE RELAY: an Up-B pass at HEAD `bc942727`, and a non-Up-B pass
at `e29bc316` covering the `overnight-agent3-moveset-and-specials` merge. The
second one was cut off partway through Finding 4. Items below are transcribed as
open unless marked; ⛔ several may already be done — grep before working one.

### Already closed by the session that received the review

* ▣ **Finding 4 (non-Up-B) / D252 — the back air is dead content for the whole
  cast.** GPT's diagnosis is the same one the queue row records and the same one
  the fix acted on: `abilities.rs:228` says an airborne non-flyer may not turn,
  `kernel.rs:427` applies `facing_intent` unconditionally, so the body turns to
  face the back input and `attack_dir_from_axis` folds the reversal away. ✔ Fixed
  at the PRODUCER (`brain/player.rs`), not in the shared kernel — the human
  translator now steers facing only when the body may actually turn
  (`actor_on_ground || actor_aerial`, the flyer carve-out matching `fly_enabled`).
  Acceptance is the row's own instrument: `moveset_takes` went from
  `MISMATCH: drove air_back but the engine played {"air_forward"}` to
  `moves={"air_back"}`, 0 mismatches across all 19 takes.

### Open — Up-B pass (HEAD `bc942727`)

* ▣ **FIXED — being launched off the shark can consume the knockback and then erase it.** The open half WAS only the proof: `a_mounted_launch_carries_the_same_travel_as_an_unmounted_one` asks for equivalence rather than a magnitude and both roads produce `(-1884.0382, -1884.0382)`; reverting the deferral releases the mounted arm at exactly `(0, 0)`. `65b89da85`. ORIGINAL: The rider runs the full kernel; `step_motion` takes the launch, then `sync_riders_to_mounts` pins the rider and zeroes velocity on the same tick, so the hit ends the RELATIONSHIP but the knockback is gone. ⚠ PARTLY ADDRESSED ALREADY by `pose_owned_externally` + `LaunchTravel::Deferred` — grep `pending_launch_state` before working this; the open half may be only the proof. The poison GPT asks for: mounted vs unmounted Pirate take the same strong hit, and the released body must actually TRAVEL.
* ▣ **FIXED — the 36-HP shark is one-shot by George.** The census reads the whole selectable cast across both authoring crates and names the fighter; 36 → 40. `ed4e32e4b`. ORIGINAL: `SUMMON_SHARK_HEALTH = 36`; George's forward smash is 21 × 1.7 full charge = `(21*1.7).round()` = 36. The test proving "no single hit kills it" scans only `pirate_admiral_moveset()`, not the selectable Smash roster. Build the census from the resolved repertoire and require `>` not `>=`. Also `shark_ride_probe` calls the Admiral's 29 the "worst single connection in the game", which is false.
* ▣ **FIXED — the flinch half of `a_flinch_leaves_the_admiral_aboard_and_a_launch_takes_him_off` never hits the Pirate.** It lands a real low-knockback hit and asserts the hit LANDED before asserting he stayed aboard. ⚠ AND MY EARLIER NOTE IN THAT FILE WAS WRONG: it recorded that a weak volume could not reach a mounted rider; built exactly like the launch box and differing only in damage/knockback, it lands first try. ✔ AND THE RECOVERY-REFUND HALF IS ASSERTED NOW. It could not be until `533bd9a05`: the grounded landing-class refresh handed the charge back one frame after `call_the_shark` spent it, for every fighter, so the admiral boarded holding a recovery he had paid for. ORIGINAL: It waits ~20 frames and asserts `RidingOn`, so it proves only that an undisturbed rider stays mounted. Give it a real low-knockback hit and assert the hit landed, the recovery charge was restored, and `RidingOn` remains.
* ▣ **FIXED — finish `PoseOwnedExternally`'s semantics.** Ownership is stated axis by axis and pinned by an app-level arm; the buffered-evade spend it found is refused. `fc04a8990`. ORIGINAL: Today it means "suppress voluntary axes before running the normal kernel"; the name implies "another authority owns this body's pose episode". Define what a constrained body still owes the kernel (gravity, collision, ground transitions, maneuver state) rather than running everything with a zero stick.
* ▣ **FIXED — D250, CPU Pirate cannot reason about vehicle recovery.** Recovery admits route KINDS now: burst, sustained authority, teleport. `1a1daddc9`, and both hacks stayed rejected. ORIGINAL: The planner understands a `RecoveryLift` (one-shot displacement); `call_the_shark` authors none. Recovery should admit multiple route kinds — burst displacement, sustained movement authority, teleport. ⛔ GPT agrees with rejecting both hacks (fake impulse; special-casing the id).
* ▣ Smaller, BOTH FIXED: `tick_departures`' doc names `WorldPrepSet::BeforeIntegrate`, and `shark_ride_probe` waits on the open-match condition rather than a fixed 240 frames.

### Open — non-Up-B pass (`e29bc316`), and GPT does NOT consider D254 complete

* ▣ **FIXED — brandishing a move weapon can duplicate a physically-held item.** The custody reconciler asks what the body has CUSTODY of, which during a brandish is what the brandish displaced. `72f004ca7`. ORIGINAL: `MoveBrandishedItem` stores only `move_id` + `previous: Option<String>` and overwrites the canonical `HeldItem`; `return_released_items()` then sees the body's held id no longer matches the `GroundItem` in `ItemCustody::Held` and returns the real object to `InWorld`. When the move ends the body reconstructs `HeldItem(A.spec)` from a string — logically holding A while A also lies on the floor. The brandish tests use a plain `HeldItem` and cannot see it. Fix: a temporary move weapon is an OVERLAY, not a replacement of the custody answer.
* ▣ **FIXED — stored neutral-B charge treats DEATH as an instruction to bank.** `MoveEnd::{Interrupted,LeftPlay}` is a required argument; the compiler found three call sites the grep missed. Also caught a rollback probe gap I had already pushed. `741a1f59b`. ORIGINAL: `cancel_move_playback` banks any unreleased storing charge, and death (`death_rules.rs`) and stock loss (`stocks.rs`) both call it — so a KO'd Projectile Polygon respawns with the charge banked, and an existing bank has no death/reset clear. The function's own comment argues no termination reason is needed; the stored-charge customer disproves it. Fix: an explicit end reason (store / release / interrupted / left play), not a character-specific death hook.
* ▣ **FIXED — D253's root cause is NOT flight posture.** Behaviour `22361bab3`; the queue's flight-posture diagnosis is corrected in `4a551ccc5`; five direct arms at the seam landed with `only_a_press_that_resolves_to_the_bubble_raises_the_guard`. ORIGINAL: `sustain_bubble_shield` (`avatar/starting_character.rs:983-1027`) reads the RAW `special_pressed` in `PlayerInputSet::ControlGate` and sets `shield_held` because Robot's body kit names `bubble_shield` — before anything resolves WHICH directional special was meant. Grounded shield + direction is an evade; airborne is an air dodge; Combat then refuses the special from the state the compatibility layer just created. One shared authority error, not five broken moves. ⛔ Update D253's queue diagnosis when this is fixed — the "flight posture" suspicion must not stay as the leading explanation.
* ◐ Also listed, unreviewed in detail — THREE OF SIX ARE DONE: the held bomb now blasts at its holder's hand (`69a0918f5`), the Admiral's gun-sword has the discharge its comments promised (`216316bef`), and aim assist no longer bends toward an `OutOfPlay` fighter (same push). The ponytail hits on BOTH legs (`f4910156a`) and the guard puts a stored charge away (`7656d8124`). ALL SIX ARE DONE — the last two are the item-interaction population (`a901cdc2f`) and hard impact against a BODY (`25c93bd89`). Tracked as D255 R8/R10/R11 in `triage/gpt-review-2026-08-27-remaining.md`.

### The rest of the sixth review, at HEAD `e8fbaa0d6` — twelve more, none overlapping the above

⚠ GPT deliberately EXCLUDED everything the dead pass had already named, so this
list is additive. Its own priority order: stale `PoseOwnedExternally` after mount
death, then the lost bomb-impact velocity, then the Class-B teleport omission,
then the portal/boomerang vector mismatch.

* ▣ **FIXED — the canonical mount relation is not dissolved or restored ATOMICALLY.** `RideConstraints` names the pair once, `rider_of` composes it, and both transitions use it; `spawn_pair` builds the real relation now. Both arms poison-verified independently. `bcbe97abf`. ORIGINAL: `614fb04` made `rider_of(mount)` mean `RidingOn + Mounted + PoseOwnedExternally`, and the requested-dismount road removes all three. `enforce_mount_rider_link` still runs the older two-component machine: mount death removes only `Mounted` (`lib.rs:1082-1143`), restoring gravity and autonomous control while LEAVING `PoseOwnedExternally` — so the rider is declared autonomous while the kernel still believes another authority owns its pose. The inverse is wrong too: same-room revive (`lib.rs:1057-1080`) reinstalls `Mounted`, the cached brain and zero gravity but NOT `PoseOwnedExternally`. ⛔ `mount_pair_tests.rs` builds `Mounted + RidingOn` by hand instead of through `rider_of`, so its death/revive coverage never starts from the relation the runtime installs.
* ▣ **FIXED — a hard bomb collision can fail to detonate because impact speed is sampled ONE TICK EARLY.** `SettledItem::impact_speed` is published by the step that zeroes the velocity; the bomb's `last_speed` is deleted. `32bb34e0b`. ORIGINAL: `bomb.rs:139-164` gates on `bomb.last_speed >= impact_speed`, but `ground_item_physics` zeroes `item.vel` and stamps `SettledItem` in the same tick the collision resolves, and bomb processing runs after in `Combat::Settle`. A bomb thrown hard at a near wall collides on its FIRST free tick while `last_speed` still holds the zero it had while held. Second form: a falling bomb crosses the threshold from gravity DURING the collision tick. ⇒ collision resolution should publish the PRE-RESOLUTION impact speed rather than the bomb reconstructing it from last tick.
* ▣ **FIXED — `moveset_takes` attributes the CPU opponent's hitboxes and projectiles to the move under inspection.** Each records `subject_owned`; the maxima count only the subject. ⚠ MEASURED: the numbers did NOT move — the contamination was reachable, not reached, in the run sampled. `e2deb1808`. ORIGINAL: The match is `smash_roster([character, character])`, so seat 1 is a live CPU; the sampler collects EVERY `ProjectileGameplay` and every live `Hitbox` in the world, reads `hitbox.owner` only for its anchor and drops it, and never queries `ProjectileOwner`. So a hitless movement special can show a hitbox and `max_live_projectiles` can exceed what the move fires. ⛔ Do NOT make the opponent inert — record provenance and derive the subject's statistics from the subject's output.
* ▣ **FIXED — the inspector still cannot represent a RANGED attack.** The export names the fire frame and the shot's damage/speed, endlag comes from the shot, and the take viewer draws projectiles with their owner. Admiral side-B: `no startup / 0 damage / 0.52 endlag` → `fire_at 0.20 / 8 dmg @ 620 / endlag 0.32`. `bd6452669`, `52dc6d92a`. ORIGINAL: `moveset_export.rs:105-184` derives startup/active/endlag/damage/knockback/reach entirely from `WindowTag::Active` MELEE volumes and exports only a boolean `fires_projectile`. Admiral side-B therefore reports no startup, zero damage, zero knockback and the whole move as endlag; Projectile Polygon's charged neutral-B is the same shape. Separately `app.js:601-668` draws bodies and hitboxes only — there is no projectile loop, though the take records `frame.projectiles` and the docs promise them.
* ▣ **FIXED — `sum_damage` does not implement the engine's PULSE semantics.** Carry derived by pulse; 3 real corrections (`smash_forward` 32→21), and the 8 remaining multihits verified genuine. `8d21ae538`. ORIGINAL: The exporter flattens and sums every authored volume; the runtime gives overlapping siblings a ranking so ONE connection wins, and a continuous Active pulse shares one per-victim ledger (`moveset/mod.rs:1190-1223`). A 21 sweetspot + 11 sourspot is not a 32-damage carry — they are alternatives. Derive carry BY PULSE: siblings are alternatives, genuinely separated pulses accumulate.
* ▣ **FIXED — missing compare values become ZERO and contaminate roster medians and outlier flags.** Absence filtered before conversion, in both the median and the cell. `5073e452b`. ORIGINAL: `app.js:463-468` calls `Number(get(r))` before the `Number.isFinite` filter, and `Number(null)` is `0`. A row shown as `—` still moves the median, gets a hot/cold class and a zero-width bar. Finding 4 makes it worse because projectile-only moves currently have null startup.
* ▣ **FIXED — the authored Smash teleport does a Class-B body remap WITHOUT entering the Class-B ledger.** Records `ScriptedTeleport` at the `transit_body`, and the `ActorActionMessage` road has its first test. `931facc13`. ORIGINAL: `teleport.rs:181-186` calls `transit_body` directly; `blink.rs:170-183` does the analogous transit and immediately records `ClassBRemap::ScriptedTeleport`. The collision oracle uses that ledger both to exempt legitimate discontinuity from clipping detection and to catch two Class-B authorities remapping one body in a frame — so a valid teleport looks like unexplained discontinuity and a same-frame conflict is invisible.
* ▣ **FIXED — teleport aiming is gravity-aware but its LEDGE ASSIST hardcodes normal-world top faces.** Rewritten on the existing gravity-relative vocabulary; one fixture rotated through four frames, both terms each. `fcc37a439`. ORIGINAL: `apply_authored_teleports` derives `gravity_dir` from `ResolvedMotionFrame`, but `ledge_assisted_arrival` defines a ledge as `Block::top()`, compares world `y` and lands at `block.aabb.top() - half.y`. Under flipped or sideways gravity the teleport aims in the right frame while the assist searches the wrong face. Existing tests are all `+Y`; mirror them through `-Y`, `+X`, `-X`.
* ▣ **FIXED — the ponytail boomerang ends ~79px BEHIND its launch point.** The round trip is analytic (`2·out_s`); the `+0.15` is deleted. −79.2px → −1.4px. `1ba3a5fa0`. ORIGINAL: `ProjectileFlight::boomerang(0.34)` sets lifetime `out_s * 2 + 0.15` intending "expires just past the hand"; at the real 60 Hz integration it first re-crosses the launch point at ~0.667s and stays alive to ~0.83s, by then ~79.2px behind and moving ~603 px/s backward. Lifetime is a weak proxy for "returned" — give it a real catch condition (crossing the launch plane on the return leg, or a catch radius around the stored origin).
* ▣ **FIXED — portal transit rotates a boomerang's VELOCITY and leaves its world-space return ACCELERATION in the old frame.** A projectile post-transit adapter maps `accel` through the same portal map. ⚠ its first version PASSED ITS POISON twice over — see the commit. `f7be763f9`. ORIGINAL: Every projectile opts into portals with `carry_velocity: true`; `ProjectileGameplay.accel` is documented as a WORLD acceleration and the ponytail is its only nonzero user. Transit maps `BodyKinematics.vel` by the entry/exit normals and does not touch `accel`, so a ponytail exits travelling correctly while its "come home" pull still points the pre-portal way. ⛔ The generic portal core need not know about projectiles — add a projectile post-transit adapter. Current tests instantiate `accel: ZERO` and cannot see it.
* ▣ **FIXED — Author's teleport requests `player.blink` TWICE on the teleport frame.** The executor is the one authority; the authored timeline event is gone (`947b97b`) and the count is now asserted through the real move (`78d7d0240`). ORIGINAL: `author_moveset.rs:80-81` authors a move-timeline SFX event for it and `teleport.rs:201-207` independently emits `PLAYER_BLINK`. Two separate roads, one frame, same cue. Pick one authority.
* ▣ **FIXED — the inspector HTTP server permits `/data/../` traversal.** Resolves and asserts containment; escapes answer as a miss. Regression in `scripts/tests/`. `5073e452b`. ORIGINAL: `server.py:52-62` returns `DATA / parsed[len("/data/"):]` without normalising or checking containment; GPT confirmed it reaches a repo file once `data/` exists. Default host is `127.0.0.1`, but `--host` is deliberately supported, so binding it to an interface makes it file disclosure.

⚠ GPT's closing note, and it is the one that matters for how the inspector is USED: opponent-output contamination + ranged moves reported as zero-damage melee + sweet/sour summed as multihit + nulls entering comparison statistics together mean several of its apparently quantitative conclusions can be wrong for STRUCTURAL reasons rather than balance ones. Fix those before tuning against its numbers.


### The boomerang's FIRST-HIT expiry — scoped 2026-08-27, not yet built

⛔ THE REMAINING HALF, and it is a MECHANIC, not a patch. A projectile currently
ends by DESPAWNING on its first body hit (three sites in
`projectile/systems.rs`), and that despawn IS the hit-once rule — there is no
per-victim ledger on a projectile anywhere. So "the tail survives its first
victim" cannot be expressed by deleting a despawn: an overlapping shot with no
guard hits every tick, which is a machine-gun rather than a boomerang.

⭐ THE GENRE ANSWER IS A LEG, NOT A VICTIM LIST. Smash's returning throws connect
once ON THE WAY OUT and once ON THE WAY BACK. The leg is already derivable from
state the shot carries — `accel` is `-v0 / out_s`, so it encodes the launch
direction, and `vel · accel` says which leg the tail is on — so the rule needs
ONE bit ("has connected on this leg") rather than a list.

⛔ THAT BIT IS ROLLBACK STATE. `ProjectileGameplay` is snapshot-carried, so the
field is a wire-format change: the schema baseline moves and the presence probe
must see the new value (the same obligation `SettledItem::impact_speed` incurred
in `741a1f59b`, which the exit oracle caught).

⚠ AND IT IS WHY THE RETURN-LEG FIX IS CURRENTLY UNOBSERVABLE IN PLAY. `1ba3a5fa0`
made the tail come home to the hand instead of 79px past it; until the first hit
stops ending the flight, most throws never reach the return leg at all.

### The workspace suite has been RED, and nobody could see which tests (2026-08-28)

⛔⛔ **`workspace (default features)` HAS BEEN FAILING SINCE AT LEAST HEAD
`a945c1de5`, AND I MIS-ATTRIBUTED IT.** A sweep that morning reported the job red
and I read it as the flinch test that was red at the same time, without opening
the job's own output. It was not. Switching the runner to `cargo nextest`
printed the failures by name for the first time — 6458 tests, seven red. Each has
now been BISECTED against a worktree at `a945c1de5` rather than guessed at:

* ✔ **`ambition_demo_twintrack_app::twintrack_it` — five, PRE-EXISTING, FIXED
  2026-08-28. 24/24 pass.** The laboratory twin spends exactly ONE tick of her
  life as a seatless `Passive` NPC, because `adopt_the_laboratory_twin` QUEUES
  `DrivingParticipant` and the insert lands a flush later. A seatless `Passive` is
  what the engine calls an "undescribed-pool STROLLER", so she takes one stroll
  step worth -96 px/s and drag bleeds it over seven ticks into a permanent 6.16px
  offset — which every reading taken against *"the twin is at rest"* inherited.
  `restore_the_laboratory_twins_mark` puts her back on `Added<LaboratoryTwin>`.
  ⛔ **AND MY OWN EARLIER NOTE WAS THE LAST OBSTACLE.** It read *"ONE IMPULSE AT
  CONSTRUCTION, not a force and not a walk — the velocity only decays"*, and
  pointed at the causal instrument as the next step. It is a walk: a second body
  spawned from the same request 420px away accelerates -96, -194, -294, -398, -506
  and pins at the -540 cap, walking left forever. The decay was drag on the ONE
  step, not the shape of an impulse. The instrument that answered it was a
  twelve-line probe, not the causal recorder.
  ⚠ **and the first fix attempt sampled the wrong moment**: correcting her inside
  the adoption read `720.0` with zero velocity — before the step it needed to
  undo — and changed nothing, while printing a line that looked like success.
  <details><summary>the original measurement</summary> At `a945c1de5`, run alone, `each_seat_moves_its_own_body_and_leaves_the_others_alone`
  fails with the same numbers as today: the laboratory twin goes
  `715.9127 → 713.8359` while seat zero presses RIGHT. The four siblings
  (`both_observers_measure_the_light_pulse_at_the_invariant_speed` — *"the lab
  twin should be at rest, was 0.08836941"* against a 1e-4 premise —
  `with_nobody_in_the_second_seat_the_twin_stands_still_and_stays_watched`,
  `the_two_observers_disagree_about_the_pulses_direction_and_colour`,
  `two_observers_report_different_orderings_of_the_same_flash_pair`) share the
  shape: a body that should be still is drifting.
  * ⛔⛔ **AND THE ASSERTION'S OWN MESSAGE IS A MISDIAGNOSIS.** It says *"the two
    seats are sharing one control frame"* — but the twin moved 2.07px LEFT while
    the input pressed RIGHT. A shared frame would push it the other way. Whoever
    picks this up should not start from that sentence.
  * ⚠ One thing DID change and it is not what fails: the twin's resting `y` went
    `450` → `446.015` between that baseline and today, which the movement work
    merged from `specials-are-real-moves` explains. The failing assertion is on
    `x` and is identical either side.
  </details>
* ✔ **`ambition_demo_smash_app::…::holding_attack_walks_the_jab_string_into_the_rapid_jab`
  is NOT a regression.** It passes ALONE at HEAD (39/39) and at every bisect
  point between the merge and HEAD. It failed only inside the 6458-test run,
  where nextest has ~14 test processes live at once. A Bevy app test that is
  sensitive to CPU contention is worth knowing about; it is not a code defect,
  and treating it as one would have sent somebody hunting a change that does not
  exist.
* ✔ `ambition_sprite_sheet::fx::every_authored_effect_row_is_reachable_by_name`
  — fixed: 196 rows, because the trapdoor art arrived with the renderer that
  draws one.

⭐ THE LESSON IS THE MIS-ATTRIBUTION, not the reds. A job that prints one verdict
for six thousand tests is a job whose failures get explained by whatever else was
failing that morning, and I did exactly that. `./run_tests.sh` names them now.

### The Trap: she is visible under the stage, and the trapdoor is a puff (2026-08-28)

⭐⭐ **JON:** *"the actor's down-b has her go subterranian, but her body needs to
be masked so its not visible when she is under ground, right now her head is
poking out, and she is flashing likely due to the invulnerability state always
producing a flash, which is not what it should be doing. Often it does but it
shouldn't always be the case. There should be a trapdoor sprite she is replaced
with on the ground that can only move along a ground surface (i.e. it can't go
over a ledge). And she should be able to pop up at any time from it in a big
firework display that damages whoever is on top or above the trap door when she
emerges. I'm thinking it might be a good idea to rename the actor given its
conflation with a very core concept in the architecture. But we can do that in a
different pass."*

* ✔ **VISIBLE + FLASHING — ONE CAUSE, FIXED 2026-08-28.** `update_body_mode`
  resolved Standing-or-Crouching from the stick and wrote it UNCONDITIONALLY, so
  `BodyMode::Submerged` was deleted on the tick AFTER the trapdoor set it.
  `Climbing` and `MorphBall` survived only because they have arms of their own in
  that function; the mode the trapdoor added had none. Every symptom followed:
  she was never hidden (so `sync_submerged_visibility` had nothing to hide), she
  kept gravity and geometry, and the move's `Invuln` window blinked her for a
  second while she stood on the boards she had just dropped through. The blink
  itself is innocent — `overlay_look` already returns zero intensity for a hidden
  source, so a body that is properly absent does not flash. Photographed either
  side with `capture_scene pirate_cove player --character actor --press
  hold:down,g,release:down --frames 30 --stride 3`.
* ▢ **A TRAPDOOR SPRITE SHE IS REPLACED WITH.** Today the door is a 0.42s
  `trapdoor_boards` puff at each end and nothing in between; while she is under
  there is no object on the stage at all.
* ▢ **SURFACE-LOCKED TRAVEL.** `integrate_submerged_clusters` steers her
  horizontally with no geometry whatsoever, so she passes under a ledge and out
  past the end of the stage. Jon: the door *"can't go over a ledge"*.
* ▢ **POP UP AT ANY TIME, WITH A FIREWORK THAT HITS ABOVE THE DOOR.** Surfacing
  is fixed at `SURFACE_AT_S` and is hitless.
* ▢ **RENAME "the actor"** — Jon's own note that it collides with the engine's
  actor concept. Explicitly deferred to its own pass.
