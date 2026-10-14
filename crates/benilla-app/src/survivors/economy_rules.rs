//! Deterministic purchases, equipment and potion rules, independent of rendering.
use serde::{Deserialize, Serialize};
pub const POTION_PRICE: u32 = 20;
pub const POTION_LIMIT: u8 = 3;
pub const POTION_COOLDOWN: f32 = 20.0;
pub const MAX_TIER: u8 = 8;
pub const MAX_QUALITY: u8 = 4;
pub const MAX_INVENTORY: usize = 64;
pub const MAX_ITEM_ID: u64 = (u32::MAX - 900_000) as u64;

/// A permanent boss drop. Quality is 0 white through 4 legendary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedItem {
    pub id: u64,
    pub slot: usize,
    pub quality: u8,
    pub tier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub gold: u32,
    pub potions: u8,
    /// Weapon, chest armour, ring, boots. Equipment tiers always upgrade, never replace downwards.
    pub gear: [u8; 4],
    /// Ownership stays in `gear`; these flags control whether each owned item contributes stats.
    pub equipped: [bool; 4],
    /// Permanent boss equipment, retained across runs.
    pub inventory: Vec<OwnedItem>,
    /// Item ids worn in weapon, chest, ring and boot slots.
    pub worn: [Option<u64>; 4],
    pub next_item_id: u64,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            gold: 0,
            potions: 0,
            gear: [0; 4],
            equipped: [true; 4],
            inventory: Vec::new(),
            worn: [None; 4],
            next_item_id: 1,
        }
    }
}

pub fn chest_price(opened: u32) -> u32 {
    40u32.saturating_add(opened.saturating_mul(20))
}

impl Account {
    /// Grant a permanent drop with a stable, account-local id.
    pub fn grant_boss_item(
        &mut self,
        slot: usize,
        quality: u8,
        tier: u8,
    ) -> Result<u64, &'static str> {
        if slot >= 4 {
            return Err("Emplacement d’équipement invalide.");
        }
        if quality > MAX_QUALITY {
            return Err("Qualité d’objet invalide.");
        }
        if tier > MAX_TIER {
            return Err("Niveau d’objet invalide.");
        }
        if self.inventory.len() >= MAX_INVENTORY {
            return Err("Votre inventaire est plein.");
        }
        let id = self.next_item_id;
        if id == 0 || id > MAX_ITEM_ID {
            return Err("Identifiant d’objet épuisé.");
        }
        self.next_item_id = id.checked_add(1).ok_or("Identifiant d’objet épuisé.")?;
        self.inventory.push(OwnedItem {
            id,
            slot,
            quality,
            tier,
        });
        Ok(id)
    }

    /// A run chest improves temporary gear only; permanent boss items remain untouched.
    pub fn open_world_chest(
        &mut self,
        temporary: &mut [u8; 4],
        opened: &mut u32,
        roll: usize,
    ) -> Result<usize, &'static str> {
        let available: Vec<usize> = (0..4).filter(|&slot| temporary[slot] < MAX_TIER).collect();
        if available.is_empty() {
            return Err("Votre équipement temporaire est déjà au niveau maximal.");
        }
        let price = chest_price(*opened);
        if self.gold < price {
            return Err("Vous n'avez pas assez d'or pour ce coffre.");
        }
        let slot = available[roll % available.len()];
        self.gold -= price;
        temporary[slot] += 1;
        *opened = opened.saturating_add(1);
        Ok(slot)
    }

    /// Wear an owned item, returning the displaced item to the inventory.
    pub fn equip_item(&mut self, id: u64) -> Result<Option<u64>, &'static str> {
        let item = self
            .inventory
            .iter()
            .find(|item| item.id == id)
            .ok_or("Objet introuvable.")?;
        let slot = item.slot;
        let displaced = self.worn[slot].replace(id);
        self.equipped[slot] = true;
        self.gear[slot] = item.tier;
        Ok(displaced)
    }

    pub fn remove_worn(&mut self, slot: usize) -> Result<Option<u64>, &'static str> {
        if slot >= 4 {
            return Err("Emplacement d’équipement invalide.");
        }
        let removed = self.worn[slot].take();
        if removed.is_some() {
            self.gear[slot] = 0;
            self.equipped[slot] = false;
        }
        Ok(removed)
    }

    /// Fuse three unequipped items of the same slot and quality into one higher-quality item.
    /// Validation completes before any inventory or id mutation.
    pub fn fuse_items(&mut self, ids: [u64; 3]) -> Result<u64, &'static str> {
        if ids[0] == ids[1] || ids[0] == ids[2] || ids[1] == ids[2] {
            return Err("Choisissez trois objets différents.");
        }
        let selected: Vec<_> = ids
            .iter()
            .map(|id| {
                self.inventory
                    .iter()
                    .find(|item| item.id == *id)
                    .ok_or("Objet introuvable.")
            })
            .collect::<Result<_, _>>()?;
        let first = selected[0];
        if first.quality >= MAX_QUALITY {
            return Err("Cet objet a déjà la qualité maximale.");
        }
        if selected.iter().any(|item| {
            item.slot != first.slot
                || item.quality != first.quality
                || self.worn[item.slot] == Some(item.id)
        }) {
            return Err("Fusion impossible : objets équipés ou incompatibles.");
        }
        let (slot, quality) = (first.slot, first.quality);
        let tier = selected
            .iter()
            .map(|item| item.tier)
            .max()
            .unwrap_or(first.tier);
        if tier >= MAX_TIER {
            return Err("Le niveau de cet objet a déjà atteint son maximum.");
        }
        let id = self.next_item_id;
        if id == 0 || id > MAX_ITEM_ID {
            return Err("Identifiant d’objet épuisé.");
        }
        let next_id = id.checked_add(1).ok_or("Identifiant d’objet épuisé.")?;
        let tier = tier + 1;
        self.inventory.retain(|item| !ids.contains(&item.id));
        self.inventory.push(OwnedItem {
            id,
            slot,
            quality: quality + 1,
            tier,
        });
        self.next_item_id = next_id;
        Ok(id)
    }

    pub fn gear_effective(&self) -> [u8; 4] {
        std::array::from_fn(|slot| {
            if self.equipped[slot] {
                self.gear[slot]
            } else {
                0
            }
        })
    }

    /// No gold is charged, no item destroyed, and no potion stock affected.
    pub fn set_equipped(&mut self, slot: usize, equipped: bool) -> Result<(), &'static str> {
        if slot >= self.gear.len() {
            return Err("Emplacement d’équipement invalide.");
        }
        if self.gear[slot] == 0 {
            return Err("Vous ne possédez pas d’objet dans cet emplacement.");
        }
        self.equipped[slot] = equipped;
        Ok(())
    }

    pub fn toggle_equipped(&mut self, slot: usize) -> Result<bool, &'static str> {
        let Some(&equipped) = self.equipped.get(slot) else {
            return Err("Emplacement d’équipement invalide.");
        };
        self.set_equipped(slot, !equipped)?;
        Ok(!equipped)
    }
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
