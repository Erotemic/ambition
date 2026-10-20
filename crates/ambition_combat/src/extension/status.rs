//! The adapters of a body's statuses: `ambition.body.hold_status`,
//! `ambition.body.status_tick`, `ambition.body.time_scale`,
//! `ambition.body.status_clock` and `ambition.body.in_play`. See the port
//! cards in `ambition_combat_port::status`.
//!
//! A status is a name a module holds on a body; its state is that module's
//! records on the body. This keeps only which names a body holds, so a move
//! that lives on a body for a while needs no component of its own here.

use std::collections::BTreeMap;

use ambition_combat_port::{HoldStatusPort, InPlayPort, StatusClockPort, StatusTick, StatusTickPort, TimeScalePort};
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox, InHitReaction, InTechniqueExecution,
    LowersIn,
};
use ambition_extension_sdk::phases::{HIT_REACTION, TECHNIQUE_EXECUTION};
use ambition_extension_sdk::Port as _;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Install the status trigger, its requests and the in-play fact in
/// `technique_execution`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<StatusTickPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", queue_status_ticks);
    app.install_extension_request::<HoldStatusPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_held_statuses::<InTechniqueExecution>,
    );
    app.install_extension_request::<TimeScalePort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_time_scales);
    app.install_extension_request::<StatusClockPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_status_clocks);
    app.install_extension_observation::<InPlayPort>(TECHNIQUE_EXECUTION, "ambition_combat", in_play);
}

/// Offer holding a status in `hit_reaction`, where a strike leaves one on the
/// body it struck.
pub fn install_for_hit_reaction(app: &mut App) {
    app.install_extension_request::<HoldStatusPort, _>(HIT_REACTION, "ambition_combat", lower_held_statuses::<InHitReaction>);
}

/// The statuses a body holds, by kind, in order. Never empty: a body that
/// holds none carries no component.
///
/// Rollback state: whether a body is ticked for a status decides what its
/// module does next.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyStatuses(pub Vec<String>);

impl BodyStatuses {
    pub fn holds(&self, kind: &str) -> bool {
        self.0.iter().any(|held| held == kind)
    }
}

/// The rollback value projection: every kind, in order.
pub fn body_statuses_probe(statuses: &BodyStatuses) -> u64 {
    // FNV-1a over the kinds and a separator, so ["ab"] and ["a", "b"] differ.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for kind in &statuses.0 {
        for byte in kind.bytes().chain(std::iter::once(0xff)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// How much of each status a body holds is left, by kind, in order: what a
/// player reads as a clock on the body. Never empty. A clock is read only
/// while its body holds its status, and letting go of the status removes it.
///
/// Rollback state, as `BodyStatuses` is: a rewind restores the clock with the
/// status, rather than leaving the last one drawn.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct StatusClocks(pub Vec<(String, f32)>);

impl StatusClocks {
    /// The clocks of the statuses `held` names, in order.
    pub fn shown<'a>(&'a self, held: &'a BodyStatuses) -> impl Iterator<Item = (&'a str, f32)> + 'a {
        self.0.iter().filter(|(kind, _)| held.holds(kind)).map(|(kind, remaining)| (kind.as_str(), *remaining))
    }
}

/// The rollback value projection: every kind and its clock in thousandths.
pub fn status_clocks_probe(clocks: &StatusClocks) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (kind, remaining) in &clocks.0 {
        let thousandths = (remaining * 1000.0).round() as u32;
        for byte in kind.bytes().chain(std::iter::once(0xff)).chain(thousandths.to_le_bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Whether `scope` is in play: `false` while it is `OutOfPlay`. Absent for a
/// scope that is not a body.
pub fn in_play(world: &World, scope: Entity) -> Option<bool> {
    world.get::<ae::BodyKinematics>(scope)?;
    Some(!world.entity(scope).contains::<crate::death_rules::OutOfPlay>())
}

/// One tick of each status that an admitted entry binds, on each body that
/// holds it, in the order of the bodies' simulation identity, then entity.
pub fn queue_status_ticks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<(
        Entity,
        &BodyStatuses,
        &ae::BodyKinematics,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
) {
    // A paused world ticks no status (see the port card).
    if time.sim_dt() <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == StatusTickPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut holders: Vec<_> = bodies.iter().collect();
    holders.sort_by(|a, b| a.3.cmp(&b.3).then(a.0.cmp(&b.0)));
    for (body, statuses, kin, _) in holders {
        for kind in statuses.0.iter().filter(|kind| bound.contains(&kind.as_str())) {
            invocations.trigger::<StatusTickPort>(
                &TECHNIQUE_EXECUTION,
                kind.clone(),
                body,
                None,
                false,
                StatusTick { position: [kin.pos.x, kin.pos.y], facing: kin.facing.signum() },
            );
        }
    }
}

/// Hold or let go of each status a module asked for, in the order asked.
///
/// The requests of a tick are applied to one staged copy per body before any
/// is written back, so a let-go and a later hold of one kind leave it held.
pub fn lower_held_statuses<L: LowersIn>(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut holders: Query<&mut BodyStatuses>,
    mut clocks: Query<&mut StatusClocks>,
    bodies: Query<(), With<ae::BodyKinematics>>,
) {
    let mut staged: BTreeMap<Entity, Vec<String>> = BTreeMap::new();
    for submitted in outbox.drain::<HoldStatusPort>(&L::PHASE) {
        let hold = submitted.value;
        if hold.kind.is_empty() {
            warn!("extension entry {} asked to hold a status with no kind; refused", submitted.entry);
            continue;
        }
        if !bodies.contains(submitted.scope) {
            continue;
        }
        let kinds = staged
            .entry(submitted.scope)
            .or_insert_with(|| holders.get(submitted.scope).map(|held| held.0.clone()).unwrap_or_default());
        let at = kinds.binary_search(&hold.kind);
        match (hold.held, at) {
            (true, Err(slot)) => kinds.insert(slot, hold.kind),
            (false, Ok(slot)) => {
                kinds.remove(slot);
            }
            _ => {}
        }
    }
    for (body, kinds) in staged {
        // A status let go of takes its clock with it.
        if let Ok(mut clock) = clocks.get_mut(body) {
            if clock.0.iter().any(|(kind, _)| kinds.binary_search(kind).is_err()) {
                clock.0.retain(|(kind, _)| kinds.binary_search(kind).is_ok());
                if clock.0.is_empty() {
                    commands.entity(body).try_remove::<StatusClocks>();
                }
            }
        }
        match (holders.get_mut(body), kinds.is_empty()) {
            (Ok(mut held), false) => {
                if held.0 != kinds {
                    held.0 = kinds;
                }
            }
            (Ok(_), true) => {
                commands.entity(body).try_remove::<BodyStatuses>();
            }
            (Err(_), false) => {
                commands.entity(body).try_insert(BodyStatuses(kinds));
            }
            (Err(_), true) => {}
        }
    }
}

/// Set each body's clock scale a module asked for, in the order asked.
pub fn lower_time_scales(mut commands: Commands, mut outbox: ResMut<ExtensionOutbox>) {
    for submitted in outbox.drain::<TimeScalePort>(&TECHNIQUE_EXECUTION) {
        let scale = submitted.value.scale;
        if !scale.is_finite() || scale <= 0.0 {
            warn!("extension entry {} asked for a clock scale of {scale}; refused", submitted.entry);
            continue;
        }
        commands.entity(submitted.scope).try_insert(ambition_time::ProperTimeScale(scale));
    }
}

/// Set how much of each status is left that a module said, in the order said.
pub fn lower_status_clocks(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut clocks: Query<&mut StatusClocks>,
    bodies: Query<(), With<ae::BodyKinematics>>,
) {
    let mut staged: BTreeMap<Entity, Vec<(String, f32)>> = BTreeMap::new();
    for submitted in outbox.drain::<StatusClockPort>(&TECHNIQUE_EXECUTION) {
        let clock = submitted.value;
        if clock.kind.is_empty() || !clock.remaining.is_finite() {
            warn!(
                "extension entry {} said a status `{}` has {} left; refused",
                submitted.entry, clock.kind, clock.remaining
            );
            continue;
        }
        if !bodies.contains(submitted.scope) {
            continue;
        }
        let kinds = staged
            .entry(submitted.scope)
            .or_insert_with(|| clocks.get(submitted.scope).map(|held| held.0.clone()).unwrap_or_default());
        let remaining = clock.remaining.clamp(0.0, 1.0);
        match kinds.binary_search_by(|(kind, _)| kind.cmp(&clock.kind)) {
            Ok(slot) => kinds[slot].1 = remaining,
            Err(slot) => kinds.insert(slot, (clock.kind, remaining)),
        }
    }
    for (body, kinds) in staged {
        match clocks.get_mut(body) {
            Ok(mut held) => {
                if held.0 != kinds {
                    held.0 = kinds;
                }
            }
            Err(_) => {
                commands.entity(body).try_insert(StatusClocks(kinds));
            }
        }
    }
}

#[cfg(test)]
mod tests;
