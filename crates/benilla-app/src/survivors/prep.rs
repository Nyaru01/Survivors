//! Preparation before a new adventure, using the client's original panel/button templates.
use super::{data, hud, Phase, Run, Selection};
use benilla_ui::script::{SpellBookState, SpellSlotView, SpellTabView, SpellTooltipView, UiScript};
use bevy::prelude::*;

#[path = "camp.rs"]
pub(super) mod camp;

const PAGE: &str = r#"
if not SurvivorsPreparationFrame then
    local f=CreateFrame('Frame','SurvivorsPreparationFrame',UIParent)
    f:SetAllPoints(UIParent); f:SetFrameStrata('BACKGROUND')
    local title=f:CreateFontString('SurvivorsPreparationTitle','OVERLAY','GameFontNormalLarge')
    title:SetPoint('TOP',f,'TOP',0,-36)
    local info=f:CreateFontString('SurvivorsPreparationInfo','OVERLAY','GameFontHighlight')
    info:SetPoint('TOP',title,'BOTTOM',0,-12)
    local camp=f:CreateFontString('SurvivorsPreparationCamp','OVERLAY','GameFontNormal')
    camp:SetPoint('TOP',info,'BOTTOM',0,-34)
    camp:SetText('Camp de préparation')
    local merchant=CreateFrame('Button','SurvivorsPreparationMerchant',f,'UIPanelButtonTemplate')
    merchant:SetWidth(170); merchant:SetHeight(32); merchant:SetPoint('TOP',camp,'BOTTOM',-105,-12)
    merchant:SetText('Marchand')
    merchant:SetScript('OnClick',function() __survivors_merchant=true end)
    local artisan=CreateFrame('Button','SurvivorsPreparationArtisan',f,'UIPanelButtonTemplate')
    artisan:SetWidth(170); artisan:SetHeight(32); artisan:SetPoint('LEFT',merchant,'RIGHT',40,0)
    artisan:SetText('Artisan')
    local forge=CreateFrame('Frame','SurvivorsPreparationForge',f)
    forge:SetWidth(480); forge:SetHeight(174); forge:SetPoint('TOP',artisan,'BOTTOM',0,-10)
    forge:SetBackdrop({bgFile='Interface\\DialogFrame\\UI-DialogBox-Background',edgeFile='Interface\\DialogFrame\\UI-DialogBox-Border',tile=true,tileSize=32,edgeSize=32,insets={left=11,right=12,top=12,bottom=11}})
    forge:SetBackdropColor(0.08,0.06,0.03,0.96)
    forge:SetBackdropBorderColor(0.78,0.62,0.28,1)
    local forgeTitle=forge:CreateFontString('SurvivorsPreparationForgeTitle','OVERLAY','GameFontNormal')
    forgeTitle:SetPoint('TOP',forge,'TOP',0,-4); forgeTitle:SetText('Fusion de l’artisan')
    local forgeInfo=forge:CreateFontString('SurvivorsPreparationForgeInfo','OVERLAY','GameFontHighlightSmall')
    forgeInfo:SetWidth(460); forgeInfo:SetPoint('TOP',forgeTitle,'BOTTOM',0,-8)
    forgeInfo:SetJustifyH('CENTER')
    forgeInfo:SetText('Fusionnez 3 pièces non équipées, de même qualité et de même emplacement, pour obtenir la qualité suivante. La fusion est permanente.')
    local fusion=CreateFrame('Frame','SurvivorsPreparationFusionButtons',forge)
    fusion:SetPoint('TOP',forgeInfo,'BOTTOM',0,-10); fusion:SetWidth(460); fusion:SetHeight(30)
    local fusionLabels={'Arme','Armure','Anneau','Bottes'}
    for slot=0,3 do
        local selectedSlot=slot
        local b=CreateFrame('Button','SurvivorsPreparationFuse'..slot,fusion,'UIPanelButtonTemplate')
        b:SetWidth(108); b:SetHeight(28); b:SetPoint('LEFT',fusion,'LEFT',slot*116,0)
        b:SetText(fusionLabels[slot+1])
        b:SetScript('OnClick',function() __survivors_fuse_slot=selectedSlot end)
    end
    local forgeSummary=forge:CreateFontString('SurvivorsPreparationForgeSummary','OVERLAY','GameFontHighlight')
    forgeSummary:SetPoint('TOP',fusion,'BOTTOM',0,-9); forgeSummary:SetWidth(460)
    forgeSummary:SetJustifyH('CENTER')
    forge:Hide()
    artisan:SetScript('OnClick',function() if forge:IsShown() then forge:Hide() else forge:Show() end end)
    local function button(name,text,x,y,w,callback)
        local b=CreateFrame('Button',name,f,'UIPanelButtonTemplate')
        b:SetWidth(w); b:SetHeight(32); b:SetPoint('BOTTOM',f,'BOTTOM',x,y)
        b:SetText(text); b:SetScript('OnClick',callback)
        return b
    end
    button('SurvivorsPreparationTalents','Talents',-240,92,150,function() ToggleTalentFrame() end)
    button('SurvivorsPreparationEquipment','Équipement',-80,92,150,function() ToggleCharacter('PaperDollFrame') end)
    button('SurvivorsPreparationSpellBook','Grimoire',80,92,150,function() ToggleSpellBook(BOOKTYPE_SPELL) end)
    button('SurvivorsPreparationBags','Sacs',240,92,150,function() ToggleAllBags() end)
    button('SurvivorsPreparationEnter','Portail : entrer dans le monde',0,48,260,function() __survivors_prepare_enter=true end)
    button('SurvivorsPreparationBack','Retour',0,10,120,function() __survivors_prepare_back=true end)
    f:Hide()
end
-- The grimoire lists class progression spells; casts remain on the actual combat bar.
function SpellButton_OnClick() end
function SpellButton_OnDrag() end
"#;

#[derive(Resource, Default)]
struct Page {
    session: u64,
    book: Option<(u8, u8, super::talent_rules::TalentProgress)>,
}

pub(super) fn plugin(app: &mut App) {
    camp::plugin(app);
    app.init_resource::<Page>()
        .init_resource::<camp::ForgeState>()
        .add_systems(
            Update,
            page.after(crate::ui_script::UiInput)
                .after(hud::menu_preview),
        )
        .add_systems(Update, feed_book.in_set(crate::ui_script::UiFeed));
}

#[allow(clippy::too_many_arguments)]
fn page(
    script: Option<NonSendMut<UiScript>>,
    phase: Res<State<Phase>>,
    stock: Res<super::blizzard_ui::StockUi>,
    mut memo: ResMut<Page>,
    mut selection: ResMut<Selection>,
    progress: Res<super::character_save::TalentProgressStore>,
    mut forge: ResMut<camp::ForgeState>,
    mut run: ResMut<Run>,
    mut economy: ResMut<super::economy::Economy>,
    mut next: ResMut<NextState<Phase>>,
    world: Res<super::character_save::WorldProgressStore>,
    mut commands: Commands,
    mut heroes: Query<(&mut Transform, &mut super::hero::Hero)>,
    terrain: benilla_world::world_point::WorldPoint,
) {
    let Some(mut script) = script else {
        return;
    };
    if !stock.ready {
        return;
    }
    if memo.session != script.session() {
        if let Err(error) = script.run(PAGE) {
            warn!("Preparation Blizzard: {error}");
            return;
        }
        memo.session = script.session();
        memo.book = None;
    }
    let preparing = *phase.get() == Phase::Preparing;
    let _ = script.run(if preparing {
        "SurvivorsPreparationFrame:Show()"
    } else {
        "SurvivorsPreparationFrame:Hide()"
    });
    forge.0 = preparing
        && script
            .eval::<bool>(
                "return SurvivorsPreparationForge and SurvivorsPreparationForge:IsShown() or false",
            )
            .unwrap_or(false);
    if !preparing {
        return;
    }
    let _ = script.run(
        "SurvivorsPreparationForgeSummary:SetText(__survivors_fusion_summary or 'Inventaire et or : informations indisponibles')",
    );
    if let Some(class) = selection.class_def() {
        let _ = script.lua().globals().set(
            "__survivors_prepare_title",
            format!(
                "{} — {}",
                selection.name,
                super::localization::class_name(class.id)
            ),
        );
        let zone = super::localization::zone_name(selection.map_def().name)
            .unwrap_or(selection.map_def().name);
        let _ = script.lua().globals().set(
            "__survivors_prepare_info",
            format!(
                "{} · Talents disponibles : {} · Équipement et sorts",
                zone,
                progress.0.available(class.id)
            ),
        );
        let _=script.run("SurvivorsPreparationTitle:SetText(__survivors_prepare_title); SurvivorsPreparationInfo:SetText(__survivors_prepare_info)");
    }
    if script
        .eval::<bool>("return __survivors_prepare_back or false")
        .unwrap_or(false)
    {
        let _ = script.run("__survivors_prepare_back=false; CloseAllWindows()");
        next.set(Phase::Menu);
        return;
    }
    if script
        .eval::<bool>("return __survivors_prepare_enter or false")
        .unwrap_or(false)
    {
        let _ = script.run("__survivors_prepare_enter=false");
        let Some(&(_, race, _)) = selection.look.and_then(|i| selection.looks.get(i)) else {
            return;
        };
        if !world.0.available(race, selection.map) {
            return;
        }
        if hud::start_run(&mut run, &mut selection) {
            economy.new_run();
            super::economy::sync_gear(&mut run, &economy.account);
            run.hp = run.stats().max_hp;
            let mut pos = super::hero::arena(selection.stage_def());
            if let Some(height) = terrain.terrain_height_under(pos) {
                pos.y = height;
            }
            if let Ok((mut transform, mut hero)) = heroes.single_mut() {
                transform.translation = pos;
                hero.air = 0.0;
                hero.vy = 0.0;
            } else if heroes.is_empty() {
                let Some(class) = selection.class_def() else {
                    return;
                };
                let Some(&(display, race, sex)) =
                    selection.look.and_then(|i| selection.looks.get(i))
                else {
                    return;
                };
                let serial = run.guid();
                super::hero::spawn_hero(
                    &mut commands,
                    class,
                    display,
                    race,
                    sex,
                    serial,
                    pos,
                    selection.appearance,
                );
            } else {
                return;
            }
            let _ = script.run("CloseAllWindows()");
            next.set(Phase::Playing);
        }
    }
}

fn feed_book(
    script: Option<NonSendMut<UiScript>>,
    stock: Res<super::blizzard_ui::StockUi>,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    run: Res<Run>,
    progress: Res<super::character_save::TalentProgressStore>,
    talents: Option<Res<crate::ui_talent::Talents>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut memo: ResMut<Page>,
) {
    if !stock.ready {
        return;
    }
    let (Some(mut script), Some(spells)) = (script, spells) else {
        return;
    };
    let class = if matches!(*phase.get(), Phase::Menu | Phase::Preparing) {
        selection.class_def().unwrap_or(run.class)
    } else {
        run.class
    };
    let race = if matches!(*phase.get(), Phase::Menu | Phase::Preparing) {
        selection
            .look
            .and_then(|i| selection.looks.get(i))
            .map_or(1, |&(_, race, _)| race)
    } else {
        run.race
    };
    let key = (class.id, race, progress.0.clone());
    if memo.book.as_ref() == Some(&key) {
        return;
    }
    let mut slots = Vec::new();
    for key in class.pool {
        let ability = data::ability(key);
        let locked = talents.as_deref().is_some_and(|catalog| {
            catalog
                .catalog
                .tabs_for_class(race, class.id)
                .iter()
                .any(|tab| {
                    catalog.catalog.talents_in_tab(tab.id).iter().any(|talent| {
                        talent.ranks.contains(&ability.spell)
                            && progress.0.rank(class.id, talent.id) == 0
                    })
                })
        });
        if locked {
            continue;
        }
        let Some(spell) = spells.catalog.get(ability.spell) else {
            continue;
        };
        let kind = match ability.kind {
            data::Kind::Heal => "Soins",
            data::Kind::Shield => "Absorption",
            _ => "Dégâts",
        };
        let description=format!("Disponible dans les choix de progression de votre classe.\nValeurs de base au rang 1 : {kind} : {}. Recharge : {} s. Portée : {} m.",ability.damage,ability.cooldown,ability.range);
        script.set_spell_tooltip(
            ability.spell,
            SpellTooltipView {
                name: spell.name.clone(),
                description,
                item_met: true,
                form_met: true,
                ..Default::default()
            },
        );
        slots.push(SpellSlotView {
            spell_id: ability.spell,
            name: spell.name.clone(),
            texture: spell.icon.clone(),
            ..Default::default()
        });
    }
    slots.sort_by(|a, b| a.name.cmp(&b.name));
    let tab = SpellTabView {
        name: super::localization::class_name(class.id).into(),
        texture: slots.first().and_then(|s| s.texture.clone()),
        offset: 0,
        num_spells: slots.len() as u32,
    };
    script.set_spellbook(SpellBookState {
        tabs: vec![tab],
        slots,
    });
    script.fire_event("SPELLS_CHANGED", vec![]);
    memo.book = Some(key);
}
