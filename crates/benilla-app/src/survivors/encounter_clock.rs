pub(super) const INTERVAL: f32 = 180.0;
pub(super) fn due(elapsed: f32) -> u32 {
    (elapsed.max(0.0) / INTERVAL).floor() as u32
}
pub(super) fn remaining(elapsed: f32) -> u32 {
    (((due(elapsed) + 1) as f32 * INTERVAL) - elapsed)
        .ceil()
        .clamp(0.0, 180.0) as u32
}
