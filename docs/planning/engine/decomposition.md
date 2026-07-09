# The decomposition playbook — killing the monolith, carve by carve

**Authored by fable 2026-07-05; compressed 2026-07-09 once Phase D-A
completed.** THE highest-priority engineering track (Jon, binding):
`ambition_actors` and the fat app decompose into the crate set of
[`architecture.md`](architecture.md) — referred to below BY ROLE (*[the sim
heart]*, *[the space IR]*) — so that (a) small agents can navigate and modify
any one domain safely, (b) content/demos plug in without touching core, (c)
hot-edit rebuilds shrink.

**Phase D-A is COMPLETE.** The per-carve task cards have been executed and
compressed away; their record is the execution log in
[`../tracks.md`](../tracks.md), and the post-carve verification is
[`fable-final-audit-2026-07-07.md`](fable-final-audit-2026-07-07.md). What
remains here is what still binds: the method rules, the anti-god rules, the
measured ledger, the open residue, D-B, D-C, and the exit criteria.

**Anchor style (evergreen):** cite `path` + SYMBOL, never line numbers. If a
named symbol has moved, `rg` for it; if it's gone, that's drift — update the
doc in the same commit (living-plan discipline), don't guess.

## Method rules (all carves)

- **Measure OUTWARD deps first.** "Names no content" ≠ "extractable"; a module
  with dozens of inbound mechanic deps stays until inversions land.
- **The D2 template:** kill cycles/misplacements INSIDE the crate first
  (compiling, committable steps), then ONE atomic move of the module to its
  crate, then repoint every consumer. Never a lasting facade; delete
  re-export shims in the same arc.
- **Test accounting (BINDING — fable final audit F7):** before the atomic
  move, list every `#[test]` in the moved modules **by name**; after the move,
  every name must exist somewhere. A test that can't run in the new crate gets
  its FIXTURE moved (the `cfg(test)` fixture-manifest pattern), never deleted
  — the W3 carve silently dropped four ruled contract tests this way.
- **Compile-parity gates:** after each carve, `cargo build -p ambition_app
  --features rl_sim` + the suite trio (`ambition_actors` lib, content, app
  rl_sim) + the architecture-boundary tests. **The content suite must run with
  `--features portal`** (CC6 discovery: the portal adapter tests are
  feature-gated and the bare `-p ambition_content` gate silently skips them).
- **Feature discipline:** `ambition_runtime` forwards
  `headless`+`input`+`portal_ldtk`; new crates declare features explicitly;
  never rely on unification accidents.
- **Record compile-time before/after** per carve (`cargo build -p <crate>
  --timings`) — rebuild speed is half the point.

## Anti-god-structure rules (BINDING on every executor)

The failure mode this playbook exists to prevent is re-centralization — an
agent "simplifying" by putting things in one place. These are hard rules;
violating them is wrong even when it compiles and reads cleaner to you:

1. **No `utils`/`common`/`shared`/`prelude`-dump crates, ever.** A type with
   no clear owner means the classification is unfinished — finish it
   (vocabulary moves DOWN to the crate that OWNS the domain; facts invert to
   parameters).
2. **Every moved module keeps/ships its OWN `Plugin`** registering its own
   systems/messages/resources. The runtime GROUP composes plugins; it never
   absorbs their registrations inline. A carve may leave a crate with no
   plugin (pure vocabulary) — never the reverse (a plugin registering another
   domain's systems).
3. **The `features/` hub facade DIES; it does not migrate.** No new re-export
   hub may be created in `ambition_actors`, the runtime, or anywhere else.
   Consumers import from the owning crate, explicitly. A "convenience" hub is
   the monolith's ghost.
4. **One-way doors:** a lower tier may NEVER import a higher one, and sibling
   domain crates may not import each other except along the arrows
   architecture.md draws. When you want a sideways import, you have found
   either (a) a vocabulary type that belongs a tier down, or (b) a fact that
   should be a parameter/message. There is no (c).
5. **Resources are owned.** A resource is defined + initialized in exactly one
   crate (its plugin); other crates read it via system params. Cross-crate
   `init_resource` of another domain's type is a review flag.
6. **When splitting, split by AUTHORITY (who mutates), not by theme.** "All
   the boss stuff together" is a theme; "the systems that mutate
   `BossPhaseState`" is an authority. Themes produce god crates.

---

## THE LEDGER — measured 2026-07-09 (supersedes the 2026-07-06 projection)

Every carve landed. Every carve also left an adapter shell behind, and the
2026-07-06 projection did not model that. **The old numbers said the residual
would bottom out at ≈31–35k and called it "the deliberate floor". It is
63.5k.** Only ~31k of the projected ~64k actually left `ambition_actors`.

`ambition_actors` src, by subdirectory:

| Subdir | LOC | Shape |
|---|---:|---|
| `features/` | 25.0k | the real actor domain (spawn/tick/perception/damage-routing/mount/bosses) + the surviving glue |
| `player/` | 6.6k | the last structural player-centrism; folds at S5/S6 |
| `boss_encounter/` | 5.5k | adapter residue after the E6 three-way split |
| `abilities/` | 4.1k | D-B carve candidate (`ambition_abilities`), iff measurement is clean |
| `character_sprites/` | 2.7k | actor/content join: animation pickers, authored hitbox resolution, catalog-aware loading |
| `world/` | 1.8k | overlay rebuild (reads live feature components) + the avian physics adapter |
| `projectile/` | 1.8k | the three woven steppers (charge input, victim routing, world collision) |
| `dev/` `items/` `encounter/` | 4.6k | sim-coupled adapters for their carved crates |
| rest | ~11k | time, session, body_mode, portal glue, gravity, roster, shrine, cutscene, assets tail |

Destination crates today: `engine_core` 17.5k, `characters` 17.0k, `combat`
9.5k, `render` 9.4k, `portal_presentation` 6.5k, `sprite_sheet` 6.0k, `portal`
5.3k, `ldtk_map` 5.0k, `primitives` 4.1k, `asset_manager` 4.0k, `persistence`
3.7k, `world` 2.9k, `audio` 2.9k, `sim_view` 2.8k, `menu` 2.4k; nothing else
exceeds ~2.3k. `game/ambition_app` is 20.7k (its `menu/` stayed app-side by the
E1e ruling — the host stack + grid backend couple up to items/player/sfx).

**Open question for the next structural session:** is the adapter floor THE
floor (in which case re-baseline the ledger and say so), or is there a real
carve left in `features/` 25k? Do not pre-commit — re-measure (U1).

**Efficiency (why the split costs the game nothing):** crate boundaries are
COMPILE-TIME structure — the same systems run in the same schedule (E5's carve
was byte-parity-gated, the precedent). Rust inlines generics across crates;
the kernels already live in `engine_core`; thin-LTO is the lever if a boundary
ever shows in a profile. The costs that DO exist are paid deliberately: the E4
read-model copies view facts once per tick (bought: netcode/RL/render
decoupling — Q32), and the win is INCREMENTAL COMPILE.

### Why these pieces are THE pieces (the elegance argument)

The crate boundaries follow the four real fault lines in the domain, not line
counts: (1) **vocabulary vs. simulation** — schemas/registries/formats
(`entity_catalog`, `sprite_sheet`, `characters`) sit below the systems that
step them (`actors`, `combat`), so content and tools can depend on vocabulary
without dragging the sim; (2) **sim vs. space** — the world IR
(`ambition_world`) is authored INPUT to the sim, never a peer
(backend-agnostic by construction, which is what makes Tiled/Godot importers
additive); (3) **sim vs. observation** — `ambition_sim_view` is the one-way
read-model boundary (render, netcode confirmation, RL observation, and the
slower-light shaders are all THE SAME KIND of consumer); (4) **engine vs. host
vs. content** — `ambition_runtime` (headless sim assembly) / `ambition_host`
(windowed wiring) / content crates (named worlds+rosters+rules). Every demo
and the game compose from exactly these five faces.

### The demo/game → crate support matrix (the proof of sufficiency)

| Consumer | Exercises beyond the shared core |
|---|---|
| Sanic | momentum kernel (`engine_core::surface`), `ambition_world` chains channel, mode-scope seam |
| Super Mary-O | `ambition_items` equipment policies, camera policy knobs, cutscene kit |
| Super Smash Siblings | `ambition_combat` CM stack, N1 slot routing (`ambition_host`), fighter brain (`ambition_characters`), `ambition_sim_view` damage-meter read |
| Hollow Lite | boss pipeline (characters + encounter + combat), `ambition_persistence` (benches), respawn policy (actors) |
| Ambition itself | ALL of the above + portals, dialog, menu, audio, falling-sand content plugin — and hosts each demo via mode scopes |

Shared core in every column: runtime + host + actors + combat + world +
sim_view + characters + entity_catalog + input. If a demo needs a crate edit
outside its column's expectation, that's the oracle firing.

---

## Phase D-A — DONE

E1a–e (persistence, audio, dialog, dev_tools, settings-IR + the first
extension crate), E2 (combat kit + projectile model), E3 (sprite-sheet
absorb), E4 (the observation boundary + `ambition_sim_view`), E5 (the sim
assembly + `ambition_host` — **the demo gate**), E6 (boss tail), E7 (rename +
workspace re-home + facade dissolution), E8 (items), E9 (the `ambition`
umbrella + demo crate homes), W1–W4 (the world/LDtk split, `PlacementRecord`,
the lowering registry, ADR 0021), and the F1–F9 audit queue.

### The E4 contract (permanent, governs all future presentation work)

The carve relocated and SEALED the view types; the rules it fixed still bind:

- Extraction systems are FUNCTIONS of sim state, running LAST in the sim
  schedule. Presentation reads ONLY `SimView`. Plain data — no `Entity`
  borrows beyond opaque ids, no `Handle<T>`, no interior mutability — so
  netcode N3.1 serializes it for free.
- **View rows key by the same stable-id vocabulary the snapshot registry
  uses** (actor `config.id`, player slots, deterministic spawn ids): one
  identity system, two consumers. Render maps its presentation entities off
  those ids, never off sim `Entity` values.
- `PresentationFact` is the ONE event channel presentation consumes; its dedup
  identity is the triple `(tick, source SimId, kind)` — that is what resim
  suppression keys on.
- Render-spawned helper props are PRESENTATION CACHES keyed off view rows
  (despawn/respawn freely; never readable by sim). Anything the sim reads must
  be a sim fact. **A render-inserted component the sim queries (the old
  `BossAnimator` shape) is the boundary violation this carve exists to kill**
  — `ambition_render/tests/observation_boundary.rs` enforces it.

### Open residue (small, enumerated, not blocking)

- **E6 deferred teardown:** the fused `gnu_ton` profile +
  `sync_boss_split_overlay` + `BossOverlayLayer` + split z-consts. Retarget
  the referencing tests to the linked-pair arena first.
- **E7 named-content residue:** `dialog/speech_sfx.rs`'s voice table wants a
  content voice-profile registry; the `StartingCharacter` worn-sheet residue
  (`PLAYER_CHARACTER_ID` / `PLAYER_FILE_ROOT` in
  `character_sprites/attack_hitbox.rs`).
- **E-assets tail:** `assets/game_assets` still names gameplay/presentation
  vocabulary; it shrinks with the actor/presentation carves, never by
  reintroducing asset-manager upward deps.
- **E-enc adapters stay by authority:** LDtk loading, the live encounter tick,
  lock-wall contribution, switch-index rebuild, and
  `features/ecs/encounter_rewards.rs` spawn mobs/chests and write
  save/quest/banner state.
- **`world/overlay{,_rebuild}.rs`** join `ambition_world` once the rebuild's
  inputs become plain solids; `world/physics.rs` (debris/avian) is
  presentation-adjacent and can join the render/host side whenever.

---

## Phase D-B — the post-carve `ambition_actors` and the navigability standard

- **Re-measure before further splits** (U1 stands). The likely-clean further
  carve if measurement supports it: the traversal-ability kit
  (blink/dive/grapple/possession) — it reads the controlled-subject seam and
  kinematics, not the spawn machinery. Do NOT pre-commit.
- **The navigability standard applies INSIDE the crate** — this is where
  "agents can work cleanly" is actually won, and it applies to every engine
  crate. Status:
  - every module ≤ ~1.5k lines, each with a header stating its ONE concern,
    its authoritative state, and its seams — ✅ **holds today** (no module in
    `ambition_actors` exceeds 1.5k);
  - `features/mod.rs` hub-glob patterns dissolved into explicit imports — ✅
    **done**;
  - the schedule vocabulary documented in one place;
  - **a `MODULES.md` map at each crate root — ❌ NOT DONE in any crate.**
    Maintained by the same rule as TODO discipline. Mechanical [sonnet].

## Phase D-C — the demo-hosting seam (ambition runs the demos)

**NOT STARTED.** Vision §5 forces one more decomposition artifact — the
**scoped game-mode pattern**: a demo's rules crate exposes `<Demo>RulesPlugin`
whose systems are gated on an area/room tag, not on global state. Design
detail in [`../demos/README.md`](../demos/README.md); the engine-side slice is
the room-scoped run-condition helper + the mode field. Pre-solved:

```rust
// ambition_world (RoomMetadata):   pub mode: Option<String>,  // merge: first Some wins
// ambition_runtime:
pub fn in_mode(name: &'static str) -> impl Condition { /* reads ActiveRoomMetadata */ }
#[derive(Component)] pub struct ModeScopedEntity(pub String); // despawned when the mode deactivates
```

A rules crate attaches every system `.run_if(in_mode("sanic"))` when hosted,
or unconditionally when standalone — the APP chooses via
`SanicRulesPlugin::hosted()` vs `::global()` (a constructor flag, not two
plugins). Mode resources live on a mode-owner entity carrying
`ModeScopedEntity`, so zone exit cleans up by the same sweep
`RoomScopedEntity` uses (generalize that sweep, don't duplicate it).

## Exit criteria (the whole playbook)

1. ✅ The monolith no longer exists (renamed residue included); every crate in
   architecture.md's stack is real with imports flowing downward, enforced by
   `game/ambition_app/tests/architecture_boundaries.rs`.
2. ✅ The named-content grep over engine crates hits zero (test fixtures
   allowed only under `cfg(test)`; the one `include_str!` is the sanctioned
   fixture pattern).
3. ⏳ A demo app builds from runtime+host groups + its content crate with zero
   engine edits (the oracle, executable). The umbrella + demo crates exist and
   the AUTHORING oracle held; the remaining half is a demo binary, which fable
   ruled interactive work (feel cannot ship headless).
4. ✅ Workspace green: `cargo test --workspace --all-targets --features
   rl_sim` — 44/44 suites, zero failures as of 2026-07-09.
5. ❌ **Cannot be met as written.** It requires a hot-path incremental rebuild
   "measurably below the monolith baseline recorded before D-A" — that
   baseline was never recorded. Reconstruct it (check out a pre-D-A commit and
   time it) or rewrite the criterion.
