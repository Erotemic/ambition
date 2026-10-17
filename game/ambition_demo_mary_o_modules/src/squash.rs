//! The squash: a player on the head of an AI Slop, or of a Laden Swallow,
//! bounces up and squashes it. The classic contact stomp, not the engine's
//! attack-hitbox pogo.
//!
//! "On its head" is the shared [`crate::stomp::PlayerTouch::Top`] rule, so
//! this and the snake's shell can not disagree about the same contact, and a
//! player who stands still on a mob is stomping it, not being hurt by it. A
//! side touch is left alone here and lands as the shared contact damage.
//!
//! It decides in `body_contact`, before the shared contact damage, so the
//! squashed mob hurts nobody in that tick (`ambition.body.defeat`). It has no
//! score value and no drop table, so the squash is the whole of its end; the
//! dust is the visible pop.

use ambition_combat_port::{
    BodyContact, BodyContactPort, BodyDefeat, BodyDefeatPort, Burst, BurstPort, ContactBounce, ContactBouncePort,
};
use ambition_extension_sdk::{
    phases::BODY_CONTACT, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

use crate::rules::{StompRules, StompRulesPort};
use crate::stomp::{player_touch, Bounds, PlayerTouch};

/// The brain key of an AI Slop placement.
pub const AI_SLOP_BRAIN_KEY: &str = "mary_o_ai_slop";

/// The low, tan dust of a squash.
const DUST: (u32, f32, [f32; 4]) = (12, 130.0, [0.80, 0.68, 0.48, 1.0]);

fn entry(selector: &'static str) -> EntryDescriptor {
    EntryDescriptor {
        key: selector.into(),
        phase: BODY_CONTACT,
        trigger: TriggerBinding { port: BodyContactPort::KEY, selector: selector.into() },
        // The band and the bounce are her pack's (`stomp`), the snake's too,
        // so a bounce off either enemy feels the same.
        reads: vec![StompRulesPort::KEY],
        writes: Vec::new(),
        requests: vec![ContactBouncePort::KEY, BurstPort::KEY, BodyDefeatPort::KEY],
        after: Vec::new(),
        limits: Limits { max_requests: 3 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(squash),
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "squash"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![entry(AI_SLOP_BRAIN_KEY), entry(crate::laden_swallow::BRAIN_KEY)],
    }
}

/// The first toucher (in identity order) that is on the mob's head, if any.
pub fn stomper(contact: &BodyContact, rules: StompRules) -> Option<usize> {
    let body = Bounds::of(contact.body.position, contact.body.size);
    contact
        .touchers
        .iter()
        .position(|t| player_touch(body, Bounds::of(t.position, t.size), t.velocity, rules.band) == Some(PlayerTouch::Top))
}

fn squash(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let contact: BodyContact = inv.trigger::<BodyContactPort>()?.clone();
    let rules = *inv.observe::<StompRulesPort>()?;
    let Some(stomper) = stomper(&contact, rules) else {
        return Ok(());
    };
    inv.submit::<ContactBouncePort>(ContactBounce { toucher: stomper as u32, speed: rules.bounce_speed, pogo_cue: true })?;
    let (count, speed, color) = DUST;
    inv.submit::<BurstPort>(Burst { at: contact.body.position, count, speed, color, kind: "dust".into() })?;
    inv.submit::<BodyDefeatPort>(BodyDefeat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::ContactBody;

    /// A mob 28 x 28 at (100, 100): its head is at y 86.
    fn mob() -> ContactBody {
        ContactBody { position: [100.0, 100.0], size: [28.0, 28.0], velocity: [0.0, 0.0] }
    }

    /// A player 28 x 40 at `x` with its feet on the mob's head.
    fn on_head(x: f32, vel_y: f32) -> ContactBody {
        ContactBody { position: [x, 86.0 - 20.0 + 1.0], size: [28.0, 40.0], velocity: [0.0, vel_y] }
    }

    /// A falling player on the head, or one at rest there, stomps; a rising
    /// one is taking a hit from below, and a side touch is the contact
    /// damage's.
    #[test]
    fn a_falling_or_resting_player_on_the_head_stomps_and_a_rising_one_does_not() {
        let contact = |touchers: Vec<ContactBody>| BodyContact { body: mob(), touchers };
        // A band wider than half the mob: the half rules.
        let stomper_of = |contact: &BodyContact| stomper(contact, StompRules { band: 16.0, bounce_speed: 430.0 });
        assert_eq!(stomper_of(&contact(vec![on_head(100.0, 120.0)])), Some(0), "falling onto it");
        assert_eq!(stomper_of(&contact(vec![on_head(100.0, 0.0)])), Some(0), "at rest on it");
        assert_eq!(stomper_of(&contact(vec![on_head(100.0, -80.0)])), None, "rising into it");
        let beside = ContactBody { position: [120.0, 100.0], size: [28.0, 40.0], velocity: [0.0, 0.0] };
        assert_eq!(stomper_of(&contact(vec![beside])), None, "beside it");
        assert_eq!(stomper_of(&contact(vec![beside, on_head(95.0, 60.0)])), Some(1), "the toucher on the head, not the first");
    }
}
