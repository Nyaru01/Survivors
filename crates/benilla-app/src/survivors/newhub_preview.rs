//! A disposable display-ID preview, separate from camp actors and the saved layout.
use super::{
    hero::{CamRig, Hero},
    newhub_editor::HubEditor,
    units, Phase,
};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct AssetPreview;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, preview.after(super::prep::camp::walk));
}

fn preview(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    editor: Res<HubEditor>,
    rig: Res<CamRig>,
    hero: Query<&Transform, (With<Hero>, Without<AssetPreview>)>,
    existing: Query<Entity, With<AssetPreview>>,
    mut last: Local<Option<(u32, bool)>>,
) {
    let candidate = editor
        .preview
        .as_ref()
        .filter(|_| editor.active && *phase.get() == Phase::Preparing);
    let key = candidate.map(|p| (p.0, p.2));
    if key == *last && (key.is_none() || !existing.is_empty()) {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    *last = None;
    let Some((id, path, portal)) = candidate else {
        return;
    };
    let Ok(hero) = hero.single() else { return };
    // Stand beside the player, toward the camera, without borrowing a service's entity.
    let toward_camera = Vec3::new(rig.yaw.sin(), 0.0, rig.yaw.cos());
    let right = Vec3::new(toward_camera.z, 0.0, -toward_camera.x);
    let pos = hero.translation + toward_camera * 2.0 + right * 3.0;
    let entity = if *portal {
        let scale = if path.to_ascii_lowercase().contains("mageportals\\") {
            0.1
        } else {
            1.0
        };
        let e = units::spawn_object(&mut commands, 8_000_000, *id, scale, pos);
        commands.entity(e).insert(
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(units::yaw_toward(toward_camera))),
        );
        e
    } else {
        units::spawn_unit(
            &mut commands,
            &units::UnitSpec {
                serial: 8_000_001,
                display: *id,
                scale: 1.0,
                level: 1,
                faction: units::FACTION_FRIENDLY,
                bytes0: 0,
                weapons: &[],
                sheath: 0,
                appearance: None,
            },
            pos,
            units::yaw_toward(toward_camera),
        )
    };
    commands.entity(entity).insert(AssetPreview);
    *last = key;
}
