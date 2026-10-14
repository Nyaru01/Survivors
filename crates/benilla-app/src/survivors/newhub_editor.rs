//! Survivors-only camp authoring tool: place services at the player's grounded feet.
use super::{
    hero::Hero,
    newhub_layout_rules::{Layout, Pose},
    newhub_sites,
    newinteract::{Interactable, InteractableKind},
    prep::camp::{self, CampEntity},
    Phase, Selection,
};
use bevy::prelude::*;

#[derive(Resource)]
pub(crate) struct HubEditor {
    pub active: bool,
    selected: usize,
    layout: Layout,
    status: String,
}

impl Default for HubEditor {
    fn default() -> Self {
        let layout = crate::local_state::home()
            .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-hubs.txt")).ok())
            .and_then(|s| Layout::decode(&s))
            .unwrap_or_default();
        Self {
            active: false,
            selected: 0,
            layout,
            status: String::new(),
        }
    }
}

fn faction(site: &newhub_sites::FactionHub) -> usize {
    usize::from(site.faction == newhub_sites::HubFaction::Horde)
}

pub(crate) fn actor_index(kind: InteractableKind) -> Option<usize> {
    match kind {
        InteractableKind::Merchant => Some(0),
        InteractableKind::Artisan => Some(1),
        InteractableKind::Portal => Some(2),
        _ => None,
    }
}

fn original(site: &newhub_sites::FactionHub, actor: usize) -> Pose {
    let location = match actor {
        0 => site.merchant.location,
        1 => site.artisan.location,
        _ => site.portal.anchor,
    };
    let pos = super::units::wow_to_bevy(location.xyz);
    Pose {
        xyz: pos.to_array(),
        yaw: if actor == 2 {
            0.0
        } else {
            location.orientation
        },
    }
}

impl HubEditor {
    pub(crate) fn transform(&self, site: &newhub_sites::FactionHub, actor: usize) -> Transform {
        let fallback = original(site, actor);
        let arrival = super::units::wow_to_bevy(site.arrival.xyz);
        let pose = self.layout.0[faction(site)][actor]
            .filter(|p| Vec3::from_array(p.xyz).distance(arrival) <= 90.0)
            .unwrap_or(fallback);
        Transform::from_translation(Vec3::from_array(pose.xyz))
            .with_rotation(Quat::from_rotation_y(pose.yaw))
    }
}

#[derive(Component)]
struct EditorHelp;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HubEditor>()
        .add_systems(Startup, help)
        .add_systems(Update, edit.after(camp::walk));
}

fn help(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        EditorHelp,
        Text::new(""),
        TextFont {
            font: assets.load("mpq://Fonts/FRIZQT__.ttf"),
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.0)),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            top: Val::Px(130.0),
            max_width: Val::Px(540.0),
            padding: UiRect::all(Val::Px(12.0)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        GlobalZIndex(1500),
    ));
}

#[allow(clippy::too_many_arguments)]
fn edit(
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<Selection>,
    settings: Res<super::settings::Settings>,
    stock: Res<super::blizzard_ui::StockUi>,
    forge: Res<camp::ForgeState>,
    mut editor: ResMut<HubEditor>,
    hero: Query<&Transform, With<Hero>>,
    mut actors: Query<(&Interactable, &mut Transform), (With<CampEntity>, Without<Hero>)>,
    mut help: Query<(&mut Text, &mut Node), With<EditorHelp>>,
) {
    if *phase.get() != Phase::Preparing {
        editor.active = false;
    }
    let blocked = settings.open || stock.modal_open || forge.0;
    if *phase.get() == Phase::Preparing && !blocked && keys.just_pressed(KeyCode::F8) {
        editor.active = !editor.active;
        editor.status = "Positions non sauvegardées : F9 pour conserver.".into();
    }
    if editor.active && !blocked {
        if keys.just_pressed(KeyCode::F6) {
            editor.selected = (editor.selected + 1) % 3;
        }
        let site = newhub_sites::for_race(super::menu::selected_race(&selection).unwrap_or(1));
        let selected = editor.selected;
        for (actor, mut tf) in &mut actors {
            if actor_index(actor.kind) != Some(editor.selected) {
                continue;
            }
            let mut changed = false;
            if keys.just_pressed(KeyCode::F7) {
                if let Ok(player) = hero.single() {
                    if player
                        .translation
                        .distance(super::units::wow_to_bevy(site.arrival.xyz))
                        <= 90.0
                    {
                        tf.translation = player.translation;
                        tf.rotation = player.rotation;
                        changed = true;
                    }
                }
            }
            let turn = (keys.just_pressed(KeyCode::PageUp) as i8
                - keys.just_pressed(KeyCode::PageDown) as i8) as f32;
            if turn != 0.0 {
                tf.rotate_y(turn * std::f32::consts::PI / 12.0);
                changed = true;
            }
            if changed {
                editor.layout.0[faction(site)][selected] = Some(Pose {
                    xyz: tf.translation.to_array(),
                    yaw: tf.rotation.to_euler(EulerRot::YXZ).0,
                });
                editor.status = "Placement modifié — F9 pour sauvegarder.".into();
            }
            if keys.just_pressed(KeyCode::Delete) {
                editor.layout.0[faction(site)][selected] = None;
                *tf = editor.transform(site, editor.selected);
                editor.status = "Position d’origine rétablie — F9 pour sauvegarder.".into();
            }
        }
        if keys.just_pressed(KeyCode::F9) {
            editor.status = match crate::local_state::home() {
                Some(home) => match crate::local_state::write_atomic(
                    &home.join("survivors-hubs.txt"),
                    &editor.layout.encode(),
                ) {
                    Ok(()) => "Positions sauvegardées pour les prochaines sessions.".into(),
                    Err(error) => format!("Échec de sauvegarde : {error}"),
                },
                None => "Sauvegarde indisponible dans ce mode.".into(),
            };
        }
    }
    for (mut text, mut node) in &mut help {
        node.display = if *phase.get() == Phase::Preparing && !blocked {
            Display::Flex
        } else {
            Display::None
        };
        text.0 = if editor.active {
            format!("PLACEMENT DU CAMP — {}\nF6 : choisir | F7 : poser à vos pieds\nPage précédente/suivante : tourner de 15°\nSuppr : position d’origine | F9 : sauvegarder\nF8 : fermer | Déplacement et caméra habituels\n{}", ["Marchand", "Artisan", "Portail"][editor.selected], editor.status)
        } else {
            "F8 : placer les PNJ et le portail".into()
        };
    }
}
