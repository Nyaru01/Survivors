#[path = "../src/survivors/camera_input.rs"]
mod camera_input;
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
