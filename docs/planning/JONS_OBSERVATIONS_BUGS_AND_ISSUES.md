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

* Sanic hitting the spikes should not be an insta kill. It should hurt him and knock out his rings. Sanic should only die after getting hit with 0 rings (or crushed, but there is no way to do that in the demo yet). This needs to be a fairly faithful reimplementation of sonic physics and mechanics.

* The goblins in the goblin encounter don't have sprites anymore. They have magenta boxes. I think "Goblin" was never a proper enemy multi-instance character.

* Sanic still says "fly" instead of "super transform" or "untransform" which is what that button maps to. 

* Sanic should not pick up anything that turns him into super sanic in the level. 

* Super sanics spikes are clipped by the sprite renderer. This might need a
  structural fix. We should not be able to clip sprite artwork so easily.

* The current player V3 collision / hurt box  is larger than the player sprite. It needs to be slightly inset from the visible parts of the player. It should be under the main head, and well within the player arms. The player hitbox needs to be very forgiving to the player.

  ▢ Not started; the sprite and box numbers were never converted to a common unit, and this may be the player-side case of the quad-from-bbox decision below.

* A sword respects an authored hurtbox and a bolt never has — `step_projectiles` tests the coarse `CenteredAabb` while melee consults `DamageableVolumes`. [agent-found]

  ▢ Found 2026-08-08 and deliberately not fixed, because it changes how bolts connect and the feel call is yours.

* In smash if you throw out an attack you hurt yourself.

* In smash there does not seem to be any knockback.

* In mary-o secret blocks or invisible blocks (or question mark blocks which currently work correctly) need to change their tile sprite to spent blocks. A brick block with a quasar in 1-1 just keeps its brick texture. That needs to be fixed.

* In mary-o she can only have 1 fireball out a time. We should allow her to have 2 out a time.

* In mary-o 1-2 the flagpole doesn't have a flag. 

* In mary-o when you restart the level all item blocks and enemies and anything else that is part of the stage should reset. Currently some blocks from the last run remain spent

* In mary-o we need an SFX for when you collect coins

* In mary-o we need need a block that contains coins. (i.e. the multi-coin block where num-coins=1 is the default instance of that block). It just visually pops out a coin when you jump up into it. It's not a real coin entity, just a vfx and your coin count directly goes up by 1. When the counter goes to zero the brick becomes spent until reset.

* In mary-o small mary-o should not be able to headbutt bricks to break them. Only tall or fire should be able to break bricks with the headbutt.

* In mary-o when you die the level doesn't restart you just stay right where you were. When you die you should restart the level with 1 less life. For now let's allow lives to go negative and the user to play forever, so no game over screen yet.

* Spent blocks in 1-2 don't look spent. There are is also no tile texture in 1-2.

* JON (2026-08-08 STILL OPEN) The snake and AI slop are still way too big visually, and the sprite might not match the box for the snake.

  ◐ Their collision bodies are the right size in world units now (snake 1.00x Mary-O's width, slop 1.09x), and what is left is that the drawn quad is 2.46x the body inside it.
  * ⊙ Decide the quad-from-bbox route — it also fixes the Hall's size spread and deletes `collision_scale` for any sheet that authors a body.
  * Sizing the quad from the bbox without also cropping the drawn region was tried and reverted: it stretches the art badly.
  * It needs four coupled sites, not the three the design doc names — there are two render-size publishers, and fixing one leaves both of the characters you complained about untouched.

* Low priority: For the web build we can't use kaledioscope because lunex doesn't support wasm

* In 1-2 jumping into the invisible brick from below doesn't seem to trigger it.

* The pirates in the cover are horribly miss-sized. The heavies need to get a little smaller (this should probably be something done in data by the sprite renderer, not in code) and the other pirates need to probably scale up 2x, They are as tall as the player robot who is supposed to be chibi


* The pirates in the pirate sky no longer ride their sharks. 

* In the sky enemy the instance of iron marry doesn't use her swordgun, she shoots fireballs, which is not something her character should be able to do. I suppose we do need a distinction between unique characters and re spawning archetype characters.

* Changing rooms flashes magenta squares for a brief moment. We need to have cleaner transitions between rooms than that.

* After I fought in the pirate sky, and enemies died and dropped their swords I walked into the ninja dojo and there was a laser sword gun just existing there. I was able to fly to it and pick it up and use it. So props are not being despawned correctly - or made intangible and queued for removal - when you leave a room. I suppose there is an interesting question because it means we have to answer the question: in the ambition game, what should happen if you leave an item somewhere? Should it despawn? When? If you come back should it still be there? For the skyrim aspect of the game I think sometimes we do need items to remember where they were and if they were moved, but maybe we defer that and just have items be scoped to rooms. 


* When I have the laser sword in ambition and I use it, I incorrectly still use my normal jab attack. Holding an item should reroute normal attack actions to the item action, which might be like throw for bombs or fire for the gun sword. Some items may do different things depending on if your attack is a directioned tilt or airial or neutral, but the default for the gun sword is they all route to the one action the item has: shoot (I guess direction does change which way it shoots).

* In the ambition game, when I move from one room to another that is separated in LDTK, the camera moved as if there is a pan that should be happening. The camera room transition pans are just wrong.


* The main character shield sprite has the bubble in the wrong place, just kinda to the upper left. 

* In smash, choosing robot v3, if you do your attack (among other issues with smash combat right now) the VFX happens in the top left corner, not in the authored area for the character.  

* NOTE: developing out the smash combat system and the ambition combat system should be very similar and feed each other, because I want the combat in ambition to feel a little smash like. I want kockback to increase depending on how damaged you are. The ambition game will have health, and not percent, so there will be a limit, and maybe some enemy characters won't have this property, but the main character will. The knockback is what will make this game fun.

* We need an animation for a main character for when they are knocked down. We need an animation (or at least architecture slots) for a slow getup, a tech, and a getup attack. All smash characters will need this too.


* in the title menu FPS is 60 FPS, whereas ambition itself gets 140 FPS, and I don't know if the title 60FPS is intentional


* When you challenge PCA in the C4 symmetry room we should change the music to a smash track.


* I want to implement a camera mode for gravity where the camera just follows the player's reference frame. We should be careful so we use player-reference frame inputs in this mode. It doesn't need special gravity affordances. 


* Holding up for 2 seconds should be an alternative way of entering a door or interacting with an object.
