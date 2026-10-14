//! Keep bottom-bar descriptions inside the visible canvas, above their action slots.
pub(super) fn tooltip_position(icon: [f32; 4], tooltip: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let width = tooltip[0].max(120.0).min(screen[0] - 8.0);
    let height = tooltip[1].max(32.0).min(screen[1] - 8.0);
    let left = (icon[0] - icon[2] / 2.0).clamp(4.0, (screen[0] - width - 4.0).max(4.0));
    let top =
        (icon[1] - icon[3] / 2.0 - height - 8.0).clamp(4.0, (screen[1] - height - 4.0).max(4.0));
    [left, top]
}
