//! Survivors preferences, separate from the real WoW installation's configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Preferences {
    pub azerty: bool,
    pub sensitivity: f32,
    pub invert_y: bool,
    pub farclip: f32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub resolution: usize,
    pub polish: bool,
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub ambience: f32,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            azerty: false,
            sensitivity: 1.0,
            invert_y: false,
            farclip: 350.0,
            fullscreen: false,
            vsync: true,
            resolution: 0,
            polish: true,
            master: 1.0,
            music: 0.4,
            effects: 1.0,
            ambience: 0.6,
        }
    }
}
impl Preferences {
    pub fn encode(self) -> String {
        format!(
            "SURVIVORS_OPTIONS_1\n{} {} {} {} {} {} {} {} {} {} {} {}\n",
            self.azerty as u8,
            self.sensitivity,
            self.invert_y as u8,
            self.farclip,
            self.fullscreen as u8,
            self.vsync as u8,
            self.resolution,
            self.polish as u8,
            self.master,
            self.music,
            self.effects,
            self.ambience
        )
    }
    pub fn decode(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != "SURVIVORS_OPTIONS_1" {
            return None;
        }
        let v: Vec<f32> = lines
            .next()?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        if v.len() != 12 || v.iter().any(|v| !v.is_finite()) {
            return None;
        }
        for i in [0, 2, 4, 5, 7] {
            if v[i] != 0.0 && v[i] != 1.0 {
                return None;
            }
        }
        if !(0.25..=3.0).contains(&v[1])
            || !(177.0..=777.0).contains(&v[3])
            || !(0.0..=2.0).contains(&v[6])
            || v[6].fract() != 0.0
        {
            return None;
        }
        if v[8..].iter().any(|v| !(0.0..=1.0).contains(v)) {
            return None;
        }
        Some(Self {
            azerty: v[0] != 0.0,
            sensitivity: v[1],
            invert_y: v[2] != 0.0,
            farclip: v[3],
            fullscreen: v[4] != 0.0,
            vsync: v[5] != 0.0,
            resolution: v[6] as usize,
            polish: v[7] != 0.0,
            master: v[8],
            music: v[9],
            effects: v[10],
            ambience: v[11],
        })
    }
}
