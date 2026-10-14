//! Local authored hub overrides, separate from character progression and Vanilla data.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Pose {
    pub xyz: [f32; 3],
    pub yaw: f32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Portal {
    pub name: String,
    pub display: u32,
    pub destination: Option<usize>,
    pub pose: Option<Pose>,
    pub scale: f32,
}
impl Default for Portal {
    fn default() -> Self {
        Self {
            name: "Portail".into(),
            display: 4396,
            destination: None,
            pose: None,
            scale: 1.0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Hub {
    pub npcs: [NpcPlacement; 2],
    pub displays: [u32; 2],
    pub portals: Vec<Portal>,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct NpcPlacement {
    pub pose: Option<Pose>,
}
impl Default for Hub {
    fn default() -> Self {
        Self {
            npcs: [NpcPlacement::default(), NpcPlacement::default()],
            displays: [191, 243],
            portals: vec![Portal::default()],
        }
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Document {
    pub version: u32,
    pub hubs: [Hub; 2],
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 2,
            hubs: [Hub::default(), Hub::default()],
        }
    }
}
impl Document {
    pub fn decode(text: &str) -> Option<Self> {
        if text.starts_with("survivors-hub-layout-v1") {
            let legacy = Layout::decode(text)?;
            let mut result = Self::default();
            for i in 0..2 {
                result.hubs[i].npcs = [
                    NpcPlacement {
                        pose: legacy.0[i][0],
                    },
                    NpcPlacement {
                        pose: legacy.0[i][1],
                    },
                ];
                result.hubs[i].portals[0].pose = legacy.0[i][2];
            }
            return Some(result);
        }
        let doc: Self = toml::from_str(text).ok()?;
        if doc.version != 2 {
            return None;
        }
        for hub in &doc.hubs {
            if hub.displays.contains(&0) || hub.portals.is_empty() || hub.portals.len() > 64 {
                return None;
            }
            for p in &hub.portals {
                if p.display == 0
                    || p.name.trim().is_empty()
                    || p.name.chars().count() > 48
                    || p.name.chars().any(char::is_control)
                    || !p.scale.is_finite()
                    || !(0.05..=3.0).contains(&p.scale)
                    || p.destination.is_some_and(|m| m >= 16)
                {
                    return None;
                }
            }
            if hub
                .npcs
                .iter()
                .map(|n| &n.pose)
                .chain(hub.portals.iter().map(|p| &p.pose))
                .flatten()
                .any(|p| !p.yaw.is_finite() || p.xyz.iter().any(|x| !x.is_finite()))
            {
                return None;
            }
        }
        Some(doc)
    }
    pub fn encode(&self) -> String {
        toml::to_string_pretty(self).expect("valid hub settings")
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct Layout(pub [[Option<Pose>; 3]; 2]);

impl Layout {
    pub fn encode(&self) -> String {
        let mut text = String::from("survivors-hub-layout-v1\n");
        for (faction, poses) in self.0.iter().enumerate() {
            for (actor, pose) in poses.iter().enumerate() {
                if let Some(p) = pose {
                    text.push_str(&format!(
                        "{faction} {actor} {} {} {} {}\n",
                        p.xyz[0], p.xyz[1], p.xyz[2], p.yaw
                    ));
                }
            }
        }
        text
    }

    pub fn decode(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != "survivors-hub-layout-v1" {
            return None;
        }
        let mut result = Self::default();
        for line in lines.filter(|line| !line.trim().is_empty()) {
            let words: Vec<_> = line.split_whitespace().collect();
            if words.len() != 6 {
                return None;
            }
            let faction: usize = words[0].parse().ok()?;
            let actor: usize = words[1].parse().ok()?;
            if faction > 1 || actor > 2 || result.0[faction][actor].is_some() {
                return None;
            }
            let mut values = [0.0f32; 4];
            for (value, word) in values.iter_mut().zip(&words[2..]) {
                *value = word.parse().ok()?;
                if !value.is_finite() {
                    return None;
                }
            }
            result.0[faction][actor] = Some(Pose {
                xyz: [values[0], values[1], values[2]],
                yaw: values[3],
            });
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrate_placements_and_keep_named_destinations_separate() {
        let mut old = Layout::default();
        old.0[1][2] = Some(Pose {
            xyz: [1.0, 2.0, 3.0],
            yaw: 0.2,
        });
        let mut doc = Document::decode(&old.encode()).unwrap();
        assert_eq!(doc.hubs[1].portals[0].pose, old.0[1][2]);
        doc.hubs[0].portals.push(Portal {
            name: "Forêt d’Elwynn".into(),
            destination: Some(5),
            ..Portal::default()
        });
        assert_eq!(Document::decode(&doc.encode()), Some(doc));
    }
    #[test]
    fn reject_invalid_destination_and_size() {
        let mut doc = Document::default();
        doc.hubs[0].portals[0].destination = Some(16);
        assert!(Document::decode(&doc.encode()).is_none());
        doc.hubs[0].portals[0].destination = None;
        doc.hubs[0].portals[0].scale = 0.0;
        assert!(Document::decode(&doc.encode()).is_none());
    }
    #[test]
    fn factions_and_unmodified_actors_survive_roundtrip() {
        let mut layout = Layout::default();
        layout.0[0][1] = Some(Pose {
            xyz: [-9014.94, 873.326, 148.616],
            yaw: 1.2,
        });
        layout.0[1][2] = Some(Pose {
            xyz: [1800.66, -4374.51, -17.3475],
            yaw: -0.2,
        });
        assert_eq!(Layout::decode(&layout.encode()), Some(layout));
    }
    #[test]
    fn reject_corrupt_nonfinite_and_duplicate_rows() {
        for row in [
            "2 0 1 2 3 4",
            "0 3 1 2 3 4",
            "0 0 NaN 2 3 4",
            "0 0 1 2 3 inf",
            "0 0 1 2 3",
            "0 0 1 2 3 4\n0 0 1 2 3 4",
        ] {
            assert!(Layout::decode(&format!("survivors-hub-layout-v1\n{row}\n")).is_none());
        }
    }
}
