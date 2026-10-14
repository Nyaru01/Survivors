//! Shared cooldown gating for automatic and deliberately requested spell casts.

pub(super) const SLOTS: usize = 6;
pub(super) struct CastControl {
    automatic: [bool; SLOTS],
    requested: [bool; SLOTS],
}

impl Default for CastControl {
    fn default() -> Self {
        Self {
            automatic: [true; SLOTS],
            requested: [false; SLOTS],
        }
    }
}

impl CastControl {
    pub(super) fn request(&mut self, slot: usize) {
        if let Some(request) = self.requested.get_mut(slot) {
            *request = true;
        }
    }
    pub(super) fn toggle(&mut self, slot: usize) {
        if let Some(automatic) = self.automatic.get_mut(slot) {
            *automatic = !*automatic;
        }
    }
    pub(super) fn automatic(&self, slot: usize) -> bool {
        self.automatic.get(slot).copied().unwrap_or(false)
    }
    pub(super) fn should_cast(&mut self, slot: usize, ready: bool) -> bool {
        let Some(request) = self.requested.get_mut(slot) else {
            return false;
        };
        let manual = std::mem::take(request);
        ready && (self.automatic[slot] || manual)
    }
}
