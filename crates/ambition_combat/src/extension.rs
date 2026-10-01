//! The combat domain's extension request adapter: a module's damage box
//! enters the effect executor's one road (`EffectRequest` →
//! `Effect::DamageBox`), owned by the body the invocation ran for and on that
//! body's EFFECTIVE faction (`targeting::effective_faction`: a driven body
//! fights as the player). See the port card on `ambition_combat_port`.

use ambition_combat_port::DamageBoxPort;
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::components::ActorFaction;

/// Install the request port in `technique_execution`, before the effect
/// executor.
pub fn install(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_damage_boxes,
    );
}

fn lower_damage_boxes(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    for submitted in outbox.drain::<DamageBoxPort>() {
        // ⛔ SUBMITTED IS NOT APPLIED. A body with no faction cannot say whom
        // its box hurts, and the module may not say it either.
        let Ok((authored, driver)) = factions.get(submitted.scope) else {
            warn!(
                "extension entry {} asked for a damage box for {:?}, which has no faction; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let faction = crate::targeting::effective_faction(*authored, driver);
        let b = submitted.value;
        effects.write(ambition_vfx::EffectRequest {
            owner: submitted.scope,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: ae::Vec2::from(b.center),
                faction: crate::hit_side_from_actor_faction(faction),
                half_extent: ae::Vec2::from(b.half_extent),
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
                name: None,
            }),
        });
    }
}
