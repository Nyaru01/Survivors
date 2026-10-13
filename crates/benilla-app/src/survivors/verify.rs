//! Opt-in, isolated ECS integration fixture. Never sends keyboard or mouse input to Windows.
use super::{
    character_save, data, economy, enemies, hero, hud, menu, settings, Phase, Run, Selection,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::path::PathBuf;

#[derive(Resource)]
struct Probe {
    home: PathBuf,
    ui: bool,
    resume: bool,
    step: u8,
    age: f32,
    wait: f32,
    expected: Option<super::save_format::Session>,
    account: Option<super::economy_rules::Account>,
    report: String,
    created: Option<benilla_protocol::CharCreateReq>,
}
pub(super) fn plugin(app: &mut App) {
    let Some(mode) = crate::webenv::var("WOW_SURVIVORS_VERIFY") else {
        return;
    };
    if !matches!(mode.as_str(), "seed" | "resume" | "ui") {
        return;
    }
    let Some(home) = crate::local_state::home() else {
        return;
    };
    if crate::local_state::read_to_string(&home.join("survivors-probe.marker"))
        .ok()
        .as_deref()
        .map(str::trim)
        != Some("isolated validation")
    {
        warn!("Survivors verification refused: missing isolated profile marker.");
        return;
    }
    if crate::webenv::var("WOW_SURVIVORS_AUTO").is_some() {
        return;
    }
    let _ = std::fs::create_dir_all(home.join("shots"));
    app.insert_resource(Probe {
        home,
        ui: mode == "ui",
        resume: mode == "resume",
        step: 0,
        age: 0.0,
        wait: 0.0,
        expected: None,
        account: None,
        report: String::new(),
        created: None,
    })
    .add_systems(
        Update,
        drive
            .after(hud::show_panels)
            .after(menu::refresh_menu_ui)
            .after(character_save::resume)
            .before(hud::icon_tooltip)
            .run_if(|p: Res<Probe>| !p.ui),
    )
    .add_systems(
        Update,
        drive_creation
            .after(menu::refresh_menu_ui)
            .run_if(|p: Res<Probe>| p.ui),
    );
}
fn capture(commands: &mut Commands, probe: &Probe, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            probe.home.join("shots").join(format!("{name}.png")),
        ));
}
fn finish(probe: &Probe, exit: &mut MessageWriter<AppExit>, success: bool, detail: &str) {
    let text = format!(
        "STATUS={}\n{}\n{}\n",
        if success { "PASS" } else { "FAIL" },
        probe.report,
        detail
    );
    let mode = if probe.ui {
        "ui"
    } else if probe.resume {
        "resume"
    } else {
        "seed"
    };
    let _ = crate::local_state::write_atomic(
        &probe.home.join(format!("verification-{mode}.txt")),
        &text,
    );
    info!("Survivors verification {mode}: {text}");
    exit.write(AppExit::Success);
}

#[derive(SystemParam)]
struct CreationActors<'w, 's> {
    local: ResMut<'w, super::blizzard_create::LocalCreate>,
    next: ResMut<'w, NextState<super::blizzard_create::CreateScreen>>,
    state: Res<'w, State<super::blizzard_create::CreateScreen>>,
    native: ResMut<'w, crate::char_create::CreateSelection>,
    preview: ResMut<'w, crate::portrait::GluePreview>,
    portraits: Res<'w, crate::portrait::PortraitImages>,
    stock: ResMut<'w, super::blizzard_ui::StockUi>,
    pause_overlay: Query<'w, 's, &'static mut Visibility, With<hud::PauseRoot>>,
}

/// Captures actual rendered stock creation and exercises the five native dial operations.
/// The caller must provide the same isolated-profile marker as the save/resume fixture.
fn drive_creation(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
    mut economy: ResMut<economy::Economy>,
    saved: Res<character_save::CharacterSave>,
    mut actors: CreationActors,
    mut exit: MessageWriter<AppExit>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    script: Option<NonSendMut<benilla_ui::script::UiScript>>,
) {
    probe.age += time.delta_secs();
    probe.wait += time.delta_secs();
    if *phase.get() == Phase::Paused {
        for mut visibility in &mut actors.pause_overlay {
            *visibility = Visibility::Hidden;
        }
    }
    if probe.step == 255 {
        return;
    }
    if probe.age > 180.0 {
        finish(&probe, &mut exit, false, "Creation fixture timed out.");
        probe.step = 255;
        return;
    }
    let (Some(creatures), Some(catalog)) = (creatures.as_deref(), catalog.as_deref()) else {
        return;
    };
    if probe.step == 0 {
        if *phase.get() != Phase::Menu || loading.covering() || probe.wait < 3.0 {
            return;
        }
        let class = data::CLASSES.iter().position(|c| c.id == 9).unwrap();
        let looks = hud::looks_for(&data::CLASSES[class], creatures);
        let Some(look) = looks.iter().position(|&(_, r, s)| r == 1 && s == 0) else {
            return;
        };
        selection.class = Some(class);
        selection.looks = looks;
        selection.look = Some(look);
        selection.name = "Nyarutest".into();
        selection.appearance = [0; 5];
        selection.version += 1;
        probe.account = Some(economy.account.clone());
        probe.expected = saved.suspended.clone();
        super::blizzard_create::open(&selection, &mut actors.local, &mut actors.next);
        probe.step = 100;
        probe.wait = 0.0;
    } else if probe.step == 100 {
        if *actors.state.get() != super::blizzard_create::CreateScreen::Creating
            || !actors.local.active
            || probe.wait < 6.0
        {
            return;
        }
        if !matches!(
            actors.portraits.0.get(crate::portrait::GLUE_SLOT),
            Some(crate::portrait::PortraitSource::Live(_))
        ) {
            return;
        }
        let request = actors.native.offline_probe_snapshot();
        if request.name != "Nyarutest"
            || request.race != 1
            || request.class != 9
            || request.gender != 0
        {
            finish(
                &probe,
                &mut exit,
                false,
                "Native creation selection did not receive the offline profile.",
            );
            probe.step = 255;
            return;
        }
        capture(&mut commands, &probe, "creation-blizzard");
        probe
            .report
            .push_str("Stock creation screen active: Human male Warlock, native name field.\n");
        probe.step = 101;
        probe.wait = 0.0;
    } else if (101..=105).contains(&probe.step) && probe.wait > 2.5 {
        let dial = usize::from(probe.step - 101);
        let before = actors.native.offline_probe_snapshot();
        let look = actors.native.offline_probe_cycle(catalog, dial);
        actors.preview.look = Some(crate::portrait::GlueLook::Create(look));
        let after = actors.native.offline_probe_snapshot();
        let counts = catalog
            .0
            .ranges(after.race, after.gender)
            .map(|r| [r.skin, r.face, r.hair_style, r.hair_color, r.facial_hair])
            .unwrap_or([1; 5]);
        let old = [
            before.skin,
            before.face,
            before.hair_style,
            before.hair_color,
            before.facial_hair,
        ];
        let new = [
            after.skin,
            after.face,
            after.hair_style,
            after.hair_color,
            after.facial_hair,
        ];
        if new[dial] != (old[dial] + 1) % counts[dial].max(1) {
            finish(
                &probe,
                &mut exit,
                false,
                "Native appearance dial did not cycle within its catalog range.",
            );
            probe.step = 255;
            return;
        }
        probe.report.push_str(&format!(
            "Native dial {dial}: {} -> {} (range {}).\n",
            old[dial], new[dial], counts[dial]
        ));
        probe.step += 10;
        probe.wait = 0.0;
    } else if (111..=115).contains(&probe.step) && probe.wait > 3.0 {
        capture(
            &mut commands,
            &probe,
            &format!("creation-dial-{}", probe.step - 111),
        );
        probe.step = if probe.step == 115 {
            106
        } else {
            probe.step - 9
        };
        probe.wait = 0.0;
    } else if probe.step == 106 && probe.wait > 2.0 {
        probe.created = Some(actors.native.offline_probe_snapshot());
        actors.local.accepted = Some(actors.native.offline_probe_snapshot());
        probe.step = 107;
        probe.wait = 0.0;
    } else if probe.step == 107
        && probe.wait > 3.0
        && *actors.state.get() == super::blizzard_create::CreateScreen::Closed
    {
        let expected = probe.created.as_ref().unwrap();
        let saved_run_matches =
            toml::to_string(&probe.expected).ok() == toml::to_string(&saved.suspended).ok();
        let valid = selection.name == expected.name
            && selection
                .class_def()
                .is_some_and(|c| c.id == expected.class)
            && selection
                .look
                .and_then(|i| selection.looks.get(i))
                .is_some_and(|&(_, r, s)| r == expected.race && s == expected.gender)
            && selection.appearance
                == [
                    expected.skin,
                    expected.face,
                    expected.hair_style,
                    expected.hair_color,
                    expected.facial_hair,
                ]
            && probe
                .account
                .as_ref()
                .is_some_and(|a| *a == economy.account)
            && saved_run_matches;
        if !valid {
            finish(
                &probe,
                &mut exit,
                false,
                "Acceptance changed profile incorrectly or altered account/suspended adventure.",
            );
            probe.step = 255;
            return;
        }
        capture(&mut commands, &probe, "creation-return-selection");
        probe.report.push_str("Acceptance preserved account and suspended adventure; native appearance transferred to profile.\n");
        probe.step = 108;
        probe.wait = 0.0;
    } else if probe.step == 108 && probe.wait > 3.0 {
        economy.new_run();
        economy.account.gold = 75;
        economy.account.potions = 2;
        economy.account.gear = [1, 2, 1, 1];
        if !hud::start_run(&mut run, &mut selection) {
            finish(
                &probe,
                &mut exit,
                false,
                "Could not start native UI capture adventure.",
            );
            probe.step = 255;
            return;
        }
        run.gear = economy.account.gear;
        run.hp = run.stats().max_hp;
        run.immune_t = 600.0;
        next.set(Phase::Playing);
        probe.step = 109;
        probe.wait = 0.0;
    } else if probe.step == 109
        && *phase.get() == Phase::Playing
        && !loading.covering()
        && probe.wait > 3.0
    {
        if !actors.stock.errors.is_empty() {
            finish(
                &probe,
                &mut exit,
                false,
                &format!("Blizzard UI load errors: {:?}", actors.stock.errors),
            );
            probe.step = 255;
            return;
        }
        if !actors.stock.ready {
            return;
        }
        actors.stock.merchant_requested = true;
        next.set(Phase::Paused);
        probe.step = 110;
        probe.wait = 0.0;
    } else if probe.step >= 110 && probe.wait > 3.0 {
        let Some(script) = script else {
            return;
        };
        capture_stock_windows(
            &mut commands,
            &mut probe,
            &mut actors.stock,
            &script,
            &mut exit,
        );
    }
}

fn capture_stock_windows(
    commands: &mut Commands,
    probe: &mut Probe,
    stock: &mut super::blizzard_ui::StockUi,
    script: &benilla_ui::script::UiScript,
    exit: &mut MessageWriter<AppExit>,
) {
    let (frame, shot) = match probe.step {
        110 => ("MerchantFrame", "merchant-blizzard"),
        211 => ("CharacterFrame", "character-inventory-blizzard"),
        121 => ("GameMenuFrame", "menu-blizzard"),
        131 => ("OptionsFrame", "options-graphics-blizzard"),
        141 => ("SoundOptionsFrame", "options-sound-blizzard"),
        151 => ("KeyBindingFrame", "options-controls-blizzard"),
        160 => {
            finish(probe, exit, true, "Actual framebuffer captures written; visual fidelity still requires inspecting the PNG files.");
            probe.step = 255;
            return;
        }
        _ => ("", ""),
    };
    if !frame.is_empty() {
        let visible = script
            .eval::<bool>(&format!("return {frame} and {frame}:IsVisible() or false"))
            .unwrap_or(false);
        let errors = script.errors();
        let backpack_visible = frame != "CharacterFrame"
            || script
                .eval::<bool>("return IsBagOpen(0) ~= nil")
                .unwrap_or(false);
        if !visible || !backpack_visible || !errors.is_empty() {
            finish(
                probe,
                exit,
                false,
                &format!("{frame}: visible={visible}, backpack_visible={backpack_visible}, Lua errors={errors:?}"),
            );
            probe.step = 255;
            return;
        }
        capture(commands, probe, shot);
        probe.report.push_str(&format!(
            "{frame} visible; Lua error list empty; captured {shot}.\n"
        ));
        probe.step = match probe.step {
            110 => 210,
            211 => 120,
            121 => 130,
            131 => 140,
            141 => 150,
            _ => 160,
        };
        probe.wait = 0.0;
        return;
    }
    let command = match probe.step {
        210 => {
            stock.merchant_requested = false;
            stock.menu_requested = false;
            probe.step = 211;
            "HideUIPanel(MerchantFrame); CloseAllWindows(); ToggleCharacter('PaperDollFrame'); OpenBackpack()"
        }
        120 => {
            stock.merchant_requested = false;
            stock.menu_requested = true;
            probe.step = 121;
            "CloseAllWindows(); HideUIPanel(CharacterFrame); CloseAllBags()"
        }
        130 => {
            stock.menu_requested = false;
            probe.step = 131;
            "HideUIPanel(GameMenuFrame); ShowUIPanel(OptionsFrame)"
        }
        140 => {
            probe.step = 141;
            "HideUIPanel(OptionsFrame); ShowUIPanel(SoundOptionsFrame)"
        }
        150 => {
            probe.step = 151;
            "HideUIPanel(SoundOptionsFrame); ShowUIPanel(KeyBindingFrame)"
        }
        _ => return,
    };
    if let Err(error) = script.run(command) {
        finish(
            probe,
            exit,
            false,
            &format!("Window command failed: {command}: {error}"),
        );
        probe.step = 255;
        return;
    }
    probe.wait = 0.0;
}
#[derive(SystemParam)]
struct Actors<'w, 's> {
    visuals: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static benilla_world::model_fade::ModelAlpha>,
            Has<crate::entities::VisualAttached>,
        ),
        Or<(With<hero::Hero>, With<enemies::Enemy>)>,
    >,
    hero: Query<'w, 's, &'static Transform, With<hero::Hero>>,
    enemies: Query<'w, 's, &'static enemies::Enemy>,
    tips: Query<'w, 's, (&'static hud::TipFor, &'static mut Interaction)>,
    pause_overlay: Query<'w, 's, &'static mut Visibility, With<hud::PauseRoot>>,
}
#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut economy: ResMut<economy::Economy>,
    mut saved: ResMut<character_save::CharacterSave>,
    mut settings: ResMut<settings::Settings>,
    creatures: Option<Res<crate::entities::Creatures>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut actors: Actors,
    mut exit: MessageWriter<AppExit>,
) {
    probe.age += time.delta_secs();
    probe.wait += time.delta_secs();
    if *phase.get() == Phase::Paused {
        for mut visibility in &mut actors.pause_overlay {
            *visibility = Visibility::Hidden;
        }
    }
    if probe.age > 180.0 {
        finish(
            &probe,
            &mut exit,
            false,
            "Timed out waiting for assets or phase.",
        );
        return;
    }
    if probe.step == 0 {
        if *phase.get() != Phase::Menu || loading.covering() || probe.wait < 2.0 {
            return;
        }
        let (Some(creatures), Some(catalog)) = (creatures.as_deref(), catalog.as_deref()) else {
            return;
        };
        if probe.resume {
            let Some(session) = saved.suspended.clone() else {
                finish(&probe, &mut exit, false, "No suspended run loaded.");
                return;
            };
            probe.expected = Some(session);
            probe.account = Some(economy.account.clone());
            saved.resume_requested = true;
            probe.step = 20;
            probe.wait = 0.0;
            return;
        }
        let class = data::CLASSES.iter().position(|c| c.id == 9).unwrap();
        let looks = hud::looks_for(&data::CLASSES[class], creatures);
        let Some(look) = looks.iter().position(|&(_, r, s)| r == 1 && s == 0) else {
            return;
        };
        selection.class = Some(class);
        selection.looks = looks;
        selection.look = Some(look);
        selection.name = "Vérification".into();
        selection.appearance = [1; 5];
        character_save::clamp(&mut selection, catalog);
        selection.version += 1;
        probe.step = 1;
        probe.wait = 0.0;
    } else if probe.step == 1 && probe.wait > 3.0 {
        capture(&mut commands, &probe, "menu");
        probe.step = 2;
        probe.wait = 0.0;
    } else if probe.step == 2 && probe.wait > 1.0 {
        economy.new_run();
        economy.account.gold = 75;
        economy.account.potions = 2;
        economy.account.gear = [1, 2, 1, 1];
        if !hud::start_run(&mut run, &mut selection) {
            finish(
                &probe,
                &mut exit,
                false,
                "Could not start selected character.",
            );
            return;
        }
        run.gear = economy.account.gear;
        run.hp = run.stats().max_hp;
        run.immune_t = 600.0;
        run.elapsed = 361.0;
        run.bosses = 0;
        next.set(Phase::Playing);
        probe.step = 3;
        probe.wait = 0.0;
    } else if probe.step == 3 {
        if *phase.get() != Phase::Playing || loading.covering() || probe.wait < 2.0 {
            return;
        }
        let bosses = actors.enemies.iter().filter(|e| e.boss && !e.dead).count();
        if bosses < 2 {
            return;
        }
        let Ok(h) = actors.hero.single() else {
            return;
        };
        let pos = h.translation;
        economy::restore_chest(&mut commands, &mut run, pos + Vec3::X * 4.0);
        enemies::restore_gem(
            &mut commands,
            &mut run,
            enemies::GemKind::Gold(15),
            pos + Vec3::X * 8.0,
        );
        enemies::restore_gem(
            &mut commands,
            &mut run,
            enemies::GemKind::Xp(10),
            pos + Vec3::X * 8.5,
        );
        probe
            .report
            .push_str(&format!("Spawned bosses concurrently: {bosses}\n"));
        next.set(Phase::Paused);
        probe.step = 4;
        probe.wait = 0.0;
    } else if probe.step == 4 && probe.wait > 2.0 {
        capture(&mut commands, &probe, "gameplay");
        probe.step = 5;
        probe.wait = 0.0;
    } else if probe.step == 5 {
        for (tip, mut interaction) in &mut actors.tips {
            if matches!(tip, hud::TipFor::Ability(0)) {
                *interaction = Interaction::Hovered;
            }
        }
        if probe.wait > 3.0 {
            capture(&mut commands, &probe, "tooltip");
            probe.step = 50;
            probe.wait = 0.0;
        }
    } else if probe.step == 50 && probe.wait > 1.0 {
        settings.open = true;
        settings.tab = 0;
        probe.step = 6;
        probe.wait = 0.0;
    } else if matches!(probe.step, 6..=8) && probe.wait > 3.0 {
        let name = match probe.step {
            6 => "settings-graphics",
            7 => "settings-controls",
            _ => "settings-sound",
        };
        capture(&mut commands, &probe, name);
        probe.step = probe.step * 10;
        probe.wait = 0.0;
    } else if matches!(probe.step, 60 | 70 | 80) && probe.wait > 1.0 {
        if probe.step != 80 {
            settings.tab = probe.step / 10 - 5;
            probe.step = probe.step / 10 + 1;
        } else {
            settings.open = false;
            probe.step = 9;
        }
        probe.wait = 0.0;
    } else if probe.step == 9 && probe.wait > 3.0 {
        let bosses = actors.enemies.iter().filter(|e| e.boss && !e.dead).count();
        let path = probe.home.join("survivors-save.toml");
        let disk = crate::local_state::read_to_string(&path)
            .ok()
            .and_then(|t| super::save_format::decode(&t).ok());
        let valid = disk.as_ref().is_some_and(|s| {
            s.gold == economy.account.gold
                && s.gold >= 75
                && s.potions == 2
                && s.gear == [1, 2, 1, 1]
                && s.character.name == "Vérification"
                && s.run.as_ref().is_some_and(|r| {
                    r.bosses == 2
                        && r.enemies.iter().filter(|e| e.boss).count() == 2
                        && !r.chests.is_empty()
                        && r.pickups.iter().any(|p| p.kind == 2)
                })
        });
        finish(
            &probe,
            &mut exit,
            valid && bosses == 2,
            "Validated actual on-disk profile, equipment, pickups, chest and concurrent bosses.",
        );
        probe.step = 255;
    } else if probe.step == 20
        && *phase.get() == Phase::Paused
        && !loading.covering()
        && probe.wait > 3.0
    {
        let expected = probe.expected.as_ref().unwrap();
        let account = probe.account.as_ref().unwrap();
        let bosses = actors.enemies.iter().filter(|e| e.boss && !e.dead).count();
        let visible = actors
            .visuals
            .iter()
            .all(|(_, alpha, attached)| attached && alpha.is_none_or(|a| a.0 >= 0.99));
        let position = actors
            .hero
            .single()
            .map(|h| h.translation.distance(Vec3::from_array(expected.pos)) < 0.02)
            .unwrap_or(false);
        let valid = (run.elapsed - expected.elapsed).abs() < 0.001
            && (run.hp - expected.hp).abs() < 0.001
            && run.bosses == expected.bosses
            && bosses == 2
            && position
            && economy.account == *account
            && run.gear == account.gear
            && visible;
        capture(&mut commands, &probe, "resume");
        probe.report.push_str(&format!(
            "Restored elapsed={:.3}, hp={:.3}, bosses={}, gold={}, gear={:?}, position_match={}, visible_models={}\n",
            run.elapsed, run.hp, bosses, economy.account.gold, run.gear, position, visible
        ));
        probe.step = 21;
        probe.wait = 0.0;
        if !valid {
            finish(
                &probe,
                &mut exit,
                false,
                "Restored run differs from saved session.",
            );
            probe.step = 255;
        }
    } else if probe.step == 21 && probe.wait > 3.0 {
        finish(
            &probe,
            &mut exit,
            true,
            "Paused resume matches saved time, health, bosses, economy, equipment and position.",
        );
        probe.step = 255;
    }
}
