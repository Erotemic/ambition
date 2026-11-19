//! Bounded body queries publish identified facts at a declared read cut.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{
    Port, PortKey, PortRole,
    action::{ActionTicket, ReplyTo},
};

use crate::targets::TargetRelation;

#[derive(Clone, Debug, PartialEq)]
pub struct BodyQuery {
    pub reply: ReplyTo,
    pub origin: [f32; 2],
    pub radius: f32,
    pub limit: u32,
    pub relation: Option<TargetRelation>,
    pub include_self: bool,
    pub identities: Vec<String>,
    pub require_all: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyTarget {
    pub id: String,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub velocity: [f32; 2],
    pub driven: bool,
    pub relation: Option<TargetRelation>,
    pub distance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyQueryResult {
    pub ticket: ActionTicket,
    pub observed_at: u64,
    pub room: u32,
    pub phase: String,
    pub truncated: bool,
    pub targets: Vec<BodyTarget>,
}

ambition_extension_sdk::port_record! {
    BodyQuery {
        reply: ReplyTo,
        /// A point in the caller's live room, in world units with +Y down.
        origin: [f32; 2],
        /// A finite radius around body centres, in world units.
        radius: f32,
        /// Maximum returned bodies. The domain also enforces its own ceiling.
        limit: u32,
        relation: Option<TargetRelation> = None,
        include_self: bool = false,
        /// Optional exact identities from an earlier read. Empty searches the area.
        identities: Vec<String> = Vec::new(),
        /// Reject if an exact identity is absent, out of play, or outside the filters.
        require_all: bool = false,
    }
}
ambition_extension_sdk::port_record! {
    BodyTarget {
        id: String,
        /// The physical body's centre, in the room's coordinates.
        position: [f32; 2],
        /// The physical body's full extent under its current gravity.
        size: [f32; 2],
        velocity: [f32; 2],
        driven: bool,
        relation: Option<TargetRelation>,
        /// Distance from the query origin to the body centre.
        distance: f32,
    }
}
ambition_extension_sdk::port_record! {
    BodyQueryResult {
        ticket: ActionTicket,
        observed_at: u64,
        /// The live room where these facts were sampled.
        room: u32,
        phase: String,
        /// More matching bodies existed than the requested limit.
        truncated: bool,
        /// Nearest first, with simulation identity as the tie-break.
        targets: Vec<BodyTarget>,
    }
}
pub struct BodyQueryPort;
pub struct BodyQueryResultPort;
macro_rules! query_port {
    ($port:ty, $key:literal, $role:ident, $value:ty $(, $action:item)?) => {
        impl Port for $port {
            const KEY: PortKey = PortKey::new($key, 1);
            const ROLE: PortRole = PortRole::$role;
            type Value = $value;
            $($action)?
            fn encode(value: &Self::Value, out: &mut Vec<u8>) {
                ambition_extension_sdk::structure::put(value, out);
            }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
                ambition_extension_sdk::structure::read(reader)
            }
        }
        ambition_extension_sdk::script_port!($port);
    };
}
query_port!(
    BodyQueryPort,
    "ambition.combat.body_query",
    Request,
    BodyQuery,
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "combat.body_query", &value.reply))
    }
);
query_port!(
    BodyQueryResultPort,
    "ambition.combat.body_query_result",
    Observation,
    Option<BodyQueryResult>
);
