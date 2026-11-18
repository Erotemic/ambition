//! Input sample order is independent of the estimated host tick. Render
//! frames in one tick share that tick and keep their own sequence numbers.
//!
//! Run-ahead: a guest stamps its input `lead` ticks past its estimate of the
//! host tick, so the input reaches the host before the tick it names runs
//! and the host has no late input to rewind for. The host reports how early
//! each input arrived ([`crate::InputMargin`]); the guest moves its lead so
//! its inputs arrive [`ARRIVAL_SLACK`] ticks early.

use std::collections::VecDeque;
use std::time::Duration;
use bevy::prelude::Resource;

/// How far past the last host tick it saw a guest's estimate of the host
/// tick may run on its own clock.
pub(crate) const MAX_INPUT_LEAD: u64 = 8;
/// The most ticks a guest stamps past its estimate of the host tick: a round
/// trip of about half a second at 60 Hz.
pub(crate) const MAX_RUN_AHEAD: u64 = 30;
/// The ticks early a guest's input aims to reach the host: one tick of slack
/// against the link's jitter.
pub(crate) const ARRIVAL_SLACK: i64 = 1;
/// The sent samples whose lead the clock remembers, to read a report against.
const REMEMBERED: usize = 512;

#[derive(Resource, Default)]
pub(crate) struct GuestInputClock {
    seen: Option<u64>,
    base_tick: u64,
    observed_at: Duration,
    /// The last estimate of the host tick, without the lead.
    last_tick: u64,
    /// The last tick stamped: stamps never go back.
    last_stamp: u64,
    sequence: u64,
    /// The ticks past the estimate this guest stamps.
    lead: u64,
    /// Each recent sample's sequence and the lead it was stamped with.
    leads: VecDeque<(u64, u64)>,
}

impl GuestInputClock {
    /// The sample's sequence and the tick it is stamped for.
    pub(crate) fn sample(&mut self, host: u64, elapsed: Duration, step: Duration) -> (u64, u64) {
        if self.seen != Some(host) {
            self.seen = Some(host);
            self.base_tick = host.max(self.last_tick);
            self.observed_at = elapsed;
        }
        let advance = (elapsed.saturating_sub(self.observed_at).as_nanos() / step.as_nanos().max(1))
            .min(u128::from(MAX_INPUT_LEAD)) as u64;
        let tick = self.base_tick.saturating_add(advance).min(host.saturating_add(MAX_INPUT_LEAD));
        self.last_tick = tick;
        // A lead that shrinks does not stamp a tick already stamped: the
        // estimate catches up with the stamps instead.
        let stamp = tick.saturating_add(self.lead).max(self.last_stamp);
        self.last_stamp = stamp;
        self.sequence = self.sequence.checked_add(1).expect("guest input sequence exhausted");
        if self.leads.len() == REMEMBERED {
            self.leads.pop_front();
        }
        self.leads.push_back((self.sequence, self.lead));
        (self.sequence, stamp)
    }

    /// The host's report: the sample `sequence` arrived `margin` ticks before
    /// the tick it named ran (negative: after). Read against the lead that
    /// sample was stamped with, so a report that crossed a change of lead
    /// does not move it twice.
    pub(crate) fn arrived(&mut self, sequence: u64, margin: i64) {
        while self.leads.front().is_some_and(|(sent, _)| *sent < sequence) {
            self.leads.pop_front();
        }
        let Some(&(sent, used)) = self.leads.front() else { return };
        if sent != sequence {
            return;
        }
        let margin_now = margin + self.lead as i64 - used as i64;
        self.lead = (self.lead as i64 + ARRIVAL_SLACK - margin_now).clamp(0, MAX_RUN_AHEAD as i64) as u64;
    }

    /// The ticks past its estimate this guest stamps.
    pub(crate) fn lead(&self) -> u64 {
        self.lead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use ambition_characters::control::{PlayerSlot, SeatRawFrames};
    use crate::{GuestInput, HostTick, LocalSeat, send_local_input};

    #[test]
    fn the_sender_stamps_render_samples_by_time_and_keeps_their_order() {
        for durations in [vec![1.0 / 30.0; 24], vec![1.0 / 144.0; 120], vec![0.004, 0.012, 0.035, 0.002, 0.02].repeat(10)] {
            let fast = durations.iter().all(|dt| *dt < 1.0 / 60.0);
            let mut app = App::new();
            app.add_message::<GuestInput>().init_resource::<GuestInputClock>()
                .init_resource::<SeatRawFrames>().init_resource::<Time<Real>>()
                .insert_resource(Time::<Fixed>::from_hz(60.0))
                .insert_resource(LocalSeat(PlayerSlot(1)))
                .add_systems(Update, send_local_input);
            let clock = app.world_mut().spawn(HostTick(100)).id();
            let mut elapsed = Duration::ZERO;
            let mut last = None;
            let mut shared_ticks = false;
            for (index, seconds) in durations.into_iter().enumerate() {
                let dt = Duration::from_secs_f64(seconds);
                elapsed += dt;
                app.world_mut().resource_mut::<Time<Real>>().advance_by(dt);
                let host = 100 + (elapsed.as_secs_f64() * 60.0).floor() as u64;
                app.world_mut().get_mut::<HostTick>(clock).unwrap().0 = host;
                app.update();
                let sample = app.world_mut().resource_mut::<Messages<GuestInput>>().drain().next().unwrap();
                assert_eq!(sample.sequence, index as u64 + 1);
                assert_eq!(sample.tick, host, "render frames must not add simulation ticks");
                if let Some(previous) = last {
                    assert!(sample.tick >= previous);
                    shared_ticks |= sample.tick == previous;
                }
                last = Some(sample.tick);
            }
            if fast { assert!(shared_ticks, "fast renders share simulation ticks"); }
        }
    }

    /// A guest `delay` ticks from the host each way, stamping once a host
    /// tick, with the host reporting each sample's margin as it arrives: the
    /// lead settles where each input arrives `ARRIVAL_SLACK` ticks early,
    /// and the stamps never go back.
    #[test]
    fn the_lead_settles_where_inputs_arrive_with_the_slack() {
        let step = Duration::from_secs_f64(1.0 / 60.0);
        for delay in [0_u64, 1, 3, 6, 12] {
            let mut clock = GuestInputClock::default();
            // Samples on their way to the host, and reports on their way back.
            let mut up: VecDeque<(u64, u64, u64)> = VecDeque::new();
            let mut down: VecDeque<(u64, u64, i64)> = VecDeque::new();
            let mut last_stamp = 0;
            let mut margins = Vec::new();
            for now in 100_u64..400 {
                // The guest sees the host tick `delay` old.
                let seen = now - delay;
                while down.front().is_some_and(|(due, _, _)| *due <= now) {
                    let (_, sequence, margin) = down.pop_front().unwrap();
                    clock.arrived(sequence, margin);
                }
                let (sequence, stamp) = clock.sample(seen, Duration::from_secs_f64(now as f64 / 60.0), step);
                assert!(stamp >= last_stamp, "a stamp went back with delay {delay}");
                last_stamp = stamp;
                up.push_back((now + delay, sequence, stamp));
                while up.front().is_some_and(|(due, _, _)| *due <= now) {
                    let (_, sequence, stamp) = up.pop_front().unwrap();
                    // The host's next tick is `now + 1`.
                    let margin = stamp as i64 - (now as i64 + 1);
                    margins.push(margin);
                    down.push_back((now + delay, sequence, margin));
                }
            }
            let settled = &margins[margins.len() - 60..];
            assert!(
                settled.iter().all(|margin| *margin == ARRIVAL_SLACK),
                "with {delay} ticks each way the inputs did not settle {ARRIVAL_SLACK} tick early: {settled:?} (lead {})",
                clock.lead()
            );
        }
    }
}
