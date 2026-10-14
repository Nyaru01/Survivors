//! Survivors racial traits, applied to the mode's derived stats rather than the client DBCs.
use super::Stats;
pub(super) fn apply(race: u8, elapsed: f32, stats: &mut Stats) {
    match race {
        1 => {
            stats.growth *= 1.1;
            if elapsed.rem_euclid(30.0) < 10.0 {
                stats.might *= 1.15;
            }
        }
        2 => stats.might *= 1.15,
        3 => stats.armor += 4.0,
        4 => {
            stats.speed *= 1.05;
            stats.crit += 0.02;
        }
        5 => {
            stats.regen += 0.5;
            stats.crit += 0.03;
        }
        6 => stats.max_hp *= 1.1,
        7 => {
            stats.speed += 0.3;
            stats.growth *= 1.05;
        }
        8 => {
            stats.regen += 1.0;
            stats.haste += 0.1;
        }
        _ => {}
    }
}
pub(super) fn description(race: u8) -> &'static str {
    match race {
        1 => "Perception : +15 % dégâts pendant 10 s toutes les 30 s.\nEsprit humain : +10 % expérience. Diplomatie : +20 % or.",
        2 => "Fureur sanguinaire : +15 % dégâts.",
        3 => "Peau de pierre : +4 armure.",
        4 => "Rapidité : +5 % vitesse et +2 points de critique.",
        5 => "Volonté des Réprouvés : +0,5 vie/s et +3 points de critique.",
        6 => "Endurance : +10 % vie maximale.",
        7 => "Ingéniosité : +5 % expérience et +0,3 vitesse.",
        8 => "Régénération : +1 vie/s. Berserker : +10 points de hâte.",
        _ => "",
    }
}
