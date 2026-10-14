//! Persistent race-specific world routes. Map indices remain the existing stable MAPS indices.
use serde::{Deserialize, Serialize};

pub const MAP_COUNT: usize = 16;
pub const BOSSES_REQUIRED: u32 = 4;
// Each race begins at home, then its appropriate early world. Other peoples' starter zones
// remain visible but are not destinations in this route; legacy access is preserved separately.
const COMMON_ROUTE: [usize; 10] = [0, 11, 13, 12, 4, 3, 1, 14, 15, 2];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldProgress {
    #[serde(default)]
    pub profiles: Vec<RaceProgress>,
    /// Missing old Save.worlds is distinguishable from an empty new character's progression.
    #[serde(skip)]
    pub legacy_missing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaceProgress {
    pub race: u8,
    #[serde(default)]
    pub completed: Vec<usize>,
    #[serde(default)]
    pub legacy_access: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldError {
    InvalidRace,
    InvalidMap,
    DuplicateProfile,
    DuplicateMap,
    Locked,
    FourBossesRequired,
    HeroDead,
}

pub fn home(race: u8) -> Option<usize> {
    match race {
        1 => Some(5),
        2 | 8 => Some(6),
        3 | 7 => Some(8),
        4 => Some(7),
        5 => Some(10),
        6 => Some(9),
        _ => None,
    }
}

pub fn route(race: u8) -> Vec<usize> {
    let Some(home) = home(race) else {
        return Vec::new();
    };
    let next = if matches!(race, 1 | 3 | 7) { 11 } else { 0 };
    [home, next]
        .into_iter()
        .chain(
            COMMON_ROUTE
                .into_iter()
                .filter(|&map| map != home && map != next),
        )
        .collect()
}

pub fn due_bosses(elapsed: f32) -> u32 {
    if !elapsed.is_finite() {
        return 0;
    }
    ((elapsed.max(0.0) / 180.0).floor() as u32).min(BOSSES_REQUIRED)
}

pub fn victory(boss_kills: u32, alive: bool) -> bool {
    alive && boss_kills >= BOSSES_REQUIRED
}

impl WorldProgress {
    pub fn legacy() -> Self {
        Self {
            legacy_missing: true,
            ..Self::default()
        }
    }
    pub fn profile(&self, race: u8) -> Option<&RaceProgress> {
        self.profiles.iter().find(|p| p.race == race)
    }
    pub fn world_number(&self, race: u8, map: usize) -> Option<u32> {
        route(race)
            .iter()
            .position(|&v| v == map)
            .map(|i| i as u32 + 1)
    }
    pub fn next_world(&self, race: u8, map: usize) -> Option<usize> {
        let route = route(race);
        route
            .iter()
            .position(|&v| v == map)
            .and_then(|i| route.get(i + 1).copied())
    }
    pub fn completed(&self, race: u8, map: usize) -> bool {
        self.profile(race)
            .is_some_and(|p| p.completed.contains(&map))
    }
    pub fn available(&self, race: u8, map: usize) -> bool {
        if home(race).is_none() || map >= MAP_COUNT {
            return false;
        }
        if self
            .profile(race)
            .is_some_and(|p| p.legacy_access.contains(&map))
        {
            return true;
        }
        let route = route(race);
        let Some(index) = route.iter().position(|&m| m == map) else {
            return false;
        };
        index == 0
            || self.profile(race).is_some_and(|p| {
                p.completed.contains(&map)
                    || p.legacy_access.contains(&map)
                    || p.completed.contains(&route[index - 1])
            })
    }
    pub fn first_available(&self, race: u8) -> Option<usize> {
        home(race)
    }
    fn ensure_profile(&mut self, race: u8) -> Result<&mut RaceProgress, WorldError> {
        home(race).ok_or(WorldError::InvalidRace)?;
        if let Some(i) = self.profiles.iter().position(|p| p.race == race) {
            return Ok(&mut self.profiles[i]);
        }
        self.profiles.push(RaceProgress {
            race,
            completed: Vec::new(),
            legacy_access: Vec::new(),
        });
        Ok(self
            .profiles
            .last_mut()
            .expect("just inserted race profile"))
    }
    pub fn preserve_legacy(&mut self, race: u8, map: usize) -> Result<(), WorldError> {
        if map >= MAP_COUNT {
            return Err(WorldError::InvalidMap);
        }
        let p = self.ensure_profile(race)?;
        if !p.legacy_access.contains(&map) {
            p.legacy_access.push(map);
        }
        Ok(())
    }
    /// The boss kill count comes from credited deaths, never elapsed time or spawned bosses.
    pub fn complete(
        &mut self,
        race: u8,
        map: usize,
        boss_kills: u32,
        alive: bool,
    ) -> Result<Option<usize>, WorldError> {
        self.validate_basic()?;
        if !alive {
            return Err(WorldError::HeroDead);
        }
        if boss_kills < BOSSES_REQUIRED {
            return Err(WorldError::FourBossesRequired);
        }
        if !self.available(race, map) {
            return Err(if home(race).is_none() {
                WorldError::InvalidRace
            } else if map >= MAP_COUNT {
                WorldError::InvalidMap
            } else {
                WorldError::Locked
            });
        }
        let p = self.ensure_profile(race)?;
        if !p.completed.contains(&map) {
            p.completed.push(map);
        }
        Ok(self.next_world(race, map))
    }
    pub fn validate_basic(&self) -> Result<(), WorldError> {
        for (i, p) in self.profiles.iter().enumerate() {
            if home(p.race).is_none() {
                return Err(WorldError::InvalidRace);
            }
            if self.profiles[..i].iter().any(|v| v.race == p.race) {
                return Err(WorldError::DuplicateProfile);
            }
            for list in [&p.completed, &p.legacy_access] {
                for (j, &map) in list.iter().enumerate() {
                    if map >= MAP_COUNT {
                        return Err(WorldError::InvalidMap);
                    }
                    if list[..j].contains(&map) {
                        return Err(WorldError::DuplicateMap);
                    }
                }
            }
        }
        Ok(())
    }
}
