//! Draw the stage, including the thing that kills you.
//!
//! The drawing lives in `crate::stage_diagram`; this is the front door to it.

use clap::Args;

#[derive(Args, Debug)]
pub struct StageDiagramArgs {
    /// Where to write the PNG.
    #[arg(default_value = "/tmp/smash_stage.png")]
    pub out: String,
}

pub fn run(args: StageDiagramArgs) {
    let png = crate::stage_diagram::render_stage_diagram();
    std::fs::write(&args.out, png).expect("write the stage diagram");
    println!("[stage_diagram] wrote {}", args.out);
    println!(
        "[stage_diagram] white = world bounds, solid = platform, dashed = blast \
         envelope (past this a body is gone), dot = respawn"
    );
}
