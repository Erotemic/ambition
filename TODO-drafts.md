# This is a document that Jon might work in while an agent is working, it is
# where he drafts TODO items before making them official.

* When an NPC starts to attack they become a goblin enemy. 

* When an NPC dies after you kill them they have a dead goblin sprite and continue to move around. And you can still talk to them. Not sure if they are trying to respawn or supposed to remain dead.

* The goblin encounter music, the intro starts loud, but then it fades out instead of cleanly going into the main loop:
2026-05-07T20:30:23.658242Z  INFO ambition_music: start_adaptive_state cue=first_goblin_tune_v2 state=wave1 section=wave1 old_bank=A new_bank=B looped=true crossfade=1.70s gains=0.73,0.00,0.77,0.00,0.14,0.06
2026-05-07T20:30:23.658261Z  INFO ambition_music: started_music_sources cue=first_goblin_tune_v2 state=wave1 section=wave1 bank=B source_count=6 volume_blend=1.05s

This causes the user to hear a clear audio cut,  and from their point of view they should never notice that it isn't a single piece of music.


* We need to pad the sides so the touch control area does not overlap the gameplay area. 


* the fireball projectile needs to respect and bounce correctly on all
  surfaces, not just the solid ones. I.e. one way platforms.


* The boss music / or intro sequence is bugged?


* There is a collision issue on the wall that locks you into the goblin
  encounter. I think a ledge grab pushes me OOB.


* If the mouse is hovering over an option, but the user is using the arrow keys to navigate, the mouse prevents them from moving in any menu.

* the crates/ambition_sandbox/src/music/first_goblin.rs is probably too specific. We will have lots of encounters moving forward, so we need a generalized structure.

* moving on a ladder shouldn't slow you down. Need to be able to jump or dash off of it.






* Make the audio asset generation scripts more sane, so where staging and production is is more clear, and how to edit pieces is more clear.  

* Same clarify thing with image asset generation. We need to do a lot of code consolidation and improvement.

- [ ] I would like a "bubble shield" with a parry feature, dodge and roll mechanics, more smash-like ledge grabs.

* I want the on-screen buttons to be context sensitive. I.e. interact should
  show what the interaction is. The projectile button should probably become a
  "special" move, but neutral special will be the fireball or haudoken if we
  have the quarter circle forward input.

  The regular desktop mode might have a controls part of the HUD to show what
  inputs are available (e.g. OOT) to make sure players are aware of the control
  options.


- [ ] Gravity room, with gravity columns that change the direction of gravity.
  Switches can toggle these. Some of them might be on moving platforms.


* Would be nice to have a "falling sand sim" inside of the game. Something
  simple at first, just sand, oil, water, and you can make fire with your
  fireballs. Switches toggle which spouts are on. You can emit a trail to trace a
  path and have some sort of interaction with it.


* DESIGN RULE: These should never be a point in the game where you can't go
  back and get a refresher on some tutorial or how controls work, or what
  quests you are currently on, or what you are doing.

* Can we make the headless version of the game able to render screenshots so we can attempt to visually verify properties of the game in tests?


* Enemies are still running at normal time during bullet time, our multi-clock design should prevent this from ever happening, maybe we have introduced hacks? Maybe we need to make the system more robust so errors like these are not expressible?


DONE:

~~~ * Reset sandbox unloads every sprite except the character. The debug outlines show hitboxes, but I need to press f11 to get sprites to reload.  ~~~
~~~* I want to add a pirate faction room, and inside that is another room to fight a boss: the mockingbird. It should have some simple attacks where it swoops down at you, and every once in awhile maybe it shoots a hadouken or fireball and hovers for a few seconds. ~~~
~~~* The game assets are loading slowly at the start, probably because of all the music. We need an on demand music and sprite loader, instead of loading it all at the start.A!~~~~
~~~* I need a sprite for the main guy in flying mode, maybe with jets from its feet or something to show how its flying. A lot of the main guy sprites are just getting hurt, so we need to edit them in the sprite generation tool to have better visuals for fly. And crouch as well, when we crouch right now we move the sprite down into the floor too, which is not great. We could hack this and just squash the sprite if adding a visual is too hard, but note it as a TODO. and a hack.~~~




The into needs to have you moving up vertically through the lab maze. This lets you branch left and right to encounter one faction or the other, as well as rooms that meet in the middle where they are also fighting each other. The creator needs to be scripted to try and escape with you. So the creator gets an AI and fights with you and also waits for you to show up. If you attack the creator you he might say something like "you're really going to turn on me?" or something. The top of the lab needs to be a nazi and ludite boss, they fight each other, and you have to defeat whoever remains unless you surrender to either side or something mabye? Not sure exactly the shape, but I do know that the escape from the lab needs to be mostly vertical with a few horizontal sections and optional sections. 
