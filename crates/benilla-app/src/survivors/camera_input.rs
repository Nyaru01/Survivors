//! Camera gestures, independent of rendering so locked-view input can be regression-tested.

pub(super) const FIXED_PITCH: f32 = 61.0 * std::f32::consts::PI / 180.0;

pub(super) struct OrbitState {
    pub unlocked: bool,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Default)]
pub(super) struct OrbitInput {
    pub playing: bool,
    pub toggle: bool,
    pub right: bool,
    pub left: bool,
    pub dx: f32,
    pub dy: f32,
}

pub(super) fn update(mut state: OrbitState, input: OrbitInput) -> OrbitState {
    if !input.playing {
        return state;
    }
    if input.toggle {
        state.unlocked = !state.unlocked;
        if !state.unlocked {
            state.yaw = 0.0;
            state.pitch = FIXED_PITCH;
        }
    }
    if (input.right || (state.unlocked && input.left)) && (input.dx != 0.0 || input.dy != 0.0) {
        state.unlocked = true;
        state.yaw -= input.dx * 0.008;
        state.pitch = (state.pitch + input.dy * 0.008).clamp(0.18, 1.45);
    }
    state
}
