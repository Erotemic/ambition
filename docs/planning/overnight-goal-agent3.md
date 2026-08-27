### Moveset Balance UI

I want a moveset balance stats diagnostic presentation. I want some mechanism
(perhaps some web / html / javascript interface - or maybe even via python +
qt) to view the stats of each smash character, see their movesets, numbers,
hitbox visualizations, etc... i.e. things related to balancing or inspecting
characters that is faster to do than loading up the game and doing a playtest
match with them. Perhaps, similar to the music ratings GUI there can be a way
to provide feedback on a specific version of a moveset or character in general
so I can more efficiently give feedback and prompt revisions of characters
(i.e. I can say address the feedback on the pointed polygon character).

It may be helpful to add an ability to the smash game to directly export what
is drawn and consumed in game, so we can compare it to how we might model it in
python / javascript. (We could even build a python server for the interactive
webpage that consumes assets, I'm not sure, I have no major opinion on the form
this feedback mechanism takes, except that its some form of UI).

In addition to tuning, this should serve as documentation and a demo of the
different characters and their moveset to help new players learn the game.

This should let us "prove" that up-b works because to build this we run the
characters in the real engine and use control frames to show how the game
reacts to their inputs and we will see things like the pirate flying around on
the shark because we need to make the animations we save for the moveset
inspector to represent in game behavior.


### Pirate LaserGunSword as Side B.

The pirate side b should briefly equip the lasergun sword and fire a lasersword
projectile in the left/right direction the side b was directed towards. When
the side-b resolves it should locate the nearest opponent and angle the
equipped gun and shot so it fires in their direction if they are in the half
plane the side-b was directed towards.


### Projectile Polygon Down-B should "poop" bombs.

The projectile polygon should poop a bomb onto the stage, they should be able
to pick it up and throw it.  The bomb should detonate in 4 seconds or if it
hits something with enough velocity, whichever comes first.


### Mewtwo / Palutena / Zelda style teleports

The robot has a blink up-b, similar to how it works in ambition in terms of the
animation, but the animation for the author teleport up b is different, instead
of a phase-out effect, it is more of a affine transform to a point, with a
store of star flash for the blink out, and the opposite of that for the blink
in at the destination spot. We need to be sure we have some sort of aim assist
when the blinks are aimed at a ledge.


### Projectile Polygon Neutral B (power ball) should be able to hold its charge.

This should have parity with samus / mewtwo 'b', so that means it needs to be able to store a charge and fire at different sizes.


### Boomarang Projectiles

I think the projectile polygon should be able to use her ponytail as a
boomarang for her side-b.

