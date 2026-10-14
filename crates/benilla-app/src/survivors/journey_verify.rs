//! Isolated preparation/world/death integration fixture; no OS input and no real profile writes.
use super::{data, hero, hud, menu, Phase, Run, Selection};
use benilla_ui::script::UiScript;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[derive(Resource)]
struct Probe {
    home: std::path::PathBuf,
    step: u8,
    age: f32,
    wait: f32,
    captured: bool,
    camera: Option<Transform>,
    report: String,
}
pub(super) fn plugin(app: &mut App) {
    if crate::webenv::var("WOW_SURVIVORS_VERIFY").as_deref() != Some("journey") {
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
        || crate::webenv::var("WOW_SURVIVORS_AUTO").is_some()
    {
        return;
    }
    let _ = std::fs::create_dir_all(home.join("shots"));
    app.insert_resource(Probe {
        home,
        step: 0,
        age: 0.0,
        wait: 0.0,
        captured: false,
        camera: None,
        report: String::new(),
    })
    .add_systems(
        Update,
        drive
            .after(hud::show_panels)
            .after(hero::hero_vitals)
            .after(menu::world_labels),
    );
}
fn shot(commands: &mut Commands, p: &Probe, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            p.home.join("shots").join(format!("{name}.png")),
        ));
}
fn finish(p: &mut Probe, exit: &mut MessageWriter<AppExit>, good: bool, detail: &str) {
    let _ = crate::local_state::write_atomic(
        &p.home.join("verification-journey.txt"),
        &format!(
            "STATUS={}\n{}\n{detail}\n",
            if good { "PASS" } else { "FAIL" },
            p.report
        ),
    );
    p.step = 255;
    exit.write(AppExit::Success);
}
#[derive(SystemParam)]
struct Sources<'w, 's> {
    creatures: Option<Res<'w, crate::entities::Creatures>>,
    catalog: Option<Res<'w, crate::ui_talent::Talents>>,
    spells: Option<Res<'w, crate::ui_action::Spells>>,
    stock: Res<'w, super::blizzard_ui::StockUi>,
    camera: Query<'w, 's, &'static Transform, With<benilla_world::view::WorldCamera>>,
    hero: Query<'w, 's, &'static Transform, With<hero::Hero>>,
    mobs: Query<'w, 's, (Entity, &'static super::enemies::Enemy)>,
    text: Query<'w, 's, &'static Text>,
    actions: Query<'w, 's, &'static hud::UiAction>,
    hits: MessageWriter<'w, super::abilities::Hit>,
}
#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut p: ResMut<Probe>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut account: ResMut<super::economy::Economy>,
    mut talents: ResMut<super::character_save::TalentProgressStore>,
    mut worlds: ResMut<super::character_save::WorldProgressStore>,
    script: Option<NonSendMut<UiScript>>,
    mut src: Sources,
    mut exit: MessageWriter<AppExit>,
) {
    if p.step == 255 {
        return;
    }
    p.age += time.delta_secs();
    p.wait += time.delta_secs();
    if p.age > 180.0 {
        let step = p.step;
        finish(
            &mut p,
            &mut exit,
            false,
            &format!("Timed out at stage {step}"),
        );
        return;
    }
    let (Some(creatures), Some(catalog), Some(spells), Some(mut script)) = (
        src.creatures.as_deref(),
        src.catalog.as_deref(),
        src.spells.as_deref(),
        script,
    ) else {
        return;
    };
    if !src.stock.ready {
        return;
    }
    // Enemy XP is incidental to this navigation fixture; do not leave it waiting for a card.
    if *phase.get() == Phase::LevelUp && p.step >= 9 {
        next.set(Phase::Playing);
        return;
    }
    macro_rules! check {
        ($condition:expr,$message:expr) => {
            if !$condition {
                finish(&mut p, &mut exit, false, $message);
                return;
            }
        };
    }
    match p.step {
        0 if *phase.get() == Phase::Menu && !loading.covering() && p.wait > 3.0 => {
            let class = data::CLASSES.iter().position(|c| c.id == 8).unwrap();
            let looks = hud::looks_for(&data::CLASSES[class], creatures);
            let Some(look) = looks.iter().position(|l| l.1 == 1 && l.2 == 0) else {
                return;
            };
            selection.class = Some(class);
            selection.looks = looks;
            selection.look = Some(look);
            selection.name = "PréparationTest".into();
            selection.map = 5;
            selection.stage = 5;
            selection.version += 1;
            worlds.0 = Default::default();
            talents.0 = Default::default();
            let _ = talents.0.observe_level(8, 60);
            let defs = super::talents::definitions(&catalog.catalog, &spells.catalog, 1, 8);
            for _ in 0..5 {
                check!(
                    talents.0.allocate(8, 26, &defs, &[]).is_ok(),
                    "Cannot seed first-row talent points."
                );
            }
            account.account.gear = [1, 2, 1, 1];
            account.account.equipped = [true; 4];
            p.step = 1;
            p.wait = 0.0;
        }
        1 if p.wait > 4.0 && !loading.covering() && !src.hero.is_empty() => {
            check!(
                selection.map == 5 && !worlds.0.available(1, 11),
                "A fresh human must start at Elwynn with Westfall locked."
            );
            p.camera = src.camera.single().ok().cloned();
            if !p.captured {
                shot(&mut commands, &p, "selection-world-locked");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            p.step = 2;
            p.wait = 0.0;
        }
        2 if p.wait > 2.0 => {
            let still = p
                .camera
                .as_ref()
                .zip(src.camera.single().ok())
                .is_some_and(|(a, b)| {
                    a.translation.distance(b.translation) < 0.001
                        && a.rotation.angle_between(b.rotation) < 0.001
                });
            check!(still, "Selection camera kept rotating.");
            p.report
                .push_str("Camera fixed; human Elwynn only, Westfall locked.\n");
            next.set(Phase::Preparing);
            p.step = 3;
            p.wait = 0.0;
        }
        3 if *phase.get() == Phase::Preparing && p.wait > 3.0 => {
            check!(
                script
                    .eval::<bool>("return SurvivorsPreparationFrame:IsShown()")
                    .unwrap_or(false),
                "Preparation page missing."
            );
            if !p.captured {
                shot(&mut commands, &p, "preparation");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            check!(
                script.run("ToggleTalentFrame()").is_ok(),
                "Talent open failed."
            );
            p.step = 4;
            p.wait = 0.0;
        }
        4 if p.wait > 2.0 => {
            check!(
                script
                    .eval::<bool>("return TalentFrame:IsShown() and GetNumTalentTabs()==3")
                    .unwrap_or(false),
                "Preparation talents not populated."
            );
            if !p.captured {
                shot(&mut commands, &p, "preparation-talents");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            let tabs = catalog.catalog.tabs_for_class(1, 8);
            let tab = tabs
                .iter()
                .position(|t| {
                    catalog
                        .catalog
                        .talents_in_tab(t.id)
                        .iter()
                        .any(|t| t.id == 27)
                })
                .unwrap();
            let index = catalog
                .catalog
                .talents_in_tab(tabs[tab].id)
                .iter()
                .position(|t| t.id == 27)
                .unwrap();
            check!(script.run(&format!("LearnTalent({},{}); HideUIPanel(TalentFrame); ToggleCharacter('PaperDollFrame')",tab+1,index+1)).is_ok(),"Preparation learn/equipment failed.");
            p.step = 5;
            p.wait = 0.0;
        }
        5 if p.wait > 2.0 => {
            check!(
                talents.0.rank(8, 27) == 1,
                "Talent could not be allocated before world entry."
            );
            check!(
                script
                    .eval::<bool>("return CharacterFrame:IsShown()")
                    .unwrap_or(false),
                "Equipment frame missing."
            );
            let _ = script.run("__survivors_gear_toggle=5");
            p.step = 6;
            p.wait = 0.0;
        }
        6 if p.wait > 2.0 => {
            check!(
                !account.account.equipped[1] && account.account.gear[1] == 2,
                "Unequipping lost the owned chest or kept it worn."
            );
            if !p.captured {
                shot(&mut commands, &p, "preparation-equipment");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            let _ = script.run("__survivors_gear_equip=1");
            p.step = 7;
            p.wait = 0.0;
        }
        7 if p.wait > 2.0 => {
            check!(
                account.account.equipped[1],
                "Bag equipment could not be put back on."
            );
            check!(
                script
                    .run("HideUIPanel(CharacterFrame); ToggleSpellBook(BOOKTYPE_SPELL)")
                    .is_ok(),
                "Grimoire open failed."
            );
            p.step = 8;
            p.wait = 0.0;
        }
        8 if p.wait > 2.0 => {
            check!(
                script
                    .eval::<bool>("return SpellBookFrame:IsShown() and GetNumSpellTabs()==1")
                    .unwrap_or(false),
                "Class grimoire missing."
            );
            if !p.captured {
                shot(&mut commands, &p, "preparation-spells");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            check!(
                script
                    .run("CloseAllWindows(); SurvivorsPreparationEnter:GetScript('OnClick')()")
                    .is_ok(),
                "Enter button callback failed."
            );
            p.step = 9;
            p.wait = 0.0;
        }
        9 if *phase.get() == Phase::Playing && !loading.covering() && p.wait > 3.0 => {
            let correct = src.hero.single().is_ok_and(|t| {
                (t.translation - hero::arena(selection.stage_def()))
                    .with_y(0.0)
                    .length()
                    < 1.0
            });
            check!(
                correct && run.class.id == 8 && run.gear == [1, 2, 1, 1],
                "World entry used the wrong class, location or equipment."
            );
            p.report.push_str("Preparation talents, native equipment off/on and class spellbook pass; Elwynn entry position correct.\n");
            run.hp = run.stats().max_hp;
            run.immune_t = 600.0;
            run.elapsed = 721.0;
            run.bosses = 0;
            run.boss_kills = 0;
            p.step = 10;
            p.wait = 0.0;
        }
        10 if p.wait > 1.0 => {
            let bosses: Vec<_> = src
                .mobs
                .iter()
                .filter(|(_, e)| e.boss && !e.dead)
                .map(|(e, _)| e)
                .collect();
            if bosses.len() != 4 {
                return;
            }
            check!(
                run.bosses == 4 && !worlds.0.available(1, 11),
                "Spawned bosses incorrectly unlocked a world."
            );
            for e in bosses.into_iter().take(3) {
                src.hits
                    .write(super::abilities::Hit::direct(e, 1_000_000.0, false));
            }
            p.step = 11;
            p.wait = 0.0;
        }
        11 if p.wait > 2.0 => {
            check!(
                run.boss_kills == 3 && !worlds.0.available(1, 11),
                "Three kills must not unlock the next world."
            );
            let Some((last, _)) = src.mobs.iter().find(|(_, e)| e.boss && !e.dead) else {
                finish(&mut p, &mut exit, false, "Fourth boss missing.");
                return;
            };
            src.hits
                .write(super::abilities::Hit::direct(last, 1_000_000.0, false));
            p.step = 12;
            p.wait = 0.0;
        }
        12 if *phase.get() == Phase::GameOver && p.wait > 2.0 => {
            check!(
                run.won && run.boss_kills == 4 && worlds.0.available(1, 11),
                "Four credited boss kills did not unlock Westfall."
            );
            p.report.push_str("Four concurrent bosses, three kills locked, fourth kill won and unlocked Westfall.\n");
            if !p.captured {
                shot(&mut commands, &p, "world-victory");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            next.set(Phase::Menu);
            p.step = 13;
            p.wait = 0.0;
        }
        13 if *phase.get() == Phase::Menu && p.wait > 3.0 => {
            if !p.captured {
                shot(&mut commands, &p, "selection-world-unlocked");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            selection.map = 11;
            selection.stage = 11;
            selection.version += 1;
            next.set(Phase::Preparing);
            p.step = 14;
            p.wait = 0.0;
        }
        14 if *phase.get() == Phase::Preparing && !loading.covering() && p.wait > 3.0 => {
            let _ = script.run("SurvivorsPreparationEnter:GetScript('OnClick')()");
            p.step = 15;
            p.wait = 0.0;
        }
        15 if *phase.get() == Phase::Playing && !loading.covering() && p.wait > 2.0 => {
            run.hp = 0.0;
            run.immune_t = 0.0;
            run.shield = 0.0;
            p.step = 16;
            p.wait = 0.0;
        }
        16 if *phase.get() == Phase::GameOver && p.wait > 1.0 => {
            check!(
                !run.won
                    && src.text.iter().any(|t| t.0 == "Sélection du personnage")
                    && src
                        .actions
                        .iter()
                        .any(|a| matches!(a, hud::UiAction::MainMenu)),
                "Death selection button missing."
            );
            if !p.captured {
                shot(&mut commands, &p, "death-return-selection");
                p.captured = true;
                p.wait = 0.0;
                return;
            }
            p.captured = false;
            next.set(Phase::Menu);
            p.step = 17;
            p.wait = 0.0;
        }
        17 if *phase.get() == Phase::Menu && p.wait > 3.0 => {
            let saved = crate::local_state::read_to_string(&p.home.join("survivors-save.toml"))
                .ok()
                .and_then(|s| super::save_format::decode(&s).ok());
            check!(
                saved.is_some_and(|s| s.worlds.available(1, 11)
                    && s.talents.rank(8, 27) == 1
                    && s.equipped == [true; 4]),
                "Preparation choices/world completion not persisted."
            );
            check!(script.errors().is_empty(), "Native UI reported Lua errors.");
            p.report.push_str(
                "Death returned to selection; talents, equipment and unlocked world persisted.\n",
            );
            finish(
                &mut p,
                &mut exit,
                true,
                "Native captures written; inspect layout separately.",
            );
        }
        _ => {}
    }
}
