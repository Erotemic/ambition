# Review at the main machine

Tasks parked here because they need Jon **in person at the main machine**
(in-game reproduction, the LDtk editor GUI, or listening to audio) rather than
remote supervision. Committed so it reaches the main machine via git. Pivoting
to remote-friendly features in the meantime; pick these up when back.

_Last updated: 2026-06-21 (Opus 4.8)_

---

## 0d. NPC/Enemy unification — agreed, plan below (2026-06-22)

**Bug fixed first:** aggressive NPCs ballooned because the body-metrics render
size lived on `NpcConfig`, which the peaceful→hostile conversion drops (it swaps
the NPC cluster for the enemy cluster but keeps the body-sized `kin.size`, so the
enemy render re-applied `collision_scale`). Moved the render size to a SHARED
`ActorRenderSize` component that survives the flip; both sprite paths honor it.
Committed (`8211584a`).

**The real ask (agreed): there should be no "NPC" type — just an actor that is
hostile or not, Skyrim-style, hostility toward *anyone* (no player-centrism).**
How unified we already are: every actor shares `BodyKinematics` /
`ActorSurfaceState` / `Brain` / `ActionSet` / `ActorControl` / `ActorDisposition`
/ `ActorFaction` / `ActorIdentity` / health+combat read-models. What still
splits NPC from Enemy:
1. `ActorRuntime::{Npc, Enemy}` enum tag.
2. `NpcConfig`/`NpcStatus` vs `EnemyConfig`/`EnemyStatus` cluster components.
3. Two tick systems (`update_ecs_npcs` vs `update_ecs_actors`) — split ONLY
   because one query with both cluster QueryDatas panics on conflicting mutable
   access to the shared components.
4. The peaceful→hostile *conversion* that swaps clusters + retags.

Collapsing it *removes* code: one `ActorConfig` (the NPC-only bits —
interactable/dialogue, patrol/talk radii — become optional fields or a small
`Interactable`-carrying component), disposition is just `ActorDisposition` +
faction targeting, ONE cluster → ONE tick query (no conflict) → no migration
(hostility = flip disposition + swap Brain/ActionSet in place). Faction targeting
already exists (`ActorTarget`/`select_actor_targets`) and is the seam for
"hostile to non-player actors."

**Why staged, not one big-bang:** it changes runtime combat/patrol behavior I
can't verify headless — needs you to test "provoke an NPC, watch patrol/idle,
confirm no regressions" at checkpoints. Proposed order (each compiles + ships
green):
- S1 ✅ shared `ActorRenderSize` (done — and the pattern: actor-wide facts leave
  the per-disposition cluster).
- S2: merge `NpcStatus` into the shared status read-models (hit_flash/ai_mode
  already mirrored to components) — delete `NpcStatus`.
- S3: fold NPC-only config (interactable, patrol/talk radii) into a small
  `Interactable`+`PatrolBounds` component carried by ANY actor; delete the
  NPC-specific fields from the cluster.
- S4: unify the cluster + the two tick systems into one (the big one — resolves
  the query split by construction).
- S5: delete `ActorRuntime` + the conversion; hostility = disposition flip +
  brain/action swap in place.

**Scope found after an exhaustive map (2026-06-22):** ~30 interconnected files,
70+ branch points on `ActorRuntime::{Npc,Enemy}`. Two findings that matter:
1. **The seam already exists and is right.** `ActorAggression { mode, target }`
   (Passive / RetaliatesWhenHit{threshold} / HostileToPlayer, + a `target`
   Entity) + `ActorDisposition` already model "hostile or not, toward whom" —
   exactly the Skyrim shape, and `target` is the non-player-centric hook.
2. **The real obstacle is the brain pipeline, not the tags.** Enemies are
   ARCHETYPE-driven (`EnemyBrain` → `spec_for_brain` → tuning + brain), NPCs are
   CATALOG-driven (`character_id` → `default_brain_for_character_id`). The
   unified config must hold/recronstruct EITHER. The good news: the live `Brain`
   COMPONENT is already the per-tick authority for both — only *reconstruction*
   (provoke/dismount) reads the archetype/catalog path — so the merge keeps the
   `Brain` component authoritative and only unifies the reconstruction seam.

The cluster merge (S4) is atomic — the two ticks exist *solely* because two
QueryDatas both `&mut BodyKinematics`; you can't half-merge and still compile.
So S2/S3 are prep that compiles green but the headline "one tick" win only lands
at S4. This is a dedicated grind, not a session-end change.

**→ Full execution brief written:
[docs/planning/npc-enemy-unification.md](docs/planning/npc-enemy-unification.md)** —
self-contained, 7 phases, for a fresh agent to run end-to-end. Captures Jon's
Skyrim vision (relational/dynamic hostility, no actor "type", combat+dialogue
universal) and the agent-pluggable-brain future. Kills the archetype-vs-catalog
brain split (one resolver, `Brain` component authoritative).

---

## 0c. Debug box labels + FSM box flip-alignment (2026-06-22)

- **Every debug box is now labeled** with its type, as world-space text
  (`hurtbox` / `contact` / `collision` / `telegraph` / `active` / `npc` /
  `enemy` / `breakable` / `chest` / `hazard` / `hit:player|enemy|boss|npc` /
  `player` / `atk`). Label color matches the box color; each box *type* uses a
  distinct corner so overlapping boxes don't stack illegibly.
  - **TEXT SIZE KNOB:** one const — `DEBUG_LABEL_FONT_PX` in
    [crates/ambition_app/src/dev/debug_overlay/prims.rs](crates/ambition_app/src/dev/debug_overlay/prims.rs)
    (it's near the top of the "debug box labels" section). Bump it up/down to
    scale every label; nothing else to touch. (World-space `Text2d`, so it also
    scales with camera zoom.)

- **FSM box misalignment fixed (blind).** Root cause: the boss sprite flips to
  face the player, but the combat geometry was computed in the *unflipped* frame.
  The FSM's body sits off-center in its frame (the union-crop is widened by
  asymmetric attack noodles → left-biased `body_pixel_bbox`), so when it faces
  left the boxes landed on the opposite side from the visible body. Now the
  hurtbox + collision + contact boxes mirror with facing (no-op for centered
  bodies, so no other boss regresses). **Verify with the new labels** — the cyan
  `hurtbox` and orange `collision` / magenta `contact` should now sit on the
  visible body whichever way the FSM faces. If a box is still off, tell me which
  *labeled* one and I can target it precisely.

---

## 0b. FSM rerender fixed + authoritative hurtboxes (2026-06-21)

The flying-spaghetti-monster "crazy cropped mess that flashes" was a **boss
atlas dims desync**, now fixed two ways:

- **Flashing (root cause):** the boss sprite path recomputed a uniform atlas
  grid from the const `BossSheetSpec`'s authored-at-first-pass `frame_width`
  (169). After the `FRAME_SIZE` bump the regenerated sheet had 393×344 cells, so
  the boss indexed the wrong pixels and flickered. Fixed: the boss now builds its
  atlas + render aspect from the **published RON's per-frame rects** (via
  `record_for_target`), not the const — the same data-driven path characters use
  (and packing-ready). A row-alignment guard falls back to the const grid for
  sheets with no baked record. Also fixes the generic `boss` sheet, which the
  `render_scale=2` rollout had silently desynced (128 const vs 256 sheet).
  **No more Rust edit needed when a boss sheet's resolution changes.**

- **Authoritative hurtboxes (your point):** the FSM published only a single
  `body_pixel_bbox`, so combat used the coarse idle alpha bbox (the whole noodle
  spread) for every pose. The tack-on `build_sheet` now publishes **per-animation
  hurtboxes** keyed to the GENERIC keys the boss combat looks up
  (`rest`/`side_sweep`/`floor_slam`/`spike_halo`/`dash_echo`/`hit`/`death`) — the
  FSM remaps its rows (`noodle_whip`→`side_sweep`, …) so they're consumed. The
  published RON now carries all 7. **Rebuild + run to verify** (I published the
  FSM into the asset dir; the Rust fix needs a rebuild).
  - *Still using the generic fallback:* attack **hitbox** geometry (where the
    FSM's attacks damage the player) is still `BossAttackProfile`-driven. Authoring
    sprite-exact hitboxes is a follow-up that wants your in-game debug overlay
    (and they plug into the same `attack_hitboxes` hook I just added). The
    per-pose hurtboxes are the full visible body — if you want a *tight core*
    (only the eye is vulnerable) say so and I'll author `body_pixel_parts`.

---

## 0. Latest pass (2026-06-21 cont.) — RUN `./regen_sprites.sh` FIRST

Three things landed; the first explains why your sprite changes "looked the
same":

- **Publishing bug fixed (the big one).** Rigged characters (Emmy/`noether` and
  any rig-editor `.rig.json`) were listed in regen's expected-files but **never
  actually published** by it, and the caches ignored `.rig.json` — so rig edits
  silently never reached the crate (Emmy's sheet was weeks stale). Fixed: regen
  now discovers + publishes every rigged doc and hashes `.rig.json` in both
  caches. **Action: run `./regen_sprites.sh`** — Emmy should then be taller
  (collision_scale 2.0), hi-res, and her hairpin rigid (no bobbing). Verified
  the publish produces those; you just never had it.
  - *Publishing-elegance Q:* the contained fix is done. A bigger win would be a
    `cargo`/watch hook that auto-republishes changed content, but that's a
    separate refactor — say the word.

- **LDtk confusing sprites removed.** Every entity except `PlayerStart` is now a
  plain colored **region box** (renderMode Rectangle) instead of a gizmo/sprite
  stretched over its bounds — that stretching is what made zones look like
  geometry, and Tile mode hid the box outline (why the Solid square was
  invisible; the outline is back now). New tool: `visual-manifest
  clear-entity-icons`. The generic spawners (boss/enemy/npc) got no sprite (they
  couldn't say *which* one). **Verify:** regions readable, Solid visible. If
  Solid's fill is still faint, I'll bump its opacity (one tweak). Door→door
  sprite + per-enemy art are the noted follow-ups.

- **Slash effect reference frame — fixed twice.** First pass oriented it but
  pulled `dir` from the *manifest* hitbox (`manifest_attack_hitbox_world`),
  which positions with SCREEN-axis offsets and never rotates into gravity — so
  in the rotated C4 arms a forward jab pointed screen-left/right (up/down were
  fine). Now it drives off `spec.hitbox_offset`, which is `into_world_frame`d
  (gravity-rotated), so the effect lives in the player frame under all four C4
  gravities. Pinned by the combat C4 test (strike dir) + render rotation test.
  **Re-verify in the symmetry room.** Handedness (left-facing crescent curl) +
  size are the remaining likely tweaks.
  - *Hitbox-vs-effect decision (your "can't decide"):* hitbox stays the
    authority, effect orients to it.

- **DONE — hitbox from sprite metadata (your "bigger general" point).** NPC
  collision is now derived from published sprite `body_metrics`: a `body_pixel_bbox`
  (or multi-part bounding box) **supersedes** the LDtk spawn box, so an NPC's
  hitbox is a box around the *visible* body. No new flag — presence of metadata =
  authoritative; **absent → falls back to the LDtk bounds** (portals etc. keep
  their box, exactly your example). The sprite still draws at its current size:
  the derivation also stores the render-quad size and the renderer uses it, so the
  collision shrinks to the body without the sprite double-scaling. New helper
  `sprite_body_collision_for_character_id` (mirrors the boss `body_metrics`
  pipeline, generalized to catalog characters). **Verify in-game:** Emmy's debug
  hitbox should now wrap her silhouette; her sprite should look unchanged. Feet
  planting under the new (body-sized) collision is the one thing worth a glance.
  - *Scope:* applied to **NPCs** (the Emmy case) — narrow-first. The helper is
    reusable; enemies still use their hand-tuned archetype `default_size`
    (changing those affects platforming/combat fit, so opt-in later if wanted).
    Props don't go through the character-sheet path, so they keep LDtk bounds.

---

## 1. Mockingbird attack swing-gate — capture an F8 trace ⏳ (do this)

**Why it's machine-bound:** needs you to reproduce the bug live and press F8.

**The bug:** standing/flying inside the mockingbird body, pressing attack
produces **no swing sound and no hit** (works fine when disjoint). You confirmed
you're *not* taking contact damage inside the body, so it isn't hitstun.

**What I found:** I traced the whole attack pipeline (input → `melee_pressed`
passthrough → `resolve()` → `emit_brain_action_messages` → `start_attack`).
There is **no position / boss-overlap / fly-state dependency** that should gate
the swing. The only gates are `hitstun<=0` (ruled out), `abilities.attack` (on),
and `attack.is_none()`. So the cause isn't visible statically — it needs a trace
from your real repro.

**What to do:**
1. Reproduce (fly into the boss, press attack, observe no swing).
2. Press **F8** within ~4 s (the ring buffer holds ~240 frames).
3. Send me the dump's `## Frames` block from `debug_traces/`.

**What it'll tell us:** each frame line now carries
`atk[p=.. s=.. abil=.. hs=.. inv=..]` — `p`=attack pressed, `s`=swing live
(`attacking`), `abil`=attack ability on, `hs`=hitstun, `inv`=invuln. A frame
with `p=true` where `s` never flips `true` (with `hs=0`, `abil=true`) pinpoints
whether it's a stuck `attacking`, the ability flipping off, or a logic bug in
the start gate. (Instrumentation landed in commits `a6460e3b`, `42e58ee6`.)

---

## 2. LDtk tiles & sprites not rendering nicely in the editor 🎨

**Why it's machine-bound:** needs the LDtk editor GUI to see what's actually
wrong and to verify a fix.

**STATUS — shipped (2026-06-21).** The sprite renderer now emits an
LDtk-consumable visual manifest and the worlds use it. New command:
`python -m ambition_sprite2d_renderer ldtk-manifest [--all-sheets]`
→ writes `crates/.../assets/sprites/ldtk_sprite_manifest.json`
(`{tilesets, entity_icons}`), consumed by the existing
`ambition_ldtk_tools … visual-manifest apply-manifest`. `regen_sprites.sh`
emits + re-applies it.

The committed/default set is **curated** — every registered tileset is
actually used by an entity def (no orphans):
- `PlayerStart` → `player_robot`
- `NpcSpawn` → `merchant_prototype`
- `EnemySpawn` → `goblin`
- `BossSpawn` → `gnu_ton_boss`

wired into all three worlds. Open the editor and those entities should show
real sprites instead of gizmos. `--all-sheets` registers **all 120** sheets
as tilesets so any of them can be assigned by hand (not committed by default:
~6.8k-line `.ldtk` diff + every sheet PNG is gitignored + 116 would be
unused — run it locally when you want the full tileset browser).

**To verify at the main machine:** open `sandbox.ldtk` — PlayerStart /
NpcSpawn / EnemySpawn / BossSpawn should render real sprites. Decide whether
to (a) commit `--all-sheets` too, and/or (b) pick different representatives
(it's a one-line-each dict, `DEFAULT_ENTITY_SPRITE_MAP` in
`tools/ambition_sprite2d_renderer/.../ldtk_manifest.py`).

**Phase 2 (the real richness — needs an LDtk-schema decision):** the generic
spawners (`EnemySpawn` / `NpcSpawn` / `BossSpawn`) are 1:many, so a single
representative would mislead. Proper per-instance editor visuals mean adding a
field (an enum whose values carry each character's `tileRect`, or a Tile/enum
ref) so each *placed* spawn shows its actual character. That's a schema +
tooling task to design in the editor — left for review.

**Symptom (your words):** tiles and sprites don't render nicely in the LDtk
editor; we may need to emit JSON at generation time that LDtk can consume.

**What already exists (so we build on it, not from scratch):**
- `editor_icons.png` + an `EditorIcons` tileset with per-entity `tileRect`
  wiring (entity *icons* — commits `cccb04e2`, `e5d15def`). This is the *entity*
  side and is at least partly working.
- `tools/ambition_ldtk_tools/.../edit/tilesets.py` — registers tileset defs
  (`defs.tilesets[]` = source PNG + grid metadata). See **ADR 0015 (LDtk
  tileset rendering)** for the plan this was meant to unblock.
- `.../edit/assets.py` — points entity defs at tiles in registered tilesets.
- `.../edit/visual_manifest.py` — **the key one.** Its docstring says it is
  *designed* to consume "a small, stable manifest shape that can be generated
  from today's fixtures and later from whatever RON/YAML metadata the sprite
  refactor emits." That is exactly your "emit JSON LDtk can use" idea — the
  consumer half exists; the **generator half (sprite/tile renderer emitting that
  manifest) is not wired up.**

**Working hypothesis:** the sprite/tile generator
(`tools/ambition_sprite2d_renderer`) produces PNG atlases but does **not** emit
the LDtk-consumable visual manifest (tileset grid size / spacing / padding +
per-sprite tile rects). So tilesets are registered by hand or not at all, and
LDtk's uniform-grid tile slicing doesn't line up with our packed atlases →
sprites/tiles look wrong in the editor.

**To review/decide when back:**
1. Open the `.ldtk` in the editor and pin down *what* is broken — Tiles layers,
   entity sprites, both? Wrong cell/offset, clipped, or just missing?
2. Check: are the tilesets registered (`defs.tilesets[]`), do the referenced
   PNGs exist on disk, and do their dimensions match the `.ldtk`'s recorded
   tileset sizes? (mismatch = slicing garbage).
3. Decide the manifest schema the sprite generator should emit (grid size,
   spacing, padding, per-sprite rect) so `visual_manifest.py` can consume it.
4. Wire the generator to emit it; regen via `regen_sprites.sh` (keep a fresh
   clone working — regen-invariant). Then re-open the editor to verify.

**Constraints to respect:** never hand-edit `sandbox.ldtk` JSON — go through
`ambition_ldtk_tools` (add a subcommand if one's missing). Sheets/atlases are
gitignored (regenerated); commit Python + the `.ldtk` wiring only.

---

## 3. Player spawn placement — feet sink into the floor (in-game verify) 🔎

**Why it's machine-bound:** needs in-game verification I can't do headless.

**Status:** deferred + flagged earlier. When spawning from a `PlayerStart`
(`--start-room` / respawn), the player's feet land ~17–31 px *into* the floor
(its AABB is top-aligned to the entity). **Door entry is correct**
(`gap_below_feet≈2 px`), so it's a Rust spawn-placement convention issue
affecting all rooms via `--start-room`/respawn, not a one-room LDtk tweak. The
trace warm-up gate already silences the harmless 1-frame dump it caused.
Low urgency; worth a proper fix once you can eyeball it. Say the word and I'll
take it as its own pass.

---

## 4. Sprite resolution / in-game size knobs (rig path done; fleet rollout pending) 🔭

**Why partly machine-bound:** the *crispness* and *Emmy's height vs the player*
only read true in the running game.

**Done (2026-06-21):** rigged characters can now specify their own in-game
tuning + render resolution, instead of inheriting the `DEFAULT_TUNING`
(collision_scale 1.5) fallback:
- A rig doc may carry `"sprite_tuning": {"collision_scale": …, …}` — emitted to
  the sheet RON and used by the runtime for **in-game display size**
  (height = `collision * collision_scale`). This is the knob for "make X
  taller/bigger" without touching gameplay collision.
- `"frame": {"render_scale": N}` multiplies the texture's pixel resolution
  (geometry stays in base-frame units). The in-game **size is unchanged** —
  `sprite_render_size` derives height from collision and only takes *aspect*
  from the frame — so this is pure anti-pixelation: more native pixels under
  the same display quad.
- Emmy (`noether`) uses both: `collision_scale 2.0` (taller than the short
  robot) + `render_scale 2` (crisp).

**Verify at the main machine:** is Emmy now clearly taller than the player, and
not pixelated? `collision_scale` is a single number in `gen_noether_rig.py`
(`sprite_tuning`) — nudge if she reads too tall/short. (I calibrated 2.0
blind; the player is ~`48 * its collision_scale * 1.16` tall.)

**Fleet rollout — DONE (2026-06-21).** `RenderConfig.render_scale` (default
**2**) now flows through `authoring/sheet.py` (`build_spritesheet`), so every
toon/adapter sheet renders at 2× native resolution. The "128-base" generators
(`robot_side`/`goblin_side`/`boss_side`) were drawing at an absolute scale
that ignored the canvas size — fixed to scale-to-frame-width like the toon
generator (`S = float(scale) * size[0] / 128`), identical at the 128 default,
only activating at render_scale>1. Verified headless: player_robot 128→256,
goblin 121→239, boss 128→256, absurd_general 90→176 — all ~2× with aspect
preserved; 21 slow-render pipeline tests pass at the new default.

Non-breaking: in-game display size is collision-driven and takes only aspect
from the frame, so this is pure anti-pixelation. Sheets are gitignored, so this
landed as a **code-only change** — the fleet re-renders crisp on the next
`./regen_sprites.sh`.

**Verify at the main machine:** run `./regen_sprites.sh`, then confirm in game
that NPCs/enemies/player are noticeably sharper. If something is *still* soft,
it's being displayed even larger than 2× covers — bump the default to 3 in
`RenderConfig`, or set `render_scale: 3` on that character's config. (Cost:
disk + render time scale with the square of render_scale.)

Per-character in-game *size* tuning for the toon path already exists too
(`sheet_tuning` in a character's YAML → RON), so retiring the hardcoded Rust
`SheetTuning` consts in favor of data-driven tuning is available whenever you
want it.

**Big bespoke sprites scaled up — DONE (2026-06-21 cont.).** Root cause for the
flying spaghetti monster / Broadside Bess softness: `render_scale` only flows
through the **adapter** path (`build_spritesheet`). The big bosses + heavy
pirates are **tack-on** generators (`build_sheet`) that bake their own
`FRAME_SIZE` — typically a 128–320 base, i.e. *half* the adapter baseline (the
player is 256). Displayed 2–3× larger than a normal character, they upscaled and
read soft. Measured native bodies (cropped frame): spaghetti **169×150**,
broadside_bess **172×138** — both *smaller* than the 256 player despite rendering
much bigger. Fix = bump each bespoke generator's `FRAME_SIZE` (geometry is
authored in `WORK_FRAME_SIZE` units + supersampled, so this only preserves more
detail — no redraw, no gameplay change):
- `flying_spaghetti_monster_boss`: `FRAME_SIZE (320,256)→(800,640)` → body
  **169→393** (2.3×). Sheet 3244×2408, ~31 MB texture.
- `pirate_heavy` (covers broadside_bess / iron_mary / salt_annet):
  `FRAME_SIZE (320,288)→(640,576)` → body **172→319** (1.85×). Sheet 2652×1500.
- Surveyed the rest: gnu_ton (768) + mockingbird (576) already crisp; smirking
  behemoth renders at `collision_scale 1.0` (doesn't display large). So these two
  were the real offenders. Verified headless; both well under the 8192 texture
  limit. Lands on the next `./regen_sprites.sh` (leaf-hash sees the `.py` edit).

**Verify:** spaghetti monster + heavy pirates noticeably sharper. If a boss is
*still* soft it's displaying even bigger — bump its `FRAME_SIZE` further (cost:
the texture grows with the square; a 400px-body boss is already ~30 MB, a truly
crisp 600px-body one is ~130 MB, so there's a VRAM ceiling — that's where
splitting / packing earns its keep).

**✅ RESOLVED (was: 3 sheets exceed the 8192 GPU texture limit).** This is now
**done** and superseded — do not treat it as open. The generator alpha-trims +
MaxRects-packs every frame into pages capped at `page_size` (default 4096), policy
data-driven per target in `registry/pack_groups.py`; the old one-column-of-rows
layout is gone (now "the legacy grid" fallback for untrimmed targets only). The
16384-px dimension crash is guarded by `page_size`/`max_dim`. The Rust atlas
builder consumes per-frame `rects` + trim offsets (`with_render_basis`), so no Rust
change was needed for the layout. See `docs/planning/engine/visual-quality-profiles.md`
("Sheet packing — already done") for the current state; the remaining VRAM lever is
*render resolution*, not packing.

  *Historical note (the original latent finding):* `player_robot` **2148×10752**,
  `player_extended` 2168×8960, `sandbag_full_review` 2190×8960 were tall because the
  layout stacked every animation **row** in one column (`frame_height × num_rows`).
  That layout no longer ships.

---

## 5. Player slash effect — hooked up; tune look/size/placement in game 🗡️

**Why machine-bound:** the effect's on-screen size, position vs the strike,
and timing only read true in the running game.

**Done (2026-06-21):** the generated `robot_slash` sheet (was never referenced
in Rust — the player only ever spawned a yellow debug hitbox box) is now wired
to player attacks. `start_attack` emits `VfxMessage::Slash`, picking the row
from the attack intent: **Side/Up** energy-arc crescents for most swings, the
tapered **Down** lance/poke for down-tilt & pogo (the "different one" — already
generated). One shared effect; each attack can graduate to a bespoke one later.

**Verify at the main machine:** do attacks now show the arc/poke (not a yellow
box)? Is it the right size/position? Knobs:
- size: `slash_effect_size` in `attack.rs` (currently 2× the hitbox's max dim).
- position: spawned at the hitbox center — could anchor at the sheet's `origin`
  (hand/pivot) anchor instead for a more rooted swing.
- timing: fires once at swing start; uses scaled time (slows in bullet-time).

**Refactor that came with it (your call paid off):** the shrine's private
record→atlas / row-lookup helpers were lifted into a shared
`rendering::sheet_atlas`; the slash lives in its own `rendering::slash_visuals`
next to `shrine_visuals`, both built on it. The old `SlashPreview` debug box is
retired (if you want a toggleable hitbox overlay back, that's a small debug
gizmo, separate from the effect).
