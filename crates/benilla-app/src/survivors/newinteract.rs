//! Shared world interactions for Survivors preparation services and run loot.
//!
//! An [`Interactable`] belongs on the logical actor (not each animated M2 mesh). Its volume gives
//! picking a stable target while the model animates and the click range stays tied to the actor.

use super::{hero::Hero, prep::camp, Phase};
use benilla_ui::script::{ScriptValue, UiScript};
use benilla_world::interact::{WorldClick, WorldRightClick};
use benilla_world::view::WorldCamera;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub(crate) const INTERACTION_RANGE: f32 = 3.4;
const RIGHT_DRAG_THRESHOLD: f32 = 5.0;

pub(crate) fn within_interaction_range(player: Vec3, target: Vec3) -> bool {
    player.distance(target) <= INTERACTION_RANGE
}

fn unobstructed_before_target(target_distance: f32, world_distance: Option<f32>) -> bool {
    !world_distance.is_some_and(|distance| distance + 0.05 < target_distance)
}

/// The service or world object an interaction receiver should handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractableKind {
    Merchant,
    Artisan,
    Portal,
    Chest,
}

/// A click volume attached to the logical M2/GameObject entity. `center` is local to its transform.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Interactable {
    pub kind: InteractableKind,
    pub center: Vec3,
    pub radius: f32,
}

impl Interactable {
    pub(crate) const fn sphere(kind: InteractableKind, center: Vec3, radius: f32) -> Self {
        Self {
            kind,
            center,
            radius,
        }
    }
}

/// Emitted only for a valid, in-range user interaction; run economy code can listen for chests.
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct InteractionIntent {
    pub target: Entity,
    pub kind: InteractableKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractionFailure {
    Miss,
    OutOfRange,
}

#[derive(Message, Clone, Copy)]
struct CapturedRightClick;

/// Optional target from a left click, separate from the hero's class/menu selection.
#[derive(Resource, Default)]
pub(crate) struct InteractableSelection(pub Option<Entity>);

#[derive(Resource, Default)]
struct CapturedRightGesture {
    armed: bool,
    motion: f32,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct HoverCursor;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<InteractableSelection>()
        .init_resource::<CapturedRightGesture>()
        .add_message::<InteractionIntent>()
        .add_message::<CapturedRightClick>()
        .add_systems(
            Update,
            (
                captured_right_click,
                left_select,
                right_interact,
                key_interact,
            )
                .chain()
                .after(crate::ui_script::UiInput)
                .after(crate::target::TargetUpdate),
        )
        .add_systems(
            Update,
            hover_cursor
                .in_set(HoverCursor)
                .after(crate::target::TargetUpdate),
        )
        .add_systems(
            Update,
            open_camp_service.after(right_interact).after(key_interact),
        );
}

/// CaptureMode skips the normal player controller's primary-button gesture, so Survivors records
/// the same button release locally. Accumulated Bevy motion rejects a right-drag before it becomes
/// a context click; this never reads an OS-level input API.
fn captured_right_click(
    capture: Option<Res<crate::run_mode::CaptureMode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    over_ui: Res<crate::ui_script::PointerOverUiPanel>,
    mut gesture: ResMut<CapturedRightGesture>,
    mut clicks: MessageWriter<CapturedRightClick>,
) {
    if capture.is_none() {
        return;
    }
    if mouse.just_pressed(MouseButton::Right) {
        gesture.armed = !over_ui.0;
        gesture.motion = 0.0;
    }
    if gesture.armed && mouse.pressed(MouseButton::Right) {
        gesture.motion += motion.delta.length();
        if gesture.motion > RIGHT_DRAG_THRESHOLD {
            gesture.armed = false;
        }
    }
    if mouse.just_released(MouseButton::Right) {
        if gesture.armed && !over_ui.0 && gesture.motion <= RIGHT_DRAG_THRESHOLD {
            clicks.write(CapturedRightClick);
        }
        gesture.armed = false;
    }
}

fn cursor_ray(
    window: &Window,
    cameras: &Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
) -> Option<Ray3d> {
    let cursor = window.cursor_position()?;
    let (camera, transform) = cameras.single().ok()?;
    camera.viewport_to_world(transform, cursor).ok()
}

fn ray_sphere(ray: Ray3d, center: Vec3, radius: f32) -> Option<f32> {
    let offset = ray.origin - center;
    let b = offset.dot(*ray.direction);
    let c = offset.length_squared() - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let near = -b - discriminant.sqrt();
    let far = -b + discriminant.sqrt();
    (far >= 0.0).then_some(near.max(0.0))
}

pub(crate) fn ray_target(
    ray: Ray3d,
    targets: &Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: &benilla_world::collision::WorldCollision,
) -> Option<(Entity, InteractableKind, Vec3, f32)> {
    targets
        .iter()
        .filter_map(|(entity, transform, volume)| {
            let center = transform.transform_point(volume.center);
            ray_sphere(ray, center, volume.radius).and_then(|distance| {
                let wall = collision.ray_body(ray.origin, ray.direction, distance);
                unobstructed_before_target(distance, wall.map(|hit| hit.distance)).then_some((
                    entity,
                    volume.kind,
                    transform.translation(),
                    distance,
                ))
            })
        })
        .min_by(|a, b| a.3.total_cmp(&b.3))
}

/// The same ray, range and world-occlusion path used by a native right-click, exposed so isolated
/// verification can exercise interactions without injecting desktop input or bypassing range.
pub(crate) fn intent_from_ray(
    ray: Ray3d,
    player: Vec3,
    targets: &Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: &benilla_world::collision::WorldCollision,
) -> Result<InteractionIntent, InteractionFailure> {
    let Some((target, kind, root, _)) = ray_target(ray, targets, collision) else {
        return Err(InteractionFailure::Miss);
    };
    if !within_interaction_range(player, root) {
        return Err(InteractionFailure::OutOfRange);
    }
    Ok(InteractionIntent { target, kind })
}

fn left_select(
    mut clicks: MessageReader<WorldClick>,
    over_ui: Res<crate::ui_script::PointerOverUiPanel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    targets: Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: benilla_world::collision::WorldCollision,
    mut selected: ResMut<InteractableSelection>,
) {
    if clicks.read().last().is_none() || over_ui.0 {
        return;
    }
    let Some(window) = windows.single().ok() else {
        return;
    };
    selected.0 = cursor_ray(window, &cameras)
        .and_then(|ray| ray_target(ray, &targets, &collision))
        .map(|hit| hit.0);
}

fn hover_cursor(
    over_ui: Res<crate::ui_script::PointerOverUiPanel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    targets: Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: benilla_world::collision::WorldCollision,
    hero: Query<&Transform, With<Hero>>,
    mut cursor: ResMut<crate::target::WorldCursor>,
) {
    if over_ui.0 {
        return;
    }
    let (Some(window), Ok(hero)) = (windows.single().ok(), hero.single()) else {
        return;
    };
    let Some(hit) =
        cursor_ray(window, &cameras).and_then(|ray| ray_target(ray, &targets, &collision))
    else {
        return;
    };
    cursor.kind = match hit.1 {
        InteractableKind::Merchant => crate::target::CursorKind::Buy,
        InteractableKind::Artisan | InteractableKind::Chest => crate::target::CursorKind::Interact,
        InteractableKind::Portal => crate::target::CursorKind::Taxi,
    };
    cursor.unable = !within_interaction_range(hero.translation, hit.2);
}

fn right_interact(
    mut clicks: MessageReader<WorldRightClick>,
    mut captured_clicks: MessageReader<CapturedRightClick>,
    over_ui: Res<crate::ui_script::PointerOverUiPanel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    targets: Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: benilla_world::collision::WorldCollision,
    hero: Query<&Transform, With<Hero>>,
    mut intents: MessageWriter<InteractionIntent>,
    script: Option<NonSendMut<UiScript>>,
) {
    let clicked = clicks.read().last().is_some() || captured_clicks.read().last().is_some();
    if !clicked || over_ui.0 {
        return;
    }
    let (Some(window), Ok(hero)) = (windows.single().ok(), hero.single()) else {
        return;
    };
    let Some(ray) = cursor_ray(window, &cameras) else {
        return;
    };
    match intent_from_ray(ray, hero.translation, &targets, &collision) {
        Ok(intent) => {
            intents.write(intent);
        }
        Err(InteractionFailure::OutOfRange) => show_error(script, "Trop loin pour interagir."),
        Err(InteractionFailure::Miss) => {}
    }
}

fn key_interact(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    over_ui: Res<crate::ui_script::PointerOverUiPanel>,
    hero: Query<&Transform, With<Hero>>,
    targets: Query<(Entity, &GlobalTransform, &Interactable)>,
    collision: benilla_world::collision::WorldCollision,
    mut intents: MessageWriter<InteractionIntent>,
    script: Option<NonSendMut<UiScript>>,
) {
    if !keys.just_pressed(KeyCode::KeyE) || over_ui.0 {
        return;
    }
    let Ok(hero) = hero.single() else { return };
    let nearest = targets
        .iter()
        .filter(|(_, _, interactable)| {
            *phase.get() == Phase::Preparing || interactable.kind == InteractableKind::Chest
        })
        .map(|(entity, transform, interactable)| {
            let root = transform.translation();
            (
                entity,
                interactable.kind,
                root,
                transform.transform_point(interactable.center),
                hero.translation.distance(root),
            )
        })
        .filter(|(_, _, _, center, _)| {
            let eye = hero.translation + Vec3::Y * 1.4;
            let delta = *center - eye;
            let distance = delta.length();
            let Ok(direction) = Dir3::new(delta) else {
                return false;
            };
            let wall = collision.ray_body(eye, direction, distance);
            unobstructed_before_target(distance, wall.map(|hit| hit.distance))
        })
        .min_by(|a, b| a.4.total_cmp(&b.4));
    let Some((target, kind, root, _, _)) = nearest else {
        return;
    };
    if !within_interaction_range(hero.translation, root) {
        show_error(script, "Trop loin pour interagir.");
    } else {
        intents.write(InteractionIntent { target, kind });
    }
}

fn show_error(script: Option<NonSendMut<UiScript>>, message: &str) {
    if let Some(mut script) = script {
        script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())]);
    }
}

fn open_camp_service(
    mut intents: MessageReader<InteractionIntent>,
    phase: Res<State<Phase>>,
    mut stock: ResMut<super::blizzard_ui::StockUi>,
    mut forge: ResMut<camp::ForgeState>,
    script: Option<NonSendMut<UiScript>>,
) {
    if *phase.get() != Phase::Preparing {
        return;
    }
    let Some(intent) = intents.read().last().copied() else {
        return;
    };
    let Some(mut script) = script else { return };
    match intent.kind {
        InteractableKind::Merchant => {
            let _ = script.run(
                "CloseAllWindows(); SurvivorsPreparationForge:Hide(); __survivors_merchant=true",
            );
            stock.merchant_requested = false;
            forge.0 = false;
        }
        InteractableKind::Artisan => {
            let _ = script.run("CloseAllWindows(); __survivors_open_forge()");
            stock.merchant_requested = false;
            forge.0 = true;
        }
        InteractableKind::Portal => {
            let _ = script.run("CloseAllWindows(); SurvivorsPreparationForge:Hide(); __survivors_prepare_enter=true");
            stock.merchant_requested = false;
            forge.0 = false;
        }
        InteractableKind::Chest => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_sphere_returns_nearest_forward_contact() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::Z);
        assert_eq!(ray_sphere(ray, Vec3::new(0.0, 0.0, 5.0), 1.0), Some(4.0));
        assert_eq!(ray_sphere(ray, Vec3::new(0.0, 0.0, -5.0), 1.0), None);
        assert_eq!(ray_sphere(ray, Vec3::new(2.0, 0.0, 5.0), 1.0), None);
    }

    #[test]
    fn interaction_range_is_strictly_limited_to_service_distance() {
        assert!(within_interaction_range(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 3.39)
        ));
        assert!(!within_interaction_range(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 3.41)
        ));
        // A normal floor-height difference remains in range when the 3D root distance is below 3.4.
        assert!(within_interaction_range(
            Vec3::new(0.0, 1.7, 0.0),
            Vec3::new(0.0, 3.0, 3.0)
        ));
        assert!(!within_interaction_range(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 20.0, 0.0)
        ));
    }

    #[test]
    fn world_geometry_occludes_only_when_it_is_ahead_of_the_click_volume() {
        assert!(unobstructed_before_target(10.0, None));
        assert!(unobstructed_before_target(10.0, Some(9.96)));
        assert!(!unobstructed_before_target(10.0, Some(9.94)));
    }
}
