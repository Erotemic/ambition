//! Draw the stage, including the thing that kills you.
//!
//! `cargo run -p ambition_demo_smash_app --bin stage_diagram -- [OUT.png]`
//!
//! The drawing lives in `ambition_demo_smash_app::stage_diagram`; this is the
//! front door to it.

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/smash_stage.png".to_string());
    let png = ambition_demo_smash_app::stage_diagram::render_stage_diagram();
    std::fs::write(&out, png).expect("write the stage diagram");
    println!("[stage_diagram] wrote {out}");
    println!(
        "[stage_diagram] white = world bounds, solid = platform, dashed = blast \
         envelope (past this a body is gone), dot = respawn"
    );
}
