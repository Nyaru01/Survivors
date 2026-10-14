//! Persistent classic talent points and allocation validation, independent of UI, DBC and Bevy.
use serde::{Deserialize, Serialize};

pub const MAX_LEVEL: u8 = 60;
pub const MAX_POINTS: u8 = 51;
const CLASSES: [u8; 9] = [1, 2, 3, 4, 5, 7, 8, 9, 11];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalentProgress {
    #[serde(default)]
    pub profiles: Vec<ClassProfile>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassProfile {
    pub class: u8,
    pub highest_level: u8,
    #[serde(default)]
    pub ranks: Vec<TalentRank>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalentRank {
    pub id: u32,
    pub rank: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prerequisite {
    pub id: u32,
    /// One-based rank required in the prerequisite talent (convert the DBC's rank index).
    pub rank: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TalentDef {
    pub id: u32,
    pub class: u8,
    /// The real TalentTab id, not a position in a screen.
    pub tree: u32,
    /// Zero-based row; each row needs five points in earlier rows of this same tree.
    pub tier: u8,
    pub max_rank: u8,
    pub prerequisite: Option<Prerequisite>,
    pub required_spell: Option<u32>,
    /// The effect adapter must explicitly opt in; unimplemented effects never spend points.
    pub supported: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalentError {
    InvalidClass,
    InvalidLevel,
    DuplicateProfile,
    DuplicateTalent,
    UnknownTalent,
    WrongClass,
    WrongTree,
    InvalidDefinition,
    InvalidRank,
    MaximumRank,
    Unsupported,
    RequiredSpell,
    Prerequisite,
    Tier,
    NoPoints,
    Overspent,
}

impl std::fmt::Display for TalentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidClass => "Classe de talents invalide.",
            Self::InvalidLevel => "Niveau de progression des talents invalide.",
            Self::DuplicateProfile => "Plusieurs profils de talents existent pour cette classe.",
            Self::DuplicateTalent => "Ce talent apparaît plusieurs fois dans les données.",
            Self::UnknownTalent => "Ce talent est absent du catalogue du client.",
            Self::WrongClass => "Ce talent appartient à une autre classe.",
            Self::WrongTree => "Ce talent appartient à un autre arbre.",
            Self::InvalidDefinition => "Les prérequis de ce talent sont incohérents.",
            Self::InvalidRank => "Le rang enregistré pour ce talent est invalide.",
            Self::MaximumRank => "Ce talent est déjà au rang maximal.",
            Self::Unsupported => {
                "L’effet de ce talent n’est pas encore pris en charge dans Survivors."
            }
            Self::RequiredSpell => "Vous devez d’abord apprendre le sort requis par ce talent.",
            Self::Prerequisite => "Vous devez atteindre le rang requis dans le talent précédent.",
            Self::Tier => "Il faut davantage de points dans les paliers précédents de cet arbre.",
            Self::NoPoints => "Vous n’avez aucun point de talent disponible.",
            Self::Overspent => "Les talents enregistrés dépassent le nombre de points gagnés.",
        };
        f.write_str(message)
    }
}

impl ClassProfile {
    pub fn earned(&self) -> u8 {
        self.highest_level.min(MAX_LEVEL).saturating_sub(9)
    }
    pub fn spent(&self) -> u32 {
        self.ranks.iter().map(|r| u32::from(r.rank)).sum()
    }
    pub fn available(&self) -> u8 {
        u32::from(self.earned()).saturating_sub(self.spent()) as u8
    }
    pub fn rank(&self, id: u32) -> u8 {
        self.ranks.iter().find(|r| r.id == id).map_or(0, |r| r.rank)
    }
    pub fn tree_points(&self, tree: u32, catalog: &[TalentDef]) -> u32 {
        self.ranks
            .iter()
            .filter_map(|r| {
                catalog
                    .iter()
                    .find(|d| d.id == r.id && d.class == self.class && d.tree == tree)
                    .map(|_| u32::from(r.rank))
            })
            .sum()
    }
    fn earlier_points(&self, def: &TalentDef, catalog: &[TalentDef]) -> u32 {
        self.ranks
            .iter()
            .filter_map(|r| {
                catalog
                    .iter()
                    .find(|d| {
                        d.id == r.id
                            && d.class == self.class
                            && d.tree == def.tree
                            && d.tier < def.tier
                    })
                    .map(|_| u32::from(r.rank))
            })
            .sum()
    }
}

impl TalentProgress {
    pub fn profile(&self, class: u8) -> Option<&ClassProfile> {
        self.profiles.iter().find(|p| p.class == class)
    }
    pub fn rank(&self, class: u8, id: u32) -> u8 {
        self.profile(class).map_or(0, |p| p.rank(id))
    }
    pub fn available(&self, class: u8) -> u32 {
        self.profile(class).map_or(0, |p| u32::from(p.available()))
    }
    pub fn highest_level(&self, class: u8) -> u32 {
        self.profile(class)
            .map_or(1, |p| u32::from(p.highest_level))
    }
    /// Points come from the highest level ever observed, so replaying early levels grants none.
    pub fn observe_level(&mut self, class: u8, level: u32) -> Result<(), TalentError> {
        if !CLASSES.contains(&class) {
            return Err(TalentError::InvalidClass);
        }
        if level == 0 {
            return Err(TalentError::InvalidLevel);
        }
        let level = level.min(u32::from(MAX_LEVEL)) as u8;
        if let Some(p) = self.profiles.iter_mut().find(|p| p.class == class) {
            p.highest_level = p.highest_level.max(level);
        } else {
            self.profiles.push(ClassProfile {
                class,
                highest_level: level,
                ranks: Vec::new(),
            });
        }
        Ok(())
    }

    /// Validate the save's structure without requiring a particular client's talent catalog.
    pub fn validate_basic(&self) -> Result<(), TalentError> {
        for (i, p) in self.profiles.iter().enumerate() {
            if !CLASSES.contains(&p.class) {
                return Err(TalentError::InvalidClass);
            }
            if !(1..=MAX_LEVEL).contains(&p.highest_level) {
                return Err(TalentError::InvalidLevel);
            }
            if self.profiles[..i].iter().any(|v| v.class == p.class) {
                return Err(TalentError::DuplicateProfile);
            }
            for (j, r) in p.ranks.iter().enumerate() {
                if r.id == 0 || !(1..=5).contains(&r.rank) {
                    return Err(TalentError::InvalidRank);
                }
                if p.ranks[..j].iter().any(|v| v.id == r.id) {
                    return Err(TalentError::DuplicateTalent);
                }
            }
            if p.spent() > u32::from(p.earned()) {
                return Err(TalentError::Overspent);
            }
        }
        Ok(())
    }

    fn definition<'a>(
        class: u8,
        id: u32,
        catalog: &'a [TalentDef],
    ) -> Result<&'a TalentDef, TalentError> {
        let def = catalog
            .iter()
            .find(|d| d.id == id)
            .ok_or(TalentError::UnknownTalent)?;
        if catalog.iter().filter(|d| d.id == id).count() != 1 {
            return Err(TalentError::DuplicateTalent);
        }
        if def.class != class {
            return Err(TalentError::WrongClass);
        }
        if def.tree == 0 {
            return Err(TalentError::WrongTree);
        }
        if def.id == 0 || def.tier > 6 || !(1..=5).contains(&def.max_rank) {
            return Err(TalentError::InvalidDefinition);
        }
        Ok(def)
    }

    fn requirements(
        p: &ClassProfile,
        def: &TalentDef,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        if !def.supported {
            return Err(TalentError::Unsupported);
        }
        if def
            .required_spell
            .is_some_and(|spell| !spells.contains(&spell))
        {
            return Err(TalentError::RequiredSpell);
        }
        let mut cursor = def;
        let mut seen = vec![def.id];
        while let Some(req) = cursor.prerequisite {
            let parent = Self::definition(p.class, req.id, catalog)?;
            if seen.contains(&parent.id)
                || parent.tree != def.tree
                || req.rank == 0
                || req.rank > parent.max_rank
                || parent.tier > cursor.tier
            {
                return Err(TalentError::InvalidDefinition);
            }
            seen.push(parent.id);
            cursor = parent;
        }
        if let Some(req) = def.prerequisite {
            let parent = Self::definition(p.class, req.id, catalog)?;
            if parent.tree != def.tree
                || parent.id == def.id
                || req.rank == 0
                || req.rank > parent.max_rank
                || parent.tier > def.tier
            {
                return Err(TalentError::InvalidDefinition);
            }
            if p.rank(req.id) < req.rank {
                return Err(TalentError::Prerequisite);
            }
        }
        if p.earlier_points(def, catalog) < u32::from(def.tier) * 5 {
            return Err(TalentError::Tier);
        }
        Ok(())
    }

    /// Strict validation is immutable: a catalog mismatch is reported, never silently erased.
    pub fn validate_profile(
        &self,
        class: u8,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        self.validate_basic()?;
        if !CLASSES.contains(&class) {
            return Err(TalentError::InvalidClass);
        }
        let Some(p) = self.profile(class) else {
            return Ok(());
        };
        for rank in &p.ranks {
            let def = Self::definition(class, rank.id, catalog)?;
            if rank.rank > def.max_rank {
                return Err(TalentError::InvalidRank);
            }
            Self::requirements(p, def, catalog, spells)?;
        }
        Ok(())
    }
    pub fn validate_catalog(
        &self,
        class: u8,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        self.validate_profile(class, catalog, spells)
    }

    pub fn can_allocate(
        &self,
        class: u8,
        id: u32,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        self.validate_profile(class, catalog, spells)?;
        let def = Self::definition(class, id, catalog)?;
        let Some(p) = self.profile(class) else {
            return Err(TalentError::NoPoints);
        };
        Self::requirements(p, def, catalog, spells)?;
        if p.rank(id) >= def.max_rank {
            return Err(TalentError::MaximumRank);
        }
        if p.available() == 0 {
            return Err(TalentError::NoPoints);
        }
        Ok(())
    }

    /// Spend exactly one point after every check succeeded; any failure leaves the build intact.
    pub fn allocate(
        &mut self,
        class: u8,
        id: u32,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        self.can_allocate(class, id, catalog, spells)?;
        let p = self
            .profiles
            .iter_mut()
            .find(|p| p.class == class)
            .ok_or(TalentError::NoPoints)?;
        if let Some(rank) = p.ranks.iter_mut().find(|r| r.id == id) {
            rank.rank += 1;
        } else {
            p.ranks.push(TalentRank { id, rank: 1 });
        }
        Ok(())
    }

    /// Useful when a tab click supplies an explicit tree id; a talent from another tab is refused.
    pub fn allocate_in_tree(
        &mut self,
        class: u8,
        tree: u32,
        id: u32,
        catalog: &[TalentDef],
        spells: &[u32],
    ) -> Result<(), TalentError> {
        if Self::definition(class, id, catalog)?.tree != tree {
            return Err(TalentError::WrongTree);
        }
        self.allocate(class, id, catalog, spells)
    }
}
