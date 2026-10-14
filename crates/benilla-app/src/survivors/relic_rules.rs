//! Run-only equipment that strengthens one known class spell without consuming spell ranks.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum RelicKind {
    Grimoire,
    Rune,
    Focus,
    Precision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RelicSlot {
    Ring,
    Trinket,
    Amulet,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SpellRelic {
    pub spell: String,
    pub kind: RelicKind,
}

impl RelicKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Grimoire => "Anneau de puissance",
            Self::Rune => "Anneau de célérité",
            Self::Focus => "Ampleur de Main-Noire",
            Self::Precision => "Amulette de Sombrelune",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::Grimoire => "+40 % dégâts pour ce sort",
            Self::Rune => "Recharge réduite de 35 % pour ce sort",
            Self::Focus => "+20 % de rayon pour ce sort",
            Self::Precision => "+20 % chances de coup critique pour ce sort",
        }
    }

    pub fn damage_mult(self) -> f32 {
        match self {
            Self::Grimoire => 1.4,
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
        0
    }

    pub fn area_mult(self) -> f32 {
        if matches!(self, Self::Focus) {
            1.2
        } else {
            1.0
        }
    }

    pub fn crit_bonus(self) -> f32 {
        if matches!(self, Self::Precision) {
            0.2
        } else {
            0.0
        }
    }

    pub fn slot(self) -> RelicSlot {
        match self {
            Self::Grimoire | Self::Rune => RelicSlot::Ring,
            Self::Focus => RelicSlot::Trinket,
            Self::Precision => RelicSlot::Amulet,
        }
    }

    pub fn capacity(self) -> usize {
        match self.slot() {
            RelicSlot::Ring => 2,
            RelicSlot::Trinket => 2,
            RelicSlot::Amulet => 1,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Grimoire => r"Interface\Icons\INV_Jewelry_Ring_01",
            Self::Rune => r"Interface\Icons\INV_Jewelry_Ring_05",
            Self::Focus => r"Interface\Icons\INV_Misc_EngGizmos_18",
            Self::Precision => r"Interface\Icons\INV_Jewelry_Necklace_01",
        }
    }

    pub fn rarity(self) -> u8 {
        match self {
            Self::Rune => 2,
            Self::Grimoire | Self::Precision => 3,
            Self::Focus => 4,
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
