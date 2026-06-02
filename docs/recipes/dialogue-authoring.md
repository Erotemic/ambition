# Dialogue authoring — Yarn commands & functions

What authored `.yarn` content can invoke at runtime. This is the
author-facing companion to [ADR 0008](../adr/0008-dialogue-and-commerce-architecture.md);
the implementation (and the single source of truth) is
[`crates/ambition_sandbox/src/dialog/yarn_bindings.rs`](../../crates/ambition_sandbox/src/dialog/yarn_bindings.rs).
Keep this table in sync when you add or change a binding.

`.yarn` files live under `crates/ambition_sandbox/assets/dialogue/`.
The `kernel.yarn` "test menu" exercises most of these verbs live, so
it doubles as a worked example.

## Commands — `<<verb args>>`

Commands *drive* gameplay. Each maps to a Bevy system that writes a
typed message channel (or mutates a resource).

| Command | Args | Effect | Status |
|---|---|---|---|
| `set_flag` | `"id"` | Set a save flag to `true` (via `SetFlagRequested`; quest-advance + save-mirror consumers see it). | Live |
| `clear_flag` | `"id"` | Set a save flag to `false`. | Live |
| `give_item` | `"kind" count` | Add `count` of the item to the live `PlayerInventory`. `kind` is loose (`HealthPotion` / `health_potion` / `health potion`); unknown kind or non-positive count is logged + ignored. | Live |
| `play_sfx` | `"id"` | Play a sound effect by id (`SfxMessage`). | Live |
| `spawn_fireworks` | — | Celebratory VFX burst at the player. | Live |
| `reset_cut_rope_room` | — | Replay the Smirking Behemoth "cut the rope" room (latched until the dialogue closes). | Live |
| `watch_cut_rope_video` | — | Sets a flag now; the optional desktop browser-launch is a TODO. | Partial |
| `camera_zoom` | `factor` | Cinematic zoom. **Logged stub** — needs a camera-composition seam (see TODO "kernel dialog tree"). | Stub |
| `spawn_chest` | `"id"` | Spawn a reward chest. **Logged stub** — chest spawns are room/encounter-spec driven today; needs a position/contents decision. | Stub |

## Functions — `<<if fn(args)>>`

Functions *read* gameplay. They are synchronous pure functions (they
cannot be Bevy systems), so they read a per-frame `YarnStateMirror`
snapshot refreshed by `refresh_yarn_state_mirror`. All are live.

| Function | Returns | Reads |
|---|---|---|
| `flag("id")` | bool | Save flag on/off. |
| `boss_cleared("id")` | bool | Whether the boss encounter `id` is in `Cleared` state. |
| `quest_active("id")` | bool | Whether quest `id` is `InProgress`. |
| `visit_count("id")` | number (f32) | How many times the named dialogue node has been entered. |
| `inventory_has("item")` | bool | Whether the player holds ≥1 of `item` (live `PlayerInventory`; loose item-id spelling). |
| `cut_rope_heavy_object_is("id")` | bool | Whether the runtime-selected cut-rope heavy object matches `id` (`anvil` / `piano` / …). |

## Markup cues — inline

| Markup | Effect |
|---|---|
| `[shout]LINE[/shout]` | Flags the line for camera-shake / louder-voice presentation consumers. |
| `[whisper]LINE[/whisper]` | Flags the line for pitch/volume-drop presentation consumers. |

## Adding a binding

1. Add a `cmd_*` system (command) or an `add_function` closure
   (function) in `yarn_bindings.rs`, register it in
   `register_commands` / `register_functions`.
2. A function that reads game state needs a slice on
   `YarnStateMirrorData` + a line in `refresh_yarn_state_mirror`
   (see how `inventory_has` mirrors `PlayerInventory`).
3. Update this table.
