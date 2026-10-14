//! Experience thresholds for Warcraft Survivors, independent of Bevy and roster data.

/// Experience needed to advance from `level` to the next level.
///
/// At the current twofold gem rate, a normal starter pace reaches level two in about half a
/// minute and level five in roughly four minutes; the same steady increase continues late-run.
pub(crate) fn xp_to_next(level: u32) -> f32 {
    55.0 + level.saturating_sub(1) as f32 * 35.0
}
