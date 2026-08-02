# Agents should only edit this file to mark something as potentially done. Jon
# will remove it if it is actually done, or mark it not actually solved if an
# attempt doesn't work.


* In mary-o we need a "growing" animation when she grows or transforms. In single player this might request that time around the transforming character slows down as an effect, but in a multi-player setting the time slow needs to be agreed upon by all players, as we (should have) codified in the very early stages of development. [potentially done — verified in the code 2026-07-28, not by me implementing it: `crates/ambition_platformer2d_actor_monolith/src/features/transform_beat.rs` is the ENGINE beat, and `sync_grown_form` requests it on a tier step-up. Two demos asking for the same thing was read as an engine capability rather than two demo hacks, so what each game owns is only the DECISION that a transformation happened plus its numbers (`TransformBeatPolicy`: duration, held pose, clock scale, untouchable). **Your multiplayer clause is structural, not promised:** the beat never writes `ClockState::time_scale` — it writes a `ClockScaleRequest`, and `apply_clock_scale_requests` consults the active `RegimePolicy` before granting, so a regime that cannot afford one participant bending everyone's clock DENIES it. A demo that dilated time by writing the scale would be correct solo and silently wrong the moment a second participant existed. It ticks on WALL time, because a beat that dilates the clock and then measures itself with the dilated clock stretches itself by its own effect. Five tests, including `the_dilation_is_a_request_not_a_write`. ⚠ Mary-O's held pose is the DEATH row — the closest thing her sheets have to a non-locomotion held frame. A dedicated `grow` row is a generator change and is still open.]

ABOVE IS NOT DONE: the growing, and transform into fire mode is nearly instant if it even exists. The transform from tall to fire should probably use a shader or perhaps flash between the two sprites to make an effect. I know there are explicit grow and transform animations for maryo in the sprite sheets now, so its a matter of using them.


* Mary-o Needs a transform SFX. [WIRED 2026-07-24: `sync_grown_form` now emits a `mary_o.transform` power-up chime (`SfxMessage::Play{id}`) when she steps UP a power tier (small→grown, grown→fire), and stays silent on a downgrade (the hit already speaks). PLACEHOLDER timbre — a procedural octave-up sine sweep (520→1040 Hz) declared in the provider's `SfxRegistry`, exactly like the existing Hit/Pogo placeholder specs; the emit names the id, not the sound, so retune freely. Test `stepping_up_a_power_tier_voices_the_transform_chime`. The GROWING ANIMATION (the separate bug above) is NOT addressed here — this is only the sound.]

ABOVE IS NOT DONE.

* Similarly to mary-o sanic needs the transform animation, probably SFX. [potentially done — verified 2026-07-28: Sanic authors his own `TransformBeatPolicy` and requests the same engine beat (`game/ambition_demo_sanic/src/lib.rs`). The shared beat is what the Mary-O entry above describes.]

ABOVE IS NOT DONE. Transform is instant. I don't know if there is a transform sprite sheet for sanic, so he needs one. 


* When SANIC is hit, there it seems like he is given no iframes. He should also have some hitstun and be knocked back a bit, and then have a few second of recovery iframes. The rings don't splash out nearly large enough. He needs an opportunity to recollect some of them after his hitstun wears off and before they disappear. 

* In super sanic mode, sanic should be invincible, even to spikes. 


* Super maryo needs a super-star equivalent? We already have a "super star" invincibility music track ready to go. We need to use a shader for her invisible mode. It needs to have similar behavior to the superstar in mario in that when it rises up out of the brick it has a bounce behavior. Her super star equivalent prop will be the "cosmic quasar" or "pocket quasar" or something like a big bright galaxy, maybe. Details are less important than the effect.

* I want to change the maryo-milk to be the magical girl wand, which we've added a sprite for. I've recently changed the maryo sprites so tall is a magical girl, and fire is a second stage magical girl. We need to change the milk to be the magical girl wand, and the spark blossom needs to be replaced with the lantern (cinder beacon).

* In super maryo, The "turn big" powerup - previously milk, now the magical girl wand, needs to rise up from the brick and then move right or left bouncing off of walls like the mushroom in mario does.

* In super maryo, the enemies need to reverse direction when they hit a wall. And snakes should reverse direction if they are about to fall of a cliff, but ai slop should not.



* For the web build we can't use kaledioscope because lunex doesn't support wasm


* When maryo is in her death animation, she still gets hit by enemies.

* Maryo's fireball is very tiny, so it needs to get bigger. It also only shoots to her right, not the way she is facing. 

* There needs to be a bit of hitstun when maryo gets hit and there needs to be a similar transform animation down to the previous state with non instant duration.


* Maryo coins and sanic rings should not be magnetic to the player. 


* Maryo world 1-2 needs moving platforms that move vertically down and up like an elevator. When they go OOB (far enough so they are off screen of the player in normal gameplay) they can teleport to the top / bottom of the screen to make an infinite elevator effect.

* The maryo world 1-2 needs to happen after she wins world 1-1, she doesn't just get to go there in the middle of 1-1 and come back. It should be a new level. When she beats world 1-2 we should cycle back to world 1-1 for the demo. World 1-2 also needs to be built out a lot more, its very plain, and there are no enemies. I would like to add the flying snakes on a plane as enemies in this world. 

* In the hall of characters, the humanoid characters are all dramatically out of scale with each other. Alice and bob are great, but characters like the vikings, or jeff hinter render as tiny little characters and look out of place compared to the rest of the cast. The character art needs to be rescaled (probably at the generator level, not via some post-hoc fix) to balance the size of these characters so they make more sense appearing in the same game together. Note the player robot v3 is supposed to be chibi and short compared to other humanoids.


* maryo flashes when her fireball hits an enemy. that should not happen.


* I noticed a bug: maryo can stand on a broken brick. 


* There is an issue with resets in the maryo game (probably a problem in other games as well). When I reset the level old drops from enemies seem to be still seem to be there.


* When maryo-dies the enemies seem to reset before the death animation is finished. The level reset needs to happen all at once at a time that is easy to express in the game code. This might be a larger refactor if there is a structural problem here, and we need to avoid creating a monolith.


* We probably need an engine concept that allows actors to be dormant. This is important for maryo because ai slop will just walk off the edge of the level before she even gets to that part of the level, so we need to wake or sleep their brain depending on how close she is to them. This sort of optimization will likely be generally important for any game using the engine, although it's not something that should be inherent. There might be characters that don't go dormant off screen, this matters a lot for split screen or network multiplayer games. It also might matter in other cases. Not 100% sure how its elegantly expressed though.
