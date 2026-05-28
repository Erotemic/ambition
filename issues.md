Several issues:

* First we are not making use of the new sprites, especially the new tilesets. We need to hook those in, and likely make ldtk aware of them so visuals in ltdk somewhat match the visuals in game. We will need to ensure we are rending them there too. Make sure not to hack anything together if it is supported by cannonical bevy or cannonical ldtk usage (esp wrt to the bevy_ldtk crate).

   Specific examples of this: we are not using the new creators sprite, we are not using the wagon sprite, we have lots of lab props that are not used, and the intro raiders need their placeholder art reviewed against the current sprite roster.

   
   The door into the intro should spawn us on the diagnostic cart, just as it would happen if the game was completed and we started a new game.


* Second: We almost never want to use doors when defining levels. Entrances and exits should be side scrolling exists, so you just walk through to the next room, or the room just opens up as a very big room. We should prefer a Gridvania layout for the intro ldtk levels as we want to build up a strong map that is fun to platform through and explore. 


* Cutscenes are completely broken, they only show text in the debug view, and it says press E to continue and that's not even right. We need to make sure we have robust ECS native systems for dialog where the user needs to provide input or acknowledge and more real time dialog where characters just say thing (like when they get hit).


* We need to be sure the aggressiveness levels of the actors as intro enemies is already set to the point where they attack the player. Architecturally there should be no distinction between an enemy and an NPC other than that for an enemy the aggressiveness is at a level where they attack. I think this is codified as an "Actor" but maybe it needs more polish and refactoring. 
