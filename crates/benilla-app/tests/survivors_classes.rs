#[allow(dead_code)]
#[path = "../src/survivors/xp_rules.rs"]
mod xp_rules;

#[path = "../src/survivors/camera_input.rs"]
mod camera_input;
#[path = "../src/survivors/encounter_clock.rs"]
mod encounter_clock;
#[path = "../src/survivors/layout_rules.rs"]
mod layout_rules;
#[path = "../src/survivors/settings_rules.rs"]
mod settings_rules;

#[test]
fn preferences_roundtrip_and_reject_invalid_values() {
    let mut prefs = settings_rules::Preferences::default();
    prefs.azerty = true;
    prefs.master = 0.5;
    prefs.sensitivity = 1.5;
    assert_eq!(
        settings_rules::Preferences::decode(&prefs.encode()),
        Some(prefs)
    );
    assert!(settings_rules::Preferences::decode(
        "SURVIVORS_OPTIONS_1\n0 NaN 0 350 0 1 0 1 1 1 1 1\n"
    )
    .is_none());
}

#[test]
fn boss_arrivals_continue_every_three_minutes() {
    assert_eq!(encounter_clock::due(179.9), 0);
    assert_eq!(encounter_clock::due(180.0), 1);
    assert_eq!(encounter_clock::due(360.0), 2);
    assert_eq!(encounter_clock::due(720.0), 4);
    assert_eq!(encounter_clock::remaining(0.0), 180);
    assert_eq!(encounter_clock::remaining(181.0), 179);
}

#[test]
fn bottom_bar_tooltips_stay_above_the_icon_and_inside_the_screen() {
    let p = layout_rules::tooltip_position(
        [1000.0, 746.0, 40.0, 40.0],
        [300.0, 220.0],
        [1024.0, 768.0],
    );
    assert!(p[0] + 300.0 <= 1024.0);
    assert!(p[1] + 220.0 < 726.0);
    let edge =
        layout_rules::tooltip_position([20.0, 20.0, 40.0, 40.0], [300.0, 220.0], [1024.0, 768.0]);
    assert_eq!(edge, [4.0, 4.0]);
}
#[path = "../src/survivors/spell_input.rs"]
mod spell_input;

#[test]
fn spells_are_automatic_by_default_and_manual_requests_respect_cooldowns() {
    let mut control = spell_input::CastControl::default();
    assert!(control.automatic(0));
    assert!(control.should_cast(0, true));
    assert!(!control.should_cast(0, false));
    control.toggle(0);
    assert!(!control.automatic(0));
    assert!(!control.should_cast(0, true));
    control.request(0);
    assert!(control.should_cast(0, true));
    assert!(!control.should_cast(0, true));
    control.request(0);
    assert!(!control.should_cast(0, false));
    assert!(!control.should_cast(0, true));
    control.toggle(0);
    assert!(control.should_cast(0, true));
    control.request(99);
    control.toggle(99);
    assert!(!control.should_cast(99, true));
}
#[allow(dead_code)]
#[path = "../src/survivors/data.rs"]
mod data;

#[test]
fn right_drag_rotates_a_locked_camera_without_pressing_c() {
    let state = camera_input::update(
        camera_input::OrbitState {
            unlocked: false,
            yaw: 0.0,
            pitch: camera_input::FIXED_PITCH,
        },
        camera_input::OrbitInput {
            playing: true,
            right: true,
            dx: 50.0,
            dy: -30.0,
            ..Default::default()
        },
    );
    assert!(state.unlocked);
    assert!((state.yaw + 0.4).abs() < 1e-6);
    assert!(state.pitch < camera_input::FIXED_PITCH);
}

#[test]
fn clicking_a_card_does_not_rotate_the_locked_camera() {
    let state = camera_input::update(
        camera_input::OrbitState {
            unlocked: false,
            yaw: 0.0,
            pitch: camera_input::FIXED_PITCH,
        },
        camera_input::OrbitInput {
            playing: true,
            left: true,
            dx: 50.0,
            ..Default::default()
        },
    );
    assert!(!state.unlocked);
    assert_eq!(state.yaw, 0.0);
}

#[test]
fn pause_and_level_up_ignore_camera_drag() {
    let state = camera_input::update(
        camera_input::OrbitState {
            unlocked: true,
            yaw: 0.8,
            pitch: 0.7,
        },
        camera_input::OrbitInput {
            right: true,
            dx: 50.0,
            dy: 20.0,
            ..Default::default()
        },
    );
    assert_eq!(state.yaw, 0.8);
    assert_eq!(state.pitch, 0.7);
}

#[test]
fn c_restores_the_top_down_view_and_pitch_stays_above_ground() {
    let state = camera_input::update(
        camera_input::OrbitState {
            unlocked: true,
            yaw: 1.0,
            pitch: 0.5,
        },
        camera_input::OrbitInput {
            playing: true,
            toggle: true,
            ..Default::default()
        },
    );
    assert!(!state.unlocked);
    assert_eq!(state.yaw, 0.0);
    assert_eq!(state.pitch, camera_input::FIXED_PITCH);
    let dragged = camera_input::update(
        state,
        camera_input::OrbitInput {
            playing: true,
            right: true,
            dy: -10000.0,
            ..Default::default()
        },
    );
    assert_eq!(dragged.pitch, 0.18);
}

#[test]
fn warlock_never_receives_other_classes_buffs() {
    let offered: Vec<_> = data::passives_for(9).map(|p| p.key).collect();
    assert_eq!(offered, ["detect"]);
    assert!(!offered.contains(&"intellect"));
    assert!(!offered.contains(&"might"));
    assert!(!offered.contains(&"renew"));
}

#[test]
fn every_class_starts_with_one_of_its_own_spells() {
    for class in data::CLASSES {
        assert!(class.pool.contains(&class.start), "{}", class.name);
        for key in class.pool {
            assert_eq!(data::ability(key).key, *key);
        }
    }
}

#[test]
fn active_spell_pools_are_disjoint() {
    let mut owners = std::collections::HashMap::new();
    for class in data::CLASSES {
        for key in class.pool {
            assert!(
                owners.insert(key, class.id).is_none(),
                "Shared active spell {key}"
            );
        }
    }
}

#[test]
fn each_passive_has_a_known_owner_and_is_only_offered_to_that_class() {
    for passive in data::PASSIVES {
        assert!(data::CLASSES.iter().any(|c| c.id == passive.class));
        for class in data::CLASSES {
            assert_eq!(
                data::passives_for(class.id).any(|p| p.key == passive.key),
                class.id == passive.class
            );
        }
    }
}
