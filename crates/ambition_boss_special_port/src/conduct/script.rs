//! Script forms of the native conduct observations.

use super::*;

ambition_extension_sdk::port_record! {
    RoomHall {
        /// World Y of the floor surface.
        floor: f32,
        /// World X of the left wall.
        left: f32,
        /// World X of the right wall.
        right: f32,
    }
}

ambition_extension_sdk::port_record! {
    LiveMove {
        /// The selected special key.
        key: String,
        /// Seconds left in this part of the move.
        remaining: f32,
    }
}

ambition_extension_sdk::port_record! {
    BossConduct {
        /// World centre, in world units with +Y down.
        position: [f32; 2],
        /// World velocity, in world units per second.
        velocity: [f32; 2],
        /// Body facing sign.
        facing: f32,
        /// Facing sign selected by the conductor.
        side: f32,
        /// The body has health left.
        alive: bool,
        /// The move whose warning is active.
        telegraph: Option<LiveMove>,
        /// The move whose strike is active.
        active: Option<LiveMove>,
        /// Last observed target position.
        target: Option<[f32; 2]>,
        /// Floor and side walls in this live room, when available.
        hall: Option<RoomHall>,
        /// A participant controls this body.
        driven: bool,
        /// The encounter is in its enrage phase.
        enraged: bool,
        /// The boss holds a body.
        holding: bool,
        /// Living non-boss summons in this encounter and live room.
        minions: u32,
        /// The encounter holds a transition between phases.
        between_phases: bool,
        /// Live room width and height, when available.
        room: Option<[f32; 2]>,
        /// Current body health.
        health: i32,
        /// The encounter currently refuses damage to the boss.
        invulnerable: bool,
        /// Current target body geometry and move, when available.
        target_body: Option<BodyBox>,
        /// Full body width and height.
        size: [f32; 2],
        /// The encounter has left its dormant phase.
        awake: bool,
        /// The predecessor boss in this live room, when available.
        predecessor: Option<Predecessor>,
        /// Free movement distances [left, right, up, down], each capped at REACH_CAP.
        reach: Option<[f32; 4]>,
    }
}

ambition_extension_sdk::port_record! {
    Predecessor {
        /// World centre, in world units with +Y down.
        position: [f32; 2],
        /// Full body width and height.
        size: [f32; 2],
        /// Facing sign selected by the conductor.
        side: f32,
        /// The body has health left.
        alive: bool,
        /// The floor and walls under it, when the room has a floor there.
        hall: Option<RoomHall>,
    }
}

ambition_extension_sdk::port_record! {
    BodyBox {
        /// World centre, in world units with +Y down.
        position: [f32; 2],
        /// Full body width and height.
        size: [f32; 2],
        /// The move this body currently plays, when available.
        swing: Option<BodySwing>,
    }
}

ambition_extension_sdk::port_record! {
    BodySwing {
        /// Seconds since the move started.
        since: f32,
        /// Seconds until the move can first hurt, or zero.
        hurts_in: f32,
        /// The move can hurt now.
        live: bool,
        /// Seconds until the move ends.
        ends_in: f32,
    }
}

ambition_extension_sdk::port_record! {
    MountLimb {
        /// Limb rig slot name.
        slot: String,
        /// World centre, in world units with +Y down.
        position: [f32; 2],
        /// Full body width and height.
        size: [f32; 2],
        /// Limb rest position relative to its mount.
        home_offset: [f32; 2],
        /// Maximum health less current health.
        lost_health: i32,
        /// The conductor holds this limb pose.
        held: bool,
        /// Current solid contact position, when available.
        contact: Option<[f32; 2]>,
        /// Bodies hit by the current limb strike generation.
        struck: u32,
    }
}

ambition_extension_sdk::port_record! {
    MountView {
        /// World centre, in world units with +Y down.
        position: [f32; 2],
        /// World velocity, in world units per second.
        velocity: [f32; 2],
        /// Full body width and height.
        size: [f32; 2],
        /// Body facing sign.
        facing: f32,
        /// The art keeps its authored facing.
        unmirrored: bool,
        /// Back box [min x, min y, max x, max y], when available.
        back: Option<[f32; 4]>,
        /// Standing ledges as [left x, left y, right x, right y].
        ledges: Vec<[f32; 4]>,
        /// Floor and side walls in this live room, when available.
        hall: Option<RoomHall>,
        /// Live limbs in rig slot order.
        limbs: Vec<MountLimb>,
        /// Walkable slope points in segment order; empty when absent.
        slope: Vec<[f32; 2]>,
    }
}

ambition_extension_sdk::script_port!(BossConductPort);
ambition_extension_sdk::script_port!(MountViewPort);
