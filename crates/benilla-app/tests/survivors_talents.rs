#[allow(dead_code)]
#[path = "../src/survivors/talent_rules.rs"]
mod talent_rules;
use talent_rules::*;

fn talent(id: u32, tree: u32, tier: u8) -> TalentDef {
    TalentDef {
        id,
        class: 9,
        tree,
        tier,
        max_rank: 5,
        prerequisite: None,
        required_spell: None,
        supported: true,
    }
}

#[test]
fn points_start_at_ten_cap_at_sixty_and_cannot_be_farmed_by_new_runs() {
    let mut p = TalentProgress::default();
    p.observe_level(9, 9).unwrap();
    assert_eq!(p.available(9), 0);
    p.observe_level(9, 10).unwrap();
    assert_eq!(p.available(9), 1);
    p.observe_level(9, 60).unwrap();
    assert_eq!(p.available(9), u32::from(MAX_POINTS));
    p.observe_level(9, 1000).unwrap();
    p.observe_level(9, 1).unwrap();
    assert_eq!((p.highest_level(9), p.available(9)), (60, 51));
    p.observe_level(8, 10).unwrap();
    assert_eq!(p.available(8), 1);
    assert_eq!(p.available(9), 51);
}

#[test]
fn rank_class_tree_and_unsupported_checks_never_partly_spend() {
    let mut p = TalentProgress::default();
    p.observe_level(9, 60).unwrap();
    let mut foreign = talent(2, 1, 0);
    foreign.class = 8;
    let mut unsupported = talent(3, 1, 0);
    unsupported.supported = false;
    let catalog = [talent(1, 1, 0), foreign, unsupported];
    for _ in 0..5 {
        p.allocate(9, 1, &catalog, &[]).unwrap();
    }
    let before = p.clone();
    assert_eq!(
        p.allocate(9, 1, &catalog, &[]),
        Err(TalentError::MaximumRank)
    );
    assert_eq!(
        p.allocate(9, 2, &catalog, &[]),
        Err(TalentError::WrongClass)
    );
    assert_eq!(
        p.allocate(9, 3, &catalog, &[]),
        Err(TalentError::Unsupported)
    );
    assert_eq!(
        p.allocate_in_tree(9, 2, 1, &catalog, &[]),
        Err(TalentError::WrongTree)
    );
    assert_eq!(
        p.allocate(9, 999, &catalog, &[]),
        Err(TalentError::UnknownTalent)
    );
    assert_eq!(p, before);
}

#[test]
fn tier_points_must_come_from_earlier_rows_in_the_same_tree() {
    let catalog = [talent(1, 1, 0), talent(2, 2, 0), talent(3, 2, 1)];
    let mut p = TalentProgress::default();
    p.observe_level(9, 60).unwrap();
    for _ in 0..5 {
        p.allocate(9, 1, &catalog, &[]).unwrap();
    }
    assert_eq!(p.allocate(9, 3, &catalog, &[]), Err(TalentError::Tier));
    for _ in 0..5 {
        p.allocate(9, 2, &catalog, &[]).unwrap();
    }
    p.allocate(9, 3, &catalog, &[]).unwrap();
    assert_eq!(p.profile(9).unwrap().tree_points(2, &catalog), 6);
}

#[test]
fn prerequisite_rank_and_required_spell_are_checked_before_spending() {
    let mut child = talent(2, 1, 0);
    child.prerequisite = Some(Prerequisite { id: 1, rank: 3 });
    child.required_spell = Some(686);
    let catalog = [talent(1, 1, 0), child];
    let mut p = TalentProgress::default();
    p.observe_level(9, 60).unwrap();
    assert_eq!(
        p.allocate(9, 2, &catalog, &[]),
        Err(TalentError::RequiredSpell)
    );
    assert_eq!(
        p.allocate(9, 2, &catalog, &[686]),
        Err(TalentError::Prerequisite)
    );
    for _ in 0..3 {
        p.allocate(9, 1, &catalog, &[686]).unwrap();
    }
    p.allocate(9, 2, &catalog, &[686]).unwrap();
    assert_eq!(p.rank(9, 2), 1);
}

#[test]
fn exhausted_points_refuse_allocation_and_remain_spent_after_replay() {
    let catalog = [talent(1, 1, 0)];
    let mut p = TalentProgress::default();
    p.observe_level(9, 10).unwrap();
    p.allocate(9, 1, &catalog, &[]).unwrap();
    p.observe_level(9, 1).unwrap();
    p.observe_level(9, 10).unwrap();
    assert_eq!(p.available(9), 0);
    assert_eq!(p.allocate(9, 1, &catalog, &[]), Err(TalentError::NoPoints));
    p.observe_level(9, 11).unwrap();
    assert_eq!(p.available(9), 1);
}

#[test]
fn malformed_saved_profiles_and_unknown_or_cyclic_builds_are_rejected_immutably() {
    let mut p = TalentProgress::default();
    p.observe_level(9, 60).unwrap();
    p.profiles[0].ranks = vec![TalentRank { id: 1, rank: 1 }, TalentRank { id: 2, rank: 1 }];
    let before = p.clone();
    assert_eq!(
        p.validate_catalog(9, &[talent(1, 1, 0)], &[]),
        Err(TalentError::UnknownTalent)
    );
    let mut a = talent(1, 1, 0);
    a.prerequisite = Some(Prerequisite { id: 2, rank: 1 });
    let mut b = talent(2, 1, 0);
    b.prerequisite = Some(Prerequisite { id: 1, rank: 1 });
    assert_eq!(
        p.validate_catalog(9, &[a, b], &[]),
        Err(TalentError::InvalidDefinition)
    );
    assert_eq!(p, before);
    p.profiles.push(p.profiles[0].clone());
    assert_eq!(p.validate_basic(), Err(TalentError::DuplicateProfile));
}

#[test]
fn serde_roundtrip_keeps_each_class_and_talent_id_without_numeric_toml_keys() {
    let mut p = TalentProgress::default();
    p.observe_level(9, 14).unwrap();
    p.observe_level(8, 27).unwrap();
    p.allocate(9, 1, &[talent(1, 1, 0)], &[]).unwrap();
    let text = toml::to_string(&p).unwrap();
    let decoded: TalentProgress = toml::from_str(&text).unwrap();
    assert_eq!(decoded, p);
    assert_eq!(p.observe_level(0, 10), Err(TalentError::InvalidClass));
    assert_eq!(p.observe_level(9, 0), Err(TalentError::InvalidLevel));
}

#[test]
fn failures_have_clear_french_player_messages() {
    assert!(TalentError::NoPoints
        .to_string()
        .contains("point de talent"));
    assert!(TalentError::Unsupported.to_string().contains("Survivors"));
    assert!(TalentError::WrongClass.to_string().contains("autre classe"));
}
