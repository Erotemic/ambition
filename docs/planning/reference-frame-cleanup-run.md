# Reference-frame cleanup + consolidation run (2026-06-16)

Autonomous run. Tasks from Jon, in priority order. Live progress window — Jon
reads this while away.

Foundation already landed this session: `engine_core::reference_frame::AccelerationFrame`
(explicit input/player/world frames; `descend` / `to_world` / `to_world_half` /
`launch`; non-cardinal-capable). Pogo now classified + placed in the player frame
(confirmed working in-game).

## Tasks

- [ ] **A. Delete redundant engine pogo** (`try_pogo_clusters` + `pogo_hits` +
  its glide_and_air tests). Greenlit now that the sandbox hitbox pogo is confirmed.
  Consolidate to one pogo path.
- [ ] **B. Player AABB orientation under sideways gravity.** The player's visible
  collision AABB stays upright while the sprite rotates (sprite clips the wall).
  Same class as the slug; unify the "oriented footprint" idea via AccelerationFrame.
- [ ] **C. Flight gravity-relative.** Flight ignores the reference frame today;
  under right-gravity, pressing right should move the player *player-right* (screen-up).
- [ ] **D. Configurable input→player frame mapping.** Default = the hybrid gut-feel
  (rotate ≤90°, screen-align past 90°). Options: "screen frame = input frame",
  "player frame = input frame". A config the AccelerationFrame / gates respect.
- [ ] **E. Touch-joystick U/D/L/R markers** that rotate with the player frame — a
  live visual of the player reference frame vs the control frame.

## Progress log

(start) — run begun; wakeup floor set at 30 min.

- DONE **C (flight)** + **D-foundation**: `InputFrameMode {Screen,Player,Hybrid}` +
  `AccelerationFrame::control_frame(mode)`; `MovementTuning.input_frame_mode`;
  flight integrates in control-frame components (byte-identical normal gravity,
  rotates under sideways/up). Next: D runtime config switch + dev key.

- DONE **A (dead code)**: deleted the redundant probe-based engine pogo
  (`try_pogo_clusters` + `handle_attacks_clusters` pogo branch + `FrameEvents.pogo_hits`
  + phases consumer + 2 engine-pogo unit tests). One pogo path now: the sandbox
  hitbox pogo (`advance_attack`). Replay byte-identical; 166 engine tests green.
  (`player_control_phase` has 2 now-vestigial params marked `_`, removable later.)
