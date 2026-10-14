//! Blizzard's MainMenuBar artwork, using the original FrameXML atlas coordinates.
use super::hud::UiArt;
use bevy::prelude::*;

pub(super) fn spawn(parent: &mut ChildSpawnerCommands) {
    for (i, top) in [0.83203125, 0.58203125, 0.33203125, 0.08203125]
        .into_iter()
        .enumerate()
    {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(i as f32 * 256.0),
                bottom: Val::Px(0.0),
                width: Val::Px(256.0),
                height: Val::Px(43.0),
                ..default()
            },
            ImageNode::default(),
            UiArt::crop(
                "Interface\\MainMenuBar\\UI-MainMenuBar-Dwarf",
                [0.0, 1.0, top, top + 43.0 / 256.0],
            ),
        ));
    }
    for (left, flip) in [(-96.0, false), (992.0, true)] {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                bottom: Val::Px(0.0),
                width: Val::Px(128.0),
                height: Val::Px(128.0),
                ..default()
            },
            ImageNode {
                flip_x: flip,
                ..default()
            },
            UiArt::crop(
                "Interface\\MainMenuBar\\UI-MainMenuBar-EndCap-Dwarf",
                [0.0, 1.0, 0.0, 1.0],
            ),
        ));
    }
}
