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

## Batch delivered (testable)
- DONE flight gravity-relative; pogo player-frame fix; engine-pogo consolidation.
- DONE boss orientation (sprite roll + gravity-aware facing) — earlier this session.
- DONE enemy + NPC + slug footprint AABB → AccelerationFrame (kernel guide fixed).
- DONE apple projectile rotates to its acceleration frame.
- DONE joystick U/D/L/R glyphs (input→player mapping; no screen relationship).

## Remaining (careful) — ALL DONE
- DONE Player COLLISION box → `aabb_oriented(gravity_dir)` everywhere in the sweep
  (sweep_x/sweep_y/resolve_axis/resolve_vertical). Byte-identical for vertical
  gravity; replay green, 166 engine tests green.
- DONE Player VISIBLE debug box → `aabb_oriented`; gravity threaded via
  `FeatureDebugQueries` (SystemParam) to stay under Bevy's 16-param ceiling.
- DONE Grounded-boss footprint AABB → `AccelerationFrame::to_world_half` in
  `bosses/tick.rs` (gravity threaded via `GravityCtx`).
- DONE Mount + rider rotate as a unit: new `Mass` component (RON-authored,
  serde default 1.0; shark = 6.0). `sync_riders_to_mounts` rotates the saddle
  offset by the gravity frame + pivots the rider around the mass-weighted COG.

## Follow-up bugs from Jon's testing (FIXED)
- Run-axis sign: `move_axis` returned screen-down for BOTH walls (sign-blind).
  Run now follows the control-frame `side` → right-gravity = screen-up,
  left-gravity = screen-down, up-gravity screen-aligned (Hybrid). `move_axis` deleted.
- Facing: `gravity_aware_flip_x` dropped its `g.x > 0` term (an artifact of the old
  screen-down move-axis). The rolled sprite's natural facing `(g.y,-g.x)` IS the
  control-frame `side`, so the sprite faces the run direction; only up-gravity
  inverts. Player now moves AND faces correctly under sideways gravity.

## What to test (runtime-visual, shipped blind)
- Cycle gravity with `\` (down→left→up→right). Under each: run left/right and check
  the sprite both MOVES and FACES the player-relative direction.
- Toggle the debug box (player hitbox) — the cyan box should rotate to match the
  rolled sprite + collision under sideways gravity.
- Grounded bosses (behemoth/gnuton/trex) + the pirate-on-shark under a flip: footprint
  + saddle should track the rotation (shark heavy → pirate orbits it).
