//! Run-only equipment that strengthens one known class spell without consuming spell ranks.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum RelicKind {
    Grimoire,
    Rune,
    Focus,
    Precision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SpellRelic {
    pub spell: String,
    pub kind: RelicKind,
}

impl RelicKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Grimoire => "Grimoire de puissance",
            Self::Rune => "Rune de célérité",
            Self::Focus => "Focaliseur arcanique",
            Self::Precision => "Lentille de précision",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::Grimoire => "+40 % dégâts pour ce sort",
            Self::Rune => "Recharge réduite de 35 % pour ce sort",
            Self::Focus => "+1 cible et +20 % dégâts pour ce sort",
            Self::Precision => "+20 % chances de coup critique pour ce sort",
        }
    }

    pub fn damage_mult(self) -> f32 {
        match self {
            Self::Grimoire => 1.4,
            Self::Focus => 1.2,
            _ => 1.0,
        }
    }

    pub fn cooldown_mult(self) -> f32 {
        match self {
            Self::Rune => 0.65,
            _ => 1.0,
        }
    }

    pub fn extra_targets(self) -> u32 {
        u32::from(matches!(self, Self::Focus))
    }

    pub fn crit_bonus(self) -> f32 {
        if matches!(self, Self::Precision) {
            0.2
        } else {
            0.0
        }
    }
}

pub(super) fn for_spell<'a>(
    relics: &'a [SpellRelic],
    key: &'a str,
) -> impl Iterator<Item = RelicKind> + 'a {
    relics
        .iter()
        .filter(move |r| r.spell == key)
        .map(|r| r.kind)
}
