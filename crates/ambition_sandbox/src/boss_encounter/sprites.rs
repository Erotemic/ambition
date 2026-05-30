//! Boss spritesheet animation, parallel to `character_sprites` but with the
//! boss generator's own animation rows (rest / floor_slam / side_sweep /
//! spike_halo / dash_echo / hit / death) instead of the standard 8-row
//! `CharacterAnim` grid.
//!
//! Bosses don't walk/run/jump like a platforming character, so reusing the
//! `CharacterAnim` enum would either force the boss generator to emit
//! placeholder rows or force the gameplay layer to mis-label its
//! animations. The split keeps both clean and makes it obvious which sheet
//! a given pipeline expects.

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::features::FeatureVisualKind;

/// Boss animation rows in the order the generator emits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossAnim {
    Rest = 0,
    FloorSlam = 1,
    SideSweep = 2,
    SpikeHalo = 3,
    DashEcho = 4,
    Hit = 5,
    Death = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for a boss sheet. Mirror of `character_sprites::CharacterSheetSpec`
/// with sparse rows so different boss generators can emit different
/// row subsets (e.g. the gradient sentinel ships 7 rows; the mockingbird
/// ships 6 with no `FloorSlam`/`SideSweep`). Per-target anchor/scale
/// tuning keeps bosses rendered at the right scale relative to playable
/// characters.
#[derive(Clone, Copy, Debug)]
pub struct BossSheetSpec {
    pub label_width: u32,
    /// Per-frame size in source-image pixels after the gen2d union-bbox
    /// crop. Each generator picks its own canvas; resync after a sheet
    /// regen by checking the manifest's `frame_size` block.
    pub frame_width: u32,
    pub frame_height: u32,
    /// Animation rows in the order the generator emits them in the PNG.
    /// Sparse: a sheet may omit any row except `Rest` (the fallback).
    pub rows: &'static [(BossAnim, AnimRow)],
    /// Multiplier applied to an entity's collision-box max dimension to
    /// derive the rendered sprite's height. Width is derived from the
    /// cropped frame's aspect ratio so the boss isn't squashed.
    pub collision_scale: f32,
    /// Custom sprite anchor on the y axis. Tuned per sheet because each
    /// generator's character takes different vertical space within the
    /// frame.
    pub feet_anchor_y: f32,
    /// Sample inset (pixels) on every URect to prevent bilinear filtering
    /// from sampling neighboring frames.
    pub frame_sample_inset: u32,
    /// True for flying / floating bosses whose body should be centered
    /// in the collision box rather than anchored to its bottom (the
    /// default for ground-locked humanoid bosses). When set,
    /// `collision_anchor` short-circuits and treats `feet_anchor_y` as
    /// the body's normalized vertical offset within the sprite quad
    /// (Bevy +Y-up; 0 = sprite center). The gradient sentinel is
    /// ground-locked so it stays `false`; the mockingbird is airborne
    /// so it sets this `true` and the sprite quad sits centered on
    /// the AABB instead of hanging below it.
    pub body_centered: bool,
}

// `feet_anchor_y` matches the body-metrics measurement for the current
// generator output. Resync after regenerating the boss sheet by checking
// the manifest's `body_metrics.feet_anchor_norm.y`.
pub const BOSS_SHEET: BossSheetSpec = BossSheetSpec {
    label_width: 100,
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            BossAnim::Rest,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            BossAnim::FloorSlam,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.082,
            },
        ),
        (
            BossAnim::SideSweep,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.072,
            },
        ),
        (
            BossAnim::SpikeHalo,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.092,
            },
        ),
        (
            BossAnim::DashEcho,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
        (
            BossAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            BossAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
    ],
    // Bosses are visually larger than goblins; a slightly smaller scale
    // factor stops them from overpowering the rendered scene.
    collision_scale: 1.6,
    feet_anchor_y: -0.336,
    frame_sample_inset: 1,
    body_centered: false,
};

/// The Mockingbird boss sheet from the standalone Python generator
/// (`tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py`,
/// installed via that script's `install` command). Rows in PNG order:
/// hover, thrust, bite, slash, hit, death. Mapped onto the existing
/// `BossAnim` vocabulary so the gameplay layer can issue the same
/// verbs across both bosses:
/// - `hover`  → `Rest`      (the long idle / hover-in-place pose)
/// - `thrust` → `DashEcho`  (the swoop / dive attack)
/// - `bite`   → `FloorSlam` (close-range commit attack)
/// - `slash`  → `SpikeHalo` (used for the ranged Hadouken / fireball
///   beat — the slash pose telegraphs an outward strike that the
///   sandbox controller pairs with a projectile spawn)
/// - `hit`/`death` keep their meanings.
///
/// `SideSweep` is unmapped; `BossAnimator::request` falls back to
/// `Rest` if the schedule asks for a row this sheet doesn't ship.
pub const MOCKINGBIRD_SHEET: BossSheetSpec = BossSheetSpec {
    // The mockingbird sheet has no per-row label strip — frame 0
    // sits at x=0 — so label_width is zero.
    label_width: 0,
    // 576×216 wide frames straight from the manifest. The extra edge
    // margin keeps the pointed nose/flame silhouettes safely inside each
    // atlas rect while still spending native pixels on the bird instead of
    // packing a short/wide silhouette into a mostly-empty square canvas.
    frame_width: 576,
    frame_height: 216,
    rows: &[
        (
            BossAnim::Rest,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.110,
            },
        ),
        (
            BossAnim::DashEcho,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.090,
            },
        ),
        (
            BossAnim::FloorSlam,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.090,
            },
        ),
        (
            BossAnim::SpikeHalo,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.088,
            },
        ),
        (
            BossAnim::Hit,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.080,
            },
        ),
        (
            BossAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.105,
            },
        ),
    ],
    // The 576×216 generator output now fits the wide silhouette a bit
    // more conservatively (roughly y≈34..194 on the hover row) so the
    // beak / tail / flame tips keep safer atlas margins during in-game
    // animation. We still no longer need the old 3× Rust-side blow-up
    // used for the sparse 256×256 sheet.
    // 1.25 keeps the visible body close to the authored 185px tall
    // combat box while using much denser native source pixels.
    collision_scale: 1.25,
    // `body_centered: true` below makes this read as the body's
    // normalized vertical offset within the sprite quad rather than
    // a feet-on-floor delta. Texture bbox-center sits near y=101 of
    // 216 → (108-114)/108 ≈ -0.05 in Bevy +Y-up.
    feet_anchor_y: -0.055,
    frame_sample_inset: 1,
    body_centered: true,
};

/// Smirking Behemoth / "You Have To Cut The Rope" boss sheet.
///
/// Generated by
/// `tools/ambition_sprite2d_renderer ... publish smirking_behemoth_boss`.
/// Rows in PNG order: `rest`, `mouth_open`, `eye_beam`, `death`.
/// Those map onto the existing gameplay animation vocabulary as:
/// - `rest`       -> `Rest`
/// - `mouth_open` -> `FloorSlam` (close-range / open-mouth tell)
/// - `eye_beam`   -> `SpikeHalo` (flashing eye / ranged tell)
/// - `death`      -> `Death`
///
/// Rows that the sheet does not ship (`SideSweep`, `DashEcho`, `Hit`)
/// fall back to `Rest` through `BossSheetSpec::resolve_anim`.
pub const SMIRKING_BEHEMOTH_SHEET: BossSheetSpec = BossSheetSpec {
    label_width: 100,
    frame_width: 208,
    frame_height: 240,
    rows: &[
        (
            BossAnim::Rest,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.125,
            },
        ),
        (
            BossAnim::FloorSlam,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.092,
            },
        ),
        (
            BossAnim::SpikeHalo,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.082,
            },
        ),
        (
            BossAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.108,
            },
        ),
    ],
    // The visible monolith body is 224 px tall inside a 240 px frame.
    // Use 240/224 so the rendered opaque body height matches the
    // LDtk/authored collision body instead of growing taller and being
    // pushed upward by collision.
    collision_scale: 1.071_429,
    feet_anchor_y: -0.433_333,
    frame_sample_inset: 1,
    body_centered: false,
};

impl BossSheetSpec {
    fn row_index(&self, anim: BossAnim) -> Option<usize> {
        self.rows.iter().position(|(row_anim, _)| *row_anim == anim)
    }

    /// Resolve a requested animation against this sheet's row set.
    /// Falls back to `Rest` if the requested row isn't shipped (e.g.
    /// the mockingbird sheet has no `SideSweep`). Bosses without a
    /// `Rest` row would crash the indexer; the static `BOSS_SHEET` /
    /// `MOCKINGBIRD_SHEET` constants both ship one. Future sheets that
    /// omit `Rest` should fail loudly, not silently — change this if
    /// the contract changes.
    pub fn resolve_anim(&self, anim: BossAnim) -> BossAnim {
        if self.row_index(anim).is_some() {
            anim
        } else {
            BossAnim::Rest
        }
    }

    pub(crate) fn row(&self, anim: BossAnim) -> AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("boss sprite sheet must define a Rest row");
        self.rows[idx].1
    }

    pub fn build_atlas(&self) -> TextureAtlasLayout {
        let max_frames = self
            .rows
            .iter()
            .map(|(_, r)| r.frame_count)
            .max()
            .unwrap_or(0) as u32;
        let total_w = self.label_width + max_frames * self.frame_width;
        let total_h = self.rows.len() as u32 * self.frame_height;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (row_idx, (_, row)) in self.rows.iter().enumerate() {
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = row_idx as u32 * self.frame_height;
                let min = UVec2::new(x + inset, y + inset);
                let max = UVec2::new(x + self.frame_width - inset, y + self.frame_height - inset);
                layout.add_texture(URect { min, max });
            }
        }
        layout
    }

    pub fn flat_index(&self, anim: BossAnim, frame: usize) -> usize {
        let resolved = self.resolve_anim(anim);
        let row = self
            .row_index(resolved)
            .expect("boss sprite sheet must define a Rest row");
        let frames_before: usize = self.rows[..row].iter().map(|(_, r)| r.frame_count).sum();
        let max_frame = self.rows[row].1.frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    pub fn frame_count(&self, anim: BossAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn render_size(&self, collision: Vec2) -> Vec2 {
        // Height collision-driven; width preserves the cropped frame's
        // aspect ratio so the boss isn't squashed when frames are non-square.
        let height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let width = height * (self.frame_width as f32 / self.frame_height as f32);
        Vec2::new(width, height)
    }

    pub fn anchor(&self) -> Anchor {
        Anchor(Vec2::new(0.0, self.feet_anchor_y))
    }

    /// Collision-aware anchor that places the rendered boss's feet on the
    /// bottom of the collision box rather than at its centre. Mirrors
    /// `character_sprites::feet_anchor_for` — see that function for the
    /// derivation.
    ///
    /// For `body_centered` sheets (flying bosses) we skip the feet-delta
    /// term and use `feet_anchor_y` directly as the body-center offset.
    /// Otherwise the sprite quad would hang below the AABB the same way
    /// a humanoid sheet hangs below its waist when the anchor is at
    /// "feet" — wrong silhouette for an airborne creature.
    pub fn collision_anchor(&self, collision: Vec2) -> Anchor {
        if self.body_centered {
            return Anchor(Vec2::new(0.0, self.feet_anchor_y));
        }
        let render_height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let half_collision_y = collision.y * 0.5;
        let ay = self.feet_anchor_y + half_collision_y / render_height;
        Anchor(Vec2::new(0.0, ay))
    }
}

#[derive(Clone)]
pub struct BossSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: BossSheetSpec,
}

pub(crate) const BOSS_FILENAME: &str = "boss_spritesheet.png";
pub(crate) const MOCKINGBIRD_FILENAME: &str = "mockingbird_boss/mockingbird_boss_spritesheet.png";
pub(crate) const GNU_TON_FILENAME: &str = "gnu_ton_boss/gnu_ton_boss_spritesheet.png";
pub(crate) const SMIRKING_BEHEMOTH_FILENAME: &str = "smirking_behemoth_boss_spritesheet.png";
// Layered GNU-ton sheets emitted alongside the full sheet by the
// Python generator. `_body` excludes hands + attack VFX; `_hands` is
// only hands + VFX. Runtime z-layers the body behind platforms and the
// hands in front so the player can read jump targets and incoming
// danger separately.
pub(crate) const GNU_TON_BODY_FILENAME: &str = "gnu_ton_boss/gnu_ton_boss_body_spritesheet.png";
pub(crate) const GNU_TON_HANDS_FILENAME: &str = "gnu_ton_boss/gnu_ton_boss_hands_spritesheet.png";

/// GNU-ton boss sheet.
///
/// Frame layout: 768×576 pixels per frame, 6 animation rows. Bumped from
/// the older 512×384 to keep the giant readable when blown up to its
/// in-game render scale.
/// Rows map to BossAnim as: Rest/FloorSlam/SideSweep/SpikeHalo/Hit/Death.
///
/// The collision box is placed at the giant's shoulder ridge, where the
/// GNU-ton scholar's feet touch the body. The runtime GNU-ton hitboxes
/// use the same design-space anchor, so the head and hands line up with
/// the generated sprite instead of a generic boss rectangle.
///
/// `collision_scale: 4.5` makes the 768×576 sprite render much larger
/// than the authored boss box, so the giant body dominates the arena
/// while runtime hitboxes stay tied to named parts.
pub const GNU_TON_SHEET: BossSheetSpec = BossSheetSpec {
    label_width: 0,
    frame_width: 768,
    frame_height: 576,
    rows: &[
        (
            BossAnim::Rest,
            AnimRow {
                frame_count: 10,
                duration_secs: 0.110,
            },
        ),
        (
            BossAnim::FloorSlam,
            AnimRow {
                frame_count: 10,
                duration_secs: 0.072,
            },
        ),
        (
            BossAnim::SideSweep,
            AnimRow {
                frame_count: 10,
                duration_secs: 0.065,
            },
        ),
        (
            BossAnim::SpikeHalo,
            AnimRow {
                frame_count: 9,
                duration_secs: 0.090,
            },
        ),
        (
            BossAnim::Hit,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.080,
            },
        ),
        (
            BossAnim::Death,
            AnimRow {
                frame_count: 10,
                duration_secs: 0.105,
            },
        ),
    ],
    collision_scale: 4.5,
    // Design-space shoulder top moved to y ≈ -2 (REST_BODY_Y 60 - 62) in
    // the new 768×576 frame; Bevy anchor.y = +2 / 576. The scholar sits
    // 18 px above the shoulder (smaller silhouette), so `BossRuntime::pos`
    // lands at the shoulder ridge — same semantic as before.
    feet_anchor_y: 2.0 / 576.0,
    frame_sample_inset: 1,
    body_centered: true,
};

/// Sandbox-side `(label, filename)` rows for every boss spritesheet
/// the sandbox knows about. The aggregator in
/// [`crate::sandbox_assets`] registers one catalog entry per row;
/// loaders here read the catalog by label.
pub fn all_boss_sprite_filenames() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gradient_sentinel", BOSS_FILENAME),
        ("mockingbird", MOCKINGBIRD_FILENAME),
        ("gnu_ton", GNU_TON_FILENAME),
        ("smirking_behemoth_boss", SMIRKING_BEHEMOTH_FILENAME),
        ("gnu_ton_body", GNU_TON_BODY_FILENAME),
        ("gnu_ton_hands", GNU_TON_HANDS_FILENAME),
    ]
}

/// Build the boss sprite asset for the gradient sentinel sheet.
/// Returns `None` if the catalog reports the asset disabled or the
/// active profile's optional-image gate skips it — callers fall back
/// to the static `EntitySprite::BossCore` image, which in turn falls
/// back to the colored rectangle.
pub fn load_boss_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(
        catalog,
        asset_server,
        layouts,
        "gradient_sentinel",
        BOSS_SHEET,
    )
}

/// Build the boss sprite asset for the mockingbird sheet (installed by
/// `tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py install`).
/// Returns `None` if the PNG is missing — the rendering layer keeps
/// the colored-rectangle fallback for that boss.
pub fn load_mockingbird_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(
        catalog,
        asset_server,
        layouts,
        "mockingbird",
        MOCKINGBIRD_SHEET,
    )
}

/// Build the boss sprite asset for the GNU-ton sheet (installed by
/// `tools/ambition_sprite2d_renderer render-publish gnu_ton_boss`).
/// Returns `None` if the PNG is missing — falls back to colored rectangle.
pub fn load_gnu_ton_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(catalog, asset_server, layouts, "gnu_ton", GNU_TON_SHEET)
}

/// Build the Smirking Behemoth boss sprite asset.
///
/// Returns `None` until the sprite renderer has published
/// `smirking_behemoth_boss_spritesheet.png` into the sandbox sprite
/// asset folder; the rendering layer then falls back to the generic
/// boss sheet instead of hard-failing the room.
pub fn load_smirking_behemoth_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(
        catalog,
        asset_server,
        layouts,
        "smirking_behemoth_boss",
        SMIRKING_BEHEMOTH_SHEET,
    )
}

/// Body-only GNU-ton sheet (no hands, no attack VFX). Rendered behind
/// platforms so the player can see jump targets through the giant body.
/// Same atlas layout as `GNU_TON_SHEET` so `flat_index` works for both.
pub fn load_gnu_ton_body_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(
        catalog,
        asset_server,
        layouts,
        "gnu_ton_body",
        GNU_TON_SHEET,
    )
}

/// Hands-only GNU-ton sheet (with attack VFX). Rendered in front of
/// platforms so incoming danger reads clearly.
pub fn load_gnu_ton_hands_sprite_in(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(
        catalog,
        asset_server,
        layouts,
        "gnu_ton_hands",
        GNU_TON_SHEET,
    )
}

fn load_named_boss_sprite_via_catalog(
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    label: &str,
    spec: BossSheetSpec,
) -> Option<BossSpriteAsset> {
    let id = crate::assets::sandbox_assets::ids::boss_sprite(label);
    let Some(path) = catalog.try_path_for_load(&id) else {
        eprintln!(
            "[boss_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to entity sprite",
            catalog.profile().label(),
        );
        return None;
    };
    let layout = layouts.add(spec.build_atlas());
    Some(BossSpriteAsset {
        texture: asset_server.load(path),
        layout,
        spec,
    })
}

/// Per-entity boss animation cursor. Same shape as `CharacterAnimator` but
/// keyed off `BossAnim`.
#[derive(Component)]
pub struct BossAnimator {
    pub spec: BossSheetSpec,
    pub current: BossAnim,
    pub drive_phase: BossAnimDrivePhase,
    pub frame: usize,
    pub elapsed: f32,
    pub clip_held: bool,
}

impl BossAnimator {
    pub fn new(spec: BossSheetSpec) -> Self {
        Self {
            spec,
            current: BossAnim::Rest,
            drive_phase: BossAnimDrivePhase::Rest,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
        }
    }

    pub fn request(&mut self, anim: BossAnim) {
        self.request_for_phase(anim, BossAnimDrivePhase::Rest);
    }

    /// Select a boss animation row and the gameplay phase that is
    /// driving it. Some bosses intentionally use the same visual row
    /// for windup and strike. Treating the phase as part of the
    /// animation identity keeps the row from playing once during the
    /// telegraph, holding its final frame through the strike, and then
    /// snapping boxes back to rest later.
    pub fn request_for_phase(&mut self, anim: BossAnim, drive_phase: BossAnimDrivePhase) {
        if self.current == anim && self.drive_phase == drive_phase {
            return;
        }
        self.current = anim;
        self.drive_phase = drive_phase;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    pub fn tick(&mut self, dt: f32) -> usize {
        let row = self.spec.row(self.current);
        if row.frame_count == 0 || row.duration_secs <= 0.0 {
            return self.spec.flat_index(self.current, self.frame);
        }
        if self.clip_held {
            return self.spec.flat_index(self.current, self.frame);
        }
        self.elapsed += dt;
        while self.elapsed >= row.duration_secs {
            self.elapsed -= row.duration_secs;
            if self.frame + 1 >= row.frame_count {
                if non_looping(self.current) {
                    self.frame = row.frame_count - 1;
                    self.clip_held = true;
                    break;
                } else {
                    self.frame = 0;
                }
            } else {
                self.frame += 1;
            }
        }
        self.spec.flat_index(self.current, self.frame)
    }
}

fn non_looping(anim: BossAnim) -> bool {
    matches!(
        anim,
        BossAnim::FloorSlam
            | BossAnim::SideSweep
            | BossAnim::SpikeHalo
            | BossAnim::DashEcho
            | BossAnim::Hit
            | BossAnim::Death
    )
}

/// Gameplay phase currently driving a boss animation row.
///
/// This is deliberately separate from [`BossAnim`]. A single authored row
/// can be reused for both telegraph and strike; those are separate plays of
/// the clip, not one long held animation. Keeping the phase in the animator
/// identity makes sprite frames, authored boxes, and debug overlays advance
/// together across phase boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossAnimDrivePhase {
    Rest,
    Windup,
    Active,
    Hit,
    Death,
}

/// Snapshot of boss state used to drive its animation. Pulled from
/// `BossRuntime` by the rendering layer so this module stays free of
/// gameplay imports.
#[derive(Clone, Copy, Debug)]
pub struct BossAnimState {
    pub alive: bool,
    pub attack_active: bool,
    pub attack_windup: bool,
    pub hit_flash: bool,
    /// Profile-resolved animation to play during windup, when the
    /// gameplay layer can map the boss's active profile onto this
    /// sheet's row vocabulary. `None` keeps the generic fallback.
    pub windup_anim: Option<BossAnim>,
    /// Profile-resolved animation to play during the strike.
    pub active_anim: Option<BossAnim>,
    /// Boss-pattern timer used to vary which active-attack clip plays
    /// when no profile-resolved animation is available.
    pub pattern_timer: f32,
    /// Horizontal facing: -1.0 = left, +1.0 = right.
    pub facing: f32,
}

impl BossAnimState {
    pub fn drive_phase(self) -> BossAnimDrivePhase {
        if !self.alive {
            return BossAnimDrivePhase::Death;
        }
        if self.hit_flash {
            return BossAnimDrivePhase::Hit;
        }
        if self.attack_windup {
            return BossAnimDrivePhase::Windup;
        }
        if self.attack_active {
            return BossAnimDrivePhase::Active;
        }
        BossAnimDrivePhase::Rest
    }
}

pub fn pick_boss_anim(state: BossAnimState) -> BossAnim {
    if !state.alive {
        return BossAnim::Death;
    }
    if state.hit_flash {
        return BossAnim::Hit;
    }
    if state.attack_windup {
        return state.windup_anim.unwrap_or(BossAnim::SpikeHalo);
    }
    if state.attack_active {
        if let Some(anim) = state.active_anim {
            return anim;
        }
        // Rotate active-attack clips so the boss reads with variety even
        // though the gameplay AI is currently a single pattern.
        let bucket = (state.pattern_timer.abs() as i32) % 3;
        return match bucket {
            0 => BossAnim::FloorSlam,
            1 => BossAnim::SideSweep,
            _ => BossAnim::DashEcho,
        };
    }
    BossAnim::Rest
}

/// True if a feature kind is "the boss". Kept here so the rendering layer
/// can ask `BossSprites::should_animate(kind)` without inlining the match.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_boss_kind(kind: FeatureVisualKind) -> bool {
    matches!(kind, FeatureVisualKind::Boss)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_sheet_has_seven_animation_rows() {
        // The enum has 7 variants and the spec has 7 rows; if these
        // ever drift, indexing by `anim as usize` would panic at
        // runtime.
        assert_eq!(BOSS_SHEET.rows.len(), 7);
    }

    #[test]
    fn mockingbird_anchor_keeps_body_inside_aabb() {
        // Mockingbird is body_centered, so collision_anchor must return
        // the spec's feet_anchor_y verbatim (no half_collision_y boost).
        // Concrete repro: with the old collision_anchor the bird hung
        // ~half its render height below the AABB.
        let aabb = Vec2::new(150.0, 185.0);
        let anchor = MOCKINGBIRD_SHEET.collision_anchor(aabb);
        assert!(MOCKINGBIRD_SHEET.body_centered);
        // feet_anchor_y is small (slight downward offset), nowhere
        // near +0.5 (which is what the feet-delta term would push it
        // to for this AABB / scale combo).
        assert!(anchor.0.y.abs() < 0.20, "anchor.y = {}", anchor.0.y);
    }

    #[test]
    fn boss_sheet_anchor_adds_feet_delta_when_not_body_centered() {
        // The gradient sentinel keeps the feet-on-floor anchoring:
        // collision_anchor adds half_collision_y / render_height to
        // feet_anchor_y. Pin the additive behavior — if body_centered
        // accidentally flips to true here, the sprite would slide
        // half its render height down.
        assert!(!BOSS_SHEET.body_centered);
        let aabb = Vec2::new(60.0, 80.0);
        let anchor = BOSS_SHEET.collision_anchor(aabb);
        let render_h = aabb.x.max(aabb.y).max(8.0) * BOSS_SHEET.collision_scale;
        let expected = BOSS_SHEET.feet_anchor_y + (aabb.y * 0.5) / render_h;
        assert!(
            (anchor.0.y - expected).abs() < 1e-4,
            "expected {} got {}",
            expected,
            anchor.0.y
        );
        // And the additive term must be non-trivial (not a no-op).
        assert!((anchor.0.y - BOSS_SHEET.feet_anchor_y).abs() > 0.05);
    }

    #[test]
    fn mockingbird_sheet_maps_six_rows_with_passthrough_for_missing() {
        // The mockingbird sheet ships hover/thrust/bite/slash/hit/death,
        // mapped onto the existing BossAnim vocabulary. SideSweep is
        // intentionally absent — `resolve_anim` must fall back to Rest
        // so a schedule that asks for SideSweep doesn't panic the
        // indexer.
        assert_eq!(MOCKINGBIRD_SHEET.rows.len(), 6);
        assert_eq!(
            MOCKINGBIRD_SHEET.resolve_anim(BossAnim::SideSweep),
            BossAnim::Rest
        );
        // The mapped rows resolve to themselves.
        for anim in [
            BossAnim::Rest,
            BossAnim::DashEcho,
            BossAnim::FloorSlam,
            BossAnim::SpikeHalo,
            BossAnim::Hit,
            BossAnim::Death,
        ] {
            assert_eq!(MOCKINGBIRD_SHEET.resolve_anim(anim), anim);
        }
    }

    #[test]
    fn frame_count_matches_spec_rows() {
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::Rest), 8);
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::FloorSlam), 7);
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::Death), 8);
    }

    #[test]
    fn flat_index_lays_rows_end_to_end() {
        // First frame of each row sits at the cumulative sum of prior
        // frame counts. The first row starts at 0.
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::Rest, 0), 0);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::FloorSlam, 0), 8);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::SideSweep, 0), 8 + 7);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::SpikeHalo, 0), 8 + 7 + 7);
    }

    #[test]
    fn flat_index_clamps_to_last_frame_of_row() {
        // Asking for frame index past the end of a row clamps to the
        // last valid frame; this avoids out-of-bounds atlas reads when
        // an animation cursor overshoots due to a long delta-t.
        let last_rest = BOSS_SHEET.flat_index(BossAnim::Rest, 999);
        assert_eq!(last_rest, BOSS_SHEET.frame_count(BossAnim::Rest) - 1);
    }

    #[test]
    fn render_size_preserves_frame_aspect_ratio() {
        // BOSS_SHEET is 128x128 (square) → width / height = 1.
        let size = BOSS_SHEET.render_size(Vec2::new(50.0, 50.0));
        assert!((size.x - size.y).abs() < 1e-3);
    }

    #[test]
    fn render_size_floors_at_minimum_extent() {
        // collision_scale * max(min_extent, 8.0): collision smaller
        // than 8 should still produce a visible quad.
        let size = BOSS_SHEET.render_size(Vec2::new(2.0, 2.0));
        assert!(size.y >= 8.0 * BOSS_SHEET.collision_scale - 1e-3);
    }

    #[test]
    fn is_boss_kind_only_true_for_boss_variant() {
        assert!(is_boss_kind(FeatureVisualKind::Boss));
        assert!(!is_boss_kind(FeatureVisualKind::Enemy));
        assert!(!is_boss_kind(FeatureVisualKind::Hazard));
        assert!(!is_boss_kind(FeatureVisualKind::Chest));
    }

    #[test]
    fn gnu_ton_sheet_has_six_rows() {
        assert_eq!(GNU_TON_SHEET.rows.len(), 6);
    }

    #[test]
    fn gnu_ton_sheet_is_body_centered() {
        // body_centered:true is required so the man (at top of frame)
        // is placed at the entity transform origin rather than the
        // GNU's hooves (at the bottom of frame).
        assert!(GNU_TON_SHEET.body_centered);
    }

    #[test]
    fn gnu_ton_anchor_is_above_sprite_center() {
        // feet_anchor_y > 0 means the entity position is above the
        // sprite center — placing the man (upper frame) at entity pos.
        assert!(
            GNU_TON_SHEET.feet_anchor_y > 0.0,
            "feet_anchor_y should be positive for GNU-ton (man at top), got {}",
            GNU_TON_SHEET.feet_anchor_y
        );
        // Should not be so large that the man falls outside the frame.
        assert!(
            GNU_TON_SHEET.feet_anchor_y < 0.5,
            "feet_anchor_y too large, would place entity at sprite top edge"
        );
    }

    #[test]
    fn gnu_ton_side_sweep_resolves_to_itself() {
        assert_eq!(
            GNU_TON_SHEET.resolve_anim(BossAnim::SideSweep),
            BossAnim::SideSweep
        );
    }
}
