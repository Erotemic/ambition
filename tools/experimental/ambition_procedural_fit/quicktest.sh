export CUDA_VISIBLE_DEVICES=1

python -m ambition_procedural_fit init-template \
    --target ./test-img.png \
    --out ./generated/robot_sprite_seed_v3.yaml \
    --size 512 \
    --rects 48 \
    --ellipses 16 \
    --segments 32

python -m ambition_procedural_fit fit \
    --target ./test-img.png \
    --template ./generated/robot_sprite_seed_v3.yaml \
    --out-dir ./generated/robot_sprite_fit_v3 \
    --steps 1600 \
    --lr 0.01 \
    --size 512 \
    --restarts 4 \
    --mode sprite \
    --sharpness-start 32 \
    --sharpness-end 260 \
    --save-debug \
    --debug-every 40 \
    --debug-max-frames 24

# ----
#

export CUDA_VISIBLE_DEVICES=1

python -m ambition_procedural_fit init-template \
    --target ./background-layer-image.png \
    --out ./generated/background_layer_seed_v3.yaml \
    --size 256 \
    --rects 64 \
    --ellipses 24 \
    --segments 32

python -m ambition_procedural_fit fit \
    --target ./background-layer-image.png \
    --template ./generated/background_layer_seed_v3.yaml \
    --out-dir ./generated/background_layer_fit_v3 \
    --steps 800 \
    --lr 0.03 \
    --size 256 \
    --restarts 3 \
    --mode background \
    --sharpness-start 48 \
    --sharpness-end 120 \
    --save-debug \
    --debug-every 50 \
    --debug-max-frames 24
