# Code smell backlog

Running log of smells noticed *opportunistically* while doing other work (Jon's
standing instruction, 2026-06-10). The rule: while focused on a big task, don't chase
smells — append them here so they aren't forgotten, and revisit later. Only fix inline
when the fix is very clear AND carries no risk of slowing the main task.

Append-only during runs; triage/prune during cleanup passes (move fixed items to the
Resolved section with the fixing commit).

Entry format:

```
## YYYY-MM-DD <short title>
- **Where:** file:line (or module)
- **Smell:** what's wrong, one or two sentences
- **Noticed while:** the task being worked
- **Suggested fix / size:** sketch + rough effort (S/M/L)
```

---

## Open

## 2026-06-13 Docs reference deleted RON-based levels
- **Where:** docs/adr/0009 (the RON-room archaeology — fixed in this pass); likely other docs, since 0009's own "Consequences" implies RON-world-authoring docs still exist unswept
- **Smell:** RON-shaped room/world levels were fully removed (LDtk is the only world source), but docs still describe them as if extant or as a live alternative. Jon's standing rule: a doc describing something that no longer exists is a smell — log it.
- **Noticed while:** updating ADR 0009 during the Technique/Effects architecture discussion
- **Suggested fix / size:** S — grep docs for old room-data / "RON room|world|manifest|level" mentions, archive or rewrite; consider a check_doc_links-style guard so doc drift fails loudly

## 2026-06-10 check_doc_links.py was already red before Stage 20
- **Where:** docs/planning/universal-brain-interface.md (engine_core/movement paths), dev/journals/lessons_learned.md (body_mode.rs -> body_mode/), docs/adr/0019 missing "## Current implications for agents" section. (player-ecs-bandaid-phase0.md has since been deleted, so its breaks are moot.)
- **Smell:** broken local links + 1 missing section predate Stage 20 — they reference files deleted in earlier refactor waves (ae::Player deletion, engine_core crate extraction). The checker isn't in CI so drift accumulates.
- **Noticed while:** Stage 20 A3 (fixing the ~37 link breaks the bisection itself caused)
- **Suggested fix / size:** S — update the stale links (or mark those plan docs archived in stale_docs_index.md) and add check_doc_links to CI

## 2026-06-10 ItemKind/Item enums are content-flavored machinery
- **Where:** crates/ambition_sandbox/src/inventory (ItemKind), items (Item) — named variants (HealthPotion, DataChip, PortalGun...) baked into machinery enums
- **Smell:** the item ROSTER is named content but lives as machinery enums; true content/machinery split of items needs a data-driven item registry
- **Noticed while:** A1 menu equip inversion
- **Suggested fix / size:** L — item registry keyed by id, roster authored content-side

- **UPDATE 2026-06-15 (long run):** the legacy `ItemKind`/`PlayerInventory` half is DELETED (collapsed onto `OwnedItems`/`Item`); the dual-bag smell is resolved. The remaining `Item` 24-variant enum is INTENTIONALLY kept — it has type-level equip/ability/held-item wiring (match arms), and the decision doc prefers a NARROW closed enum over a wide data-keyed registry for that. The `ItemMeta` table is already data. Verdict: leave `Item` as the enum; B4 (data-key it) is anti-elegant. Won't-do-by-analysis.
## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit
- **Where:** crates/ambition_sandbox/src/mechanics/combat/events.rs (FeatureVisualKind)
- **Smell:** a named-ish variant in kit vocabulary (excluded from the combat-kit guard word list)
- **Noticed while:** A2 guard authoring
- **Suggested fix / size:** S — rename to TrainingDummy (touches LDtk/content mapping)

## 2026-06-10 BossAttackProfile brain enum carries named variants
- **Where:** crates/ambition_actor/src/brain/boss_pattern.rs (HandSlam, HeadDescent, MemorizedVolley, LockOnBeam, PitTrap, RotatingCross, MinionCascade...) — moved here from ambition_sandbox in Stage 20
- **Smell:** the foundation brain enum names specific boss attacks; every new boss special grows a foundation enum. This forced boss attack_geometry to live in boss_encounter rather than the combat kit.
- **Noticed while:** A2 stretch (combat-kit guard rejected boss_attack_geometry)
- **Suggested fix / size:** M-L — replace variants with data-keyed attack profiles (string/interned id + spec struct), content registers specs. NOW the active target of the Technique/Effects framework design (2026-06-13).

- **UPDATE 2026-06-15 (long run):** analysis says LEAVE the enum. The named melee variants (FloorSlam ×18, SideSweep ×35, HandSlam ×12, HazardColumn ×10, …) are SHARED attack-shape vocabulary authored across MANY bosses in `boss_profiles.ron`, not per-boss content — a narrow closed enum is correct engine vocabulary (decision doc: narrow > wide). The content-specific specials already route through the `Special(String)` seam, and their consumers (`spawn_gnu_apple_rain_*` etc.) ALREADY live in `ambition_content::bosses::specials` (C3 done). Won't-do-by-analysis.
## 2026-06-10 audio/music runtime interleaves game reads with playback machinery
- **Where:** crates/ambition_sandbox/src/audio/runtime.rs (apply_encounter_music reads EncounterMusicRequest/RoomMusicRequest inline), music/mod.rs (UserSettings reads), environment.rs (player position reads)
- **Smell:** doc 20's B1 ("ambition_audio is the cleanest warm-up") underestimates this: the playback engine and the game-event adapters are item-level interleaved in the same files, so the crate extraction is real surgery (~4 seams: AudioMixSettings sync, AudioSpec resource, request->MusicIntent (exists), bank/catalog glue). Post-bisection the compile-time payoff also shrank (audio already left the content-edit hot path).
- **Noticed while:** Stage 20 overflow triage (B1 deprioritized in favor of C1 per Jon's task pick)
- **Suggested fix / size:** M — split runtime.rs/mod.rs item-by-item along the seams above, then the crate move is mechanical

- **UPDATE 2026-06-15 (long run):** the authored music-cue catalog (the goblin adaptive tune + binding) moved to `ambition_content::music` (the lib's audio plugin no longer hard-codes it; the content plugin installs the `MusicCueCatalog`, which the director already takes as `Option<Res<_>>`). What remains (`audio/runtime.rs`, 89L) is thin game-glue translating `EncounterMusicRequest`/`RoomMusicRequest` → playback; the generic half is already `ambition_audio`, settings already decouple via `MusicMix`. No crate to extract — E1/E2 are effectively addressed.
## 2026-06-10 Boss sprite assets are named GameAssets fields + per-boss loader fns
- **Where:** crates/ambition_sandbox/src/assets/game_assets.rs (mockingbird/gnu_ton/gnu_ton_body/gnu_ton_hands/smirking/spaghetti/trex fields), boss_encounter/sprites.rs (load_<boss>_sprite_in wrappers + named sheet consts), presentation/rendering/actors.rs ~695-760 (the per-boss if-chain incl. the GNU-ton body+hands layered render)
- **Smell:** the last named-content pocket in the render path. Inversion sketch: `boss_sprites: HashMap<String, BossSpriteAsset>` + `boss_layers: HashMap<String, BossLayeredSprite>` keyed by behavior id, loaded from a data table (boss id -> sheet asset ids + layer split), with the if-chain becoming `assets.boss_sprite(boss_key).or(generic)`. Land AFTER the sheet-data migration is runtime-verified (the layered GNU-ton render is the most visually delicate path in the repo).
- **Noticed while:** B3 session 2 (presentation de-naming)
- **Suggested fix / size:** M (~1h) — same registry/data pattern as the character sheets
- **RESOLVED 2026-06-15** (long run): the 7 named `GameAssets` boss fields collapsed to one `boss_sprites: HashMap<&str, BossSpriteAsset>` keyed by `boss_key`, the per-boss render if-else chain collapsed to `assets.boss_sprite(&boss_key).or(boss)`, and the 7 loader fns collapsed to a `dedicated_boss_sheets()` `(key, spec)` data table looped at load. The machinery's public asset struct + renderer name NO boss; the names live only in the one data table. Behavior-preserving (replay bit-identical). GNU-ton's delicate layered render logic is untouched (reads suffixed keys). The remaining bit (the boss->sheet ASSIGNMENT table itself moving to content) is deferred — it needs the sheet-data/catalog migration.

## 2026-06-10 Special-attack EFFECTS consumers are half-vocabulary (post de-name)
- **Where:** crates/ambition_sandbox/src/features/ecs/brain_effects.rs (spawn_gnu_apple_rain_from_special_messages, spawn_overfit_volley_from_special_messages, the LockOnBeam/PitTrap/RotatingCross/MinionCascade consumers); SpecialActionSpec doc comments in ambition_actor/src/brain/action_set.rs
- **Smell:** the BossAttackProfile de-name (3e344d95) is honest at the key/schedule/geometry layers and at the spec-PARAMETER layer (DebrisRain{interval_s,spawn_speed,damage} etc. are RON-authored per boss), but the consumer implementations still bake content: apple art identity, "constants reused from the spec but baked here", gnu-named fns, spec docs claiming "GNU-ton boss:". Inconsistent grep threads (Jon's review caught this).
- **Fix sketch / size:** M — lift the baked constants + projectile-art identity into the spec fields (RON), rename consumers to the vocabulary with a "first authored by gnu_ton" breadcrumb. Verdict + discussion: doc 22 session 3.
- **Noticed while:** Stage 22 boss_encounter core move (Jon pushed back on the rename)
- **UPDATE 2026-06-13:** the original "no generic effect-composition framework until a second boss needs one" verdict is **superseded** — the engine-for-other-games north star greenlit a generalized `ambition_effects` crate (Technique → Effect → Combat layering). This smell + the BossAttackProfile enum above are now the active targets of that design.

## 2026-06-15 Dual inventory bags — `PlayerInventory`/`ItemKind` (3) shadows `OwnedItems`/`Item` (24)

- **Where:** `crates/ambition_sandbox/src/inventory/model.rs` (`ItemKind` 3-variant enum + `PlayerInventory` `[u32;3]` bag); `crates/ambition_sandbox/src/items/mod.rs` (`Item` 24-row table catalog + `OwnedItems` `[u32;ITEM_COUNT]` bag + the `legacy_kind`/`from_legacy_kind` bridge + the `ItemMeta.legacy_kind` column); consumers `dialog/yarn_bindings.rs` (`apply_give_item` grants into `PlayerInventory`; the snapshot reads `OwnedItems` then falls back to `PlayerInventory`) and `crates/ambition_content/src/quest.rs` (`PIRATE_TREASURE_REWARD: &[(ItemKind, u32)]` grants into `PlayerInventory`).
- **Smell:** two parallel item-count bags keyed by two parallel item enums for the same player. `Item`/`OwnedItems` is the real 24-item catalog (table-driven, `dialog_id`, sprites); `ItemKind`/`PlayerInventory` is a legacy 3-kind subset bridged by `legacy_kind`. Every grant/query path has to pick a bag, and the yarn snapshot writes BOTH dialog ids per item to keep them reconciled. A new item can only be granted-by-dialogue if it exists in the legacy bag.
- **Fix sketch / size:** M (~1-2h) — delete `ItemKind` + `PlayerInventory` + the `legacy_kind` column/methods; key the single bag on `Item` (it's `Copy+Eq+Hash` with explicit 0..23 discriminants, so `OwnedItems` already IS that bag). Repoint `apply_give_item` -> `OwnedItems` + `Item::from_dialog_id` (already accepts the legacy ids), and `content::quest::PIRATE_TREASURE_REWARD` -> `&[(Item, u32)]`. Drop the snapshot's dual-write fallback. Touches the content crate + a handful of tests; behaviour-equivalent for the 3 overlapping items, and UNLOCKS dialogue-granting any of the 24. **Deferred from the 2026-06-15 window** (spans the content crate + dual-bag tests; wanted a clean uninterrupted block).
- **Noticed while:** 2026-06-15 monolith-breakup run, scoping "named content (ItemKind) out of machinery".
- **RESOLVED 2026-06-15** (same run): deleted `ItemKind`+`PlayerInventory`+the `legacy_kind` bridge; collapsed onto `OwnedItems`/`Item`. The lone divergent alias (`healthpotion`→`HealthCell`) survives as `Item::legacy_dialog_alias()`. Dialogue can now grant any of the 24 items. Pinned by `legacy_health_alias_resolves_to_health_cell` + the migrated yarn/quest tests.

## Resolved

## 2026-06-14 Cube edge-button (page-turn) handling was duplicated per face — RESOLVED 2026-06-14
- **Was:** `kaleidoscope_focus_nav`'s placeholder Map/Quest branch and `system_focus_nav` each hand-rolled "cursor on a `>`/`<` edge → step inward / rotate / select-rotate". They drifted: SELECT-on-edge was simply MISSING on the System face (fell through to the row dispatch with `current` normalised to `rows[0]`, so selecting `>Quest` activated the first System row). Jon: "Why is system being treated in such a special way?"
- **Resolved:** extracted a single `edge_button_nav(cursor, pages, active_page, dx, select, allow_page_turn, inward: EdgeInward, sfx) -> EdgeNav`, consumed by BOTH callers. The only per-face difference (where an INWARD step lands) is the `EdgeInward` param — `OppositeEdge` for placeholders, `Into(System(0))` for the row list. Each caller now owns only its centre content. Pinned by `system_edge_outward_arrow_rotates`, `placeholder_edge_nav_matches_other_faces`, `select_on_system_edge_button_turns_the_page` + the existing inward/bumper tests.

## 2026-06-10 EnemyConfig.archetype is the per-archetype tuning hub — RESOLVED 2026-06-13
- **Was:** the named `EnemyArchetype` enum woven into the generic actor layer as its tuning provider; blocked moving the actor combat core into mechanics::combat.
- **Resolved across three steps:** (1) sim-side reads projected to `EnemyTuning` + `CombatCapabilities` (`6dc440b9`, 2026-06-11); (2) the durable `EnemyConfig` + per-frame `EnemyMut` made archetype-free via a new `EnemyBrainSpec`, enum confined to the spawn-time seed (Session 6, 2026-06-13); (3) the whole roster lifted to `ambition_content` (`crates/ambition_content/src/enemy_roster.rs`) and the `EnemyArchetype` enum **deleted** — enemies now resolve by brain-key against an installed `EnemyRoster` holder. Guarded by `architecture_boundaries_enemy_config_is_archetype_free`. The lib names no enemy archetype.
