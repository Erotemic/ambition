//! Portable room story observations and action requests.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// One `actor_think` invocation for each live instance of the selected room
/// definition. Its scope is the room root. No invocation runs while its
/// room clock is held. The trigger contains simulation facts only.
pub struct StoryTickPort;
/// Submit an authored command in the invocation's room. Lowering prepares
/// the line against `CommandCatalog`. A bad line or absent room is refused
/// and logged. The command owner applies the effect through its usual path.
pub struct StoryCommandPort;
/// Queue a known cutscene in the invocation's room. Playback owns the queue,
/// skip policy and cursor. With a reply, the request is answered `queued`,
/// then `started` when playback begins, then terminal `cutscene_ended` when it
/// ends (played out or skipped); an unknown id, a caller with no live room or
/// a cutscene already seen is refused.
pub struct PlayCutscenePort;

#[derive(Clone, Debug, PartialEq)]
pub struct StoryTick {
    pub busy: bool,
    pub flags: Vec<String>,
    pub bodies: Vec<StoryBody>,
    pub marks: Vec<StoryMark>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryBody {
    pub id: String,
    pub form: String,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub on_ground: bool,
    pub alive: bool,
    pub driven: bool,
    pub struck_recently: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryCommand {
    pub line: String,
    pub reply: Option<ambition_extension_sdk::action::ReplyTo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayCutscene {
    pub id: String,
    pub reply: Option<ambition_extension_sdk::action::ReplyTo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryMark {
    pub name: String,
    pub position: [f32; 2],
}

ambition_extension_sdk::port_record! {
    StoryTick {
        /// A conversation or cutscene currently holds this room.
        busy: bool,
        /// The save flags that are true, sorted by name.
        flags: Vec<String>,
        /// This room's bodies, sorted by simulation identity.
        bodies: Vec<StoryBody>,
        /// Named LDtk labels and their points, in room pixels.
        marks: Vec<StoryMark>,
    }
}
ambition_extension_sdk::port_record! {
    StoryMark {
        /// The authored label name.
        name: String,
        /// The label's point in room pixels.
        position: [f32; 2],
    }
}
ambition_extension_sdk::port_record! {
    StoryBody {
        /// The body's simulation identity.
        id: String,
        /// The character id.
        form: String,
        /// The body's centre, in room pixels with +Y down.
        position: [f32; 2],
        /// The body's width and height.
        size: [f32; 2],
        /// The shared movement system reports ground contact.
        on_ground: bool,
        /// The body is alive.
        alive: bool,
        /// A participant drives this body.
        driven: bool,
        /// The shared hit resolver reports a recent strike on this body.
        struck_recently: bool,
    }
}
ambition_extension_sdk::port_record! {
    StoryCommand {
        /// A command line from the authored command catalog. It runs in this room.
        line: String,
        /// Optional correlated replies from the command owner.
        reply: Option<ambition_extension_sdk::action::ReplyTo> = None,
    }
}
ambition_extension_sdk::port_record! {
    PlayCutscene {
        /// A script id from the cutscene library. It plays in this room.
        id: String,
        /// Optional correlated replies from cutscene playback.
        reply: Option<ambition_extension_sdk::action::ReplyTo> = None,
    }
}

macro_rules! story_port {
    ($port:ty, $key:literal, $role:ident, $value:ty, $version:literal $(, $action:item)?) => {
        impl Port for $port {
            const KEY: PortKey = PortKey::new($key, $version);
            const ROLE: PortRole = PortRole::$role;
            type Value = $value;
            $($action)?
            fn encode(value: &Self::Value, out: &mut Vec<u8>) {
                ambition_extension_sdk::structure::put(value, out)
            }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
                ambition_extension_sdk::structure::read(reader)
            }
        }
        ambition_extension_sdk::script_port!($port);
    };
}
story_port!(StoryTickPort, "ambition.story.tick", Trigger, StoryTick, 1);
story_port!(
    StoryCommandPort,
    "ambition.story.command",
    Request,
    StoryCommand,
    2,
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(
            value, "authored.command", value.reply.as_ref()?))
    }
);
story_port!(
    PlayCutscenePort,
    "ambition.story.play_cutscene",
    Request,
    PlayCutscene,
    2,
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(
            value, "story.play_cutscene", value.reply.as_ref()?))
    }
);
