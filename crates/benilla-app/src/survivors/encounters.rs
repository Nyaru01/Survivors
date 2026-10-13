//! Boss arrivals are scheduled by elapsed game time, independently of surviving bosses.
use super::{enemies::Enemy, hud, Phase, Run};
use bevy::prelude::*;
#[derive(Component)]
struct Countdown;
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup.after(hud::setup_hud))
        .add_systems(Update, update.after(hud::show_panels));
}
fn setup(mut commands: Commands, font: Res<hud::HudFont>) {
    commands
        .spawn((
            Countdown,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(92.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(401),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn(hud::text_bundle(
                &font.0,
                "Boss dans 03:00",
                16.0,
                Color::srgb(1.0, 0.82, 0.0),
            ));
        });
}
fn update(
    run: Res<Run>,
    phase: Res<State<Phase>>,
    enemies: Query<&Enemy>,
    mut root: Query<(&Children, &mut Visibility), With<Countdown>>,
    mut text: Query<&mut Text>,
) {
    let Ok((children, mut visibility)) = root.single_mut() else {
        return;
    };
    *visibility = if matches!(*phase.get(), Phase::Menu | Phase::GameOver) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    let seconds = super::encounter_clock::remaining(run.elapsed);
    let alive = enemies.iter().filter(|e| e.boss && !e.dead).count();
    let suffix = if alive > 0 {
        format!(" · {alive} boss en vie")
    } else {
        String::new()
    };
    for child in children {
        if let Ok(mut t) = text.get_mut(*child) {
            **t = format!("Boss dans {:02}:{:02}{suffix}", seconds / 60, seconds % 60);
        }
    }
}
