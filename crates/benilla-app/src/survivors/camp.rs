//! The preparation camp is a small, safe space in the selected world; combat time stays paused.
use crate::creature_anim::{move_flags, MovementState};
use crate::survivors::hero::{self, Hero};
use crate::survivors::units::{self, UnitSpec};
use crate::survivors::{Phase, Selection};
use benilla_ui::script::UiScript;
use benilla_world::view::WorldCamera;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

const WALK_SPEED: f32 = 6.0;
const CAMP_RADIUS: f32 = 18.0;

#[derive(Component)]
pub(crate) struct CampEntity;

#[derive(Component)]
struct CampLabel {
    target: Entity,
    height: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum CampService {
    Merchant,
    Artisan,
}

#[derive(Component)]
pub(crate) struct PortalRing;

#[derive(Resource, Default)]
pub(super) struct ForgeState(pub bool);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        ensure_camp
            .after(crate::survivors::hud::menu_preview)
            .after(hero::place_camera),
    )
    .add_systems(Update, walk.after(super::page).after(ensure_camp))
    .add_systems(
        Update,
        interact
            .after(crate::ui_script::UiInput)
            .after(walk)
            .after(super::page),
    )
    .add_systems(Update, project_labels.after(hero::place_camera))
    .add_systems(Update, animate_portal)
    .add_systems(OnExit(Phase::Preparing), clear_camp);
}

fn clear_camp(mut commands: Commands, camp: Query<Entity, With<CampEntity>>) {
    for entity in &camp {
        commands.entity(entity).despawn();
    }
}

fn ensure_camp(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    mut heroes: Query<&mut Transform, With<Hero>>,
    existing: Query<Entity, With<CampEntity>>,
    world: WorldPoint,
    mut player: ResMut<crate::player::Player>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut economy: ResMut<crate::survivors::economy::Economy>,
) {
    if *phase.get() != Phase::Preparing || !existing.is_empty() {
        return;
    }
    let Ok(mut hero_tf) = heroes.single_mut() else {
        return;
    };
    let Some(class) = selection.class_def() else {
        return;
    };
    let Some(&(display, race, sex)) = selection.look.and_then(|i| selection.looks.get(i)) else {
        return;
    };
    let mut origin = hero::arena(selection.stage_def());
    origin.y = world
        .terrain_height_under(origin)
        .unwrap_or(hero_tf.translation.y);
    hero_tf.translation = origin;
    player.active = true;
    player.detached = false;
    player.settling = false;
    player.world_stale = false;
    player.pos = origin;
    economy.stock = crate::survivors::economy_rules::POTION_LIMIT;
    let specs = [
        (Vec3::new(4.0, 0.0, -3.0), CampService::Merchant),
        (Vec3::new(-4.0, 0.0, -3.0), CampService::Artisan),
    ];
    let font = assets.load("mpq://Fonts/FRIZQT__.ttf");
    for (index, (offset, service)) in specs.into_iter().enumerate() {
        let mut pos = origin + offset;
        // The displayed character is a model already loaded by the selection preview.
        pos.y = world.terrain_height_under(pos).unwrap_or(origin.y);
        let yaw = units::yaw_toward(origin - pos);
        let npc = units::spawn_unit(
            &mut commands,
            &UnitSpec {
                serial: 1_000_001 + index as u64,
                display,
                scale: 1.0,
                level: 1,
                faction: units::FACTION_FRIENDLY,
                bytes0: u32::from(race)
                    | u32::from(class.id) << 8
                    | u32::from(sex) << 16
                    | u32::from(class.power) << 24,
                weapons: &[],
                sheath: 0,
                appearance: Some(selection.appearance),
            },
            pos,
            yaw,
        );
        commands.entity(npc).insert((CampEntity, service));
        spawn_label(
            &mut commands,
            &font,
            npc,
            2.7,
            if index == 0 {
                "Marchand\nE : commercer"
            } else {
                "Artisan\nE : ouvrir l’atelier"
            },
            Color::srgb(1.0, 0.82, 0.0),
        );
    }

    let mut portal_pos = origin + Vec3::new(0.0, 0.0, -10.0);
    portal_pos.y = world.terrain_height_under(portal_pos).unwrap_or(origin.y);
    let ring = commands
        .spawn((
            Mesh3d(meshes.add(Torus::new(1.2, 1.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.55, 0.9),
                emissive: Color::srgb(0.05, 0.35, 0.8).into(),
                metallic: 0.15,
                perceptual_roughness: 0.24,
                ..default()
            })),
            Transform::from_translation(portal_pos + Vec3::Y * 1.6)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
            PortalRing,
            CampEntity,
        ))
        .id();
    spawn_label(
        &mut commands,
        &font,
        ring,
        -1.0,
        "Portail — E",
        Color::srgb(0.4, 0.8, 1.0),
    );
}

fn spawn_label(
    commands: &mut Commands,
    font: &Handle<Font>,
    target: Entity,
    height: f32,
    value: &str,
    color: Color,
) {
    commands.spawn((
        CampEntity,
        CampLabel { target, height },
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: 18.0,
            ..default()
        },
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::BLACK,
        },
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        GlobalZIndex(1200),
    ));
}

#[allow(clippy::too_many_arguments)]
fn walk(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<crate::survivors::settings::Settings>,
    stock: Res<crate::survivors::blizzard_ui::StockUi>,
    forge: Res<ForgeState>,
    rig: Res<hero::CamRig>,
    selection: Res<Selection>,
    world: WorldPoint,
    mut player: ResMut<crate::player::Player>,
    mut heroes: Query<(&mut Transform, &mut MovementState), With<Hero>>,
) {
    if *phase.get() != Phase::Preparing || settings.open || stock.modal_open || forge.0 {
        return;
    }
    let Ok((mut transform, mut movement)) = heroes.single_mut() else {
        return;
    };
    let controls = settings.movement_down();
    let mut direction = Vec3::new(
        (controls[3] || keys.pressed(KeyCode::ArrowRight)) as u8 as f32
            - (controls[2] || keys.pressed(KeyCode::ArrowLeft)) as u8 as f32,
        0.0,
        (controls[1] || keys.pressed(KeyCode::ArrowDown)) as u8 as f32
            - (controls[0] || keys.pressed(KeyCode::ArrowUp)) as u8 as f32,
    );
    if direction != Vec3::ZERO {
        let (sy, cy) = rig.yaw.sin_cos();
        direction = Vec3::new(
            direction.x * cy + direction.z * sy,
            0.0,
            direction.z * cy - direction.x * sy,
        )
        .normalize();
        let dt = time.delta_secs();
        let mut position = transform.translation + direction * WALK_SPEED * dt;
        let origin = hero::arena(selection.stage_def());
        let offset = units::flat(position - origin);
        if offset.length() > CAMP_RADIUS {
            let held = origin + offset.normalize() * CAMP_RADIUS;
            position.x = held.x;
            position.z = held.z;
        }
        position.y = world
            .terrain_height_under(position)
            .unwrap_or(transform.translation.y);
        transform.translation = position;
        transform.rotation = Quat::from_rotation_y(units::yaw_toward(direction));
        movement.flags = move_flags::FORWARD;
        movement.speed = WALK_SPEED;
    } else {
        movement.flags = 0;
        movement.speed = 0.0;
    }
    player.active = true;
    player.detached = false;
    player.settling = false;
    player.world_stale = false;
    player.pos = transform.translation;
}

fn interact(
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    hero: Query<&Transform, With<Hero>>,
    targets: Query<(&Transform, Option<&PortalRing>, Option<&CampService>), With<CampEntity>>,
    script: Option<NonSendMut<UiScript>>,
) {
    if *phase.get() != Phase::Preparing || !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok(hero) = hero.single() else { return };
    let nearest = targets
        .iter()
        .map(|(transform, portal, service)| {
            (
                units::flat(transform.translation - hero.translation).length(),
                portal.is_some(),
                service.copied(),
            )
        })
        .filter(|(distance, _, _)| *distance <= 3.4)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let (Some(mut script), Some((_, portal, service))) = (script, nearest) else {
        return;
    };
    if portal {
        let _ = script.run("__survivors_prepare_enter=true");
    } else if matches!(service, Some(CampService::Merchant)) {
        let _ = script.run("__survivors_merchant=true");
    } else if matches!(service, Some(CampService::Artisan)) {
        let _ = script.run("SurvivorsPreparationArtisan:Click()");
    }
}

fn project_labels(
    phase: Res<State<Phase>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    targets: Query<&Transform, Without<CampLabel>>,
    mut labels: Query<(&CampLabel, &mut Node), Without<WorldCamera>>,
    scale: Res<UiScale>,
) {
    let Ok((camera, camera_transform)) = cams.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        if *phase.get() != Phase::Preparing {
            node.display = Display::None;
            continue;
        }
        let Ok(target) = targets.get(label.target) else {
            node.display = Display::None;
            continue;
        };
        let Ok(screen) = camera.world_to_viewport(
            &GlobalTransform::from(*camera_transform),
            target.translation + Vec3::Y * label.height,
        ) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = Val::Px(screen.x / scale.0 - 100.0);
        node.top = Val::Px(screen.y / scale.0 - 38.0);
        node.width = Val::Px(200.0);
    }
}

fn animate_portal(time: Res<Time<Real>>, mut portal: Query<&mut Transform, With<PortalRing>>) {
    for mut transform in &mut portal {
        transform.rotate_local_z(time.delta_secs() * 0.2);
    }
}
