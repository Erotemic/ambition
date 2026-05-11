use ambition_engine as ae;

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<ae::BossEncounterSpec> {
    vec![
        ae::BossEncounterSpec::gradient_sentinel(),
        ae::BossEncounterSpec::mockingbird(),
    ]
}
