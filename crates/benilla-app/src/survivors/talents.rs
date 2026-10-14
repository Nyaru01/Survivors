//! Offline talent provider. The artwork, layout and tooltips remain Blizzard's.
use super::{character_save::TalentProgressStore, data, talent_effects, talent_rules, Phase, Run};
use benilla_formats::{Talent, TalentCatalog};
use benilla_ui::script::{ScriptValue, TalentUiState, UiScript};
use bevy::prelude::*;
use std::collections::BTreeSet;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, feed.in_set(crate::ui_script::UiFeed))
        .add_systems(Update, drain.after(crate::ui_script::UiInput));
}

fn active_ability(class: u8, talent: &Talent) -> Option<&'static data::AbilityDef> {
    let class = data::CLASSES.iter().find(|c| c.id == class)?;
    class
        .pool
        .iter()
        .map(|k| data::ability(k))
        .find(|a| talent.ranks.contains(&a.spell))
}

fn support(
    class: u8,
    talent: &Talent,
    spells: &benilla_formats::SpellCatalog,
) -> Result<(), String> {
    if active_ability(class, talent).is_some() {
        return Ok(());
    }
    // Every purchased rank must work; a later unsupported rank cannot trap points.
    for spell in talent
        .ranks
        .iter()
        .copied()
        .take(talent.max_rank() as usize)
    {
        if let Some(d) = spells.get(spell) {
            if d.equipped_item_class != -1 {
                let compatible = data::CLASSES
                    .iter()
                    .find(|c| c.id == class)
                    .is_some_and(|c| {
                        c.weapons.iter().any(|w| {
                            i32::from(w.2) == d.equipped_item_class
                                && (d.equipped_item_subclass_mask == 0
                                    || d.equipped_item_subclass_mask & (1u32 << w.3) != 0)
                                && (d.equipped_item_inventory_type_mask == 0
                                    || d.equipped_item_inventory_type_mask & (1u32 << w.5) != 0)
                        })
                    });
                if !compatible {
                    return Err("L’arme requise n’est pas équipée par cette classe.".into());
                }
            }
        }
        talent_effects::support_status(spell, spells)?;
        let effects = talent_effects::derive(&[spell], spells);
        if let Some(c) = data::CLASSES.iter().find(|c| c.id == class) {
            for modifier in &effects.spell_modifiers {
                if !c.pool.iter().any(|k| {
                    spells
                        .get(data::ability(k).spell)
                        .is_some_and(|d| modifier.matches(d))
                }) {
                    return Err("Les sorts concernés ne sont pas disponibles dans ce mode.".into());
                }
            }
        }
    }
    Ok(())
}

pub(super) fn definitions(
    catalog: &TalentCatalog,
    spells: &benilla_formats::SpellCatalog,
    race: u8,
    class: u8,
) -> Vec<talent_rules::TalentDef> {
    catalog
        .tabs_for_class(race, class)
        .into_iter()
        .flat_map(|tab| {
            catalog
                .talents_in_tab(tab.id)
                .iter()
                .map(move |t| talent_rules::TalentDef {
                    id: t.id,
                    class,
                    tree: tab.id,
                    tier: t.row as u8,
                    max_rank: t.max_rank() as u8,
                    prerequisite: (t.prereq_talent != 0).then_some(talent_rules::Prerequisite {
                        id: t.prereq_talent,
                        rank: (t.prereq_rank + 1) as u8,
                    }),
                    required_spell: (t.required_spell != 0).then_some(t.required_spell),
                    supported: support(class, t, spells).is_ok(),
                })
        })
        .collect()
}

fn known(
    progress: &talent_rules::TalentProgress,
    catalog: &TalentCatalog,
    race: u8,
    class: u8,
) -> BTreeSet<u32> {
    let mut result = BTreeSet::new();
    // The locally available class spell pool supplies required-spell checks.
    if let Some(c) = data::CLASSES.iter().find(|c| c.id == class) {
        result.extend(c.pool.iter().map(|k| data::ability(k).spell).filter(|id| {
            !catalog.tabs_for_class(race, class).iter().any(|tab| {
                catalog
                    .talents_in_tab(tab.id)
                    .iter()
                    .any(|t| t.ranks.contains(id))
            })
        }));
    }
    for tab in catalog.tabs_for_class(race, class) {
        for t in catalog.talents_in_tab(tab.id) {
            let rank = progress.rank(class, t.id);
            result.extend(t.ranks.iter().copied().take(rank as usize));
        }
    }
    result
}

#[derive(Default)]
struct Memory {
    pushed: Option<TalentUiState>,
    session: u64,
    key: Option<(talent_rules::TalentProgress, u8, u8, u64)>,
    effects: talent_effects::Effects,
    locked: Vec<u32>,
}

fn feed(
    mut run: ResMut<Run>,
    phase: Res<State<Phase>>,
    selection: Res<super::Selection>,
    mut progress: ResMut<TalentProgressStore>,
    stock: Res<super::blizzard_ui::StockUi>,
    catalog: Option<Res<crate::ui_talent::Talents>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    script: Option<NonSendMut<UiScript>>,
    mut memory: Local<Memory>,
) {
    if !stock.ready || *phase.get() == Phase::Menu {
        return;
    }
    let (Some(catalog), Some(spells), Some(mut script)) = (catalog, spells, script) else {
        return;
    };
    if *phase.get() == Phase::Preparing {
        if let Some(c) = selection.class_def() {
            run.class = c;
        }
        if let Some(r) = super::menu::selected_race(&selection) {
            run.race = r;
        }
    } else if run.dead_t.is_none() {
        let _ = progress.0.observe_level(run.class.id, run.level);
    }
    let class = run.class.id;
    let key = (progress.0.clone(), run.race, class, script.session());
    if memory.key.as_ref() == Some(&key) {
        run.talent_effects = memory.effects.clone();
        run.locked_talent_spells = memory.locked.clone();
        refresh_owned(&mut run, &spells.catalog);
        return;
    }
    super::blizzard_ui::talent_tooltips::prefetch(
        &mut script,
        &spells,
        &catalog.catalog,
        run.race,
        class,
    );
    let known = known(&progress.0, &catalog.catalog, run.race, class);
    let points = u32::from(progress.0.available(class));
    let definitions = definitions(&catalog.catalog, &spells.catalog, run.race, class);
    let known_vec: Vec<_> = known.iter().copied().collect();
    if let Err(error) = progress.0.validate_profile(class, &definitions, &known_vec) {
        run.talent_effects = Default::default();
        run.locked_talent_spells = definitions
            .iter()
            .filter_map(|d| {
                catalog
                    .catalog
                    .tabs_for_class(run.race, class)
                    .iter()
                    .find_map(|tab| {
                        catalog
                            .catalog
                            .talent(tab.id, d.id)
                            .and_then(|t| active_ability(class, t).map(|a| a.spell))
                    })
            })
            .collect();
        script.fire_event(
            "UI_ERROR_MESSAGE",
            vec![ScriptValue::Str(format!(
                "Talents conservés mais non appliqués : {error}"
            ))],
        );
        refresh_owned(&mut run, &spells.catalog);
        memory.effects = Default::default();
        memory.locked = run.locked_talent_spells.clone();
        memory.key = Some(key);
        script.set_talents(TalentUiState::default());
        return;
    }
    let mut selected = Vec::new();
    let mut locked = Vec::new();
    for tab in catalog.catalog.tabs_for_class(run.race, class) {
        for t in catalog.catalog.talents_in_tab(tab.id) {
            let rank = progress.0.rank(class, t.id);
            if let Some(a) = active_ability(class, t) {
                if rank == 0 {
                    locked.push(a.spell);
                }
            } else if rank > 0 {
                selected.push(t.ranks[rank as usize - 1]);
            }
        }
    }
    run.locked_talent_spells = locked;
    run.talent_effects = talent_effects::derive(&selected, &spells.catalog);
    refresh_owned(&mut run, &spells.catalog);
    memory.effects = run.talent_effects.clone();
    memory.locked = run.locked_talent_spells.clone();
    memory.key = Some(key);
    let get = |key: &str| {
        script
            .lua()
            .globals()
            .get::<String>(key)
            .ok()
            .filter(|v| !v.is_empty())
    };
    let mut fresh = crate::ui_talent::build_pages(
        &catalog.catalog,
        &known,
        &spells.catalog,
        run.race,
        class,
        (points, 0),
        &get,
    );
    for (page, tab) in fresh
        .talents
        .iter_mut()
        .zip(catalog.catalog.tabs_for_class(run.race, class))
    {
        for (view, t) in page.iter_mut().zip(catalog.catalog.talents_in_tab(tab.id)) {
            if let Err(reason) = support(class, t, &spells.catalog) {
                view.learnable = false;
                view.meets_prereq = false;
                view.req_lines
                    .push(format!("Indisponible dans Survivors : {reason}"));
            } else if active_ability(class, t).is_some() {
                view.req_lines
                    .push("Débloque ce sort dans les choix de progression Survivors.".into());
            } else {
                let rank = progress.0.rank(class, t.id).max(1);
                view.req_lines.extend(talent_effects::adaptation_notes(
                    t.ranks[rank as usize - 1],
                    &spells.catalog,
                ));
            }
        }
    }
    let level = progress.0.profile(class).map_or(1, |p| p.highest_level);
    let _ = script
        .lua()
        .globals()
        .set("__survivors_talent_level", level);
    if memory.session != script.session() {
        memory.pushed = None;
        memory.session = script.session();
    }
    if memory.pushed.as_ref() != Some(&fresh) {
        let old = memory.pushed.as_ref().map_or(0, |v| v.points.0);
        script.set_talents(fresh.clone());
        script.fire_event(
            "CHARACTER_POINTS_CHANGED",
            vec![
                ScriptValue::Int(points as i64 - old as i64),
                ScriptValue::Int(0),
            ],
        );
        memory.pushed = Some(fresh);
    }
}

fn refresh_owned(run: &mut Run, spells: &benilla_formats::SpellCatalog) {
    for owned in &mut run.abilities {
        owned.talent_mods = spells
            .get(owned.def.spell)
            .map(|d| run.talent_effects.for_spell(d))
            .unwrap_or_default();
    }
}

fn drain(
    run: Res<Run>,
    phase: Res<State<Phase>>,
    selection: Res<super::Selection>,
    mut progress: ResMut<TalentProgressStore>,
    catalog: Option<Res<crate::ui_talent::Talents>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    script: Option<NonSendMut<UiScript>>,
) {
    let (Some(catalog), Some(spells), Some(mut script)) = (catalog, spells, script) else {
        return;
    };
    let clicks = script.take_talent_learns();
    if *phase.get() == Phase::Menu || (*phase.get() != Phase::Preparing && run.dead_t.is_some()) {
        return;
    }
    let class = if *phase.get() == Phase::Preparing {
        selection.class_def().map_or(run.class.id, |c| c.id)
    } else {
        run.class.id
    };
    let race = if *phase.get() == Phase::Preparing {
        super::menu::selected_race(&selection).unwrap_or(run.race)
    } else {
        run.race
    };
    let definitions = definitions(&catalog.catalog, &spells.catalog, race, class);
    let tabs = catalog.catalog.tabs_for_class(race, class);
    for (tab, index) in clicks {
        let talent = tab
            .checked_sub(1)
            .and_then(|i| tabs.get(i as usize))
            .and_then(|t| {
                index
                    .checked_sub(1)
                    .and_then(|i| catalog.catalog.talents_in_tab(t.id).get(i as usize))
            });
        let Some(talent) = talent else {
            continue;
        };
        let known: Vec<_> = known(&progress.0, &catalog.catalog, race, class)
            .into_iter()
            .collect();
        if let Err(error) = progress.0.allocate(class, talent.id, &definitions, &known) {
            script.fire_event(
                "UI_ERROR_MESSAGE",
                vec![ScriptValue::Str(error.to_string())],
            );
        }
    }
}
