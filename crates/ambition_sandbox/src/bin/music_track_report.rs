//! Diagnostic: report every music track in `sandbox.ron`.
//!
//! Walks `SandboxDataSpec::load_embedded().audio.music_tracks` and
//! prints one line per track with id, display name, BPM, total beats,
//! computed duration in seconds, and validation status. Marks the
//! `default_music_track` so an agent can verify the audio manifest at
//! a glance without opening the RON file.
//!
//! Usage:
//!     cargo run -p ambition_sandbox --bin music_track_report
//!
//! Optional `--render <out_dir>` writes a `<id>.wav` for every valid
//! track and reports the output paths. `--id <track_id>` limits the
//! render to a single track. Both render flags require the `audio`
//! feature (the binary is gated behind it in Cargo.toml).
//!
//! The report itself does not need the `audio` feature — the validate
//! / duration computations are pure data work. The binary is feature-
//! gated only because the optional WAV render reuses the audio
//! subsystem.

use ambition_sandbox::audio::{render_music_preview, wav_bytes_from_rendered_audio};
use ambition_sandbox::data::{MusicTrackSpec, SandboxDataSpec};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_SAMPLE_RATE: u32 = 44_100;

struct Args {
    render: Option<PathBuf>,
    only_id: Option<String>,
    sample_rate: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        render: None,
        only_id: None,
        sample_rate: DEFAULT_SAMPLE_RATE,
    };
    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--render" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--render requires an output dir".to_string())?;
                args.render = Some(PathBuf::from(value));
            }
            "--id" => {
                args.only_id = Some(
                    iter.next()
                        .ok_or_else(|| "--id requires a track identifier".to_string())?,
                );
            }
            "--sample-rate" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--sample-rate requires a number".to_string())?;
                args.sample_rate = value
                    .parse()
                    .map_err(|err| format!("invalid sample rate '{value}': {err}"))?;
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(args)
}

fn print_usage() {
    eprintln!(
        "usage: music_track_report [--render <dir>] [--id <track>] [--sample-rate <hz>]\n\
         \n\
         Reports every track in the embedded sandbox.ron audio manifest.\n\
         With --render, also renders each track to <dir>/<id>.wav.\n\
         With --id, limits both report and render to one track id.",
    );
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let spec = SandboxDataSpec::load_embedded();
    let audio = &spec.audio;

    let manifest_status = match audio.validate() {
        Ok(()) => "manifest validation: PASS".to_string(),
        Err(err) => format!("manifest validation: FAIL ({err})"),
    };

    println!("# music track report");
    println!("default_music_track: {}", audio.default_music_track);
    println!("track count: {}", audio.music_tracks.len());
    println!("{}", manifest_status);
    println!();

    if let Some(id) = &args.only_id {
        if audio.track(id).is_none() {
            return Err(format!(
                "no music track with id '{id}'. Known: {}",
                audio
                    .music_tracks
                    .iter()
                    .map(|t| t.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    let mut rendered_paths: Vec<(String, PathBuf)> = Vec::new();
    if let Some(out_dir) = &args.render {
        fs::create_dir_all(out_dir)
            .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;
    }

    for track in &audio.music_tracks {
        if let Some(id) = &args.only_id {
            if &track.id != id {
                continue;
            }
        }
        report_track(track, audio.default_music_track == track.id);

        if let Some(out_dir) = &args.render {
            match render_track_to_wav(track, out_dir, args.sample_rate) {
                Ok(path) => {
                    rendered_paths.push((track.id.clone(), path));
                }
                Err(err) => {
                    println!("    render: FAIL ({err})");
                }
            }
        }
    }

    if !rendered_paths.is_empty() {
        println!();
        println!("# rendered tracks");
        for (id, path) in &rendered_paths {
            println!("{id}: {}", path.display());
        }
    }
    Ok(())
}

fn report_track(track: &MusicTrackSpec, is_default: bool) {
    let arrangement = &track.arrangement;
    let duration = arrangement.duration_seconds();
    let bars = arrangement.bar_count();
    let validation = match arrangement.validate() {
        Ok(()) => "PASS".to_string(),
        Err(err) => format!("FAIL ({err})"),
    };
    let default_marker = if is_default { " [default]" } else { "" };
    println!(
        "{}{default_marker}\n  display_name: {}\n  bpm: {:.1}  total_beats: {:.1}  bars: {}  duration_s: {:.2}\n  validation: {validation}",
        track.id,
        track.display_name,
        arrangement.bpm,
        arrangement.total_beats,
        bars,
        duration,
    );
}

fn render_track_to_wav(
    track: &MusicTrackSpec,
    out_dir: &Path,
    sample_rate: u32,
) -> Result<PathBuf, String> {
    track
        .arrangement
        .validate()
        .map_err(|err| format!("invalid arrangement: {err}"))?;
    let rendered = render_music_preview(track, sample_rate);
    let bytes = wav_bytes_from_rendered_audio(&rendered);
    let path = out_dir.join(format!("{}.wav", track.id));
    fs::write(&path, bytes).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(path)
}
