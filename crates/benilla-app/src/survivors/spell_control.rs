//! Click and keyboard requests, retaining auto-cast by default like a WoW auto-cast button.

use super::{hud::TipFor, spell_input::CastControl, Run};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct SpellControl(pub(super) CastControl);

pub(super) fn reset(mut control: ResMut<SpellControl>) {
    control.0 = CastControl::default();
}

pub(super) fn collect(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    run: Res<Run>,
    slots: Query<(&Interaction, &TipFor)>,
    mut control: ResMut<SpellControl>,
) {
    for (slot, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ]
    .into_iter()
    .enumerate()
    {
        if slot < run.abilities.len() && keys.just_pressed(key) {
            control.0.request(slot);
        }
    }
    for (interaction, tip) in &slots {
        let TipFor::Ability(slot) = *tip else {
            continue;
        };
        if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            control.0.request(slot);
        } else if *interaction == Interaction::Hovered && mouse.just_pressed(MouseButton::Right) {
            control.0.toggle(slot);
        }
    }
}
