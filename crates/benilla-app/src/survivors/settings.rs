//! Accessible graphics, controls and sound settings, in the menu or with F10 during play.
use super::{hud, settings_rules::Preferences, Phase};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};

#[derive(Resource)]
pub(super) struct Settings {
    pub open: bool,
    pub prefs: Preferences,
    pub(super) tab: u8,
    apply: bool,
    resized: bool,
    paused_by_us: bool,
}
impl Default for Settings {
    fn default() -> Self {
        let loaded = crate::local_state::home()
            .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-options.txt")).ok())
            .and_then(|t| Preferences::decode(&t));
        Self {
            open: false,
            prefs: loaded.unwrap_or_default(),
            tab: 0,
            apply: true,
            resized: loaded.is_some(),
            paused_by_us: false,
        }
    }
}
impl Settings {
    pub fn movement_keys(&self) -> [KeyCode; 4] {
        if self.prefs.azerty {
            [KeyCode::KeyZ, KeyCode::KeyS, KeyCode::KeyQ, KeyCode::KeyD]
        } else {
            [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD]
        }
    }
    pub fn mouse_factors(&self) -> [f32; 2] {
        [
            self.prefs.sensitivity,
            self.prefs.sensitivity * if self.prefs.invert_y { -1.0 } else { 1.0 },
        ]
    }
}
#[derive(Component)]
struct Open;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Blocker;
#[derive(Component)]
struct Page(u8);
#[derive(Component, Clone, Copy)]
enum Action {
    Toggle,
    Tab(u8),
    Distance(f32),
    Fullscreen,
    Vsync,
    Resolution,
    Polish,
    Layout,
    Sensitivity(f32),
    Invert,
    Volume(u8, f32),
}
#[derive(Component)]
struct Status(u8);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Settings>()
        .add_systems(Startup, setup.after(hud::setup_hud))
        .add_systems(Update, (input, refresh, apply).chain().after(hud::keys));
}
fn button(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str, action: Action) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.24, 0.06, 0.04)),
            BorderColor::all(Color::srgb(0.55, 0.45, 0.25)),
        ))
        .with_children(|p| {
            p.spawn(hud::text_bundle(
                font,
                label,
                14.0,
                Color::srgb(1.0, 0.82, 0.0),
            ));
        });
}
fn line(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    a: Action,
    b: Option<Action>,
) {
    parent
        .spawn(Node {
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|p| {
            button(p, font, label, a);
            if let Some(b) = b {
                button(p, font, "+", b);
            }
        });
}
fn setup(mut commands: Commands, font: Res<hud::HudFont>) {
    let f = &font.0;
    commands.spawn((
        Blocker,
        Button,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::None,
            ..default()
        },
        GlobalZIndex(1499),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
    ));
    commands
        .spawn((
            Open,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Px(12.0),
                ..default()
            },
            UiTransform::from_translation(Val2::new(Val::Percent(-50.0), Val::Px(0.0))),
            GlobalZIndex(1500),
        ))
        .with_children(|p| button(p, f, "Options — F10", Action::Toggle));
    commands.spawn((Panel,Node{position_type:PositionType::Absolute,left:Val::Percent(50.0),top:Val::Px(82.0),width:Val::Px(480.0),padding:UiRect::all(Val::Px(18.0)),border:UiRect::all(Val::Px(2.0)),flex_direction:FlexDirection::Column,row_gap:Val::Px(14.0),display:Display::None,..default()},
        UiTransform::from_translation(Val2::new(Val::Percent(-50.0),Val::Px(0.0))),GlobalZIndex(1501),BackgroundColor(Color::srgba(0.025,0.025,0.04,0.98)),BorderColor::all(Color::srgb(0.55,0.45,0.25))))
    .with_children(|p| {
        p.spawn(hud::text_bundle(f,"Paramètres",22.0,Color::srgb(1.0,0.82,0.0)));
        p.spawn(Node{column_gap:Val::Px(8.0),..default()}).with_children(|p|{for (i,n) in ["Graphismes","Contrôles","Son"].into_iter().enumerate() {button(p,f,n,Action::Tab(i as u8));}});
        for tab in 0..3 {p.spawn((Page(tab),Node{flex_direction:FlexDirection::Column,row_gap:Val::Px(12.0),..default()})).with_children(|p| {
            p.spawn((Status(tab),hud::text_bundle(f,"",14.0,Color::WHITE)));
            match tab {
                0=>{line(p,f,"Distance −",Action::Distance(-100.0),Some(Action::Distance(100.0)));line(p,f,"Plein écran / fenêtre",Action::Fullscreen,None);line(p,f,"Synchronisation verticale",Action::Vsync,None);line(p,f,"Changer la résolution",Action::Resolution,None);line(p,f,"Netteté et couleurs",Action::Polish,None);},
                1=>{line(p,f,"Clavier WASD / ZQSD",Action::Layout,None);line(p,f,"Sensibilité caméra −",Action::Sensitivity(-0.25),Some(Action::Sensitivity(0.25)));line(p,f,"Inverser la caméra verticale",Action::Invert,None);
                    p.spawn(hud::text_bundle(f,"Flèches : déplacement\nClic droit : caméra · Molette : zoom\n1–6 : sorts · F : mobilité\nE : coffre · P : boutique · H : potion\nÉchap : pause · F10 : options",14.0,Color::WHITE));},
                _=>{for (i,n) in ["Volume général −","Musique −","Effets −","Ambiance −"].into_iter().enumerate() {line(p,f,n,Action::Volume(i as u8,-0.1),Some(Action::Volume(i as u8,0.1)));}}
            }
        });}
        button(p,f,"Fermer",Action::Toggle);
    });
}
fn toggle(s: &mut Settings, phase: Phase, next: &mut NextState<Phase>) {
    s.open = !s.open;
    if s.open && phase == Phase::Playing {
        s.paused_by_us = true;
        next.set(Phase::Paused);
    } else if !s.open && s.paused_by_us {
        s.paused_by_us = false;
        next.set(Phase::Playing);
    }
}
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut settings: ResMut<Settings>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
) {
    if keys.just_pressed(KeyCode::F10) {
        toggle(&mut settings, *phase.get(), &mut next);
    }
    if settings.open && keys.just_pressed(KeyCode::Escape) {
        toggle(&mut settings, *phase.get(), &mut next);
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if matches!(action, Action::Toggle) {
            toggle(&mut settings, *phase.get(), &mut next);
            continue;
        }
        if !settings.open {
            continue;
        }
        let p = &mut settings.prefs;
        match *action {
            Action::Tab(t) => {
                settings.tab = t;
                continue;
            }
            Action::Distance(d) => p.farclip = (p.farclip + d).clamp(177.0, 777.0),
            Action::Fullscreen => {
                p.fullscreen = !p.fullscreen;
                settings.resized = true;
            }
            Action::Vsync => p.vsync = !p.vsync,
            Action::Resolution => {
                p.resolution = (p.resolution + 1) % 3;
                settings.resized = true;
            }
            Action::Polish => p.polish = !p.polish,
            Action::Layout => p.azerty = !p.azerty,
            Action::Sensitivity(d) => p.sensitivity = (p.sensitivity + d).clamp(0.25, 3.0),
            Action::Invert => p.invert_y = !p.invert_y,
            Action::Volume(i, d) => {
                let v = match i {
                    0 => &mut p.master,
                    1 => &mut p.music,
                    2 => &mut p.effects,
                    _ => &mut p.ambience,
                };
                *v = (*v + d).clamp(0.0, 1.0);
            }
            Action::Toggle => {}
        }
        settings.apply = true;
        if let Some(h) = crate::local_state::home() {
            if let Err(e) = crate::local_state::write_atomic(
                &h.join("survivors-options.txt"),
                &settings.prefs.encode(),
            ) {
                warn!("Options save: {e}");
            }
        }
    }
}
fn refresh(
    settings: Res<Settings>,
    phase: Res<State<Phase>>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<Open>>,
        Query<&mut Node, With<Panel>>,
        Query<(&Page, &mut Node)>,
        Query<&mut Node, With<Blocker>>,
    )>,
    mut labels: Query<(&Status, &mut Text)>,
) {
    for mut n in &mut nodes.p0() {
        n.display = if !settings.open && matches!(*phase.get(), Phase::Menu | Phase::Paused) {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut nodes.p1() {
        n.display = if settings.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (page, mut n) in &mut nodes.p2() {
        n.display = if page.0 == settings.tab {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut nodes.p3() {
        n.display = if settings.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    let p = settings.prefs;
    let on = |v| if v { "Activé" } else { "Désactivé" };
    for (s, mut t) in &mut labels {
        **t=match s.0 {
        0=>format!("Distance : {:.0} m\nÉcran : {}\nVSync : {} · Couleurs : {}\nRésolution fenêtre : {}",p.farclip,if p.fullscreen {"plein écran"} else {"fenêtre"},on(p.vsync),on(p.polish),["1280 × 720","1600 × 900","1920 × 1080"][p.resolution]),
        1=>format!("Clavier : {} · Sensibilité : {:.2}\nInversion verticale : {}",if p.azerty {"ZQSD"} else {"WASD"},p.sensitivity,on(p.invert_y)),
        _=>format!("Général : {:.0}% · Musique : {:.0}%\nEffets : {:.0}% · Ambiance : {:.0}%",p.master*100.0,p.music*100.0,p.effects*100.0,p.ambience*100.0),
    };
    }
}
fn apply(
    mut settings: ResMut<Settings>,
    mut sound: ResMut<crate::sound::SoundConfig>,
    mut view: ResMut<benilla_world::view::ViewDistance>,
    mut grade: ResMut<super::grade::GradeOn>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.apply {
        return;
    }
    settings.apply = false;
    let p = settings.prefs;
    sound.master = p.master;
    sound.music = p.music;
    sound.sfx = p.effects;
    sound.ambience = p.ambience;
    view.farclip = p.farclip;
    grade.0 = p.polish;
    if let Ok(mut w) = windows.single_mut() {
        w.present_mode = if p.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        if settings.resized || p.fullscreen {
            w.mode = if p.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            };
            if !p.fullscreen {
                let (width, height) = [(1280, 720), (1600, 900), (1920, 1080)][p.resolution];
                w.resolution.set(width as f32, height as f32);
            }
        }
    }
    settings.resized = false;
}
