# Agents should only edit this file to mark something as potentially done. Jon
# will remove it if it is actually done, or mark it not actually solved if an
# attempt doesn't work.


* In the Sanic demo, when sanic runs past a point, he turns super sanic no matter what and this should not happen.

* The pipe to let Mary-o come back up is placed in a weird spot, and the button to interact with it is "interact" instead of being correctly bound to "down"  or "up" depending on if you go down into the pipe or up into the pipe.

* I see no way to get mary-o to have the fireflower, or whatever our equivalent is, and it does not change her sprite to the fireflower one. 

* In mary-o we need a "growing" animation when she grows or transforms. In single player this might request that time around the transforming character slows down as an effect, but in a multi-player setting the time slow needs to be agreed upon by all players, as we (should have) codified in the very early stages of development.

* Mary-o Needs a transform SFX.

* In sanic, the rings don't explode outward when sanic gets hit and has rings.

* Similarly to mary-o sanic needs the transform animation, probably SFX.


* When you strike a dead character entity bbox when they are dead, they still bark. It might be worth something structural to prevent intangible things from interacting or presenting in any way.



* For the web build we can't use kaledioscope because lunex doesn't support wasm


* mouse over the icons in the game select title screen for ambition does nothing. Should probably have similar touch interactions as the grid menu. Or maybe we even reuse the grid menu. Also in this title menu the "ambition" and whatever text is at the bottom is WAY too small. Buttons need to be bigger and touch optimized. "Play" needs to be "Choose Game".

Some form of the "Menu" probably should be available here, so you can change global engine properties like audio mute. Currently the touch menu icon does nothing. We should not use kaleidoscope menu here, but we might use part of the IR of the system menu currently used by ambition, but only for generic global all-game properties. Then in ambition itself, it would extend or compose with that IR to add the additional one it needs to build its in-game system menu. 


* In "Sanic" and "Mary-O" Health and mana overlay the rings. These probably should be per-game HUDs. Mana and health might not be intrinsic to a character. The ambition character will have it, but sanic has the ring health mechanism, and mary-o has the power up health mechanism, and neither has mana.


* We should change title music to "Something Worth Building", and make it easy to define these mappings.
