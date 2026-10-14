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
    local forge=CreateFrame('Frame','SurvivorsPreparationForge',f)
    forge:SetWidth(560); forge:SetHeight(226); forge:SetPoint('CENTER',f,'CENTER',0,8)
    forge:SetBackdrop({bgFile='Interface\\DialogFrame\\UI-DialogBox-Background',edgeFile='Interface\\DialogFrame\\UI-DialogBox-Border',tile=true,tileSize=32,edgeSize=32,insets={left=11,right=12,top=12,bottom=11}})
    forge:SetBackdropColor(0.08,0.06,0.03,0.96)
    forge:SetBackdropBorderColor(0.78,0.62,0.28,1)
    local close=CreateFrame('Button','SurvivorsPreparationForgeClose',forge,'UIPanelCloseButton')
    close:SetPoint('TOPRIGHT',forge,'TOPRIGHT',-4,-4)
    close:SetScript('OnClick',function() forge:Hide() end)
    tinsert(UISpecialFrames,'SurvivorsPreparationForge')
    local forgeTitle=forge:CreateFontString('SurvivorsPreparationForgeTitle','OVERLAY','GameFontNormal')
    forgeTitle:SetPoint('TOP',forge,'TOP',0,-4); forgeTitle:SetText('Fusion de l’artisan')
    local forgeInfo=forge:CreateFontString('SurvivorsPreparationForgeInfo','OVERLAY','GameFontHighlightSmall')
    forgeInfo:SetWidth(520); forgeInfo:SetPoint('TOP',forgeTitle,'BOTTOM',0,-8)
    forgeInfo:SetJustifyH('CENTER')
    forgeInfo:SetText('Fusionnez 3 copies blanches identiques, non équipées et de même emplacement et rang. Résultat permanent : qualité verte.')
    local fusion=CreateFrame('Frame','SurvivorsPreparationFusionButtons',forge)
    fusion:SetPoint('TOP',forgeInfo,'BOTTOM',0,-8); fusion:SetWidth(520); fusion:SetHeight(28)
    local fusionLabels={'Arme','Armure','Anneau','Bottes'}
    for slot=0,3 do
        local selectedSlot=slot
        local b=CreateFrame('Button','SurvivorsPreparationFuse'..slot,fusion,'UIPanelButtonTemplate')
        b:SetWidth(124); b:SetHeight(26); b:SetPoint('LEFT',fusion,'LEFT',slot*130,0)
        b:SetText(fusionLabels[slot+1])
        b:SetScript('OnClick',function() __survivors_fusion_selected=selectedSlot end)
    end
    local ingredients=CreateFrame('Frame','SurvivorsPreparationFusionIngredients',forge)
    ingredients:SetPoint('TOP',fusion,'BOTTOM',0,-10); ingredients:SetWidth(520); ingredients:SetHeight(64)
    for i=1,3 do
        local icon=CreateFrame('Button','SurvivorsPreparationIngredient'..i,ingredients)
        icon:SetWidth(42); icon:SetHeight(42); icon:SetPoint('LEFT',ingredients,'LEFT',(i-1)*52+102,0)
        icon:SetNormalTexture('Interface\\Icons\\INV_Misc_QuestionMark')
        icon.texture=icon:GetNormalTexture()
        icon.count=icon:CreateFontString(nil,'OVERLAY','NumberFontNormal')
        icon.count:SetPoint('BOTTOMRIGHT',icon,'BOTTOMRIGHT',2,0)
        icon:SetScript('OnEnter',function() local self=icon
            if self.tooltip then GameTooltip:SetOwner(self,'ANCHOR_RIGHT'); GameTooltip:SetText(self.tooltip); GameTooltip:Show() end
        end)
        icon:SetScript('OnLeave',function() GameTooltip:Hide() end)
    end
    local arrow=ingredients:CreateFontString(nil,'OVERLAY','GameFontNormalLarge')
    arrow:SetPoint('LEFT',ingredients,'LEFT',286,0); arrow:SetText('>')
    local result=CreateFrame('Button','SurvivorsPreparationFusionResult',ingredients)
    result:SetWidth(42); result:SetHeight(42); result:SetPoint('LEFT',ingredients,'LEFT',326,0)
    result:SetNormalTexture('Interface\\Icons\\INV_Misc_QuestionMark')
    result.texture=result:GetNormalTexture()
    result:SetScript('OnEnter',function() local self=result
        if self.tooltip then GameTooltip:SetOwner(self,'ANCHOR_RIGHT'); GameTooltip:SetText(self.tooltip); GameTooltip:Show() end
    end)
    result:SetScript('OnLeave',function() GameTooltip:Hide() end)
    local fuse=CreateFrame('Button','SurvivorsPreparationFuseNow',ingredients,'UIPanelButtonTemplate')
    fuse:SetWidth(112); fuse:SetHeight(30); fuse:SetPoint('LEFT',ingredients,'LEFT',384,0)
    fuse:SetText('Fusionner')
    fuse:SetScript('OnClick',function()
        local row=(__survivors_fusion_slots or {})[(__survivors_fusion_selected or 0)+1]
        if row and (row.count or 0)>=3 then __survivors_fuse_slot=__survivors_fusion_selected end
    end)
    local forgeSummary=forge:CreateFontString('SurvivorsPreparationForgeSummary','OVERLAY','GameFontHighlight')
    forgeSummary:SetPoint('TOP',ingredients,'BOTTOM',0,-1); forgeSummary:SetWidth(520)
    forgeSummary:SetJustifyH('CENTER')
    forge:Hide()
    __survivors_open_forge=function()
        if MerchantFrame and MerchantFrame:IsShown() then HideUIPanel(MerchantFrame) end
        if CharacterFrame and CharacterFrame:IsShown() then HideUIPanel(CharacterFrame) end
        if TalentFrame and TalentFrame:IsShown() then HideUIPanel(TalentFrame) end
        if SpellBookFrame and SpellBookFrame:IsShown() then HideUIPanel(SpellBookFrame) end
        CloseAllBags()
        forge:Show()
    end
    __survivors_close_forge=function() forge:Hide() end
    __survivors_refresh_fusion=function()
        local rows=__survivors_fusion_slots or {}
        local selected=__survivors_fusion_selected or 0
        for slot=0,3 do
            local row=rows[slot+1] or {}
            local b=getglobal('SurvivorsPreparationFuse'..slot)
            b:SetText(fusionLabels[slot+1]..'  '..(row.count or 0)..'/3')
        end
        local row=rows[selected+1] or {}
        for i=1,3 do
            local icon=getglobal('SurvivorsPreparationIngredient'..i)
            local ingredient=row.ingredients and row.ingredients[i] or nil
            icon.texture:SetTexture(ingredient and ingredient.icon or 'Interface\\Icons\\INV_Misc_QuestionMark')
            icon.count:SetText(ingredient and ingredient.count or '')
            icon.tooltip=ingredient and ingredient.tooltip or nil
        end
        SurvivorsPreparationFusionResult.texture:SetTexture(row.result_icon or 'Interface\\Icons\\INV_Misc_QuestionMark')
        SurvivorsPreparationFusionResult.tooltip=row.result_tooltip or row.result_name or nil
        SurvivorsPreparationFusionResult:SetAlpha((row.count or 0)>=3 and 1 or 0.45)
        SurvivorsPreparationFuseNow:SetAlpha((row.count or 0)>=3 and 1 or 0.45)
    end
    local function button(name,text,x,y,w,callback)
        local b=CreateFrame('Button',name,f,'UIPanelButtonTemplate')
        b:SetWidth(w); b:SetHeight(32); b:SetPoint('BOTTOM',f,'BOTTOM',x,y)
        b:SetText(text); b:SetScript('OnClick',callback)
        return b
    end
    button('SurvivorsPreparationTalents','Talents',-180,12,112,function() ToggleTalentFrame() end)
    button('SurvivorsPreparationEquipment','Équipement',-60,12,112,function() ToggleCharacter('PaperDollFrame') end)
    button('SurvivorsPreparationSpellBook','Grimoire',60,12,112,function() ToggleSpellBook(BOOKTYPE_SPELL) end)
    button('SurvivorsPreparationBags','Sacs',180,12,112,function() OpenAllBags() end)
    f:Hide()
end
-- The grimoire lists class progression spells; casts remain on the actual combat bar.
function SpellButton_OnClick() end
function SpellButton_OnDrag() end
"#;

#[derive(Resource, Default)]
struct Page {
    preparing: bool,
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
    keys: Res<ButtonInput<KeyCode>>,
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
    if preparing && !memo.preparing {
        let _=script.run("CloseAllWindows(); if __survivors_close_forge then __survivors_close_forge() end; SurvivorsPreparationTitle:Hide(); SurvivorsPreparationInfo:Hide()");
    }
    memo.preparing = preparing;
    if preparing && keys.just_pressed(KeyCode::Escape) {
        let _ = script.run("if __survivors_close_forge then __survivors_close_forge() end");
    }
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
    let _ = script.run("if __survivors_refresh_fusion then __survivors_refresh_fusion() end");
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
            let camp_stock = economy.stock;
            economy.new_run();
            economy.stock = camp_stock;
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
        let description = format!(
            "Disponible dans les choix de progression de votre classe.\nValeurs de base au rang 1 : {kind} : {}. Recharge : {} s. Portée : {} m.",
            ability.damage, ability.cooldown, ability.range
        );
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
