//! The save codec can be checked without loading MPQ data or the renderer.
#[allow(dead_code)]
#[path = "../src/survivors/data.rs"]
mod data;
#[allow(dead_code)]
#[path = "../src/survivors/economy_rules.rs"]
mod economy_rules;
#[allow(dead_code)]
#[path = "../src/survivors/encounter_clock.rs"]
mod encounter_clock;
#[allow(dead_code)]
#[path = "../src/survivors/meta_rules.rs"]
mod meta_rules;
#[allow(dead_code)]
#[path = "../src/survivors/progression_world.rs"]
mod progression_world;
#[allow(dead_code)]
#[path = "../src/survivors/relic_rules.rs"]
mod relic_rules;
#[allow(dead_code)]
#[path = "../src/survivors/save_format.rs"]
mod save_format;
#[allow(dead_code)]
#[path = "../src/survivors/talent_rules.rs"]
mod talent_rules;
#[allow(dead_code)]
#[path = "../src/survivors/xp_rules.rs"]
mod xp_rules;

#[test]
fn legacy_tiers_migrate_to_permanent_owned_items_with_worn_state() {
    let save = save_format::Save {
        meta_progress: Default::default(),
        version: 1,
        character: save_format::Character {
            name: "Élodie".into(),
            class: 0,
            race: 1,
            sex: 0,
            map: 0,
            appearance: [0; 5],
        },
        gold: 0,
        potions: 0,
        gear: [3, 0, 1, 0],
        equipped: [true, true, false, true],
        inventory: vec![],
        worn: [None; 4],
        next_item_id: 1,
        worlds: progression_world::WorldProgress::default(),
        talents: talent_rules::TalentProgress::default(),
        run: None,
    };
    let migrated = save_format::decode(&toml::to_string(&save).unwrap()).unwrap();
    assert_eq!(migrated.inventory.len(), 2);
    assert_eq!(migrated.inventory[0].slot, 0);
    assert_eq!(migrated.inventory[0].tier, 3);
    assert_eq!(migrated.worn[0], Some(migrated.inventory[0].id));
    assert_eq!(migrated.worn[2], None);
    assert_eq!(migrated.next_item_id, 3);
}

#[test]
fn invalid_or_duplicate_worn_inventory_references_are_rejected() {
    let save = save_format::Save {
        meta_progress: Default::default(),
        version: 1,
        character: save_format::Character {
            name: "Élodie".into(),
            class: 0,
            race: 1,
            sex: 0,
            map: 0,
            appearance: [0; 5],
        },
        gold: 0,
        potions: 0,
        gear: [0; 4],
        equipped: [true; 4],
        inventory: vec![economy_rules::OwnedItem {
            id: 7,
            slot: 0,
            quality: 1,
            tier: 2,
        }],
        worn: [Some(7), Some(7), None, None],
        next_item_id: 8,
        worlds: progression_world::WorldProgress::default(),
        talents: talent_rules::TalentProgress::default(),
        run: None,
    };
    assert!(save_format::decode(&toml::to_string(&save).unwrap()).is_err());
}

#[test]
fn inventory_limit_is_enforced() {
    let save = save_format::Save {
        meta_progress: Default::default(),
        version: 1,
        character: save_format::Character {
            name: "Élodie".into(),
            class: 0,
            race: 1,
            sex: 0,
            map: 0,
            appearance: [0; 5],
        },
        gold: 0,
        potions: 0,
        gear: [0; 4],
        equipped: [true; 4],
        inventory: (1..=65)
            .map(|id| economy_rules::OwnedItem {
                id,
                slot: (id as usize) % 4,
                quality: 0,
                tier: 1,
            })
            .collect(),
        worn: [None; 4],
        next_item_id: 66,
        worlds: progression_world::WorldProgress::default(),
        talents: talent_rules::TalentProgress::default(),
        run: None,
    };
    assert!(save_format::decode(&toml::to_string(&save).unwrap()).is_err());
}
