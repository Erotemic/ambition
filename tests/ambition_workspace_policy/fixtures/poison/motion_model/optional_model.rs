//! Poison fixture: the shapes the movement-model guard must catch.

fn optional_query(q: Query<Option<&MotionModel>, Without<MotionModel>>) {
    let _ = q;
    let _: Option<&mut MotionModel> = None;
}

fn crawler_flag_dispatch(tuning: &ActorTuning) -> bool {
    tuning.surface_walker
}
