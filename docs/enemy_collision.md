# Enemy collision and knockback

Dummies are now simulated against the same room collision world used by the player.

The first dummy implementation only collided with a flat floor, so repeated slash knockback could push a target through walls. The current engine-side `Dummy::update_in_world` resolves dummy motion against `Solid`, `BlinkWall`, and landing `OneWay` blocks. It also sub-steps large knockback displacements so a high-speed dummy cannot skip over thin walls in one frame.

This is still intentionally simple: dummies are AABB bodies and do not yet have enemy AI, platform carrying semantics, crushing, or slope collision. Those should be added as explicit engine tests before they become gameplay-critical.
