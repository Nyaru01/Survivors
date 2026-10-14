#[path = "../src/survivors/progression_world.rs"]
#[allow(dead_code)]
mod rules;

#[test]
fn racial_homes_and_early_worlds_are_exact_and_do_not_include_other_peoples_starter_zones() {
    for (race, home, next) in [
        (1, 5, 11),
        (2, 6, 0),
        (8, 6, 0),
        (3, 8, 11),
        (7, 8, 11),
        (4, 7, 0),
        (5, 10, 0),
        (6, 9, 0),
    ] {
        let route = rules::route(race);
        assert_eq!(&route[..2], &[home, next]);
        let mut distinct = route.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), route.len());
        assert!(route
            .iter()
            .filter(|&&map| (5..=10).contains(&map))
            .all(|&map| map == home));
    }
}
#[test]
fn four_boss_arrivals_are_independent_of_alive_bosses_and_time_never_wins() {
    assert_eq!(rules::due_bosses(179.9), 0);
    for (seconds, due) in [(180.0, 1), (360.0, 2), (540.0, 3), (720.0, 4), (10000.0, 4)] {
        assert_eq!(rules::due_bosses(seconds), due);
    }
    assert!(!rules::victory(3, true));
    assert!(!rules::victory(4, false));
    assert!(rules::victory(4, true));
}
#[test]
fn completing_one_world_unlocks_only_its_successor_and_failures_preserve_progress() {
    let mut progress = rules::WorldProgress::default();
    assert!(progress.available(1, 5));
    assert!(!progress.available(1, 11));
    assert_eq!(
        progress.complete(1, 5, 3, true),
        Err(rules::WorldError::FourBossesRequired)
    );
    assert!(progress.profiles.is_empty());
    assert_eq!(
        progress.complete(1, 5, 4, false),
        Err(rules::WorldError::HeroDead)
    );
    assert_eq!(progress.complete(1, 5, 4, true), Ok(Some(11)));
    assert!(progress.available(1, 11));
    assert!(!progress.available(1, 0));
    assert!(!progress.available(2, 11));
    assert_eq!(progress.complete(1, 5, 4, true), Ok(Some(11)));
    assert_eq!(progress.profile(1).unwrap().completed, vec![5]);
}
#[test]
fn explicit_legacy_access_preserves_old_zone_without_unlocking_future_or_other_peoples_routes() {
    let mut progress = rules::WorldProgress::default();
    progress.preserve_legacy(1, 7).unwrap();
    assert!(progress.available(1, 7));
    assert_eq!(progress.world_number(1, 7), None);
    assert_eq!(progress.next_world(1, 7), None);
    assert!(!progress.available(1, 11));
    assert!(!progress.available(2, 7));
    assert!(!progress.available(0, 0));
    assert!(!progress.available(1, 16));
}
