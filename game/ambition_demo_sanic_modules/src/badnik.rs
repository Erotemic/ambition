//! The badnik's squash: what a player's touch does to a speedway badnik.
//!
//! A player falling onto a badnik's head bounces off and squashes it. A
//! player who rolls (the ball dash or the crouch roll), or whose touch harms
//! (the super form), squashes it on any touch and keeps its speed: a rolling
//! body does not bounce, and a super stomp still does. A side touch that is
//! neither is left alone here and lands as the shared contact damage.
//!
//! It decides in `body_contact`, before the shared contact damage, so the
//! squashed badnik hurts nobody in that tick (`ambition.body.defeat`). The
//! first toucher in identity order whose touch squashes is the one that does.
//! The band, the bounce, the bursts and the sound are his pack's, and whether
//! a toucher rolls or harms is the game's (`crate::rules`).

use ambition_combat_port::motion::Place;
use ambition_combat_port::riding::{Burst, BurstPort};
use ambition_combat_port::wielded::{BodySound, BodySoundPort};
use ambition_combat_port::{BodyContact, BodyContactPort, BodyDefeat, BodyDefeatPort, ContactBounce, ContactBouncePort};
use ambition_extension_sdk::{
    phases::BODY_CONTACT, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

use crate::rules::{BadnikRulesPort, ContactTouchersPort, ToucherFacts};

/// The brain key of the plain badnik (`EnemySpawn` with
/// `brain: "sanic_badnik"`).
pub const BADNIK_BRAIN_KEY: &str = "sanic_badnik";

/// Every badnik kind his levels place, by the brain key its placement
/// authors (his cast's badnik rows). Each is an entry of this module, so a
/// touch on each is decided here; a kind missing from this list is a plain
/// enemy whose touch only hurts.
pub const BADNIK_KINDS: [&str; 6] = [
    BADNIK_BRAIN_KEY,
    "sanic_badnik_liar",
    "sanic_badnik_noodle",
    "sanic_badnik_friend",
    "sanic_badnik_swarm",
    "sanic_badnik_saint",
];

/// The most bursts a squash may play (`effects.badnik_squash`): with the
/// bounce, the sound and the defeat, the entry's request limit.
pub const MAX_SQUASH_BURSTS: u32 = 9;

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "badnik"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: BADNIK_KINDS
            .iter()
            .map(|kind| EntryDescriptor {
                replies: Vec::new(),
                key: (*kind).into(),
                phase: BODY_CONTACT,
                trigger: TriggerBinding { port: BodyContactPort::KEY, selector: (*kind).into() },
                reads: vec![BadnikRulesPort::KEY, ContactTouchersPort::KEY],
                writes: Vec::new(),
                requests: vec![ContactBouncePort::KEY, BurstPort::KEY, BodySoundPort::KEY, BodyDefeatPort::KEY],
                after: Vec::new(),
                // A bounce, the sound, the defeat, and the pack's bursts.
                limits: Limits { max_requests: MAX_SQUASH_BURSTS + 3 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(squash),
            })
            .collect(),
    }
}

/// How a toucher squashes the badnik: its index, and whether it bounces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Squash {
    pub toucher: usize,
    pub bounce: bool,
}

/// The first player of the contact (in the trigger's identity order) whose
/// touch squashes the badnik. Screen gravity is +y: falling is `vel.y > 0`
/// and the feet are the bottom edge.
pub fn squasher(contact: &BodyContact, facts: &[ToucherFacts], band: f32) -> Option<Squash> {
    if contact.driven {
        return None;
    }
    let half = |size: [f32; 2]| [size[0] * 0.5, size[1] * 0.5];
    let [gx, gy] = contact.body.position;
    let [ghx, ghy] = half(contact.body.size);
    contact.touchers.iter().enumerate().find_map(|(index, toucher)| {
        if !toucher.driven {
            return None;
        }
        let ToucherFacts { rolling, harms_on_contact } = facts.get(index).copied().unwrap_or_default();
        let [px, py] = toucher.body.position;
        let [phx, phy] = half(toucher.body.size);
        let overlap_x = px - phx < gx + ghx && px + phx > gx - ghx;
        let overlap_y = py - phy < gy + ghy && py + phy > gy - ghy;
        let falling = toucher.body.velocity[1] > 0.0;
        let feet = py + phy;
        let top = gy - ghy;
        let stomp = falling && overlap_x && feet >= top - band && feet <= top + band;
        let roll = (rolling || harms_on_contact) && overlap_x && overlap_y;
        (stomp || roll).then_some(Squash { toucher: index, bounce: stomp && !rolling })
    })
}

fn squash(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let contact: BodyContact = inv.trigger::<BodyContactPort>()?.clone();
    let rules = inv.observe::<BadnikRulesPort>()?.clone();
    let facts = inv.observe::<ContactTouchersPort>()?.clone();
    let Some(squash) = squasher(&contact, &facts, rules.band) else {
        return Ok(());
    };
    if squash.bounce {
        inv.submit::<ContactBouncePort>(ContactBounce {
            toucher: squash.toucher as u32,
            speed: rules.bounce_speed,
            pogo_cue: false,
        })?;
    }
    for burst in &rules.squash {
        inv.submit::<BurstPort>(Burst {
            at: contact.body.position,
            count: burst.count,
            speed: burst.speed,
            color: burst.color,
            kind: burst.kind.clone(),
        })?;
    }
    inv.submit::<BodySoundPort>(BodySound { cue: rules.cue.clone(), at: Place::World(contact.body.position) })?;
    inv.submit::<BodyDefeatPort>(BodyDefeat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::{ContactBody, ContactParticipant};

    /// A badnik 28 x 32 at (0, 32): its head is at y 16.
    fn badnik() -> ContactBody {
        ContactBody { position: [0.0, 32.0], size: [28.0, 32.0], velocity: [0.0, 0.0] }
    }

    fn contact(touchers: Vec<ContactBody>) -> BodyContact {
        BodyContact {
            body: badnik(),
            touchers: touchers
                .into_iter()
                .map(|body| ContactParticipant { body, driven: true, ..Default::default() })
                .collect(),
            ..Default::default()
        }
    }

    fn player(at: [f32; 2], vel: [f32; 2]) -> ContactBody {
        ContactBody { position: at, size: [28.0, 32.0], velocity: vel }
    }

    const PLAIN: ToucherFacts = ToucherFacts { rolling: false, harms_on_contact: false };
    const ROLLING: ToucherFacts = ToucherFacts { rolling: true, harms_on_contact: false };
    const HARMS: ToucherFacts = ToucherFacts { rolling: false, harms_on_contact: true };

    /// The native rule's cases (`ambition_demo_sanic::badnik`'s tests, which
    /// this module replaces): a falling player on the head squashes and
    /// bounces; a rolling one squashes on overlap and does not bounce; one
    /// whose touch harms squashes on any touch and, falling on the head,
    /// still bounces; a rising or walking plain player does not.
    #[test]
    fn the_native_rules_cases() {
        let band = 8.0;
        let on_head_falling = contact(vec![player([0.0, 0.0], [0.0, 240.0])]);
        assert_eq!(squasher(&on_head_falling, &[PLAIN], band), Some(Squash { toucher: 0, bounce: true }));
        let overlapping = contact(vec![player([10.0, 20.0], [600.0, 0.0])]);
        assert_eq!(squasher(&overlapping, &[ROLLING], band), Some(Squash { toucher: 0, bounce: false }));
        assert_eq!(squasher(&overlapping, &[HARMS], band), Some(Squash { toucher: 0, bounce: false }));
        assert_eq!(squasher(&overlapping, &[PLAIN], band), None, "a plain walk-into is the contact damage's");
        assert_eq!(squasher(&on_head_falling, &[HARMS], band), Some(Squash { toucher: 0, bounce: true }), "a super stomp bounces");
        let rising = contact(vec![player([0.0, 0.0], [0.0, -200.0])]);
        assert_eq!(squasher(&rising, &[PLAIN], band), None);
        assert_eq!(squasher(&on_head_falling, &[], band), Some(Squash { toucher: 0, bounce: true }), "no facts: plain");
    }

    /// The first squashing toucher in the trigger's order is the one, and a
    /// toucher no seat drives, or a badnik a seat drives, squashes nothing.
    #[test]
    fn the_first_player_that_squashes_is_the_one() {
        let band = 8.0;
        let mut two = contact(vec![player([10.0, 20.0], [100.0, 0.0]), player([0.0, 0.0], [0.0, 240.0])]);
        assert_eq!(squasher(&two, &[PLAIN, PLAIN], band), Some(Squash { toucher: 1, bounce: true }));
        two.touchers[1].driven = false;
        assert_eq!(squasher(&two, &[PLAIN, PLAIN], band), None);
        let mut driven = contact(vec![player([0.0, 0.0], [0.0, 240.0])]);
        driven.driven = true;
        assert_eq!(squasher(&driven, &[PLAIN], band), None);
    }
}
