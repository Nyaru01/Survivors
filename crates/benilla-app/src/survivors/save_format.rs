//! Versioned, inspectable save data. No game-install files are touched.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Character {
    pub name: String,
    pub class: usize,
    pub race: u8,
    pub sex: u8,
    pub map: usize,
    pub appearance: [u8; 5],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Save {
    pub version: u32,
    pub character: Character,
    pub gold: u32,
    pub potions: u8,
    pub gear: [u8; 4],
    #[serde(default)]
    pub talents: super::talent_rules::TalentProgress,
    pub run: Option<Session>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Session {
    pub character: Character,
    pub elapsed: f32,
    pub hp: f32,
    pub xp: f32,
    pub level: u32,
    pub kills: u32,
    pub bosses: u32,
    pub rings: u32,
    pub pending_levels: u32,
    pub damage_done: f64,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub abilities: Vec<Spell>,
    pub passives: Vec<Spell>,
    pub effects: [f32; 10],
    pub thorns_spell: u32,
    pub mobile: [f32; 4],
    pub stock: u8,
    pub opened: u32,
    pub next_chest: f32,
    pub potion_cd: f32,
    pub enemies: Vec<Foe>,
    pub pickups: Vec<Pickup>,
    pub chests: Vec<[f32; 3]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Spell {
    pub key: String,
    pub rank: u8,
    pub cd: f32,
    pub automatic: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Foe {
    pub display: u32,
    pub name: String,
    pub boss: bool,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub radius: f32,
    pub xp: u32,
    pub effects: [f32; 9],
    pub dots: Vec<[f32; 3]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Pickup {
    pub kind: u8,
    pub value: f32,
    pub pos: [f32; 3],
}

fn nonnegative(v: f32) -> bool {
    v.is_finite() && v >= 0.0
}
pub(super) fn valid_character(c: &Character) -> bool {
    !c.name.trim().is_empty()
        && c.name.chars().count() <= 24
        && !c.name.chars().any(char::is_control)
        && c.class < super::data::CLASSES.len()
        && c.map < super::data::MAPS.len()
        && [1, 2, 3, 4, 5, 6, 7, 8].contains(&c.race)
        && c.sex <= 1
}
pub(super) fn decode(text: &str) -> Result<Save, String> {
    let save: Save = toml::from_str(text).map_err(|e| e.to_string())?;
    save.talents
        .validate_basic()
        .map_err(|e| format!("Progression des talents invalide : {e:?}"))?;
    if save.version != 1
        || !valid_character(&save.character)
        || save.potions > 3
        || save
            .gear
            .iter()
            .any(|x| *x > super::economy_rules::MAX_TIER)
    {
        return Err("Personnage ou version de sauvegarde invalide".into());
    }
    if let Some(r) = &save.run {
        if !valid_character(&r.character)
            || !nonnegative(r.elapsed)
            || r.elapsed > 86400.0
            || !nonnegative(r.hp)
            || !nonnegative(r.xp)
            || !(1..=10000).contains(&r.level)
            || r.pos.iter().any(|v| !v.is_finite())
            || !r.yaw.is_finite()
            || !r.damage_done.is_finite()
            || r.abilities.len() > super::data::MAX_ABILITIES
            || r.passives.len() > super::data::MAX_PASSIVES
            || r.stock > 3
            || !nonnegative(r.next_chest)
            || !nonnegative(r.potion_cd)
            || r.effects
                .iter()
                .chain(r.mobile.iter())
                .any(|v| !nonnegative(*v))
            || r.enemies.len() > 5000
            || r.pickups.len() > 20000
            || r.chests.len() > 1000
        {
            return Err("Partie sauvegardée invalide".into());
        }
        let class = &super::data::CLASSES[r.character.class];
        for (i, s) in r.abilities.iter().enumerate() {
            if !class.pool.contains(&s.key.as_str())
                || s.rank == 0
                || s.rank > super::data::MAX_ABILITY_LEVEL
                || !nonnegative(s.cd)
                || r.abilities[..i].iter().any(|p| p.key == s.key)
            {
                return Err("Sort incompatible avec la classe".into());
            }
        }
        for (i, s) in r.passives.iter().enumerate() {
            if !super::data::passives_for(class.id).any(|p| p.key == s.key)
                || s.rank == 0
                || s.rank > super::data::MAX_PASSIVE_LEVEL
                || r.passives[..i].iter().any(|p| p.key == s.key)
            {
                return Err("Passif incompatible avec la classe".into());
            }
        }
        let roster = super::data::roster(r.character.map);
        for e in &r.enemies {
            if !roster
                .enemies
                .iter()
                .chain(roster.bosses.iter())
                .any(|d| d.display == e.display && d.name == e.name)
                || e.pos.iter().any(|v| !v.is_finite())
                || !e.yaw.is_finite()
                || !nonnegative(e.hp)
                || !nonnegative(e.max_hp)
                || e.hp > e.max_hp
                || !nonnegative(e.speed)
                || !nonnegative(e.damage)
                || !nonnegative(e.radius)
                || e.effects.iter().any(|v| !nonnegative(*v))
                || e.dots.iter().flatten().any(|v| !nonnegative(*v))
            {
                return Err("Ennemi sauvegardé invalide".into());
            }
        }
        for p in &r.pickups {
            if p.kind > 2 || !nonnegative(p.value) || p.pos.iter().any(|v| !v.is_finite()) {
                return Err("Butin sauvegardé invalide".into());
            }
        }
        if r.chests.iter().flatten().any(|v| !v.is_finite()) {
            return Err("Coffre sauvegardé invalide".into());
        }
    }
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Save {
        Save {
            version: 1,
            character: Character {
                name: "Élodie".into(),
                class: 0,
                race: 1,
                sex: 1,
                map: 0,
                appearance: [1, 2, 3, 4, 5],
            },
            gold: 42,
            potions: 2,
            gear: [1, 2, 0, 0],
            talents: super::super::talent_rules::TalentProgress::default(),
            run: None,
        }
    }
    #[test]
    fn roundtrip_keeps_identity_wallet_and_gear() {
        let save = fixture();
        let decoded = decode(&toml::to_string(&save).unwrap()).unwrap();
        assert_eq!(decoded.character.name, "Élodie");
        assert_eq!(decoded.character.appearance, [1, 2, 3, 4, 5]);
        assert_eq!(
            (decoded.gold, decoded.potions, decoded.gear),
            (42, 2, [1, 2, 0, 0])
        );
    }
    #[test]
    fn corrupt_and_future_saves_are_rejected() {
        assert!(decode("truncated=[").is_err());
        let mut s = fixture();
        s.version = 9;
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
        s.version = 1;
        s.character.map = super::super::data::MAPS.len();
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
    }
    #[test]
    fn legacy_version_one_save_without_talents_still_preserves_the_character() {
        let mut value = toml::Value::try_from(fixture()).unwrap();
        value.as_table_mut().unwrap().remove("talents");
        let saved = decode(&toml::to_string(&value).unwrap()).unwrap();
        assert_eq!(saved.character.name, "Élodie");
        assert_eq!(
            (saved.gold, saved.potions, saved.gear),
            (42, 2, [1, 2, 0, 0])
        );
        assert!(saved.talents.profiles.is_empty());
    }
    #[test]
    fn talent_progress_roundtrips_but_overspent_save_is_rejected() {
        use super::super::talent_rules::{TalentDef, TalentProgress};
        let mut s = fixture();
        let mut talents = TalentProgress::default();
        talents.observe_level(1, 14).unwrap();
        let def = TalentDef {
            id: 10,
            class: 1,
            tree: 1,
            tier: 0,
            max_rank: 5,
            prerequisite: None,
            required_spell: None,
            supported: true,
        };
        talents.allocate(1, 10, &[def], &[]).unwrap();
        s.talents = talents.clone();
        let saved = decode(&toml::to_string(&s).unwrap()).unwrap();
        assert_eq!(saved.talents, talents);
        s.talents.profiles[0].highest_level = 9;
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
    }

    fn running() -> Save {
        let mut save = fixture();
        let def = &super::super::data::roster(0).bosses[0];
        let boss = Foe {
            display: def.display,
            name: def.name.into(),
            boss: true,
            pos: [1.0, 2.0, 3.0],
            yaw: 0.4,
            hp: 50.0,
            max_hp: 100.0,
            speed: 3.0,
            damage: 8.0,
            radius: 1.0,
            xp: 20,
            effects: [0.0; 9],
            dots: vec![[1.0, 3.0, 0.5]],
        };
        save.run = Some(Session {
            character: save.character.clone(),
            elapsed: 361.0,
            hp: 100.0,
            xp: 4.0,
            level: 3,
            kills: 27,
            bosses: 2,
            rings: 2,
            pending_levels: 1,
            damage_done: 1234.0,
            pos: [4.0, 5.0, 6.0],
            yaw: 0.2,
            abilities: vec![Spell {
                key: "cleave".into(),
                rank: 2,
                cd: 1.5,
                automatic: false,
            }],
            passives: vec![],
            effects: [0.0; 10],
            thorns_spell: 0,
            mobile: [4.0, 1.0, 0.0, 0.0],
            stock: 1,
            opened: 3,
            next_chest: 380.0,
            potion_cd: 12.0,
            enemies: vec![boss.clone(), boss],
            pickups: vec![Pickup {
                kind: 2,
                value: 10.0,
                pos: [7.0, 8.0, 9.0],
            }],
            chests: vec![[10.0, 11.0, 12.0]],
        });
        save
    }
    #[test]
    fn ongoing_run_keeps_two_live_bosses_loot_and_shared_cooldowns() {
        let save = decode(&toml::to_string(&running()).unwrap()).unwrap();
        let r = save.run.unwrap();
        assert_eq!((r.elapsed, r.bosses, r.enemies.len()), (361.0, 2, 2));
        assert_eq!(r.enemies[0].hp, 50.0);
        assert_eq!(r.abilities[0].cd, 1.5);
        assert!(!r.abilities[0].automatic);
        assert_eq!((r.stock, r.opened, r.potion_cd), (1, 3, 12.0));
        assert_eq!(r.pickups[0].value, 10.0);
        assert_eq!(r.chests.len(), 1);
    }
    #[test]
    fn wrong_class_spells_duplicates_and_nonfinite_positions_are_rejected() {
        let mut s = running();
        s.run.as_mut().unwrap().abilities[0].key = "shadow_bolt".into();
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
        s = running();
        let spell = s.run.as_ref().unwrap().abilities[0].clone();
        s.run.as_mut().unwrap().abilities.push(spell);
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
        s = running();
        s.run.as_mut().unwrap().pos[0] = f32::NAN;
        assert!(decode(&toml::to_string(&s).unwrap()).is_err());
    }
}
