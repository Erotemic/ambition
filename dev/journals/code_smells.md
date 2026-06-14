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

## 2026-06-14 Cube edge-button (page-turn) handling is duplicated + divergent per face
- **Where:** crates/ambition_app/src/menu/kaleidoscope_app.rs — `kaleidoscope_focus_nav` (the placeholder Map/Quest branch, ~line 985-1030) vs `system_focus_nav` (~line 1210-1300). Both implement "cursor on a `>`/`<` edge button → step inward / rotate" and "SELECT on an edge → rotate", but separately.
- **Smell:** the System face re-implements edge-button nav by hand and drifted from the generic face handler. The `dx`-outward turn was already special (raw `turn_page` + `cursor = System(0)`), and SELECT-on-edge was simply MISSING — it fell through to the row dispatch with `current` normalised to `rows[0]`, so selecting `>Quest` activated the first System row. Jon: "Why is system being treated in such a special way? That is a smell." Two point-fixes landed (turn_page_seeded for the cursor landing; an explicit edge-select arm), but the duplication remains: any future edge-nav change must be made in two places.
- **Noticed while:** menu polish (select-on-edge bug + cursor-landing bug, 2026-06-14)
- **Suggested fix / size:** M — extract a shared `edge_button_nav(cursor, active_page, dx, select, allow_page_turn, pages, sfx) -> Handled` helper consumed by BOTH the placeholder branch and `system_focus_nav`, so edge → inward/rotate/select is single-source. The System branch then only owns ROW logic. Guard with the new `select_on_system_edge_button_turns_the_page` + the bumper tests.

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

## 2026-06-10 audio/music runtime interleaves game reads with playback machinery
- **Where:** crates/ambition_sandbox/src/audio/runtime.rs (apply_encounter_music reads EncounterMusicRequest/RoomMusicRequest inline), music/mod.rs (UserSettings reads), environment.rs (player position reads)
- **Smell:** doc 20's B1 ("ambition_audio is the cleanest warm-up") underestimates this: the playback engine and the game-event adapters are item-level interleaved in the same files, so the crate extraction is real surgery (~4 seams: AudioMixSettings sync, AudioSpec resource, request->MusicIntent (exists), bank/catalog glue). Post-bisection the compile-time payoff also shrank (audio already left the content-edit hot path).
- **Noticed while:** Stage 20 overflow triage (B1 deprioritized in favor of C1 per Jon's task pick)
- **Suggested fix / size:** M — split runtime.rs/mod.rs item-by-item along the seams above, then the crate move is mechanical

## 2026-06-10 Boss sprite assets are named GameAssets fields + per-boss loader fns
- **Where:** crates/ambition_sandbox/src/assets/game_assets.rs (mockingbird/gnu_ton/gnu_ton_body/gnu_ton_hands/smirking/spaghetti/trex fields), boss_encounter/sprites.rs (load_<boss>_sprite_in wrappers + named sheet consts), presentation/rendering/actors.rs ~695-760 (the per-boss if-chain incl. the GNU-ton body+hands layered render)
- **Smell:** the last named-content pocket in the render path. Inversion sketch: `boss_sprites: HashMap<String, BossSpriteAsset>` + `boss_layers: HashMap<String, BossLayeredSprite>` keyed by behavior id, loaded from a data table (boss id -> sheet asset ids + layer split), with the if-chain becoming `assets.boss_sprite(boss_key).or(generic)`. Land AFTER the sheet-data migration is runtime-verified (the layered GNU-ton render is the most visually delicate path in the repo).
- **Noticed while:** B3 session 2 (presentation de-naming)
- **Suggested fix / size:** M (~1h) — same registry/data pattern as the character sheets

## 2026-06-10 Special-attack EFFECTS consumers are half-vocabulary (post de-name)
- **Where:** crates/ambition_sandbox/src/features/ecs/brain_effects.rs (spawn_gnu_apple_rain_from_special_messages, spawn_overfit_volley_from_special_messages, the LockOnBeam/PitTrap/RotatingCross/MinionCascade consumers); SpecialActionSpec doc comments in ambition_actor/src/brain/action_set.rs
- **Smell:** the BossAttackProfile de-name (3e344d95) is honest at the key/schedule/geometry layers and at the spec-PARAMETER layer (DebrisRain{interval_s,spawn_speed,damage} etc. are RON-authored per boss), but the consumer implementations still bake content: apple art identity, "constants reused from the spec but baked here", gnu-named fns, spec docs claiming "GNU-ton boss:". Inconsistent grep threads (Jon's review caught this).
- **Fix sketch / size:** M — lift the baked constants + projectile-art identity into the spec fields (RON), rename consumers to the vocabulary with a "first authored by gnu_ton" breadcrumb. Verdict + discussion: doc 22 session 3.
- **Noticed while:** Stage 22 boss_encounter core move (Jon pushed back on the rename)
- **UPDATE 2026-06-13:** the original "no generic effect-composition framework until a second boss needs one" verdict is **superseded** — the engine-for-other-games north star greenlit a generalized `ambition_effects` crate (Technique → Effect → Combat layering). This smell + the BossAttackProfile enum above are now the active targets of that design.

## 2026-06-11 Portal body transit has no exit push-out (NPCs stick in the exit wall)
- **Where:** placement::transfer_step / transit.rs ~412
- **Smell:** a transited body is placed at `pp::map_point(centroid)` — right at the exit FACE, with no push-out along the exit normal (the comment calls this deliberate: reversibility + "emerges right at the face"). The player clears the wall over the next frames via the exit-normal MIN_EXIT_SPEED floor + carve + collision. But GROUND ITEMS get an explicit `pos = exit.pos + exit.normal * portal_exit_clearance(...)` — bodies do NOT, so a body with low exit velocity or different collision resolution (the kernel NPC, "gets stuck in the wall sometimes when you portal him around") can emerge embedded and never get pushed clear.
- **Noticed while:** portal body-transit review
- **Suggested fix / size:** M — clamp body exit depth to at least `portal_exit_clearance` along the exit normal (preserve the along-surface offset), mirroring ground items. RISK: changes core transit placement for the player too and touches the "reversibility" property — own focused slice through replay_fixture_regression / scripted_gameplay, not folded into a visuals change. (Note the avoid-pushout rule: this is the sanctioned straddle-eviction exception.)

## Resolved

## 2026-06-10 EnemyConfig.archetype is the per-archetype tuning hub — RESOLVED 2026-06-13
- **Was:** the named `EnemyArchetype` enum woven into the generic actor layer as its tuning provider; blocked moving the actor combat core into mechanics::combat.
- **Resolved across three steps:** (1) sim-side reads projected to `EnemyTuning` + `CombatCapabilities` (`6dc440b9`, 2026-06-11); (2) the durable `EnemyConfig` + per-frame `EnemyMut` made archetype-free via a new `EnemyBrainSpec`, enum confined to the spawn-time seed (Session 6, 2026-06-13); (3) the whole roster lifted to `ambition_content` (`crates/ambition_content/src/enemy_roster.rs`) and the `EnemyArchetype` enum **deleted** — enemies now resolve by brain-key against an installed `EnemyRoster` holder. Guarded by `architecture_boundaries_enemy_config_is_archetype_free`. The lib names no enemy archetype.
