# Input buffering feel pass

Ambition already had two classic platformer forgiveness systems in the engine: jump buffering and coyote time. This pass makes those feel systems more explicit and starts extending the same idea to other verbs.

Current behavior:

- `jump_buffer` keeps an early jump press alive for a short window, so pressing jump slightly before landing still jumps on the first legal frame.
- `coyote_time` keeps ground jump legal briefly after leaving a ledge.
- `dash_buffer` now keeps a dash press alive briefly while dash is cooling down or about to become available.
- sandbox interaction input now has a small buffer window so doors, chests, and NPCs do not require a single exact overlap frame.

The movement engine remains the owner of core character feel. The sandbox only buffers contextual interaction because door/chest/NPC usage depends on room-level presentation and loading-zone policy. Future buffers should follow the same split: put reusable movement/combat legality in `ambition_engine`, and keep story/sandbox context actions in the story crate until they become general mechanics.

Good next candidates:

- attack buffer during dash/recovery windows,
- pogo buffer when the player presses slightly before entering slash range,
- blink release buffering around cooldown expiration,
- per-action debug visualization in the HUD.
