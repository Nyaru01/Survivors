//! Character roster migration and persistence without the renderer or Bevy.
#[allow(dead_code)]
#[path = "../src/survivors/character_roster.rs"]
mod character_roster;
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
fn each_profile_keeps_its_own_inventory_talents_and_suspended_run() {
    let mut legacy = save_format::Save::fresh(save_format::Character {
        name: "Ariane".into(),
        class: 0,
        race: 1,
        sex: 0,
        map: 0,
        appearance: [0; 5],
    });
    legacy.gold = 120;
    legacy.talents.observe_level(1, 16).unwrap();
    let legacy_text = toml::to_string(&legacy).unwrap();
    let mut roster = character_roster::decode(&legacy_text).unwrap();
    let old_id = roster.active_id.clone();
    let new_id = roster.create("Béatrice".into()).unwrap();
    assert_eq!(roster.active().unwrap().save.gold, 0);
    assert!(roster.active().unwrap().save.talents.profiles.is_empty());

    roster.select(&old_id);
    roster.slots[0].save.run = Some(save_format::Session {
        run_id: String::new(),
        damage_taken: 0.0,
        gold_earned: 0,
        elites_killed: 0,
        guardians_killed: 0,
        relic_rerolls: 3,
        gold_remainder: 0,
        character: roster.slots[0].save.character.clone(),
        elapsed: 15.0,
        hp: 90.0,
        xp: 3.0,
        level: 2,
        kills: 1,
        bosses: 0,
        boss_kills: 0,
        rings: 0,
        pending_levels: 0,
        transient_gear: [0; 4],
        spell_relics: vec![],
        pending_chest: None,
        damage_done: 10.0,
        pos: [1.0, 2.0, 3.0],
        yaw: 0.0,
        abilities: vec![],
        passives: vec![],
        effects: [0.0; 10],
        thorns_spell: 0,
        mobile: [0.0; 4],
        stock: 0,
        opened: 0,
        next_chest: 30.0,
        potion_cd: 0.0,
        enemies: vec![],
        pickups: vec![],
        chests: vec![],
    });
    roster.select(&new_id);
    let saved = character_roster::encode(&roster).unwrap();
    let restored = character_roster::decode(&saved).unwrap();
    let old = restored
        .slots
        .iter()
        .find(|slot| slot.id == old_id)
        .unwrap();
    let new = restored
        .slots
        .iter()
        .find(|slot| slot.id == new_id)
        .unwrap();
    assert_eq!(old.save.gold, 120);
    assert_eq!(old.save.talents.profiles[0].highest_level, 16);
    assert_eq!(old.save.run.as_ref().unwrap().elapsed, 15.0);
    assert_eq!(new.save.gold, 0);
    assert!(new.save.talents.profiles.is_empty());
    assert!(new.save.run.is_none());
}

#[test]
fn deleting_profiles_keeps_the_next_stable_id_active_and_allows_an_empty_roster() {
    let mut roster = character_roster::Roster::empty();
    let first = roster
        .create_character(save_format::Character {
            name: "Ariane".into(),
            class: 0,
            race: 1,
            sex: 0,
            map: 0,
            appearance: [0; 5],
        })
        .unwrap();
    let second = roster
        .create_character(save_format::Character {
            name: "Béatrice".into(),
            class: 1,
            race: 3,
            sex: 1,
            map: 0,
            appearance: [0; 5],
        })
        .unwrap();

    assert!(roster.delete(&second));
    assert_eq!(roster.active_id, first);
    assert!(!roster.delete("stale-id"));
    assert!(roster.delete(&first));
    assert!(roster.slots.is_empty());
    assert!(roster.active_id.is_empty());

    let encoded = character_roster::encode(&roster).unwrap();
    let decoded = character_roster::decode(&encoded).unwrap();
    assert!(decoded.slots.is_empty());
    assert!(decoded.active_id.is_empty());
}

#[test]
fn roster_summaries_include_race_sex_and_saved_level() {
    let mut roster = character_roster::Roster::empty();
    let id = roster
        .create_character(save_format::Character {
            name: "Ariane".into(),
            class: 0,
            race: 4,
            sex: 1,
            map: progression_world::home(4).unwrap(),
            appearance: [0; 5],
        })
        .unwrap();
    roster
        .active_mut()
        .unwrap()
        .save
        .talents
        .observe_level(1, 17)
        .unwrap();
    let summary = roster.summaries().pop().unwrap();
    assert_eq!(summary.id, id);
    assert_eq!(summary.race, 4);
    assert_eq!(summary.sex, 1);
    assert_eq!(summary.level, 17);
    assert_eq!(summary.map, 7);
}
