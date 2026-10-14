//! Opt-in native integration fixture for profile creation, switching and deletion.
//! It requires a fresh, marked profile and drives the same Bevy UI click handlers as a player.

use super::{
    character_roster, character_save, data, hud, menu, newhub_editor, newhub_sites, save_format,
    Phase,
};
use benilla_ui::script::UiScript;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum ClickTarget {
    Menu(menu::MenuAction),
    Create(crate::char_create::CreateAction),
}

#[derive(Resource)]
struct Probe {
    home: PathBuf,
    step: u8,
    age: f32,
    wait: f32,
    click: Option<ClickTarget>,
    click_phase: u8,
    seed_id: String,
    new_id: String,
    report: String,
    admin_step: u8,
    admin_age: f32,
    admin_wait: f32,
    admin_baseline: String,
}

pub(super) fn plugin(app: &mut App) {
    if crate::webenv::var("WOW_SURVIVORS_VERIFY").as_deref() != Some("menu_admin")
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
        warn!("Survivors menu/admin verification refused: isolated profile marker is missing.");
        return;
    }
    if !seed_profile(&home) {
        warn!("Survivors menu/admin verification refused: profile is not fresh.");
        return;
    }
    let _ = std::fs::create_dir_all(home.join("shots"));
    app.insert_resource(Probe {
        home,
        step: 0,
        age: 0.0,
        wait: 0.0,
        click: None,
        click_phase: 0,
        seed_id: String::new(),
        new_id: String::new(),
        report: String::new(),
        admin_step: 0,
        admin_age: 0.0,
        admin_wait: 0.0,
        admin_baseline: String::new(),
    })
    .add_systems(
        PreUpdate,
        inject_click
            .after(bevy::ui::UiSystems::Focus)
            .before(crate::glue::glue_clicks),
    )
    .add_systems(
        Update,
        drive
            .after(menu::menu_clicks)
            .after(menu::refresh_menu_ui)
            .after(hud::show_panels),
    )
    .add_systems(
        Update,
        admin_drive
            .after(crate::ui_script::UiInput)
            .after(super::prep::camp::walk)
            .after(drive),
    );
}

/// Seed only a brand-new marked profile, so the fixture can prove switching preserves a real
/// outgoing slot without ever touching the player's normal save.
fn seed_profile(home: &std::path::Path) -> bool {
    let path = home.join("survivors-save.toml");
    if path.exists() {
        return false;
    }
    let _ = std::fs::create_dir_all(home);
    let mut roster = character_roster::Roster::empty();
    let Some(id) = roster.create_character(save_format::Character {
        name: "MenuAdminSeed".into(),
        class: 0,
        race: 1,
        sex: 0,
        map: 0,
        appearance: [0; 5],
    }) else {
        return false;
    };
    let Some(slot) = roster.active_mut() else {
        return false;
    };
    slot.save.gold = 901;
    slot.save.potions = 3;
    let Ok(text) = character_roster::encode(&roster) else {
        return false;
    };
    crate::local_state::write_atomic(&path, &text).is_ok() && !id.is_empty()
}

fn capture(commands: &mut Commands, probe: &Probe, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            probe.home.join("shots").join(format!("{name}.png")),
        ));
}

fn valid_png(path: &std::path::Path) -> bool {
    std::fs::read(path).ok().is_some_and(|bytes| {
        bytes.len() >= 24
            && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
            && u32::from_be_bytes(bytes[16..20].try_into().unwrap_or_default()) > 0
            && u32::from_be_bytes(bytes[20..24].try_into().unwrap_or_default()) > 0
    })
}

fn finish(probe: &Probe, exit: &mut MessageWriter<AppExit>, success: bool, detail: &str) {
    let text = format!(
        "STATUS={}\n{}\n{}\n",
        if success { "PASS" } else { "FAIL" },
        probe.report,
        detail
    );
    let _ =
        crate::local_state::write_atomic(&probe.home.join("verification-menu-admin.txt"), &text);
    info!("Survivors menu/admin verification: {text}");
    exit.write(AppExit::Success);
}

/// The injected interaction goes through `glue_clicks`; production menu and create systems then
/// dispatch the same action components used by physical clicks.
fn inject_click(
    mut probe: ResMut<Probe>,
    mut menu_buttons: Query<
        (Entity, &menu::MenuAction, &mut Interaction),
        Without<crate::char_create::CreateAction>,
    >,
    mut create_buttons: Query<
        (Entity, &crate::char_create::CreateAction, &mut Interaction),
        Without<menu::MenuAction>,
    >,
) {
    let Some(target) = probe.click.clone() else {
        return;
    };
    if probe.click_phase >= 2 {
        probe.click = None;
        probe.click_phase = 0;
        return;
    }
    let desired = if probe.click_phase == 0 {
        Interaction::Pressed
    } else {
        Interaction::Hovered
    };
    let found = match target {
        ClickTarget::Menu(action) => menu_buttons
            .iter_mut()
            .find(|(_, candidate, _)| **candidate == action)
            .map(|(_, _, mut interaction)| *interaction = desired)
            .is_some(),
        ClickTarget::Create(action) => create_buttons
            .iter_mut()
            .find(|(_, candidate, _)| **candidate == action)
            .map(|(_, _, mut interaction)| *interaction = desired)
            .is_some(),
    };
    if !found {
        return;
    }
    probe.click_phase += 1;
}

fn schedule_click(probe: &mut Probe, target: ClickTarget) {
    if probe.click.is_none() {
        probe.click = Some(target);
        probe.click_phase = 0;
    }
}

fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut exit: MessageWriter<AppExit>,
    saved: Res<character_save::CharacterSave>,
    mut economy: ResMut<super::economy::Economy>,
    create_state: Res<State<super::blizzard_create::CreateScreen>>,
    local: Res<super::blizzard_create::LocalCreate>,
    create_selection: Res<crate::char_create::CreateSelection>,
    create_catalog: Option<Res<crate::entities::CharCreate>>,
    mut create_buttons: Query<(Entity, &crate::char_create::CreateAction)>,
    menu_buttons: Query<(Entity, &menu::MenuAction)>,
    mut menu_nodes: Query<&mut Node, With<menu::DeleteDialog>>,
    roster_rows: Query<Entity, With<menu::RosterRow>>,
) {
    probe.age += time.delta_secs();
    probe.wait += time.delta_secs();
    if probe.step == 255 {
        return;
    }
    if probe.age > 180.0 {
        finish(&probe, &mut exit, false, "Menu/admin fixture timed out.");
        probe.step = 255;
        return;
    }
    if *phase.get() != Phase::Menu || loading.covering() {
        return;
    }
    match probe.step {
        0 if !saved.roster.is_empty()
            && !roster_rows.is_empty()
            && menu_buttons
                .iter()
                .any(|(_, action)| *action == menu::MenuAction::CreateProfile)
            && menu_buttons
                .iter()
                .any(|(_, action)| *action == menu::MenuAction::DeleteProfile)
            && probe.wait >= 3.0 =>
        {
            let Some(seed) = saved
                .roster
                .iter()
                .find(|entry| entry.name == "MenuAdminSeed")
            else {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Seed profile did not load into the menu roster.",
                );
                probe.step = 255;
                return;
            };
            probe.seed_id = seed.id.clone();
            capture(&mut commands, &probe, "selection-roster");
            probe
                .report
                .push_str("Selection screen rendered with the seeded profile row.\n");
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::CreateProfile),
            );
            probe.step = 1;
            probe.wait = 0.0;
        }
        1 if *create_state.get() == super::blizzard_create::CreateScreen::Creating
            && local.active
            && probe.wait >= 3.0 =>
        {
            let has_human = create_buttons
                .iter_mut()
                .any(|(_, action)| *action == crate::char_create::CreateAction::Race(1));
            let has_mage_slot = create_catalog.as_deref().and_then(|cat| {
                cat.0
                    .classes_for_race(1)
                    .iter()
                    .position(|&class| class == 8)
            });
            let has_mage_button = has_mage_slot.is_some_and(|slot| {
                create_buttons.iter_mut().any(|(_, action)| {
                    *action == crate::char_create::CreateAction::ClassSlot(slot as u8)
                })
            });
            let has_create = create_buttons
                .iter_mut()
                .any(|(_, action)| *action == crate::char_create::CreateAction::Create);
            let has_back = create_buttons
                .iter_mut()
                .any(|(_, action)| *action == crate::char_create::CreateAction::Back);
            if !has_human || !has_mage_button || !has_create || !has_back {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Creation screen lacks Human/Mage controls.",
                );
                probe.step = 255;
                return;
            }
            capture(&mut commands, &probe, "new-character");
            probe.report.push_str(
                "Native new-character screen rendered with Human race and Mage class controls.\n",
            );
            schedule_click(
                &mut probe,
                ClickTarget::Create(crate::char_create::CreateAction::Race(1)),
            );
            probe.step = 2;
            probe.wait = 0.0;
        }
        2 if probe.wait >= 1.0 => {
            let mage_slot = create_catalog.as_deref().and_then(|cat| {
                cat.0
                    .classes_for_race(1)
                    .iter()
                    .position(|&class| class == 8)
            });
            let Some(mage_slot) = mage_slot else {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Human Mage combination is absent from the client catalog.",
                );
                probe.step = 255;
                return;
            };
            schedule_click(
                &mut probe,
                ClickTarget::Create(crate::char_create::CreateAction::ClassSlot(mage_slot as u8)),
            );
            probe.step = 3;
            probe.wait = 0.0;
        }
        3 if probe.wait >= 1.0 => {
            let request = create_selection.offline_probe_snapshot();
            if request.race != 1 || request.class != 8 {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Native controls did not select Human Mage.",
                );
                probe.step = 255;
                return;
            }
            schedule_click(
                &mut probe,
                ClickTarget::Create(crate::char_create::CreateAction::Create),
            );
            probe.step = 4;
            probe.wait = 0.0;
        }
        4 if *create_state.get() == super::blizzard_create::CreateScreen::Closed
            && saved.roster.len() == 2
            && probe.wait >= 2.0 =>
        {
            let Some(created) = saved.roster.iter().find(|entry| entry.id != probe.seed_id) else {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Accepted profile was not added to the roster.",
                );
                probe.step = 255;
                return;
            };
            if created.race != 1 || data::CLASSES[created.class].id != 8 {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Accepted profile did not persist as Human Mage.",
                );
                probe.step = 255;
                return;
            }
            probe.new_id = created.id.clone();
            probe.report.push_str("Human Mage accepted through the native create action and added as a distinct roster slot.\n");
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::CreateProfile),
            );
            probe.step = 5;
            probe.wait = 0.0;
        }
        5 if *create_state.get() == super::blizzard_create::CreateScreen::Creating
            && local.active
            && probe.wait >= 1.0 =>
        {
            schedule_click(
                &mut probe,
                ClickTarget::Create(crate::char_create::CreateAction::Back),
            );
            probe.step = 6;
            probe.wait = 0.0;
        }
        6 if *create_state.get() == super::blizzard_create::CreateScreen::Closed
            && saved.roster.len() == 2
            && probe.wait >= 2.0 =>
        {
            probe
                .report
                .push_str("Second creation was cancelled; roster remained at two profiles.\n");
            economy.account.gold = 2_345;
            let seed_id = probe.seed_id.clone();
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::SwitchProfile(seed_id)),
            );
            probe.step = 7;
            probe.wait = 0.0;
        }
        7 if saved.active_id == probe.seed_id && probe.wait >= 2.0 => {
            let outgoing = saved.roster.iter().find(|entry| entry.id == probe.new_id);
            let persisted = std::fs::read_to_string(probe.home.join("survivors-save.toml"))
                .ok()
                .and_then(|text| character_roster::decode(&text).ok());
            if outgoing.is_none()
                || persisted
                    .as_ref()
                    .and_then(|roster| roster.slots.iter().find(|slot| slot.id == probe.new_id))
                    .is_none_or(|slot| slot.save.gold != 2_345)
            {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Switch did not persist the outgoing profile's account.",
                );
                probe.step = 255;
                return;
            }
            if economy.account.gold != 901 {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Switch did not load the selected profile's saved account.",
                );
                probe.step = 255;
                return;
            }
            probe.report.push_str(
                "Switch persisted outgoing profile gold (2345) before loading the selected slot.\n",
            );
            let new_id = probe.new_id.clone();
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::SwitchProfile(new_id)),
            );
            probe.step = 8;
            probe.wait = 0.0;
        }
        8 if saved.active_id == probe.new_id && probe.wait >= 2.0 => {
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::DeleteProfile),
            );
            probe.step = 9;
            probe.wait = 0.0;
        }
        9 if probe.wait >= 1.0 => {
            let open = menu_nodes
                .single_mut()
                .is_ok_and(|node| node.display == Display::Flex);
            if !open {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    "Delete action did not open its confirmation dialog.",
                );
                probe.step = 255;
                return;
            }
            schedule_click(
                &mut probe,
                ClickTarget::Menu(menu::MenuAction::ConfirmDelete),
            );
            probe.report.push_str("Delete confirmation dialog opened; confirm action dispatched for the active stable slot ID.\n");
            probe.step = 10;
            probe.wait = 0.0;
        }
        10 if !saved.roster.iter().any(|entry| entry.id == probe.new_id)
            && saved.active_id == probe.seed_id
            && probe.wait >= 2.0 =>
        {
            let screenshots_ok = valid_png(&probe.home.join("shots/selection-roster.png"))
                && valid_png(&probe.home.join("shots/new-character.png"));
            let roundtrip = std::fs::read_to_string(probe.home.join("survivors-save.toml"))
                .ok()
                .and_then(|text| character_roster::decode(&text).ok())
                .and_then(|roster| character_roster::encode(&roster).ok())
                .and_then(|text| character_roster::decode(&text).ok());
            let valid = roundtrip.as_ref().is_some_and(|roster| {
                roster.active_id == probe.seed_id
                    && roster.slots.len() == 1
                    && roster
                        .active()
                        .is_some_and(|slot| slot.save.character.name == "MenuAdminSeed")
            });
            if !valid || !screenshots_ok {
                finish(
                    &probe,
                    &mut exit,
                    false,
                    &format!(
                        "Roster roundtrip valid={valid}; selection/create screenshots valid={screenshots_ok}."
                    ),
                );
                probe.step = 255;
                return;
            }
            probe.report.push_str("Confirmed deletion removed only the selected profile; saved roster decode/encode/decode reload passed.\n");
            schedule_click(&mut probe, ClickTarget::Menu(menu::MenuAction::Enter));
            probe.step = 255;
            probe.admin_wait = 0.0;
        }
        _ => {}
    }
}

fn set_editor_globals(script: &UiScript, filter: &str, select: i32, model: i32) {
    let globals = script.lua().globals();
    let _ = globals.set("__hub_filter", filter);
    let _ = globals.set("__hub_select", select);
    let _ = globals.set("__hub_model", model);
}

fn admin_drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    mut editor: ResMut<newhub_editor::HubEditor>,
    selection: Res<super::Selection>,
    script: Option<NonSendMut<UiScript>>,
    actors: Query<(&newhub_editor::CampActor, &crate::net::NetEntity)>,
    previews: Query<&crate::net::NetEntity, With<super::newhub_preview::AssetPreview>>,
    mut exit: MessageWriter<AppExit>,
) {
    if probe.step != 255 || probe.admin_step == 255 {
        return;
    }
    probe.admin_age += time.delta_secs();
    probe.admin_wait += time.delta_secs();
    if probe.admin_age > 150.0 {
        finish(
            &probe,
            &mut exit,
            false,
            &format!(
                "Hub preview/apply verification timed out at admin step {}.",
                probe.admin_step
            ),
        );
        probe.admin_step = 255;
        return;
    }
    if probe.admin_step == 0 {
        if *phase.get() != Phase::Preparing || probe.admin_wait < 3.0 {
            return;
        }
        let site = newhub_sites::for_race(menu::selected_race(&selection).unwrap_or(1));
        let faction = newhub_editor::faction(site);
        if !actors.iter().any(|(actor, _)| actor.0 == 2) {
            return;
        }
        probe.admin_baseline = toml::to_string(&editor.document).unwrap_or_default();
        if probe.admin_baseline.is_empty() {
            finish(
                &probe,
                &mut exit,
                false,
                "Could not snapshot the hub layout before preview.",
            );
            probe.admin_step = 255;
            return;
        }
        editor.active = true;
        probe.report.push_str(&format!(
            "Entered preparation; initial portal display is {}. Opening the hub asset editor.\n",
            editor.document.hubs[faction].portals[0].display
        ));
        probe.admin_step = 1;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 1 && *phase.get() == Phase::Preparing && probe.admin_wait > 2.0 {
        let Some(script) = script else { return };
        if !script
            .eval::<bool>("return SurvivorsHubEditor and SurvivorsHubEditor:IsVisible() or false")
            .unwrap_or(false)
        {
            return;
        }
        set_editor_globals(&script, "MagePortal_Darnassus", 2, 0);
        probe.admin_step = 2;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 2 && probe.admin_wait > 1.0 {
        let Some(script) = script else { return };
        let globals = script.lua().globals();
        let _ = globals.set("__hub_model", 1);
        probe.admin_step = 3;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 3 && probe.admin_wait > 2.0 {
        // The first catalog load reads the local DBCs; give slower disks time to finish it.
        if editor.preview.is_none() && probe.admin_wait < 20.0 {
            return;
        }
        let candidate_ok = editor.preview.as_ref().is_some_and(|(id, path, portal)| {
            *id == 4393 && *portal && path.to_ascii_lowercase().contains("mageportal_darnassus")
        });
        let preview_model = previews.iter().any(|model| model.display_id == Some(4393));
        let unchanged =
            toml::to_string(&editor.document).is_ok_and(|text| text == probe.admin_baseline);
        if !candidate_ok || !preview_model || !unchanged {
            finish(
                &probe,
                &mut exit,
                false,
                &format!(
                    "Preview candidate={candidate_ok}, rendered preview entity={preview_model}, document unchanged={unchanged}."
                ),
            );
            probe.admin_step = 255;
            return;
        }
        capture(&mut commands, &probe, "hub-asset-preview");
        probe.report.push_str(
            "Darnassus portal 4393 rendered as a disposable preview; the saved layout stayed byte-equivalent before Apply.\n",
        );
        if let Some(script) = script {
            let _ = script.lua().globals().set("__hub_apply", true);
        } else {
            return;
        }
        probe.admin_step = 4;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 4 && probe.admin_wait > 2.0 {
        let site = newhub_sites::for_race(menu::selected_race(&selection).unwrap_or(1));
        let faction = newhub_editor::faction(site);
        let applied = editor.document.hubs[faction].portals[0].display == 4393;
        let world = actors
            .iter()
            .find(|(actor, _)| actor.0 == 2)
            .and_then(|(_, model)| model.display_id);
        if !applied || world != Some(4393) {
            return;
        }
        probe.report.push_str("Apply changed portal slot 1 to 4393 and the spawned CampActor carries NetEntity display 4393.\n");
        let Some(script) = script else { return };
        let _ = script.lua().globals().set("__hub_add", true);
        probe.admin_step = 5;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 5 && probe.admin_wait > 2.0 {
        let site = newhub_sites::for_race(menu::selected_race(&selection).unwrap_or(1));
        let faction = newhub_editor::faction(site);
        if editor.document.hubs[faction].portals.len() < 2 {
            return;
        }
        let Some(script) = script else { return };
        set_editor_globals(&script, "MagePortal_UnderCity", 0, 1);
        probe.admin_step = 6;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 6 && probe.admin_wait > 2.0 {
        let site = newhub_sites::for_race(menu::selected_race(&selection).unwrap_or(1));
        let faction = newhub_editor::faction(site);
        let candidate_ok = editor.preview.as_ref().is_some_and(|(id, path, portal)| {
            *id == 4398 && *portal && path.to_ascii_lowercase().contains("mageportal_undercity")
        });
        let preview_model = previews.iter().any(|model| model.display_id == Some(4398));
        if (!candidate_ok || !preview_model) && probe.admin_wait < 20.0 {
            return;
        }
        if !candidate_ok
            || !preview_model
            || editor.document.hubs[faction].portals[1].display != 4393
        {
            finish(
                &probe,
                &mut exit,
                false,
                "Undercity preview was missing, not rendered, or changed the second slot before Apply.",
            );
            probe.admin_step = 255;
            return;
        }
        let Some(script) = script else { return };
        let _ = script.lua().globals().set("__hub_apply", true);
        probe.report.push_str("Undercity portal 4398 previewed for slot 2 while both document slots stayed at their prior IDs until Apply.\n");
        probe.admin_step = 7;
        probe.admin_wait = 0.0;
    } else if probe.admin_step == 7 && probe.admin_wait > 3.0 {
        let site = newhub_sites::for_race(menu::selected_race(&selection).unwrap_or(1));
        let faction = newhub_editor::faction(site);
        let first = actors
            .iter()
            .find(|(actor, _)| actor.0 == 2)
            .and_then(|(_, model)| model.display_id);
        let second = actors
            .iter()
            .find(|(actor, _)| actor.0 == 3)
            .and_then(|(_, model)| model.display_id);
        let configured = editor.document.hubs[faction].portals.len() == 2
            && editor.document.hubs[faction].portals[0].display == 4393
            && editor.document.hubs[faction].portals[1].display == 4398;
        let shot_ok = valid_png(&probe.home.join("shots/hub-asset-preview.png"));
        if !configured || first != Some(4393) || second != Some(4398) || !shot_ok {
            finish(
                &probe,
                &mut exit,
                false,
                &format!(
                    "Applied config={configured}, CampActor displays={first:?}/{second:?}, preview screenshot valid={shot_ok}."
                ),
            );
            probe.admin_step = 255;
            return;
        }
        probe.report.push_str("Apply produced two live, distinct portal actors (4393 Darnassus and 4398 Undercity); rendered preview capture is valid.\n");
        finish(
            &probe,
            &mut exit,
            true,
            "Profile administration and asset preview/apply completed in the isolated profile.",
        );
        probe.admin_step = 255;
    }
}
