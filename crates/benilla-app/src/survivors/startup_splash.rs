//! A short World of Warcraft title splash for the Survivors entry point.
//!
//! The art is read from the user's 1.12 MPQ chain at runtime; no Blizzard asset is bundled.

use benilla_assets::WorldAssets;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::FocusPolicy;

const SPLASH_PATH: &str = "Interface\\Glues\\LoadingScreens\\LoadScreenKalimdor.blp";
const HOLD_SECS: f32 = 1.8;
const FADE_SECS: f32 = 0.45;

// The app already enables image/png; Bevy's ImageType PNG gate is deliberately not enabled.
pub(super) fn decode_image(bytes: &[u8]) -> Option<Image> {
    let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    Some(image)
}

#[derive(Resource, Default)]
pub(super) struct StartupSplash(pub(super) bool);

#[derive(Component)]
pub(super) struct SplashImage {
    started: f32,
}

pub(super) fn block_input(
    active: Res<StartupSplash>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut key_events: ResMut<bevy::ecs::message::Messages<bevy::input::keyboard::KeyboardInput>>,
) {
    if active.0 {
        keys.reset_all();
        mouse.reset_all();
        key_events.clear();
    }
}

pub(super) fn spawn(
    mut commands: Commands,
    mut active: ResMut<StartupSplash>,
    world_assets: Option<ResMut<WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Real>>,
) {
    // Automated captures need to reach the menu immediately and deterministically.
    if crate::webenv::var("WOW_SURVIVORS_AUTO").is_some()
        || crate::webenv::var("WOW_SURVIVORS_SHOTS").is_some()
    {
        return;
    }

    // Original generated artwork is embedded; Vanilla's low-resolution art is a fallback only.
    let texture = decode_image(include_bytes!("../../../../assets/survivors/startup.png"))
        .map(|image| images.add(image))
        .or_else(|| {
            world_assets.and_then(|mut assets| assets.sprite_texture(SPLASH_PATH, &mut images))
        });
    let Some(texture) = texture else {
        return;
    };

    *active = StartupSplash(true);
    commands
        .spawn((
            SplashImage {
                started: time.elapsed_secs(),
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(3000),
            FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(texture),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
        });
}

pub(super) fn fade(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut active: ResMut<StartupSplash>,
    mut splash: Query<(Entity, &SplashImage, &mut BackgroundColor, &Children)>,
    mut images: Query<&mut ImageNode>,
) {
    let Some((entity, splash, mut background, children)) = splash.iter_mut().next() else {
        return;
    };
    let elapsed = (time.elapsed_secs() - splash.started).max(0.0);
    let alpha = (1.0 - ((elapsed - HOLD_SECS) / FADE_SECS).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    background.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    for child in children.iter() {
        if let Ok(mut image) = images.get_mut(child) {
            image.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
    }
    if elapsed >= HOLD_SECS + FADE_SECS {
        commands.entity(entity).despawn();
        active.0 = false;
    }
}
