//! The fighter specials that more than one game's cast authors, installed for
//! their authored keys.
//!
//! Each is a module of `ambition_demo_smash_modules`: an edit to one compiles
//! that crate alone, and a running game can load it as a file. This crate
//! only offers each module for its key (`install_module_technique`), with the
//! domain rule that admits its params. A game whose cast authors them calls
//! [`install`]: the Smash demo does, and so does Ambition, whose cast authors
//! most of them, so a hub without Smash admits Ambition's moves. Installing
//! twice installs once.
//!
//! What stays with a game is its own rules around a special: Smash's
//! sleeper's mash, its Limit fill and the match that ends a placed object.

use ambition_platformer2d::combat::technique::{
    check_hydrates, InstalledTechniques, NestedReferences, TechniqueDelivery, TechniqueOffer, TechniqueParams,
};
use ambition_platformer2d::entity_catalog as catalog;
use ambition_platformer2d::extension::sdk::ModuleDescriptor;
use ambition_demo_smash_modules as modules;

/// One special: its authored key, the offer that admits it, and its module.
pub struct Special {
    pub key: &'static str,
    pub offer: TechniqueOffer,
    pub module: fn() -> ModuleDescriptor,
}

fn action(owner: &'static str, params: TechniqueParams, references: NestedReferences) -> TechniqueOffer {
    TechniqueOffer { owner, params, references, delivery: TechniqueDelivery::Action }
}

/// Every special this crate installs.
pub fn specials() -> Vec<Special> {
    vec![
        // Sing. Authored sleeps today: the performer's `speech` pulse and the
        // Shadow Oni leader's seal.
        Special {
            key: catalog::smash_sleep::SLEEP,
            offer: action(
                "ambition_demo_smash_modules::sing",
                TechniqueParams::Checked(catalog::smash_sleep::check_sleep_params),
                NestedReferences::None,
            ),
            module: modules::sing::module,
        },
        // Portal recovery. A pair is held open by a status on the fighter
        // (`ambition.portal.held_pair`), so it goes with her.
        Special {
            key: catalog::smash_portal::PORTAL_PAIR,
            offer: action(
                "ambition_demo_smash_modules::portal",
                TechniqueParams::Checked(check_hydrates::<catalog::smash_portal::PortalPairParams>),
                NestedReferences::None,
            ),
            module: modules::portal::module,
        },
        // Steered bolt: a module entity, put on the stage by the press and
        // flown from the next tick by its caster's stick. The params check
        // refuses an invisible bolt, a trail never redrawn, an unsteerable one.
        Special {
            key: catalog::smash_bolt::STEERED_BOLT,
            offer: action(
                "ambition_demo_smash_modules::bolt",
                TechniqueParams::Checked(catalog::smash_bolt::check_steered_bolt_params),
                NestedReferences::None,
            ),
            module: modules::bolt::module,
        },
        // Homing dash: it asks the engine only who is a foe
        // (`ambition.combat.foes`) and sets its fighter's velocity.
        Special {
            key: catalog::smash_homing::HOMING_DASH,
            offer: action(
                "ambition_demo_smash_modules::homing",
                TechniqueParams::Checked(check_hydrates::<catalog::smash_homing::HomingDashParams>),
                NestedReferences::None,
            ),
            module: modules::homing::module,
        },
        // Riposte: it reads the same `ActorActionMessage` the counter writes,
        // so it needs no counter-specific wiring.
        Special {
            key: catalog::smash_riposte::RIPOSTE_STRIKE,
            offer: action(
                "ambition_demo_smash_modules::riposte",
                TechniqueParams::Checked(catalog::smash_riposte::check_riposte_strike_params),
                NestedReferences::None,
            ),
            module: modules::riposte::module,
        },
        // Tether reel: a line that bit a ledge pulls on that tick, and a status
        // on the fighter reels her after. The movement kernel's ledge
        // authority catches the ledge, one phase later.
        Special {
            key: catalog::smash_tether::TETHER_PULL,
            offer: action(
                "ambition_demo_smash_modules::tether",
                TechniqueParams::Checked(check_hydrates::<catalog::smash_tether::TetherPullParams>),
                NestedReferences::None,
            ),
            module: modules::tether::module,
        },
        // Placed spring: the drop puts a module entity on the stage, and the
        // module ticks it.
        Special {
            key: catalog::smash_spring::PLACE_SPRING,
            offer: action(
                "ambition_demo_smash_modules::spring",
                TechniqueParams::Checked(catalog::smash_spring::check_place_spring_params),
                NestedReferences::None,
            ),
            module: modules::spring::module,
        },
        // A move that fills its own meter (`ambition.resources.refill`). A
        // body that holds no such meter is not filled.
        Special {
            key: catalog::smash_limit::FILL_METER,
            offer: action(
                "ambition_demo_smash_modules::limit",
                TechniqueParams::Checked(check_hydrates::<catalog::smash_limit::FillMeterParams>),
                NestedReferences::None,
            ),
            module: modules::limit::module,
        },
        // Time dilation: the module keeps the clock too, as a status on the
        // slowed body (`ambition.body.status_tick`).
        Special {
            key: catalog::smash_time_dilation::TIME_DILATION,
            offer: action(
                "ambition_demo_smash_modules::dilation",
                TechniqueParams::Checked(catalog::smash_time_dilation::check_time_dilation_params),
                NestedReferences::None,
            ),
            module: modules::dilation::module,
        },
        // Remote mine: a module entity that is a held item. It arms as it
        // ages, and module entities age in the player phase, before this
        // tick's presses, so a mine that becomes live this tick answers this
        // tick's press.
        Special {
            key: catalog::smash_mine::PLACE_MINE,
            offer: action(
                "ambition_demo_smash_modules::mine",
                TechniqueParams::Checked(catalog::smash_mine::check_place_mine_params),
                NestedReferences::HeldItems(catalog::smash_mine::mine_held_item_refs),
            ),
            module: modules::mine::module,
        },
        // Mark: answered on the body a volume struck (`ambition.combat.struck`,
        // `hit_reaction`), so the strike's own tick applies it and spends none
        // of its fuse.
        Special {
            key: catalog::smash_mark::MARK_BODY,
            offer: TechniqueOffer {
                owner: "ambition_demo_smash_modules::mark",
                params: TechniqueParams::Checked(catalog::smash_mark::check_mark_body_params),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::OnHit,
            },
            module: modules::mark::module,
        },
        // Counter stance: it holds the parry window open each tick of the
        // stance (`ambition.combat.parry_window`, lowered inside `Materialize`,
        // before `Resolve` judges), and answers a parry caught in it
        // (`ambition.combat.parried`). The answer lands on the next tick, and
        // is admitted as an effect of its own.
        Special {
            key: catalog::smash_counter::COUNTER,
            offer: action(
                "ambition_demo_smash_modules::counter",
                TechniqueParams::Checked(check_hydrates::<catalog::smash_counter::CounterParams>),
                NestedReferences::Effects(catalog::smash_counter::counter_response_refs),
            ),
            module: modules::counter::module,
        },
        // Shark ride: it summons the ride (`ambition.world.summon_ride`), and a
        // status on the rider and another on the shark end the ride and send
        // the shark away.
        Special {
            key: catalog::smash_ride::SUMMON_RIDE,
            offer: action(
                "ambition_demo_smash_modules::shark",
                TechniqueParams::Checked(catalog::smash_ride::check_summon_ride_params),
                NestedReferences::Characters(catalog::smash_ride::summon_ride_character_refs),
            ),
            module: modules::shark::module,
        },
        // Bomb: a module entity that is a held item, ticked with the items
        // after item physics (`module_entity_tick`), so an impact is read on
        // the tick it happens.
        Special {
            key: catalog::smash_bomb::DROP_BOMB,
            offer: action(
                "ambition_demo_smash_modules::bomb",
                TechniqueParams::Checked(catalog::smash_bomb::check_drop_bomb_params),
                NestedReferences::HeldItems(catalog::smash_bomb::bomb_held_item_refs),
            ),
            module: modules::bomb::module,
        },
    ]
}

/// Install every special for its key. A key this crate already installed is
/// skipped, so each game that authors them calls this; a key another owner
/// declared is still a conflict, and panics.
pub fn install(app: &mut bevy::prelude::App) {
    for special in specials() {
        let installed = app
            .world()
            .get_resource::<InstalledTechniques>()
            .and_then(|support| support.0.offer(special.key))
            .is_some_and(|held| held.owner == special.offer.owner);
        if installed {
            continue;
        }
        ambition_platformer2d::runtime::install_module_technique(app, special.key, special.offer, (special.module)());
    }
}

#[cfg(test)]
mod tests;
