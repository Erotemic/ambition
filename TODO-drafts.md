# This is a document that Jon might work in while an agent is working, it is
# where he drafts TODO items before making them official.

* The goblin encounter music, the intro starts loud, but then it fades out instead of cleanly going into the main loop:
This causes the user to hear a clear audio cut,  and from their point of view they should never notice that it isn't a single piece of music.


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

- [ ] I want the "bubble shield" to have a dodge and roll mechanic. Bubble + Down is a dodge and bubble + direction is a roll.

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

* The touch screen controls allows you to move the character in a cut scene and dialog. This indicates there is a break in contract of the controller abstraction that should shuttle any type of input device into something standard the game sees as player controls.


* the gnuton boss needs to have an attack where apples drop from the ceiling and damage you.

* We need to add an Alice NPC corresponding to the Bob NPC. Bob is the architect, and Alice is the cryptographer. We also need an Eve NPC who tries to listen to their conversations. Mallory is the malicious attacker. Trudy is the intruder. Craig is the cracker, and Sybil is a pseudonymous attacker who creates a massive number of fake identities. Trent: trusted arbitrator. Victor & Peggy: The verifier and prover. Walter is the warden. Olivia: The oracle. Judy the judge. 

* The intro needs to have you moving up vertically through the lab maze. This lets you branch left and right to encounter one raider group or the other, as well as rooms that meet in the middle where they are also fighting each other. The creator needs to be scripted to try and escape with you. So the creator gets an AI and fights with you and also waits for you to show up. If you attack the creator he might say something like "you're really going to turn on me?" or something. The top of the lab can hold a later boss handoff once the opening is no longer carrying named factions. Not sure exactly the shape, but I do know that the escape from the lab needs to be mostly vertical with a few horizontal sections and optional sections.
