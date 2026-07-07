# FABLE FINAL AUDIT — 2026-07-07 (the last fable pass)

Whole-repo audit after the opus/codex decomposition landing (E1a–e, E2
projectiles, W3 world/LDtk split, W-queue step 3 lowering proof, asset-manager
carve, sprite-sheet absorb, encounter mint, boss tail, `gameplay_core` →
`ambition_actors`, `game/` re-home). **Findings are appended IN PRIORITY ORDER
as they land — treat every entry as a plan item even if the session cut off
before it was folded into the ledgers/cards.** Anything here that contradicts
an older card wins (it is the newer ruling).

Audit order (most valuable first):
1. Dep-graph / tier audit — does the crate DAG match architecture.md's arrows?
2. `ambition_actors` (68k) — the residual monolith's next decomposition line.
3. Facade/shim census — the E7/E8 dissolution checklist, made explicit.
4. Ruling-compliance spot checks (W3 zero-LDtk, [W-e] hard error, GeoId
   adoption, SweepSample adopters, Tier-0 purity).
5. Subtle-correctness greps (query order, time domains, pushout, Entity
   identity, seam races).
6. Full test gate.
7. Elegance directions newly visible in the post-carve structure.

## Findings

(appended below, newest last)
### F1 — Dep-graph audit: the DAG is sound; ELEVEN arrows need work (none design-hard)

The workspace DAG has no cycles (`actors → sim_view` is dev-dep only) and the
big shape is RIGHT: engine_core/entity_catalog at the bottom are dep-free,
characters/combat/primitives sit above engine_core, `game/` sits on top.
The arrows below are the full remaining cleanup list, each with its
prescription — log-once so E7/E8 executors don't re-derive:

1. **`ambition_world` → combat + interaction + portal (VIOLATES the §4b
   "world IR names ZERO runtime types" ruling — transitionally).** Cause: the
   legacy `Authored<DamageVolume>` / `Authored<Interactable/Pickup/Chest/
   Breakable>` / `PortalSpec` families still ride `RoomSpec`. This is EXACTLY
   what [W-b] record-over-schema dissolution removes. **Prescription: each
   W-queue step-3 branch conversion's exit test is "delete the corresponding
   Cargo dep from ambition_world"** — hazards → drop combat; interactables/
   pickups/chests/breakables → drop interaction; portals → drop portal
   (portal placement becomes a Tier-0 schema variant: color/link/normal are
   plain data). `zone_sfx: Option<ambition_sfx::SfxId>` on the room graph is
   the same disease in miniature — an authored sfx REFERENCE should be a
   plain string/id newtype in the IR (Tier-0 idiom), killing world → sfx.
2. **`ambition_actors::portal` is a FACADE that re-exports
   `ambition_portal_presentation::*`** — the sim crate structurally deps a
   presentation crate to keep old `crate::portal::` paths alive.
   **Prescription: repoint the (few) consumers to the two real crates and
   delete the facade module + the Cargo dep.** A sim crate must never dep a
   presentation crate, even for re-export.
3. **`ambition_vfx` → `ambition_characters` for ONE type (`ActorFaction`).**
   The effect vocabulary crate pulls the whole cast crate for a tag it only
   uses to pick a tint/side. **Prescription: the vfx message carries the
   presentation-neutral fact it actually needs (a `HitSide`/tint enum owned
   by vfx, mapped at the emit site); drop the dep.**
4. **`GameMode` lives in `ambition_actors` and leaks it into host,
   touch_input, and (via schedule/run-conditions) render.** It is a tiny
   session-state enum. **Prescription: move `game_mode` DOWN (candidate: its
   own ~50-line `ambition_session_state` crate, or into
   `platformer_primitives::schedule` next to the schedule labels — states and
   labels are the same kind of vocabulary). This single move, plus schedule
   labels already being in primitives, frees host + touch_input from
   `ambition_actors` almost entirely** (host's remaining reads are dialog +
   camera_ease ticks — camera_ease is presentation-side time easing and can
   move to host/render; dialog is `ambition_dialog` already, repoint).
5. **`ambition_render` → `ambition_actors` (the E4 dep-flip blocker), now
   precisely enumerable:** rooms (11 — REPOINT to `ambition_world::rooms`,
   actors::rooms is a facade), features (9 — live ECS components; these are
   the true E4 stragglers: convert to SimView facts or repoint to combat),
   assets (8 — **`GameAssets` is an ASSET CATALOG living in the actor crate;
   move it to `ambition_asset_manager`/`ambition_sprite_sheet` side**),
   session (6 — messages like RespawnRoomVisualsRequested: move the message
   defs to a crate both can see, e.g. sim_view or world), dev (6 — debug
   overlay reads; gate behind dev_tools), portal (4 — the facade above),
   shrine/player/items/schedule (2 each — repoint/move-down leftovers).
   None of these is the hard identity work (that landed in E4 slices); they
   are moves + repoints.
6. **`ambition_items` contains `inventory_ui`** (deps ui_nav for
   `MenuFocusState`). The item MODEL and the inventory UI are different
   tiers. **Prescription: split inventory_ui out (menu-side or its own
   `ambition_inventory_ui`); items drops ui_nav.**
7. **`ambition_characters` → `ambition_input` for `ControlFrame`** — the
   two-port body means brains EMIT control frames, so the dep direction is
   defensible; but `ControlFrame` being input-crate vocabulary while
   `InputState` is engine_core vocabulary is a SPLIT-BRAIN worth one look
   when netcode lands (N-track): the brain-facing control vocabulary should
   probably live with the body contract (engine_core or primitives), making
   ambition_input purely a device-adapter crate. NOT urgent.
8. **`ambition_asset_manager` → `ambition_sfx`** — only for `SfxId` +
   `BankProvider` adapter. Acceptable today; if asset_manager is ever meant
   to be engine-generic, the sfx adapter is a feature-gated module. LOW.
9. **`ambition_runtime` → actors/combat/projectiles/etc.** — correct BY
   DESIGN (runtime is the composition tier).
10. **`ambition_host` → render + actors** — see 4; after GameMode/camera_ease
    move, host should dep only input/render/runtime/sim_view (its charter).
11. **`ambition_touch_input` → render** — touch draws its own overlay quads
    through render helpers; acceptable (it IS presentation), but then its
    name is wrong-tier: it is a presentation adapter, not an input crate.
    Optional rename/re-home under a `presentation/` grouping someday. LOW.
