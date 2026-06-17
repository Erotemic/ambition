PYTHONPATH="$(dirname "$0")/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
RUST_BACKTRACE=1 cargo run -p ambition_app --bin headless --release

