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
