//! Measure complete rig preparation from a published RON file.
use std::{hint::black_box, time::Instant};
use ambition_characters::actor::PublishedBodyRig;

fn main() {
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).expect("read the rig publication");
        let mut times = Vec::new();
        let mut counts = (0, 0, 0, 0);
        for _ in 0..9 {
            let start = Instant::now();
            for _ in 0..5 {
                let published = PublishedBodyRig::from_ron(black_box(&text)).unwrap();
                let rig = published.definition.prepare().unwrap();
                let bindings = published.part_bindings;
                let limbs = published.limbs;
                counts = (rig.joint_names().len(), rig.clip_names().count(), bindings.len(), limbs.len());
                black_box((&rig, &bindings, &limbs));
            }
            times.push(start.elapsed().as_secs_f64() * 1000.0 / 5.0);
        }
        times.sort_by(f64::total_cmp);
        println!("{path}: {} bytes, joints/clips/bindings/limbs {counts:?}, median {:.3} ms, range {:.3}..{:.3} ms", text.len(), times[4], times[0], times[8]);
    }
}
