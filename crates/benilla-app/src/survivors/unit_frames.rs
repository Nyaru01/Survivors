//! Blizzard unit-frame art and the engine's model portraits, with local Survivors health.

use bevy::prelude::*;

use super::enemies::Enemy;
use super::hero::Hero;
use super::hud::{self, UiArt};
use super::{Phase, Run};
use crate::portrait::{PortraitImages, PortraitSource};

#[derive(Resource, Default)]
pub(crate) struct PortraitUnits {
    pub(crate) hero: Option<Entity>,
    pub(crate) target: Option<Entity>,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Player,
    Target,
}
impl Slot {
    fn token(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Target => "target",
        }
    }
}
#[derive(Component)]
pub(super) struct Frame(Slot);
#[derive(Component)]
pub(super) struct Face(Slot);
#[derive(Component)]
pub(super) struct Name(Slot);
#[derive(Component)]
pub(super) struct Health(Slot);
#[derive(Component)]
pub(super) struct HealthText(Slot);
#[derive(Component)]
pub(super) struct Detail(Slot);
#[derive(Component)]
pub(super) struct SpellBarTitle;

pub(super) fn spawn(root: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    for slot in [Slot::Player, Slot::Target] {
        root.spawn((
            Frame(slot),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(if slot == Slot::Player { 14.0 } else { 102.0 }),
                width: Val::Px(232.0),
                height: Val::Px(100.0),
                ..default()
            },
        ))
        .with_children(|frame| {
            // The player's round portrait is placed inside the stock frame's left ring.
            frame.spawn((
                Face(slot),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(42.0),
                    top: Val::Px(12.0),
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    border_radius: BorderRadius::MAX,
                    overflow: Overflow::clip(),
                    ..default()
                },
                ImageNode::default(),
            ));
            frame
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(106.0),
                    top: Val::Px(24.0),
                    width: Val::Px(119.0),
                    height: Val::Px(16.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .insert(ZIndex(2))
                .with_children(|n| {
                    n.spawn((Name(slot), hud::text_bundle(font, "", 12.0, hud::GOLD_TONE)));
                });
            frame
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(106.0),
                    top: Val::Px(41.0),
                    width: Val::Px(119.0),
                    height: Val::Px(12.0),
                    ..default()
                })
                .insert((ZIndex(2), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7))))
                .with_children(|bar| {
                    bar.spawn((
                        Health(slot),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode::default(),
                        UiArt::crop(
                            "Interface\\TargetingFrame\\UI-StatusBar",
                            [0.0, 1.0, 0.0, 1.0],
                        ),
                    ));
                    bar.spawn(Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|n| {
                        n.spawn((
                            HealthText(slot),
                            hud::text_bundle(font, "", 11.0, Color::WHITE),
                        ));
                    });
                });
            frame
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(106.0),
                    top: Val::Px(56.0),
                    width: Val::Px(119.0),
                    height: Val::Px(13.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .insert(ZIndex(2))
                .with_children(|n| {
                    n.spawn((Detail(slot), hud::text_bundle(font, "", 11.0, Color::WHITE)));
                });
            // The original Blizzard art frames the portrait and the status bars.
            frame.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(232.0),
                    height: Val::Px(100.0),
                    ..default()
                },
                ImageNode {
                    flip_x: true,
                    ..default()
                },
                ZIndex(1),
                UiArt::crop(
                    "Interface\\TargetingFrame\\UI-TargetingFrame",
                    [0.09375, 1.0, 0.0, 0.78125],
                ),
            ));
        });
    }
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(14.0),
            ..default()
        },
        hud::text_bundle(
            font,
            "Camera : clic droit + glisser | Molette : zoom | C : verrouiller",
            11.0,
            Color::WHITE,
        ),
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn refresh(
    run: Res<Run>,
    phase: Res<State<Phase>>,
    heroes: Query<(Entity, &Transform), With<Hero>>,
    enemies: Query<(Entity, &Enemy, &Transform)>,
    selected: Res<crate::target::Selection>,
    mut units: ResMut<PortraitUnits>,
    portraits: Res<PortraitImages>,
    control: Res<super::spell_control::SpellControl>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut frames: Query<(&Frame, &mut Node)>,
    mut faces: Query<(&Face, &mut ImageNode), Without<Health>>,
    mut health: Query<(&Health, &mut Node, &mut ImageNode), Without<Frame>>,
    mut texts: ParamSet<(
        Query<(&Name, &mut Text)>,
        Query<(&HealthText, &mut Text)>,
        Query<(&Detail, &mut Text)>,
        Query<&mut Text, With<SpellBarTitle>>,
    )>,
) {
    let active = !matches!(*phase.get(), Phase::Menu | Phase::GameOver);
    units.hero = heroes.single().ok().map(|(e, _)| e).filter(|_| active);
    let valid = |e: Entity| {
        enemies
            .get(e)
            .is_ok_and(|(_, enemy, _)| !enemy.dead && enemy.hp > 0.0)
    };
    // Prefer the clicked enemy, retain it while alive, otherwise show the nearest threat.
    units.target = if active {
        selected
            .target
            .filter(|e| valid(*e))
            .or_else(|| units.target.filter(|e| valid(*e)))
            .or_else(|| {
                heroes.single().ok().and_then(|(_, hero)| {
                    enemies
                        .iter()
                        .filter(|(_, enemy, _)| !enemy.dead && enemy.hp > 0.0)
                        .min_by(|a, b| {
                            a.2.translation
                                .distance_squared(hero.translation)
                                .total_cmp(&b.2.translation.distance_squared(hero.translation))
                        })
                        .map(|(e, _, _)| e)
                })
            })
    } else {
        None
    };
    let target = units
        .target
        .and_then(|e| enemies.get(e).ok())
        .map(|(_, enemy, _)| enemy);
    for (frame, mut node) in &mut frames {
        node.display = if active && (frame.0 == Slot::Player || target.is_some()) {
            Display::Flex
        } else {
            Display::None
        };
    }
    let mut assets = assets;
    for (face, mut image) in &mut faces {
        let handle = match portraits.0.get(face.0.token()) {
            Some(PortraitSource::Live(handle)) => Some(handle.clone()),
            Some(PortraitSource::File(path)) => assets
                .as_deref_mut()
                .and_then(|a| a.sprite_texture(path, &mut images)),
            None => None,
        };
        if let Some(handle) = handle {
            image.image = handle;
            image.color = Color::WHITE;
        }
    }
    let stats = run.stats();
    let values = |slot| match slot {
        Slot::Player => (run.hp, stats.max_hp),
        Slot::Target => target.map_or((0.0, 1.0), |enemy| (enemy.hp, enemy.max_hp)),
    };
    for (bar, mut node, mut image) in &mut health {
        let (hp, max) = values(bar.0);
        node.width = Val::Percent((hp / max.max(1.0)).clamp(0.0, 1.0) * 100.0);
        image.color = if bar.0 == Slot::Player {
            Color::srgb(0.0, 0.85, 0.0)
        } else {
            Color::srgb(0.85, 0.12, 0.10)
        };
    }
    for (name, mut text) in &mut texts.p0() {
        let value = match name.0 {
            Slot::Player => format!(
                "{} {}",
                crate::char_select::race_name(run.race),
                run.class.name
            ),
            Slot::Target => target.map_or(String::new(), |e| e.def.name.to_string()),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (bar, mut text) in &mut texts.p1() {
        let (hp, max) = values(bar.0);
        let value = format!("{:.0} / {:.0}", hp.max(0.0), max);
        if text.0 != value {
            text.0 = value;
        }
    }
    for (detail, mut text) in &mut texts.p2() {
        let value = match detail.0 {
            Slot::Player => format!("Niveau {}", run.level),
            Slot::Target => target.map_or(String::new(), |e| {
                format!("{} {}", if e.boss { "Boss" } else { "Niveau" }, e.def.level)
            }),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for mut text in &mut texts.p3() {
        let automatic = (0..run.abilities.len())
            .filter(|&slot| control.0.automatic(slot))
            .count();
        let value = format!(
            "{} — Auto {automatic}/{} | Clic ou touches 1–6",
            run.class.name,
            run.abilities.len()
        );
        if text.0 != value {
            text.0 = value;
        }
    }
}
