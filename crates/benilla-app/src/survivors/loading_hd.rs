//! Survivors cover above all HUD layers; upstream loading readiness remains authoritative.
use benilla_assets::WorldAssets;
use bevy::prelude::*;

#[derive(Component)]
struct Cover;
#[derive(Component)]
struct Fill;
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn.after(benilla_assets::AssetSet::Open))
        .add_systems(PostUpdate, show.before(bevy::ui::UiSystems::Layout));
}
fn spawn(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world: Option<ResMut<WorldAssets>>,
) {
    let Some(image) = super::startup_splash::decode_image(include_bytes!(
        "../../../../assets/survivors/loading.png"
    )) else {
        return;
    };
    let art = images.add(image);
    let (logo, border, fill) = if let Some(mut world) = world {
        (
            world.sprite_texture("Interface\\Glues\\Common\\Glues-WoW-Logo", &mut images),
            world.sprite_texture(
                "Interface\\Glues\\LoadingBar\\Loading-BarBorder.blp",
                &mut images,
            ),
            world.sprite_texture(
                "Interface\\Glues\\LoadingBar\\Loading-BarFill.blp",
                &mut images,
            ),
        )
    } else {
        (None, None, None)
    };
    commands
        .spawn((
            Cover,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(10000),
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(art),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
            if let Some(logo) = logo {
                root.spawn((
                    ImageNode::new(logo),
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Percent(4.0),
                        right: Val::Percent(5.0),
                        width: Val::Percent(25.0),
                        aspect_ratio: Some(2.0),
                        ..default()
                    },
                ));
            }
            if let Some(fill) = fill {
                root.spawn((
                    Fill,
                    ImageNode::new(fill),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(23.75),
                        bottom: Val::Percent(6.25),
                        width: Val::Percent(0.0),
                        height: Val::Percent(2.5),
                        ..default()
                    },
                ));
            }
            if let Some(border) = border {
                root.spawn((
                    ImageNode::new(border),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(20.0),
                        bottom: Val::Percent(5.0),
                        width: Val::Percent(60.0),
                        height: Val::Percent(5.0),
                        ..default()
                    },
                ));
            }
        });
}
fn show(
    mut commands: Commands,
    screen: Res<crate::loading_screen::LoadingScreen>,
    progress: Res<benilla_world::terrain_stream::WorldLoadProgress>,
    mut cover: Query<&mut Node, (With<Cover>, Without<Fill>)>,
    mut fill: Query<&mut Node, (With<Fill>, Without<Cover>)>,
    mut displayed: Local<f32>,
    mut frames: Local<u32>,
    mut captured: Local<bool>,
    warm: Res<crate::pipe_warm::WarmPass>,
) {
    let visible = screen.covering();
    for mut node in &mut cover {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if visible {
        *displayed = displayed.max(progress.fraction());
    } else {
        *displayed = 0.0;
    }
    for mut node in &mut fill {
        node.width = Val::Percent(*displayed * 52.5);
    }
    *frames = if visible { frames.saturating_add(1) } else { 0 };
    if *frames >= 12
        && warm.satisfied()
        && !*captured
        && crate::webenv::var("WOW_SURVIVORS_HD_LOADING_PROBE").as_deref() == Some("1")
    {
        if let Some(home) = crate::local_state::home() {
            if crate::local_state::read_to_string(&home.join("survivors-probe.marker"))
                .ok()
                .is_some_and(|s| s.trim() == "isolated validation")
            {
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(
                        home.join("loading-hd.png"),
                    ));
                *captured = true;
            }
        }
    }
}
