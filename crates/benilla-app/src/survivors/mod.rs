//! Warcraft Survivors: a top-down survival mode on the benilla world, started by
//! [`crate::run_survivors`]. There is no server: the hero, the enemies and the spells are local
//! entities dressed by the same attach, animation and spell-visual systems a streamed unit gets,
//! on a battleground picked at the menu ([`data::MAPS`]), with the player controller and FrameXML
//! off ([`crate::run_mode::CaptureMode`]).

mod abilities;
mod action_bar;
mod auto;
pub(crate) mod blizzard_create;
mod blizzard_ui;
mod camera_input;
mod cards;
mod character_roster;
mod character_save;
mod data;
mod economy;
mod economy_rules;
mod encounter_clock;
mod encounters;
mod enemies;
mod grade;
mod graphics_defaults;
mod hero;
mod hud;
mod journey_verify;
mod layout_rules;
mod loading_hd;
pub(crate) mod localization;
mod menu;
#[cfg(not(target_os = "macos"))]
mod menu_admin_verify;
mod minimap;
mod mobility;
mod newhub_editor;
mod newhub_layout_rules;
mod newhub_preview;
mod newhub_sites;
mod newhub_verify;
pub(crate) mod newinteract;
mod prep;
mod progression_world;
mod relic_rules;
mod save_format;
mod settings;
mod settings_rules;
mod spell_control;
mod spell_input;
mod startup_splash;
mod talent_effects;
mod talent_rules;
mod talent_verify;
mod talents;
pub(crate) mod unit_frames;
mod units;
mod verify;
mod xp_rules;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use data::{AbilityDef, ClassDef, PassiveDef, Stat};

/// Set by [`crate::run_survivors`] before the app builds. wasm32 has no process environment, so
/// the flag lives here; `WOW_SURVIVORS` (`?survivors=1` / `__wenilla_env.survivors` on the web)
/// still turns it on from outside.
pub(crate) static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn active() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
        || crate::webenv::var("WOW_SURVIVORS").is_some()
}

pub(crate) struct SurvivorsPlugin;

impl Plugin for SurvivorsPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "cooldown.wgsl");
        // Present before `OnEnter(InWorld)`, where the UI lifecycle decides not to load FrameXML.
        app.insert_resource(crate::run_mode::CaptureMode)
            .add_plugins(grade::plugin)
            .add_plugins(graphics_defaults::GraphicsDefaultsPlugin)
            .init_state::<Phase>()
            .init_resource::<Rng>()
            .init_resource::<Selection>()
            .init_resource::<Run>()
            .init_resource::<hero::CamRig>()
            .init_resource::<hero::AimPoint>()
            .init_resource::<abilities::PendingHits>()
            .init_resource::<enemies::Spawner>()
            .init_resource::<hud::Offer>()
            .init_resource::<cards::PendingPick>()
            .init_resource::<unit_frames::PortraitUnits>()
            .init_resource::<spell_control::SpellControl>()
            .init_resource::<startup_splash::StartupSplash>()
            .add_message::<abilities::Hit>()
            .add_message::<enemies::Loot>()
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<hud::SweepUiMaterial>::default())
            .add_systems(Startup, hud::setup_hud)
            .add_systems(
                Startup,
                startup_splash::spawn.after(benilla_assets::AssetSet::Open),
            )
            .add_systems(Update, startup_splash::fade)
            .add_systems(
                Update,
                startup_splash::block_input.before(crate::ui_script::UiInput),
            )
            .add_systems(Update, hero::daylight)
            .add_systems(Update, unit_frames::refresh.after(WorldStage::Input))
            .add_systems(
                Update,
                (
                    hero::aim_cursor,
                    mobility::cast_mobility,
                    hero::drive_hero,
                    hero::place_camera,
                )
                    .chain()
                    .in_set(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    spell_control::collect,
                    enemies::spawn_enemies,
                    enemies::move_enemies,
                    abilities::cast_abilities,
                    abilities::tick_areas,
                    abilities::apply_hits,
                    enemies::tick_marks,
                    enemies::drop_loot,
                    enemies::collect_gems,
                    hero::hero_vitals,
                )
                    .chain()
                    .after(WorldStage::Input)
                    .run_if(in_state(Phase::Playing))
                    // Entering a battleground raises its loading screen: the run waits behind it.
                    .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            )
            .add_systems(
                Update,
                (enemies::reap_dead, enemies::still_gems).after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hero::follow_map,
                    hud::menu_preview,
                    hud::buttons,
                    hud::keys,
                    hud::refresh_menu,
                    menu::restore_pick,
                    menu::build_menu,
                    menu::menu_clicks,
                    menu::refresh_menu_ui.before(crate::glue::GlueVisuals),
                    cards::deal_cards,
                    cards::choose_card,
                    cards::animate_cards,
                    hud::refresh_hud,
                    hud::refresh_results,
                    hud::fill_icons,
                    hud::update_cooldown_sweeps.after(abilities::cast_abilities),
                    hud::refresh_mobility.after(mobility::cast_mobility),
                    hud::show_panels,
                )
                    .chain()
                    .after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hud::apply_ui_scale,
                    hud::fill_art,
                    hud::xp_hover,
                    hud::icon_tooltip,
                    hud::hero_plate,
                    minimap::map_buttons,
                    minimap::zone_text,
                    minimap::compose,
                )
                    .chain()
                    .after(hud::show_panels),
            )
            .add_systems(OnEnter(Phase::Playing), unpause)
            .add_systems(OnEnter(Phase::LevelUp), (pause, hud::roll_offer))
            .add_systems(OnEnter(Phase::Paused), pause)
            .add_systems(OnEnter(Phase::Preparing), unpause)
            .add_systems(OnEnter(Phase::GameOver), unpause)
            .add_systems(
                OnEnter(Phase::Menu),
                (unpause, reset_run, spell_control::reset),
            );
        if let Some(auto) = auto::from_env() {
            app.insert_resource(auto)
                .add_systems(
                    Update,
                    auto::drive.after(hud::keys).before(hud::refresh_menu),
                )
                .add_systems(Update, auto::plan.before(WorldStage::Input));
        }
        encounters::plugin(app);
        economy::plugin(app);
        settings::plugin(app);
        character_save::plugin(app);
        blizzard_create::plugin(app);
        blizzard_ui::plugin(app);
        talents::plugin(app);
        prep::plugin(app);
        newinteract::plugin(app);
        newhub_verify::plugin(app);
        loading_hd::plugin(app);
        app.add_systems(
            Update,
            menu::clamp_world
                .after(menu::menu_clicks)
                .before(menu::refresh_menu_ui),
        )
        .add_systems(Update, menu::world_labels.after(menu::refresh_menu_ui));
        journey_verify::plugin(app);
        talent_verify::plugin(app);
        verify::plugin(app);
        #[cfg(not(target_os = "macos"))]
        menu_admin_verify::plugin(app);
    }
}

/// The screen the mode is on.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum Phase {
    #[default]
    Menu,
    Preparing,
    Playing,
    LevelUp,
    Paused,
    GameOver,
}

/// Everything spawned for a run, despawned when the next begins.
#[derive(Component)]
pub(super) struct RunEntity;

/// The class, look and battleground picked at the menu; the battleground holds for the run.
#[derive(Resource, Default)]
pub(super) struct Selection {
    pub name: String,
    /// Skin, face, hairstyle, hair colour and facial hair from CharCreateCatalog.
    pub appearance: [u8; 5],
    pub class: Option<usize>,
    /// `(display, race, sex)`, one per race and sex the class's trainers come in.
    pub looks: Vec<(u32, u8, u8)>,
    pub look: Option<usize>,
    /// An index into [`data::MAPS`].
    pub map: usize,
    /// The battleground the world is standing in: the last one entered (or the saved pick at
    /// boot). Picking another at the menu changes only `map`, so the menu hero stays put and no
    /// loading screen rises until Enter World.
    pub stage: usize,
    /// Bumped on every change, so the menu rebuilds once.
    pub version: u32,
}

impl Selection {
    pub fn class_def(&self) -> Option<&'static ClassDef> {
        self.class.map(|i| &data::CLASSES[i])
    }

    pub fn map_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.map.min(data::MAPS.len() - 1)]
    }

    pub fn stage_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.stage.min(data::MAPS.len() - 1)]
    }
}

/// An ability the hero has, with its own cooldown.
pub(super) struct Owned {
    pub def: &'static AbilityDef,
    pub level: u8,
    pub cd: f32,
    pub talent_mods: talent_effects::SpellModifiers,
}

impl Owned {
    pub fn damage_mult(&self) -> f32 {
        1.0 + 0.3 * f32::from(self.level - 1)
    }
    pub fn cooldown(&self, stats: &Stats) -> f32 {
        let base = self.def.cooldown * 0.93f32.powi(i32::from(self.level) - 1);
        self.talent_mods
            .apply_cooldown(base * (1.0 - stats.haste).max(0.35))
            .max(0.2)
    }
    pub fn count(&self) -> u32 {
        let extra = [3u8, 5, 7].iter().filter(|&&l| self.level >= l).count() as u32;
        self.def.count + extra
    }
    pub fn radius(&self, stats: &Stats) -> f32 {
        self.def.radius * (1.0 + 0.08 * f32::from(self.level - 1)) * stats.area
    }
    pub fn duration(&self) -> f32 {
        self.talent_mods
            .apply_duration(self.def.duration * (1.0 + 0.1 * f32::from(self.level - 1)))
    }
}

/// The hero's derived numbers, from the base and the passives.
#[derive(Clone, Copy)]
pub(super) struct Stats {
    pub might: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub regen: f32,
    pub haste: f32,
    pub area: f32,
    pub magnet: f32,
    pub growth: f32,
    pub armor: f32,
    pub crit: f32,
}

pub(super) const BASE_HP: f32 = 120.0;
pub(super) const BASE_SPEED: f32 = 7.0;
pub(super) const BASE_MAGNET: f32 = 4.5;

/// One run's state.
#[derive(Resource)]
pub(super) struct Run {
    pub class: &'static ClassDef,
    pub race: u8,
    pub elapsed: f32,
    pub kills: u32,
    pub level: u32,
    pub xp: f32,
    pub hp: f32,
    pub gear: [u8; 4],
    pub talent_effects: talent_effects::Effects,
    /// Active talent spells are offered only after learning their actual talent.
    pub locked_talent_spells: Vec<u32>,
    pub abilities: Vec<Owned>,
    /// Spell-specific relics found in this expedition only.
    pub spell_relics: Vec<relic_rules::SpellRelic>,
    /// Chest awaiting its three-card relic selection; kept until the pick is confirmed.
    pub pending_chest: Option<[f32; 3]>,
    pub passives: Vec<(&'static PassiveDef, u8)>,
    pub shield: f32,
    pub shield_t: f32,
    pub immune_t: f32,
    pub thorns: f32,
    pub thorns_t: f32,
    pub thorns_spell: u32,
    pub heal_rate: f32,
    pub heal_t: f32,
    /// The class mobility's cooldown and running effect.
    mobile: mobility::MobilityState,
    /// Chest rewards belong only to this expedition, including its saved continuation.
    transient_gear: [u8; 4],
    pub pending_levels: u32,
    pub bosses: u32,
    pub boss_kills: u32,
    pub rings: u32,
    pub won: bool,
    /// Seconds since the hero fell, while the death plays out.
    pub dead_t: Option<f32>,
    pub next_guid: u64,
    pub damage_done: f64,
    /// Bumped whenever abilities or passives change, so the HUD rebuilds the icon rows once.
    pub version: u32,
    /// A centre-screen announcement and its seconds left.
    pub banner: Option<(String, f32)>,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            class: &data::CLASSES[0],
            race: 0,
            elapsed: 0.0,
            kills: 0,
            level: 1,
            xp: 0.0,
            hp: BASE_HP,
            gear: [0; 4],
            talent_effects: Default::default(),
            locked_talent_spells: Vec::new(),
            abilities: Vec::new(),
            spell_relics: Vec::new(),
            pending_chest: None,
            passives: Vec::new(),
            shield: 0.0,
            shield_t: 0.0,
            immune_t: 0.0,
            thorns: 0.0,
            thorns_t: 0.0,
            thorns_spell: 0,
            heal_rate: 0.0,
            heal_t: 0.0,
            mobile: mobility::MobilityState::default(),
            transient_gear: [0; 4],
            pending_levels: 0,
            bosses: 0,
            boss_kills: 0,
            rings: 0,
            won: false,
            dead_t: None,
            next_guid: 1,
            damage_done: 0.0,
            version: 0,
            banner: None,
        }
    }
}

impl Run {
    pub fn stats(&self) -> Stats {
        let mut s = Stats {
            might: 1.0,
            max_hp: BASE_HP,
            speed: BASE_SPEED,
            regen: 0.0,
            haste: 0.0,
            area: 1.0,
            magnet: BASE_MAGNET,
            growth: 1.0,
            armor: 0.0,
            crit: 0.05,
        };
        // Tanky classes start tougher, cloth starts quicker.
        match self.class.id {
            1 | 2 => s.max_hp += 40.0,
            7 | 11 | 3 | 4 => s.max_hp += 15.0,
            _ => s.speed += 0.5,
        }
        for &(p, level) in &self.passives {
            let v = p.per_level * f32::from(level);
            match p.stat {
                Stat::Might => s.might += v,
                Stat::MaxHp => s.max_hp *= 1.0 + v,
                Stat::Speed => s.speed *= 1.0 + v,
                Stat::Regen => s.regen += v,
                Stat::Haste => s.haste += v,
                Stat::Area => s.area += v,
                Stat::Magnet => s.magnet *= 1.0 + v,
                Stat::Growth => s.growth += v,
                Stat::Armor => s.armor += v,
                Stat::Crit => s.crit += v,
            }
        }
        s.might += f32::from(self.gear[0]) * 0.08;
        s.max_hp += f32::from(self.gear[1]) * 20.0;
        s.armor += f32::from(self.gear[2]) * 2.0;
        s.speed *= 1.0 + f32::from(self.gear[3]) * 0.03;
        s.max_hp = self.talent_effects.stats.max_hp.apply(s.max_hp);
        s.speed = self.talent_effects.stats.speed.apply(s.speed);
        s
    }

    pub fn guid(&mut self) -> u64 {
        self.next_guid += 1;
        self.next_guid
    }
}

/// xorshift64*: the mode's dice, seeded from the clock.
#[derive(Resource)]
pub(super) struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        let seed = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(seed | 1)
    }
}

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len.max(1) as u64) as usize
    }
}

fn pause(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn unpause(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Back at the menu: everything the last run spawned goes, and the run starts over.
fn reset_run(
    mut commands: Commands,
    spawned: Query<Entity, With<RunEntity>>,
    mut run: ResMut<Run>,
    mut pending: ResMut<abilities::PendingHits>,
    mut spawner: ResMut<enemies::Spawner>,
    mut selection: ResMut<Selection>,
) {
    for e in &spawned {
        commands.entity(e).despawn();
    }
    // Guids stay unique across runs: caches keyed by guid may outlive a despawn by a frame.
    let next_guid = run.next_guid;
    *run = Run {
        next_guid,
        ..Run::default()
    };
    pending.0.clear();
    *spawner = enemies::Spawner::default();
    // The preview hero is respawned for the look still picked.
    selection.version += 1;
}
