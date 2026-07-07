PYTHONPATH="$(dirname "$0")/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate game/ambition_content/assets/worlds/sandbox.ldtk
RUST_BACKTRACE=1 cargo run -p ambition_app --bin headless --release

