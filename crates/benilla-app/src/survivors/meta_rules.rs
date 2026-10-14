//! Lifetime goals and one-time rewards, independent of Bevy and the HUD.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Progress {
    #[serde(default)]
    pub lifetime_kills: u64,
    #[serde(default)]
    pub lifetime_bosses: u64,
    #[serde(default)]
    pub lifetime_gold: u64,
    #[serde(default)]
    pub lifetime_chests: u64,
    #[serde(default)]
    pub lifetime_runs: u64,
    #[serde(default)]
    pub lifetime_survival_seconds: u64,
    #[serde(default)]
    pub lifetime_damage_taken: u64,
    #[serde(default)]
    pub lifetime_elites: u64,
    #[serde(default)]
    pub lifetime_guardians: u64,
    /// Stable IDs prevent a resumed or revisited GameOver screen from paying out twice.
    #[serde(default)]
    pub settled_runs: Vec<String>,
    #[serde(default)]
    pub claimed_rewards: Vec<String>,
    #[serde(default)]
    pub class_xp: Vec<ClassTalentXp>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClassTalentXp {
    pub class: u8,
    pub xp: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RunSummary {
    pub kills: u64,
    pub bosses: u64,
    pub gold_earned: u64,
    pub chests: u64,
    pub survival_seconds: u64,
    pub damage_taken: u64,
    pub elites: u64,
    pub guardians: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Measure {
    Kills,
    Bosses,
    Gold,
    Chests,
    Runs,
    SurvivalSeconds,
    DamageTaken,
    Elites,
    Guardians,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Achievement {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub measure: Measure,
    pub target: u64,
    pub reward_gold: u32,
}

/// Compact, reachable milestones with rewards granted only through `Progress::claim`.
pub(super) const ACHIEVEMENTS: &[Achievement] = &[
    Achievement {
        id: "first_blood",
        title: "Premier sang",
        detail: "Éliminer 100 ennemis.",
        measure: Measure::Kills,
        target: 100,
        reward_gold: 25,
    },
    Achievement {
        id: "slayer",
        title: "Pourfendeur",
        detail: "Éliminer 1 000 ennemis.",
        measure: Measure::Kills,
        target: 1_000,
        reward_gold: 100,
    },
    Achievement {
        id: "boss_hunter",
        title: "Chasseur de boss",
        detail: "Vaincre 10 boss.",
        measure: Measure::Bosses,
        target: 10,
        reward_gold: 100,
    },
    Achievement {
        id: "treasure_finder",
        title: "Chasseur de trésors",
        detail: "Ouvrir 25 coffres.",
        measure: Measure::Chests,
        target: 25,
        reward_gold: 75,
    },
    Achievement {
        id: "veteran",
        title: "Vétéran",
        detail: "Terminer 10 expéditions.",
        measure: Measure::Runs,
        target: 10,
        reward_gold: 100,
    },
    Achievement {
        id: "marathon",
        title: "Marathon",
        detail: "Survivre 30 minutes au total.",
        measure: Measure::SurvivalSeconds,
        target: 1_800,
        reward_gold: 100,
    },
    Achievement {
        id: "riches",
        title: "Poche pleine",
        detail: "Gagner 1 000 pièces d’or en expédition.",
        measure: Measure::Gold,
        target: 1_000,
        reward_gold: 100,
    },
    Achievement {
        id: "iron_will",
        title: "Volonté de fer",
        detail: "Subir 10 000 points de dégâts.",
        measure: Measure::DamageTaken,
        target: 10_000,
        reward_gold: 75,
    },
];

impl Progress {
    pub(super) fn measure(&self, measure: Measure) -> u64 {
        match measure {
            Measure::Kills => self.lifetime_kills,
            Measure::Bosses => self.lifetime_bosses,
            Measure::Gold => self.lifetime_gold,
            Measure::Chests => self.lifetime_chests,
            Measure::Runs => self.lifetime_runs,
            Measure::SurvivalSeconds => self.lifetime_survival_seconds,
            Measure::DamageTaken => self.lifetime_damage_taken,
            Measure::Elites => self.lifetime_elites,
            Measure::Guardians => self.lifetime_guardians,
        }
    }

    pub(super) fn unlocked(&self, achievement: &Achievement) -> bool {
        self.measure(achievement.measure) >= achievement.target
    }

    pub(super) fn reward_claimed(&self, id: &str) -> bool {
        self.claimed_rewards.iter().any(|claimed| claimed == id)
    }

    /// Returns the reward value only once, and only after its lifetime goal is met.
    pub(super) fn claim(&mut self, id: &str) -> Option<u32> {
        let achievement = ACHIEVEMENTS.iter().find(|a| a.id == id)?;
        if !self.unlocked(achievement) || self.reward_claimed(id) {
            return None;
        }
        self.claimed_rewards.push(id.to_owned());
        Some(achievement.reward_gold)
    }

    /// Applies one completed run idempotently. Keep IDs for the life of the save so even old
    /// suspended sessions cannot be replayed to duplicate lifetime credit.
    pub(super) fn settle_run(&mut self, id: &str, summary: RunSummary) -> bool {
        if id.is_empty() || self.settled_runs.iter().any(|done| done == id) {
            return false;
        }
        self.settled_runs.push(id.to_owned());
        self.lifetime_runs = self.lifetime_runs.saturating_add(1);
        self.lifetime_kills = self.lifetime_kills.saturating_add(summary.kills);
        self.lifetime_bosses = self.lifetime_bosses.saturating_add(summary.bosses);
        self.lifetime_gold = self.lifetime_gold.saturating_add(summary.gold_earned);
        self.lifetime_chests = self.lifetime_chests.saturating_add(summary.chests);
        self.lifetime_survival_seconds = self
            .lifetime_survival_seconds
            .saturating_add(summary.survival_seconds);
        self.lifetime_damage_taken = self
            .lifetime_damage_taken
            .saturating_add(summary.damage_taken);
        self.lifetime_elites = self.lifetime_elites.saturating_add(summary.elites);
        self.lifetime_guardians = self.lifetime_guardians.saturating_add(summary.guardians);
        true
    }

    /// Talent XP is stored per class and converted at 100 XP per level, capped by vanilla level 60.
    pub(super) fn award_talent_xp(&mut self, class: u8, summary: RunSummary) -> u64 {
        let xp = summary.survival_seconds / 5
            + summary.kills.div_ceil(10)
            + summary.bosses.saturating_mul(75)
            + summary.elites.saturating_mul(10)
            + summary.guardians.saturating_mul(50);
        if xp == 0 {
            return 0;
        }
        if let Some(entry) = self.class_xp.iter_mut().find(|entry| entry.class == class) {
            entry.xp = entry.xp.saturating_add(xp);
        } else {
            self.class_xp.push(ClassTalentXp { class, xp });
        }
        xp
    }

    pub(super) fn class_level(&self, class: u8) -> u32 {
        let mut xp = self
            .class_xp
            .iter()
            .find(|entry| entry.class == class)
            .map_or(0, |entry| entry.xp);
        let mut points = 0;
        while points < 51 && xp >= 80 + points * 15 {
            xp -= 80 + points * 15;
            points += 1;
        }
        1 + points as u32
    }
}
