//! Multi-character envelope for the local Survivors save. The flat v1 save remains readable and
//! is wrapped as the first slot on its next write.
use super::save_format as fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Slot {
    pub id: String,
    pub save: fmt::Save,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    roster_version: u32,
    active_id: String,
    slots: Vec<Slot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RosterEntry {
    pub id: String,
    pub name: String,
    pub class: usize,
    pub map: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Roster {
    pub active_id: String,
    pub slots: Vec<Slot>,
}

impl Roster {
    pub fn empty() -> Self {
        Self {
            active_id: String::new(),
            slots: Vec::new(),
        }
    }

    pub fn summaries(&self) -> Vec<RosterEntry> {
        self.slots
            .iter()
            .map(|slot| RosterEntry {
                id: slot.id.clone(),
                name: slot.save.character.name.clone(),
                class: slot.save.character.class,
                map: slot.save.character.map,
            })
            .collect()
    }

    pub fn active(&self) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.id == self.active_id)
    }

    pub fn active_mut(&mut self) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|slot| slot.id == self.active_id)
    }

    pub fn select(&mut self, id: &str) -> bool {
        if self.slots.iter().any(|slot| slot.id == id) {
            self.active_id = id.to_owned();
            true
        } else {
            false
        }
    }

    /// Add an independent profile with fresh account, world and talent progress.
    pub fn create(&mut self, name: String) -> Option<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 24 || name.chars().any(char::is_control) {
            return None;
        }
        self.create_character(fmt::Character {
            name: name.to_owned(),
            class: 0,
            race: 1,
            sex: 0,
            map: 0,
            appearance: [0; 5],
        })
    }

    pub fn create_character(&mut self, character: fmt::Character) -> Option<String> {
        if !fmt::valid_character(&character) {
            return None;
        }
        let n = self
            .slots
            .iter()
            .filter_map(|slot| slot.id.strip_prefix("character-")?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            .checked_add(1)?;
        let id = format!("character-{n}");
        self.slots.push(Slot {
            id: id.clone(),
            save: fmt::Save::fresh(character),
        });
        self.active_id = id.clone();
        Some(id)
    }
}

pub(super) fn decode(text: &str) -> Result<Roster, String> {
    let value: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    if value.get("roster_version").is_none() {
        let save = fmt::decode(text)?;
        return Ok(Roster {
            active_id: "character-1".into(),
            slots: vec![Slot {
                id: "character-1".into(),
                save,
            }],
        });
    }
    let doc: Document = value
        .try_into()
        .map_err(|e: toml::de::Error| e.to_string())?;
    if doc.roster_version != 1 {
        return Err(format!(
            "Version de liste de personnages inconnue : {}",
            doc.roster_version
        ));
    }
    if doc.slots.is_empty()
        || doc.slots.iter().any(|slot| {
            slot.id.is_empty()
                || !fmt::valid_character(&slot.save.character)
                || slot
                    .save
                    .run
                    .as_ref()
                    .is_some_and(|run| !fmt::valid_character(&run.character))
        })
        || doc
            .slots
            .iter()
            .enumerate()
            .any(|(i, slot)| doc.slots[..i].iter().any(|previous| previous.id == slot.id))
        || !doc.slots.iter().any(|slot| slot.id == doc.active_id)
    {
        return Err("Liste de personnages invalide".into());
    }
    for slot in &doc.slots {
        // Validate every independent save with the same rules as the legacy format.
        let text = toml::to_string(&slot.save).map_err(|e| e.to_string())?;
        fmt::decode(&text)?;
    }
    Ok(Roster {
        active_id: doc.active_id,
        slots: doc.slots,
    })
}

pub(super) fn encode(roster: &Roster) -> Result<String, String> {
    toml::to_string(&Document {
        roster_version: 1,
        active_id: roster.active_id.clone(),
        slots: roster.slots.clone(),
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy() -> fmt::Save {
        fmt::Save::fresh(fmt::Character {
            name: "Élodie".into(),
            class: 0,
            race: 1,
            sex: 1,
            map: 0,
            appearance: [1, 2, 3, 4, 5],
        })
    }

    #[test]
    fn flat_legacy_save_migrates_without_losing_any_progress_or_run() {
        let mut save = legacy();
        save.gold = 371;
        save.talents.observe_level(1, 12).unwrap();
        save.worlds.complete(1, 5, 4, true).unwrap();
        let original = toml::to_string(&save).unwrap();
        let roster = decode(&original).unwrap();
        assert_eq!(roster.active().unwrap().save.gold, 371);
        assert_eq!(roster.active().unwrap().save.talents, save.talents);
        assert_eq!(roster.active().unwrap().save.worlds, save.worlds);
        let restored = decode(&encode(&roster).unwrap()).unwrap();
        assert_eq!(restored.active().unwrap().save.character.name, "Élodie");
        assert_eq!(restored.active().unwrap().save.gold, 371);
    }

    #[test]
    fn slots_switch_and_new_slot_has_no_copied_progression() {
        let mut roster = decode(&toml::to_string(&legacy()).unwrap()).unwrap();
        roster.slots[0].save.gold = 99;
        let second = roster.create("Nouveau".into()).unwrap();
        assert_eq!(roster.active_id, second);
        assert_eq!(roster.active().unwrap().save.gold, 0);
        assert!(roster.active().unwrap().save.talents.profiles.is_empty());
        assert!(roster.select("character-1"));
        assert_eq!(roster.active().unwrap().save.gold, 99);
        assert!(!roster.select("missing"));
        assert!(roster.create(" ".into()).is_none());
        let persisted = decode(&encode(&roster).unwrap()).unwrap();
        assert_eq!(persisted.active_id, "character-1");
        assert_eq!(persisted.summaries().len(), 2);
        assert_eq!(persisted.slots[1].save.character.name, "Nouveau");
        assert_eq!(persisted.slots[1].save.gold, 0);
    }

    #[test]
    fn creating_from_character_configuration_keeps_looks_without_copying_progress() {
        let mut roster = decode(&toml::to_string(&legacy()).unwrap()).unwrap();
        roster.slots[0].save.gold = 99;
        roster.slots[0].save.talents.observe_level(1, 16).unwrap();
        let character = fmt::Character {
            name: "Nouvelle".into(),
            class: 2,
            race: 4,
            sex: 1,
            map: 3,
            appearance: [1, 2, 3, 4, 5],
        };
        roster.create_character(character.clone()).unwrap();
        let fresh = roster.active().unwrap().save.clone();
        assert_eq!(fresh.character.name, character.name);
        assert_eq!(fresh.character.class, character.class);
        assert_eq!(fresh.character.race, character.race);
        assert_eq!(fresh.character.appearance, character.appearance);
        assert_eq!(fresh.gold, 0);
        assert!(fresh.talents.profiles.is_empty());
    }

    #[test]
    fn invalid_active_id_and_duplicate_ids_are_rejected() {
        let mut roster = decode(&toml::to_string(&legacy()).unwrap()).unwrap();
        roster.slots.push(roster.slots[0].clone());
        assert!(decode(&encode(&roster).unwrap()).is_err());
        roster.slots.pop();
        roster.active_id = "missing".into();
        assert!(decode(&encode(&roster).unwrap()).is_err());
    }
}
