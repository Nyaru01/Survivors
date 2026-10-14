//! Isolated native talent-window fixture; never sends input to Windows.
use super::{
    character_save::TalentProgressStore, data, hero, hud, talent_rules, talents, Phase, Run,
    Selection,
};
use benilla_ui::script::UiScript;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::path::PathBuf;

#[derive(Resource)]
struct Probe {
    home: PathBuf,
    step: u8,
    age: f32,
    wait: f32,
    report: String,
    fire_tab: u32,
    fire_index: u32,
}

pub(super) fn plugin(app: &mut App) {
    if crate::webenv::var("WOW_SURVIVORS_VERIFY").as_deref() != Some("talents")
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
        warn!("Talent verification refused: missing isolated profile marker.");
        return;
    }
    let _ = std::fs::create_dir_all(home.join("shots"));
    app.insert_resource(crate::ui_script::CapturePointerPinned(true))
        .insert_resource(Probe {
            home,
            step: 0,
            age: 0.0,
            wait: 0.0,
            report: String::new(),
            fire_tab: 0,
            fire_index: 0,
        })
        .add_systems(
            Update,
            drive
                .after(crate::ui_script::UiInput)
                .after(hud::show_panels),
        );
}
fn capture(commands: &mut Commands, probe: &Probe, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            probe.home.join("shots").join(format!("{name}.png")),
        ));
}
fn finish(probe: &mut Probe, exit: &mut MessageWriter<AppExit>, success: bool, detail: &str) {
    let text = format!(
        "STATUS={}\n{}\n{}\n",
        if success { "PASS" } else { "FAIL" },
        probe.report,
        detail
    );
    let _ = crate::local_state::write_atomic(&probe.home.join("verification-talents.txt"), &text);
    info!("Talent verification: {text}");
    probe.step = 255;
    exit.write(AppExit::Success);
}
#[allow(clippy::too_many_arguments)]
#[derive(SystemParam)]
struct Sources<'w> {
    creatures: Option<Res<'w, crate::entities::Creatures>>,
    catalog: Option<Res<'w, crate::ui_talent::Talents>>,
    spells: Option<Res<'w, crate::ui_action::Spells>>,
}

fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut progress: ResMut<TalentProgressStore>,
    sources: Sources,
    stock: Res<super::blizzard_ui::StockUi>,
    script: Option<NonSendMut<UiScript>>,
    mut exit: MessageWriter<AppExit>,
    heroes: Query<&Transform, With<hero::Hero>>,
) {
    if probe.step == 255 {
        return;
    }
    probe.age += time.delta_secs();
    probe.wait += time.delta_secs();
    if probe.age > 180.0 {
        finish(
            &mut probe,
            &mut exit,
            false,
            "Timed out loading talent fixture.",
        );
        return;
    }
    let (Some(creatures), Some(catalog), Some(spells), Some(mut script)) = (
        sources.creatures.as_deref(),
        sources.catalog.as_deref(),
        sources.spells.as_deref(),
        script,
    ) else {
        return;
    };
    if probe.step == 0 {
        if *phase.get() != Phase::Menu || loading.covering() || probe.wait < 3.0 || !stock.ready {
            return;
        }
        let class = data::CLASSES.iter().position(|c| c.id == 8).unwrap();
        let looks = hud::looks_for(&data::CLASSES[class], creatures);
        let Some(look) = looks.iter().position(|&(_, r, s)| r == 1 && s == 0) else {
            return;
        };
        let mut total = 0;
        for class in [1, 2, 3, 4, 5, 7, 8, 9, 11] {
            let Some(tabs) = (1..=8)
                .map(|race| catalog.catalog.tabs_for_class(race, class))
                .find(|tabs| tabs.len() == 3)
            else {
                finish(
                    &mut probe,
                    &mut exit,
                    false,
                    &format!("Class {class} does not have three trees."),
                );
                return;
            };
            if tabs.iter().any(|t| t.name.trim().is_empty()) {
                finish(
                    &mut probe,
                    &mut exit,
                    false,
                    "Empty localized talent-tab name.",
                );
                return;
            }
            let english_only = [
                "Arms",
                "Fury",
                "Holy",
                "Retribution",
                "Beast Mastery",
                "Marksmanship",
                "Survival",
                "Assassination",
                "Subtlety",
                "Shadow",
                "Elemental",
                "Enhancement",
                "Restoration",
                "Arcane",
                "Fire",
                "Frost",
                "Demonology",
                "Balance",
                "Feral Combat",
            ];
            if tabs
                .iter()
                .any(|tab| english_only.contains(&tab.name.as_str()))
            {
                finish(
                    &mut probe,
                    &mut exit,
                    false,
                    &format!("Class {class} still has English talent tab names."),
                );
                return;
            }
            probe.report.push_str(&format!(
                "Class {class}: {} / {} / {}\n",
                tabs[0].name, tabs[1].name, tabs[2].name
            ));
            total += tabs.len();
        }
        if total != 27 {
            finish(&mut probe, &mut exit, false, "Expected 27 class trees.");
            return;
        }
        let mage_tabs = catalog.catalog.tabs_for_class(1, 8);
        if mage_tabs
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            != ["Arcanes", "Feu", "Givre"]
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Mage tree names are not the French client strings.",
            );
            return;
        }
        for (tab_index, tab) in mage_tabs.iter().enumerate() {
            if let Some(index) = catalog
                .catalog
                .talents_in_tab(tab.id)
                .iter()
                .position(|t| t.id == 27)
            {
                probe.fire_tab = tab_index as u32 + 1;
                probe.fire_index = index as u32 + 1;
            }
        }
        if probe.fire_tab == 0 {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Improved Fire Blast talent 27 missing.",
            );
            return;
        }
        selection.class = Some(class);
        selection.looks = looks;
        selection.look = Some(look);
        selection.name = "TalentsFR".into();
        selection.appearance = [0; 5];
        selection.version += 1;
        progress.0 = Default::default();
        if !hud::start_run(&mut run, &mut selection) {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Could not start talent fixture.",
            );
            return;
        }
        run.level = 60;
        let _ = progress.0.observe_level(8, 60);
        let definitions = talents::definitions(&catalog.catalog, &spells.catalog, 1, 8);
        // Fire Blast is in the SECOND row. Unlock it legitimately with five first-row ranks.
        for _ in 0..5 {
            if let Err(error) = progress.0.allocate(8, 26, &definitions, &[]) {
                finish(
                    &mut probe,
                    &mut exit,
                    false,
                    &format!("First-row setup failed: {error}"),
                );
                return;
            }
        }
        run.abilities.push(super::Owned {
            def: data::ability("fire_blast"),
            level: 1,
            cd: 600.0,
            talent_mods: Default::default(),
        });
        run.hp = run.stats().max_hp;
        run.immune_t = 600.0;
        next.set(Phase::Playing);
        probe.step = 1;
        probe.wait = 0.0;
    } else if probe.step == 1 {
        if *phase.get() != Phase::Playing
            || loading.covering()
            || heroes.is_empty()
            || probe.wait < 3.0
            || progress.0.available(8) != 46
        {
            return;
        }
        if let Err(error) = script.run("ToggleTalentFrame()") {
            finish(&mut probe, &mut exit, false, &error.to_string());
            return;
        }
        probe.step = 2;
        probe.wait = 0.0;
    } else if matches!(probe.step, 2 | 4 | 6) && probe.wait > 3.0 {
        let tab = probe.step / 2;
        if !script
            .eval::<bool>("return TalentFrame and TalentFrame:IsVisible() or false")
            .unwrap_or(false)
            || !script.errors().is_empty()
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                &format!(
                    "Talent window invisible or Lua errors: {:?}",
                    script.errors()
                ),
            );
            return;
        }
        capture(&mut commands, &probe, &format!("talents-mage-tree-{tab}"));
        probe.step += 1;
        probe.wait = 0.0;
    } else if matches!(probe.step, 3 | 5) && probe.wait > 1.0 {
        let tab = probe.step / 2 + 1;
        let cmd = format!("this=TalentFrameTab{tab}; TalentFrameTab_OnClick()");
        if let Err(error) = script.run(&cmd) {
            finish(&mut probe, &mut exit, false, &error.to_string());
            return;
        }
        probe.step += 1;
        probe.wait = 0.0;
    } else if probe.step == 7 && probe.wait > 1.0 {
        let cmd = format!(
            "this=TalentFrameTab{}; TalentFrameTab_OnClick()",
            probe.fire_tab
        );
        if let Err(error) = script.run(&cmd) {
            finish(&mut probe, &mut exit, false, &error.to_string());
            return;
        }
        script.resolve();
        let center = script.eval::<(f32, f32)>(&format!(
            "return TalentFrameTalent{}:GetCenter()",
            probe.fire_index
        ));
        let Ok((x, y)) = center else {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Could not locate actual talent button for first hover.",
            );
            return;
        };
        script.mouse_move(x, y);
        probe.step = 8;
        probe.wait = 0.0;
    } else if probe.step == 8 && probe.wait > 3.0 {
        let visible = script
            .eval::<bool>("return GameTooltip:IsShown()")
            .unwrap_or(false);
        let raw=script.eval::<String>("local t=''; for i=1,30 do local f=getglobal('GameTooltipTextLeft'..i); if f and f:IsShown() then t=t..(f:GetText() or '')..'\\n' end end return t").unwrap_or_default();
        probe.report.push_str(&format!("Talent tooltip: {raw}\n"));
        if !visible
            || !raw.contains("Trait de feu")
            || raw.contains("$s")
            || !script.errors().is_empty()
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                "French talent tooltip missing, raw token, or Lua error.",
            );
            return;
        }
        capture(&mut commands, &probe, "talents-mage-tooltip");
        let cmd = format!(
            "GameTooltip:Hide(); LearnTalent({}, {})",
            probe.fire_tab, probe.fire_index
        );
        if let Err(error) = script.run(&cmd) {
            finish(&mut probe, &mut exit, false, &error.to_string());
            return;
        }
        probe.step = 9;
        probe.wait = 0.0;
    } else if matches!(probe.step, 9 | 10) && probe.wait > 2.0 {
        let rank = probe.step - 8;
        let fire = data::ability("fire_blast");
        let Some(spell) = spells.catalog.get(fire.spell) else {
            finish(&mut probe, &mut exit, false, "Fire Blast spell absent.");
            return;
        };
        let applied = run
            .talent_effects
            .for_spell(spell)
            .apply_cooldown(fire.cooldown);
        let expected = fire.cooldown - f32::from(rank) * 0.5;
        if progress.0.rank(8, 27) != rank
            || progress.0.available(8) != 46 - u32::from(rank)
            || (applied - expected).abs() > 0.0001
            || !run
                .abilities
                .iter()
                .find(|a| a.def.key == "fire_blast")
                .is_some_and(|a| (a.cooldown(&run.stats()) - expected).abs() < 0.0001)
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                &format!(
                    "Rank {rank}, points {}, cooldown {applied} expected {expected}.",
                    progress.0.available(8)
                ),
            );
            return;
        }
        probe.report.push_str(&format!(
            "Talent 27 rank {rank}: cooldown {} -> {applied}, points {}.\n",
            fire.cooldown,
            progress.0.available(8)
        ));
        if probe.step == 9 {
            let _ = script.run(&format!(
                "LearnTalent({}, {})",
                probe.fire_tab, probe.fire_index
            ));
            probe.step = 10;
        } else {
            probe.step = 11;
        }
        probe.wait = 0.0;
    } else if probe.step == 11 && probe.wait > 2.0 {
        let definitions = talents::definitions(&catalog.catalog, &spells.catalog, 1, 8);
        let known: Vec<_> = data::CLASSES
            .iter()
            .find(|c| c.id == 8)
            .unwrap()
            .pool
            .iter()
            .map(|k| data::ability(k).spell)
            .collect();
        let Some(tier) = definitions.iter().find(|d| {
            d.supported && d.tier > 1 && d.prerequisite.is_none() && d.required_spell.is_none()
        }) else {
            finish(
                &mut probe,
                &mut exit,
                false,
                "No supported locked-tier talent to validate.",
            );
            return;
        };
        let Some(unsupported) = definitions.iter().find(|d| !d.supported) else {
            finish(
                &mut probe,
                &mut exit,
                false,
                "No unsupported talent to validate.",
            );
            return;
        };
        let mut copy = progress.0.clone();
        if copy.allocate(8, tier.id, &definitions, &known) != Err(talent_rules::TalentError::Tier)
            || copy.allocate(8, unsupported.id, &definitions, &known)
                != Err(talent_rules::TalentError::Unsupported)
            || copy != progress.0
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Invalid talent purchase altered progress.",
            );
            return;
        }
        let mut wrong = definitions.clone();
        wrong.iter_mut().find(|d| d.id == 27).unwrap().class = 9;
        if copy.allocate(8, 27, &wrong, &known) != Err(talent_rules::TalentError::WrongClass) {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Wrong class talent was accepted.",
            );
            return;
        }
        // Queue the real Lua intents as well; the offline drain must reject both without spending.
        for id in [tier.id, unsupported.id] {
            for (tab_i, tab) in catalog.catalog.tabs_for_class(1, 8).iter().enumerate() {
                if let Some(index) = catalog
                    .catalog
                    .talents_in_tab(tab.id)
                    .iter()
                    .position(|t| t.id == id)
                {
                    let _ = script.run(&format!("LearnTalent({}, {})", tab_i + 1, index + 1));
                }
            }
        }
        probe.step = 12;
        probe.wait = 0.0;
    } else if probe.step == 12 && probe.wait > 3.0 {
        if !script.errors().is_empty() {
            finish(
                &mut probe,
                &mut exit,
                false,
                &format!("Lua errors after talent allocations: {:?}", script.errors()),
            );
            return;
        }
        if progress.0.rank(8, 27) != 2 || progress.0.available(8) != 44 {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Rejected Lua intents spent points.",
            );
            return;
        }
        let disk = crate::local_state::read_to_string(&probe.home.join("survivors-save.toml"))
            .ok()
            .and_then(|text| super::save_format::decode(&text).ok());
        if !disk
            .as_ref()
            .is_some_and(|saved| saved.talents == progress.0)
        {
            finish(
                &mut probe,
                &mut exit,
                false,
                "Talent save reload differs from live progress.",
            );
            return;
        }
        probe.report.push_str("Tier, unsupported and wrong-class rejected; actual on-disk save reload matches ranks and points.\n");
        capture(&mut commands, &probe, "talents-mage-rank-two");
        probe.step = 13;
        probe.wait = 0.0;
    } else if probe.step == 13 && probe.wait > 3.0 {
        finish(
            &mut probe,
            &mut exit,
            true,
            "Native screenshots written. Validate image layout separately.",
        );
    }
}
