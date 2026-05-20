use ambition_engine::BossPatternSchedule;

#[test]
fn snapshot_gradient_sentinel_phase1_schedule() {
    insta::assert_snapshot!(
        BossPatternSchedule::gradient_sentinel_phase1().summary(),
        @r###"boss=gradient_sentinel phase=1 seed=2759946497 total=3.170s
00: FloorSlam telegraph=0.550 active=0.180 recover=0.620
01: SideSweep telegraph=0.420 active=0.220 recover=0.480
02: Rest telegraph=0.000 active=0.350 recover=0.350"###
    );
}

#[test]
fn snapshot_gradient_sentinel_phase2_schedule() {
    insta::assert_snapshot!(
        BossPatternSchedule::gradient_sentinel_phase2().summary(),
        @r###"boss=gradient_sentinel phase=2 seed=2759946498 total=5.390s
00: FloorSlam telegraph=0.450 active=0.180 recover=0.380
01: SpikeHalo telegraph=0.650 active=1.200 recover=0.300
02: SideSweep telegraph=0.340 active=0.200 recover=0.360
03: DashEcho telegraph=0.500 active=0.280 recover=0.550"###
    );
}
