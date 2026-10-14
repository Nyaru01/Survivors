#![allow(dead_code)]
#[path = "../src/survivors/xp_rules.rs"]
mod xp_rules;
#[path = "../src/survivors/data.rs"]
mod data;
#[path = "../src/survivors/encounter_clock.rs"]
mod encounter_clock;
#[path = "../src/survivors/localization.rs"]
mod localization;

#[test]
fn every_class_active_and_passive_has_a_french_label() {
    for class in data::CLASSES {
        assert_ne!(localization::class_name(class.id), "Classe inconnue");
    }
    for ability in data::ABILITIES {
        assert!(
            localization::ability_name(ability.key).is_some(),
            "{}",
            ability.key
        );
    }
    for passive in data::PASSIVES {
        assert!(
            localization::passive_name(passive.key).is_some(),
            "{}",
            passive.key
        );
    }
}

#[test]
fn every_map_has_a_french_label() {
    for map in data::MAPS {
        assert!(localization::zone_name(map.name).is_some(), "{}", map.name);
    }
}

#[test]
fn every_current_enemy_and_boss_has_a_french_presentation_label() {
    for (index, map) in data::MAPS.iter().enumerate() {
        let roster = data::roster(index);
        for enemy in roster.enemies.iter().chain(&roster.bosses) {
            assert!(
                localization::foe_name(enemy.name).is_some(),
                "{}: {}",
                map.name,
                enemy.name
            );
        }
    }
}

#[test]
fn unknown_identifiers_remain_explicit_instead_of_becoming_wrong_names() {
    assert_eq!(localization::ability_name("unknown_new_spell"), None);
    assert_eq!(localization::passive_name("new_passive"), None);
    assert_eq!(localization::foe_name("unreviewed_creature"), None);
    assert_eq!(localization::race_name(255), "Race inconnue");
}

#[test]
fn french_labels_do_not_change_gameplay_keys_or_saved_identity() {
    assert_eq!(data::ability("shadow_bolt").key, "shadow_bolt");
    assert_eq!(localization::class_name(9), "Démoniste");
    assert_eq!(
        localization::ability_name("shadow_bolt"),
        Some("Trait de l’ombre")
    );
    assert_eq!(
        localization::zone_name("Tirisfal Glades"),
        Some("Clairières de Tirisfal")
    );
}
