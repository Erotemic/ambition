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

## 2026-06-10 check_doc_links.py was already red before Stage 20
- **Where:** docs/planning/player-ecs-bandaid-phase0.md, docs/planning/universal-brain-interface.md (engine_core/movement paths), dev/journals/lessons_learned.md (body_mode.rs -> body_mode/), docs/adr/0019 missing "## Current implications for agents" section
- **Smell:** 8 broken local links + 1 missing section predate tonight's run — they reference files deleted in earlier refactor waves (ae::Player deletion, engine_core crate extraction). The checker isn't in CI so drift accumulates.
- **Noticed while:** Stage 20 A3 (fixing the ~37 link breaks the bisection itself caused)
- **Suggested fix / size:** S — update the stale links (or mark those plan docs archived in stale_docs_index.md) and add check_doc_links to CI

## 2026-06-10 ItemKind/Item enums are content-flavored machinery
- **Where:** crates/ambition_sandbox/src/inventory (ItemKind), items (Item) — named variants (HealthPotion, DataChip, PortalGun...) baked into machinery enums
- **Smell:** the item ROSTER is named content but lives as machinery enums; true content/machinery split of items needs a data-driven item registry
- **Noticed while:** A1 menu equip inversion
- **Suggested fix / size:** L — item registry keyed by id, roster authored content-side

## 2026-06-10 EnemyConfig.archetype is the per-archetype tuning hub
- **Where:** crates/ambition_sandbox/src/features/ecs/enemy_clusters.rs:49 + ~25 method-projection reads across actors/damage/mount/save_sync
- **Smell:** the named EnemyArchetype enum is woven into the generic actor layer as its tuning provider; blocks moving the actor combat core into mechanics::combat
- **Noticed while:** A2 kit extraction
- **Suggested fix / size:** L — dissolve into capability components + tuning struct written at spawn (markers: CompositeSpawn, ChargeAttacker, respawn policy already exists)

## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit
- **Where:** crates/ambition_sandbox/src/mechanics/combat/events.rs (FeatureVisualKind)
- **Smell:** a named-ish variant in kit vocabulary (excluded from the combat-kit guard word list)
- **Noticed while:** A2 guard authoring
- **Suggested fix / size:** S — rename to TrainingDummy (touches LDtk/content mapping)

## 2026-06-10 BossAttackProfile brain enum carries named variants
- **Where:** crates/ambition_sandbox/src/brain/boss_pattern.rs (GnuHandSlam, GnuAppleRain, OverfitVolley, EyeBeam, MinimaTrap, SaddlePoint, GradientCascade...)
- **Smell:** the machinery brain enum names specific boss attacks; every new boss special grows a machinery enum. This forced boss attack_geometry to live in boss_encounter rather than the combat kit.
- **Noticed while:** A2 stretch (combat-kit guard rejected boss_attack_geometry)
- **Suggested fix / size:** M-L — replace variants with data-keyed attack profiles (string/interned id + spec struct), content registers specs

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

## Resolved

(none yet)
