//! Gold pickups, treasure chests and a deliberately limited potion merchant.
use super::{economy_rules as rules, enemies, hero::Hero, units, Phase, Rng, Run};
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

#[derive(Resource)]
pub(super) struct Economy {
    pub account: rules::Account,
    pub stock: u8,
    pub opened: u32,
    pub next_chest: f32,
    pub potion_cd: f32,
}
impl Default for Economy {
    fn default() -> Self {
        Self {
            account: rules::Account::default(),
            stock: rules::POTION_LIMIT,
            opened: 0,
            next_chest: 10.0,
            potion_cd: 0.0,
        }
    }
}

impl Economy {
    /// Called only for a new adventure, never when resuming a saved run.
    pub fn new_run(&mut self) {
        self.stock = rules::POTION_LIMIT;
        self.opened = 0;
        self.next_chest = 10.0;
        self.potion_cd = 0.0;
    }
}

#[derive(Component)]
pub(super) struct Chest;
#[derive(Resource, Default)]
struct Shop {
    open: bool,
    message: String,
}
#[derive(Component)]
struct Wallet;
#[derive(Component)]
struct WalletRoot;
#[derive(Component)]
struct ShopPanel;
#[derive(Component)]
struct ShopText;
#[derive(Component)]
struct ChestPrompt;
#[derive(Component, Clone, Copy)]
enum Trade {
    Toggle,
    Buy,
    Drink,
    Open,
    Close,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Economy>()
        .init_resource::<Shop>()
        .add_systems(Startup, setup.after(super::hud::setup_hud))
        .add_systems(Update, trade.after(enemies::collect_gems))
        .add_systems(
            Update,
            (tick, spawn_chests)
                .chain()
                .after(enemies::collect_gems)
                .run_if(in_state(Phase::Playing))
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
        )
        .add_systems(Update, refresh.after(trade).after(spawn_chests));
}

pub(super) fn restore_chest(commands: &mut Commands, run: &mut Run, pos: Vec3) {
    let e = units::spawn_object(commands, run.guid(), 259, 1.0, pos);
    commands.entity(e).insert(Chest);
}

fn tick(time: Res<Time>, mut economy: ResMut<Economy>, mut run: ResMut<Run>) {
    economy.potion_cd = (economy.potion_cd - time.delta_secs()).max(0.0);
    run.gear = economy.account.gear;
}

fn spawn_chests(
    mut commands: Commands,
    mut economy: ResMut<Economy>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    hero: Query<&Transform, With<Hero>>,
    chests: Query<Entity, With<Chest>>,
    world: WorldPoint,
) {
    if run.dead_t.is_some() || run.elapsed < economy.next_chest {
        return;
    }
    economy.next_chest = run.elapsed + 60.0;
    if chests.iter().count() >= 3 {
        return;
    }
    let Ok(h) = hero.single() else {
        return;
    };
    let angle = rng.range(0.0, std::f32::consts::TAU);
    let mut pos = h.translation + Vec3::new(angle.cos(), 0.0, angle.sin()) * 7.0;
    pos.y = world.terrain_height_under(pos).unwrap_or(h.translation.y);
    restore_chest(&mut commands, &mut run, pos);
    run.banner = Some(("Un coffre au trésor est apparu près de vous !".into(), 4.0));
}

fn nearest_chest(hero: Vec3, chests: &Query<(Entity, &Transform), With<Chest>>) -> Option<Entity> {
    chests
        .iter()
        .filter_map(|(e, t)| {
            let distance = units::flat(t.translation - hero).length();
            (distance <= 3.0).then_some((e, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e)
}

#[allow(clippy::too_many_arguments)]
fn trade(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    character: Res<super::character_save::CharacterSave>,
    settings: Res<super::settings::Settings>,
    buttons: Query<(&Interaction, &Trade), Changed<Interaction>>,
    hero: Query<&Transform, With<Hero>>,
    chests: Query<(Entity, &Transform), With<Chest>>,
    mut economy: ResMut<Economy>,
    mut shop: ResMut<Shop>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
) {
    if settings.open {
        return;
    }
    let mut actions: Vec<Trade> = buttons
        .iter()
        .filter(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, t)| *t)
        .collect();
    // At character creation, P must remain available for the character's name.
    if !character.editing_name && keys.just_pressed(KeyCode::KeyP) {
        actions.push(Trade::Toggle);
    }
    if *phase.get() == Phase::Playing {
        if keys.just_pressed(KeyCode::KeyE) {
            actions.push(Trade::Open);
        }
        if keys.just_pressed(KeyCode::KeyH) {
            actions.push(Trade::Drink);
        }
    }
    for action in actions {
        match action {
            Trade::Toggle => shop.open = !shop.open,
            Trade::Close => shop.open = false,
            Trade::Buy => {
                let Economy { account, stock, .. } = &mut *economy;
                shop.message = match account.buy_potion(stock) {
                    Ok(()) => "Potion de soins achetée. H pour la boire en jeu.".into(),
                    Err(e) => e.into(),
                };
            }
            Trade::Drink => {
                if !matches!(*phase.get(), Phase::Playing | Phase::Paused) {
                    continue;
                }
                let max = run.stats().max_hp;
                let Economy {
                    account, potion_cd, ..
                } = &mut *economy;
                match account.drink(run.hp, max, potion_cd) {
                    Ok(hp) => {
                        run.hp = hp;
                        run.banner = Some(("Potion : 50 % de vie restaurée.".into(), 2.5));
                    }
                    Err(e) => run.banner = Some((e.into(), 2.5)),
                }
            }
            Trade::Open => {
                if *phase.get() != Phase::Playing || run.dead_t.is_some() {
                    continue;
                }
                let Some(e) = hero
                    .single()
                    .ok()
                    .and_then(|h| nearest_chest(h.translation, &chests))
                else {
                    continue;
                };
                let Economy {
                    account, opened, ..
                } = &mut *economy;
                let old_max = run.stats().max_hp;
                match account.open_chest(opened, rng.index(4)) {
                    Ok(slot) => {
                        run.gear = account.gear;
                        run.hp = (run.hp + (run.stats().max_hp - old_max).max(0.0))
                            .min(run.stats().max_hp);
                        run.banner = Some((
                            format!(
                                "{} +{} — {}",
                                rules::gear_name(run.class.id, slot),
                                account.gear[slot],
                                rules::gear_bonus(slot, account.gear[slot])
                            ),
                            5.0,
                        ));
                        commands.entity(e).despawn();
                    }
                    Err(err) => run.banner = Some((err.into(), 3.0)),
                }
            }
        }
    }
}

fn label(font: &Handle<Font>, value: &str, size: f32) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.0)),
    )
}
fn button(root: &mut ChildSpawnerCommands, font: &Handle<Font>, value: &str, trade: Trade) {
    root.spawn((
        Button,
        trade,
        Node {
            min_height: Val::Px(28.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.20, 0.05, 0.03, 0.95)),
        BorderColor::all(Color::srgb(0.65, 0.5, 0.2)),
    ))
    .with_children(|r| {
        r.spawn(label(font, value, 14.0));
    });
}
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let font = assets.load("mpq://Fonts/FRIZQT__.ttf");
    commands
        .spawn((
            WalletRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(112.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GlobalZIndex(600),
        ))
        .with_children(|r| {
            r.spawn((Wallet, label(&font, "Or : 0 | Potions : 0/3", 15.0)));
            button(r, &font, "P — Boutique et équipement", Trade::Toggle);
            button(r, &font, "H — Boire une potion", Trade::Drink);
        });
    commands.spawn((
        ChestPrompt,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(150.0),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        GlobalZIndex(600),
        label(&font, "", 17.0),
    ));
    // Right side avoids both character creation and the central settings dialog.
    commands
        .spawn((
            ShopPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Percent(25.0),
                width: Val::Px(310.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                display: Display::None,
                ..default()
            },
            BorderColor::all(Color::srgb(0.65, 0.5, 0.2)),
            BackgroundColor(Color::srgba(0.04, 0.03, 0.02, 0.97)),
            GlobalZIndex(600),
        ))
        .with_children(|r| {
            r.spawn((ShopText, label(&font, "", 15.0)));
            button(r, &font, "Acheter une potion — 20 or", Trade::Buy);
            button(r, &font, "Fermer", Trade::Close);
        });
}

#[allow(clippy::too_many_arguments)]
fn refresh(
    economy: Res<Economy>,
    settings: Res<super::settings::Settings>,
    shop: Res<Shop>,
    run: Res<Run>,
    phase: Res<State<Phase>>,
    selection: Res<super::Selection>,
    mut wallet: Query<&mut Text, (With<Wallet>, Without<ShopText>, Without<ChestPrompt>)>,
    mut body: Query<&mut Text, (With<ShopText>, Without<Wallet>, Without<ChestPrompt>)>,
    mut prompt: Query<&mut Text, (With<ChestPrompt>, Without<Wallet>, Without<ShopText>)>,
    mut panel: Query<&mut Node, (With<ShopPanel>, Without<WalletRoot>)>,
    mut wallet_root: Query<&mut Node, (With<WalletRoot>, Without<ShopPanel>)>,
    hero: Query<&Transform, With<Hero>>,
    chests: Query<(Entity, &Transform), With<Chest>>,
) {
    for mut node in &mut wallet_root {
        node.bottom = Val::Px(if *phase.get() == Phase::Menu {
            6.0
        } else {
            112.0
        });
    }
    let class = if *phase.get() == Phase::Menu {
        selection.class_def().unwrap_or(run.class)
    } else {
        run.class
    };
    for mut t in &mut wallet {
        t.0 = format!(
            "Or : {} | Potions : {}/3{}",
            economy.account.gold,
            economy.account.potions,
            if economy.potion_cd > 0.0 {
                format!(" ({:.0}s)", economy.potion_cd.ceil())
            } else {
                String::new()
            }
        );
    }
    for mut node in &mut panel {
        node.display = if shop.open && !settings.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    let mut text=format!("Boutique\nOr : {}\nPotions : {}/3 — Stock : {}/3\nSoigne 50 % de vie • recharge 20 s\n\nÉquipement (conservé)\n",economy.account.gold,economy.account.potions,economy.stock);
    for slot in 0..4 {
        text.push_str(&format!(
            "{} +{} : {}\n",
            rules::gear_name(class.id, slot),
            economy.account.gear[slot],
            rules::gear_bonus(slot, economy.account.gear[slot])
        ));
    }
    text.push_str(&format!("\n{}", shop.message));
    for mut t in &mut body {
        t.0 = text.clone();
    }
    let nearby = *phase.get() == Phase::Playing
        && hero
            .single()
            .ok()
            .and_then(|h| nearest_chest(h.translation, &chests))
            .is_some();
    for mut t in &mut prompt {
        t.0 = if nearby {
            format!(
                "E — Ouvrir le coffre : {} or",
                rules::chest_price(economy.opened)
            )
        } else {
            String::new()
        };
    }
}
