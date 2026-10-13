//! Deterministic purchases, equipment and potion rules, independent of rendering.
pub const POTION_PRICE: u32 = 20;
pub const POTION_LIMIT: u8 = 3;
pub const POTION_COOLDOWN: f32 = 20.0;
pub const MAX_TIER: u8 = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Account {
    pub gold: u32,
    pub potions: u8,
    /// Weapon, chest armour, ring, boots. Equipment tiers always upgrade, never replace downwards.
    pub gear: [u8; 4],
}

pub fn chest_price(opened: u32) -> u32 {
    40u32.saturating_add(opened.saturating_mul(20))
}

impl Account {
    pub fn buy_potion(&mut self, stock: &mut u8) -> Result<(), &'static str> {
        if *stock == 0 {
            return Err("Stock de potions épuisé pour cette partie.");
        }
        if self.potions >= POTION_LIMIT {
            return Err("Vous portez déjà 3 potions.");
        }
        if self.gold < POTION_PRICE {
            return Err("Il faut 20 pièces d'or.");
        }
        self.gold -= POTION_PRICE;
        self.potions += 1;
        *stock -= 1;
        Ok(())
    }

    pub fn open_chest(&mut self, opened: &mut u32, roll: usize) -> Result<usize, &'static str> {
        let available: Vec<usize> = (0..4).filter(|&s| self.gear[s] < MAX_TIER).collect();
        if available.is_empty() {
            return Err("Votre équipement est déjà au niveau maximal.");
        }
        let price = chest_price(*opened);
        if self.gold < price {
            return Err("Vous n'avez pas assez d'or pour ce coffre.");
        }
        let slot = available[roll % available.len()];
        self.gold -= price;
        self.gear[slot] += 1;
        *opened = opened.saturating_add(1);
        Ok(slot)
    }

    pub fn drink(&mut self, hp: f32, max_hp: f32, cooldown: &mut f32) -> Result<f32, &'static str> {
        if hp <= 0.0 {
            return Err("Impossible de boire une potion après la mort.");
        }
        if self.potions == 0 {
            return Err("Vous n'avez pas de potion de soins.");
        }
        if *cooldown > 0.0 {
            return Err("La potion est encore en recharge.");
        }
        if hp >= max_hp {
            return Err("Votre vie est déjà au maximum.");
        }
        self.potions -= 1;
        *cooldown = POTION_COOLDOWN;
        Ok((hp + max_hp * 0.5).min(max_hp))
    }
}

pub fn gear_name(class: u8, slot: usize) -> &'static str {
    match slot {
        0 => match class {
            1 | 2 => "Épée",
            3 => "Arc",
            4 => "Dagues",
            7 => "Masse",
            _ => "Bâton",
        },
        1 => match class {
            1 | 2 => "Armure de plaques",
            3 | 7 => "Armure de mailles",
            4 | 11 => "Armure de cuir",
            _ => "Robe de tissu",
        },
        2 => "Anneau de protection",
        _ => "Bottes de rapidité",
    }
}

pub fn gear_bonus(slot: usize, tier: u8) -> String {
    match slot {
        0 => format!("Dégâts +{} %", u32::from(tier) * 8),
        1 => format!("Vie +{}", u32::from(tier) * 20),
        2 => format!("Armure +{}", u32::from(tier) * 2),
        _ => format!("Vitesse +{} %", u32::from(tier) * 3),
    }
}
