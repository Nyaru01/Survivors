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
// The hop uses the same 1.12 CMovement jump seed and gravity as the arena.
const JUMP_SPEED: f32 = 7.955_547;
const GRAVITY: f32 = 19.291_105;

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
    crate::survivors::newhub_editor::plugin(app);
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
    editor: Res<crate::survivors::newhub_editor::HubEditor>,

    mut economy: ResMut<crate::survivors::economy::Economy>,
    mut spawned_revision: Local<u64>,
) {
    if *phase.get() != Phase::Preparing
        || (!existing.is_empty() && *spawned_revision == editor.revision)
    {
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
    let config = &editor.document.hubs[crate::survivors::newhub_editor::faction(site)];
    if existing.is_empty() {
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
    } else {
        // Rebuild edited models without moving the player back to the arrival point.
        for entity in &existing {
            commands.entity(entity).despawn();
        }
    }
    *spawned_revision = editor.revision;
    let specs = [
        (site.merchant, InteractableKind::Merchant),
        (site.artisan, InteractableKind::Artisan),
    ];
    let font = assets.load("mpq://Fonts/FRIZQT__.ttf");
    for (index, (_placement, service)) in specs.into_iter().enumerate() {
        let transform = editor.transform(site, index);
        let pos = transform.translation;
        let yaw = transform.rotation.to_euler(EulerRot::YXZ).0;
        let npc = units::spawn_unit(
            &mut commands,
            &UnitSpec {
                serial: 1_000_001 + index as u64,
                display: config.displays[index],
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
            crate::survivors::newhub_editor::CampActor(index),
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
    for (index, portal) in config.portals.iter().enumerate() {
        let portal_tf = editor.transform(site, 2 + index);
        let portal_pos = portal_tf.translation;
        let ring = units::spawn_object(
            &mut commands,
            1_000_003 + index as u64,
            portal.display,
            portal.scale,
            portal_pos,
        );
        commands.entity(ring).insert((
            PortalRing,
            portal_tf,
            CampEntity,
            crate::survivors::newhub_editor::CampActor(index + 2),
            crate::survivors::newhub_editor::PortalDestination(portal.destination),
            // Keep a usable world-space pick volume when a large source model is scaled down.
            Interactable::sphere(
                InteractableKind::Portal,
                Vec3::Y * (1.9 / portal.scale.min(1.0)),
                1.6 / portal.scale.min(1.0),
            ),
        ));
        spawn_label(
            &mut commands,
            &font,
            ring,
            -1.0,
            &format!("{} — E", portal.name),
            Color::srgb(0.4, 0.8, 1.0),
        );
    }
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
pub(crate) fn walk(
    collision: benilla_world::collision::WorldCollision,
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<crate::survivors::settings::Settings>,
    stock: Res<crate::survivors::blizzard_ui::StockUi>,
    keyboard: Res<crate::ui_script::UiKeyboardCapture>,
    rig: Res<hero::CamRig>,
    selection: Res<Selection>,
    world: WorldPoint,
    mut player: ResMut<crate::player::Player>,
    mut heroes: Query<(&mut Transform, &mut MovementState, &mut Hero), With<Hero>>,
    script: Option<NonSendMut<benilla_ui::script::UiScript>>,
) {
    let stock_menu_open = script.as_ref().is_some_and(|script| {
        script
            .eval::<bool>(
                "return (GameMenuFrame and GameMenuFrame:IsShown()) or (OptionsFrame and OptionsFrame:IsShown()) or (SoundOptionsFrame and SoundOptionsFrame:IsShown()) or (UIOptionsFrame and UIOptionsFrame:IsShown()) or (KeyBindingFrame and KeyBindingFrame:IsShown()) or false",
            )
            .unwrap_or(false)
    });
    if *phase.get() != Phase::Preparing
        || settings.open
        || stock.menu_requested
        || stock_menu_open
        || keyboard.typing
    {
        return;
    }
    let Ok((mut transform, mut movement, mut hero_state)) = heroes.single_mut() else {
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
    let mut position = transform.translation;
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
            .cast_body(&shape, transform.translation + Vec3::Y, displacement, 0.02)
            .map_or(displacement.length(), |hit| (hit.distance - 0.05).max(0.0));
        position += direction * allowed;
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
    }
    // WMO collision provides the camp floor; sweep the capsule through the jump arc too.
    let ground_y = transform.translation.y - hero_state.air;
    if let Some(floor) = collision.ray_body(
        Vec3::new(position.x, ground_y + 1.5, position.z),
        Dir3::NEG_Y,
        4.0,
    ) {
        let floor_y = ground_y + 1.5 - floor.distance;
        if keys.just_pressed(KeyCode::Space) && hero_state.vy == 0.0 && hero_state.air == 0.0 {
            hero_state.vy = JUMP_SPEED;
        }
        if hero_state.vy != 0.0 || hero_state.air > 0.0 {
            let dt = time.delta_secs();
            hero_state.vy -= GRAVITY * dt;
            let next_air = hero_state.air + hero_state.vy * dt;
            let vertical = next_air - hero_state.air;
            if vertical != 0.0 {
                let shape = avian3d::prelude::Collider::capsule(0.35, 1.0);
                if let Some(hit) =
                    collision.cast_body(&shape, position + Vec3::Y, Vec3::Y * vertical, 0.02)
                {
                    if vertical < 0.0 {
                        hero_state.air = 0.0;
                    } else {
                        hero_state.air += (hit.distance - 0.05).max(0.0);
                    }
                    hero_state.vy = 0.0;
                } else {
                    hero_state.air = next_air;
                }
            }
            if hero_state.air <= 0.0 {
                hero_state.air = 0.0;
                hero_state.vy = 0.0;
            }
        }
        position.y = floor_y + hero_state.air;
    } else {
        position = transform.translation;
    }
    transform.translation = position;
    if direction.length_squared() > 0.0 {
        transform.rotation = Quat::from_rotation_y(units::yaw_toward(direction));
        movement.flags = move_flags::FORWARD;
        movement.speed = WALK_SPEED;
    } else {
        movement.flags = 0;
        movement.speed = 0.0;
    }
    if hero_state.vy != 0.0 || hero_state.air > 0.0 {
        movement.flags |= move_flags::FALLING;
    }
    movement.vertical_speed = hero_state.vy;
    player.active = true;
    player.detached = false;
    player.settling = false;
    player.world_stale = false;
    player.pos = transform.translation;
}

fn project_labels(
    loading: Res<crate::loading_screen::LoadingScreen>,
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
        if *phase.get() != Phase::Preparing || loading.covering() || stock.modal_open || forge.0 {
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
