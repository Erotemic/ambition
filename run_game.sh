python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload --release
