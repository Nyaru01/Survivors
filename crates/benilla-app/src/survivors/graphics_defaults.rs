//! Fresh-profile graphics defaults for the Survivors entry point.
//!
//! Keep these out of the upstream CVar registry: they are product defaults for a new local
//! profile, not claims about the 1.12 reference client. Existing `benilla-config/config.toml`
//! files are left untouched.

use bevy::prelude::*;

const FRESH_PROFILE_GRAPHICS: &[(&str, &str)] = &[
    ("farclip", "777"),
    ("WorldDetail", "2"),
    ("frillDensity", "48"),
    ("SmallCull", "0.01"),
    ("anisotropic", "4"),
    // `TerrainMip` is exposed as `1 - shadowLevel` in the stock options panel.
    ("shadowLevel", "0"),
];

pub(super) struct GraphicsDefaultsPlugin;

impl Plugin for GraphicsDefaultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            apply_fresh_profile_defaults.after(crate::cvars::CvarLoad),
        );
    }
}

fn config_is_absent() -> bool {
    let Some(path) = crate::local_state::config_path() else {
        // Captures and test harnesses intentionally have no persistent profile.
        return false;
    };
    matches!(
        crate::local_state::read_to_string(&path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}

fn apply_fresh_profile_defaults(world: &mut World) {
    if !config_is_absent() {
        return;
    }

    let events = {
        let mut cvars = world.resource_mut::<crate::cvars::Cvars>();
        for &(name, value) in FRESH_PROFILE_GRAPHICS {
            // Environment-controlled values continue to own the session.
            if !cvars.is_session_owned(name) {
                cvars.set(name, value);
            }
        }
        cvars.take_events()
    };

    // Apply each normal CVar observer now, before menu/world setup reads the knobs.
    for event in events {
        world.trigger(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_profile_defaults_apply_as_regular_cvar_changes() {
        let mut world = World::new();
        world.insert_resource(crate::cvars::Cvars::default());
        let events = {
            let mut cvars = world.resource_mut::<crate::cvars::Cvars>();
            for &(name, value) in FRESH_PROFILE_GRAPHICS {
                cvars.set(name, value);
            }
            cvars.take_events()
        };
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("farclip"),
            Some("777")
        );
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("WorldDetail"),
            Some("2")
        );
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("frillDensity"),
            Some("48")
        );
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("SmallCull"),
            Some("0.01")
        );
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("anisotropic"),
            Some("4")
        );
        assert_eq!(
            world.resource::<crate::cvars::Cvars>().get("shadowLevel"),
            Some("0")
        );
        assert_eq!(events.len(), FRESH_PROFILE_GRAPHICS.len());
    }
}
