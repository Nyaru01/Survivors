//! Opt-in native probe for the source-backed faction hubs. It uses an isolated profile marker,
//! Bevy input resources and the same projected-ray/range/occlusion helper as a world click.

use super::{data, hero, hud, newhub_sites, newinteract, Phase, Selection};
use benilla_ui::script::UiScript;
use bevy::ecs::system::SystemParam;
use bevy::input::{keyboard::KeyboardInput, ButtonState};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;
use std::path::PathBuf;

#[derive(Resource)]
struct Probe {
    home: PathBuf,
    faction: u8,
    service: u8,
    stand_attempt: u8,
    step: u8,
    age: f32,
    wait: f32,
    account_before: Option<super::economy_rules::Account>,
    initial_items: usize,
    initial_gold: u32,
    expected_position: Option<Vec3>,
    prepared_view: bool,
    original_resolution: Option<UVec2>,
    small_shot_taken: bool,
    small_resolution_set: bool,
    large_resolution_set: bool,
    portal_returning: bool,
    movement_started: bool,
    capture_logged: bool,
    action_state: u8,
    pending_capture: Option<PathBuf>,
    pending_capture_kind: u8,
    pending_capture_size: u64,
    pending_capture_stable: u8,
    service_shot_done: bool,
    panel_shot_started: bool,
    panel_shot_done: bool,
    report: String,
    editor_test: u8,
}

pub(super) fn plugin(app: &mut App) {
    if crate::webenv::var("WOW_SURVIVORS_VERIFY").as_deref() != Some("hub")
        || crate::webenv::var("WOW_SURVIVORS_AUTO").is_some()
    {
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
        warn!("Survivors hub verification refused: isolated profile marker is missing.");
        return;
    }
    let _ = std::fs::create_dir_all(home.join("shots"));
    app.insert_resource(Probe {
        home,
        faction: 0,
        service: 0,
        stand_attempt: 0,
        step: 0,
        age: 0.0,
        wait: 0.0,
        account_before: None,
        initial_items: 0,
        initial_gold: 0,
        expected_position: None,
        prepared_view: false,
        original_resolution: None,
        small_shot_taken: false,
        small_resolution_set: false,
        large_resolution_set: false,
        portal_returning: false,
        movement_started: false,
        capture_logged: false,
        action_state: 0,
        pending_capture: None,
        pending_capture_kind: 0,
        pending_capture_size: 0,
        pending_capture_stable: 0,
        service_shot_done: false,
        panel_shot_started: false,
        panel_shot_done: false,
        report: String::new(),
        editor_test: 0,
    })
    .add_systems(
        Update,
        drive
            .after(crate::ui_script::UiInput)
            .after(crate::target::TargetUpdate)
            .after(hero::place_camera)
            .after(crate::survivors::newinteract::HoverCursor)
            .after(hud::show_panels),
    );
}

#[derive(SystemParam)]
struct WorldSources<'w, 's> {
    targets: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static newinteract::Interactable,
        ),
    >,
    hero: Query<'w, 's, &'static mut Transform, With<hero::Hero>>,
    camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        With<benilla_world::view::WorldCamera>,
    >,
    map: Option<Res<'w, benilla_world::world_map::CurrentMap>>,
    window: Query<'w, 's, &'static mut Window, With<PrimaryWindow>>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    key_events: MessageWriter<'w, KeyboardInput>,
    rig: ResMut<'w, hero::CamRig>,
    collision: benilla_world::collision::WorldCollision<'w, 's>,
    loading: Res<'w, crate::loading_screen::LoadingScreen>,
    player: Res<'w, crate::player::Player>,
    economy: ResMut<'w, super::economy::Economy>,
    stock: Res<'w, super::blizzard_ui::StockUi>,
    editor: ResMut<'w, super::newhub_editor::HubEditor>,
}

fn finish(
    probe: &mut Probe,
    sources: &mut WorldSources,
    exit: &mut MessageWriter<AppExit>,
    good: bool,
    detail: &str,
) {
    if let Some(account) = probe.account_before.take() {
        sources.economy.account = account;
    }
    if let (Some(size), Ok(mut window)) = (probe.original_resolution, sources.window.single_mut()) {
        window.resolution.set(size.x as f32, size.y as f32);
    }
    let result = format!(
        "STATUS={}\n{}\n{}\n",
        if good { "PASS" } else { "FAIL" },
        probe.report,
        detail
    );
    let _ = crate::local_state::write_atomic(&probe.home.join("verification-hub.txt"), &result);
    info!("Survivors hub verification: {result}");
    probe.step = 255;
    exit.write(AppExit::Success);
}

fn capture(commands: &mut Commands, probe: &Probe, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            probe.home.join("shots").join(format!("{name}.png")),
        ));
}

fn request_capture(commands: &mut Commands, probe: &mut Probe, name: &str, kind: u8) {
    let path = probe.home.join("shots").join(format!("{name}.png"));
    // This is an isolated verification profile. Removing only this named artifact
    // prevents a prior run's screenshot from satisfying the current capture gate.
    let _ = std::fs::remove_file(&path);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    probe.pending_capture = Some(path);
    probe.pending_capture_kind = kind;
    probe.pending_capture_size = 0;
    probe.pending_capture_stable = 0;
}

fn poll_capture(probe: &mut Probe) -> bool {
    let Some(path) = probe.pending_capture.as_ref() else {
        return true;
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let size = metadata.len();
    if size == 0 || size != probe.pending_capture_size {
        probe.pending_capture_size = size;
        probe.pending_capture_stable = 0;
        return false;
    }
    probe.pending_capture_stable = probe.pending_capture_stable.saturating_add(1);
    if probe.pending_capture_stable < 3 {
        return false;
    }
    let valid_png = std::fs::read(path).ok().is_some_and(|bytes| {
        bytes.len() >= 24
            && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
            && u32::from_be_bytes(bytes[16..20].try_into().unwrap_or_default()) > 0
            && u32::from_be_bytes(bytes[20..24].try_into().unwrap_or_default()) > 0
    });
    if !valid_png {
        return false;
    }
    match probe.pending_capture_kind {
        1 => probe.service_shot_done = true,
        2 => probe.panel_shot_done = true,
        _ => {}
    }
    probe.pending_capture = None;
    true
}

fn capture_stem(faction: u8, service: u8) -> String {
    let side = if faction == 0 { "alliance" } else { "horde" };
    let service = match service {
        0 => "merchant",
        1 => "forge",
        _ => "portal",
    };
    format!("{side}-{service}")
}

fn faction_race(index: u8) -> u8 {
    if index == 0 {
        1
    } else {
        5
    }
}

fn seed_account(account: &mut super::economy_rules::Account) -> bool {
    account.gold = 1_000;
    account.inventory.clear();
    account.worn = [None; 4];
    account.next_item_id = 1;
    (0..3).all(|_| account.grant_boss_item(0, 0, 1).is_ok())
}

fn send_key(
    keys: &mut ButtonInput<KeyCode>,
    events: &mut MessageWriter<KeyboardInput>,
    state: ButtonState,
) {
    match state {
        ButtonState::Pressed => keys.press(KeyCode::ArrowRight),
        ButtonState::Released => keys.release(KeyCode::ArrowRight),
    }
    events.write(KeyboardInput {
        key_code: KeyCode::ArrowRight,
        logical_key: bevy::input::keyboard::Key::Unidentified(
            bevy::input::keyboard::NativeKey::Unidentified,
        ),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

fn choose_character(
    probe: &mut Probe,
    selection: &mut Selection,
    creatures: &crate::entities::Creatures,
) -> Option<u8> {
    let race = faction_race(probe.faction);
    let class = data::CLASSES.iter().position(|class| class.id == 8)?;
    let looks = hud::looks_for(&data::CLASSES[class], creatures);
    let look = looks
        .iter()
        .position(|(_, candidate, sex)| *candidate == race && *sex == 0)?;
    selection.class = Some(class);
    selection.looks = looks;
    selection.look = Some(look);
    selection.name = if race == 1 {
        "HubAllianceProbe"
    } else {
        "HubHordeProbe"
    }
    .into();
    selection.map = 0;
    selection.stage = 0;
    selection.version = selection.version.wrapping_add(1);
    Some(race)
}

fn site_position(site: &newhub_sites::FactionHub, kind: newinteract::InteractableKind) -> Vec3 {
    let xyz = match kind {
        newinteract::InteractableKind::Merchant => site.merchant.location.xyz,
        newinteract::InteractableKind::Artisan => site.artisan.location.xyz,
        newinteract::InteractableKind::Portal => site.portal.anchor.xyz,
        newinteract::InteractableKind::Chest => site.center.xyz,
    };
    super::units::wow_to_bevy(xyz)
}

fn expected_kind(service: u8) -> newinteract::InteractableKind {
    match service {
        0 => newinteract::InteractableKind::Merchant,
        1 => newinteract::InteractableKind::Artisan,
        _ => newinteract::InteractableKind::Portal,
    }
}

fn floor_y(collision: &benilla_world::collision::WorldCollision, expected: Vec3) -> Option<f32> {
    let probe = expected + Vec3::Y * 5.0;
    let hit = collision.ray_body(probe, Dir3::NEG_Y, 10.0)?;
    Some(probe.y - hit.distance)
}

fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    script: Option<NonSendMut<UiScript>>,
    mut intents: MessageWriter<newinteract::InteractionIntent>,
    mut exit: MessageWriter<AppExit>,
    mut sources: WorldSources,
) {
    if probe.step == 255 {
        return;
    }
    let faction = probe.faction;
    let approach_number = probe.stand_attempt + 1;
    probe.age += time.delta_secs();
    probe.wait += time.delta_secs();
    if probe.age > 240.0 {
        finish(
            &mut probe,
            &mut sources,
            &mut exit,
            false,
            "Timed out before both hubs passed.",
        );
        return;
    }
    if !poll_capture(&mut probe) {
        return;
    }
    if probe.account_before.is_none() {
        probe.account_before = Some(sources.economy.account.clone());
        probe.initial_items = sources.economy.account.inventory.len();
        probe.initial_gold = sources.economy.account.gold;
        if !seed_account(&mut sources.economy.account) {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                "Could not seed isolated fusion ingredients.",
            );
            return;
        }
    }
    let (Some(creatures), Some(mut script)) = (creatures, script) else {
        return;
    };
    if !sources.stock.ready {
        return;
    }
    match probe.action_state {
        1 if sources.stock.merchant_requested
            && script
                .eval::<bool>("return MerchantFrame and MerchantFrame:IsShown() or false")
                .unwrap_or(false) =>
        {
            if !probe.panel_shot_started {
                request_capture(
                    &mut commands,
                    &mut probe,
                    &format!(
                        "{}-merchant-panel",
                        if faction == 0 { "alliance" } else { "horde" }
                    ),
                    2,
                );
                probe.panel_shot_started = true;
                return;
            }
            if !probe.panel_shot_done {
                return;
            }
            if script
                .run("MerchantItem2ItemButton:Click('RightButton')")
                .is_err()
            {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    "Merchant buy callback could not be invoked.",
                );
                return;
            }
            probe.action_state = 2;
            probe.wait = 0.0;
            return;
        }
        1 => return,
        4 if script
            .eval::<bool>(
                "return SurvivorsPreparationForge and SurvivorsPreparationForge:IsShown() or false",
            )
            .unwrap_or(false) =>
        {
            if !probe.panel_shot_started {
                request_capture(
                    &mut commands,
                    &mut probe,
                    &format!(
                        "{}-forge-panel",
                        if faction == 0 { "alliance" } else { "horde" }
                    ),
                    2,
                );
                probe.panel_shot_started = true;
                return;
            }
            if !probe.panel_shot_done {
                return;
            }
            if script
                .run("SurvivorsPreparationFuse0:Click(); SurvivorsPreparationFuseNow:Click()")
                .is_err()
            {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    "Forge button callbacks could not be invoked.",
                );
                return;
            }
            probe.action_state = 5;
            probe.wait = 0.0;
            return;
        }
        4 => return,
        2 if probe.wait > 0.5 => {
            let bought = sources.economy.account.gold == 940
                && sources.economy.account.inventory.len() == 4
                && sources
                    .economy
                    .account
                    .inventory
                    .iter()
                    .any(|item| item.slot == 0 && item.quality == 0 && item.tier == 1);
            if !bought {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    "Merchant callback did not charge 60 gold and grant the row-two item.",
                );
                return;
            }
            probe.report.push_str(
                "Merchant panel callback bought row two: gold and permanent inventory updated.\n",
            );
            probe.action_state = 3;
            probe.wait = 0.0;
        }
        5 if probe.wait > 0.5 => {
            let fused = sources.economy.account.inventory.len() == 2
                && sources
                    .economy
                    .account
                    .inventory
                    .iter()
                    .any(|item| item.slot == 0 && item.quality == 1 && item.tier == 2);
            if !fused {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    "Forge callback did not atomically fuse three white copies into one green item.",
                );
                return;
            }
            probe.report.push_str(
                "Forge callbacks consumed three white copies and yielded one green item.\n",
            );
            probe.action_state = 0;
            probe.wait = 0.0;
        }
        2 | 5 => return,
        _ => {}
    }
    if *phase.get() == Phase::Menu && probe.step == 0 {
        let Some(race) = choose_character(&mut probe, &mut selection, &creatures) else {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                "Requested race/class look is unavailable.",
            );
            return;
        };
        probe
            .report
            .push_str(&format!("Selected race {race} for faction {}.\n", faction));
        next.set(Phase::Preparing);
        probe.step = 1;
        probe.wait = 0.0;
        return;
    }
    if probe.step == 1 {
        if *phase.get() != Phase::Preparing || loading.covering() || probe.wait < 15.0 {
            return;
        }
        let race = faction_race(faction);
        let site = newhub_sites::for_race(race);
        if crate::webenv::var("WOW_SURVIVORS_EDITOR_PROBE").as_deref() == Some("1") && faction == 0
        {
            let fi = super::newhub_editor::faction(site);
            match probe.editor_test {
                0 => {
                    sources.editor.active = true;
                    probe.editor_test = 1;
                    return;
                }
                1 => {
                    if !script
                        .eval::<bool>(
                            "return SurvivorsHubEditor and SurvivorsHubEditor:IsShown() or false",
                        )
                        .unwrap_or(false)
                    {
                        return;
                    }
                    if script.run("__hub_add=true").is_err() {
                        return;
                    }
                    probe.editor_test = 2;
                    return;
                }
                2 => {
                    if sources.editor.document.hubs[fi].portals.len() != 2 {
                        return;
                    }
                    let _ = script.run("__hub_name='Essai Elwynn'; __hub_world=1; __hub_save=true");
                    probe.editor_test = 3;
                    return;
                }
                3 => {
                    let p = &sources.editor.document.hubs[fi].portals[1];
                    if p.name != "Essai Elwynn" {
                        return;
                    }
                    let saved =
                        crate::local_state::read_to_string(&probe.home.join("survivors-hubs.txt"))
                            .ok()
                            .and_then(|s| super::newhub_layout_rules::Document::decode(&s));
                    if saved.as_ref().is_none_or(|d| {
                        d.hubs[fi].portals.len() != 2
                            || d.hubs[fi].portals[1].name != "Essai Elwynn"
                    }) {
                        return;
                    }
                    capture(&mut commands, &probe, "editor-named-portals");
                    let _ = script.run("__hub_remove=true");
                    probe.editor_test = 4;
                    return;
                }
                4 => {
                    if sources.editor.document.hubs[fi].portals.len() != 1
                        || sources.targets.iter().count() != 3
                    {
                        return;
                    }
                    sources.editor.active = false;
                    probe.editor_test = 5;
                    probe.report.push_str("Editor Lua panel added, named, assigned, persisted and removed a second portal; actor rebuild kept the camp usable.\n");
                    probe.wait = 0.0;
                    return;
                }
                _ => {}
            }
        }
        if sources.map.as_ref().is_none_or(|map| map.0 != site.map) {
            return;
        }
        if !sources.stock.ready || sources.targets.iter().count() != 3 {
            if probe.wait < 35.0 {
                return;
            }
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                "Expected three interactable hub services after map load.",
            );
            return;
        }
        let mut valid = true;
        for kind in [
            newinteract::InteractableKind::Merchant,
            newinteract::InteractableKind::Artisan,
            newinteract::InteractableKind::Portal,
        ] {
            let expected = site_position(site, kind);
            let found = sources
                .targets
                .iter()
                .find(|(_, _, item)| item.kind == kind);
            let Some((_, actual, _)) = found else {
                valid = false;
                continue;
            };
            let position_error = actual.translation().distance(expected);
            let ground_error =
                floor_y(&sources.collision, expected).map(|height| (height - expected.y).abs());
            probe.report.push_str(&format!(
                "Faction {:?} {:?}: placement error {:.2}yd, source-mesh floor ray {:?}.\n",
                site.faction, kind, position_error, ground_error
            ));
            valid &= position_error < 0.5 && ground_error.is_some_and(|error| error < 2.0);
        }
        if !valid {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                "Service placement or WMO floor/collider validation failed.",
            );
            return;
        }
        probe.report.push_str(&format!(
            "Faction {:?}: CurrentMap {} and all three source-mesh floor rays pass.\n",
            site.faction, site.map
        ));
        if !probe.capture_logged {
            if let (Ok(hero), Ok((_, camera))) = (sources.hero.single(), sources.camera.single()) {
                probe.report.push_str(&format!(
                    "Pre-capture view: hero={:?}, camera={:?}, forward={:?}, player={:?}.\n",
                    hero.translation,
                    camera.translation(),
                    camera.forward(),
                    sources.player.pos
                ));
                probe.capture_logged = true;
            } else {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    "Could not read hero/camera/player before hub capture.",
                );
                return;
            }
        }
        if !probe.small_shot_taken {
            if probe.original_resolution.is_none() {
                if let Ok(mut window) = sources.window.single_mut() {
                    probe.original_resolution = Some(UVec2::new(
                        window.resolution.physical_width(),
                        window.resolution.physical_height(),
                    ));
                }
            }
            if !probe.small_resolution_set {
                if let Ok(mut window) = sources.window.single_mut() {
                    window.resolution.set(1620.0, 690.0);
                }
                probe.small_resolution_set = true;
                probe.wait = 0.0;
                return;
            }
            if probe.wait < 1.0 {
                return;
            }
            capture(
                &mut commands,
                &probe,
                if faction == 0 {
                    "alliance-1620x690"
                } else {
                    "horde-1620x690"
                },
            );
            probe.small_shot_taken = true;
            probe.wait = 0.0;
            probe.step = 2;
            return;
        }
        if probe.step == 1 {
            probe.step = 2;
        }
    }
    if probe.step == 2 && probe.wait > 1.0 {
        let prefix = if faction == 0 { "alliance" } else { "horde" };
        if !probe
            .home
            .join("shots")
            .join(format!("{prefix}-1620x690.png"))
            .is_file()
        {
            return;
        }
        if !probe.large_resolution_set {
            if let Ok(mut window) = sources.window.single_mut() {
                window.resolution.set(1920.0, 1080.0);
            }
            probe.large_resolution_set = true;
            probe.wait = 0.0;
            return;
        }
        if probe.wait < 1.0 {
            return;
        }
        capture(
            &mut commands,
            &probe,
            if faction == 0 {
                "alliance-1920x1080"
            } else {
                "horde-1920x1080"
            },
        );
        probe.step = 4;
        probe.wait = 0.0;
    }
    if probe.step == 4 && probe.wait > 1.0 {
        let prefix = if faction == 0 { "alliance" } else { "horde" };
        if !probe
            .home
            .join("shots")
            .join(format!("{prefix}-1920x1080.png"))
            .is_file()
        {
            return;
        }
        probe.service = 0;
        if crate::webenv::var("WOW_SURVIVORS_HUB_PANEL_SMALL").as_deref() == Some("1") {
            if let Ok(mut window) = sources.window.single_mut() {
                window.resolution.set(1620.0, 690.0);
            }
            probe
                .report
                .push_str("Service panels verified at 1620x690.\n");
        } else {
            probe
                .report
                .push_str("Service panels verified at 1920x1080.\n");
        }
        probe.prepared_view = false;
        probe.step = 3;
        probe.wait = 0.0;
        return;
    }
    if probe.step == 3 {
        if probe.service >= 3 {
            if !probe.portal_returning {
                if *phase.get() != Phase::Playing || loading.covering() {
                    if probe.wait > 20.0 {
                        finish(
                            &mut probe,
                            &mut sources,
                            &mut exit,
                            false,
                            "Portal interaction did not enter the selected run.",
                        );
                    }
                    return;
                }
                if !probe.movement_started {
                    if probe.wait < 2.0 {
                        return;
                    }
                    let Ok(hero) = sources.hero.single() else {
                        return;
                    };
                    let correct = (hero.translation - hero::arena(selection.stage_def()))
                        .with_y(0.0)
                        .length()
                        < 1.0;
                    if !correct {
                        finish(
                            &mut probe,
                            &mut sources,
                            &mut exit,
                            false,
                            "Portal entered the wrong run location.",
                        );
                        return;
                    }
                    probe.expected_position = Some(hero.translation);
                    send_key(
                        &mut sources.keys,
                        &mut sources.key_events,
                        ButtonState::Pressed,
                    );
                    probe.movement_started = true;
                    probe.wait = 0.0;
                    return;
                }
                if probe.wait < 0.8 {
                    return;
                }
                send_key(
                    &mut sources.keys,
                    &mut sources.key_events,
                    ButtonState::Released,
                );
                let moved = sources.hero.single().is_ok_and(|hero| {
                    probe
                        .expected_position
                        .is_some_and(|start| (hero.translation - start).with_y(0.0).length() > 0.1)
                });
                if !moved {
                    finish(
                        &mut probe,
                        &mut sources,
                        &mut exit,
                        false,
                        "Native ArrowRight input did not move the hero in the entered world.",
                    );
                    return;
                }
                probe.report.push_str("Portal intent entered the selected run at the expected arena location; native ArrowRight moved the hero.\n");
                probe.portal_returning = true;
                probe.movement_started = false;
                next.set(Phase::Menu);
                return;
            }
            if *phase.get() != Phase::Menu || loading.covering() {
                return;
            }
            if faction == 0 {
                if !seed_account(&mut sources.economy.account) {
                    finish(
                        &mut probe,
                        &mut sources,
                        &mut exit,
                        false,
                        "Could not reset isolated test account for Horde.",
                    );
                    return;
                }
                probe.faction = 1;
                probe.small_shot_taken = false;
                probe.small_resolution_set = false;
                probe.large_resolution_set = false;
                probe.portal_returning = false;
                probe.capture_logged = false;
                probe.service_shot_done = false;
                probe.panel_shot_started = false;
                probe.panel_shot_done = false;
                probe.action_state = 0;
                probe.step = 0;
                probe.wait = 0.0;
                next.set(Phase::Menu);
                return;
            }
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                true,
                "Both faction hubs passed placement, collision, projected-ray, reach and portal-entry probes.",
            );
            return;
        }
        let kind = expected_kind(probe.service);
        let found = sources
            .targets
            .iter()
            .find(|(_, _, item)| item.kind == kind);
        let Some((target_entity, target_tf, volume)) = found else {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                "Interaction target disappeared during native ray probe.",
            );
            return;
        };
        let center = target_tf.transform_point(volume.center);
        if !probe.prepared_view {
            let (Ok(mut actor), Ok((camera, _))) =
                (sources.hero.single_mut(), sources.camera.single())
            else {
                return;
            };
            let site = newhub_sites::for_race(faction_race(faction));
            let arrival = super::units::wow_to_bevy(site.arrival.xyz);
            let toward_arrival = (arrival - target_tf.translation())
                .with_y(0.0)
                .normalize_or_zero();
            let base = if toward_arrival.length_squared() > 0.5 {
                toward_arrival
            } else {
                -Vec3::Z
            };
            let side = Vec3::new(-base.z, 0.0, base.x);
            let approach = match probe.stand_attempt {
                0 => base,
                1 => side,
                2 => -side,
                3 => -base,
                4 => (base + side).normalize_or_zero(),
                5 => (base - side).normalize_or_zero(),
                6 => (-base + side).normalize_or_zero(),
                _ => (-base - side).normalize_or_zero(),
            };
            let stand = target_tf.translation() + approach * 2.5;
            let floor = floor_y(&sources.collision, stand)
                .filter(|floor| (*floor - target_tf.translation().y).abs() < 2.0);
            let Some(floor) = floor else {
                if probe.stand_attempt < 7 {
                    probe.report.push_str(&format!(
                        "Faction {} {:?}: approach {} has no source-floor hit at target elevation; trying another side.\n",
                        faction, kind, approach_number
                    ));
                    probe.stand_attempt += 1;
                    probe.wait = 0.0;
                    return;
                }
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    &format!(
                        "Faction {} {:?}: no source-floor approach at target elevation among eight candidates.",
                        faction, kind
                    ),
                );
                return;
            };
            let mut actor_pos = stand;
            actor_pos.y = floor;
            actor.translation = actor_pos;
            let arm = actor_pos - center;
            sources.rig.yaw = arm.x.atan2(arm.z);
            sources.rig.pitch = 0.2;
            sources.rig.dist = 5.0;
            probe.expected_position = Some(actor_pos);
            probe.prepared_view = true;
            probe.wait = 0.0;
            return;
        }
        if probe.wait < 0.2 {
            return;
        }
        let Ok((camera, camera_tf)) = sources.camera.single() else {
            return;
        };
        let Ok(screen) = camera.world_to_viewport(camera_tf, center) else {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                &format!(
                    "Faction {} {:?}: target projects outside the camera.",
                    faction, kind
                ),
            );
            return;
        };
        let in_bounds = sources.window.single().is_ok_and(|window| {
            screen.x >= 0.0
                && screen.y >= 0.0
                && screen.x < window.resolution.physical_width() as f32
                && screen.y < window.resolution.physical_height() as f32
        });
        if !in_bounds {
            finish(
                &mut probe,
                &mut sources,
                &mut exit,
                false,
                &format!(
                    "Faction {} {:?}: projected service point {:?} is outside the window.",
                    faction, kind, screen
                ),
            );
            return;
        }
        let Some(ray) = camera.viewport_to_world(camera_tf, screen).ok() else {
            return;
        };
        let player_pos = sources
            .hero
            .single()
            .map(|hero| hero.translation)
            .unwrap_or(Vec3::ZERO);
        match newinteract::intent_from_ray(ray, player_pos, &sources.targets, &sources.collision) {
            Ok(intent) if intent.target == target_entity && intent.kind == kind => {
                if !probe.service_shot_done {
                    let stem = capture_stem(faction, probe.service);
                    request_capture(&mut commands, &mut probe, &stem, 1);
                    return;
                }
                probe.report.push_str(&format!(
                    "Faction {} {:?}: projected ray, wall occlusion and {:.1}yd range pass (approach {}).\n",
                    faction,
                    kind,
                    player_pos.distance(target_tf.translation()),
                    approach_number
                ));
                intents.write(intent);
                if kind == newinteract::InteractableKind::Merchant {
                    probe.action_state = 1;
                    probe.wait = 0.0;
                } else if kind == newinteract::InteractableKind::Artisan {
                    probe.action_state = 4;
                    probe.wait = 0.0;
                } else {
                    probe.wait = 0.0;
                }
            }
            Ok(intent) => {
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    &format!(
                        "Faction {} {:?}: ray selected {:?} instead.",
                        faction, kind, intent.kind
                    ),
                );
                return;
            }
            Err(error) => {
                if probe.stand_attempt < 7 {
                    probe.report.push_str(&format!(
                        "Faction {} {:?}: approach {} rejected ({:?}); trying another source-floor approach.\n",
                        faction, kind, approach_number, error
                    ));
                    probe.stand_attempt += 1;
                    probe.prepared_view = false;
                    probe.wait = 0.0;
                    return;
                }
                finish(
                    &mut probe,
                    &mut sources,
                    &mut exit,
                    false,
                    &format!(
                        "Faction {} {:?}: all eight source-floor camera approaches failed validation ({:?}).",
                        faction, kind, error,
                    ),
                );
                return;
            }
        }
        probe.service += 1;
        probe.stand_attempt = 0;
        probe.prepared_view = false;
        probe.service_shot_done = false;
        probe.stand_attempt = 0;
        probe.panel_shot_started = false;
        probe.panel_shot_done = false;
        probe.wait = 0.0;
    }
}
