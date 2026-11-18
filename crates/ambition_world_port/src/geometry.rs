//! World geometry identity and confirmed movement contacts.

ambition_extension_sdk::port_choices! {
    pub enum GeometryOrigin {
        Placement = 1 => "placement",
        TileLayer = 2 => "tile_layer",
        Generator = 3 => "generator",
        Delta = 4 => "delta",
        Anonymous = 5 => "anonymous",
    }
}
ambition_extension_sdk::port_choices! {
    pub enum ContactRole {
        Support = 1 => "support",
        Head = 2 => "head",
        Side = 3 => "side",
        Attachment = 4 => "attachment",
    }
}
ambition_extension_sdk::port_choices! {
    pub enum BlinkTier { Soft = 1 => "soft", Hard = 2 => "hard" }
}
ambition_extension_sdk::port_choices! {
    pub enum BlockMaterial {
        Solid = 1 => "solid",
        OneWay = 2 => "one_way",
        BonkOnly = 3 => "bonk_only",
        BlinkWall = 4 => "blink_wall",
        Barrier = 5 => "barrier",
        Hazard = 6 => "hazard",
        PogoOrb = 7 => "pogo_orb",
        Rebound = 8 => "rebound",
    }
}

/// A block identity in the sample's live room. Names do not identify geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryId {
    pub origin: GeometryOrigin,
    pub key: Option<String>,
    pub operation: Option<u32>,
    pub index: u32,
}
ambition_extension_sdk::port_record! {
    GeometryId {
        origin: GeometryOrigin,
        /// Placement ID, tile layer, or generator placement ID. Absent otherwise.
        key: Option<String>,
        /// The delta operation number. Present only for delta geometry.
        operation: Option<u32>,
        /// The emission ordinal within this source.
        index: u32,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BlockContact {
    pub geometry: GeometryId,
    pub material: BlockMaterial,
    pub blink_tier: Option<BlinkTier>,
    pub rebound_impulse: Option<[f32; 2]>,
}
ambition_extension_sdk::port_record! {
    BlockContact {
        geometry: GeometryId,
        material: BlockMaterial,
        blink_tier: Option<BlinkTier>,
        rebound_impulse: Option<[f32; 2]>,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChainContact {
    pub chain: u32,
    pub segment: u32,
}
ambition_extension_sdk::port_record! {
    ChainContact {
        /// Current collision-world index. This is not a durable object ID.
        chain: u32,
        segment: u32,
    }
}

/// One native movement contact. Exactly one of block and chain is present.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldContact {
    pub role: ContactRole,
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub toi: f32,
    pub surface_velocity: [f32; 2],
    pub impact_speed: f32,
    pub involuntary: bool,
    pub block: Option<BlockContact>,
    pub chain: Option<ChainContact>,
}
ambition_extension_sdk::port_record! {
    WorldContact {
        /// Role assigned by the solver against the body's resolved frame.
        role: ContactRole,
        /// Surface point in the body's root room chart, with +Y down.
        point: [f32; 2],
        /// Surface outward normal, toward the body, in the same chart.
        normal: [f32; 2],
        /// Fraction along the attempted step. Rest and repair contacts use zero.
        toi: f32,
        /// Native surface motion for this step. This is not a per-second speed.
        surface_velocity: [f32; 2],
        /// Body closing speed before resolution, in pixels per second.
        impact_speed: f32,
        involuntary: bool,
        block: Option<BlockContact>,
        chain: Option<ChainContact>,
    }
}
