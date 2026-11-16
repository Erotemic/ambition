//! Input sample order is independent of the estimated host tick. Render
//! frames in one tick share that tick and keep their own sequence numbers.

use std::time::Duration;
use bevy::prelude::Resource;

pub(crate) const MAX_INPUT_LEAD: u64 = 8;

#[derive(Resource, Default)]
pub(crate) struct GuestInputClock {
    seen: Option<u64>,
    base_tick: u64,
    observed_at: Duration,
    last_tick: u64,
    sequence: u64,
}

impl GuestInputClock {
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
        self.sequence = self.sequence.checked_add(1).expect("guest input sequence exhausted");
        (self.sequence, tick)
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
}
