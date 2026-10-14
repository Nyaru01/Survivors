#[path = "../src/survivors/xp_rules.rs"]
#[allow(dead_code)]
mod rules;

#[test]
fn experience_thresholds_grow_smoothly_without_milestone_jumps() {
    let thresholds: Vec<_> = (1..=100).map(rules::xp_to_next).collect();
    assert!(thresholds.windows(2).all(|pair| pair[1] > pair[0]));
    assert!(thresholds.windows(3).all(|triple| {
        (triple[2] - triple[1] - (triple[1] - triple[0])).abs() < f32::EPSILON
    }));
}

#[test]
fn starter_levels_fit_the_target_pacing_at_observed_kill_cadence() {
    // Lv5 arrived in 44 s with the former 5, 15, 25, 35 thresholds and XP_RATE=2,
    // an observed gain rate of about 1.82 threshold-points per second.
    let xp_per_second = (5.0 + 15.0 + 25.0 + 35.0) / 44.0;
    let first_level_seconds = rules::xp_to_next(1) / xp_per_second;
    let level_five_seconds: f32 = (1..5).map(rules::xp_to_next).sum::<f32>() / xp_per_second;
    assert!((30.0..=45.0).contains(&first_level_seconds));
    assert!((180.0..=240.0).contains(&level_five_seconds));
}
