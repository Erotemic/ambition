//! Chart presentation for the shared map page.

use ambition_platformer2d::menu::map::browser::{
    MapAction, MapBrowserView, MapEntryKind, MapEntryView,
};
use ambition_platformer2d::menu::map::view::{map_label, MapPinKind};
use ambition_platformer2d::menu::{
    MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign,
};
use bevy::prelude::{Rect, Vec2};

use crate::menu::model::{add_edge_buttons, line, menu_look, MapLook, MenuPage, MenuPageAction};

/// Map viewport in percent of the menu page.
pub(crate) const MAP_FRAME: MenuRect = MenuRect::new(14.0, 29.0, 72.0, 53.0);
const ROOM_LABEL_SIZE: f32 = 2.0;
type MapModel = MenuPageModel<MenuPage, MenuPageAction>;

/// Build the default map page for both menu renderers.
pub fn build_map_page(view: Option<&MapBrowserView>) -> MenuPageModel<MenuPage, MenuPageAction> {
    let look = menu_look();
    let palette = look.map;
    let mut model = MenuPageModel::new(MenuPage::Map, line("menu.map.title"), colour(look.page));
    add_edge_buttons(&mut model, MenuPage::Map);
    model.text(
        50.0,
        4.0,
        1.9,
        view.and_then(|view| view.parent_title.clone())
            .unwrap_or_else(|| line("menu.map.title")),
        MenuTextAlign::Center,
        colour(palette.muted_label),
    );
    let Some(view) = view else {
        model.text(
            50.0,
            48.0,
            2.8,
            line("menu.map.unmapped"),
            MenuTextAlign::Center,
            colour(look.placeholder_body),
        );
        return model;
    };
    model.text(
        50.0,
        8.0,
        4.0,
        fit_title(&view.title, MAP_FRAME.w, 4.0),
        MenuTextAlign::Center,
        colour(palette.label),
    );

    draw_tools(&mut model, view);
    draw_chart(&mut model, &palette);
    let marks = landmarks(view);
    draw_entries(&mut model, view, &palette, &marks);
    let stride = 0.25 / marks.len().max(1) as f32;
    let chart = chart_bounds();
    for (index, mark) in marks.iter().enumerate() {
        draw_symbol(
            &mut model,
            &palette,
            *mark,
            0.72 + stride * index as f32,
            stride / 3.0,
            Some(chart),
        );
    }
    draw_footer(&mut model, view, &palette, &marks);
    model
}

fn draw_chart(model: &mut MapModel, palette: &MapLook) {
    let f = MAP_FRAME;
    model.layered_panel(f.inset(-0.33), colour(palette.stroke), None, 0.0);
    model.layered_panel(
        f,
        colour(palette.frame),
        Some(MenuPageAction::Map(MapAction::Canvas)),
        0.01,
    );
    for column in 1..12 {
        let x = f.x + f.w * column as f32 / 12.0;
        model.layered_panel(
            MenuRect::new(x - 0.075, f.y, 0.15, f.h),
            colour(palette.grid),
            None,
            0.025,
        );
    }
    for row in 1..6 {
        let y = f.y + f.h * row as f32 / 6.0;
        model.layered_panel(
            MenuRect::new(f.x, y - 0.095, f.w, 0.19),
            colour(palette.grid),
            None,
            0.03,
        );
    }
}

struct Tile<'a> {
    entry: &'a MapEntryView,
    bounds: Rect,
    label: bool,
}

fn draw_entries(
    model: &mut MapModel,
    view: &MapBrowserView,
    palette: &MapLook,
    marks: &[Landmark],
) {
    let mut entries: Vec<_> = view.entries.iter().collect();
    entries.sort_by_key(|entry| (entry.selected, entry.here));
    let mut tiles = Vec::new();
    for entry in entries {
        let largest = entry
            .pieces
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
            .map(|(i, _)| i);
        tiles.extend(entry.pieces.iter().enumerate().map(|(i, &piece)| Tile {
            entry,
            bounds: Rect::from_corners(chart_point(piece.min), chart_point(piece.max)),
            label: Some(i) == largest,
        }));
    }
    let stride = 0.58 / tiles.len().max(1) as f32;
    for (index, tile) in tiles.iter().enumerate() {
        let entry = tile.entry;
        let group = entry.kind == MapEntryKind::Group;
        let fill = match (entry.here, group, entry.visited) {
            (true, _, _) => palette.room_here,
            (_, true, _) => palette.group,
            (_, _, true) => palette.room,
            _ => palette.room_unvisited,
        };
        let outline = if entry.selected && !view.room_detail {
            palette.selection
        } else if entry.here {
            palette.current
        } else {
            palette.stroke
        };
        let layer = 0.1 + stride * index as f32;
        let action = (!view.room_detail).then_some(MenuPageAction::Map(MapAction::Open(entry.id)));
        let tile_rect = inset(tile.bounds, Vec2::new(0.10, 0.15));
        let inner = inset(
            tile_rect,
            if entry.selected {
                Vec2::new(0.33, 0.48)
            } else {
                Vec2::new(0.14, 0.21)
            },
        );
        layer_rect(model, tile_rect, colour(outline), action, layer);
        layer_rect(model, inner, colour(fill), None, layer + stride * 0.33);

        if group && inner.width() >= 5.0 && inner.height() >= 3.0 {
            let accent = Rect::from_corners(inner.min, Vec2::new(inner.max.x, inner.min.y + 0.48));
            layer_rect(model, accent, colour(outline), None, layer + stride * 0.66);
        }
        let font = if group {
            let chars = map_label(&entry.title).chars().count().max(1) as f32;
            ((inner.width() - 0.8) / ((chars + 0.5) * 0.48)).clamp(1.8, 2.55)
        } else {
            ROOM_LABEL_SIZE
        };
        if view.room_detail
            || !tile.label
            || !entry.visited
            || inner.width() < 8.5
            || inner.height() < font * 1.6
        {
            continue;
        }
        let name = fit_title(&entry.title, inner.width() - 0.8, font);
        let anchor = inner.center();
        let label = Rect::from_center_size(
            anchor,
            Vec2::new(name.chars().count() as f32 * font * 0.48, font),
        );
        if tiles[index + 1..]
            .iter()
            .any(|other| !other.bounds.intersect(label).is_empty())
            || marks
                .iter()
                .any(|mark| !mark.bounds().intersect(label).is_empty())
        {
            continue;
        }
        model.text(
            anchor.x,
            label.min.y,
            font,
            name,
            MenuTextAlign::Center,
            colour(if entry.here {
                palette.label_here
            } else {
                palette.label
            }),
        );
    }
}

fn draw_tools(model: &mut MapModel, view: &MapBrowserView) {
    let mut x = MAP_FRAME.x;
    for &tool in &view.tools {
        let (label, width, space) = match tool {
            MapAction::Up => (line("menu.map.back"), 9.5, 0.0),
            MapAction::Zoom(-1) => ("−".into(), 5.0, if x > MAP_FRAME.x { 1.2 } else { 0.0 }),
            MapAction::Fit => (line("menu.map.fit"), 5.5, 0.0),
            MapAction::Zoom(1) => ("+".into(), 5.0, 0.0),
            MapAction::Pan(-1, 0) => ("←".into(), 5.0, 1.2),
            MapAction::Pan(0, -1) => ("↑".into(), 5.0, 0.0),
            MapAction::Pan(0, 1) => ("↓".into(), 5.0, 0.0),
            MapAction::Pan(1, 0) => ("→".into(), 5.0, 0.0),
            MapAction::Browse(-1) => (line("menu.map.previous"), 8.0, 1.2),
            MapAction::Browse(1) => (line("menu.map.next"), 8.0, 0.0),
            MapAction::Travel => (line("menu.map.travel"), 16.0, 0.0),
            _ => continue,
        };
        x = if tool == MapAction::Travel {
            MAP_FRAME.x + MAP_FRAME.w - width
        } else {
            x + space
        };
        model.control(
            MenuRect::new(x, 17.5, width, 8.0),
            MenuControlKind::Action,
            label,
            None,
            view.focused_tool == Some(tool),
            tool == MapAction::Travel,
            Some(MenuPageAction::Map(tool)),
        );
        x += width + 0.65;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Symbol {
    Door,
    Resident,
    Shrine,
}

impl Symbol {
    fn key(self) -> &'static str {
        match self {
            Self::Door => "menu.map.legend.door",
            Self::Resident => "menu.map.legend.resident",
            Self::Shrine => "menu.map.legend.shrine",
        }
    }

    fn color(self, palette: &MapLook) -> [f32; 4] {
        match self {
            Self::Door => palette.door,
            Self::Resident => palette.pin_resident,
            Self::Shrine => palette.pin_shrine,
        }
    }
}

#[derive(Clone, Copy)]
struct Landmark {
    symbol: Symbol,
    centre: Vec2,
}

impl Landmark {
    fn bounds(self) -> Rect {
        let size = match self.symbol {
            Symbol::Door => Vec2::new(1.05, 1.7),
            Symbol::Resident => Vec2::new(0.85, 1.28),
            Symbol::Shrine => Vec2::new(1.35, 2.0),
        };
        Rect::from_center_size(self.centre, size)
    }
}

fn landmarks(view: &MapBrowserView) -> Vec<Landmark> {
    view.doors
        .iter()
        .map(|&at| Landmark {
            symbol: Symbol::Door,
            centre: chart_point(at),
        })
        .chain(view.pins.iter().map(|pin| Landmark {
            symbol: match pin.kind {
                MapPinKind::Resident => Symbol::Resident,
                MapPinKind::Shrine => Symbol::Shrine,
            },
            centre: chart_point(pin.at),
        }))
        .collect()
}

fn draw_symbol(
    model: &mut MapModel,
    palette: &MapLook,
    mark: Landmark,
    layer: f32,
    step: f32,
    clip: Option<Rect>,
) {
    let bounds = mark.bounds();
    let accent = colour(mark.symbol.color(palette));
    let ink = colour(palette.frame);
    let mut draw = |rect: Rect, color, z| {
        layer_rect(
            model,
            clip.map_or(rect, |clip| rect.intersect(clip)),
            color,
            None,
            z,
        );
    };
    match mark.symbol {
        Symbol::Door => {
            draw(bounds, accent, layer);
            draw(inset(bounds, Vec2::new(0.23, 0.33)), ink, layer + step);
        }
        Symbol::Resident => {
            draw(
                Rect::from_center_size(bounds.center(), bounds.size() * 1.22),
                ink,
                layer,
            );
            draw(bounds, accent, layer + step);
        }
        Symbol::Shrine => {
            draw(
                Rect::from_center_size(bounds.center(), bounds.size() * 0.78),
                ink,
                layer,
            );
            draw(
                Rect::from_center_size(bounds.center(), bounds.size() * Vec2::new(1.0, 0.36)),
                accent,
                layer + step,
            );
            draw(
                Rect::from_center_size(bounds.center(), bounds.size() * Vec2::new(0.36, 1.0)),
                accent,
                layer + 2.0 * step,
            );
        }
    }
}

fn draw_footer(model: &mut MapModel, view: &MapBrowserView, palette: &MapLook, marks: &[Landmark]) {
    let selected = view.entries.iter().find(|entry| entry.selected);
    if let Some(entry) = selected {
        let label = match (view.room_detail, entry.here) {
            (true, true) => line("menu.map.here"),
            (true, false) => String::new(),
            (false, true) => format!("{} · {}", entry.title, line("menu.map.here")),
            (false, false) => entry.title.clone(),
        };
        if !label.is_empty() {
            model.text(
                50.0,
                85.0,
                2.6,
                label,
                MenuTextAlign::Center,
                colour(if entry.here {
                    palette.current
                } else {
                    palette.label
                }),
            );
        }
    } else {
        model.text(
            50.0,
            54.0,
            2.5,
            line("menu.map.empty"),
            MenuTextAlign::Center,
            colour(palette.muted_label),
        );
    }
    let symbols: Vec<_> = [Symbol::Door, Symbol::Resident, Symbol::Shrine]
        .into_iter()
        .filter(|symbol| marks.iter().any(|mark| mark.symbol == *symbol))
        .collect();
    let unvisited = view
        .entries
        .iter()
        .any(|entry| !entry.visited && !entry.pieces.is_empty());
    let count = (symbols.len() + usize::from(unvisited)) as f32;
    let mut x = 50.0 - 6.0 * (count - 1.0);
    let y = 92.0;
    for symbol in symbols {
        draw_symbol(
            model,
            palette,
            Landmark {
                symbol,
                centre: Vec2::new(x - 4.0, y),
            },
            0.74,
            0.008,
            None,
        );
        model.text(
            x + 1.0,
            y,
            1.9,
            line(symbol.key()),
            MenuTextAlign::Center,
            colour(palette.muted_label),
        );
        x += 12.0;
    }
    if unvisited {
        let sample = MenuRect::new(x - 4.6, y - 0.7, 1.2, 1.4);
        model.layered_panel(sample, colour(palette.stroke), None, 0.74);
        model.layered_panel(
            sample.inset(0.15),
            colour(palette.room_unvisited),
            None,
            0.75,
        );
        model.text(
            x + 1.0,
            y,
            1.9,
            line("menu.map.legend.unvisited"),
            MenuTextAlign::Center,
            colour(palette.muted_label),
        );
    }
}

fn chart_point(at: Vec2) -> Vec2 {
    Vec2::new(
        MAP_FRAME.x + at.x * MAP_FRAME.w,
        MAP_FRAME.y + at.y * MAP_FRAME.h,
    )
}

fn chart_bounds() -> Rect {
    Rect::from_corners(chart_point(Vec2::ZERO), chart_point(Vec2::ONE))
}

fn inset(rect: Rect, amount: Vec2) -> Rect {
    let pad = amount.min(rect.size() * 0.18);
    Rect {
        min: rect.min + pad,
        max: rect.max - pad,
    }
}

fn layer_rect(
    model: &mut MapModel,
    rect: Rect,
    color: MenuColor,
    action: Option<MenuPageAction>,
    layer: f32,
) {
    if !rect.is_empty() {
        model.layered_panel(
            MenuRect::new(rect.min.x, rect.min.y, rect.width(), rect.height()),
            color,
            action,
            layer,
        );
    }
}

fn colour([r, g, b, a]: [f32; 4]) -> MenuColor {
    MenuColor::rgba(r, g, b, a)
}

fn fit_title(title: &str, width: f32, font: f32) -> String {
    let label = map_label(title);
    let limit = (width / (font * 0.48)).floor().max(3.0) as usize;
    if label.chars().count() <= limit {
        return label;
    }
    let cut: String = label.chars().take(limit - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
pub(super) fn short_title(title: &str, width: f32) -> String {
    fit_title(title, width, ROOM_LABEL_SIZE)
}
