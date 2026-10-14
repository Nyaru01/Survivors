//! Local authored hub overrides, separate from character progression and Vanilla data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pose {
    pub xyz: [f32; 3],
    pub yaw: f32,
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
