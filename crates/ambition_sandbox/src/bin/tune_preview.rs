use ambition_sandbox::audio::{render_music_preview, wav_bytes_from_rendered_audio};
use ambition_sandbox::data::{MusicSpec, MusicTrackSpec};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const DEFAULT_SAMPLE_RATE: u32 = 44_100;

struct Args {
    track: PathBuf,
    out: PathBuf,
    sample_rate: u32,
}

enum TuneInput {
    Track(MusicTrackSpec),
    Arrangement(MusicSpec),
}

impl TuneInput {
    fn arrangement(&self) -> &MusicSpec {
        match self {
            Self::Track(track) => &track.arrangement,
            Self::Arrangement(spec) => spec,
        }
    }

    fn id(&self) -> Option<&str> {
        match self {
            Self::Track(track) => Some(&track.id),
            Self::Arrangement(_) => None,
        }
    }

    fn display_name(&self) -> Option<&str> {
        match self {
            Self::Track(track) => Some(&track.display_name),
            Self::Arrangement(_) => None,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let ron_text = fs::read_to_string(&args.track)
        .map_err(|error| format!("failed to read {}: {error}", args.track.display()))?;
    let tune = parse_tune(&ron_text)?;
    tune.arrangement()
        .validate()
        .map_err(|error| format!("invalid tune arrangement: {error}"))?;

    let rendered = match &tune {
        TuneInput::Track(track) => render_music_preview(track, args.sample_rate),
        TuneInput::Arrangement(spec) => {
            let track = MusicTrackSpec {
                id: "preview".to_string(),
                display_name: "Preview".to_string(),
                arrangement: spec.clone(),
            };
            render_music_preview(&track, args.sample_rate)
        }
    };
    let wav = wav_bytes_from_rendered_audio(&rendered);
    fs::write(&args.out, wav)
        .map_err(|error| format!("failed to write {}: {error}", args.out.display()))?;

    if let Some(id) = tune.id() {
        println!("track id: {id}");
    }
    if let Some(display_name) = tune.display_name() {
        println!("display name: {display_name}");
    }
    println!("duration: {:.2}s", rendered.duration_seconds());
    println!("sample rate: {}", rendered.sample_rate);
    println!("output: {}", args.out.display());
    Ok(())
}

fn parse_tune(text: &str) -> Result<TuneInput, String> {
    match ron::from_str::<MusicTrackSpec>(text) {
        Ok(track) => Ok(TuneInput::Track(track)),
        Err(track_error) => match ron::from_str::<MusicSpec>(text) {
            Ok(spec) => Ok(TuneInput::Arrangement(spec)),
            Err(spec_error) => Err(format!(
                "failed to parse tune as MusicTrackSpec ({track_error}) or MusicSpec ({spec_error})"
            )),
        },
    }
}

fn parse_args() -> Result<Args, String> {
    let mut track = None;
    let mut out = None;
    let mut sample_rate = DEFAULT_SAMPLE_RATE;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--track" => {
                track = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--track requires a path".to_string())?,
                ));
            }
            "--out" => {
                out = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--out requires a path".to_string())?,
                ));
            }
            "--sample-rate" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--sample-rate requires a number".to_string())?;
                sample_rate = value
                    .parse::<u32>()
                    .map_err(|error| format!("invalid --sample-rate '{value}': {error}"))?;
            }
            "-h" | "--help" => return Err("usage requested".to_string()),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    Ok(Args {
        track: track.ok_or_else(|| "missing --track PATH".to_string())?,
        out: out.ok_or_else(|| "missing --out PATH".to_string())?,
        sample_rate,
    })
}

fn print_usage() {
    eprintln!(
        "usage: cargo run -p ambition_sandbox --bin tune_preview -- \\
    --track crates/ambition_sandbox/assets/ambition/tune_examples/example_drift.ron \\
    --out /tmp/ambition_tune_preview.wav \\
    [--sample-rate 44100]"
    );
}
