//! Portable character and suspended-run save, beside the executable, with atomic replacement.
use super::{data, enemies, hero, hud, menu, save_format as fmt, Owned, Phase, Run, Selection};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

#[derive(Resource, Default, Clone, Debug)]
pub(super) struct TalentProgressStore(pub super::talent_rules::TalentProgress);

#[derive(Resource, Default)]
pub(super) struct CharacterSave {
    pub suspended: Option<fmt::Session>,
    pub resume_requested: bool,
    pub editing_name: bool,
    pub name_confirmed_this_frame: bool,
    pub notice: String,
    profile: Option<fmt::Character>,
    loaded: bool,
    blocked: bool,
    timer: f32,
    last_phase: Option<Phase>,
}
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<CharacterSave>()
        .init_resource::<TalentProgressStore>()
        .init_resource::<RestoredScene>()
        .add_systems(Startup, load)
        .add_systems(
            Update,
            restore_character
                .after(menu::restore_pick)
                .before(menu::build_menu),
        )
        .add_systems(Update, name_input.before(hud::keys))
        .add_systems(Update, resume.after(hud::show_panels))
        .add_systems(Last, autosave)
        .add_systems(
            Update,
            settle_restored_models
                .after(crate::entities::EntityVisualsSet)
                .before(benilla_world::model_fade::apply_render_fade),
        );
}

#[derive(Resource, Default)]
pub(super) struct RestoredScene(bool);

// A resumed scene starts paused, so its virtual clock cannot advance the normal appear ramp.
// Finish only the restored scene's appear ramps; combat clocks and snapshots stay frozen.
fn settle_restored_models(
    time: Res<Time>,
    phase: Res<State<Phase>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut restored: ResMut<RestoredScene>,
    mut commands: Commands,
    roots: Query<Entity, With<super::RunEntity>>,
    children: Query<&Children>,
    fades: Query<(
        Has<benilla_world::model_fade::PendingAppearFade>,
        Option<&benilla_world::model_fade::RenderFade>,
    )>,
    units: Query<(), With<benilla_world::model_fade::UnitAppearFade>>,
) {
    if !restored.0 {
        return;
    }
    if matches!(*phase.get(), Phase::Playing | Phase::GameOver) {
        restored.0 = false;
        return;
    }
    if loading.covering() {
        return;
    }
    let started = time.elapsed_secs() - benilla_world::model_fade::APPEAR_FADE_SECS - 0.1;
    for root in &roots {
        if units.contains(root) {
            commands
                .entity(root)
                .insert(benilla_world::model_fade::UnitAppearFade::Live { started });
        }
        for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
            if let Ok((pending, fade)) = fades.get(entity) {
                if pending || fade.is_some_and(|f| f.to >= 1.0 && f.from <= 0.0) {
                    commands
                        .entity(entity)
                        .remove::<benilla_world::model_fade::PendingAppearFade>()
                        .insert(benilla_world::model_fade::RenderFade::appear(started));
                }
            }
        }
    }
}
fn path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|p| p.join("survivors-save.toml"))
}
fn load(
    mut state: ResMut<CharacterSave>,
    mut economy: ResMut<super::economy::Economy>,
    mut talents: ResMut<TalentProgressStore>,
) {
    // A scripted performance/capture run must never change the real character.
    if crate::webenv::var("WOW_SURVIVORS_AUTO").is_some() {
        state.blocked = true;
        return;
    }
    let Some(path) = path() else {
        state.blocked = true;
        return;
    };
    let text = match crate::local_state::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            state.blocked = true;
            state.notice = format!("Sauvegarde inaccessible : {e}");
            warn!("{}", state.notice);
            return;
        }
    };
    let save = match fmt::decode(&text) {
        Ok(s) => s,
        Err(e) => {
            // Retain the unreadable original before attempting a backup; never overwrite it.
            let stamp = web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            let corrupt = path.with_extension(format!("corrupt-{stamp}.toml"));
            if crate::local_state::write_atomic(&corrupt, &text).is_err() {
                state.blocked = true;
            }
            state.notice =
                "Sauvegarde endommagée conservée; restauration de la copie de secours.".into();
            warn!("{} ({e})", state.notice);
            match crate::local_state::read_to_string(&path.with_extension("bak"))
                .ok()
                .and_then(|s| fmt::decode(&s).ok())
            {
                Some(s) => s,
                None => {
                    state.blocked = true;
                    state.notice="Sauvegarde endommagée conservée. Écriture suspendue pour protéger le personnage.".into();
                    return;
                }
            }
        }
    };
    economy.account.gold = save.gold;
    economy.account.potions = save.potions;
    economy.account.gear = save.gear;
    talents.0 = save.talents;
    if let Some(run) = &save.run {
        // Legacy v1 saves had no talent section; retain the level already reached in that run.
        let class = data::CLASSES[run.character.class].id;
        let _ = talents.0.observe_level(class, run.level);
    }
    state.profile = Some(save.character);
    state.suspended = save.run;
}
fn assign(
    sel: &mut Selection,
    c: &fmt::Character,
    creatures: &crate::entities::Creatures,
    cat: &crate::entities::CharCreate,
) -> bool {
    if !cat.0.allows(c.race, data::CLASSES[c.class].id) {
        return false;
    }
    let looks = hud::looks_for(&data::CLASSES[c.class], creatures);
    let Some(index) = looks
        .iter()
        .position(|&(_, r, s)| r == c.race && s == c.sex)
    else {
        return false;
    };
    sel.class = Some(c.class);
    sel.looks = looks;
    sel.look = Some(index);
    sel.map = c.map;
    sel.stage = c.map;
    sel.name = c.name.clone();
    sel.appearance = c.appearance;
    clamp(sel, cat);
    sel.version += 1;
    true
}
pub(super) fn clamp(sel: &mut Selection, cat: &crate::entities::CharCreate) {
    let Some(&(_, race, sex)) = sel.look.and_then(|i| sel.looks.get(i)) else {
        return;
    };
    if let Some(r) = cat.0.ranges(race, sex) {
        for (value, count) in sel.appearance.iter_mut().zip([
            r.skin,
            r.face,
            r.hair_style,
            r.hair_color,
            r.facial_hair,
        ]) {
            *value = (*value).min(count.saturating_sub(1));
        }
    }
}
fn restore_character(
    mut state: ResMut<CharacterSave>,
    mut sel: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
) {
    if state.loaded {
        return;
    }
    let (Some(creatures), Some(cat)) = (creatures, catalog) else {
        return;
    };
    state.loaded = true;
    if let Some(c) = state.profile.as_ref() {
        if !assign(&mut sel, c, &creatures, &cat) {
            state.notice =
                "Personnage incompatible avec les données du client. Sauvegarde conservée.".into();
            state.blocked = true;
        }
    }
    if sel.name.trim().is_empty() {
        sel.name = "Aventurier".into();
        sel.version += 1;
    }
}
fn profile(sel: &Selection) -> Option<fmt::Character> {
    let (_, race, sex) = *sel.looks.get(sel.look?)?;
    Some(fmt::Character {
        name: sel.name.trim().to_owned(),
        class: sel.class?,
        race,
        sex,
        map: sel.map,
        appearance: sel.appearance,
    })
}
fn name_input(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut state: ResMut<CharacterSave>,
    mut sel: ResMut<Selection>,
    settings: Res<super::settings::Settings>,
) {
    state.name_confirmed_this_frame = false;
    if *phase.get() != Phase::Menu || !state.editing_name || settings.open {
        events.clear();
        return;
    }
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Backspace => {
                sel.name.pop();
                sel.version += 1;
            }
            Key::Character(text) => {
                for c in text.chars().filter(|c| !c.is_control()) {
                    if sel.name.chars().count() < 24 {
                        sel.name.push(c);
                        sel.version += 1;
                    }
                }
            }
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Enter) {
        state.name_confirmed_this_frame = true;
        state.editing_name = false;
        if sel.name.trim().is_empty() {
            sel.name = "Aventurier".into();
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn resume(
    mut commands: Commands,
    mut state: ResMut<CharacterSave>,
    mut sel: ResMut<Selection>,
    mut run: ResMut<Run>,
    creatures: Option<Res<crate::entities::Creatures>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
    entities: Query<Entity, With<super::RunEntity>>,
    mut economy: ResMut<super::economy::Economy>,
    mut spells: ResMut<super::spell_control::SpellControl>,
    mut next: ResMut<NextState<Phase>>,
    mut restored: ResMut<RestoredScene>,
) {
    if !std::mem::take(&mut state.resume_requested) {
        return;
    }
    let (Some(s), Some(creatures), Some(cat)) = (state.suspended.clone(), creatures, catalog)
    else {
        return;
    };
    if !assign(&mut sel, &s.character, &creatures, &cat) {
        return;
    }
    for e in &entities {
        commands.entity(e).despawn();
    }
    hud::start_run(&mut run, &mut sel);
    run.gear = economy.account.gear;
    run.elapsed = s.elapsed;
    run.hp = s.hp;
    run.xp = s.xp;
    run.level = s.level;
    run.kills = s.kills;
    run.bosses = s.bosses;
    run.rings = s.rings;
    run.pending_levels = s.pending_levels;
    run.damage_done = s.damage_done;
    run.abilities = s
        .abilities
        .iter()
        .map(|a| Owned {
            def: data::ability(&a.key),
            level: a.rank,
            talent_mods: Default::default(),
            cd: a.cd,
        })
        .collect();
    run.passives = s
        .passives
        .iter()
        .filter_map(|a| {
            data::passives_for(run.class.id)
                .find(|p| p.key == a.key)
                .map(|p| (p, a.rank))
        })
        .collect();
    [
        run.shield,
        run.shield_t,
        run.immune_t,
        run.thorns,
        run.thorns_t,
        run.heal_rate,
        run.heal_t,
        _,
        _,
        _,
    ] = s.effects;
    run.thorns_spell = s.thorns_spell;
    run.mobile = super::mobility::MobilityState {
        cd: s.mobile[0],
        haste: s.mobile[1],
        haste_t: s.mobile[2],
        vanish_t: s.mobile[3],
    };
    economy.stock = s.stock;
    economy.opened = s.opened;
    economy.next_chest = s.next_chest;
    economy.potion_cd = s.potion_cd;
    spells.0 = super::spell_input::CastControl::default();
    for (i, a) in s.abilities.iter().enumerate() {
        if !a.automatic {
            spells.0.toggle(i);
        }
    }
    let (display, race, sex) = sel.looks[sel.look.unwrap()];
    let serial = run.guid();
    let pos = Vec3::from_array(s.pos);
    let hero = hero::spawn_hero(
        &mut commands,
        run.class,
        display,
        race,
        sex,
        serial,
        pos,
        sel.appearance,
    );
    commands
        .entity(hero)
        .insert(Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(s.yaw)));
    let roster = data::roster(s.character.map);
    for e in &s.enemies {
        let Some(def) = roster
            .enemies
            .iter()
            .chain(roster.bosses.iter())
            .find(|d| d.display == e.display && d.name == e.name)
        else {
            continue;
        };
        let id = enemies::spawn_enemy(
            &mut commands,
            &mut run,
            def,
            Vec3::from_array(e.pos),
            pos,
            e.boss,
        );
        commands.entity(id).insert((
            Transform::from_translation(Vec3::from_array(e.pos))
                .with_rotation(Quat::from_rotation_y(e.yaw)),
            enemies::Enemy {
                def,
                hp: e.hp,
                max_hp: e.max_hp,
                speed: e.speed,
                damage: e.damage,
                radius: e.radius,
                xp: e.xp,
                boss: e.boss,
                attack_cd: e.effects[0],
                slow: e.effects[1],
                slow_t: e.effects[2],
                root_t: e.effects[3],
                stun_t: e.effects[4],
                fear_t: e.effects[5],
                weaken: e.effects[6],
                weaken_t: e.effects[7],
                dots: e
                    .dots
                    .iter()
                    .map(|d| enemies::Dot {
                        dps: d[0],
                        left: d[1],
                        tick: d[2],
                    })
                    .collect(),
                dead: false,
            },
        ));
    }
    for p in &s.pickups {
        let kind = match p.kind {
            0 => enemies::GemKind::Xp(p.value as u32),
            1 => enemies::GemKind::Heal(p.value),
            _ => enemies::GemKind::Gold(p.value as u32),
        };
        enemies::restore_gem(&mut commands, &mut run, kind, Vec3::from_array(p.pos));
    }
    for p in &s.chests {
        super::economy::restore_chest(&mut commands, &mut run, Vec3::from_array(*p));
    }
    run.version += 1;
    restored.0 = true;
    run.banner = Some((
        "Partie restaurée — Échap pour ouvrir le menu et reprendre.".into(),
        4.0,
    ));
    // Always resume paused: the player sees the preserved scene before the simulation moves.
    next.set(if run.pending_levels > 0 {
        Phase::LevelUp
    } else {
        Phase::Paused
    });
    info!(
        "Partie restaurée : {} s, {} ennemis dont {} boss",
        run.elapsed,
        s.enemies.len(),
        s.enemies.iter().filter(|e| e.boss).count()
    );
}

#[allow(clippy::too_many_arguments)]
fn autosave(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    run: Res<Run>,
    economy: Res<super::economy::Economy>,
    spells: Res<super::spell_control::SpellControl>,
    talents: Res<TalentProgressStore>,
    heroes: Query<&Transform, With<hero::Hero>>,
    foes: Query<(&Transform, &enemies::Enemy)>,
    gems: Query<(&Transform, &enemies::Gem)>,
    chests: Query<&Transform, With<super::economy::Chest>>,
    mut state: ResMut<CharacterSave>,
    mut exit: MessageReader<AppExit>,
    mut close: MessageReader<bevy::window::WindowCloseRequested>,
) {
    let quitting = exit.read().count() > 0 || close.read().count() > 0;
    state.timer += time.delta_secs();
    let changed = state.last_phase != Some(*phase.get());
    state.last_phase = Some(*phase.get());
    if state.blocked || !state.loaded || (state.timer < 2.0 && !changed && !quitting) {
        return;
    }
    state.timer = 0.0;
    let Some(character) = profile(&selection) else {
        return;
    };
    if !fmt::valid_character(&character) {
        return;
    }
    if matches!(phase.get(), Phase::Playing | Phase::Paused | Phase::LevelUp)
        && run.dead_t.is_none()
    {
        if let Ok(h) = heroes.single() {
            let session = fmt::Session {
                character: character.clone(),
                elapsed: run.elapsed,
                hp: run.hp,
                xp: run.xp,
                level: run.level,
                kills: run.kills,
                bosses: run.bosses,
                rings: run.rings,
                pending_levels: run.pending_levels,
                damage_done: run.damage_done,
                pos: h.translation.to_array(),
                yaw: h.rotation.to_euler(EulerRot::YXZ).0,
                abilities: run
                    .abilities
                    .iter()
                    .enumerate()
                    .map(|(i, a)| fmt::Spell {
                        key: a.def.key.into(),
                        rank: a.level,
                        cd: a.cd.max(0.0),
                        automatic: spells.0.automatic(i),
                    })
                    .collect(),
                passives: run
                    .passives
                    .iter()
                    .map(|(p, rank)| fmt::Spell {
                        key: p.key.into(),
                        rank: *rank,
                        cd: 0.0,
                        automatic: true,
                    })
                    .collect(),
                effects: [
                    run.shield.max(0.0),
                    run.shield_t.max(0.0),
                    run.immune_t.max(0.0),
                    run.thorns.max(0.0),
                    run.thorns_t.max(0.0),
                    run.heal_rate.max(0.0),
                    run.heal_t.max(0.0),
                    0.0,
                    0.0,
                    0.0,
                ],
                thorns_spell: run.thorns_spell,
                mobile: [
                    run.mobile.cd.max(0.0),
                    run.mobile.haste.max(0.0),
                    run.mobile.haste_t.max(0.0),
                    run.mobile.vanish_t.max(0.0),
                ],
                stock: economy.stock,
                opened: economy.opened,
                next_chest: economy.next_chest,
                potion_cd: economy.potion_cd.max(0.0),
                enemies: foes
                    .iter()
                    .filter(|(_, e)| !e.dead && e.hp > 0.0)
                    .map(|(t, e)| fmt::Foe {
                        display: e.def.display,
                        name: e.def.name.into(),
                        boss: e.boss,
                        pos: t.translation.to_array(),
                        yaw: t.rotation.to_euler(EulerRot::YXZ).0,
                        hp: e.hp,
                        max_hp: e.max_hp,
                        speed: e.speed,
                        damage: e.damage,
                        radius: e.radius,
                        xp: e.xp,
                        effects: [
                            e.attack_cd.max(0.0),
                            e.slow.max(0.0),
                            e.slow_t.max(0.0),
                            e.root_t.max(0.0),
                            e.stun_t.max(0.0),
                            e.fear_t.max(0.0),
                            e.weaken.max(0.0),
                            e.weaken_t.max(0.0),
                            0.0,
                        ],
                        dots: e
                            .dots
                            .iter()
                            .map(|d| [d.dps.max(0.0), d.left.max(0.0), d.tick.max(0.0)])
                            .collect(),
                    })
                    .collect(),
                pickups: gems
                    .iter()
                    .map(|(t, g)| {
                        let (kind, value) = match g.kind {
                            enemies::GemKind::Xp(x) => (0, x as f32),
                            enemies::GemKind::Heal(h) => (1, h),
                            enemies::GemKind::Gold(g) => (2, g as f32),
                        };
                        fmt::Pickup {
                            kind,
                            value,
                            pos: t.translation.to_array(),
                        }
                    })
                    .collect(),
                chests: chests.iter().map(|t| t.translation.to_array()).collect(),
            };
            state.suspended = Some(session);
        }
    } else if *phase.get() == Phase::GameOver || run.dead_t.is_some() {
        state.suspended = None;
    }
    let save = fmt::Save {
        version: 1,
        character,
        gold: economy.account.gold,
        potions: economy.account.potions,
        gear: economy.account.gear,
        talents: talents.0.clone(),
        run: state.suspended.clone(),
    };
    let Some(path) = path() else { return };
    let Ok(text) = toml::to_string(&save) else {
        return;
    };
    if let Ok(old) = crate::local_state::read_to_string(&path) {
        if fmt::decode(&old).is_ok() {
            if let Err(e) = crate::local_state::write_atomic(&path.with_extension("bak"), &old) {
                warn!("Copie de secours impossible: {e}");
                return;
            }
        }
    }
    if let Err(e) = crate::local_state::write_atomic(&path, &text) {
        state.notice = format!("Échec sauvegarde : {e}");
        warn!("{}", state.notice);
    }
}
