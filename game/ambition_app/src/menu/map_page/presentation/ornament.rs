//! Draw the plates, corner trim, and engravings on the map.

use ambition_platformer2d::menu::{MenuColor, MenuRect};
use bevy::prelude::{Rect, Vec2};

use super::{colour, inset, MapLook, MapModel, MAP_FRAME};

pub(super) const TITLE_WIDTH: f32 = 61.0;

pub(super) fn title_plate(model: &mut MapModel, palette: &MapLook) {
    let plate = MenuRect::new(18.0, 2.2, 64.0, 12.6);
    plaque(model, plate, palette);
    for x in [15.5, 84.5] {
        paint(
            model,
            MenuRect::new(x - 2.5, 8.2, 5.0, 0.24),
            colour(palette.engraving),
            0.04,
        );
        diamond(model, Vec2::new(x, 8.4), colour(palette.brass));
    }
}

pub(super) fn folio(model: &mut MapModel, palette: &MapLook) {
    let frame = MAP_FRAME;
    let trim = colour(palette.brass);
    let muted = colour(palette.engraving);
    plaque(
        model,
        MenuRect::new(frame.x - 0.4, 16.7, frame.w + 0.8, 9.5),
        palette,
    );
    corner_trim(model, frame.inset(-0.84), Vec2::new(4.7, 6.1), trim, 0.048);
    for column in 1..12 {
        let x = frame.x + frame.w * column as f32 / 12.0;
        let tick = if column % 3 == 0 { 0.72 } else { 0.36 };
        for y in [frame.y - 0.84, frame.y + frame.h + 0.84 - tick] {
            paint(model, MenuRect::new(x - 0.09, y, 0.18, tick), muted, 0.048);
        }
    }
    plaque(model, MenuRect::new(frame.x, 84.1, frame.w, 11.6), palette);
    for x in [frame.x + 2.0, frame.x + frame.w - 2.0] {
        diamond(model, Vec2::new(x, 89.9), trim);
    }
}

pub(super) fn room_engraving(
    model: &mut MapModel,
    palette: &MapLook,
    bounds: Rect,
    fill: [f32; 4],
    layer: f32,
) {
    if bounds.width() < 12.0 || bounds.height() < 8.0 {
        return;
    }
    let inner = inset(bounds, Vec2::new(0.9, 1.3));
    let trim = colour(std::array::from_fn(|i| {
        fill[i] * 0.66 + palette.brass[i] * 0.34
    }));
    corner_trim(
        model,
        MenuRect::new(inner.min.x, inner.min.y, inner.width(), inner.height()),
        (inner.size() * 0.23).min(Vec2::new(6.0, 8.0)),
        trim,
        layer,
    );
    let rule = colour(std::array::from_fn(|i| {
        fill[i] * 0.82 + palette.brass[i] * 0.18
    }));
    let left = inner.min.x + 1.5;
    let width = inner.width() - 3.0;
    for row in 1..5 {
        let y = inner.min.y + inner.height() * row as f32 / 5.0;
        paint(model, MenuRect::new(left, y, width, 0.16), rule, layer);
    }
}

fn plaque(model: &mut MapModel, rect: MenuRect, palette: &MapLook) {
    paint(model, rect, colour(palette.engraving), 0.054);
    paint(model, rect.inset(0.23), colour(palette.plate), 0.056);
    corner_trim(
        model,
        rect,
        Vec2::new(3.4, 3.5),
        colour(palette.brass),
        0.06,
    );
}

fn corner_trim(model: &mut MapModel, rect: MenuRect, length: Vec2, color: MenuColor, layer: f32) {
    let stroke = Vec2::new(0.25, 0.34);
    for x in [rect.x, rect.x + rect.w - length.x] {
        let side = if x == rect.x {
            x
        } else {
            x + length.x - stroke.x
        };
        paint(
            model,
            MenuRect::new(x, rect.y, length.x, stroke.y),
            color,
            layer,
        );
        paint(
            model,
            MenuRect::new(x, rect.y + rect.h - stroke.y, length.x, stroke.y),
            color,
            layer,
        );
        paint(
            model,
            MenuRect::new(side, rect.y + stroke.y, stroke.x, length.y - stroke.y),
            color,
            layer,
        );
        paint(
            model,
            MenuRect::new(
                side,
                rect.y + rect.h - length.y,
                stroke.x,
                length.y - stroke.y,
            ),
            color,
            layer,
        );
    }
}

fn diamond(model: &mut MapModel, centre: Vec2, color: MenuColor) {
    let step = 0.52;
    for (row, w) in [0.34, 0.85, 1.38, 0.85, 0.34].into_iter().enumerate() {
        let y = centre.y + (row as f32 - 2.5) * step;
        paint(
            model,
            MenuRect::new(centre.x - w * 0.5, y, w, step),
            color,
            0.064,
        );
    }
}

fn paint(model: &mut MapModel, rect: MenuRect, color: MenuColor, layer: f32) {
    model.layered_panel(rect, color, None, layer);
}
