//! Prefetches localized talent rank descriptions for the serverless Blizzard window.
//! Values come from the client tables, not from invented effects on Survivors combat.
use benilla_ui::script::{SpellTooltipView, UiScript};

/// Call before `set_talents` / opening the window and whenever the class or VM changes.
/// All ranks are present on the very first hover, including the next-rank paragraph.
pub(crate) fn prefetch(
    script: &mut UiScript,
    spells: &crate::ui_action::Spells,
    talents: &benilla_formats::TalentCatalog,
    race: u8,
    class: u8,
) -> usize {
    let mut ids = std::collections::BTreeSet::new();
    for tab in talents.tabs_for_class(race, class) {
        for talent in talents.talents_in_tab(tab.id) {
            ids.extend(talent.ranks.into_iter().filter(|id| *id != 0));
        }
    }
    let built: Vec<_> = {
        let get = |key: &str| benilla_ui::strings::global(script.lua(), key);
        let lookup = |id| spells.catalog.get(id);
        let context = benilla_formats::TokenContext {
            durations: &spells.durations,
            radii: &spells.radii,
            ranges: Some(&spells.ranges),
            // No player profession/skill-line state exists in the local adventure.
            skill: &|_| 0,
            lookup: &lookup,
            mods: None,
            unmodified_points: false,
            home_area: &|| None,
            global: &get,
            printf: &crate::ui_script::token_printf,
        };
        ids.into_iter()
            .filter_map(|id| {
                let spell = spells.catalog.get(id)?;
                let description = spell
                    .description
                    .as_deref()
                    .map(|text| benilla_formats::substitute(text, spell, &context))
                    .unwrap_or_default();
                let aura_description = spell
                    .aura_description
                    .as_deref()
                    .map(|text| benilla_formats::substitute(text, spell, &context))
                    .unwrap_or_default();
                Some((
                    id,
                    SpellTooltipView {
                        name: spell.name.clone(),
                        rank: spell.rank.clone(),
                        description,
                        aura_description,
                        item_met: true,
                        form_met: true,
                        ..Default::default()
                    },
                ))
            })
            .collect()
    };
    let count = built.len();
    for (id, view) in built {
        script.set_spell_tooltip(id, view);
    }
    count
}
