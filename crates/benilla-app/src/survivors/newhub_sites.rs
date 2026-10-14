//! Source-backed Vanilla 1.12.1 locations for the Survivors preparation hubs.
//!
//! Coordinates here are raw WoW world coordinates (x, y, z), not map-image coordinates. Convert
//! them with `benilla_assets::coords::wow_to_bevy` at the call site. The SQL row keys below refer
//! to the VMaNGOS `world_full_14_june_2021.sql` snapshot from `brotalnia/database`.

/// Which playable faction owns the preparation site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HubFaction {
    Alliance,
    Horde,
}
pub(crate) fn for_race(race: u8) -> &'static FactionHub {
    if matches!(race, 1 | 3 | 4 | 7) {
        &ALLIANCE
    } else {
        &HORDE
    }
}

/// A source-backed world position, in the coordinate order used by the WoW SQL tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WowLocation {
    pub map: u32,
    pub xyz: [f32; 3],
    pub orientation: f32,
}

/// One verified Vanilla creature model placed at a source-backed or collision-sampled location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ServicePlacement {
    /// CreatureDisplayInfo.dbc ID (not CreatureModelData.dbc ID).
    pub display_id: u32,
    pub location: WowLocation,
}

/// A verified Vanilla M2 portal asset and its collision-sampled placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PortalPlacement {
    pub model_path: &'static str,
    pub texture_paths: &'static [&'static str],
    pub anchor: WowLocation,
}

/// One authored faction hub. `arrival` is an existing teleport destination, kept distinct from
/// the SQL spawn placements used to anchor the service NPCs and portal visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FactionHub {
    pub faction: HubFaction,
    pub name: &'static str,
    pub zone: &'static str,
    pub map: u32,
    /// A collision-sampled site reference point.
    pub center: WowLocation,
    pub arrival: WowLocation,
    pub merchant: ServicePlacement,
    pub artisan: ServicePlacement,
    pub portal: PortalPlacement,
}

/// Compact Vanilla portal: gameobject_template entry 176296, display 4396, size 1.
/// Local MPQ M2: one additive emissive batch, looping sequence 0 (3334 ms), four emitters.
pub(crate) const VANILLA_PORTAL_TEXTURES: &[&str] = &[
    "World\\Generic\\ActiveDoodads\\MagePortals\\Stormwind_Portal.blp",
    "World\\Generic\\ActiveDoodads\\InstancePortal\\Dust6.blp",
    "Creature\\FireElemental\\GlowBall.blp",
    "Item\\ObjectComponents\\Weapon\\Flare.blp",
];
pub(crate) const VANILLA_PORTAL_MODEL: &str =
    "World\\Generic\\ActiveDoodads\\SpellPortals\\MagePortal_Stormwind.m2";
/// Upper Wizard Sanctum: exact areatrigger_teleport Exit destination (map 0).
/// Service/portal anchors are actual non-DETAIL collision-face samples on Stormwind WMO
/// group 284, WMOAreaTable WMO 61/group 2606: French "Sanctuaire du sorcier".
/// These are authored Survivors placements, not historical NPC spawn rows.
pub(crate) const ALLIANCE: FactionHub = FactionHub {
    faction: HubFaction::Alliance,
    name: "Sanctum des Sorciers",
    zone: "Hurlevent",
    map: 0,
    center: WowLocation {
        map: 0,
        xyz: [-9004.642, 870.018, 148.621],
        orientation: 1.752,
    },
    arrival: WowLocation {
        map: 0,
        xyz: [-9014.94, 873.326, 148.616],
        orientation: 5.49779,
    },
    merchant: ServicePlacement {
        display_id: 191,
        location: WowLocation {
            map: 0,
            xyz: [-9014.460, 877.541, 148.621],
            orientation: 4.72984,
        },
    },
    artisan: ServicePlacement {
        display_id: 243,
        location: WowLocation {
            map: 0,
            xyz: [-9019.155, 873.806, 148.621],
            orientation: 0.802851,
        },
    },
    portal: PortalPlacement {
        model_path: VANILLA_PORTAL_MODEL,
        texture_paths: VANILLA_PORTAL_TEXTURES,
        anchor: WowLocation {
            map: 0,
            xyz: [-9004.642, 870.018, 148.621],
            orientation: 1.752,
        },
    },
};

/// Cleft of Shadow: arrival uses Neeru (entry 3216, creature GUID 3415) SQL anchor.
/// Three fixtures use non-DETAIL walk-collision samples of Orgrimmar WMO group 138,
/// within ten yards on its gently uneven floor. These are authored Survivors placements.
pub(crate) const HORDE: FactionHub = FactionHub {
    faction: HubFaction::Horde,
    name: "Faille de l’Ombre",
    zone: "Orgrimmar",
    map: 1,
    center: WowLocation {
        map: 1,
        xyz: [1803.147, -4372.832, -17.468],
        orientation: 4.57276,
    },
    arrival: WowLocation {
        map: 1,
        xyz: [1800.660, -4374.510, -17.3475],
        orientation: 5.20165,
    },
    merchant: ServicePlacement {
        display_id: 191,
        location: WowLocation {
            map: 1,
            xyz: [1803.147, -4372.832, -17.468],
            orientation: 4.57276,
        },
    },
    artisan: ServicePlacement {
        display_id: 243,
        location: WowLocation {
            map: 1,
            xyz: [1794.008, -4375.378, -17.188],
            orientation: 0.855211,
        },
    },
    portal: PortalPlacement {
        model_path: VANILLA_PORTAL_MODEL,
        texture_paths: VANILLA_PORTAL_TEXTURES,
        anchor: WowLocation {
            map: 1,
            xyz: [1794.876, -4382.030, -17.7444],
            orientation: 3.59538,
        },
    },
};
