//! Persistent lifetime progress and achievements, integrated by the Survivors save owner.
use bevy::prelude::*;

#[derive(Resource, Clone, Debug, Default)]
pub(super) struct MetaProgressStore(pub super::meta_rules::Progress);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MetaProgressStore>()
        .add_systems(Update, settle_game_over);
}

/// The completed run ID lives in the suspended-run save. Persisted settlement IDs make this
/// observer safe across frames, process restarts, and revisiting the result screen.
fn settle_game_over(
    phase: Res<State<super::Phase>>,
    run: Option<ResMut<super::Run>>,
    economy: Res<super::economy::Economy>,
    mut meta: ResMut<MetaProgressStore>,
    mut talents: ResMut<super::character_save::TalentProgressStore>,
) {
    if *phase.get() != super::Phase::GameOver {
        return;
    }
    let Some(mut run) = run else { return };
    let summary = super::meta_rules::RunSummary {
        kills: u64::from(run.kills),
        bosses: u64::from(run.boss_kills),
        gold_earned: u64::from(run.gold_earned),
        chests: u64::from(economy.opened),
        survival_seconds: if run.elapsed.is_finite() && run.elapsed > 0.0 {
            run.elapsed.floor() as u64
        } else {
            0
        },
        damage_taken: if run.damage_taken.is_finite() && run.damage_taken > 0.0 {
            run.damage_taken.floor() as u64
        } else {
            0
        },
        elites: u64::from(run.elites_killed),
        guardians: u64::from(run.guardians_killed),
    };
    let class = run.class.id;
    if !meta.0.settle_run(&run.run_id, summary) {
        return;
    }
    let before = meta.0.class_level(class);
    run.talent_xp_awarded = meta.0.award_talent_xp(class, summary);
    let gained = meta.0.class_level(class).saturating_sub(before);
    let level = talents
        .0
        .highest_level(class)
        .max(9)
        .saturating_add(gained)
        .min(60);
    let _ = talents.0.observe_level(class, level);
}
