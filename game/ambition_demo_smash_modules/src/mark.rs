//! The delayed mark: a clock riding on the body that was hit.
//!
//! A move authors `smash.mark_body` as the on-hit effect of its volumes. When
//! one lands, the struck body holds the status [`MARKED`] and this module
//! keeps the mark's clock and its blast in a record on that body ([`Mark`]).
//! Each tick of the status spends the clock on the world's step and shows the
//! player how much is left; when it runs out, a blast goes off where the body
//! is then (`ambition.combat.blast`), so running away does not help.
//!
//! A second mark refreshes and does not stack, so the player can still read
//! "how long have I got", and the last striker owns the blast: a refresh is a
//! new mark, so the pressure and the credit are the refresher's. The credit is
//! the striker's (`Struck::striker`), never the marked body's; a striker that
//! cannot be credited leaves no mark.
//!
//! The mark lives as long as the stock it was put on: on a tick its body is
//! out of play it is retired with no blast, so it cannot go off through the
//! death interlude or on the next stock.

use ambition_combat_port::{
    Blast, BlastPort, HoldStatus, HoldStatusPort, InPlayPort, StatusClock, StatusClockPort, StatusTickPort, Struck,
    StruckPort,
};
use ambition_extension_sdk::{
    phases::{HIT_REACTION, TECHNIQUE_EXECUTION},
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a mark. The Smash crate installs
/// this module for its catalog's key (`install_module_technique`).
pub const MARK_BODY: &str = "smash.mark_body";

/// The status a marked body holds while its clock runs.
pub const MARKED: &str = "smash.marked";

/// The entry that spends a marked body's clock.
pub const TICK: &str = "smash.mark_tick";

record! {
    /// A live mark on the body that holds it.
    pub struct Mark = SchemaKey::new(crate::PROVIDER, "mark", 1);
    /// World seconds left before it goes off.
    1 fuse_s: f32,
    /// The fuse as authored, so the clock a player reads is a fraction.
    2 fuse_total_s: f32,
    3 damage: i32,
    /// Half-extent of the blast, centred on the marked body.
    4 blast_radius: f32,
    5 knockback: f32,
    /// How long the blast stays live.
    6 blast_s: f32,
    /// Who is credited with the blast: the last striker.
    7 credit: u64,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "mark"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Mark::schema()],
        entries: vec![
            EntryDescriptor {
                key: TICK.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: StatusTickPort::KEY, selector: MARKED.into() },
                reads: vec![InPlayPort::KEY],
                writes: vec![Mark::KEY],
                requests: vec![BlastPort::KEY, HoldStatusPort::KEY, StatusClockPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(tick),
            },
            EntryDescriptor {
                key: MARK_BODY.into(),
                phase: HIT_REACTION,
                trigger: TriggerBinding { port: StruckPort::KEY, selector: MARK_BODY.into() },
                reads: Vec::new(),
                writes: vec![Mark::KEY],
                requests: vec![HoldStatusPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(mark),
            },
        ],
    }
}

/// The mark a strike's params put on a body, credited to its striker. Every
/// number is the move's, which admission checked
/// (`check_mark_body_params`), so a missing one is a fault; so is a striker
/// with no credit, whose blast would credit somebody else.
pub fn mark_of(struck: &Struck) -> Result<Mark, Fault> {
    if struck.striker == 0 {
        return Err(Fault::Module("a mark whose striker cannot be credited is not applied".into()));
    }
    let number = |path: &'static str| struck.number(path).ok_or(Fault::Module(path.into()));
    let fuse_s = number("fuse_s")?;
    Ok(Mark {
        fuse_s,
        fuse_total_s: fuse_s,
        damage: number("damage")? as i32,
        blast_radius: number("blast_radius")?,
        knockback: number("knockback")?,
        blast_s: number("blast_s")?,
        credit: struck.striker,
    })
}

/// What a tick of a mark does, given whether its body is in play and the
/// world's step: the mark it leaves (none when it ends) and the blast it sets
/// off (none unless it ran out in play).
pub fn advance(mark: &Mark, in_play: bool, dt: f32) -> (Option<Mark>, Option<Blast>) {
    if !in_play {
        return (None, None);
    }
    let fuse_s = mark.fuse_s - dt;
    if fuse_s > 0.0 {
        return (Some(Mark { fuse_s, ..mark.clone() }), None);
    }
    let blast = Blast {
        half_extent: [mark.blast_radius, mark.blast_radius],
        damage: mark.damage,
        knockback: mark.knockback,
        lifetime_s: mark.blast_s,
        credit: mark.credit,
    };
    (None, Some(blast))
}

fn status(held: bool) -> HoldStatus {
    HoldStatus { kind: MARKED.into(), held }
}

/// Put a mark on the struck body, or refresh the one it has.
fn mark(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    mark_of(&struck)?.store(inv)?;
    inv.submit::<HoldStatusPort>(status(true))
}

/// Spend a tick of a mark: retire it from a body out of play, set off its
/// blast when it runs out, and otherwise show how much is left.
fn tick(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let mark = Mark::load(inv)?;
    let in_play = *inv.observe::<InPlayPort>()?;
    match advance(&mark, in_play, inv.dt()) {
        (Some(left), _) => {
            let remaining = if left.fuse_total_s > 0.0 { left.fuse_s / left.fuse_total_s } else { 0.0 };
            left.store(inv)?;
            inv.submit::<StatusClockPort>(StatusClock { kind: MARKED.into(), remaining })
        }
        (None, blast) => {
            Mark::default().store(inv)?;
            if let Some(blast) = blast {
                inv.submit::<BlastPort>(blast)?;
            }
            inv.submit::<HoldStatusPort>(status(false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn struck(striker: u64) -> Struck {
        Struck {
            contact: [0.0, 0.0],
            striker,
            params: [("fuse_s", 0.5), ("damage", 6.0), ("blast_radius", 44.0), ("knockback", 1.2), ("blast_s", 0.08)]
                .into_iter()
                .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
                .collect(),
        }
    }

    /// A mark is the strike's numbers, credited to the striker; a striker with
    /// no credit and a strike missing a number leave none.
    #[test]
    fn a_mark_is_what_the_strike_authors() {
        let mark = mark_of(&struck(7)).unwrap();
        assert_eq!(
            mark,
            Mark { fuse_s: 0.5, fuse_total_s: 0.5, damage: 6, blast_radius: 44.0, knockback: 1.2, blast_s: 0.08, credit: 7 }
        );
        assert!(mark_of(&struck(0)).is_err(), "a mark with no credit was applied");
        let mut short = struck(7);
        short.params.retain(|(path, _)| path != "blast_s");
        assert!(mark_of(&short).is_err(), "a mark with no blast time was applied");
    }

    /// A tick spends the fuse; the tick it runs out sets off the blast, square
    /// on the body, credited to the striker; a body out of play retires it
    /// with no blast however little is left.
    #[test]
    fn a_tick_spends_the_fuse_and_the_last_one_blasts() {
        let mark = mark_of(&struck(7)).unwrap();
        let (left, blast) = advance(&mark, true, 0.25);
        assert_eq!((left.map(|m| m.fuse_s), blast), (Some(0.25), None));
        let (left, blast) = advance(&Mark { fuse_s: 0.25, ..mark.clone() }, true, 0.25);
        assert!(left.is_none(), "a spent mark stayed");
        assert_eq!(
            blast,
            Some(Blast { half_extent: [44.0, 44.0], damage: 6, knockback: 1.2, lifetime_s: 0.08, credit: 7 })
        );
        assert_eq!(advance(&Mark { fuse_s: 0.01, ..mark }, false, 0.25), (None, None), "a mark went off out of play");
    }
}
