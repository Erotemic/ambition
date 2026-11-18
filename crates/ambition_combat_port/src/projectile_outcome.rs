//! Bounded facts from native projectile creation, interception and retirement.

use ambition_extension_sdk::{
    Port, PortKey, PortRole,
    wire::{WireError, WireReader},
};

ambition_extension_sdk::port_choices! {
    pub enum ProjectileOutcomeKind {
        Created = 0 => "created",
        Reflected = 1 => "reflected",
        Absorbed = 2 => "absorbed",
        SpentByClash = 3 => "spent_by_clash",
        Expired = 4 => "expired",
        BodyContact = 5 => "body_contact",
        FeatureContact = 6 => "feature_contact",
        WorldContact = 7 => "world_contact",
        RoomRetired = 8 => "room_retired",
        AttemptReset = 9 => "attempt_reset",
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileParticipant {
    pub id: Option<String>,
    pub room: Option<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileOutcome {
    pub observed_at: u64,
    pub projectile: ProjectileParticipant,
    pub credit: Option<ProjectileParticipant>,
    pub previous_credit: Option<ProjectileParticipant>,
    pub interceptor: Option<ProjectileParticipant>,
    pub kind: ProjectileOutcomeKind,
    pub terminal: bool,
    pub position: [f32; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileOutcomes {
    pub events: Vec<ProjectileOutcome>,
    pub truncated: bool,
}
ambition_extension_sdk::port_record! {
    ProjectileParticipant { id: Option<String>, room: Option<u32>, }
}
ambition_extension_sdk::port_record! {
    ProjectileOutcome {
        observed_at: u64,
        projectile: ProjectileParticipant,
        /// Credit after this operation. It does not require a live body link.
        credit: Option<ProjectileParticipant>,
        /// Present only when reflection changed credit.
        previous_credit: Option<ProjectileParticipant>,
        interceptor: Option<ProjectileParticipant>,
        kind: ProjectileOutcomeKind,
        /// This operation removed the shot. Pending damage can still resolve.
        terminal: bool,
        /// The shot root in its declared room, not a receiver contact point.
        position: [f32; 2],
    }
}
ambition_extension_sdk::port_record! {
    ProjectileOutcomes { events: Vec<ProjectileOutcome>, truncated: bool, }
}
/// Native facts at the next `hit_reaction` read cut. Held statuses bind body
/// observers; game rules bind room observers. Damage has its own result port.
pub struct ProjectileOutcomesPort;
impl Port for ProjectileOutcomesPort {
    const KEY: PortKey = PortKey::new("ambition.combat.projectile_outcomes", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = ProjectileOutcomes;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(ProjectileOutcomesPort);
