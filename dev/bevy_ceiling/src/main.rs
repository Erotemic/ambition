//! The empty-window ceiling. See README.md.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{PresentMode, WindowResolution};

struct Args {
    present_mode: PresentMode,
    sprites: usize,
    overlay: bool,
    width: u32,
    height: u32,
    seconds: Option<f64>,
}

fn parse_args() -> Args {
    let mut args = Args {
        present_mode: PresentMode::Fifo,
        sprites: 0,
        overlay: true,
        width: 1600,
        height: 900,
        seconds: None,
    };
    let mut it = std::env::args().skip(1);
    let value = |it: &mut std::iter::Skip<std::env::Args>, flag: &str| -> String {
        it.next().unwrap_or_else(|| {
            eprintln!("{flag} needs a value");
            std::process::exit(2)
        })
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--no-vsync" => args.present_mode = PresentMode::Immediate,
            "--present-mode" => {
                args.present_mode = match value(&mut it, "--present-mode").as_str() {
                    "fifo" => PresentMode::Fifo,
                    "fifo-relaxed" => PresentMode::FifoRelaxed,
                    "mailbox" => PresentMode::Mailbox,
                    "immediate" => PresentMode::Immediate,
                    "auto-vsync" => PresentMode::AutoVsync,
                    "auto-no-vsync" => PresentMode::AutoNoVsync,
                    other => {
                        eprintln!("unknown present mode {other}");
                        std::process::exit(2)
                    }
                }
            }
            "--sprites" => args.sprites = value(&mut it, "--sprites").parse().expect("--sprites N"),
            "--no-overlay" => args.overlay = false,
            "--width" => args.width = value(&mut it, "--width").parse().expect("--width W"),
            "--height" => args.height = value(&mut it, "--height").parse().expect("--height H"),
            "--seconds" => args.seconds = Some(value(&mut it, "--seconds").parse().expect("--seconds S")),
            "-h" | "--help" => {
                println!(
                    "bevy_ceiling [--no-vsync | --present-mode MODE] [--sprites N] [--no-overlay] \
                     [--width W] [--height H] [--seconds S]\n\
                     MODE: fifo (default, what the game gets) | fifo-relaxed | mailbox | immediate | auto-vsync | auto-no-vsync"
                );
                std::process::exit(0)
            }
            other => {
                eprintln!("unknown argument {other} (try --help)");
                std::process::exit(2)
            }
        }
    }
    args
}

/// Frame times since the last report, plus the whole run, so the stdout log
/// carries both the two-second window and the number a run is remembered by.
#[derive(Resource)]
struct FrameLog {
    started: Instant,
    last_report: Instant,
    window: VecDeque<f64>,
    run: Vec<f64>,
    /// The first second is window creation and swapchain warm-up; it is not
    /// the ceiling, so it does not enter `run`.
    warmup_until: Instant,
    exit_at: Option<Instant>,
    label: String,
}

#[derive(Component)]
struct Drifter(Vec2);

fn main() {
    let args = parse_args();
    let label = format!(
        "present={:?} sprites={} overlay={} {}x{}",
        args.present_mode, args.sprites, args.overlay, args.width, args.height
    );
    let now = Instant::now();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("bevy_ceiling — {label}"),
            resolution: WindowResolution::new(args.width, args.height),
            present_mode: args.present_mode,
            ..default()
        }),
        ..default()
    }));
    if args.overlay {
        app.add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont { font_size: FontSize::Px(28.0), ..default() },
                ..default()
            },
        });
    }
    app.insert_resource(FrameLog {
        started: now,
        last_report: now,
        window: VecDeque::new(),
        run: Vec::new(),
        warmup_until: now + Duration::from_secs(1),
        exit_at: args.seconds.map(|s| now + Duration::from_secs_f64(s + 1.0)),
        label,
    })
    .insert_resource(SpriteCount(args.sprites))
    .add_systems(Startup, setup)
    .add_systems(Update, (drift, log_frames));
    app.run();
}

#[derive(Resource)]
struct SpriteCount(usize);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, count: Res<SpriteCount>, window: Query<&Window>) {
    commands.spawn(Camera2d);
    if count.0 == 0 {
        return;
    }
    // One 64x64 checker texture shared by every sprite: the cost measured is
    // the sprite pipeline's, not texture uploads.
    let size = 64u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let on = ((x / 8) + (y / 8)) % 2 == 0;
            data.extend_from_slice(if on { &[230, 120, 40, 255] } else { &[40, 120, 230, 255] });
        }
    }
    let image = images.add(Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let (w, h) = window
        .single()
        .map(|win| (win.width(), win.height()))
        .unwrap_or((1600.0, 900.0));
    // A fixed-seed LCG so two runs place the same sprites.
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f32) / ((1u64 << 31) as f32)
    };
    for _ in 0..count.0 {
        let x = (next() - 0.5) * w;
        let y = (next() - 0.5) * h;
        let vx = (next() - 0.5) * 200.0;
        let vy = (next() - 0.5) * 200.0;
        commands.spawn((
            Sprite::from_image(image.clone()),
            Transform::from_xyz(x, y, 0.0),
            Drifter(Vec2::new(vx, vy)),
        ));
    }
}

/// Sprites move so the frame is never a cached one; they wrap at the edges.
fn drift(time: Res<Time>, window: Query<&Window>, mut q: Query<(&mut Transform, &Drifter)>) {
    let (w, h) = window
        .single()
        .map(|win| (win.width(), win.height()))
        .unwrap_or((1600.0, 900.0));
    let dt = time.delta_secs();
    for (mut t, d) in &mut q {
        t.translation.x += d.0.x * dt;
        t.translation.y += d.0.y * dt;
        if t.translation.x > w / 2.0 { t.translation.x -= w; }
        if t.translation.x < -w / 2.0 { t.translation.x += w; }
        if t.translation.y > h / 2.0 { t.translation.y -= h; }
        if t.translation.y < -h / 2.0 { t.translation.y += h; }
    }
}

fn stats(samples: &mut [f64]) -> Option<(f64, f64, f64, f64, f64, f64)> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = samples.len();
    let pct = |p: f64| samples[((n as f64 - 1.0) * p).round() as usize];
    let mean = samples.iter().sum::<f64>() / n as f64;
    Some((mean, pct(0.5), pct(0.95), pct(0.99), samples[0], samples[n - 1]))
}

fn log_frames(time: Res<Time>, mut log: ResMut<FrameLog>, mut exit: MessageWriter<AppExit>) {
    let now = Instant::now();
    let ms = time.delta_secs_f64() * 1000.0;
    if now >= log.warmup_until && ms > 0.0 {
        log.window.push_back(ms);
        log.run.push(ms);
    }
    if now.duration_since(log.last_report) >= Duration::from_secs(2) {
        log.last_report = now;
        let mut samples: Vec<f64> = log.window.drain(..).collect();
        if let Some((mean, p50, p95, p99, min, max)) = stats(&mut samples) {
            println!(
                "[ceiling] t={:6.1}s frames={:5} fps={:7.1} frame_ms mean={:.3} p50={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3}",
                now.duration_since(log.started).as_secs_f64(),
                samples.len(),
                1000.0 / mean,
                mean, p50, p95, p99, min, max
            );
        }
    }
    if log.exit_at.is_some_and(|at| now >= at) {
        let mut samples = std::mem::take(&mut log.run);
        if let Some((mean, p50, p95, p99, min, max)) = stats(&mut samples) {
            println!(
                "[ceiling] SUMMARY {} frames={} fps={:.1} frame_ms mean={:.3} p50={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3}",
                log.label, samples.len(), 1000.0 / mean, mean, p50, p95, p99, min, max
            );
        }
        exit.write(AppExit::Success);
    }
}
