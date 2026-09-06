//! Analytic Minkowski light signals, local emitters/receivers, and arrival events.

use std::collections::VecDeque;

use ambition_platformer2d_core::snapshot::{
    put_bool, put_str, put_u32, put_u64, put_u8, put_vec2, Reader, SnapshotState,
};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::{
    CombatSet, Platformer2dSimulationPhaseMonolith, WorldPrepSet,
};
use ambition_relativity::{
    coordinate_frequency_from_emitter, observed_frequency_from_coordinate, InvariantSpeed,
};
use ambition_time::{ProperTimeScale, WorldTime};
use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::{
    App, Component, Entity, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Query, Res,
    ResMut, Resource, Update, Vec2, With,
};

use crate::{ActiveSpacetime2d, ProperTimeElapsed, Relativity2dSet, RelativityState2d};

const DEFAULT_HISTORY_CAPACITY: usize = 48;
const MIN_SIGNAL_DIRECTION_LENGTH_SQUARED: f32 = 1.0e-12;

/// Coordinate time owned by one active spacetime session.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SpacetimeCoordinateTime2d {
    pub seconds: f64,
    pub epoch: u64,
}

impl SpacetimeCoordinateTime2d {
    pub fn reset(&mut self) {
        self.seconds = 0.0;
        self.epoch = self.epoch.wrapping_add(1);
    }
}

impl SnapshotState for SpacetimeCoordinateTime2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.seconds.to_bits());
        put_u64(out, self.epoch);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            seconds: f64::from_bits(reader.u64()?),
            epoch: reader.u64()?,
        })
    }
}

/// A proper-time countdown carried by an emitting body.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ProperTimeCooldown2d {
    pub remaining_seconds: f64,
}

impl ProperTimeCooldown2d {
    pub fn ready(self) -> bool {
        self.remaining_seconds <= 0.0
    }

    pub fn reset(&mut self) {
        self.remaining_seconds = 0.0;
    }
}

impl SnapshotState for ProperTimeCooldown2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.remaining_seconds.to_bits());
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            remaining_seconds: f64::from_bits(reader.u64()?),
        })
    }
}

/// An opt-in local emitter. Its cooldown advances in the body's proper time.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(ProperTimeCooldown2d)]
pub struct LightEmitter2d {
    pub label: String,
    /// Stable game-owned identity copied into emitted packets and arrivals.
    pub emitter_tag: u64,
    pub pool_label: String,
    pub emitted_proper_frequency: f64,
    pub cooldown_proper_seconds: f64,
    pub source_receiver_channel: Option<u8>,
    pub next_packet_id: u64,
}

impl LightEmitter2d {
    pub fn new(
        label: impl Into<String>,
        pool_label: impl Into<String>,
        emitted_proper_frequency: f64,
        cooldown_proper_seconds: f64,
    ) -> Self {
        assert!(
            emitted_proper_frequency.is_finite() && emitted_proper_frequency > 0.0,
            "light emitter frequency must be finite and positive"
        );
        assert!(
            cooldown_proper_seconds.is_finite() && cooldown_proper_seconds >= 0.0,
            "light emitter cooldown must be finite and non-negative"
        );
        Self {
            label: label.into(),
            emitter_tag: 0,
            pool_label: pool_label.into(),
            emitted_proper_frequency,
            cooldown_proper_seconds,
            source_receiver_channel: None,
            next_packet_id: 1,
        }
    }

    pub fn with_tag(mut self, tag: u64) -> Self {
        self.emitter_tag = tag;
        self
    }

    pub fn with_source_receiver_channel(mut self, channel: u8) -> Self {
        assert!(
            channel < 64,
            "light receiver channels must fit the hit mask"
        );
        self.source_receiver_channel = Some(channel);
        self
    }
}

impl SnapshotState for LightEmitter2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.label);
        put_u64(out, self.emitter_tag);
        put_str(out, &self.pool_label);
        put_u64(out, self.emitted_proper_frequency.to_bits());
        put_u64(out, self.cooldown_proper_seconds.to_bits());
        match self.source_receiver_channel {
            Some(channel) => {
                put_bool(out, true);
                put_u8(out, channel);
            }
            None => put_bool(out, false),
        }
        put_u64(out, self.next_packet_id);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        let emitter = Self {
            label: reader.str()?.to_owned(),
            emitter_tag: reader.u64()?,
            pool_label: reader.str()?.to_owned(),
            emitted_proper_frequency: f64::from_bits(reader.u64()?),
            cooldown_proper_seconds: f64::from_bits(reader.u64()?),
            source_receiver_channel: if reader.bool()? {
                Some(reader.u8()?)
            } else {
                None
            },
            next_packet_id: reader.u64()?,
        };
        (emitter.emitted_proper_frequency.is_finite()
            && emitter.emitted_proper_frequency > 0.0
            && emitter.cooldown_proper_seconds.is_finite()
            && emitter.cooldown_proper_seconds >= 0.0
            && emitter
                .source_receiver_channel
                .is_none_or(|channel| channel < 64))
        .then_some(emitter)
    }
}

/// One deterministic, preallocated light-signal slot.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LightSignalPoolSlot2d {
    pub pool_label: String,
    pub slot_index: u16,
}

impl LightSignalPoolSlot2d {
    pub fn new(pool_label: impl Into<String>, slot_index: u16) -> Self {
        Self {
            pool_label: pool_label.into(),
            slot_index,
        }
    }
}

impl SnapshotState for LightSignalPoolSlot2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.pool_label);
        put_u32(out, u32::from(self.slot_index));
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            pool_label: reader.str()?.to_owned(),
            slot_index: u16::try_from(reader.u32()?).ok()?,
        })
    }
}

/// Canonical state of one analytic null signal.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LightSignal2d {
    pub active: bool,
    pub packet_id: u64,
    pub emitter_tag: u64,
    pub payload: u64,
    pub emission_time: f64,
    pub last_coordinate_time: f64,
    pub emission_position: Vec2,
    pub position: Vec2,
    pub direction: Vec2,
    pub coordinate_frequency: f64,
    pub emitter_velocity: Vec2,
    pub was_reflected: bool,
    pub source_receiver_channel: Option<u8>,
    /// Optional destination channel. Other receivers are transparent to this packet.
    pub target_receiver_channel: Option<u8>,
    pub hit_channels: u64,
    pub maximum_coordinate_age: f64,
}

impl Default for LightSignal2d {
    fn default() -> Self {
        Self::inactive()
    }
}

impl LightSignal2d {
    pub const fn inactive() -> Self {
        Self {
            active: false,
            packet_id: 0,
            emitter_tag: 0,
            payload: 0,
            emission_time: 0.0,
            last_coordinate_time: 0.0,
            emission_position: Vec2::ZERO,
            position: Vec2::ZERO,
            direction: Vec2::X,
            coordinate_frequency: 1.0,
            emitter_velocity: Vec2::ZERO,
            was_reflected: false,
            source_receiver_channel: None,
            target_receiver_channel: None,
            hit_channels: 0,
            maximum_coordinate_age: 8.0,
        }
    }

    fn activate(
        &mut self,
        packet_id: u64,
        emitter_tag: u64,
        payload: u64,
        emission_time: f64,
        emission_position: Vec2,
        direction: Vec2,
        coordinate_frequency: f64,
        emitter_velocity: Vec2,
        was_reflected: bool,
        source_receiver_channel: Option<u8>,
        target_receiver_channel: Option<u8>,
        maximum_coordinate_age: f64,
    ) {
        let source_hit = source_receiver_channel
            .filter(|channel| *channel < 64)
            .map_or(0, |channel| 1_u64 << channel);
        *self = Self {
            active: true,
            packet_id,
            emitter_tag,
            payload,
            emission_time,
            last_coordinate_time: emission_time,
            emission_position,
            position: emission_position,
            direction,
            coordinate_frequency,
            emitter_velocity,
            was_reflected,
            source_receiver_channel,
            target_receiver_channel,
            hit_channels: source_hit,
            maximum_coordinate_age,
        };
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn position_at(self, coordinate_time: f64, invariant_speed: InvariantSpeed) -> Vec2 {
        let age = (coordinate_time - self.emission_time).max(0.0) as f32;
        self.emission_position + self.direction * invariant_speed.get() as f32 * age
    }
}

impl SnapshotState for LightSignal2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.active);
        put_u64(out, self.packet_id);
        put_u64(out, self.emitter_tag);
        put_u64(out, self.payload);
        put_u64(out, self.emission_time.to_bits());
        put_u64(out, self.last_coordinate_time.to_bits());
        put_vec2(out, self.emission_position);
        put_vec2(out, self.position);
        put_vec2(out, self.direction);
        put_u64(out, self.coordinate_frequency.to_bits());
        put_vec2(out, self.emitter_velocity);
        put_bool(out, self.was_reflected);
        match self.source_receiver_channel {
            Some(channel) => {
                put_bool(out, true);
                put_u8(out, channel);
            }
            None => put_bool(out, false),
        }
        match self.target_receiver_channel {
            Some(channel) => {
                put_bool(out, true);
                put_u8(out, channel);
            }
            None => put_bool(out, false),
        }
        put_u64(out, self.hit_channels);
        put_u64(out, self.maximum_coordinate_age.to_bits());
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            active: reader.bool()?,
            packet_id: reader.u64()?,
            emitter_tag: reader.u64()?,
            payload: reader.u64()?,
            emission_time: f64::from_bits(reader.u64()?),
            last_coordinate_time: f64::from_bits(reader.u64()?),
            emission_position: reader.vec2()?,
            position: reader.vec2()?,
            direction: reader.vec2()?,
            coordinate_frequency: f64::from_bits(reader.u64()?),
            emitter_velocity: reader.vec2()?,
            was_reflected: reader.bool()?,
            source_receiver_channel: if reader.bool()? {
                Some(reader.u8()?)
            } else {
                None
            },
            target_receiver_channel: if reader.bool()? {
                Some(reader.u8()?)
            } else {
                None
            },
            hit_channels: reader.u64()?,
            maximum_coordinate_age: f64::from_bits(reader.u64()?),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightReceiverMode2d {
    Observe,
    Reflect,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct LightReceiver2d {
    pub label: String,
    pub channel: u8,
    pub half_extents: Vec2,
    pub accepted_frequency: Option<(f64, f64)>,
    pub mode: LightReceiverMode2d,
    pub consume_on_hit: bool,
}

impl LightReceiver2d {
    pub fn observer(label: impl Into<String>, channel: u8, half_extents: Vec2) -> Self {
        assert!(
            channel < 64,
            "light receiver channels must fit the hit mask"
        );
        assert!(
            half_extents.is_finite() && half_extents.min_element() >= 0.0,
            "light receiver half-extents must be finite and non-negative"
        );
        Self {
            label: label.into(),
            channel,
            half_extents,
            accepted_frequency: None,
            mode: LightReceiverMode2d::Observe,
            consume_on_hit: false,
        }
    }

    pub fn reflector(label: impl Into<String>, channel: u8, half_extents: Vec2) -> Self {
        let mut receiver = Self::observer(label, channel, half_extents);
        receiver.mode = LightReceiverMode2d::Reflect;
        receiver.consume_on_hit = true;
        receiver
    }

    pub fn with_passband(mut self, minimum: f64, maximum: f64) -> Self {
        assert!(
            minimum.is_finite() && maximum.is_finite() && minimum > 0.0 && maximum > 0.0,
            "light receiver passbands must be finite and positive"
        );
        self.accepted_frequency = Some((minimum.min(maximum), minimum.max(maximum)));
        self
    }

    pub fn consuming(mut self) -> Self {
        self.consume_on_hit = true;
        self
    }

    pub fn accepts(&self, observed_frequency: f64) -> bool {
        self.accepted_frequency.is_none_or(|(minimum, maximum)| {
            observed_frequency >= minimum && observed_frequency <= maximum
        })
    }

    fn is_valid(&self) -> bool {
        self.channel < 64
            && self.half_extents.is_finite()
            && self.half_extents.min_element() >= 0.0
            && self.accepted_frequency.is_none_or(|(minimum, maximum)| {
                minimum.is_finite() && maximum.is_finite() && minimum > 0.0 && maximum >= minimum
            })
    }
}

impl SnapshotState for LightReceiver2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.label);
        put_u8(out, self.channel);
        put_vec2(out, self.half_extents);
        match self.accepted_frequency {
            Some((minimum, maximum)) => {
                put_bool(out, true);
                put_u64(out, minimum.to_bits());
                put_u64(out, maximum.to_bits());
            }
            None => put_bool(out, false),
        }
        put_u8(
            out,
            match self.mode {
                LightReceiverMode2d::Observe => 0,
                LightReceiverMode2d::Reflect => 1,
            },
        );
        put_bool(out, self.consume_on_hit);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        let receiver = Self {
            label: reader.str()?.to_owned(),
            channel: reader.u8()?,
            half_extents: reader.vec2()?,
            accepted_frequency: if reader.bool()? {
                Some((f64::from_bits(reader.u64()?), f64::from_bits(reader.u64()?)))
            } else {
                None
            },
            mode: match reader.u8()? {
                0 => LightReceiverMode2d::Observe,
                1 => LightReceiverMode2d::Reflect,
                _ => return None,
            },
            consume_on_hit: reader.bool()?,
        };
        receiver.is_valid().then_some(receiver)
    }
}

/// Request one local emitter to activate an analytic null signal.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct LightEmissionRequest2d {
    pub emitter: Entity,
    pub direction: Vec2,
    /// Opaque game-owned payload transported with the light packet.
    pub payload: u64,
    /// Optional destination receiver channel. Other receivers remain transparent.
    pub target_receiver_channel: Option<u8>,
}

impl LightEmissionRequest2d {
    pub fn new(emitter: Entity, direction: Vec2) -> Self {
        Self {
            emitter,
            direction,
            payload: 0,
            target_receiver_channel: None,
        }
    }

    pub fn with_payload(mut self, payload: u64) -> Self {
        self.payload = payload;
        self
    }

    pub fn to_receiver(mut self, channel: u8) -> Self {
        assert!(
            channel < 64,
            "light receiver channels must fit the hit mask"
        );
        self.target_receiver_channel = Some(channel);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalArrivalRecord2d {
    pub packet_id: u64,
    pub emitter_tag: u64,
    pub payload: u64,
    pub receiver_label: String,
    pub receiver_channel: u8,
    pub coordinate_time: f64,
    /// Coordinate time at which this packet left its current emitter.
    pub signal_emission_time: f64,
    pub receiver_proper_time: Option<f64>,
    pub observed_frequency: f64,
    pub accepted: bool,
    pub reflected: bool,
    pub signal_was_reflected: bool,
    pub position: Vec2,
}

impl SignalArrivalRecord2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.packet_id);
        put_u64(out, self.emitter_tag);
        put_u64(out, self.payload);
        put_str(out, &self.receiver_label);
        put_u8(out, self.receiver_channel);
        put_u64(out, self.coordinate_time.to_bits());
        put_u64(out, self.signal_emission_time.to_bits());
        match self.receiver_proper_time {
            Some(time) => {
                put_bool(out, true);
                put_u64(out, time.to_bits());
            }
            None => put_bool(out, false),
        }
        put_u64(out, self.observed_frequency.to_bits());
        put_bool(out, self.accepted);
        put_bool(out, self.reflected);
        put_bool(out, self.signal_was_reflected);
        put_vec2(out, self.position);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            packet_id: reader.u64()?,
            emitter_tag: reader.u64()?,
            payload: reader.u64()?,
            receiver_label: reader.str()?.to_owned(),
            receiver_channel: reader.u8()?,
            coordinate_time: f64::from_bits(reader.u64()?),
            signal_emission_time: f64::from_bits(reader.u64()?),
            receiver_proper_time: if reader.bool()? {
                Some(f64::from_bits(reader.u64()?))
            } else {
                None
            },
            observed_frequency: f64::from_bits(reader.u64()?),
            accepted: reader.bool()?,
            reflected: reader.bool()?,
            signal_was_reflected: reader.bool()?,
            position: reader.vec2()?,
        })
    }
}

#[derive(Message, Clone, Debug, PartialEq)]
pub struct SignalArrival2d(pub SignalArrivalRecord2d);

/// Bounded authoritative event history owned by the active spacetime session.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SignalArrivalHistory2d {
    pub capacity: usize,
    pub arrivals: VecDeque<SignalArrivalRecord2d>,
}

impl Default for SignalArrivalHistory2d {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_HISTORY_CAPACITY,
            arrivals: VecDeque::with_capacity(DEFAULT_HISTORY_CAPACITY),
        }
    }
}

impl SignalArrivalHistory2d {
    pub fn clear(&mut self) {
        self.arrivals.clear();
    }

    fn push(&mut self, record: SignalArrivalRecord2d) {
        let capacity = self.capacity.max(1);
        while self.arrivals.len() >= capacity {
            self.arrivals.pop_front();
        }
        self.arrivals.push_back(record);
    }
}

impl SnapshotState for SignalArrivalHistory2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.capacity as u32);
        put_u32(out, self.arrivals.len() as u32);
        for arrival in &self.arrivals {
            arrival.encode(out);
        }
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        let capacity = reader.u32()? as usize;
        let count = reader.u32()? as usize;
        let mut arrivals = VecDeque::with_capacity(capacity.max(count));
        for _ in 0..count {
            arrivals.push_back(SignalArrivalRecord2d::decode(reader)?);
        }
        Some(Self { capacity, arrivals })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightSignalObservation2d {
    pub packet_id: u64,
    pub emitter_tag: u64,
    pub payload: u64,
    pub emission_time: f64,
    pub emission_position: Vec2,
    pub position: Vec2,
    pub direction: Vec2,
    pub target_receiver_channel: Option<u8>,
    pub coordinate_frequency: f64,
    pub age: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightReceiverObservation2d {
    pub label: String,
    pub channel: u8,
    pub position: Vec2,
    pub half_extents: Vec2,
    pub accepted_frequency: Option<(f64, f64)>,
    pub mode: LightReceiverMode2d,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightEmitterObservation2d {
    pub label: String,
    pub cooldown_remaining: f64,
    pub cooldown_duration: f64,
    pub emitted_proper_frequency: f64,
}

/// Presentation-facing signal facts. Rebuilt from canonical components each tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativitySignalView2d {
    pub coordinate_time: f64,
    pub invariant_speed: f64,
    pub active_signals: Vec<LightSignalObservation2d>,
    pub receivers: Vec<LightReceiverObservation2d>,
    pub emitters: Vec<LightEmitterObservation2d>,
    pub recent_arrivals: Vec<SignalArrivalRecord2d>,
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.clear_message_on_rollback::<LightEmissionRequest2d>(
        "ambition_relativity2d",
        "message.light_emission_request_2d",
    )
    .clear_message_on_rollback::<SignalArrival2d>(
        "ambition_relativity2d",
        "message.signal_arrival_2d",
    )
    .declare_rollback_derived_resource::<RelativitySignalView2d>(
        "ambition_relativity2d",
        "relativity.signal_view_2d",
        "presentation read model rebuilt from signal slots, receivers, emitters, and arrival history",
    )
    .rollback_component_canonical::<SpacetimeCoordinateTime2d>(
        "ambition_relativity2d",
        "relativity.coordinate_time_2d",
    )
    .rollback_component_canonical::<ProperTimeCooldown2d>(
        "ambition_relativity2d",
        "relativity.proper_time_cooldown_2d",
    )
    .rollback_component_canonical::<LightEmitter2d>(
        "ambition_relativity2d",
        "relativity.light_emitter_2d",
    )
    .rollback_component_canonical::<LightSignalPoolSlot2d>(
        "ambition_relativity2d",
        "relativity.light_signal_pool_slot_2d",
    )
    .rollback_component_canonical::<LightSignal2d>(
        "ambition_relativity2d",
        "relativity.light_signal_2d",
    )
    .rollback_component_canonical::<LightReceiver2d>(
        "ambition_relativity2d",
        "relativity.light_receiver_2d",
    )
    .rollback_component_canonical::<SignalArrivalHistory2d>(
        "ambition_relativity2d",
        "relativity.signal_arrival_history_2d",
    );
}

pub(crate) fn install_signal_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.add_message::<LightEmissionRequest2d>();
    app.add_message::<SignalArrival2d>();
    app.init_resource::<RelativitySignalView2d>();
    app.add_systems(
        sim,
        advance_coordinate_time
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::AdvanceCoordinateTime)
            .in_set(WorldPrepSet::BeforeIntegrate),
    )
    .add_systems(
        sim,
        advance_proper_time_cooldowns
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::AdvanceProperCooldowns)
            .in_set(WorldPrepSet::BeforeIntegrate)
            .after(Relativity2dSet::ResolveClocks),
    )
    .add_systems(
        sim,
        (process_light_emission_requests, propagate_light_signals)
            .chain()
            .run_if(crate::spacetime_is_active)
            .in_set(CombatSet::Materialize)
            .in_set(Platformer2dSimulationPhaseMonolith::Combat),
    )
    .add_systems(
        sim,
        publish_signal_view
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::PublishView)
            .in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
    )
    .add_systems(Update, clear_signal_view_without_live_spacetime);
}

fn advance_coordinate_time(
    time: Res<WorldTime>,
    mut coordinate_time: Query<
        &mut SpacetimeCoordinateTime2d,
        (With<SessionRoot>, With<ActiveSpacetime2d>),
    >,
) {
    let Ok(mut coordinate_time) = coordinate_time.single_mut() else {
        return;
    };
    coordinate_time.seconds += f64::from(time.sim_dt());
}

fn advance_proper_time_cooldowns(
    time: Res<WorldTime>,
    mut emitters: Query<
        (&mut ProperTimeCooldown2d, Option<&ProperTimeScale>),
        With<LightEmitter2d>,
    >,
) {
    for (mut cooldown, scale) in &mut emitters {
        let rate = f64::from(ProperTimeScale::or_default(scale).value());
        cooldown.remaining_seconds =
            (cooldown.remaining_seconds - f64::from(time.sim_dt()) * rate).max(0.0);
    }
}

fn process_light_emission_requests(
    mut requests: MessageReader<LightEmissionRequest2d>,
    spacetime: Query<(&ActiveSpacetime2d, &SpacetimeCoordinateTime2d), With<SessionRoot>>,
    mut emitters: Query<(
        &BodyKinematics,
        &mut LightEmitter2d,
        &mut ProperTimeCooldown2d,
    )>,
    mut signals: Query<(Entity, &LightSignalPoolSlot2d, &mut LightSignal2d)>,
) {
    let Ok((spacetime, coordinate_time)) = spacetime.single() else {
        return;
    };
    let invariant_speed = spacetime.model().invariant_speed();

    for request in requests.read() {
        let Ok((body, mut emitter, mut cooldown)) = emitters.get_mut(request.emitter) else {
            continue;
        };
        if !cooldown.ready()
            || request.direction.length_squared() <= MIN_SIGNAL_DIRECTION_LENGTH_SQUARED
        {
            continue;
        }
        let direction = request.direction.normalize();
        let Some(coordinate_frequency) = coordinate_frequency_from_emitter(
            emitter.emitted_proper_frequency,
            vec2_to_3(direction),
            vec2_to_3(body.vel),
            invariant_speed,
        ) else {
            continue;
        };

        let mut selected: Option<(u16, Entity)> = None;
        for (entity, slot, signal) in &mut signals {
            if slot.pool_label != emitter.pool_label || signal.active {
                continue;
            }
            if selected.is_none_or(|(index, _)| slot.slot_index < index) {
                selected = Some((slot.slot_index, entity));
            }
        }
        let Some((_, signal_entity)) = selected else {
            continue;
        };
        let Ok((_, _, mut signal)) = signals.get_mut(signal_entity) else {
            continue;
        };

        let packet_id = emitter.next_packet_id.max(1);
        emitter.next_packet_id = packet_id.wrapping_add(1).max(1);
        signal.activate(
            packet_id,
            emitter.emitter_tag,
            request.payload,
            coordinate_time.seconds,
            body.pos,
            direction,
            coordinate_frequency,
            body.vel,
            false,
            emitter.source_receiver_channel,
            request.target_receiver_channel,
            8.0,
        );
        cooldown.remaining_seconds = emitter.cooldown_proper_seconds.max(0.0);
    }
}

#[derive(Clone)]
struct ReceiverSample {
    label: String,
    channel: u8,
    half_extents: Vec2,
    accepted_frequency: Option<(f64, f64)>,
    mode: LightReceiverMode2d,
    consume_on_hit: bool,
    body: BodyKinematics,
    proper_time: Option<f64>,
    proper_time_rate: f64,
}

impl ReceiverSample {
    fn accepts(&self, frequency: f64) -> bool {
        self.accepted_frequency
            .is_none_or(|(minimum, maximum)| frequency >= minimum && frequency <= maximum)
    }
}

#[derive(Clone)]
struct ReflectedSignal {
    packet_id: u64,
    emitter_tag: u64,
    payload: u64,
    emission_time: f64,
    emission_position: Vec2,
    direction: Vec2,
    coordinate_frequency: f64,
    emitter_velocity: Vec2,
    source_receiver_channel: Option<u8>,
    target_receiver_channel: Option<u8>,
    maximum_coordinate_age: f64,
    pool_label: String,
}

fn propagate_light_signals(
    mut arrivals: MessageWriter<SignalArrival2d>,
    mut spacetime: Query<
        (
            &ActiveSpacetime2d,
            &SpacetimeCoordinateTime2d,
            &mut SignalArrivalHistory2d,
        ),
        With<SessionRoot>,
    >,
    receivers: Query<(
        &LightReceiver2d,
        &BodyKinematics,
        Option<&ProperTimeElapsed>,
        Option<&RelativityState2d>,
    )>,
    mut signals: Query<(Entity, &LightSignalPoolSlot2d, &mut LightSignal2d)>,
) {
    let Ok((spacetime, coordinate_time, mut history)) = spacetime.single_mut() else {
        return;
    };
    let invariant_speed = spacetime.model().invariant_speed();
    let mut receiver_samples: Vec<_> = receivers
        .iter()
        .map(|(receiver, body, proper_time, relativity)| ReceiverSample {
            label: receiver.label.clone(),
            channel: receiver.channel,
            half_extents: receiver.half_extents,
            accepted_frequency: receiver.accepted_frequency,
            mode: receiver.mode,
            consume_on_hit: receiver.consume_on_hit,
            body: *body,
            proper_time: proper_time.map(|clock| clock.seconds),
            proper_time_rate: relativity.map_or(1.0, |state| f64::from(state.proper_time_rate)),
        })
        .collect();
    receiver_samples.sort_by(|left, right| {
        left.channel
            .cmp(&right.channel)
            .then_with(|| left.label.cmp(&right.label))
    });

    let mut reflections = Vec::new();
    for (_, slot, mut signal) in &mut signals {
        if !signal.active {
            continue;
        }
        let start_time = signal.last_coordinate_time.max(signal.emission_time);
        let end_time = coordinate_time.seconds;
        if end_time <= start_time {
            signal.position = signal.position_at(end_time, invariant_speed);
            continue;
        }
        let start = signal.position_at(start_time, invariant_speed);
        let end = signal.position_at(end_time, invariant_speed);
        let interval = end_time - start_time;
        let mut hits = Vec::new();

        for (receiver_index, receiver) in receiver_samples.iter().enumerate() {
            if receiver.channel >= 64 {
                continue;
            }
            if signal
                .target_receiver_channel
                .is_some_and(|target| target != receiver.channel)
            {
                continue;
            }
            let channel_bit = 1_u64 << receiver.channel;
            if signal.hit_channels & channel_bit != 0 {
                continue;
            }
            let receiver_end = receiver.body.pos;
            let receiver_start = receiver_end - receiver.body.vel * interval as f32;
            let relative_start = start - receiver_start;
            let relative_end = end - receiver_end;
            if let Some(fraction) =
                segment_aabb_fraction(relative_start, relative_end, receiver.half_extents)
            {
                hits.push((fraction, receiver_index));
            }
        }
        hits.sort_by(
            |(left_fraction, left_index), (right_fraction, right_index)| {
                left_fraction
                    .total_cmp(right_fraction)
                    .then_with(|| {
                        receiver_samples[*left_index]
                            .channel
                            .cmp(&receiver_samples[*right_index].channel)
                    })
                    .then_with(|| {
                        receiver_samples[*left_index]
                            .label
                            .cmp(&receiver_samples[*right_index].label)
                    })
            },
        );

        let mut deactivated = false;
        for (fraction, receiver_index) in hits {
            let receiver = &receiver_samples[receiver_index];
            let channel_bit = 1_u64 << receiver.channel;
            if signal.hit_channels & channel_bit != 0 {
                continue;
            }
            signal.hit_channels |= channel_bit;

            let arrival_time = start_time + interval * f64::from(fraction);
            let arrival_position = start.lerp(end, fraction);
            let Some(observed_frequency) = observed_frequency_from_coordinate(
                signal.coordinate_frequency,
                vec2_to_3(signal.direction),
                vec2_to_3(receiver.body.vel),
                invariant_speed,
            ) else {
                continue;
            };
            let accepted = receiver.accepts(observed_frequency);
            let receiver_proper_time = receiver.proper_time.map(|proper_time| {
                proper_time
                    - (end_time - arrival_time).max(0.0) * receiver.proper_time_rate.max(0.0)
            });
            let reflected = receiver.mode == LightReceiverMode2d::Reflect;
            let record = SignalArrivalRecord2d {
                packet_id: signal.packet_id,
                emitter_tag: signal.emitter_tag,
                payload: signal.payload,
                receiver_label: receiver.label.clone(),
                receiver_channel: receiver.channel,
                coordinate_time: arrival_time,
                signal_emission_time: signal.emission_time,
                receiver_proper_time,
                observed_frequency,
                accepted,
                reflected,
                signal_was_reflected: signal.was_reflected,
                position: arrival_position,
            };
            history.push(record.clone());
            arrivals.write(SignalArrival2d(record));

            if reflected {
                let reflected_direction = -signal.direction;
                if let Some(reflected_coordinate_frequency) = coordinate_frequency_from_emitter(
                    observed_frequency,
                    vec2_to_3(reflected_direction),
                    vec2_to_3(receiver.body.vel),
                    invariant_speed,
                ) {
                    reflections.push(ReflectedSignal {
                        packet_id: signal.packet_id,
                        emitter_tag: signal.emitter_tag,
                        payload: signal.payload,
                        emission_time: arrival_time,
                        emission_position: arrival_position,
                        direction: reflected_direction,
                        coordinate_frequency: reflected_coordinate_frequency,
                        emitter_velocity: receiver.body.vel,
                        source_receiver_channel: Some(receiver.channel),
                        target_receiver_channel: signal.source_receiver_channel,
                        maximum_coordinate_age: signal.maximum_coordinate_age,
                        pool_label: slot.pool_label.clone(),
                    });
                }
            }
            if reflected || receiver.consume_on_hit {
                signal.deactivate();
                deactivated = true;
                break;
            }
        }

        if !deactivated {
            signal.position = end;
            signal.last_coordinate_time = end_time;
            if end_time - signal.emission_time >= signal.maximum_coordinate_age {
                signal.deactivate();
            }
        }
    }

    for reflected in reflections {
        let mut selected: Option<(u16, Entity)> = None;
        for (entity, slot, signal) in &mut signals {
            if signal.active || slot.pool_label != reflected.pool_label {
                continue;
            }
            if selected.is_none_or(|(index, _)| slot.slot_index < index) {
                selected = Some((slot.slot_index, entity));
            }
        }
        let Some((_, entity)) = selected else {
            continue;
        };
        let Ok((_, _, mut signal)) = signals.get_mut(entity) else {
            continue;
        };
        signal.activate(
            reflected.packet_id,
            reflected.emitter_tag,
            reflected.payload,
            reflected.emission_time,
            reflected.emission_position,
            reflected.direction,
            reflected.coordinate_frequency,
            reflected.emitter_velocity,
            true,
            reflected.source_receiver_channel,
            reflected.target_receiver_channel,
            reflected.maximum_coordinate_age,
        );
    }
}

fn publish_signal_view(
    spacetime: Query<
        (
            &ActiveSpacetime2d,
            &SpacetimeCoordinateTime2d,
            &SignalArrivalHistory2d,
        ),
        With<SessionRoot>,
    >,
    signals: Query<&LightSignal2d>,
    receivers: Query<(&LightReceiver2d, &BodyKinematics)>,
    emitters: Query<(&LightEmitter2d, &ProperTimeCooldown2d)>,
    mut view: ResMut<RelativitySignalView2d>,
) {
    let Ok((spacetime, coordinate_time, history)) = spacetime.single() else {
        *view = RelativitySignalView2d::default();
        return;
    };
    view.coordinate_time = coordinate_time.seconds;
    view.invariant_speed = spacetime.model().invariant_speed().get();
    view.active_signals.clear();
    view.receivers.clear();
    view.emitters.clear();
    view.recent_arrivals.clear();

    for signal in &signals {
        if signal.active {
            view.active_signals.push(LightSignalObservation2d {
                packet_id: signal.packet_id,
                emitter_tag: signal.emitter_tag,
                payload: signal.payload,
                emission_time: signal.emission_time,
                emission_position: signal.emission_position,
                position: signal.position,
                direction: signal.direction,
                target_receiver_channel: signal.target_receiver_channel,
                coordinate_frequency: signal.coordinate_frequency,
                age: (coordinate_time.seconds - signal.emission_time).max(0.0),
            });
        }
    }
    view.active_signals.sort_by_key(|signal| signal.packet_id);

    for (receiver, body) in &receivers {
        view.receivers.push(LightReceiverObservation2d {
            label: receiver.label.clone(),
            channel: receiver.channel,
            position: body.pos,
            half_extents: receiver.half_extents,
            accepted_frequency: receiver.accepted_frequency,
            mode: receiver.mode,
        });
    }
    view.receivers.sort_by_key(|receiver| receiver.channel);

    for (emitter, cooldown) in &emitters {
        view.emitters.push(LightEmitterObservation2d {
            label: emitter.label.clone(),
            cooldown_remaining: cooldown.remaining_seconds,
            cooldown_duration: emitter.cooldown_proper_seconds,
            emitted_proper_frequency: emitter.emitted_proper_frequency,
        });
    }
    view.emitters.sort_by(|lhs, rhs| lhs.label.cmp(&rhs.label));
    view.recent_arrivals
        .extend(history.arrivals.iter().cloned());
}

fn clear_signal_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut view: ResMut<RelativitySignalView2d>,
) {
    if spacetime.is_empty()
        && (view.coordinate_time != 0.0
            || !view.active_signals.is_empty()
            || !view.receivers.is_empty()
            || !view.emitters.is_empty()
            || !view.recent_arrivals.is_empty())
    {
        *view = RelativitySignalView2d::default();
    }
}

fn vec2_to_3(value: Vec2) -> [f64; 3] {
    [f64::from(value.x), f64::from(value.y), 0.0]
}

/// Fraction along a segment where it first enters a centered AABB.
fn segment_aabb_fraction(start: Vec2, end: Vec2, half_extents: Vec2) -> Option<f32> {
    let delta = end - start;
    let mut lower = 0.0_f32;
    let mut upper = 1.0_f32;
    for (origin, step, half) in [
        (start.x, delta.x, half_extents.x.abs()),
        (start.y, delta.y, half_extents.y.abs()),
    ] {
        if step.abs() <= f32::EPSILON {
            if origin < -half || origin > half {
                return None;
            }
            continue;
        }
        let inverse = 1.0 / step;
        let mut enter = (-half - origin) * inverse;
        let mut exit = (half - origin) * inverse;
        if enter > exit {
            core::mem::swap(&mut enter, &mut exit);
        }
        lower = lower.max(enter);
        upper = upper.min(exit);
        if lower > upper {
            return None;
        }
    }
    (upper >= 0.0 && lower <= 1.0).then_some(lower.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swept_segment_catches_a_thin_receiver() {
        let fraction = segment_aabb_fraction(
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(0.5, 1.0),
        )
        .unwrap();
        assert!((fraction - 0.475).abs() < 1.0e-6);
    }

    #[test]
    fn segment_misses_when_parallel_and_outside() {
        assert!(segment_aabb_fraction(
            Vec2::new(-10.0, 2.0),
            Vec2::new(10.0, 2.0),
            Vec2::new(0.5, 1.0),
        )
        .is_none());
    }

    #[test]
    fn authoritative_receiver_and_pool_configuration_round_trip() {
        let receiver = LightReceiver2d::reflector("test_reflector", 7, Vec2::new(3.0, 4.0))
            .with_passband(80.0, 120.0);
        let receiver_bytes = ambition_platformer2d_core::snapshot::encode_state(&receiver);
        assert_eq!(
            ambition_platformer2d_core::snapshot::decode_state::<LightReceiver2d>(&receiver_bytes,),
            Some(receiver),
        );

        let slot = LightSignalPoolSlot2d::new("test_pool", 12);
        let slot_bytes = ambition_platformer2d_core::snapshot::encode_state(&slot);
        assert_eq!(
            ambition_platformer2d_core::snapshot::decode_state::<LightSignalPoolSlot2d>(
                &slot_bytes,
            ),
            Some(slot),
        );
    }

    #[test]
    fn emitter_identity_and_game_payload_survive_snapshot_round_trip() {
        let emitter = LightEmitter2d::new("messenger", "pool", 100.0, 0.5)
            .with_tag(42)
            .with_source_receiver_channel(7);
        let bytes = ambition_platformer2d_core::snapshot::encode_state(&emitter);
        assert_eq!(
            ambition_platformer2d_core::snapshot::decode_state::<LightEmitter2d>(&bytes),
            Some(emitter),
        );

        let mut signal = LightSignal2d::inactive();
        signal.activate(
            9,
            42,
            0xfeed_beef,
            1.25,
            Vec2::new(3.0, 4.0),
            Vec2::new(0.6, 0.8),
            123.0,
            Vec2::new(5.0, -2.0),
            false,
            Some(7),
            Some(12),
            8.0,
        );
        let bytes = ambition_platformer2d_core::snapshot::encode_state(&signal);
        let decoded = ambition_platformer2d_core::snapshot::decode_state::<LightSignal2d>(&bytes)
            .expect("signal snapshot should decode");
        assert_eq!(decoded.emitter_tag, 42);
        assert_eq!(decoded.payload, 0xfeed_beef);
        assert_eq!(decoded, signal);
    }

    #[test]
    fn analytic_signal_position_uses_coordinate_time_and_is_null() {
        let c = InvariantSpeed::new(10.0).unwrap();
        let mut signal = LightSignal2d::inactive();
        signal.activate(
            1,
            0,
            0,
            2.0,
            Vec2::new(3.0, 4.0),
            Vec2::X,
            100.0,
            Vec2::ZERO,
            false,
            None,
            None,
            10.0,
        );
        let end = signal.position_at(5.0, c);
        assert_eq!(end, Vec2::new(33.0, 4.0));
        let interval = ambition_relativity::minkowski_interval(
            ambition_relativity::MinkowskiEvent {
                coordinate_time: 3.0,
                position: [f64::from(end.x - 3.0), f64::from(end.y - 4.0), 0.0],
            },
            c,
        )
        .unwrap();
        assert_eq!(interval.kind, ambition_relativity::IntervalKind::Null);
    }
}
