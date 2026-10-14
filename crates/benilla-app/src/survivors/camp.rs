//! The preparation camp is a small, safe space in the selected world; combat time stays paused.
use crate::creature_anim::{move_flags, MovementState};
use crate::survivors::hero::{self, Hero};
use crate::survivors::newinteract::{Interactable, InteractableKind};
use crate::survivors::units::{self, UnitSpec};
use crate::survivors::{Phase, Selection};
use benilla_world::view::WorldCamera;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

const WALK_SPEED: f32 = 6.0;
const CAMP_RADIUS: f32 = 90.0;

#[derive(Component)]
pub(crate) struct CampEntity;

#[derive(Component)]
struct CampLabel {
    target: Entity,
    height: f32,
}

#[derive(Component)]
pub(crate) struct PortalRing;

#[derive(Resource, Default)]
pub(crate) struct ForgeState(pub bool);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        ensure_camp
            .after(crate::survivors::hud::menu_preview)
            .after(hero::place_camera),
    )
    .add_systems(Update, walk.after(super::page).after(ensure_camp))
    .add_systems(Update, project_labels.after(hero::place_camera))
    .add_systems(OnExit(Phase::Preparing), clear_camp);
}

fn clear_camp(mut commands: Commands, camp: Query<Entity, With<CampEntity>>) {
    for entity in &camp {
        commands.entity(entity).despawn();
    }
}

fn ensure_camp(
    mut rig: ResMut<hero::CamRig>,
    mut commands: Commands,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    mut heroes: Query<&mut Transform, With<Hero>>,
    existing: Query<Entity, With<CampEntity>>,
    world: WorldPoint,
    mut player: ResMut<crate::player::Player>,
    assets: Res<AssetServer>,

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
    let Some(&(_, race, sex)) = selection.look.and_then(|i| selection.looks.get(i)) else {
        return;
    };
    let site = crate::survivors::newhub_sites::for_race(race);
    let origin = units::wow_to_bevy(site.arrival.xyz);
    hero_tf.translation = origin;
    rig.dist = 10.0;
    rig.pitch = 0.35;
    let portal_arm = (origin - units::wow_to_bevy(site.portal.anchor.xyz)).with_y(0.0);
    rig.yaw = portal_arm.x.atan2(portal_arm.z);
    player.active = true;
    player.detached = false;
    player.settling = false;
    player.world_stale = false;
    player.pos = origin;
    economy.stock = crate::survivors::economy_rules::POTION_LIMIT;
    let specs = [
        (site.merchant, InteractableKind::Merchant),
        (site.artisan, InteractableKind::Artisan),
    ];
    let font = assets.load("mpq://Fonts/FRIZQT__.ttf");
    for (index, (placement, service)) in specs.into_iter().enumerate() {
        let pos = units::wow_to_bevy(placement.location.xyz);
        let yaw = placement.location.orientation;
        let npc = units::spawn_unit(
            &mut commands,
            &UnitSpec {
                serial: 1_000_001 + index as u64,
                display: placement.display_id,
                scale: 1.0,
                level: 1,
                faction: units::FACTION_FRIENDLY,
                bytes0: u32::from(race)
                    | u32::from(class.id) << 8
                    | u32::from(sex) << 16
                    | u32::from(class.power) << 24,
                weapons: &[],
                sheath: 0,
                appearance: None,
            },
            pos,
            yaw,
        );
        commands.entity(npc).insert((
            CampEntity,
            Interactable::sphere(service, Vec3::Y * 1.1, 1.25),
        ));
        spawn_label(
            &mut commands,
            &font,
            npc,
            2.7,
            if index == 0 {
                "<Marchand>"
            } else {
                "<Artisan des fusions>"
            },
            Color::srgb(1.0, 0.82, 0.0),
        );
    }

    // GameObjectDisplayInfo 4396: Vanilla Portal to Stormwind, template 176296, scale 1.
    let portal_pos = units::wow_to_bevy(site.portal.anchor.xyz);
    let ring = units::spawn_object(&mut commands, 1_000_003, 4396, 1.0, portal_pos);
    commands.entity(ring).insert((
        PortalRing,
        CampEntity,
        Interactable::sphere(InteractableKind::Portal, Vec3::Y * 1.9, 1.6),
    ));
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
    collision: benilla_world::collision::WorldCollision,
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
        let displacement = direction * WALK_SPEED * dt;
        let shape = avian3d::prelude::Collider::capsule(0.35, 1.0);
        let allowed = collision
            .cast_body(
                &shape,
                transform.translation + Vec3::Y * 1.0,
                displacement,
                0.02,
            )
            .map_or(displacement.length(), |hit| (hit.distance - 0.05).max(0.0));
        let mut position = transform.translation + direction * allowed;
        let origin = units::wow_to_bevy(
            crate::survivors::newhub_sites::for_race(
                crate::survivors::menu::selected_race(&selection).unwrap_or(1),
            )
            .arrival
            .xyz,
        );
        let offset = units::flat(position - origin);
        if offset.length() > CAMP_RADIUS {
            let held = origin + offset.normalize() * CAMP_RADIUS;
            position.x = held.x;
            position.z = held.z;
        }
        // Interior floors live in WMO collision, not the outdoor ADT heightfield.
        if let Some(floor) = collision.ray_body(position + Vec3::Y * 1.5, Dir3::NEG_Y, 4.0) {
            position.y = position.y + 1.5 - floor.distance;
        } else {
            position = transform.translation;
        }
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

fn project_labels(
    stock: Res<crate::survivors::blizzard_ui::StockUi>,
    forge: Res<ForgeState>,
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
        if *phase.get() != Phase::Preparing || stock.modal_open || forge.0 {
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
