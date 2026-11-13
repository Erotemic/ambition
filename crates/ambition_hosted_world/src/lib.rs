//! A hosted world's replication: the host's bodies reach a guest App by
//! `SimId` (NET-AUTHORITY-EXPERIMENT, `engine/hosted-world-networking-experiment.md`).
//!
//! Both Apps build a room's bodies themselves, from the same definition, so a
//! body has the same `SimId` on both. The host marks each body for replication
//! with a [`Signature`] of its `SimId`, and the guest puts the same signature on
//! the body it built: replication continues into that body instead of making a
//! second one. What the host sends is the body row (NA-2): where the body is,
//! how it moves and which way it faces.
//!
//! The link is the caller's: `bevy_replicon` has no I/O of its own.

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
        app.replicate::<BodyRow>();
        match self.role {
            HostedRole::Host => {
                app.add_systems(Update, mark_hosted_bodies);
                app.add_systems(PostUpdate, write_body_rows.before(ServerSystems::Send));
            }
            HostedRole::Guest => {
                app.add_systems(Update, sign_own_bodies);
                app.add_systems(PreUpdate, apply_body_rows.after(ClientSystems::Receive));
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

/// Put each received row on the guest's body.
fn apply_body_rows(mut bodies: Query<(&BodyRow, &mut BodyKinematics), Changed<BodyRow>>) {
    for (row, mut body) in &mut bodies {
        row.write_into(&mut body);
    }
}

/// `bevy_replicon` itself, so a caller names the version this crate replicates with.
pub use bevy_replicon;
