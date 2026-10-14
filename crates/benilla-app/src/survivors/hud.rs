//! The mode's screens, in bevy_ui since FrameXML is off: the class and hero pick, the in-run HUD,
//! the level-up cards, pause, and the results.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};
use bevy::window::PrimaryWindow;

use super::data::{self, AbilityDef, Effect, Kind, PassiveDef, Stat};
use super::hero::{self, Hero};
use super::minimap::{self, MiniMap};
use super::{Owned, Phase, Rng, Run, Selection, Stats};

const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// The mobility button's edge, an action button's 36 units.
const MOBILITY_SIZE: f32 = 40.0;
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const BUTTON: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const BUTTON_HOT: Color = Color::srgba(0.36, 0.12, 0.06, 0.98);
const DIM: Color = Color::srgb(0.6, 0.6, 0.6);
pub(super) const DIM_GREY: Color = DIM;
pub(super) const GOLD_TONE: Color = GOLD;

#[derive(Component)]
pub(super) struct MenuRoot;
#[derive(Component)]
pub(super) struct HudRoot;
#[derive(Component)]
pub(super) struct LevelUpRoot;
#[derive(Component)]
pub(super) struct PauseRoot;
#[derive(Component)]
pub(super) struct OverRoot;
#[derive(Component)]
pub(super) struct XpBar;
#[derive(Component)]
pub(super) struct XpFill;
#[derive(Component)]
pub(super) struct XpText;
#[derive(Component)]
pub(super) struct LevelText;
#[derive(Component)]
pub(super) struct TimerText;
#[derive(Component)]
pub(super) struct KillText;
#[derive(Component)]
pub(super) struct HpFrame;
#[derive(Component)]
pub(super) struct HpBar;
#[derive(Component)]
pub(super) struct HpLevelSeat;
#[derive(Component)]
pub(super) struct HpLevel;
#[derive(Component)]
pub(super) struct HpFill;
#[derive(Component)]
pub(super) struct ShieldFill;
#[derive(Component)]
pub(super) struct BannerText;
#[derive(Component)]
pub(super) struct AbilityRow;
#[derive(Component)]
pub(super) struct PassiveRow;
#[derive(Component)]
pub(super) struct OfferRow;
/// The line under NIVEAU SUPÉRIEUR: which level this is.
#[derive(Component)]
pub(super) struct LevelUpSub;
#[derive(Component)]
pub(super) struct LevelUpTitle;

/// A radial cooldown sweep over one ability icon: `sweep` is the remaining fraction
/// (1.0 = just cast, 0.0 = ready), drawn by `ui_sweep.wgsl`.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct SweepUiMaterial {
    #[uniform(0)]
    pub(super) sweep: f32,
}

impl UiMaterial for SweepUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_app/survivors/cooldown.wgsl".into()
    }
}

/// The cooldown overlay on one ability slot, driven every frame.
#[derive(Component)]
pub(super) struct AbilitySweep {
    pub(super) handle: Handle<SweepUiMaterial>,
    pub(super) index: usize,
}

/// The class mobility's action button, bottom centre over the XP bar; shown once it is learned.
#[derive(Component)]
pub(super) struct MobilitySlot;
#[derive(Component)]
pub(super) struct MobilityIcon;
#[derive(Component)]
pub(super) struct MobilitySweep(Handle<SweepUiMaterial>);
#[derive(Component)]
pub(super) struct OverTitle;
#[derive(Component)]
pub(super) struct OverStats;
#[derive(Component)]
pub(super) struct OverLine;
#[derive(Component)]
pub(super) struct OverStat(usize);

/// An icon slot filled from the spell's `SpellIcon` once the client data is up.
#[derive(Component)]
pub(super) struct SpellIcon(pub(super) u32);
#[derive(Component)]
pub(super) struct IconDone;

/// A FrameXML `Texture` region: a client art file, its `TexCoords` (left, right, top, bottom) and
/// vertex colour, drawn once the archives are readable.
#[derive(Component)]
pub(super) struct UiArt {
    path: &'static str,
    coords: [f32; 4],
    tint: Color,
}

impl UiArt {
    pub(super) fn crop(path: &'static str, coords: [f32; 4]) -> Self {
        Self {
            path,
            coords,
            tint: Color::WHITE,
        }
    }

    fn tinted(path: &'static str, tint: Color) -> Self {
        Self {
            path,
            coords: [0.0, 1.0, 0.0, 1.0],
            tint,
        }
    }
}

#[derive(Component)]
pub(super) struct ArtDone;

/// What an icon slot's tooltip describes: an index into the run's abilities or passives.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TipFor {
    Ability(usize),
    Passive(usize),
    Mobility,
}

/// The one `GameTooltip`, and the column its lines go in.
#[derive(Component)]
pub(super) struct GameTooltip;
#[derive(Component)]
pub(super) struct TipBody;

#[derive(Component, Clone, Copy, PartialEq)]
pub(super) enum UiAction {
    Resume,
    MainMenu,
    Quit,
}

/// The colour a button returns to when not hovered.
#[derive(Component)]
pub(super) struct BaseColor(Color);

#[derive(Clone, Copy)]
pub(super) enum Choice {
    NewAbility(&'static AbilityDef),
    Upgrade(usize),
    NewPassive(&'static PassiveDef),
    UpPassive(usize),
    Heal,
    SpellRelic(&'static str, super::relic_rules::RelicKind),
}

/// The level-up cards on offer.
#[derive(Resource, Default)]
pub(super) struct Offer {
    choices: Vec<Choice>,
}

impl Offer {
    pub(super) fn len(&self) -> usize {
        self.choices.len()
    }

    pub(super) fn choices(&self) -> &[Choice] {
        &self.choices
    }
}

#[derive(Resource)]
pub(super) struct HudFont(pub(super) Handle<Font>);

pub(super) fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::BLACK,
        },
    )
}

pub(super) fn text_bundle(
    font: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    text(font, s, size, color)
}

pub(super) fn centered_bundle(
    font: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    centered(font, s, size, color)
}

fn centered(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        text(font, s, size, color),
        TextLayout::new_with_justify(Justify::Center),
    )
}

fn button(action: UiAction, base: Color) -> impl Bundle {
    edged_button(action, base, EDGE)
}

fn edged_button(action: UiAction, base: Color, edge: Color) -> impl Bundle {
    (
        Button,
        action,
        BaseColor(base),
        BackgroundColor(base),
        BorderColor::all(edge),
    )
}

fn icon(spell: u32, size: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
        ImageNode {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            ..default()
        },
        SpellIcon(spell),
    )
}

fn spell_name(spells: Option<&crate::ui_action::Spells>, spell: u32, fallback: &str) -> String {
    if let Some(name) = super::localization::ability_name(fallback) {
        return name.into();
    }
    spells
        .and_then(|s| s.catalog.get(spell))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| fallback.replace('_', " "))
}

pub(super) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut sweeps_init: ResMut<Assets<SweepUiMaterial>>,
) {
    let font = crate::char_select::wow_font(&asset_server);
    let number_font: Handle<Font> = asset_server.load("mpq://Fonts/ARIALN.ttf");
    commands.insert_resource(HudFont(font.clone()));
    let f = &font;
    let map = MiniMap::new(&mut images);

    // --- The in-run HUD. ---
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(400),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            super::unit_frames::spawn(root, f);
            // Blizzard's experience strip sits immediately above its main action bar.
            root.spawn((
                XpBar,
                Interaction::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(43.0),
                    margin: UiRect::left(Val::Px(-512.0)),
                    width: Val::Px(1024.0),
                    height: Val::Px(13.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    XpFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::tinted(
                        "Interface\\TargetingFrame\\UI-StatusBar",
                        Color::srgb(0.58, 0.0, 0.55),
                    ),
                ));
                // `MainMenuXPBarTexture0`..`3`: the segment rules, 256×10 on the bar's top.
                for (i, top) in [0.792_968_75, 0.542_968_75, 0.292_968_75, 0.042_968_75]
                    .into_iter()
                    .enumerate()
                {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(256.0 * i as f32),
                            top: Val::Px(0.0),
                            width: Val::Px(256.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop(
                            "Interface\\MainMenuBar\\UI-MainMenuBar-Dwarf",
                            [0.0, 1.0, top, top + 0.039_062_5],
                        ),
                    ));
                }
                // `MainMenuBarExpText`, `TextStatusBarText` centred one unit up, shown on hover.
                bar.spawn(Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    top: Val::Px(-1.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|n| {
                    n.spawn((
                        XpText,
                        Text::new(""),
                        TextFont {
                            font: number_font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::BLACK,
                        },
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        Visibility::Hidden,
                    ));
                });
            });
            minimap::spawn_cluster(root, f, &map);
            // The mobility button: a 36-unit action slot in `UI-Quickslot2`, its key over it.
            root.spawn((
                MobilitySlot,
                ZIndex(2),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(5.0),
                    margin: UiRect::left(Val::Px(285.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    display: Display::None,
                    ..default()
                },
            ))
            .with_children(|col| {
                col.spawn((
                    Node {
                        width: Val::Px(MOBILITY_SIZE),
                        height: Val::Px(MOBILITY_SIZE),
                        ..default()
                    },
                    Interaction::default(),
                    TipFor::Mobility,
                ))
                .with_children(|slot| {
                    // `UI-Quickslot2` is the empty slot's 64×64 frame, the button 36 inside it.
                    slot.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-MOBILITY_SIZE * 0.42),
                            top: Val::Px(-MOBILITY_SIZE * 0.42),
                            width: Val::Px(MOBILITY_SIZE * 1.84),
                            height: Val::Px(MOBILITY_SIZE * 1.84),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop("Interface\\Buttons\\UI-Quickslot2", [0.0, 1.0, 0.0, 1.0]),
                    ));
                    slot.spawn((
                        MobilityIcon,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                            ..default()
                        },
                    ));
                    let handle = sweeps_init.add(SweepUiMaterial { sweep: 0.0 });
                    slot.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        MaterialNode(handle.clone()),
                        MobilitySweep(handle),
                    ));
                    slot.spawn(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(2.0),
                        top: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|key| {
                        key.spawn(text(f, "F", 12.0, Color::WHITE));
                    });
                });
            });
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(0.0),
                    width: Val::Px(1024.0),
                    height: Val::Px(53.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .insert((UiTransform::from_translation(Val2::new(
                    Val::Percent(-50.0),
                    Val::Px(0.0),
                )),))
                .with_children(|col| {
                    super::action_bar::spawn(col);
                    col.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(82.0),
                            ..default()
                        },
                        super::unit_frames::SpellBarTitle,
                        text(f, "Sorts automatiques", 12.0, GOLD),
                    ));
                    col.spawn((
                        AbilityRow,
                        ZIndex(1),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(8.0),
                            bottom: Val::Px(4.0),
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                    col.spawn((
                        PassiveRow,
                        ZIndex(1),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(600.0),
                            bottom: Val::Px(8.0),
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                });
                // The clock, with the level and the kill count on one line under it.
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(0.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((TimerText, centered(f, "00:00", 34.0, Color::WHITE)));
                    col.spawn(Node {
                        column_gap: Val::Px(14.0),
                        ..default()
                    })
                    .with_children(|line| {
                        line.spawn((LevelText, text(f, "Niveau 1", 14.0, GOLD)));
                        line.spawn((KillText, text(f, "Ennemis vaincus : 0", 14.0, Color::WHITE)));
                    });
                });
            });
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(22.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|n| {
                n.spawn((BannerText, centered(f, "", 30.0, GOLD)));
            });
            // The hero's health as a nameplate (`CGNamePlateFrame`): the bar under the border that
            // caps it, the level in the border's seat; [`hero_plate`] sizes and seats it.
            let absolute = || Node {
                position_type: PositionType::Absolute,
                ..default()
            };
            let unpainted = || ImageNode {
                color: Color::NONE,
                ..default()
            };
            let [r, g, b, _] = crate::vplates::PLATE_FRIENDLY;
            root.spawn((HpFrame, absolute(), Visibility::Hidden))
                .with_children(|plate| {
                    plate.spawn((HpBar, absolute())).with_children(|bar| {
                        bar.spawn((
                            HpFill,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            unpainted(),
                            UiArt::tinted(nameplate::BAR_FILL_TEXTURE, Color::srgb(r, g, b)),
                        ));
                        bar.spawn((
                            ShieldFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..absolute()
                            },
                            unpainted(),
                            UiArt::tinted(
                                nameplate::BAR_FILL_TEXTURE,
                                Color::srgba(0.85, 0.9, 1.0, 0.75),
                            ),
                        ));
                    });
                    plate.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..absolute()
                        },
                        unpainted(),
                        UiArt::crop(nameplate::BORDER_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
                    ));
                    let [r, g, b, _] = crate::vplates::CON_YELLOW;
                    plate
                        .spawn((
                            HpLevelSeat,
                            Node {
                                width: Val::Px(0.0),
                                height: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..absolute()
                            },
                        ))
                        .with_children(|seat| {
                            seat.spawn((
                                HpLevel,
                                Text::new("1"),
                                TextFont {
                                    font: f.clone(),
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(r, g, b)),
                                TextShadow {
                                    offset: Vec2::new(1.0, 1.0),
                                    color: Color::BLACK,
                                },
                                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                            ));
                        });
                });
        });
    commands.insert_resource(map);

    // --- The run's pick screen; [`super::menu`] builds into it once the glue art is in. ---
    commands.spawn((
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(410),
        bevy::ui::FocusPolicy::Pass,
    ));

    // --- Level up. ---
    commands
        .spawn((
            LevelUpRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            // A dark vignette closing in on the cards.
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.04, 0.025, 0.0, 0.72), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.9), 100.0),
                ],
            )),
            GlobalZIndex(420),
            Visibility::Hidden,
            // The build's icons stay hoverable under the cards.
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            // The slow golden burst behind everything.
            root.spawn((
                super::cards::Rays,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(1400.0),
                    height: Val::Px(1400.0),
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect::new(
                        Val::Px(-700.0),
                        Val::Px(0.0),
                        Val::Px(-700.0),
                        Val::Px(0.0),
                    ),
                    ..default()
                },
                BackgroundGradient(vec![
                    Gradient::Conic(ConicGradient::new(
                        UiPosition::CENTER,
                        (0..24)
                            .map(|i| {
                                let a = i as f32 / 24.0 * std::f32::consts::TAU;
                                let on = i % 2 == 0;
                                AngularColorStop::new(
                                    if on {
                                        Color::srgba(1.0, 0.8, 0.3, 0.07)
                                    } else {
                                        Color::srgba(1.0, 0.8, 0.3, 0.0)
                                    },
                                    a,
                                )
                            })
                            .collect(),
                    )),
                    Gradient::Radial(RadialGradient::new(
                        UiPosition::CENTER,
                        RadialGradientShape::ClosestSide,
                        vec![
                            ColorStop::percent(Color::srgba(1.0, 0.75, 0.25, 0.22), 0.0),
                            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.0), 100.0),
                        ],
                    )),
                ]),
                UiTransform::default(),
                bevy::ui::FocusPolicy::Pass,
            ));
            root.spawn((LevelUpTitle, centered(f, "NIVEAU SUPÉRIEUR", 52.0, GOLD)));
            root.spawn((
                LevelUpSub,
                centered(
                    f,
                    "Choisissez votre amélioration",
                    16.0,
                    Color::srgb(0.85, 0.78, 0.6),
                ),
            ));
            root.spawn((
                OfferRow,
                Node {
                    column_gap: Val::Px(22.0),
                    // Room for a hovered card's lift.
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(28.0), Val::Px(16.0)),
                    ..default()
                },
            ));
            root.spawn(centered(
                f,
                "Cliquez sur une carte ou appuyez sur 1, 2 ou 3",
                14.0,
                DIM,
            ));
        });

    // --- Pause. ---
    commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(430),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn(centered(f, "Pause", 46.0, GOLD));
            for (label, action) in [
                ("Reprendre", UiAction::Resume),
                ("Abandonner la partie", UiAction::MainMenu),
                ("Quitter le jeu", UiAction::Quit),
            ] {
                root.spawn((
                    button(action, BUTTON),
                    Node {
                        width: Val::Px(240.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn(text(f, label, 20.0, Color::WHITE));
                });
            }
        });

    // --- Results. ---
    commands
        .spawn((
            OverRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.07, 0.02, 0.0, 0.8), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.94), 100.0),
                ],
            )),
            GlobalZIndex(440),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((OverTitle, centered(f, "", 64.0, GOLD)));
            root.spawn((
                OverLine,
                centered(f, "", 18.0, Color::srgb(0.85, 0.78, 0.6)),
            ));
            // The run in numbers, one tile each.
            root.spawn(Node {
                column_gap: Val::Px(12.0),
                margin: UiRect::vertical(Val::Px(14.0)),
                ..default()
            })
            .with_children(|row| {
                for (i, label) in [
                    "Temps de survie",
                    "Niveau",
                    "Ennemis vaincus",
                    "Dégâts",
                    "Bosses",
                ]
                .into_iter()
                .enumerate()
                {
                    row.spawn((
                        Node {
                            width: Val::Px(128.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(12.0)),
                            row_gap: Val::Px(4.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundGradient::from(LinearGradient::to_bottom(vec![
                            ColorStop::percent(Color::srgba(0.2, 0.14, 0.05, 0.92), 0.0),
                            ColorStop::percent(Color::srgba(0.04, 0.03, 0.02, 0.94), 100.0),
                        ])),
                        BorderColor::all(EDGE),
                    ))
                    .with_children(|tile| {
                        tile.spawn((OverStat(i), centered(f, "", 26.0, Color::WHITE)));
                        tile.spawn(centered(f, label, 12.0, DIM));
                    });
                }
            });
            root.spawn((OverStats, centered(f, "", 15.0, DIM)));
            root.spawn((
                edged_button(UiAction::MainMenu, BUTTON, GOLD),
                Node {
                    width: Val::Px(340.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(11.0)),
                    margin: UiRect::top(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BoxShadow::new(
                    Color::srgba(1.0, 0.7, 0.2, 0.35),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(16.0),
                ),
            ))
            .with_children(|b| {
                b.spawn(text(f, "Sélection du personnage", 20.0, GOLD));
            });
            root.spawn((
                button(UiAction::Quit, Color::NONE),
                Node {
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                    ..default()
                },
            ))
            .with_children(|b| {
                b.spawn(text(f, "Quitter le jeu", 15.0, DIM));
            });
        });
}

/// The menu's hero is the run's hero: respawned whenever the pick changes.
pub(super) fn menu_preview(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    heroes: Query<Entity, With<Hero>>,
    world: benilla_world::world_point::WorldPoint,
    rig: Res<hero::CamRig>,
    mut shown: Local<Option<(usize, u32, usize, [u8; 5], Phase)>>,
    create: Res<super::blizzard_create::LocalCreate>,
) {
    if !matches!(*phase.get(), Phase::Menu | Phase::Preparing) || create.active {
        return;
    }
    let want = selection
        .class
        .zip(selection.look.and_then(|i| selection.looks.get(i)).copied());
    let key = want.map(|(c, l)| (c, l.0, selection.stage, selection.appearance, *phase.get()));
    if key == *shown && (key.is_none() || !heroes.is_empty()) {
        return;
    }
    for e in &heroes {
        commands.entity(e).despawn();
    }
    *shown = key;
    let Some((class, (display, race, sex))) = want else {
        return;
    };
    let mut pos = if *phase.get() == Phase::Preparing {
        super::units::wow_to_bevy(super::newhub_sites::for_race(race).arrival.xyz)
    } else {
        hero::arena(selection.stage_def())
    };
    if *phase.get() != Phase::Preparing {
        if let Some(h) = world.terrain_height_under(pos) {
            pos.y = h;
        }
    }
    let serial = run.guid();
    let hero = hero::spawn_hero(
        &mut commands,
        &data::CLASSES[class],
        display,
        race,
        sex,
        serial,
        pos,
        selection.appearance,
    );
    // Keep the selection portrait facing the fixed menu camera, independent of the active run's
    // orbit state.
    let camera_dir = Vec3::new(rig.orbit.sin(), 0.0, rig.orbit.cos());
    commands.entity(hero).insert(
        Transform::from_translation(pos)
            .with_rotation(Quat::from_rotation_y(super::units::yaw_toward(camera_dir))),
    );
}

pub(super) fn start_run(run: &mut Run, selection: &mut Selection) -> bool {
    let (Some(class), Some(&(_, race, _))) = (
        selection.class_def(),
        selection.look.and_then(|i| selection.looks.get(i)),
    ) else {
        return false;
    };
    // The world goes to the battleground picked; until now it stood in the last one.
    selection.stage = selection.map;
    let next_guid = run.next_guid;
    *run = Run {
        class,
        race,
        next_guid,
        ..Run::default()
    };
    run.abilities.push(Owned {
        def: data::ability(class.start),
        level: 1,
        cd: 0.5,
        talent_mods: Default::default(),
    });
    run.hp = run.stats().max_hp;
    run.version += 1;
    true
}

fn choices_for(run: &Run, rng: &mut Rng) -> Vec<Choice> {
    let mut pool: Vec<(Choice, f32)> = Vec::new();
    for (i, o) in run.abilities.iter().enumerate() {
        if o.level < data::MAX_ABILITY_LEVEL {
            pool.push((Choice::Upgrade(i), 1.2));
        }
    }
    if run.abilities.len() < data::MAX_ABILITIES {
        for key in run.class.pool {
            if !run.abilities.iter().any(|o| o.def.key == *key)
                && !run.locked_talent_spells.contains(&data::ability(key).spell)
            {
                pool.push((Choice::NewAbility(data::ability(key)), 1.0));
            }
        }
    }
    for (i, (_, level)) in run.passives.iter().enumerate() {
        if *level < data::MAX_PASSIVE_LEVEL {
            pool.push((Choice::UpPassive(i), 0.8));
        }
    }
    if run.passives.len() < data::MAX_PASSIVES {
        for p in data::passives_for(run.class.id) {
            if !run.passives.iter().any(|(q, _)| q.key == p.key) {
                pool.push((Choice::NewPassive(p), 0.5));
            }
        }
    }
    let mut out = Vec::with_capacity(3);
    while out.len() < 3 && !pool.is_empty() {
        let total: f32 = pool.iter().map(|(_, w)| w).sum();
        let mut pick = rng.f32() * total;
        let mut idx = pool.len() - 1;
        for (i, (_, w)) in pool.iter().enumerate() {
            if pick < *w {
                idx = i;
                break;
            }
            pick -= w;
        }
        out.push(pool.swap_remove(idx).0);
    }
    if out.is_empty() {
        out.push(Choice::Heal);
    }
    out
}

fn chest_choices(run: &Run, rng: &mut Rng) -> Vec<Choice> {
    use super::relic_rules::RelicKind;
    let mut pool = Vec::new();
    for ability in &run.abilities {
        let def = ability.def;
        let damages = def.damage > 0.0 || def.dot > 0.0;
        pool.push(Choice::SpellRelic(def.key, RelicKind::Rune));
        if damages {
            pool.push(Choice::SpellRelic(def.key, RelicKind::Grimoire));
            pool.push(Choice::SpellRelic(def.key, RelicKind::Precision));
            if matches!(
                def.kind,
                Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Chain | Kind::Dot | Kind::Orbit
            ) {
                pool.push(Choice::SpellRelic(def.key, RelicKind::Focus));
            }
        }
    }
    let mut out = Vec::new();
    while out.len() < 3 && !pool.is_empty() {
        let i = rng.index(pool.len());
        out.push(pool.swap_remove(i));
    }
    // Every class starts with a damaging spell, so Rune + Grimoire + Precision guarantee
    // three meaningful choices without a heal or a fake damage bonus on utility spells.
    out
}

pub(super) fn roll_offer(run: Res<Run>, mut rng: ResMut<Rng>, mut offer: ResMut<Offer>) {
    offer.choices = if run.pending_chest.is_some() {
        chest_choices(&run, &mut rng)
    } else {
        choices_for(&run, &mut rng)
    };
}

pub(super) fn pick(
    i: usize,
    run: &mut Run,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
    economy: Option<&mut super::economy::Economy>,
) {
    let Some(&choice) = offer.choices.get(i) else {
        return;
    };
    if matches!(choice, Choice::SpellRelic(..)) {
        let Some(economy) = economy else { return };
        let price = super::economy_rules::chest_price(economy.opened);
        if economy.account.gold < price || run.pending_chest.is_none() {
            return;
        }
        economy.account.gold -= price;
        economy.opened = economy.opened.saturating_add(1);
    }
    match choice {
        Choice::NewAbility(def) => run.abilities.push(Owned {
            def,
            level: 1,
            cd: 0.3,
            talent_mods: Default::default(),
        }),
        Choice::Upgrade(k) => run.abilities[k].level += 1,
        Choice::NewPassive(p) => run.passives.push((p, 1)),
        Choice::UpPassive(k) => run.passives[k].1 += 1,
        Choice::Heal => run.hp = run.stats().max_hp,
        Choice::SpellRelic(key, kind) => {
            if run.pending_chest.is_some() {
                run.spell_relics.push(super::relic_rules::SpellRelic {
                    spell: key.into(),
                    kind,
                });
                run.pending_chest = None;
                run.banner = Some((
                    format!(
                        "{} : {} — {}",
                        kind.name(),
                        super::localization::ability_name(key).unwrap_or(key),
                        kind.desc()
                    ),
                    5.0,
                ));
                run.version += 1;
                offer.choices.clear();
                if run.pending_levels > 0 {
                    offer.choices = choices_for(run, rng);
                } else {
                    next.set(Phase::Playing);
                }
                return;
            }
        }
    }
    run.version += 1;
    run.pending_levels = run.pending_levels.saturating_sub(1);
    if run.pending_levels > 0 {
        offer.choices = choices_for(run, rng);
    } else {
        offer.choices.clear();
        next.set(Phase::Playing);
    }
}

pub(super) fn looks_for(
    class: &data::ClassDef,
    creatures: &crate::entities::Creatures,
) -> Vec<(u32, u8, u8)> {
    let mut out: Vec<(u32, u8, u8)> = Vec::new();
    for &d in class.looks {
        if let Some((race, sex)) = creatures.display_race_sex(d) {
            if !out.iter().any(|&(_, r, s)| r == race && s == sex) {
                out.push((d, race, sex));
            }
        }
    }
    out.sort_by_key(|&(_, r, s)| (r, s));
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn buttons(
    mut presses: Query<
        (&Interaction, &UiAction, &BaseColor, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next: ResMut<NextState<Phase>>,
    mut exit: MessageWriter<AppExit>,
    phase: Res<State<Phase>>,
) {
    for (interaction, action, base, mut bg) in &mut presses {
        match interaction {
            Interaction::Hovered => bg.0 = BUTTON_HOT,
            Interaction::None => bg.0 = base.0,
            Interaction::Pressed => {
                bg.0 = BUTTON_HOT;
                match (*action, *phase.get()) {
                    (UiAction::Resume, Phase::Paused) => next.set(Phase::Playing),
                    (UiAction::MainMenu, Phase::Paused | Phase::GameOver) => next.set(Phase::Menu),
                    (UiAction::Quit, Phase::Paused | Phase::GameOver) => {
                        exit.write(AppExit::Success);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(super) fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    saved: Res<super::character_save::CharacterSave>,
    mut economy: ResMut<super::economy::Economy>,
    settings: Res<super::settings::Settings>,
    create: Res<super::blizzard_create::LocalCreate>,
    splash: Res<super::startup_splash::StartupSplash>,
) {
    if settings.open || create.active || splash.0 {
        return;
    }
    match phase.get() {
        Phase::Menu => {
            if saved.editing_name || saved.name_confirmed_this_frame {
                return;
            }
            if keys.just_pressed(KeyCode::Enter)
                && selection.class_def().is_some()
                && selection.look.is_some()
            {
                next.set(Phase::Preparing);
            }
        }
        Phase::Preparing => {}
        Phase::Playing => {
            // Escape is handled by the genuine Blizzard GameMenu binding.
        }
        Phase::Paused => {
            if keys.just_pressed(KeyCode::Escape) && !settings.open {
                next.set(Phase::Playing);
            }
        }
        Phase::LevelUp => {}
        Phase::GameOver => {
            if keys.just_pressed(KeyCode::Enter) {
                next.set(Phase::Menu);
            }
        }
    }
}

/// The class's looks once the creature data is in, defaulting to the first. The auto-runner picks
/// a class with no look; the menu itself always sets both.
pub(super) fn refresh_menu(
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
) {
    if let (Some(class), Some(creatures)) = (selection.class_def(), creatures.as_deref()) {
        if selection.looks.is_empty() {
            let looks = looks_for(class, creatures);
            if !looks.is_empty() {
                selection.looks = looks;
                selection.look = Some(0);
                selection.version += 1;
            }
        }
    }
}

/// What a level-up card shows.
pub(super) struct CardText {
    pub spell: u32,
    pub name: String,
    pub tag: String,
    pub tag_color: Color,
    pub desc: String,
    /// `(rank now, top rank)`, for the pips; `None` for the healthstone.
    pub ranks: Option<(u8, u8)>,
}

pub(super) fn describe_card(
    choice: Choice,
    run: &Run,
    spells: Option<&crate::ui_action::Spells>,
) -> CardText {
    let new = Color::srgb(0.3, 1.0, 0.3);
    match choice {
        Choice::NewAbility(def) => {
            let stats = run.stats();
            let c = cast_at(def, 1, &stats, None, &run.spell_relics);
            CardText {
                spell: def.spell,
                name: spell_name(spells, def.spell, def.key),
                tag: "Nouveau sort".into(),
                tag_color: new,
                desc: format!("{}.", ability_body(def, &c)),
                ranks: Some((0, data::MAX_ABILITY_LEVEL)),
            }
        }
        Choice::Upgrade(k) => {
            let o = &run.abilities[k];
            let next = o.level + 1;
            let stats = run.stats();
            let (a, b) = (
                cast_at(
                    o.def,
                    o.level,
                    &stats,
                    Some(&o.talent_mods),
                    &run.spell_relics,
                ),
                cast_at(o.def, next, &stats, Some(&o.talent_mods), &run.spell_relics),
            );
            // What actually changes, as before → after.
            let mut lines = Vec::new();
            let (da, db) = if a.dmg > 0.0 {
                (a.dmg, b.dmg)
            } else {
                (a.dot, b.dot)
            };
            if db > da {
                lines.push(format!("Dégâts {} » {}", num(da), num(db)));
            }
            if b.cooldown < a.cooldown - 0.01 {
                lines.push(format!(
                    "Recharge {} s » {} s",
                    num(a.cooldown),
                    num(b.cooldown)
                ));
            }
            if b.count > a.count {
                lines.push(format!("Cibles {} » {}", a.count, b.count));
            }
            if b.radius > a.radius + 0.05 {
                lines.push(format!("Zone {} » {} m", num(a.radius), num(b.radius)));
            }
            if b.duration > a.duration + 0.05 {
                lines.push(format!(
                    "Durée {} s » {} s",
                    num(a.duration),
                    num(b.duration)
                ));
            }
            CardText {
                spell: o.def.spell,
                name: spell_name(spells, o.def.spell, o.def.key),
                tag: format!("Rang {} » {next}", o.level),
                tag_color: Color::WHITE,
                desc: lines.join("\n"),
                ranks: Some((o.level, data::MAX_ABILITY_LEVEL)),
            }
        }
        Choice::NewPassive(p) => CardText {
            spell: p.spell,
            name: super::localization::passive_name(p.key)
                .unwrap_or(p.name)
                .into(),
            tag: "Nouveau talent".into(),
            tag_color: new,
            desc: passive_text(p.stat, p.per_level),
            ranks: Some((0, data::MAX_PASSIVE_LEVEL)),
        },
        Choice::UpPassive(k) => {
            let (p, level) = run.passives[k];
            let fmt = |v: f32| match p.stat {
                Stat::Regen | Stat::Armor => num(v),
                _ => pct(v),
            };
            CardText {
                spell: p.spell,
                name: super::localization::passive_name(p.key)
                    .unwrap_or(p.name)
                    .into(),
                tag: format!("Rang {level} » {}", level + 1),
                tag_color: Color::WHITE,
                desc: format!(
                    "{}\n{} » {}",
                    stat_label(p.stat),
                    fmt(p.per_level * f32::from(level)),
                    fmt(p.per_level * f32::from(level + 1)),
                ),
                ranks: Some((level, data::MAX_PASSIVE_LEVEL)),
            }
        }
        Choice::Heal => CardText {
            spell: 5720,
            name: "Pierre de soins".into(),
            tag: "Soins".into(),
            tag_color: Color::WHITE,
            desc: "Toutes les améliorations sont au maximum. Rend tous vos points de vie.".into(),
            ranks: None,
        },
        Choice::SpellRelic(key, kind) => {
            let def = data::ability(key);
            CardText {
                spell: def.spell,
                name: kind.name().into(),
                tag: super::localization::ability_name(key).unwrap_or(key).into(),
                tag_color: Color::srgb(1.0, 0.62, 0.18),
                desc: kind.desc().into(),
                ranks: None,
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_hud(
    mut commands: Commands,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    font: Res<HudFont>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LevelText>>,
        Query<&mut Text, With<TimerText>>,
        Query<&mut Text, With<KillText>>,
        Query<&mut Text, With<BannerText>>,
        Query<&mut Text, With<OverTitle>>,
        Query<&mut Text, With<OverStats>>,
        Query<&mut Text, With<XpText>>,
    )>,
    mut xp_fill: Query<&mut Node, With<XpFill>>,
    rows: Query<(Entity, Has<AbilityRow>), Or<(With<AbilityRow>, With<PassiveRow>)>>,
    mut sweeps: ResMut<Assets<SweepUiMaterial>>,
    mut last_version: Local<Option<(u8, u32)>>,
) {
    let need = data::xp_to_next(run.level);
    if let Ok(mut n) = xp_fill.single_mut() {
        n.width = Val::Percent((run.xp / need * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut t) = texts.p6().single_mut() {
        let xp = format!("XP {} / {}", run.xp as u32, need as u32);
        if **t != xp {
            **t = xp;
        }
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        **t = format!("Niveau {}", run.level);
    }
    let secs = run.elapsed as u32;
    if let Ok(mut t) = texts.p1().single_mut() {
        **t = format!("{:02}:{:02}", secs / 60, secs % 60);
    }
    let kills = run.kills;
    if let Ok(mut t) = texts.p2().single_mut() {
        **t = format!("Ennemis vaincus : {kills}");
    }
    let banner = match run.banner.as_mut() {
        Some((msg, t)) => {
            *t -= time.delta_secs();
            if *t > 0.0 {
                msg.clone()
            } else {
                String::new()
            }
        }
        None => String::new(),
    };
    if banner.is_empty() {
        run.banner = None;
    }
    // The picker owns the screen: an announcement waits until it closes.
    let banner = if *phase.get() == Phase::LevelUp {
        String::new()
    } else {
        banner
    };
    if let Ok(mut t) = texts.p3().single_mut() {
        if **t != banner {
            **t = banner;
        }
    }
    let title = if run.won {
        "Victoire !"
    } else {
        "Vous êtes mort"
    }
    .to_string();
    if let Ok(mut t) = texts.p4().single_mut() {
        **t = title;
    }
    let summary = format!(
        "{} {} — {}   —   Entrée : sélection du personnage",
        super::localization::race_name(run.race),
        super::localization::class_name(run.class.id),
        super::localization::zone_name(selection.map_def().name)
            .unwrap_or(selection.map_def().name),
    );
    if let Ok(mut t) = texts.p5().single_mut() {
        if **t != summary {
            **t = summary;
        }
    }

    // The ability and passive icons, rebuilt when the build changes.
    let build = (run.class.id, run.version);
    if *last_version == Some(build) {
        return;
    }
    *last_version = Some(build);
    let f = &font.0;
    for (row, is_abilities) in &rows {
        commands.entity(row).despawn_related::<Children>();
        commands.entity(row).with_children(|row| {
            let entries: Vec<(u32, u8, f32, TipFor)> = if is_abilities {
                run.abilities
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (o.def.spell, o.level, 40.0, TipFor::Ability(i)))
                    .collect()
            } else {
                run.passives
                    .iter()
                    .enumerate()
                    .map(|(i, (p, l))| (p.spell, *l, 30.0, TipFor::Passive(i)))
                    .collect()
            };
            let filled = entries.len();
            for (spell, level, size, tip) in entries {
                let ability = match &tip {
                    TipFor::Ability(i) => Some(*i),
                    _ => None,
                };
                row.spawn((
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(EDGE),
                    Interaction::default(),
                    tip,
                ))
                .with_children(|slot| {
                    slot.spawn(icon(spell, size - 2.0));
                    // The cooldown sweep sits over the icon, under the level text.
                    if let Some(i) = ability {
                        let handle = sweeps.add(SweepUiMaterial { sweep: 0.0 });
                        slot.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            MaterialNode(handle.clone()),
                            AbilitySweep { handle, index: i },
                        ));
                        slot.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(2.0),
                                top: Val::Px(0.0),
                                ..default()
                            },
                            text(f, (i + 1).to_string(), 12.0, Color::WHITE),
                        ));
                    }
                    slot.spawn(Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(2.0),
                        bottom: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|n| {
                        n.spawn(text(f, level.to_string(), 12.0, Color::WHITE));
                    });
                });
            }
            if is_abilities {
                for _ in filled..12 {
                    row.spawn((
                        Node {
                            width: Val::Px(40.0),
                            height: Val::Px(40.0),
                            ..default()
                        },
                        ImageNode::default(),
                        UiArt::crop(
                            "Interface\\Buttons\\UI-Quickslot2",
                            [0.21875, 0.78125, 0.21875, 0.78125],
                        ),
                    ));
                }
            }
        });
    }
}

/// Fill icon slots from `SpellIcon.dbc` once the client data and the asset reader are up.
pub(super) fn fill_icons(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &SpellIcon), Without<IconDone>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<HashMap<u32, Handle<Image>>>,
) {
    let (Some(spells), Some(assets)) = (spells.as_deref(), assets.as_deref_mut()) else {
        return;
    };
    for (e, mut img, icon) in &mut slots {
        let handle = match cache.get(&icon.0) {
            Some(h) => Some(h.clone()),
            None => {
                let path = spells
                    .catalog
                    .get(icon.0)
                    .and_then(|d| d.icon.clone())
                    .unwrap_or_else(|| "Interface\\Icons\\INV_Misc_QuestionMark".into());
                let h = assets.sprite_texture(&format!("{path}.blp"), &mut images);
                if let Some(h) = &h {
                    cache.insert(icon.0, h.clone());
                }
                h
            }
        };
        if let Some(h) = handle {
            img.image = h;
            img.color = Color::WHITE;
        }
        commands.entity(e).insert(IconDone);
    }
}

/// The mobility button: shown once learned, its icon the class's spell, its sweep the cooldown.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_mobility(
    run: Res<Run>,
    mut commands: Commands,
    mut slot: Query<&mut Node, With<MobilitySlot>>,
    icon: Query<(Entity, Option<&SpellIcon>), With<MobilityIcon>>,
    sweep: Query<&MobilitySweep>,
    mut mats: ResMut<Assets<SweepUiMaterial>>,
) {
    let def = run.mobility();
    if let Ok(mut node) = slot.single_mut() {
        let display = if def.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some(def) = def else { return };
    // A new spell re-arms the icon fill ([`fill_icons`]).
    if let Ok((e, have)) = icon.single() {
        if have.is_none_or(|s| s.0 != def.spell) {
            commands
                .entity(e)
                .remove::<IconDone>()
                .insert(SpellIcon(def.spell));
        }
    }
    if let Ok(s) = sweep.single() {
        let f = (run.mobile.cd / def.cooldown.max(0.01)).clamp(0.0, 1.0);
        if let Some(m) = mats.get_mut(&s.0) {
            if m.sweep != f {
                m.sweep = f;
            }
        }
    }
}

/// Drive each ability icon's cooldown sweep from its remaining cooldown.
pub(super) fn update_cooldown_sweeps(
    run: Res<Run>,
    mut mats: ResMut<Assets<SweepUiMaterial>>,
    sweeps: Query<&AbilitySweep>,
) {
    let stats = run.stats();
    for s in &sweeps {
        let Some(o) = run.abilities.get(s.index) else {
            continue;
        };
        let relic_cooldown: f32 = super::relic_rules::for_spell(&run.spell_relics, o.def.key)
            .map(|r| r.cooldown_mult())
            .product();
        let total = (o.cooldown(&stats) * relic_cooldown).max(0.2);
        let f = (o.cd / total).clamp(0.0, 1.0);
        if let Some(m) = mats.get_mut(&s.handle) {
            m.sweep = f;
        }
    }
}

/// Draw each [`UiArt`] region once its file can be read.
pub(super) fn fill_art(
    mut commands: Commands,
    mut regions: Query<(Entity, &mut ImageNode, &UiArt), Without<ArtDone>>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut assets) = assets else { return };
    for (e, mut img, art) in &mut regions {
        commands.entity(e).insert(ArtDone);
        let Some(h) = assets.sprite_texture(art.path, &mut images) else {
            warn!("survivors: no UI art {}", art.path);
            continue;
        };
        let [l, r, t, b] = art.coords;
        if let Some(size) = images.get(&h).map(Image::size_f32) {
            img.rect = Some(Rect::new(l * size.x, t * size.y, r * size.x, b * size.y));
        }
        img.image = h;
        img.color = art.tint;
    }
}

/// The reference lays its UI out on a screen 768 units tall at any resolution (`useUiScale` off).
pub(super) fn apply_ui_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    create: Res<super::blizzard_create::LocalCreate>,
) {
    let Ok(window) = windows.single() else { return };
    let want = if create.active {
        1.0
    } else {
        window.height() / 768.0
    };
    if want > 0.0 && (scale.0 - want).abs() > 1e-4 {
        scale.0 = want;
    }
}

/// Between the hero's feet and the top of its plate's bar, in gx units.
const PLATE_FEET_GAP: f32 = 0.004;
/// A couple of screen pixels lower than the gap alone sets it.
const PLATE_NUDGE: f32 = 2.0;

/// The hero's nameplate under its feet, sized from the screen diagonal as [`crate::vplates`]
/// sizes a plate, outside the UI scale. The border's top half seats a name the hero has none of,
/// so the plate rises until its bar sits just under the feet.
#[allow(clippy::type_complexity)]
pub(super) fn hero_plate(
    run: Res<Run>,
    ui_scale: Res<UiScale>,
    hero: Query<&Transform, With<Hero>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut nodes: ParamSet<(
        Query<(&mut Node, &mut Visibility), With<HpFrame>>,
        Query<&mut Node, With<HpBar>>,
        Query<&mut Node, With<HpFill>>,
        Query<&mut Node, With<ShieldFill>>,
        Query<&mut Node, With<HpLevelSeat>>,
    )>,
    mut level: Query<(&mut Text, &mut TextFont), With<HpLevel>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    let seat = hero.single().ok().and_then(|h| {
        let (cam, tf) = cams.single().ok()?;
        let viewport = cam.logical_viewport_size()?;
        let feet = cam
            .world_to_viewport(&GlobalTransform::from(*tf), h.translation)
            .ok()?;
        Some((feet, viewport))
    });
    let mut frames = nodes.p0();
    let Ok((mut frame, mut vis)) = frames.single_mut() else {
        return;
    };
    let Some((feet, viewport)) = seat else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Inherited;
    let s = ui_scale.0;
    let basis = plate_basis(viewport);
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = (gx(PLATE_W), gx(PLATE_H));
    let bar_top = ph - gx(BAR_OFF_Y) - gx(BAR_H);
    frame.width = Val::Px(pw);
    frame.height = Val::Px(ph);
    frame.left = Val::Px(feet.x / s - pw / 2.0);
    frame.top = Val::Px(feet.y / s + gx(PLATE_FEET_GAP) - bar_top + PLATE_NUDGE / s);
    if let Ok(mut bar) = nodes.p1().single_mut() {
        bar.left = Val::Px(gx(BAR_OFF_X));
        bar.top = Val::Px(bar_top);
        bar.width = Val::Px(gx(BAR_W));
        bar.height = Val::Px(gx(BAR_H));
    }
    let stats = run.stats();
    let hp = (run.hp / stats.max_hp).clamp(0.0, 1.0);
    let shield = if run.shield_t > 0.0 {
        (run.shield / stats.max_hp).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if let Ok(mut n) = nodes.p2().single_mut() {
        n.width = Val::Percent(hp * 100.0);
    }
    if let Ok(mut n) = nodes.p3().single_mut() {
        n.width = Val::Percent(shield * 100.0);
    }
    if let Ok(mut seat) = nodes.p4().single_mut() {
        seat.left = Val::Px(pw - gx(LEVEL_OFF_X));
        seat.top = Val::Px(ph - gx(LEVEL_OFF_Y));
    }
    if let Ok((mut text, mut font)) = level.single_mut() {
        let n = run.level.to_string();
        if text.0 != n {
            text.0 = n;
        }
        let size = text_px(LEVEL_H, basis) / s;
        if font.font_size != size {
            font.font_size = size;
        }
    }
}

/// `MainMenuExpBar`'s `OnEnter` and `OnLeave`: the numbers show while the cursor is on the bar.
pub(super) fn xp_hover(
    bars: Query<&Interaction, (With<XpBar>, Changed<Interaction>)>,
    mut texts: Query<&mut Visibility, With<XpText>>,
) {
    for interaction in &bars {
        let want = if *interaction == Interaction::None {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        for mut v in &mut texts {
            *v = want;
        }
    }
}

// ── The build icons' `GameTooltip` ──────────────────────────────────────────────────────────────

/// `TOOLTIP_DEFAULT_BACKGROUND_COLOR`, the `GameTooltip` backdrop tint (`GameTooltip.lua`).
const TIP_FILL: Color = Color::srgb(0.09, 0.09, 0.19);
/// The grey a spell tooltip prints its rank in, right of the name.
const TIP_GREY: Color = Color::srgb(0.5, 0.5, 0.5);

/// One tooltip line: the left text, an optional right-aligned one, the colour and font size
/// (`GameTooltipHeaderText` 14, `GameTooltipText` 12).
struct TipLine {
    left: String,
    right: Option<(String, Color)>,
    color: Color,
    size: f32,
}

impl TipLine {
    fn new(left: impl Into<String>, color: Color) -> Self {
        Self {
            left: left.into(),
            right: None,
            color,
            size: 12.0,
        }
    }

    fn right(mut self, text: impl Into<String>, color: Color) -> Self {
        self.right = Some((text.into(), color));
        self
    }

    fn header(mut self) -> Self {
        self.size = 14.0;
        self
    }
}

/// A number as the tooltips print it: whole when near enough, else to one decimal.

/// The results screen's tiles and its line under the title.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_results(
    run: Res<Run>,
    selection: Res<Selection>,
    mut over_texts: ParamSet<(
        Query<(&OverStat, &mut Text)>,
        Query<&mut Text, With<OverLine>>,
    )>,
) {
    let secs = run.elapsed as u32;
    let stats_line = if run.won {
        "Vous avez survécu à tous les assauts."
    } else {
        "Les ennemis ont eu raison de vous cette fois."
    };
    let values = [
        format!("{:02}:{:02}", secs / 60, secs % 60),
        run.level.to_string(),
        run.kills.to_string(),
        compact(run.damage_done),
        run.boss_kills.to_string(),
    ];
    for (stat, mut t) in &mut over_texts.p0() {
        if let Some(v) = values.get(stat.0) {
            if **t != *v {
                **t = v.clone();
            }
        }
    }
    if let Ok(mut t) = over_texts.p1().single_mut() {
        if **t != stats_line {
            **t = stats_line.to_string();
        }
    }
}

/// 12.3k, 4.5M.
fn compact(v: f64) -> String {
    if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e4 {
        format!("{:.1}k", v / 1e3)
    } else {
        format!("{}", v.round() as u64)
    }
}

fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn pct(v: f32) -> String {
    format!("{}%", num(v * 100.0))
}

/// The numbers an ability casts with at a level, as [`super::abilities::cast_abilities`] takes
/// them from [`Owned`].
struct Cast {
    dmg: f32,
    dot: f32,
    count: u32,
    radius: f32,
    duration: f32,
    cooldown: f32,
    crit: f32,
}

fn cast_at(
    def: &'static AbilityDef,
    level: u8,
    stats: &Stats,
    mods: Option<&super::talent_effects::SpellModifiers>,
    relics: &[super::relic_rules::SpellRelic],
) -> Cast {
    let o = Owned {
        def,
        level,
        cd: 0.0,
        talent_mods: mods.cloned().unwrap_or_default(),
    };
    let kinds: Vec<_> = super::relic_rules::for_spell(relics, def.key).collect();
    let damage_mult: f32 = kinds.iter().map(|r| r.damage_mult()).product();
    let cooldown_mult: f32 = kinds.iter().map(|r| r.cooldown_mult()).product();
    let extra_targets: u32 = kinds.iter().map(|r| r.extra_targets()).sum();
    let crit_bonus: f32 = kinds.iter().map(|r| r.crit_bonus()).sum();
    let mult = o.damage_mult() * damage_mult * stats.might;
    let damage = o.talent_mods.apply_damage(def.damage * mult);
    Cast {
        dmg: if matches!(
            def.kind,
            data::Kind::Ground | data::Kind::GroundSelf | data::Kind::Aura
        ) {
            o.talent_mods.dot.apply(damage)
        } else {
            damage
        },
        dot: o.talent_mods.apply_dot(def.dot * mult),
        count: o.count().saturating_add(extra_targets),
        radius: o.radius(stats),
        duration: o.duration(),
        cooldown: (o.cooldown(stats) * cooldown_mult).max(0.2),
        crit: (o.talent_mods.apply_crit(stats.crit) + crit_bonus).min(0.9),
    }
}

fn effect_text(effect: Effect) -> Option<String> {
    Some(match effect {
        Effect::None => return None,
        Effect::Slow(m, t) => format!("les ralentit de {} pendant {} s", pct(1.0 - m), num(t)),
        Effect::Root(t) => format!("les immobilise pendant {} s", num(t)),
        Effect::Stun(t) => format!("les étourdit pendant {} s", num(t)),
        Effect::Fear(t) => format!("les fait fuir pendant {} s", num(t)),
        Effect::Weaken(m, t) => format!(
            "réduit leurs dégâts de {} pendant {} s",
            pct(1.0 - m),
            num(t)
        ),
        Effect::Drain(f) => format!("vous soigne à hauteur de {} des dégâts infligés", pct(f)),
        Effect::Execute(f) => format!("les tue instantanément sous {} de vie", pct(f)),
    })
}

/// What an ability does at these numbers, in the voice of a spell description.
fn ability_body(def: &AbilityDef, c: &Cast) -> String {
    let d = num(c.dmg);
    let burst = def.duration <= 0.0;
    let mut s = match def.kind {
        Kind::Bolt | Kind::Strike | Kind::Melee => {
            let toughest = matches!(
                def.key,
                "pyroblast" | "aimed_shot" | "exorcism" | "starfire"
            );
            let melee = def.kind == Kind::Melee;
            let who = match (toughest, melee, c.count) {
                (true, _, 1) => "l’ennemi le plus résistant à portée".to_string(),
                (true, _, n) => format!("les {n} ennemis les plus résistants à portée"),
                (false, true, 1) => "l’ennemi le plus proche".into(),
                (false, true, n) => format!("les {n} ennemis les plus proches"),
                (false, false, 1) => "l’ennemi le plus proche".into(),
                (false, false, n) => format!("les {n} ennemis les plus proches"),
            };
            let mut s = format!(
                "{} {who} et inflige {d} points de dégâts",
                if melee { "Frappe" } else { "Touche" }
            );
            if c.radius > 0.0 && !melee {
                s += &format!(
                    ", ainsi que les ennemis à moins de {} m du point d’impact",
                    num(c.radius)
                );
            }
            s
        }
        Kind::Nova => format!(
            "Inflige {d} points de dégâts à chaque ennemi à moins de {} m de vous",
            num(c.radius)
        ),
        Kind::Cone => format!(
            "Inflige {d} points de dégâts à chaque ennemi dans un cône de {} m dirigé vers le plus proche",
            num(c.radius)
        ),
        Kind::Ground | Kind::GroundSelf => {
            let at = if def.kind == Kind::GroundSelf {
                "autour de vous"
            } else {
                "sur le groupe le plus dense"
            };
            if burst {
                format!(
                    "Frappe une zone de {} m {at} et inflige {d} points de dégâts",
                    num(c.radius)
                )
            } else {
                format!(
                    "Couvre une zone de {} m {at} pendant {} s et inflige {d} points de dégâts chaque seconde",
                    num(c.radius),
                    num(c.duration)
                )
            }
        }
        Kind::Chain => format!(
            "Inflige {d} points de dégâts à l’ennemi le plus proche, puis rebondit vers {} autres ennemis distants de moins de {} m ; chaque rebond réduit les dégâts de 15 %",
            c.count,
            num(c.radius)
        ),
        Kind::Dot => format!(
            "Inflige à {} ennemis à portée {} points de dégâts chaque seconde pendant {} s",
            c.count,
            num(c.dot),
            num(c.duration)
        ),
        Kind::Aura => format!(
            "Toutes les {} s, inflige {d} points de dégâts à chaque ennemi à moins de {} m de vous",
            num(c.cooldown),
            num(c.radius)
        ),
        Kind::Orbit => format!(
            "Toutes les {} s, frappe {} à moins de {} m de vous et inflige {d} points de dégâts",
            num(c.cooldown),
            if c.count == 1 {
                "un ennemi".to_string()
            } else {
                format!("{} ennemis", c.count)
            },
            num(c.radius)
        ),
        Kind::Shield => format!(
            "Absorbe les prochains {d} points de dégâts subis pendant {} s",
            num(c.duration)
        ),
        Kind::Immune => format!(
            "Lorsqu’un ennemi s’approche, vous rend insensible à tous les dégâts pendant {} s",
            num(c.duration)
        ),
        Kind::Thorns => format!(
            "Lorsque les ennemis s’approchent, inflige {d} points de dégâts à chaque attaquant pendant {} s",
            num(c.duration)
        ),
        Kind::Heal => format!(
            "Sous 85 % de vie, vous rend {d} points de vie en {} s",
            num(c.duration)
        ),
        Kind::Totem if def.key == "magma_totem" => format!(
            "Pose pendant {} s un totem qui inflige {d} points de dégâts toutes les 2 s à chaque ennemi à moins de {} m",
            num(c.duration),
            num(c.radius)
        ),
        Kind::Totem => format!(
            "Pose pendant {} s un totem qui inflige {d} points de dégâts chaque seconde à l’ennemi le plus proche à moins de {} m",
            num(c.duration),
            num(def.range)
        ),
    };
    // Areas that tick carry no over-time part; a burst leaves a 5 second one.
    let ground = matches!(def.kind, Kind::Ground | Kind::GroundSelf);
    if c.dot > 0.0 && def.kind != Kind::Dot && (!ground || burst) {
        let secs = if ground { 5.0 } else { c.duration };
        s += &format!(
            ", puis {} points de dégâts chaque seconde pendant {} s",
            num(c.dot),
            num(secs)
        );
    }
    if let Some(e) = effect_text(def.effect) {
        s += ", ";
        s += &e;
    }
    s.push('.');
    s
}

/// What the next rank changes, at the current stats.
fn next_rank(def: &AbilityDef, now: &Cast, next: &Cast) -> String {
    let what = match def.kind {
        Kind::Shield => "points absorbés",
        Kind::Heal => "points de soins",
        _ => "points de dégâts",
    };
    let mut gains = Vec::new();
    if next.dmg > 0.0 && def.kind != Kind::Dot && def.kind != Kind::Immune {
        gains.push(format!("{} {what}", num(next.dmg)));
    }
    if next.dot > 0.0 {
        gains.push(format!("{} points de dégâts par seconde", num(next.dot)));
    }
    if next.count != now.count
        && matches!(
            def.kind,
            Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Dot | Kind::Chain | Kind::Orbit
        )
    {
        gains.push(match def.kind {
            Kind::Chain => format!("{} rebonds", next.count),
            _ => format!("{} cibles", next.count),
        });
    }
    if next.radius > 0.0 {
        gains.push(format!("{} m", num(next.radius)));
    }
    if next.duration > 0.0 {
        gains.push(format!("{} s", num(next.duration)));
    }
    gains.push(format!("{} s de recharge", num(next.cooldown)));
    gains.join(", ")
}

fn ability_tip(
    o: &Owned,
    stats: &Stats,
    spells: Option<&crate::ui_action::Spells>,
    relics: &[super::relic_rules::SpellRelic],
) -> Vec<TipLine> {
    let def = o.def;
    let now = cast_at(def, o.level, stats, Some(&o.talent_mods), relics);
    let mut lines = vec![
        TipLine::new(spell_name(spells, def.spell, def.key), Color::WHITE)
            .header()
            .right(format!("Rang {}", o.level), TIP_GREY),
    ];
    let range = match def.kind {
        Kind::Melee => Some("Portée de mêlée".to_string()),
        Kind::Bolt | Kind::Strike | Kind::Chain | Kind::Dot | Kind::Ground | Kind::Totem => {
            Some(format!("Portée : {} m", num(def.range)))
        }
        _ => None,
    };
    let mut cast = TipLine::new("Lancement automatique", Color::WHITE);
    if let Some(range) = range {
        cast = cast.right(range, Color::WHITE);
    }
    lines.push(cast);
    lines.push(
        TipLine::new("Instantané", Color::WHITE)
            .right(format!("{} s de recharge", num(now.cooldown)), Color::WHITE),
    );
    lines.push(TipLine::new(ability_body(def, &now), GOLD));
    if !matches!(
        def.kind,
        Kind::Shield | Kind::Immune | Kind::Heal | Kind::Dot
    ) {
        lines.push(TipLine::new(
            format!(
                "{} de chances d’infliger un coup critique doublant les dégâts",
                pct(now.crit)
            ),
            DIM,
        ));
    }
    if o.level < data::MAX_ABILITY_LEVEL {
        let next = cast_at(def, o.level + 1, stats, Some(&o.talent_mods), relics);
        lines.push(TipLine::new("Rang suivant :", Color::WHITE));
        lines.push(TipLine::new(next_rank(def, &now, &next), GOLD));
    } else {
        lines.push(TipLine::new("Rang maximal", DIM));
    }
    lines
}

/// What a passive's bonus `v` does, in the voice of a buff.
/// A stat's short name, for the card's before » after line.
fn stat_label(stat: Stat) -> &'static str {
    match stat {
        Stat::Might => "Tous les dégâts",
        Stat::MaxHp => "Vie maximale",
        Stat::Speed => "Vitesse de déplacement",
        Stat::Regen => "Vie par seconde",
        Stat::Haste => "Vitesse de recharge",
        Stat::Area => "Taille des zones d’effet",
        Stat::Magnet => "Distance de ramassage",
        Stat::Growth => "Expérience gagnée",
        Stat::Armor => "Dégâts bloqués par coup",
        Stat::Crit => "Chances de coup critique",
    }
}

fn passive_text(stat: Stat, v: f32) -> String {
    match stat {
        Stat::Might => format!("Augmente tous vos dégâts de {}.", pct(v)),
        Stat::MaxHp => format!("Augmente votre vie maximale de {}.", pct(v)),
        Stat::Speed => format!("Augmente votre vitesse de déplacement de {}.", pct(v)),
        Stat::Regen => format!("Rend {} points de vie chaque seconde.", num(v)),
        Stat::Haste => format!("Vos sorts se rechargent {} plus vite.", pct(v)),
        Stat::Area => format!("Augmente la taille de vos zones d’effet de {}.", pct(v)),
        Stat::Magnet => format!("Augmente votre distance de ramassage de {}.", pct(v)),
        Stat::Growth => format!("Augmente vos gains d’expérience de {}.", pct(v)),
        Stat::Armor => format!("Réduit les dégâts de chaque coup reçu de {}.", num(v)),
        Stat::Crit => format!(
            "Augmente de {} vos chances d’infliger un coup critique doublant les dégâts.",
            pct(v)
        ),
    }
}

/// The hero's total for the stat a passive raises, every source counted.
fn stat_total(stat: Stat, s: &Stats) -> String {
    match stat {
        Stat::Might => format!("Dégâts infligés : {}", pct(s.might)),
        Stat::MaxHp => format!("Vie maximale: {}", num(s.max_hp)),
        Stat::Speed => format!("Vitesse de déplacement: {} m par seconde", num(s.speed)),
        Stat::Regen => format!("Vie rendue : {} par seconde", num(s.regen)),
        Stat::Haste => format!("Temps de recharge réduits de {}", pct(s.haste.min(0.65))),
        Stat::Area => format!("Zone d’effet : {}", pct(s.area)),
        Stat::Magnet => format!("Distance de ramassage: {} m", num(s.magnet)),
        Stat::Growth => format!("Expérience gagnée: {}", pct(s.growth)),
        Stat::Armor => format!("Dégâts bloqués par coup: {}", num(s.armor)),
        Stat::Crit => format!("Chances de coup critique: {}", pct(s.crit)),
    }
}

fn passive_tip(p: &PassiveDef, level: u8, stats: &Stats) -> Vec<TipLine> {
    let mut lines = vec![
        TipLine::new(
            super::localization::passive_name(p.key).unwrap_or(p.name),
            Color::WHITE,
        )
        .header()
        .right(format!("Rang {level}"), TIP_GREY),
        TipLine::new(passive_text(p.stat, p.per_level * f32::from(level)), GOLD),
        TipLine::new(stat_total(p.stat, stats), Color::WHITE),
    ];
    if level < data::MAX_PASSIVE_LEVEL {
        lines.push(TipLine::new("Rang suivant :", Color::WHITE));
        lines.push(TipLine::new(
            passive_text(p.stat, p.per_level * f32::from(level + 1)),
            GOLD,
        ));
    } else {
        lines.push(TipLine::new("Rang maximal", DIM));
    }
    lines
}

/// The `GameTooltip` frame: `UI-Tooltip-Background` tiled under `UI-Tooltip-Border`, 16-unit edges
/// and 5-unit insets, its lines 10 units in. Returns the frame and its line column.
fn spawn_tooltip(
    commands: &mut Commands,
    assets: &mut benilla_assets::WorldAssets,
    images: &mut Assets<Image>,
    ui_scale: f32,
) -> (Entity, Entity) {
    use crate::glue::backdrop::{backdrop_border, backdrop_edges, tiled_bg_node};
    let bg = assets.sprite_texture("Interface\\Tooltips\\UI-Tooltip-Background", images);
    let edges = backdrop_edges(assets, "Interface\\Tooltips\\UI-Tooltip-Border", images);
    let mut body = Entity::PLACEHOLDER;
    let mut root = commands.spawn((
        GameTooltip,
        Node {
            position_type: PositionType::Absolute,
            max_width: Val::Px(300.0),
            ..default()
        },
        GlobalZIndex(1000),
        Visibility::Hidden,
        bevy::ui::FocusPolicy::Pass,
    ));
    if bg.is_none() || edges.is_none() {
        root.insert(BackgroundColor(TIP_FILL.with_alpha(0.95)));
    }
    root.with_children(|t| {
        if let Some(bg) = bg {
            t.spawn((
                tiled_bg_node(bg, 16.0, ui_scale, TIP_FILL),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(5.0),
                    right: Val::Px(5.0),
                    top: Val::Px(5.0),
                    bottom: Val::Px(5.0),
                    ..default()
                },
            ));
        }
        if let Some(edges) = &edges {
            backdrop_border(t, edges, 16.0, Color::WHITE);
        }
        body = t
            .spawn((
                TipBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
            ))
            .id();
    });
    (root.id(), body)
}

/// Hovering a build icon raises its `GameTooltip` under the icon, every number from the current
/// rank and stats, and what the next rank brings.
#[allow(clippy::too_many_arguments)]
pub(super) fn icon_tooltip(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    spells: Option<Res<crate::ui_action::Spells>>,
    font: Res<HudFont>,
    ui_scale: Res<UiScale>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    slots: Query<(
        &Interaction,
        &TipFor,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut frames: Query<(&mut Node, &mut Visibility, &ComputedNode), With<GameTooltip>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut built: Local<Option<(Entity, Entity)>>,
    mut shown: Local<Option<(TipFor, u32)>>,
    stock: Res<super::blizzard_ui::StockUi>,
) {
    let Some((frame, body)) = *built else {
        if let Some(assets) = assets.as_deref_mut() {
            *built = Some(spawn_tooltip(
                &mut commands,
                assets,
                &mut images,
                ui_scale.0,
            ));
        }
        return;
    };
    let Ok((mut node, mut vis, frame_size)) = frames.get_mut(frame) else {
        return;
    };
    if stock.modal_open {
        *vis = Visibility::Hidden;
        *shown = None;
        return;
    }
    let hovered = slots
        .iter()
        .find(|(i, ..)| **i != Interaction::None)
        .filter(|_| *phase.get() != Phase::Menu);
    let Some((_, &tip, computed, at)) = hovered else {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
        *shown = None;
        return;
    };
    // Physical pixels to the 768-unit canvas the `Val`s are on.
    let inv = computed.inverse_scale_factor;
    let screen = windows
        .single()
        .map(|w| {
            [
                w.physical_width() as f32 * inv,
                w.physical_height() as f32 * inv,
            ]
        })
        .unwrap_or([1024.0, 768.0]);
    let [x, y] = super::layout_rules::tooltip_position(
        [
            at.translation.x * inv,
            at.translation.y * inv,
            computed.size().x * inv,
            computed.size().y * inv,
        ],
        [frame_size.size().x * inv, frame_size.size().y * inv],
        screen,
    );
    let (left, top) = (Val::Px(x), Val::Px(y));
    if node.left != left || node.top != top {
        node.left = left;
        node.top = top;
    }
    if *shown == Some((tip, run.version)) {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        return;
    }
    let stats = run.stats();
    let lines = match tip {
        TipFor::Ability(i) => run
            .abilities
            .get(i)
            .map(|o| ability_tip(o, &stats, spells.as_deref(), &run.spell_relics)),
        TipFor::Passive(i) => run
            .passives
            .get(i)
            .map(|&(p, level)| passive_tip(p, level, &stats)),
        TipFor::Mobility => run.mobility().map(|m| {
            vec![
                TipLine::new(spell_name(spells.as_deref(), m.spell, m.name), Color::WHITE).header(),
                TipLine::new("Appuyez sur F", Color::WHITE)
                    .right(format!("{} s de recharge", num(m.cooldown)), Color::WHITE),
                TipLine::new(m.desc, GOLD),
            ]
        }),
    };
    let Some(lines) = lines else { return };
    *shown = Some((tip, run.version));
    // Shown from the next frame, once the new lines have laid out.
    *vis = Visibility::Hidden;
    let f = &font.0;
    commands.entity(body).despawn_related::<Children>();
    commands.entity(body).with_children(|b| {
        for line in lines {
            let face = |s: String, size: f32, color: Color| {
                (
                    Text::new(s),
                    TextFont {
                        font: f.clone(),
                        font_size: size,
                        ..default()
                    },
                    TextColor(color),
                    TextShadow {
                        offset: Vec2::splat(1.0),
                        color: Color::BLACK,
                    },
                )
            };
            b.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(face(line.left, line.size, line.color));
                if let Some((text, color)) = line.right {
                    row.spawn((
                        face(text, line.size, color),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    ));
                }
            });
        }
    });
}

#[allow(clippy::type_complexity)]
pub(super) fn show_panels(
    phase: Res<State<Phase>>,
    run: Res<Run>,
    economy: Res<super::economy::Economy>,
    mut level_sub: Query<&mut Text, With<LevelUpSub>>,
    mut level_title: Query<&mut Text, With<LevelUpTitle>>,
    create: Res<super::blizzard_create::LocalCreate>,
    stock: Res<super::blizzard_ui::StockUi>,
    mut roots: ParamSet<(
        Query<(&mut Visibility, &mut Node), With<MenuRoot>>,
        Query<(&mut Visibility, &mut Node), With<HudRoot>>,
        Query<(&mut Visibility, &mut Node), With<LevelUpRoot>>,
        Query<(&mut Visibility, &mut Node), With<PauseRoot>>,
        Query<(&mut Visibility, &mut Node), With<OverRoot>>,
    )>,
) {
    let p = *phase.get();
    if let Ok(mut text) = level_sub.single_mut() {
        let desired = if run.pending_chest.is_some() {
            format!(
                "Choisissez une relique de sort — coût : {} or",
                super::economy_rules::chest_price(economy.opened)
            )
        } else {
            "Choisissez votre amélioration".into()
        };
        if **text != desired {
            **text = desired;
        }
    }
    if let Ok(mut text) = level_title.single_mut() {
        let desired = if run.pending_chest.is_some() {
            "COFFRE AU TRÉSOR"
        } else {
            "NIVEAU SUPÉRIEUR"
        };
        if **text != desired {
            **text = desired.into();
        }
    }
    // A hidden panel leaves the layout too: a merely invisible button still takes clicks.
    let set = |(mut v, mut node): (Mut<Visibility>, Mut<Node>), on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    for root in &mut roots.p0() {
        set(
            root,
            p == Phase::Menu && !create.active && !stock.modal_open,
        );
    }
    for root in &mut roots.p1() {
        set(
            root,
            !matches!(p, Phase::Menu | Phase::Preparing | Phase::GameOver)
                && !stock.modal_open
                && !create.active,
        );
    }
    for root in &mut roots.p2() {
        set(
            root,
            p == Phase::LevelUp && !stock.modal_open && !create.active,
        );
    }
    for root in &mut roots.p3() {
        set(
            root,
            p == Phase::Paused && !stock.ready && !stock.modal_open && !create.active,
        );
    }
    for root in &mut roots.p4() {
        set(
            root,
            p == Phase::GameOver && !stock.modal_open && !create.active,
        );
    }
}
