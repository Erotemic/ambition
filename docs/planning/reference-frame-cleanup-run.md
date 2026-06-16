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
- DONE **E (joystick glyphs)**: U/D/L/R overlay on the move joystick, rotated into screen space each frame by the live AccelerationFrame (raw player frame). Pickable::IGNORE so it never blocks the stick.
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

## Scope expanded (Jon, mid-run): "everything relative to the reference frame"

The AccelerationFrame is the foundation; the remaining work is applying it everywhere:
- **Actor collision/footprint AABB** orients under the frame (player, kernel NPC,
  enemies, grounded bosses behemoth/gnuton/trex). `frame.to_world_half(size/2)`;
  byte-identical for vertical gravity, swaps sideways. (Slug already does this via
  surface_normal — generalize to gravity for the rest.)
- **Sprite "upright"** = gravity-relative for ALL (player/NPC/enemy/boss already via
  ActorRoll — boss fixed earlier this session; mockingbird/gradient-sentinel too).
- **Apple (projectile) sprite** must rotate to the frame (currently doesn't).
- **Mount + rider as a UNIT**: shark + rider rotate together around their COMBINED
  (mass-weighted) center of gravity, not independently. Needs:
- **Mass per entity** (a `Mass` component) → COG = Σ(mᵢ·posᵢ)/Σmᵢ; shark ≫ rider.
- **D: done in principle** (InputFrameMode + control_frame + tuning field); no dev
  key wanted (F3 UI later).
- **E (joystick U/D/L/R glyphs)**: mirror the button glyph/Text pattern; position 4
  glyphs by the control_frame each frame. Low risk.

- Jon: **mass comes from authored RON** (serde default so no sprite regen).
- Jon picked: mass-weighted COG + shared roll for mount/rider; sequence = joystick → AABB → apple → mount/mass.
