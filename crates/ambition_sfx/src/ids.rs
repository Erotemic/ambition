//! Hand-maintained ids for reusable gameplay/presentation SFX. Provider-specific
//! cast, story, and named-content ids belong with their provider/content.
//!
//! One-off cues may use `SfxId::from_static` directly. [`sfx_ids!`] emits both
//! constants and the `(id, name)` table used by diagnostics, keeping the
//! one-way hash and its human-readable spelling in one declaration.

use crate::SfxId;

/// Declare an id constant and record the same authored spelling in [`NAMED`]
/// for reverse diagnostic lookup.
#[macro_export]
macro_rules! sfx_ids {
    ($($(#[$note:meta])* $name:ident => $spelling:literal),* $(,)?) => {
        $($(#[$note])* pub const $name: $crate::SfxId = $crate::SfxId::from_static($spelling);)*

        /// Every engine cue as `(id, authored spelling)`, in declaration order.
        pub const NAMED: &[($crate::SfxId, &str)] = &[$(($name, $spelling)),*];
    };
}

sfx_ids! {
    // Player movement
    PLAYER_JUMP => "player.jump",
    PLAYER_DOUBLE_JUMP => "player.double_jump",
    PLAYER_DASH => "player.dash",
    PLAYER_BLINK => "player.blink",
    PLAYER_PRECISION_BLINK => "player.precision_blink",
    PLAYER_POGO => "player.pogo",
    PLAYER_LAND => "player.land",
    PLAYER_FAST_FALL => "player.fast_fall",
    PLAYER_WALL_JUMP => "player.wall_jump",
    PLAYER_WALL_SLIDE => "player.wall_slide",
    PLAYER_WALL_CLING => "player.wall_cling",
    PLAYER_LEDGE_GRAB => "player.ledge_grab",
    PLAYER_REBOUND => "player.rebound",

    // Player combat / vitals
    PLAYER_SLASH => "player.slash",
    /// Canonical robot protagonist blade: the dry swing through open air.
    PLAYER_ROBOT_SLASH_AIR => "player.robot.slash.air",
    /// Internal selector cue carried by the robot protagonist's hit volume. The
    /// victim-side resolver replaces it with the matching material variant.
    PLAYER_ROBOT_SLASH_IMPACT => "player.robot.slash.impact",
    PLAYER_ROBOT_SLASH_IMPACT_FLESH_LIGHT => "player.robot.slash.impact.flesh.light",
    PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP => "player.robot.slash.impact.flesh.deep",
    PLAYER_ROBOT_SLASH_IMPACT_ROBOT => "player.robot.slash.impact.robot",
    PLAYER_ROBOT_SLASH_IMPACT_METAL_CHINK => "player.robot.slash.impact.metal.chink",
    PLAYER_ROBOT_SLASH_IMPACT_METAL_GONG => "player.robot.slash.impact.metal.gong",
    PLAYER_ROBOT_SLASH_IMPACT_POGO => "player.robot.slash.impact.pogo",
    PLAYER_HIT => "player.hit",
    PLAYER_DAMAGE => "player.damage",
    PLAYER_HEAL => "player.heal",
    PLAYER_DEATH => "player.death",
    PLAYER_RESPAWN => "player.respawn",
    PLAYER_RESET => "player.reset",
    PLAYER_LOW_HEALTH_PULSE => "player.low_health.pulse",
    PLAYER_STAMINA_EMPTY => "player.stamina_empty",
    /// The bright CLANG of a perfect shield catching a strike.
    ///
    /// The only audible evidence a parry happened: a caught strike is negated
    /// outright, so there is no impact and no hurt sound to hear instead.
    PLAYER_PARRY => "player.parry",
    /// The crisp CLACK of a successful tech — the defender refusing a
    /// knockdown. Deliberately not the dash cue the getup roll uses: the two
    /// used to share one sound and a spectator could not tell them apart.
    PLAYER_TECH => "player.tech",
    /// The mechanical LATCH the instant a smash charge takes hold — the beat
    /// that tells a player the hold registered and the move is now waiting on
    /// them. Fires once per charge, on the edge, never per frame.
    PLAYER_SMASH_CHARGE_LATCH => "player.smash_charge.latch",
    /// The short, higher LOCK once a smash reaches maximum charge. Fires at
    /// most once per charge: holding past maximum buys nothing and must not
    /// keep saying so.
    PLAYER_SMASH_CHARGE_LOADED => "player.smash_charge.loaded",
    PLAYER_ABILITY_UNLOCK => "player.ability_unlock",

    // Player damage-type variants (when source is typed)
    PLAYER_HIT_FIRE => "player.hit.fire",
    PLAYER_HIT_ICE => "player.hit.ice",
    PLAYER_HIT_LIGHTNING => "player.hit.lightning",
    PLAYER_HIT_POISON => "player.hit.poison",

    // Hazards (single-shot contacts)
    HAZARD_LAVA_SPLASH => "hazard.lava.splash",
    HAZARD_ACID_SPLASH => "hazard.acid.splash",
    HAZARD_SPIKE_HIT => "hazard.spike.hit",
    HAZARD_ELECTRIC_ARC => "hazard.electric.arc",
    HAZARD_SAW_HIT => "hazard.saw.hit",
    // Looped hazard ambients (start/stop on volume entry/exit): wiring lives in TODO
    // until the loop-lifecycle subsystem lands.
    HAZARD_WIND_GUST_LOOP => "hazard.wind.gust_loop",
    HAZARD_POISON_CLOUD_LOOP => "hazard.poison.cloud_loop",
    HAZARD_ELECTRIC_LOOP => "hazard.electric.loop",
    HAZARD_SAW_LOOP => "hazard.saw.loop",

    // UI
    // Menu movement currently uses the icon-move cue.
    UI_MENU_MOVE => "ui.menu.move_icon",

    UI_MENU_ACCEPT => "ui.menu.accept",
    UI_MENU_BACK => "ui.menu.back",
    UI_TAB_CHANGE => "ui.tab.change",
    UI_PAUSE_OPEN => "ui.pause.open",
    UI_PAUSE_CLOSE => "ui.pause.close",
    UI_SAVE_COMPLETE => "ui.save.complete",
    UI_ERROR => "ui.error",

    // Dialogue / Yarn presentation. These ids are authored under
    // `tools/ambition_sfx_renderer/sounds/active/dialogue.*.sfx.yaml` and
    // play through the open-ended `SfxMessage::Play { id, .. }` path.
    DIALOGUE_BLIP_GENERIC => "dialogue.blip.generic",
    DIALOGUE_BLIP_WHISPER_GENERIC => "dialogue.blip.whisper.generic",
    DIALOGUE_BLIP_SHOUT_GENERIC => "dialogue.blip.shout.generic",
    DIALOGUE_LINE_ADVANCE => "dialogue.line.advance",
    DIALOGUE_CHOICE_APPEAR => "dialogue.choice.appear",
    DIALOGUE_CHOICE_SELECT => "dialogue.choice.select",
    DIALOGUE_MARKUP_SHOUT => "dialogue.markup.shout",
    DIALOGUE_MARKUP_WHISPER => "dialogue.markup.whisper",

    // Cube inventory menu (3D OoT cube). These `ui.*` ids are authored under
    // `tools/ambition_sfx_renderer/sounds/active/ui.*.sfx.yaml`; if an id isn't packed
    // into the runtime bank yet the play just no-ops, so wiring them is always safe.
    UI_MENU_OPEN => "ui.menu.open",
    UI_MENU_CLOSE => "ui.menu.close",
    UI_MENU_ROTATE => "ui.menu.rotate",
    UI_MENU_ROTATE_LEFT => "ui.menu.rotate_left",
    UI_MENU_ROTATE_RIGHT => "ui.menu.rotate_right",
    UI_MENU_EQUIP => "ui.menu.equip",
    UI_MENU_UNEQUIP => "ui.menu.unequip",
    UI_MENU_ERROR => "ui.menu.error",

    // Footsteps (variants are sibling ids; gameplay picks among them)
    PLAYER_FOOTSTEP_STONE_01 => "player.footstep.stone.01",
    PLAYER_FOOTSTEP_STONE_02 => "player.footstep.stone.02",
    PLAYER_FOOTSTEP_METAL_01 => "player.footstep.metal.01",
    PLAYER_FOOTSTEP_METAL_02 => "player.footstep.metal.02",
    PLAYER_FOOTSTEP_SOFT_01 => "player.footstep.soft.01",
    PLAYER_FOOTSTEP_SOFT_02 => "player.footstep.soft.02",

    // World interactions
    WORLD_TREASURE_CHEST_OPEN => "world.treasure_chest.open",
    WORLD_DOOR_OPEN => "world.door.open",
    WORLD_DOOR_CLOSE => "world.door.close",
    WORLD_DOOR_HEAVY_OPEN => "world.door.heavy_open",
    WORLD_DOOR_HEAVY_CLOSE => "world.door.heavy_close",
    WORLD_DOOR_LOCKED_RATTLE => "world.door.locked.rattle",
    WORLD_GATE_RISE => "world.gate.rise",
    WORLD_GATE_FALL => "world.gate.fall",
    WORLD_LEVER_ENGAGE => "world.lever.engage",
    WORLD_LEVER_DISENGAGE => "world.lever.disengage",
    WORLD_LOCK_OPEN => "world.lock.open",
    WORLD_PRESSURE_PLATE_CLICK_ON => "world.pressure_plate.click_on",
    WORLD_PRESSURE_PLATE_CLICK_OFF => "world.pressure_plate.click_off",
    WORLD_SWITCH_TOGGLE => "world.switch.toggle",
    WORLD_CRATE_BREAK => "world.crate.break",
    WORLD_ROCK_BREAK => "world.rock.break",
    WORLD_ROCK_HIT => "world.rock.hit",
    // Reusable generated explosion SFX. These IDs are authored under
    // tools/ambition_sfx_renderer/sounds/active/vfx.explosion.*.sfx.yaml and
    // packed into the runtime SFX bank; there is no committed OGG/WAV asset.
    VFX_EXPLOSION_CLASSIC_BURST => "vfx.explosion.classic_burst",
    VFX_EXPLOSION_BURST_ROUND => "vfx.explosion.burst_round",
    VFX_EXPLOSION_SHOCKWAVE => "vfx.explosion.shockwave",
    VFX_EXPLOSION_SMOKE_BURST => "vfx.explosion.smoke_burst",
    VFX_EXPLOSION_STARBURST => "vfx.explosion.starburst",
    WORLD_PORTAL_ENTER => "world.portal.enter",
    WORLD_CHECKPOINT_ACTIVATE => "world.checkpoint.activate",
    WORLD_SAVE_POINT_ACTIVATE => "world.save_point.activate",
    WORLD_SAVE_POINT_IDLE_LOOP => "world.save_point.idle_loop",
    WORLD_TELEPORTER_LOOP => "world.teleporter.loop",
    WORLD_SECRET_REVEAL => "world.secret.reveal",
    WORLD_ABILITY_UNLOCK => "world.ability.unlock",
    WORLD_UPGRADE_PERMANENT => "world.upgrade.permanent",
    WORLD_PLATFORM_START => "world.platform.start",
    WORLD_PLATFORM_LOOP => "world.platform.loop",
    WORLD_PLATFORM_STOP => "world.platform.stop",

    // Pickups
    WORLD_PICKUP_GENERIC => "world.pickup.generic",
    WORLD_HEALTH_COLLECT => "world.health.collect",
    WORLD_HEART_CONTAINER_COLLECT => "world.heart_container.collect",
    WORLD_COIN_PICKUP => "world.coin.pickup",
    WORLD_COIN_COLLECT => "world.coin.collect",
    WORLD_COIN_LARGE => "world.coin.large",
    WORLD_COIN_HUGE => "world.coin.huge",
    WORLD_KEY_PICKUP => "world.key.pickup",
    WORLD_LORE_PICKUP => "world.lore.pickup",
    PLAYER_COLLECT_COIN => "player.collect.coin",
    PLAYER_COLLECT_HEALTH => "player.collect.health",
    PLAYER_PICKUP_HEALTH => "player.pickup.health",

    // Ladder / climbing
    PLAYER_LADDER_GRAB => "player.ladder.grab",
    PLAYER_LADDER_CLIMB => "player.ladder.climb",
    PLAYER_LADDER_CLIMB_LOOP => "player.ladder.climb_loop",

    // Footstep variants by surface (variant numbers chosen per surface)
    PLAYER_FOOTSTEP_GRASS_01 => "player.footstep.grass.01",
    PLAYER_FOOTSTEP_GRASS_02 => "player.footstep.grass.02",
    PLAYER_FOOTSTEP_GRASS_03 => "player.footstep.grass.03",
    PLAYER_FOOTSTEP_WOOD_01 => "player.footstep.wood.01",
    PLAYER_FOOTSTEP_WOOD_02 => "player.footstep.wood.02",
    PLAYER_FOOTSTEP_WOOD_03 => "player.footstep.wood.03",
    PLAYER_FOOTSTEP_WATER_01 => "player.footstep.water.01",
    PLAYER_FOOTSTEP_WATER_02 => "player.footstep.water.02",
    PLAYER_FOOTSTEP_WATER_03 => "player.footstep.water.03",
    PLAYER_FOOTSTEP_ICE_01 => "player.footstep.ice.01",
    PLAYER_FOOTSTEP_ICE_02 => "player.footstep.ice.02",
    PLAYER_FOOTSTEP_ICE_03 => "player.footstep.ice.03",
    PLAYER_FOOTSTEP_SAND_01 => "player.footstep.sand.01",
    PLAYER_FOOTSTEP_SAND_02 => "player.footstep.sand.02",
    PLAYER_FOOTSTEP_SAND_03 => "player.footstep.sand.03",
    PLAYER_FOOTSTEP_SNOW_01 => "player.footstep.snow.01",
    PLAYER_FOOTSTEP_SNOW_02 => "player.footstep.snow.02",
    PLAYER_FOOTSTEP_SNOW_03 => "player.footstep.snow.03",
    PLAYER_FOOTSTEP_GLASS_01 => "player.footstep.glass.01",
    PLAYER_FOOTSTEP_GLASS_02 => "player.footstep.glass.02",

    // UI (additional)
    UI_ACCEPT => "ui.accept",
    UI_BACK => "ui.back",
    UI_CONFIRM_WARNING => "ui.confirm.warning",
    UI_SLIDER_TICK => "ui.slider.tick",
    UI_TOGGLE_ON => "ui.toggle.on",
    UI_TOGGLE_OFF => "ui.toggle.off",
    UI_TOOLTIP_APPEAR => "ui.tooltip.appear",
    UI_NOTIFICATION_DISCOVERY => "ui.notification.discovery",
    UI_NOTIFICATION_QUEST_COMPLETE => "ui.notification.quest_complete",

    // Portal gun
    PORTAL_POWERUP => "portal.powerup",
    PORTAL_FIRE => "portal.fire",
    PORTAL_TRAVEL => "portal.travel",
    PORTAL_ATTACH => "portal.attach",
    PORTAL_INVALID => "portal.invalid",
    PORTAL_HUM => "portal.hum",
    PORTAL_ENTER => "portal.enter",
    PORTAL_EXIT => "portal.exit",
    PORTAL_CLOSE => "portal.close",
    PORTAL_FIZZLE => "portal.fizzle",
}

/// Back-compat semantic alias for call sites that only need "an explosion".
/// Not in [`NAMED`] on purpose: the hash belongs to
/// [`VFX_EXPLOSION_CLASSIC_BURST`], and one hash gets one name.
pub const WORLD_EXPLOSION: SfxId = VFX_EXPLOSION_CLASSIC_BURST;

/// The authored spelling of an engine cue, if this id is one.
///
/// A linear scan of ~150 entries, which is right: this runs when a cue failed
/// to resolve and a human is about to read a log line, never in the play path.
/// Provider-local cue ids are not here — see the audio crate's
/// `describe_sfx_id`, which falls back to the loaded banks' name sections.
pub fn name_of(id: SfxId) -> Option<&'static str> {
    NAMED
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the macro exists to hold: every constant's hash resolves
    /// back to the string it was declared with. If someone adds a constant by
    /// hand outside `sfx_ids!`, its diagnostics silently degrade to a hash —
    /// this catches the ones that matter (the whole table) rather than trying
    /// to catch the act of writing one.
    #[test]
    fn every_engine_cue_can_be_named_from_its_hash() {
        assert!(NAMED.len() > 100, "the table lost most of its entries");
        for (id, spelling) in NAMED {
            assert_eq!(
                *id,
                SfxId::new(spelling),
                "{spelling} does not hash to its own constant"
            );
            assert_eq!(name_of(*id), Some(*spelling));
        }
    }

    #[test]
    fn an_unknown_id_is_not_invented() {
        assert_eq!(name_of(SfxId::new("nobody.authored.this")), None);
    }
}
