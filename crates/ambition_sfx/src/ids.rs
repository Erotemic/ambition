//! Hand-maintained SFX ids for reusable gameplay and presentation semantics.
//!
//! Provider-specific cast, story, and named-content cue ids belong in the
//! provider/content crate rather than this generic runtime contract.
//!
//! Adding to this list is purely an ergonomics call: the bank stores everything
//! regardless. Use `SfxId::from_static("foo.bar")` at the call site for one-off
//! or rare SFX. IDs match the catalog produced by `ambition_sfx_renderer` under
//! `tools/ambition_sfx_renderer/output/`. When in doubt, run:
//!
//! ```text
//! python3 tools/ambition_sfx_pack/pack.py --dump
//! ```
//!
//! and grep `crates/ambition_actors/assets/audio/sfx.bank.txt`.

use crate::SfxId;

// Player movement
pub const PLAYER_JUMP: SfxId = SfxId::from_static("player.jump");
pub const PLAYER_DOUBLE_JUMP: SfxId = SfxId::from_static("player.double_jump");
pub const PLAYER_DASH: SfxId = SfxId::from_static("player.dash");
pub const PLAYER_BLINK: SfxId = SfxId::from_static("player.blink");
pub const PLAYER_PRECISION_BLINK: SfxId = SfxId::from_static("player.precision_blink");
pub const PLAYER_POGO: SfxId = SfxId::from_static("player.pogo");
pub const PLAYER_LAND: SfxId = SfxId::from_static("player.land");
pub const PLAYER_FAST_FALL: SfxId = SfxId::from_static("player.fast_fall");
pub const PLAYER_WALL_JUMP: SfxId = SfxId::from_static("player.wall_jump");
pub const PLAYER_WALL_SLIDE: SfxId = SfxId::from_static("player.wall_slide");
pub const PLAYER_WALL_CLING: SfxId = SfxId::from_static("player.wall_cling");
pub const PLAYER_LEDGE_GRAB: SfxId = SfxId::from_static("player.ledge_grab");
pub const PLAYER_REBOUND: SfxId = SfxId::from_static("player.rebound");

// Player combat / vitals
pub const PLAYER_SLASH: SfxId = SfxId::from_static("player.slash");
pub const PLAYER_HIT: SfxId = SfxId::from_static("player.hit");
pub const PLAYER_DAMAGE: SfxId = SfxId::from_static("player.damage");
pub const PLAYER_HEAL: SfxId = SfxId::from_static("player.heal");
pub const PLAYER_DEATH: SfxId = SfxId::from_static("player.death");
pub const PLAYER_RESPAWN: SfxId = SfxId::from_static("player.respawn");
pub const PLAYER_RESET: SfxId = SfxId::from_static("player.reset");
pub const PLAYER_LOW_HEALTH_PULSE: SfxId = SfxId::from_static("player.low_health.pulse");
pub const PLAYER_STAMINA_EMPTY: SfxId = SfxId::from_static("player.stamina_empty");
pub const PLAYER_ABILITY_UNLOCK: SfxId = SfxId::from_static("player.ability_unlock");

// Player damage-type variants (when source is typed)
pub const PLAYER_HIT_FIRE: SfxId = SfxId::from_static("player.hit.fire");
pub const PLAYER_HIT_ICE: SfxId = SfxId::from_static("player.hit.ice");
pub const PLAYER_HIT_LIGHTNING: SfxId = SfxId::from_static("player.hit.lightning");
pub const PLAYER_HIT_POISON: SfxId = SfxId::from_static("player.hit.poison");

// Hazards (single-shot contacts)
pub const HAZARD_LAVA_SPLASH: SfxId = SfxId::from_static("hazard.lava.splash");
pub const HAZARD_ACID_SPLASH: SfxId = SfxId::from_static("hazard.acid.splash");
pub const HAZARD_SPIKE_HIT: SfxId = SfxId::from_static("hazard.spike.hit");
pub const HAZARD_ELECTRIC_ARC: SfxId = SfxId::from_static("hazard.electric.arc");
pub const HAZARD_SAW_HIT: SfxId = SfxId::from_static("hazard.saw.hit");
// Looped hazard ambients (start/stop on volume entry/exit): wiring lives in TODO
// until the loop-lifecycle subsystem lands.
pub const HAZARD_WIND_GUST_LOOP: SfxId = SfxId::from_static("hazard.wind.gust_loop");
pub const HAZARD_POISON_CLOUD_LOOP: SfxId = SfxId::from_static("hazard.poison.cloud_loop");
pub const HAZARD_ELECTRIC_LOOP: SfxId = SfxId::from_static("hazard.electric.loop");
pub const HAZARD_SAW_LOOP: SfxId = SfxId::from_static("hazard.saw.loop");

// UI
//pub const UI_MENU_MOVE: SfxId = SfxId::from_static("ui.menu.move");
// Test to see if the move icon sounds better than the original move version:
// yes it is much better as of 2026-06-07, not sure if we need separate icon sounds
pub const UI_MENU_MOVE: SfxId = SfxId::from_static("ui.menu.move_icon");

pub const UI_MENU_ACCEPT: SfxId = SfxId::from_static("ui.menu.accept");
pub const UI_MENU_BACK: SfxId = SfxId::from_static("ui.menu.back");
pub const UI_TAB_CHANGE: SfxId = SfxId::from_static("ui.tab.change");
pub const UI_PAUSE_OPEN: SfxId = SfxId::from_static("ui.pause.open");
pub const UI_PAUSE_CLOSE: SfxId = SfxId::from_static("ui.pause.close");
pub const UI_SAVE_COMPLETE: SfxId = SfxId::from_static("ui.save.complete");
pub const UI_ERROR: SfxId = SfxId::from_static("ui.error");

// Dialogue / Yarn presentation. These ids are authored under
// `tools/ambition_sfx_renderer/sounds/active/dialogue.*.sfx.yaml` and
// play through the open-ended `SfxMessage::Play { id, .. }` path.
pub const DIALOGUE_BLIP_GENERIC: SfxId = SfxId::from_static("dialogue.blip.generic");
pub const DIALOGUE_BLIP_WHISPER_GENERIC: SfxId =
    SfxId::from_static("dialogue.blip.whisper.generic");
pub const DIALOGUE_BLIP_SHOUT_GENERIC: SfxId = SfxId::from_static("dialogue.blip.shout.generic");
pub const DIALOGUE_LINE_ADVANCE: SfxId = SfxId::from_static("dialogue.line.advance");
pub const DIALOGUE_CHOICE_APPEAR: SfxId = SfxId::from_static("dialogue.choice.appear");
pub const DIALOGUE_CHOICE_SELECT: SfxId = SfxId::from_static("dialogue.choice.select");
pub const DIALOGUE_MARKUP_SHOUT: SfxId = SfxId::from_static("dialogue.markup.shout");
pub const DIALOGUE_MARKUP_WHISPER: SfxId = SfxId::from_static("dialogue.markup.whisper");

// Cube inventory menu (3D OoT cube). These `ui.*` ids are authored under
// `tools/ambition_sfx_renderer/sounds/active/ui.*.sfx.yaml`; if an id isn't packed
// into the runtime bank yet the play just no-ops, so wiring them is always safe.
pub const UI_MENU_OPEN: SfxId = SfxId::from_static("ui.menu.open");
pub const UI_MENU_CLOSE: SfxId = SfxId::from_static("ui.menu.close");
pub const UI_MENU_ROTATE: SfxId = SfxId::from_static("ui.menu.rotate");
pub const UI_MENU_ROTATE_LEFT: SfxId = SfxId::from_static("ui.menu.rotate_left");
pub const UI_MENU_ROTATE_RIGHT: SfxId = SfxId::from_static("ui.menu.rotate_right");
pub const UI_MENU_EQUIP: SfxId = SfxId::from_static("ui.menu.equip");
pub const UI_MENU_UNEQUIP: SfxId = SfxId::from_static("ui.menu.unequip");
pub const UI_MENU_ERROR: SfxId = SfxId::from_static("ui.menu.error");

// Footsteps (variants are sibling ids; gameplay picks among them)
pub const PLAYER_FOOTSTEP_STONE_01: SfxId = SfxId::from_static("player.footstep.stone.01");
pub const PLAYER_FOOTSTEP_STONE_02: SfxId = SfxId::from_static("player.footstep.stone.02");
pub const PLAYER_FOOTSTEP_METAL_01: SfxId = SfxId::from_static("player.footstep.metal.01");
pub const PLAYER_FOOTSTEP_METAL_02: SfxId = SfxId::from_static("player.footstep.metal.02");
pub const PLAYER_FOOTSTEP_SOFT_01: SfxId = SfxId::from_static("player.footstep.soft.01");
pub const PLAYER_FOOTSTEP_SOFT_02: SfxId = SfxId::from_static("player.footstep.soft.02");

// World interactions
pub const WORLD_TREASURE_CHEST_OPEN: SfxId = SfxId::from_static("world.treasure_chest.open");
pub const WORLD_DOOR_OPEN: SfxId = SfxId::from_static("world.door.open");
pub const WORLD_DOOR_CLOSE: SfxId = SfxId::from_static("world.door.close");
pub const WORLD_DOOR_HEAVY_OPEN: SfxId = SfxId::from_static("world.door.heavy_open");
pub const WORLD_DOOR_HEAVY_CLOSE: SfxId = SfxId::from_static("world.door.heavy_close");
pub const WORLD_DOOR_LOCKED_RATTLE: SfxId = SfxId::from_static("world.door.locked.rattle");
pub const WORLD_GATE_RISE: SfxId = SfxId::from_static("world.gate.rise");
pub const WORLD_GATE_FALL: SfxId = SfxId::from_static("world.gate.fall");
pub const WORLD_LEVER_ENGAGE: SfxId = SfxId::from_static("world.lever.engage");
pub const WORLD_LEVER_DISENGAGE: SfxId = SfxId::from_static("world.lever.disengage");
pub const WORLD_LOCK_OPEN: SfxId = SfxId::from_static("world.lock.open");
pub const WORLD_PRESSURE_PLATE_CLICK_ON: SfxId =
    SfxId::from_static("world.pressure_plate.click_on");
pub const WORLD_PRESSURE_PLATE_CLICK_OFF: SfxId =
    SfxId::from_static("world.pressure_plate.click_off");
pub const WORLD_SWITCH_TOGGLE: SfxId = SfxId::from_static("world.switch.toggle");
pub const WORLD_CRATE_BREAK: SfxId = SfxId::from_static("world.crate.break");
pub const WORLD_ROCK_BREAK: SfxId = SfxId::from_static("world.rock.break");
pub const WORLD_ROCK_HIT: SfxId = SfxId::from_static("world.rock.hit");
// Reusable generated explosion SFX. These IDs are authored under
// tools/ambition_sfx_renderer/sounds/active/vfx.explosion.*.sfx.yaml and
// packed into the runtime SFX bank; there is no committed OGG/WAV asset.
pub const VFX_EXPLOSION_CLASSIC_BURST: SfxId = SfxId::from_static("vfx.explosion.classic_burst");
pub const VFX_EXPLOSION_BURST_ROUND: SfxId = SfxId::from_static("vfx.explosion.burst_round");
pub const VFX_EXPLOSION_SHOCKWAVE: SfxId = SfxId::from_static("vfx.explosion.shockwave");
pub const VFX_EXPLOSION_SMOKE_BURST: SfxId = SfxId::from_static("vfx.explosion.smoke_burst");
pub const VFX_EXPLOSION_STARBURST: SfxId = SfxId::from_static("vfx.explosion.starburst");
// Back-compat semantic alias for call sites that only need "an explosion".
pub const WORLD_EXPLOSION: SfxId = VFX_EXPLOSION_CLASSIC_BURST;
pub const WORLD_PORTAL_ENTER: SfxId = SfxId::from_static("world.portal.enter");
pub const WORLD_CHECKPOINT_ACTIVATE: SfxId = SfxId::from_static("world.checkpoint.activate");
pub const WORLD_SAVE_POINT_ACTIVATE: SfxId = SfxId::from_static("world.save_point.activate");
pub const WORLD_SAVE_POINT_IDLE_LOOP: SfxId = SfxId::from_static("world.save_point.idle_loop");
pub const WORLD_TELEPORTER_LOOP: SfxId = SfxId::from_static("world.teleporter.loop");
pub const WORLD_SECRET_REVEAL: SfxId = SfxId::from_static("world.secret.reveal");
pub const WORLD_ABILITY_UNLOCK: SfxId = SfxId::from_static("world.ability.unlock");
pub const WORLD_UPGRADE_PERMANENT: SfxId = SfxId::from_static("world.upgrade.permanent");
pub const WORLD_PLATFORM_START: SfxId = SfxId::from_static("world.platform.start");
pub const WORLD_PLATFORM_LOOP: SfxId = SfxId::from_static("world.platform.loop");
pub const WORLD_PLATFORM_STOP: SfxId = SfxId::from_static("world.platform.stop");

// Pickups
pub const WORLD_PICKUP_GENERIC: SfxId = SfxId::from_static("world.pickup.generic");
pub const WORLD_HEALTH_COLLECT: SfxId = SfxId::from_static("world.health.collect");
pub const WORLD_HEART_CONTAINER_COLLECT: SfxId =
    SfxId::from_static("world.heart_container.collect");
pub const WORLD_COIN_PICKUP: SfxId = SfxId::from_static("world.coin.pickup");
pub const WORLD_COIN_COLLECT: SfxId = SfxId::from_static("world.coin.collect");
pub const WORLD_COIN_LARGE: SfxId = SfxId::from_static("world.coin.large");
pub const WORLD_COIN_HUGE: SfxId = SfxId::from_static("world.coin.huge");
pub const WORLD_KEY_PICKUP: SfxId = SfxId::from_static("world.key.pickup");
pub const WORLD_LORE_PICKUP: SfxId = SfxId::from_static("world.lore.pickup");
pub const PLAYER_COLLECT_COIN: SfxId = SfxId::from_static("player.collect.coin");
pub const PLAYER_COLLECT_HEALTH: SfxId = SfxId::from_static("player.collect.health");
pub const PLAYER_PICKUP_HEALTH: SfxId = SfxId::from_static("player.pickup.health");

// Ladder / climbing
pub const PLAYER_LADDER_GRAB: SfxId = SfxId::from_static("player.ladder.grab");
pub const PLAYER_LADDER_CLIMB: SfxId = SfxId::from_static("player.ladder.climb");
pub const PLAYER_LADDER_CLIMB_LOOP: SfxId = SfxId::from_static("player.ladder.climb_loop");

// Footstep variants by surface (variant numbers chosen per surface)
pub const PLAYER_FOOTSTEP_GRASS_01: SfxId = SfxId::from_static("player.footstep.grass.01");
pub const PLAYER_FOOTSTEP_GRASS_02: SfxId = SfxId::from_static("player.footstep.grass.02");
pub const PLAYER_FOOTSTEP_GRASS_03: SfxId = SfxId::from_static("player.footstep.grass.03");
pub const PLAYER_FOOTSTEP_WOOD_01: SfxId = SfxId::from_static("player.footstep.wood.01");
pub const PLAYER_FOOTSTEP_WOOD_02: SfxId = SfxId::from_static("player.footstep.wood.02");
pub const PLAYER_FOOTSTEP_WOOD_03: SfxId = SfxId::from_static("player.footstep.wood.03");
pub const PLAYER_FOOTSTEP_WATER_01: SfxId = SfxId::from_static("player.footstep.water.01");
pub const PLAYER_FOOTSTEP_WATER_02: SfxId = SfxId::from_static("player.footstep.water.02");
pub const PLAYER_FOOTSTEP_WATER_03: SfxId = SfxId::from_static("player.footstep.water.03");
pub const PLAYER_FOOTSTEP_ICE_01: SfxId = SfxId::from_static("player.footstep.ice.01");
pub const PLAYER_FOOTSTEP_ICE_02: SfxId = SfxId::from_static("player.footstep.ice.02");
pub const PLAYER_FOOTSTEP_ICE_03: SfxId = SfxId::from_static("player.footstep.ice.03");
pub const PLAYER_FOOTSTEP_SAND_01: SfxId = SfxId::from_static("player.footstep.sand.01");
pub const PLAYER_FOOTSTEP_SAND_02: SfxId = SfxId::from_static("player.footstep.sand.02");
pub const PLAYER_FOOTSTEP_SAND_03: SfxId = SfxId::from_static("player.footstep.sand.03");
pub const PLAYER_FOOTSTEP_SNOW_01: SfxId = SfxId::from_static("player.footstep.snow.01");
pub const PLAYER_FOOTSTEP_SNOW_02: SfxId = SfxId::from_static("player.footstep.snow.02");
pub const PLAYER_FOOTSTEP_SNOW_03: SfxId = SfxId::from_static("player.footstep.snow.03");
pub const PLAYER_FOOTSTEP_GLASS_01: SfxId = SfxId::from_static("player.footstep.glass.01");
pub const PLAYER_FOOTSTEP_GLASS_02: SfxId = SfxId::from_static("player.footstep.glass.02");

// UI (additional)
pub const UI_ACCEPT: SfxId = SfxId::from_static("ui.accept");
pub const UI_BACK: SfxId = SfxId::from_static("ui.back");
pub const UI_CONFIRM_WARNING: SfxId = SfxId::from_static("ui.confirm.warning");
pub const UI_SLIDER_TICK: SfxId = SfxId::from_static("ui.slider.tick");
pub const UI_TOGGLE_ON: SfxId = SfxId::from_static("ui.toggle.on");
pub const UI_TOGGLE_OFF: SfxId = SfxId::from_static("ui.toggle.off");
pub const UI_TOOLTIP_APPEAR: SfxId = SfxId::from_static("ui.tooltip.appear");
pub const UI_NOTIFICATION_DISCOVERY: SfxId = SfxId::from_static("ui.notification.discovery");
pub const UI_NOTIFICATION_QUEST_COMPLETE: SfxId =
    SfxId::from_static("ui.notification.quest_complete");

// Portal gun
pub const PORTAL_POWERUP: SfxId = SfxId::from_static("portal.powerup");
pub const PORTAL_FIRE: SfxId = SfxId::from_static("portal.fire");
pub const PORTAL_TRAVEL: SfxId = SfxId::from_static("portal.travel");
pub const PORTAL_ATTACH: SfxId = SfxId::from_static("portal.attach");
pub const PORTAL_INVALID: SfxId = SfxId::from_static("portal.invalid");
pub const PORTAL_HUM: SfxId = SfxId::from_static("portal.hum");
pub const PORTAL_ENTER: SfxId = SfxId::from_static("portal.enter");
pub const PORTAL_EXIT: SfxId = SfxId::from_static("portal.exit");
pub const PORTAL_CLOSE: SfxId = SfxId::from_static("portal.close");
pub const PORTAL_FIZZLE: SfxId = SfxId::from_static("portal.fizzle");
