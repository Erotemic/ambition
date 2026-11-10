//! The benchmark domain of RADIANT-NPC-HTN slice 1, planned for each scientist.

use ambition_htn::{Budget, Chosen, Condition, Domain, DomainError, Facts, Operand, PlanFailure, Value};

const DOMAIN: &str = include_str!("data/eureka_scientists_day.ron");

fn domain() -> Domain {
    Domain::from_ron(DOMAIN).expect("the benchmark domain is admitted")
}

/// One scientist's facts at an hour: Nikobolt keeps 8-12-18, Coulomba 10-13-20.
fn facts(who: &str, hour: i64, at: &str, bench_free: bool, ate: bool) -> Facts {
    let (arrive, lunch, leave) = if who == "nikobolt" { (8, 12, 18) } else { (10, 13, 20) };
    let mut facts = Facts::new();
    for (key, value) in [
        ("hour", Value::Int(hour)),
        ("arrive_hour", Value::Int(arrive)),
        ("lunch_hour", Value::Int(lunch)),
        ("leave_hour", Value::Int(leave)),
        ("at", Value::Sym(at.into())),
        ("bench_free", Value::Bool(bench_free)),
        ("ate", Value::Bool(ate)),
        ("talked", Value::Bool(false)),
        ("partner_here", Value::Bool(true)),
    ] {
        facts.insert(key.into(), value);
    }
    facts
}

/// The plan as `verb target` words.
fn doing(domain: &Domain, facts: &Facts) -> Vec<String> {
    let plan = domain.plan(facts, Budget::default()).expect("a plan");
    plan.steps
        .iter()
        .map(|step| format!("{} {}", step.action.verb, step.action.target.as_deref().unwrap_or("")).trim().to_string())
        .collect()
}

#[test]
fn each_scientist_follows_their_own_hours() {
    let domain = domain();
    // Nine o'clock: Nikobolt has arrived, Coulomba has not.
    assert_eq!(
        doing(&domain, &facts("nikobolt", 9, "home", true, false)),
        ["go_to eureka_coil_lab", "claim workbench", "use workbench", "release workbench", "say eureka_talk"]
    );
    assert_eq!(doing(&domain, &facts("coulomba", 9, "home", true, false)), ["rest"]);
    // Half past twelve: Nikobolt's lunch hour, not yet Coulomba's.
    assert_eq!(doing(&domain, &facts("nikobolt", 12, "lab", true, false)), ["go_to canteen", "use lunch"]);
    assert_eq!(
        doing(&domain, &facts("coulomba", 12, "lab", true, false)),
        ["claim workbench", "use workbench", "release workbench", "say eureka_talk"]
    );
    // Seven in the evening: Nikobolt walks home, Coulomba works on.
    assert_eq!(doing(&domain, &facts("nikobolt", 19, "lab", true, true)), ["go_to home", "rest"]);
    assert_eq!(doing(&domain, &facts("coulomba", 19, "lab", true, true))[0], "claim workbench");
}

#[test]
fn a_taken_bench_is_a_different_plan_that_says_why() {
    let domain = domain();
    let plan = domain.plan(&facts("coulomba", 15, "lab", false, true), Budget::default()).expect("a plan");
    let notes = plan.steps.iter().find(|step| step.task == "read_notes").expect("reads notes instead");
    assert_eq!(
        notes.chain,
        [
            Chosen { task: "live_the_day".into(), method: "a working day".into() },
            Chosen { task: "work".into(), method: "read notes while the bench is taken".into() },
        ]
    );
    let set_aside = plan
        .rejected
        .iter()
        .find(|rejection| rejection.method.as_deref() == Some("use the bench"))
        .expect("the bench method was set aside");
    assert_eq!(set_aside.failed, Condition::Eq("bench_free".into(), Operand::Is(Value::Bool(true))));
    // Control: a free bench is used, and nothing about the bench is set aside.
    let free = domain.plan(&facts("coulomba", 15, "lab", true, true), Budget::default()).expect("a plan");
    assert!(free.steps.iter().any(|step| step.task == "claim_bench"));
    assert!(free.rejected.iter().all(|rejection| rejection.task != "work"));
}

#[test]
fn the_same_facts_give_the_same_plan() {
    let domain = domain();
    let facts = facts("nikobolt", 10, "home", true, false);
    let first = domain.plan(&facts, Budget::default()).expect("a plan");
    // A domain read again from its text, and the facts rebuilt in another order.
    let mut reordered = Facts::new();
    for (key, value) in facts.iter().rev() {
        reordered.insert(key.clone(), value.clone());
    }
    assert_eq!(Domain::from_ron(DOMAIN).unwrap().plan(&reordered, Budget::default()).unwrap(), first);
    assert_eq!(first.predicted["at"], Value::Sym("lab".into()));
    assert_eq!(first.predicted["experiments"], Value::Int(1));
}

#[test]
fn a_domain_naming_an_undeclared_task_is_refused_by_name() {
    let broken = DOMAIN.replace("subtasks: [\"read_notes\"]", "subtasks: [\"read_the_notes\"]");
    assert_eq!(
        Domain::from_ron(&broken),
        Err(DomainError::UnknownSubtask(
            "work".into(),
            "read notes while the bench is taken".into(),
            "read_the_notes".into()
        ))
    );
    let rootless = DOMAIN.replace("root: \"live_the_day\"", "root: \"live_the_night\"");
    assert_eq!(Domain::from_ron(&rootless), Err(DomainError::UnknownRoot("live_the_night".into())));
}

#[test]
fn a_domain_that_recurses_forever_stops_at_its_budget() {
    let domain = Domain::from_ron(
        r#"(root: "again", compound: { "again": [(name: "once more", subtasks: ["again"])] })"#,
    )
    .unwrap();
    assert_eq!(domain.plan(&Facts::new(), Budget::default()), Err(PlanFailure::OverBudget));
}

/// Planning time per NPC, for the slice-1 comparison. Not a gate: it prints.
#[test]
#[ignore = "measurement: prints planning time per plan; run with --ignored --nocapture"]
fn how_long_a_plan_takes() {
    let domain = domain();
    let cases: Vec<Facts> = (0..24)
        .flat_map(|hour| {
            [("nikobolt", true), ("coulomba", false)].map(|(who, free)| facts(who, hour, "lab", free, hour > 14))
        })
        .collect();
    let rounds = 2000;
    let started = std::time::Instant::now();
    let mut steps = 0;
    for _ in 0..rounds {
        for facts in &cases {
            steps += domain.plan(facts, Budget::default()).expect("a plan").steps.len();
        }
    }
    let plans = rounds * cases.len();
    println!(
        "{plans} plans ({steps} steps) in {:?}: {:.2} us per plan",
        started.elapsed(),
        started.elapsed().as_secs_f64() * 1e6 / plans as f64
    );
}

#[test]
fn a_step_carried_out_makes_its_effects_facts() {
    let domain = domain();
    let mut facts = facts("nikobolt", 10, "lab", true, false);
    domain.apply_step("claim_bench", &mut facts);
    domain.apply_step("experiment", &mut facts);
    domain.apply_step("experiment", &mut facts);
    assert_eq!(facts["bench_free"], Value::Bool(false));
    assert_eq!(facts["experiments"], Value::Int(2));
    // A task with no effects, or no task of that name, changes nothing.
    let before = facts.clone();
    domain.apply_step("rest", &mut facts);
    domain.apply_step("no_such_task", &mut facts);
    assert_eq!(facts, before);
}
