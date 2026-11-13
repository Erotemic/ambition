//! A hosted world's replication: the host's bodies reach a guest App by
//! `SimId` (NET-AUTHORITY-EXPERIMENT, `engine/hosted-world-networking-experiment.md`).
//!
//! Both Apps build a room's bodies themselves, from the same definition, so a
//! body has the same `SimId` on both. The host marks each body for replication
//! with a [`Signature`] of its `SimId`, and the guest puts the same signature on
//! the body it built: replication continues into that body instead of making a
//! second one. What the host sends is the body row (NA-2): where the body is,
//! how it moves and which way it faces; and its life row (NA-2b): its health
//! and its death's beat, which is what draws a defeat.
//!
//! The link is the caller's: `bevy_replicon` has no I/O of its own.

use ambition_characters::actor::{BodyHealth, Corpse};
use ambition_combat::components::DeathBeat;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

/// Where a body is, how it moves and which way it faces: what a guest needs to
/// draw it where the host has it (NA-2's 5 floats).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyRow {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub facing: f32,
}

impl BodyRow {
    pub fn of(body: &BodyKinematics) -> Self {
        Self { pos: body.pos.to_array(), vel: body.vel.to_array(), facing: body.facing }
    }

    pub fn write_into(&self, body: &mut BodyKinematics) {
        body.pos = Vec2::from_array(self.pos);
        body.vel = Vec2::from_array(self.vel);
        body.facing = self.facing;
    }
}

/// A body's life: its health, and its death's beat (seconds since it died,
/// whether it lived, what it leaves). The guest simulates no death, so it
/// draws one from this (NA-2b: without the beat, never).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifeRow {
    pub current: i32,
    pub since_s: Option<f32>,
    pub lived: bool,
    pub corpse: u8,
}

impl LifeRow {
    pub fn of(health: &BodyHealth, beat: Option<&DeathBeat>) -> Self {
        Self {
            current: health.health.current,
            since_s: beat.and_then(|beat| beat.since_s),
            lived: beat.is_some_and(|beat| beat.lived),
            corpse: beat.map_or(0, |beat| corpse_byte(beat.corpse)),
        }
    }

    pub fn write_into(&self, health: &mut BodyHealth, beat: Option<&mut DeathBeat>) {
        health.health.current = self.current;
        if let Some(beat) = beat {
            beat.since_s = self.since_s;
            beat.lived = self.lived;
            beat.corpse = corpse_of(self.corpse);
        }
    }
}

fn corpse_byte(corpse: Corpse) -> u8 {
    match corpse {
        Corpse::Stay => 0,
        Corpse::Vanish => 1,
        Corpse::Gone => 2,
    }
}

fn corpse_of(byte: u8) -> Corpse {
    match byte {
        0 => Corpse::Stay,
        2 => Corpse::Gone,
        _ => Corpse::Vanish,
    }
}

/// Which end of a hosted world this App is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedRole {
    /// Owns the world: simulates it and sends its bodies.
    Host,
    /// Shows the host's world: applies the bodies it is sent.
    Guest,
}

/// The replication of a hosted world's bodies. Add it with `RepliconPlugins`
/// (and a link) to both Apps.
pub struct HostedWorldPlugin {
    pub role: HostedRole,
}

impl Plugin for HostedWorldPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<BodyRow>().replicate::<LifeRow>();
        match self.role {
            HostedRole::Host => {
                app.add_systems(Update, mark_hosted_bodies);
                app.add_systems(PostUpdate, (write_body_rows, write_life_rows).before(ServerSystems::Send));
            }
            HostedRole::Guest => {
                app.add_systems(Update, sign_own_bodies);
                app.add_systems(PreUpdate, (apply_body_rows, apply_life_rows).after(ClientSystems::Receive));
            }
        }
    }
}

/// A body is replicated under the signature of its `SimId`.
fn signature() -> Signature {
    Signature::of::<SimId>()
}

/// Mark each new body of the host's world for replication.
fn mark_hosted_bodies(
    mut commands: Commands,
    bodies: Query<Entity, (With<SimId>, With<BodyKinematics>, Without<Replicated>)>,
) {
    for body in &bodies {
        commands.entity(body).insert((Replicated, signature()));
    }
}

/// The guest signs the bodies it built, so the host's replication continues
/// into them.
fn sign_own_bodies(
    mut commands: Commands,
    bodies: Query<Entity, (With<SimId>, With<BodyKinematics>, Without<Signature>)>,
) {
    for body in &bodies {
        commands.entity(body).insert(signature());
    }
}

/// Write each replicated body's row from its kinematics, before the send.
fn write_body_rows(
    mut commands: Commands,
    mut bodies: Query<(Entity, &BodyKinematics, Option<&mut BodyRow>), With<Replicated>>,
) {
    for (entity, body, row) in &mut bodies {
        let now = BodyRow::of(body);
        match row {
            Some(mut row) => {
                row.set_if_neq(now);
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
    }
}

/// Write each replicated body's life row, before the send.
fn write_life_rows(
    mut commands: Commands,
    mut bodies: Query<(Entity, &BodyHealth, Option<&DeathBeat>, Option<&mut LifeRow>), With<Replicated>>,
) {
    for (entity, health, beat, row) in &mut bodies {
        let now = LifeRow::of(health, beat);
        match row {
            Some(mut row) => {
                row.set_if_neq(now);
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
    }
}

/// Put each received life row on the guest's body.
fn apply_life_rows(mut bodies: Query<(&LifeRow, &mut BodyHealth, Option<&mut DeathBeat>), Changed<LifeRow>>) {
    for (row, mut health, beat) in &mut bodies {
        row.write_into(&mut health, beat.map(|beat| beat.into_inner()));
    }
}

/// Put each received row on the guest's body.
fn apply_body_rows(mut bodies: Query<(&BodyRow, &mut BodyKinematics), Changed<BodyRow>>) {
    for (row, mut body) in &mut bodies {
        row.write_into(&mut body);
    }
}

/// `bevy_replicon` itself, so a caller names the version this crate replicates with.
pub use bevy_replicon;
