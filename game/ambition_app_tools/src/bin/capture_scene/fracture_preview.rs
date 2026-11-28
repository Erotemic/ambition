use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::LiveRooms;
use ambition_platformer2d::vfx::vfx::{BreakMotion, Fracture, VfxMessage, VfxWriter};
use bevy::prelude::*;

#[derive(Clone, Debug)]
pub(super) struct Preview {
    pub(super) tick: u64,
    pub(super) fracture: Fracture,
}

impl Preview {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let bad =
            || "--fracture needs TICK:LOOK:X,Y,W,H:SEED with positive width and height".to_owned();
        let [tick, look, bounds, seed] = value
            .split(':')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| bad())?;
        let [x, y, width, height] = bounds
            .split(',')
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| bad())?
            .try_into()
            .map_err(|_| bad())?;
        if ![x, y, width, height].into_iter().all(f32::is_finite)
            || width <= 0.0
            || height <= 0.0
            || look.is_empty()
        {
            return Err(bad());
        }
        let tick = tick.parse().map_err(|_| bad())?;
        let center = ae::Vec2::new(x, y);
        Ok(Self {
            tick,
            fracture: Fracture {
                look: Some(look.to_owned()),
                bounds: ae::Aabb::new(center, ae::Vec2::new(width, height) * 0.5),
                motion: BreakMotion::Strike {
                    contact: center - ae::Vec2::X * width * 0.5,
                    inward: ae::Vec2::X,
                },
                seed: seed.parse().map_err(|_| bad())?,
                tick,
            },
        })
    }
}

pub(super) fn publish(
    config: Res<super::SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    rooms: LiveRooms,
    mut effects: VfxWriter,
) {
    let Some(room) = rooms.sole() else {
        return;
    };
    for preview in config
        .fractures
        .iter()
        .filter(|preview| preview.tick == tick.0)
    {
        effects.write_in(Some(room), VfxMessage::Fracture(preview.fracture.clone()));
    }
}
