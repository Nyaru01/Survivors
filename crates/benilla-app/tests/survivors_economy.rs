#[path = "../src/survivors/economy_rules.rs"]
#[allow(dead_code)]
mod rules;

#[test]
fn purchases_are_atomic_and_stock_limited_even_after_drinking() {
    let mut a = rules::Account {
        gold: 100,
        potions: 0,
        gear: [0; 4],
        ..Default::default()
    };
    let mut stock = 3;
    for _ in 0..3 {
        a.buy_potion(&mut stock).unwrap();
    }
    assert_eq!((a.gold, a.potions, stock), (40, 3, 0));
    let before = a.clone();
    assert!(a.buy_potion(&mut stock).is_err());
    assert_eq!(a, before);
    a.drink(10.0, 100.0, &mut 0.0).unwrap();
    assert!(a.buy_potion(&mut stock).is_err());
}
#[test]
fn insufficient_funds_never_take_stock_or_gear() {
    let mut a = rules::Account::default();
    let mut stock = 3;
    let mut opened = 0;
    assert!(a.buy_potion(&mut stock).is_err());
    assert!(a.open_chest(&mut opened, 0).is_err());
    assert_eq!((a, stock, opened), (rules::Account::default(), 3, 0));
}
#[test]
fn chests_upgrade_available_slots_and_charge_increasing_price() {
    let mut a = rules::Account {
        gold: 1000,
        potions: 0,
        gear: [8, 0, 0, 0],
        ..Default::default()
    };
    let mut opened = 0;
    assert_eq!(a.open_chest(&mut opened, 0).unwrap(), 1);
    assert_eq!((a.gear, a.gold, opened), ([8, 1, 0, 0], 960, 1));
    a.open_chest(&mut opened, 0).unwrap();
    assert_eq!(a.gold, 900);
    a.gear = [8; 4];
    let before = a.clone();
    assert!(a.open_chest(&mut opened, 0).is_err());
    assert_eq!(a, before);
}
#[test]
fn potion_respects_max_hp_shared_cooldown_and_death() {
    let mut a = rules::Account {
        potions: 3,
        ..Default::default()
    };
    let mut cd = 0.0;
    assert_eq!(a.drink(70.0, 100.0, &mut cd).unwrap(), 100.0);
    assert_eq!(cd, 20.0);
    assert!(a.drink(10.0, 100.0, &mut cd).is_err());
    assert_eq!(a.potions, 2);
    cd = 0.0;
    assert!(a.drink(100.0, 100.0, &mut cd).is_err());
    assert!(a.drink(0.0, 100.0, &mut cd).is_err());
    assert_eq!(a.potions, 2);
}
#[test]
fn classes_receive_appropriate_weapon_and_armour_names() {
    assert_eq!(rules::gear_name(9, 0), "Bâton");
    assert_eq!(rules::gear_name(9, 1), "Robe de tissu");
    assert_eq!(rules::gear_name(2, 0), "Épée");
    assert_eq!(rules::gear_name(2, 1), "Armure de plaques");
    assert_eq!(rules::gear_name(3, 0), "Arc");
    assert_eq!(rules::gear_name(4, 0), "Dagues");
}

#[test]
fn unequipping_keeps_owned_tiers_gold_and_potions_and_only_removes_effective_stats() {
    let mut account = rules::Account {
        gold: 200,
        potions: 2,
        gear: [2, 3, 1, 4],
        ..Default::default()
    };
    assert_eq!(account.gear_effective(), [2, 3, 1, 4]);
    assert_eq!(account.toggle_equipped(1), Ok(false));
    assert_eq!(account.gear_effective(), [2, 0, 1, 4]);
    assert_eq!(
        (account.gear, account.gold, account.potions),
        ([2, 3, 1, 4], 200, 2)
    );
    account.set_equipped(1, true).unwrap();
    assert_eq!(account.gear_effective(), [2, 3, 1, 4]);
}

#[test]
fn empty_and_invalid_equipment_intents_are_atomic_and_upgrades_preserve_unequipped_choice() {
    let mut account = rules::Account {
        gold: 100,
        gear: [2, 0, 0, 0],
        equipped: [false, true, true, true],
        ..Default::default()
    };
    let before = account.clone();
    assert!(account.toggle_equipped(1).is_err());
    assert!(account.set_equipped(10, true).is_err());
    assert_eq!(account, before);
    assert_eq!(account.open_chest(&mut 0, 0), Ok(0));
    assert_eq!(account.gear[0], 3);
    assert!(!account.equipped[0]);
    assert_eq!(account.gear_effective(), [0; 4]);
}

#[test]
fn boss_items_persist_in_inventory_and_worn_item_syncs_legacy_gear() {
    let mut account = rules::Account::default();
    let a = account.grant_boss_item(0, 1, 2).unwrap();
    let b = account.grant_boss_item(0, 2, 4).unwrap();
    assert_ne!(a, b);
    assert_eq!(account.inventory.len(), 2);
    assert_eq!(account.equip_item(a), Ok(None));
    assert_eq!(account.gear_effective(), [2, 0, 0, 0]);
    assert_eq!(account.equip_item(b), Ok(Some(a)));
    assert_eq!(account.worn[0], Some(b));
    assert_eq!(account.gear_effective(), [4, 0, 0, 0]);
    assert_eq!(account.remove_worn(0), Ok(Some(b)));
    assert_eq!(account.gear_effective(), [0; 4]);
    assert_eq!(account.inventory.len(), 2);
}

#[test]
fn fusion_consumes_three_identical_white_templates_and_advances_tier_and_equipped_power() {
    assert_eq!(rules::fusion_result_tier(2), Some(3));
    assert_eq!(
        rules::fusion_result_tier(rules::MAX_TIER),
        Some(rules::MAX_TIER)
    );
    assert_eq!(rules::fusion_result_tier(rules::MAX_TIER + 1), None);
    let mut account = rules::Account::default();
    let ids = [
        account.grant_boss_item(2, 0, 2).unwrap(),
        account.grant_boss_item(2, 0, 2).unwrap(),
        account.grant_boss_item(2, 0, 2).unwrap(),
    ];
    assert_ne!(ids[0], ids[1]);
    assert_eq!(
        rules::item_template_id(2, account.inventory[0].quality, account.inventory[0].tier),
        rules::item_template_id(2, account.inventory[1].quality, account.inventory[1].tier),
    );
    assert_ne!(
        rules::item_template_id(2, 0, 2),
        rules::item_template_id(2, 0, 3)
    );
    assert_ne!(
        rules::item_template_id(2, 0, 2),
        rules::item_template_id(1, 0, 2)
    );
    let result = account.fuse_items(ids).unwrap();
    assert_eq!(
        account.inventory,
        vec![rules::OwnedItem {
            id: result,
            slot: 2,
            quality: 1,
            tier: 3
        }]
    );
    account.equip_item(result).unwrap();
    assert_eq!(account.gear_effective()[2], 3);
    assert_eq!(rules::gear_bonus(2, 3), "Armure +6");
}

#[test]
fn fusion_candidates_prefer_a_complete_group_then_largest_partial_group() {
    let mut account = rules::Account::default();
    let partial = [
        account.grant_boss_item(0, 0, 2).unwrap(),
        account.grant_boss_item(0, 0, 2).unwrap(),
    ];
    let complete = [
        account.grant_boss_item(0, 0, 3).unwrap(),
        account.grant_boss_item(0, 0, 3).unwrap(),
        account.grant_boss_item(0, 0, 3).unwrap(),
    ];
    account.grant_boss_item(0, 1, 2).unwrap();
    assert_eq!(account.fusion_candidates(0), complete);
    assert_eq!(account.fusion_ingredients(0), Some(complete));
    account.equip_item(complete[0]).unwrap();
    assert_eq!(account.fusion_ingredients(0), None);
    assert_eq!(account.fusion_candidates(0), partial.to_vec());
    assert!(account.fusion_candidates(4).is_empty());
}

#[test]
fn failed_fusion_is_atomic_for_missing_equipped_incompatible_and_capped_items() {
    let mut account = rules::Account::default();
    let one = account.grant_boss_item(0, 1, 1).unwrap();
    let two = account.grant_boss_item(0, 1, 1).unwrap();
    let three = account.grant_boss_item(1, 1, 1).unwrap();
    let before = account.clone();
    assert!(account.fuse_items([one, two, three]).is_err());
    assert_eq!(account, before);
    account.equip_item(one).unwrap();
    let before = account.clone();
    assert!(account.fuse_items([one, two, two]).is_err());
    assert_eq!(account, before);
    account.remove_worn(0).unwrap();
    let capped = account.grant_boss_item(0, rules::MAX_QUALITY, 4).unwrap();
    let before = account.clone();
    assert!(account.fuse_items([capped, two, two]).is_err());
    assert_eq!(account, before);
}

#[test]
fn fusion_rejects_different_tiers_and_nonwhite_items_without_mutation() {
    let mut account = rules::Account::default();
    let white = [
        account.grant_boss_item(0, 0, 1).unwrap(),
        account.grant_boss_item(0, 0, 1).unwrap(),
        account.grant_boss_item(0, 0, 2).unwrap(),
    ];
    let before = account.clone();
    assert!(account.fuse_items(white).is_err());
    assert_eq!(account, before);
    let green = account.grant_boss_item(0, 1, 1).unwrap();
    let same_white = [white[0], white[1], green];
    let before = account.clone();
    assert!(account.fuse_items(same_white).is_err());
    assert_eq!(account, before);
}

#[test]
fn repeated_fusion_request_cannot_duplicate_a_result() {
    let mut account = rules::Account::default();
    let ids = [
        account.grant_boss_item(1, 0, 3).unwrap(),
        account.grant_boss_item(1, 0, 3).unwrap(),
        account.grant_boss_item(1, 0, 3).unwrap(),
    ];
    let result = account.fuse_items(ids).unwrap();
    let after_success = account.clone();
    assert!(account.fuse_items(ids).is_err());
    assert_eq!(account, after_success);
    assert_eq!(
        account
            .inventory
            .iter()
            .filter(|item| item.id == result)
            .count(),
        1
    );
}

#[test]
fn fusion_at_maximum_tier_keeps_the_tier_capped_while_turning_green() {
    let mut account = rules::Account::default();
    let ids = [
        account.grant_boss_item(3, 0, rules::MAX_TIER).unwrap(),
        account.grant_boss_item(3, 0, rules::MAX_TIER).unwrap(),
        account.grant_boss_item(3, 0, rules::MAX_TIER).unwrap(),
    ];
    let result = account.fuse_items(ids).unwrap();
    let item = account
        .inventory
        .iter()
        .find(|item| item.id == result)
        .unwrap();
    assert_eq!((item.quality, item.tier), (1, rules::MAX_TIER));
}

#[test]
fn world_chests_only_upgrade_run_temporary_gear_atomically() {
    let mut account = rules::Account {
        gold: 100,
        gear: [3, 2, 1, 4],
        ..Default::default()
    };
    let id = account.grant_boss_item(0, 1, 5).unwrap();
    let mut temporary = [0, 8, 1, 0];
    let mut opened = 0;
    let before = account.clone();
    assert!(account
        .open_world_chest(&mut temporary, &mut opened, 0)
        .is_ok());
    assert_eq!(temporary, [1, 8, 1, 0]);
    assert_eq!(account.inventory, before.inventory);
    assert_eq!(account.gear, before.gear);
    assert_eq!(account.inventory[0].id, id);
    assert_eq!((account.gold, opened), (60, 1));
    temporary = [8; 4];
    let snapshot = (account.clone(), temporary, opened);
    assert!(account
        .open_world_chest(&mut temporary, &mut opened, 0)
        .is_err());
    assert_eq!((account, temporary, opened), snapshot);
}

#[test]
fn new_run_temporary_gear_starts_empty_without_changing_permanent_inventory() {
    let mut account = rules::Account::default();
    let item = account.grant_boss_item(1, 2, 4).unwrap();
    let next_run_temporary = [0; 4];
    assert_eq!(next_run_temporary, [0; 4]);
    assert_eq!(account.worn, [None; 4]);
    assert_eq!(account.inventory[0].id, item);
}
