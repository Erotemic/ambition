# Gameplay idea index

Status: working index for story-progress planning. Use this to keep ideas tied
to concrete Ambition gameplay hooks before promoting them into systems, recipes,
LDtk rooms, or code TODOs.

## How to use this index

- Add ideas here when they are still rough but should be findable.
- Keep each idea mapped to an existing hook or an explicit missing hook.
- Promote implemented ideas to system docs, room recipes, or source comments.
- Archive ideas only when they are misleading or superseded.

## Existing playable hooks worth reusing

| Hook | Current source of truth | Good story use |
|---|---|---|
| LDtk rooms/loading zones | `assets/ambition/worlds/*.ldtk` | Gates, ripples, faction doors, route choice |
| NPC dialogue ids | `src/dialog/content.rs`, `src/intro/dialog.rs` | Quest givers, story reframes, faction voice |
| Quest specs | `src/content/quest.rs` | Multi-step progression, courier chains, boss return loops |
| Save flags | `crates/ambition_sandbox/src/persistence/save.rs`, `crates/ambition_sandbox/src/persistence/` | Durable choices, observed/tampered messages, post-boss state |
| Encounters | `src/encounter/` | Room lockdowns, combat labs, escort/interceptor tests |
| Boss profiles | `src/boss_encounter/` | Story bosses that teach pattern reading |
| Switches/lock walls | `src/encounter/switches.rs`, `lock_walls.rs` | Protocol puzzles, doors, commit/reveal beats |
| Projectiles/damage volumes | `src/projectile/`, `src/enemy_projectile/`, LDtk entities | Interceptors, signals, hazards, bullet patterns |
| Blink/soft walls | `docs/mechanics/blink.md`, entity sprites | Ripple traversal, key-gated movement |
| Body modes | `docs/mechanics/body-modes.md` | Crawl/morph identity, fitting through unofficial cracks |
| Generated music/SFX | `docs/recipes/generated-music-workflow.md`, audio tools | Arc motifs, faction stingers, ripple identity |
| Generated sprites | `tools/ambition_sprite2d_renderer/` | Alice/Bob placeholders, factions, boss silhouettes |

## Intro and hub ideas

| Idea | Gameplay form | Current hook | Missing hook / asset |
|---|---|---|---|
| Embodiment before exposition | Wake room movement before long dialogue | `intro_wake_room`, Creator dialogue | Optional `wake_room_exited` flag |
| Speed affects final words | Fast clear gives extra Creator fragment | CreatorFinalFast/Impossible dialogue variants exist | Reliable timer/route selector |
| Wrong-list incident | Kiosk/newsboard reframe the raid | Manifest Clerk, News Board | Maybe a quest log entry after kiosk |
| Stable gates vs ripples | Large official portal plus small unofficial crack | Gate sprites, portal switch, loading zone | Reusable `Ripple` entity |
| Kernel as playable index | Hub/basement doors teach mechanics | central hub and basement labs | Intentional story gating/order |
| Debug as sense organ | Player sees labels/hitboxes as perception | Architect dialogue/debug overlays | Diegetic debug UI polish |

## Alice/Bob crypto-gang ideas

| Idea | Gameplay form | Current hook | Missing hook / asset |
|---|---|---|---|
| Handshake quest | Alice challenge -> Bob response -> Alice verify | Quest steps, NPCs, flags | Alice/Bob sprites, flag-setting dialogue |
| Eve observes | Interceptor touches message and records `observed` | Damage/projectile triggers as placeholder | Non-damaging contact trigger, dialogue branching |
| Mallory modifies | Fast route tampers, slow route verifies | Switches, lock walls, route flags | Route-consequence UI |
| Key exchange movement | Challenge-response opens blink/door/platform | Blink walls, switches, moving platforms | Protocol-themed room props |
| Love-story route | Trust preserved or damaged over repeated quests | Save flags and quest state | General conditional dialogue variants |
| Public/private key doors | Public switch visible; private response opens route | Switch + lock wall | Key-state display |
| One-time pad traversal | One safe pass through a hazard field | Pickups/flags/damage volumes | Consumable quest item or temporary shield |
| Commit-reveal platform | Commit on switch, reveal before timer expires | Switches, moving platforms | Timer display / route feedback |

## Faction and boss ideas

| Idea | Gameplay form | Current hook | Missing hook / asset |
|---|---|---|---|
| Pirates and ninjas share a target | Two entrances to Mockingbird arena | Pirate Cove, Ninja Dojo, Mockingbird | Post-boss route consequences beyond cove |
| Mockingbird steals voice/song/kata | Boss mimics audio/attack tells | Existing boss and dialogue | Stronger mimic audio/visual motif |
| Clockwork Warden as first proof boss | Simple pattern-reading fight | `basement_boss` | Story reward tied to first_steps |
| GNU-ton as optional scholar boss | Heavy giant fight, Newton flavor | `gnu_ton_arena` | More Newton-forward barks/tune polish |
| Military tower as authoritarian temptation | Easier official route, narrower choices | Military Tower + General | Choice consequences, UI changes |
| Tech-bros basement as exploit economy | Downward slope/platform pitch room | Tech Bros Basement | Consequence flags, funding provenance |
| Goblin cantina as movement rehearsal | Tables, low ceilings, chaotic nonlethal test | Goblin Cantina | Traversal props / tavern tune |
| Pulse Voyager as timing area | Beat stones / time-as-position | Pulse Voyager Outpost | Moving platforms synced to music |

## Missing gameplay primitives to consider

These are not prerequisites for the next slice, but they are likely to become
useful if the story draft continues in this direction.

| Primitive | Why it matters | Placeholder now |
|---|---|---|
| Flag-setting dialogue choice | Alice/Bob and route decisions need it | Use room/switch flags |
| Conditional dialogue variants | Save-state consequences should be visible | One-off redirect pattern |
| Quest payload/item text | Courier quests need inspectable messages | Debug labels and quest log text |
| Ripple entity | Small AI-only cracks should feel distinct | Loading zone plus portal sprite |
| Non-damaging trigger volume | Observation/tamper should not always hurt | DamageVolume with low/no damage or switch |
| Ability gating policy | Story mode needs unlocks; sandbox needs all verbs | Flags first, enforcement later |
| Music stem request per story arc | Rooms should feel connected by motif | Existing room/encounter music requests |

## Development order recommendation

1. Build Alice/Bob handshake with placeholder flags and two tiny rooms.
2. Add one reusable flag-setting dialogue/event hook only if the placeholder is
   too awkward; otherwise defer.
3. Add a handshake motif and rough sprites.
4. Use the quest to unlock or visually justify a ripple route back to The
   Kernel.
5. Only then add Eve/Mallory variants, because they need conditional dialogue
   and durable route consequences to matter.
