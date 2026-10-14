//! Offline talent modifiers derived from the selected highest-rank DBC spells.
//! Only unconditional effects that the mode can reproduce are admitted. Never stack all
//! earlier rank spells: each learned talent contributes its current rank's spell once.
//! Aura 107/108's selection mask is EffectItemType, NOT the talent's SpellFamilyFlags.
use benilla_formats::{SpellCatalog, SpellDisplay};

/// Flat and percentage additions sum independently, then apply flat first, percentage second.
/// Percent is an additive percentage (5 means +5%), not a ratio. Crit flat uses percentage points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Modifier {
    pub flat: f32,
    pub percent: f32,
}

impl Modifier {
    pub fn apply(self, value: f32) -> f32 {
        ((value + self.flat) * (1.0 + self.percent / 100.0).max(0.0)).max(0.0)
    }
    fn add(&mut self, other: Self) {
        self.flat += other.flat;
        self.percent += other.percent;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Damage,
    Dot,
    Cooldown,
    Crit,
    Range,
    Duration,
}

#[derive(Clone, Debug)]
pub struct SpellModifier {
    pub source_spell: u32,
    /// 0 means any family (used only by school-wide effects).
    pub family: u32,
    /// 0 means any spell in the school selector; scoped spell mods require a nonzero mask.
    pub family_mask: u64,
    /// 0 means all schools; otherwise a bitset 1 << SpellDisplay.school.
    pub school_mask: u32,
    pub operation: Operation,
    pub amount: Modifier,
}

impl SpellModifier {
    pub fn matches(&self, spell: &SpellDisplay) -> bool {
        (self.family == 0 || self.family == spell.spell_family)
            && (self.family_mask == 0 || self.family_mask & spell.spell_family_flags != 0)
            && (self.school_mask == 0 || spell.school < 32 && self.school_mask & (1u32 << spell.school) != 0)
            // The stock client excludes this spell from spell-family modifiers.
            && (self.family_mask == 0 || spell.attributes_ex3 & 0x2000_0000 == 0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpellModifiers {
    pub damage: Modifier,
    pub dot: Modifier,
    /// Flat cooldown/duration values are seconds; the DBC millisecond values are converted once.
    pub cooldown: Modifier,
    pub crit: Modifier,
    /// Native world units (the game's displayed yards/metres conversion remains presentation).
    pub range: Modifier,
    pub duration: Modifier,
}

impl SpellModifiers {
    pub fn apply_damage(&self, base: f32) -> f32 {
        self.damage.apply(base)
    }
    pub fn apply_dot(&self, base: f32) -> f32 {
        self.dot.apply(self.damage.apply(base))
    }
    pub fn apply_cooldown(&self, seconds: f32) -> f32 {
        self.cooldown.apply(seconds)
    }
    pub fn apply_range(&self, base: f32) -> f32 {
        self.range.apply(base)
    }
    pub fn apply_duration(&self, seconds: f32) -> f32 {
        self.duration.apply(seconds)
    }
    /// Input and output are ratios (0.1 = 10%); DBC crit additions are percentage points.
    pub fn apply_crit(&self, ratio: f32) -> f32 {
        (self.crit.apply(ratio * 100.0) / 100.0).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct StatModifiers {
    pub max_hp: Modifier,
    pub speed: Modifier,
    /// Additive percentage points of avoiding a melee contact hit (parry/dodge together).
    /// The caller must roll once, clamp the combined chance, and apply it to contact hits only.
    pub avoid_percent: f32,
    // Vanilla armour cannot be mapped to the mode's flat damage blocked per hit. Such talents
    // remain unsupported until a real armour mitigation model exists, rather than inventing buffs.
}

#[derive(Clone, Debug, Default)]
pub struct Effects {
    pub spell_modifiers: Vec<SpellModifier>,
    pub stats: StatModifiers,
    pub unsupported: Vec<(u32, String)>,
    pub adaptations: Vec<String>,
}

impl Effects {
    pub fn for_spell(&self, spell: &SpellDisplay) -> SpellModifiers {
        let mut result = SpellModifiers::default();
        for modifier in self.spell_modifiers.iter().filter(|m| m.matches(spell)) {
            match modifier.operation {
                Operation::Damage => result.damage.add(modifier.amount),
                Operation::Dot => result.dot.add(modifier.amount),
                Operation::Cooldown => result.cooldown.add(modifier.amount),
                Operation::Crit => result.crit.add(modifier.amount),
                Operation::Range => result.range.add(modifier.amount),
                Operation::Duration => result.duration.add(modifier.amount),
            }
        }
        result
    }
}

/// Err contains a French explanation. Mixed supported/unsupported effects reject the entire
/// rank, so spending a point never grants an incomplete approximation of the described talent.
pub fn support_status(spell_id: u32, catalog: &SpellCatalog) -> Result<(), String> {
    analyze(spell_id, catalog).map(|_| ())
}

pub fn support_note(spell_id: u32, catalog: &SpellCatalog) -> Option<String> {
    let effects = analyze(spell_id, catalog).ok()?;
    (!effects.adaptations.is_empty()).then(|| effects.adaptations.join("\n"))
}

pub fn adaptation_notes(spell_id: u32, catalog: &SpellCatalog) -> Vec<String> {
    analyze(spell_id, catalog)
        .map(|effects| effects.adaptations)
        .unwrap_or_default()
}

pub fn derive(allocated_spell_ids: &[u32], catalog: &SpellCatalog) -> Effects {
    let mut result = Effects::default();
    let mut seen = std::collections::HashSet::new();
    for &spell in allocated_spell_ids {
        if !seen.insert(spell) {
            continue;
        }
        match analyze(spell, catalog) {
            Ok(part) => {
                result.spell_modifiers.extend(part.spell_modifiers);
                result.stats.max_hp.add(part.stats.max_hp);
                result.stats.speed.add(part.stats.speed);
                result.stats.avoid_percent += part.stats.avoid_percent;
                for note in part.adaptations {
                    if !result.adaptations.contains(&note) {
                        result.adaptations.push(note);
                    }
                }
            }
            Err(reason) => result.unsupported.push((spell, reason)),
        }
    }
    result
}

fn analyze(spell_id: u32, catalog: &SpellCatalog) -> Result<Effects, String> {
    let Some(spell) = catalog.get(spell_id) else {
        return Err("Données du talent introuvables.".into());
    };
    if !spell.passive {
        return Err(
            "Cette capacité active doit être reliée à un sort jouable de Survivors.".into(),
        );
    }
    if spell.proc_flags != 0 || spell.proc_charges != 0 || spell.stances != 0 {
        return Err(
            "Ce talent dépend d’un déclenchement ou d’une posture non pris en charge.".into(),
        );
    }
    // Survivors has no stance/form state: its effective stance mask is always zero. A
    // forbidden-form mask (e.g. Priest Shadowform) cannot match that mask and is irrelevant.
    // Positive required-form masks above still cannot be fulfilled and remain rejected.
    // Subclass/inventory masks are irrelevant without an equipped item class requirement.
    // Several genuine vanilla talents author those ignored masks as 0xffffffff.
    // Weapon requirements (class 2) are admitted only conditionally: the class adapter MUST
    // verify subclass and inventory masks against its actual virtual weapons before purchase.
    if spell.equipped_item_class != -1 && spell.equipped_item_class != 2 {
        return Err("Ce talent exige un type d’arme ou d’équipement non simulé.".into());
    }
    let mut result = Effects::default();
    let mut count = 0;
    for slot in 0..3 {
        let effect = spell.effects[slot];
        if effect == 0 {
            continue;
        }
        count += 1;
        if effect != 6 {
            return Err("Ce talent demande un effet de combat non pris en charge.".into());
        }
        if spell.effect_die_sides[slot] > 1
            || spell.effect_dice_per_level[slot] != 0
            || spell.effect_real_points_per_level[slot] != 0.0
        {
            return Err(
                "Ce talent utilise un effet aléatoire ou évolutif non pris en charge.".into(),
            );
        }
        let value = spell.effect_base_points[slot].saturating_add(1) as f32;
        let misc = spell.effect_misc_value[slot];
        let aura = spell.effect_apply_aura[slot];
        match aura {
            107 | 108 => {
                let operation = match misc {
                    0 => Operation::Damage, 1 => Operation::Duration, 5 => Operation::Range,
                    7 => Operation::Crit, 11 => Operation::Cooldown, 22 => Operation::Dot,
                    10 => {
                        result.adaptations.push("Adaptation Survivors : la réduction d’incantation réduit l’intervalle de lancement automatique et la recharge manuelle.".into());
                        Operation::Cooldown
                    },
                    8 => return Err("Ce talent modifie plusieurs effets : son application complète n’est pas disponible.".into()),
                    _ => return Err("L’effet précis de ce talent n’est pas encore simulé dans Survivors.".into()),
                };
                let mask = u64::from(spell.effect_item_type[slot]);
                if spell.spell_family == 0 || mask == 0 {
                    return Err(
                        "La sélection exacte des sorts affectés n’est pas disponible.".into(),
                    );
                }
                let mut amount = Modifier::default();
                if aura == 108 {
                    amount.percent = value;
                } else {
                    amount.flat = if matches!(operation, Operation::Cooldown | Operation::Duration)
                    {
                        value / 1000.0
                    } else {
                        value
                    };
                }
                result.spell_modifiers.push(SpellModifier {
                    source_spell: spell_id,
                    family: spell.spell_family,
                    family_mask: mask,
                    school_mask: 0,
                    operation,
                    amount,
                });
            }
            // MOD_DAMAGE_PERCENT_DONE is a school-specific modifier, not a class/global bonus.
            79 if misc > 0 && misc & !0x7f == 0 => result.spell_modifiers.push(SpellModifier {
                source_spell: spell_id,
                family: 0,
                family_mask: 0,
                school_mask: misc as u32,
                operation: Operation::Damage,
                amount: Modifier {
                    flat: 0.0,
                    percent: value,
                },
            }),
            // MOD_SPELL_CRIT_CHANCE excludes physical school; MOD_SPELL_CRIT_CHANCE_SCHOOL
            // uses the explicitly authored school mask.
            52 | 57 | 71 => {
                let mask = if aura == 52 {
                    1 // MOD_WEAPON_CRIT_PERCENT: physical attacks only, not spells of other schools.
                } else if aura == 57 {
                    0x7e
                } else if misc > 0 && misc & !0x7f == 0 {
                    misc as u32
                } else {
                    return Err("École du bonus de critique non résolue.".into());
                };
                result.spell_modifiers.push(SpellModifier {
                    source_spell: spell_id,
                    family: 0,
                    family_mask: 0,
                    school_mask: mask,
                    operation: Operation::Crit,
                    amount: Modifier {
                        flat: value,
                        percent: 0.0,
                    },
                });
            }
            34 => result.stats.max_hp.flat += value, // MOD_INCREASE_HEALTH
            133 => result.stats.max_hp.percent += value, // MOD_INCREASE_HEALTH_PERCENT
            31 => result.stats.speed.percent += value, // MOD_INCREASE_SPEED, unmounted run speed
            47 | 49 => result.stats.avoid_percent += value, // MOD_PARRY_PERCENT / MOD_DODGE_PERCENT
            80 | 137 => {
                match misc {
                    2 => {
                        result.stats.max_hp.percent += value;
                        result.adaptations.push("Adaptation Survivors : l’endurance augmente directement la vie maximale, sans reproduire les caractéristiques de base de WoW.".into());
                    }
                    0 => {
                        result.spell_modifiers.push(SpellModifier {
                            source_spell: spell_id, family: 0, family_mask: 0, school_mask: 1,
                            operation: Operation::Damage, amount: Modifier { flat: 0.0, percent: value },
                        });
                        result.adaptations.push("Adaptation Survivors : la force augmente directement les dégâts physiques, sans reproduire la puissance d’attaque de WoW.".into());
                    }
                    _ => return Err("Cette caractéristique (intelligence, esprit ou agilité) n’est pas simulée dans Survivors.".into()),
                }
            }
            22 | 83 | 101 => {
                return Err(
                    "Survivors ne simule pas encore la réduction de dégâts par l’armure de WoW."
                        .into(),
                )
            }
            _ => {
                return Err(
                    "Ce talent dépend d’une mécanique non prise en charge dans Survivors.".into(),
                )
            }
        }
    }
    if count == 0 {
        return Err("Aucun effet applicable n’a été trouvé pour ce talent.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn modifier_spell(id: u32, aura: u32, operation: i32, value: i32) -> SpellDisplay {
        let mut spell = SpellDisplay::default();
        spell.id = id;
        spell.passive = true;
        spell.spell_family = 3;
        spell.effects[0] = 6;
        spell.effect_apply_aura[0] = aura;
        spell.effect_misc_value[0] = operation;
        spell.effect_item_type[0] = 2;
        spell.effect_base_points[0] = value - 1;
        spell
    }
    #[test]
    fn cooldown_uses_dbc_milliseconds_and_exact_family_mask() {
        let catalog =
            SpellCatalog::from_displays([(11078, modifier_spell(11078, 107, 11, -500))].into());
        let effects = derive(&[11078, 11078], &catalog);
        assert!(effects.unsupported.is_empty());
        let mut target = SpellDisplay::default();
        target.spell_family = 3;
        target.spell_family_flags = 2;
        assert_eq!(effects.for_spell(&target).apply_cooldown(8.0), 7.5);
        target.spell_family_flags = 1;
        assert_eq!(effects.for_spell(&target).apply_cooldown(8.0), 8.0);
        target.spell_family_flags = 2;
        target.spell_family = 5;
        assert_eq!(effects.for_spell(&target).apply_cooldown(8.0), 8.0);
    }
    #[test]
    fn unsupported_effect_rejects_entire_talent() {
        let mut mixed = modifier_spell(1, 108, 0, 5);
        mixed.effects[1] = 6;
        mixed.effect_apply_aura[1] = 42;
        let catalog =
            SpellCatalog::from_displays([(1, mixed), (2, modifier_spell(2, 107, 10, -100))].into());
        let effects = derive(&[1], &catalog);
        assert_eq!(effects.unsupported.len(), 1);
        assert!(effects.spell_modifiers.is_empty());
    }
    #[test]
    fn cast_time_adaptation_is_explicit_and_ignored_weapon_masks_do_not_block() {
        let mut cast_time = modifier_spell(2, 107, 10, -100);
        cast_time.equipped_item_subclass_mask = u32::MAX;
        cast_time.equipped_item_inventory_type_mask = u32::MAX;
        let mut weapon = modifier_spell(3, 107, 11, -500);
        weapon.equipped_item_class = 4;
        let catalog = SpellCatalog::from_displays([(2, cast_time), (3, weapon)].into());
        assert!(support_status(2, &catalog).is_ok());
        assert!(support_status(3, &catalog).is_err());
        assert!(support_note(2, &catalog)
            .unwrap()
            .contains("Adaptation Survivors"));
        let effects = derive(&[2], &catalog);
        let mut target = SpellDisplay::default();
        target.spell_family = 3;
        target.spell_family_flags = 2;
        assert!((effects.for_spell(&target).apply_cooldown(2.0) - 1.9).abs() < 0.00001);
        assert_eq!(effects.adaptations.len(), 1);
    }
    #[test]
    fn flat_then_percent_and_crit_percentage_points_are_explicit() {
        assert_eq!(
            Modifier {
                flat: 10.0,
                percent: 20.0
            }
            .apply(100.0),
            132.0
        );
        let mods = SpellModifiers {
            crit: Modifier {
                flat: 5.0,
                percent: 0.0,
            },
            ..Default::default()
        };
        assert!((mods.apply_crit(0.1) - 0.15).abs() < 0.00001);
    }
}
